//! Idempotency support for ingest re-runs (PRD-066 FR-5, AC-3).
//!
//! Each generated artifact carries a stable `source_hash` derived from
//! `(rule_id, full_source_text)`. On re-run the engine compares the new hash
//! against the existing one extracted from the artifact body to decide whether
//! to skip, create, or update.
//!
//! The hash is embedded in the artifact body as an HTML comment so it survives
//! markdown round-trips without being rendered:
//!
//! ```text
//! <!-- source_hash: 6c1eaab… -->
//! ```
//!
//! See [SPEC-004 §`source_hash`](../../../../.forgeplan/specs/SPEC-004-mapping-yaml-schema.md)
//! for the contract.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::IngestArtifactDraft;
use super::sources::ParsedSource;
use crate::artifact::canonical_key::{ArtifactCandidate, mint_artifact_key, resolve_artifact};
use crate::artifact::id_alloc::{AllocationGuard, preview_display_number};
use crate::artifact::types::ArtifactKind;
use crate::constellation::facade::capture_full_workspace_with_permit;
use crate::db::store::LanceStore;
use crate::ingest::{ArtifactTargetKind, IfExists};
use crate::projection::MutationContext;
use crate::workspace::lock::WorkspaceLock;
#[cfg(test)]
use crate::projection;

/// Decision returned by [`artifact_needs_update`].
///
/// `#[non_exhaustive]` so future idempotency policies (`Merge`, `Diff`)
/// can be added without breaking downstream `match` arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UpdateDecision {
    /// No existing artifact — create it.
    Create,
    /// Existing artifact's hash differs — update it.
    Update,
    /// Existing artifact's hash matches — no-op.
    Skip,
}

/// Compute the canonical idempotency hash for a `(rule, source)` pair.
///
/// Uses sha256 over `"{rule_id}\n{full_text}"` and returns the hex digest. The
/// rule id is included so two different rules pointing at the same source
/// document still produce distinct artifacts.
///
/// The hash is deliberately **not** computed over rendered output: that would
/// make every template tweak look like a content change. Source-level hashing
/// keeps re-runs stable across cosmetic mapping edits.
pub fn compute_source_hash(parsed: &ParsedSource, rule_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(rule_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(parsed.full_text.as_bytes());
    hex_encode(&hasher.finalize())
}

/// Compare two hashes and decide what to do on apply.
///
/// `existing_hash` should be the value extracted from the existing artifact via
/// [`extract_existing_source_hash`]; `None` means no artifact yet.
pub fn artifact_needs_update(existing_hash: Option<&str>, new_hash: &str) -> UpdateDecision {
    match existing_hash {
        None => UpdateDecision::Create,
        Some(prev) if prev == new_hash => UpdateDecision::Skip,
        Some(_) => UpdateDecision::Update,
    }
}

/// Extract the previously-stored source hash from an artifact body.
///
/// Looks for the marker `<!-- source_hash: <hex> -->`. Whitespace inside the
/// comment is tolerated; if the marker is missing or malformed the function
/// returns `None` (callers treat that as "create new artifact").
pub fn extract_existing_source_hash(artifact_body: &str) -> Option<String> {
    for line in artifact_body.lines() {
        let trimmed = line.trim();
        // Cheap filter before more expensive parsing.
        if !trimmed.starts_with("<!--") {
            continue;
        }
        // Strip the comment delimiters.
        let inner = trimmed
            .trim_start_matches("<!--")
            .trim_end_matches("-->")
            .trim();
        // Expect "source_hash: <hex>".
        let rest = match inner.strip_prefix("source_hash:") {
            Some(r) => r.trim(),
            None => continue,
        };
        if rest.is_empty() {
            continue;
        }
        // Defensive: keep only the leading hex token, drop any trailing junk.
        let hex_token: String = rest.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        if !hex_token.is_empty() {
            return Some(hex_token);
        }
    }
    None
}

/// Render the source-hash marker that should be embedded in artifact bodies.
///
/// Convenience helper used by the engine when assembling drafts.
pub fn render_source_hash_marker(hash: &str) -> String {
    format!("<!-- source_hash: {hash} -->")
}

/// One hash-bearing artifact eligible for an ingest re-run.
///
/// Callers change `source_hash` only after the corresponding body write
/// succeeds, so later drafts in the same operation see the actual new state.
#[derive(Debug)]
pub struct IngestCandidate {
    pub id: String,
    pub canonical_key: String,
    pub path: PathBuf,
    pub source_hash: String,
    rule_id: Option<String>,
}

/// One ingest operation's candidates, shared by the public CLI and native
/// playbooks. Full bodies are examined once, not per draft, and an unmarked
/// title collision is never an update candidate.
#[derive(Default)]
pub struct IngestCandidates {
    by_kind: HashMap<String, HashMap<String, Vec<IngestCandidate>>>,
}

impl IngestCandidates {
    /// Read candidates from one complete canonical capture under the caller's
    /// workspace lock. A derived index may omit a document or share an alias,
    /// so it cannot be the source of this selection.
    pub async fn load(ctx: &MutationContext<'_>) -> anyhow::Result<Self> {
        let mut candidates = Self::default();
        let captured = capture_full_workspace_with_permit(ctx.workspace(), ctx.guard).await?;
        for artifact in captured.artifacts.iter().chain(&captured.conflict_artifacts) {
            let relative_path = Path::new(&artifact.relative_path);
            if let Some(source_hash) = extract_existing_source_hash(&artifact.body) {
                candidates.insert(
                    &artifact.kind,
                    &artifact.title,
                    IngestCandidate {
                        id: artifact.display_id.clone(),
                        canonical_key: artifact.canonical_key.clone(),
                        path: ctx.workspace().join(relative_path),
                        source_hash,
                        rule_id: ingest_rule_id(&artifact.tags),
                    },
                );
            }
        }
        Ok(candidates)
    }

    /// Find the exact hash across all same-kind/title candidates first.
    /// A changed import then uses its rule identity, or an older hash-bearing
    /// candidate whose source tag was not persisted. Unrelated rules and
    /// unmarked artifacts cannot mask the matching import. An ambiguous
    /// match is an error: choosing one would overwrite an arbitrary document.
    pub fn matching(
        &mut self,
        kind: &str,
        draft: &IngestArtifactDraft,
    ) -> anyhow::Result<Option<&mut IngestCandidate>> {
        let Some(bucket) = self
            .by_kind
            .get_mut(kind)
            .and_then(|titles| titles.get_mut(&draft.title.to_ascii_lowercase()))
        else {
            return Ok(None);
        };
        let mut index = unique_candidate_index(bucket, |candidate| {
            candidate.source_hash == draft.source_hash
        })?;
        if index.is_none() {
            index = unique_candidate_index(bucket, |candidate| {
                candidate.rule_id.as_deref() == Some(draft.rule_id.as_str())
            })?;
        }
        if index.is_none() {
            index = unique_candidate_index(bucket, |candidate| candidate.rule_id.is_none())?;
        }
        Ok(index.and_then(|index| bucket.get_mut(index)))
    }


    /// Remember a successfully created artifact before processing another
    /// draft. Failed writes must not be inserted as existing candidates.
    pub fn record_written(
        &mut self,
        kind: &str,
        draft: &IngestArtifactDraft,
        id: String,
        canonical_key: String,
        path: PathBuf,
    ) {
        self.insert(
            kind,
            &draft.title,
            IngestCandidate {
                id,
                canonical_key,
                path,
                source_hash: draft.source_hash.clone(),
                rule_id: Some(draft.rule_id.clone()),
            },
        );
    }

    fn insert(&mut self, kind: &str, title: &str, candidate: IngestCandidate) {
        let bucket = self
            .by_kind
            .entry(kind.to_string())
            .or_default()
            .entry(title.to_ascii_lowercase())
            .or_default();
        match bucket.binary_search_by(|existing| existing.canonical_key.cmp(&candidate.canonical_key)) {
            Ok(index) => bucket[index] = candidate,
            Err(index) => bucket.insert(index, candidate),
        }
    }


}

fn unique_candidate_index(
    candidates: &[IngestCandidate],
    matches: impl Fn(&IngestCandidate) -> bool,
) -> anyhow::Result<Option<usize>> {
    let mut matched = candidates.iter().enumerate().filter(|(_, candidate)| matches(candidate));
    let Some((index, first)) = matched.next() else {
        return Ok(None);
    };
    if let Some((_, second)) = matched.next() {
        anyhow::bail!(
            "ambiguous ingest candidates: {:?} and {:?}",
            first.canonical_key,
            second.canonical_key
        );
    }
    Ok(Some(index))
}

fn ingest_rule_id(tags: &[String]) -> Option<String> {
    tags.iter()
        .find_map(|tag| tag.strip_prefix("source=ingest:").map(str::to_string))
}

/// A preflighted write retaining its selected canonical identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannedAction {
    Create { id: String, key: String },
    Update { id: String, key: String },
    Skip { id: String, key: String },
}

/// A target and relation resolved before opening a writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedLink {
    pub target: String,
    pub relation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedDraft {
    pub action: PlannedAction,
    pub links: Vec<PlannedLink>,
}

/// Map the engine's target kind to the canonical allocator kind.
pub fn draft_artifact_kind(kind: &ArtifactTargetKind) -> ArtifactKind {
    match kind {
        ArtifactTargetKind::Prd => ArtifactKind::Prd,
        ArtifactTargetKind::Adr => ArtifactKind::Adr,
        ArtifactTargetKind::Epic => ArtifactKind::Epic,
        ArtifactTargetKind::Note => ArtifactKind::Note,
        ArtifactTargetKind::Spec => ArtifactKind::Spec,
        ArtifactTargetKind::Problem => ArtifactKind::ProblemCard,
    }
}

/// Preflight a batch under the workspace and allocator guards. Replanning
/// after migration reuses minted keys and refuses changed identities or links.
pub async fn plan_drafts(
    workspace: &Path,
    guard: &WorkspaceLock,
    allocation: &AllocationGuard,
    store: Option<&LanceStore>,
    drafts: &[IngestArtifactDraft],
    allow_update: bool,
    prior: Option<&[PlannedDraft]>,
) -> anyhow::Result<Vec<PlannedDraft>> {
    let ctx = MutationContext::new(workspace, guard, store);
    let mut ingest = IngestCandidates::load(&ctx).await?;
    let mut candidates = crate::artifact::documents::workspace_candidates(workspace).await?;
    let mut edges = HashSet::<(String, String, String)>::new();
    for candidate in &candidates {
        let text = tokio::fs::read_to_string(workspace.join(&candidate.relative_path)).await?;
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&text)?;
        for (target, relation) in crate::artifact::frontmatter::links_from_frontmatter(&fm) {
            let target = if target.contains('@') {
                target
            } else {
                resolve_artifact(&target, &candidates).ok().flatten()
                    .map(|found| found.identity.canonical_key)
                    .unwrap_or(target)
            };
            edges.insert((candidate.canonical_key.clone(), target, relation));
        }
    }

    let mut bumps = BTreeMap::<String, u32>::new();
    let mut plan = Vec::with_capacity(drafts.len());
    for (index, draft) in drafts.iter().enumerate() {
        let kind = draft_artifact_kind(&draft.kind);
        let kind_key = kind.template_key();
        let action = if let Some(existing) = ingest.matching(kind_key, draft)? {
            let resolved = resolve_artifact(&existing.canonical_key, &candidates)?
                .ok_or_else(|| anyhow::anyhow!("ingest target '{}' is missing", existing.canonical_key))?;
            if resolved.identity.canonical_key != existing.canonical_key {
                anyhow::bail!("ingest target identity changed");
            }
            match artifact_needs_update(Some(&existing.source_hash), &draft.source_hash) {
                UpdateDecision::Skip => PlannedAction::Skip { id: existing.id.clone(), key: existing.canonical_key.clone() },
                UpdateDecision::Update if !allow_update => PlannedAction::Skip { id: existing.id.clone(), key: existing.canonical_key.clone() },
                UpdateDecision::Update => {
                    existing.source_hash.clone_from(&draft.source_hash);
                    PlannedAction::Update { id: existing.id.clone(), key: existing.canonical_key.clone() }
                }
                UpdateDecision::Create => anyhow::bail!("unexpected ingest update decision"),
            }
        } else {
            let key = match prior.and_then(|plan| plan.get(index)) {
                Some(PlannedDraft { action: PlannedAction::Create { key, .. }, .. }) => key.clone(),
                Some(_) => anyhow::bail!("ingest action changed while reopening the index"),
                None if prior.is_some() => anyhow::bail!("ingest plan length changed"),
                None => mint_artifact_key(&kind, &draft.title)?,
            };
            if candidates.iter().any(|candidate| candidate.canonical_key == key) {
                anyhow::bail!("ingest canonical key '{key}' is already claimed");
            }
            let prefix = kind.prefix().to_string();
            let (id, number) = preview_display_number(
                workspace, guard, allocation, store, &kind, 3, bumps.get(&prefix).copied(),
            ).await?;
            bumps.insert(prefix, number);
            ingest.record_written(kind_key, draft, id.clone(), key.clone(), PathBuf::new());
            candidates.push(ArtifactCandidate {
                canonical_key: key.clone(),
                display_id: id.clone(),
                title: draft.title.clone(),
                relative_path: PathBuf::new(),
            });
            PlannedAction::Create { id, key }
        };
        plan.push(PlannedDraft { action, links: Vec::new() });
    }

    for (draft, planned) in drafts.iter().zip(&mut plan) {
        let source = match &planned.action {
            PlannedAction::Skip { .. } => continue,
            PlannedAction::Create { key, .. } | PlannedAction::Update { key, .. } => key,
        };
        for link in &draft.links {
            let relation = crate::link::normalize_relation(&link.relation)?;
            let target = if link.target.contains('@') {
                link.target.clone()
            } else {
                match resolve_artifact(&link.target, &candidates)? {
                    Some(found) if found.identity.canonical_key == *source => {
                        anyhow::bail!("ingest self-link for '{source}'");
                    }
                    Some(found) => found.identity.canonical_key,
                    None if matches!(link.if_exists, IfExists::Error) => {
                        anyhow::bail!("ingest link target '{}' not found", link.target);
                    }
                    None => continue,
                }
            };
            if !edges.insert((source.clone(), target.clone(), relation.clone())) {
                if matches!(link.if_exists, IfExists::Error) {
                    anyhow::bail!("ingest link already exists for '{source}'");
                }
                continue;
            }
            planned.links.push(PlannedLink { target, relation });
        }
    }
    if prior.is_some_and(|before| before != plan.as_slice()) {
        anyhow::bail!("ingest identities or links changed while reopening the index");
    }
    Ok(plan)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::sources::ParsedSource;
    use serde_json::json;
    use std::collections::HashMap;

    fn make_parsed(text: &str) -> ParsedSource {
        ParsedSource {
            path: "test.md".to_owned(),
            front_matter: json!({}),
            sections: HashMap::new(),
            full_text: text.to_owned(),
            line_count: text.lines().count(),
        }
    }

    #[test]
    fn same_input_same_hash() {
        let p = make_parsed("hello world");
        let a = compute_source_hash(&p, "rule-1");
        let b = compute_source_hash(&p, "rule-1");
        assert_eq!(a, b);
        // sha256 hex digest is 64 chars.
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn different_rule_id_changes_hash() {
        let p = make_parsed("hello world");
        let a = compute_source_hash(&p, "rule-1");
        let b = compute_source_hash(&p, "rule-2");
        assert_ne!(a, b);
    }

    #[test]
    fn different_text_changes_hash() {
        let a = compute_source_hash(&make_parsed("hello world"), "r");
        let b = compute_source_hash(&make_parsed("hello world!"), "r");
        assert_ne!(a, b);
    }

    #[test]
    fn update_decision_matrix() {
        assert_eq!(artifact_needs_update(None, "abc"), UpdateDecision::Create);
        assert_eq!(
            artifact_needs_update(Some("abc"), "abc"),
            UpdateDecision::Skip
        );
        assert_eq!(
            artifact_needs_update(Some("abc"), "xyz"),
            UpdateDecision::Update
        );
    }

    #[test]
    fn extract_marker_round_trips() {
        let marker = render_source_hash_marker("deadbeef");
        let body = format!("# Title\n\n{marker}\n\nbody text\n");
        assert_eq!(
            extract_existing_source_hash(&body).as_deref(),
            Some("deadbeef")
        );
    }

    #[test]
    fn extract_marker_missing_returns_none() {
        assert!(extract_existing_source_hash("# Title\n\nno hash here\n").is_none());
        assert!(extract_existing_source_hash("").is_none());
    }

    #[test]
    fn extract_marker_handles_extra_whitespace() {
        let body = "<!--   source_hash:    cafe1234   -->\n";
        assert_eq!(
            extract_existing_source_hash(body).as_deref(),
            Some("cafe1234")
        );
    }

    #[test]
    fn extract_marker_rejects_non_hex_payload() {
        let body = "<!-- source_hash: not-a-hash -->\n";
        // "not" starts with non-hex chars, so leading hex token is empty → None.
        assert!(extract_existing_source_hash(body).is_none());
    }

    #[test]
    fn shared_display_id_does_not_replace_another_ingest_candidate() {
        let draft = |hash: &str| IngestArtifactDraft {
            kind: super::super::ArtifactTargetKind::Note,
            title: "Shared title".to_string(),
            body: String::new(),
            source_hash: hash.to_string(),
            links: Vec::new(),
            rule_id: "rule".to_string(),
            source_path: "source.md".to_string(),
        };
        let first = draft("first");
        let second = draft("second");
        let mut candidates = IngestCandidates::default();
        candidates.record_written("note", &first, "NOTE-001".into(), "note-first".into(), "first.md".into());
        candidates.record_written("note", &second, "NOTE-001".into(), "note-second".into(), "second.md".into());
        assert_eq!(candidates.matching("note", &first).unwrap().unwrap().canonical_key, "note-first");
        assert_eq!(candidates.matching("note", &second).unwrap().unwrap().canonical_key, "note-second");
        let error = candidates.matching("note", &draft("changed")).unwrap_err();
        assert!(error.to_string().contains("note-first"));
        assert!(error.to_string().contains("note-second"));
        assert_eq!(candidates.matching("note", &first).unwrap().unwrap().canonical_key, "note-first");
        assert_eq!(candidates.matching("note", &second).unwrap().unwrap().canonical_key, "note-second");
    }

    #[tokio::test]
    async fn batch_preflight_resolves_forward_links_without_writing_documents() {
        use crate::artifact::id_alloc::acquire_allocation_guard;
        use crate::ingest::DraftLink;
        use crate::workspace::{init_workspace, lock::acquire_workspace_lock};

        let project = tempfile::TempDir::new().unwrap();
        let ws = init_workspace(project.path(), "batch-preflight").unwrap();
        let guard = acquire_workspace_lock(&ws).await.unwrap();
        let kinds = [ArtifactKind::Note];
        let allocation = acquire_allocation_guard(&ws, &guard, &kinds).await.unwrap();
        let draft = |title: &str, rule: &str, links| IngestArtifactDraft {
            kind: ArtifactTargetKind::Note,
            title: title.into(),
            body: format!("## Summary\n\n{title}\n"),
            source_hash: rule.into(),
            links,
            rule_id: rule.into(),
            source_path: "input.md".into(),
        };
        let drafts = vec![
            draft("First", "a", vec![DraftLink {
                target: "NOTE-002".into(),
                relation: "informs".into(),
                if_exists: IfExists::Error,
            }]),
            draft("Second", "b", Vec::new()),
        ];
        let before = crate::artifact::documents::workspace_candidates(&ws).await.unwrap();
        let plan = plan_drafts(&ws, &guard, &allocation, None, &drafts, false, None).await.unwrap();
        let PlannedAction::Create { id, key } = &plan[1].action else { panic!("expected creation") };
        assert_eq!(id, "NOTE-002");
        assert_eq!(plan[0].links[0].target, *key);
        assert_eq!(plan[0].links[0].relation, "informs");
        let again = plan_drafts(&ws, &guard, &allocation, None, &drafts, false, Some(&plan)).await.unwrap();
        assert_eq!(plan, again);
        assert_eq!(before, crate::artifact::documents::workspace_candidates(&ws).await.unwrap());
    }

    #[tokio::test]
    async fn batch_preflight_rejects_ambiguous_link_before_any_write() {
        use crate::artifact::id_alloc::acquire_allocation_guard;
        use crate::db::store::NewArtifact;
        use crate::ingest::DraftLink;
        use crate::workspace::{init_workspace, lock::acquire_workspace_lock};

        let project = tempfile::TempDir::new().unwrap();
        let ws = init_workspace(project.path(), "batch-ambiguity").unwrap();
        let guard = acquire_workspace_lock(&ws).await.unwrap();
        let ctx = MutationContext::new(&ws, &guard, None);
        for (key, title) in [("note-first", "First"), ("note-second", "Second")] {
            projection::create_artifact_with_canonical_key(&ctx, &NewArtifact {
                canonical_key: key.into(), id: "NOTE-001".into(), kind: "note".into(),
                status: "draft".into(), title: title.into(), body: "## Summary\n\nText\n".into(),
                depth: "tactical".into(), author: None, parent_epic: None,
                valid_until: None, tags: Vec::new(),
            }).await.unwrap();
        }
        let allocation = acquire_allocation_guard(&ws, &guard, &[ArtifactKind::Note]).await.unwrap();
        let draft = IngestArtifactDraft {
            kind: ArtifactTargetKind::Note, title: "New".into(), body: "## Summary\n".into(),
            source_hash: "abc".into(), rule_id: "new".into(), source_path: "input.md".into(),
            links: vec![DraftLink { target: "NOTE-001".into(), relation: "informs".into(), if_exists: IfExists::Skip }],
        };
        let before = crate::artifact::documents::workspace_candidates(&ws).await.unwrap();
        let error = plan_drafts(&ws, &guard, &allocation, None, &[draft], false, None).await.unwrap_err();
        assert!(error.to_string().contains("matches 2 documents"), "{error:#}");
        assert_eq!(before, crate::artifact::documents::workspace_candidates(&ws).await.unwrap());
    }

    #[tokio::test]
    async fn candidate_index_loads_both_owners_and_remembers_real_writes() {
        use crate::constellation::capture::CaptureLimits;
        use crate::constellation::facade::{MaintainCapability, maintain_index};
        use crate::db::store::{LanceStore, NewArtifact};
        use crate::workspace::{init_workspace, lock::acquire_workspace_lock};
        use std::time::{Duration, Instant};

        let draft = |title: &str, text: &str| {
            let source_hash = compute_source_hash(&make_parsed(text), "rule");
            IngestArtifactDraft {
                kind: super::super::ArtifactTargetKind::Note,
                title: title.to_string(),
                body: format!(
                    "## Summary\n\n{text}\n\n{}",
                    render_source_hash_marker(&source_hash)
                ),
                source_hash,
                links: Vec::new(),
                rule_id: "rule".to_string(),
                source_path: "source.md".to_string(),
            }
        };
        let exact = draft("Shared title", "exact source");
        let other = draft("Shared title", "other source");
        let updated = draft("Shared title", "updated source");
        let created = draft("New title", "new source");

        for migrated in [false, true] {
            let project = tempfile::TempDir::new().expect("project");
            let ws = init_workspace(project.path(), "ingest-candidates").expect("workspace");
            let lock = acquire_workspace_lock(&ws).await.expect("seed lock");
            let store = LanceStore::init_new(&ws, &lock).await.expect("legacy fixture");
            let ctx = MutationContext::new(&ws, &lock, Some(&store));
            let mut unmarked_path = None;
            for (id, body) in [
                ("NOTE-001", "Unrelated unmarked title collision."),
                ("NOTE-002", other.body.as_str()),
                ("NOTE-003", exact.body.as_str()),
            ] {
                let path = projection::create_artifact_with_projection(
                    &ctx,
                    &NewArtifact {
                        canonical_key: id.to_string(),
                        id: id.to_string(),
                        kind: "note".to_string(),
                        title: "Shared title".to_string(),
                        body: body.to_string(),
                        status: "draft".to_string(),
                        depth: "tactical".to_string(),
                        author: None,
                        parent_epic: None,
                        valid_until: None,
                        tags: vec!["source=ingest:rule".to_string()],
                    },
                )
                .await
                .expect("seed canonical artifact");
                if id == "NOTE-001" {
                    unmarked_path = Some(path);
                }
            }
            let unmarked_path = unmarked_path.expect("unmarked path");
            let unmarked = std::fs::read(&unmarked_path).expect("unmarked bytes");
            drop(store);
            drop(lock);
            if migrated {
                let budget = Duration::from_secs(120);
                maintain_index(
                    &MaintainCapability::for_strict_maintenance(
                        ws.canonicalize().expect("canonical"),
                    ),
                    CaptureLimits {
                        budget,
                        ..CaptureLimits::default()
                    },
                    Instant::now() + budget,
                )
                .await
                .expect("publish fixture");
            }

            let lock = acquire_workspace_lock(&ws).await.expect("mutation lock");
            let store = if migrated {
                None
            } else {
                Some(LanceStore::open(&ws).await.expect("legacy owner"))
            };
            let ctx = MutationContext::new(&ws, &lock, store.as_ref());
            let mut candidates = IngestCandidates::load(&ctx).await.expect("owning source");
            assert_eq!(
                candidates.matching("note", &exact).unwrap().expect("exact match").id,
                "NOTE-003"
            );
            assert!(candidates.matching("prd", &exact).unwrap().is_none());
            assert!(candidates.matching("note", &created).unwrap().is_none());

            let build = |id: &str, _number: u32, key: &str| Ok(NewArtifact {
                canonical_key: key.to_string(),
                id: id.to_string(),
                kind: "note".to_string(),
                title: created.title.clone(),
                body: created.body.clone(),
                status: "draft".to_string(),
                depth: "tactical".to_string(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: vec!["source=ingest:rule".to_string()],
            });
            let written = crate::artifact::id_alloc::create_artifact_with_minted_key(
                &ws, &lock, store.as_ref(), &ArtifactKind::Note, 3, &created.title, &build,
            )
            .await
            .expect("real creation");
            assert_eq!(written.id, "NOTE-004");
            let path = written.filepath;
            candidates.record_written("note", &created, written.id, written.canonical_key, path.clone());
            let candidate = candidates
                .matching("note", &created)
                .unwrap()
                .expect("created candidate");
            assert_eq!(candidate.id, "NOTE-004");
            assert_eq!(candidate.path, path);
            assert!(
                std::fs::read_to_string(&path)
                    .expect("created body")
                    .contains(&created.source_hash)
            );

            let candidate = candidates
                .matching("note", &exact)
                .unwrap()
                .expect("existing import");
            projection::update_body_with_projection(&ctx, &candidate.canonical_key, &updated.body)
                .await
                .expect("real update");
            candidate.source_hash.clone_from(&updated.source_hash);
            assert_eq!(
                candidates
                    .matching("note", &updated)
                    .unwrap()
                    .expect("updated candidate")
                    .id,
                "NOTE-003"
            );
            let next = crate::artifact::id_alloc::reserve_display_number(
                &ws, &lock, store.as_ref(), &ArtifactKind::Note, 3,
            ).await.unwrap();
            assert_eq!(next.id, "NOTE-005");
            assert_eq!(
                std::fs::read(unmarked_path).expect("preserved collision"),
                unmarked
            );
            drop(store);
            drop(lock);
        }
    }
}
