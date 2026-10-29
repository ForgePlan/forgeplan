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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::IngestArtifactDraft;
use super::sources::ParsedSource;
use crate::artifact::types::ArtifactKind;
use crate::constellation::capture::CaptureLimits;
use crate::constellation::facade::open_workspace_read;
use crate::projection::{self, MutationContext};

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
    pub path: PathBuf,
    pub source_hash: String,
    rule_id: Option<String>,
}

/// One ingest operation's candidates and ID high-water marks, shared by the
/// public CLI and native playbooks. Full bodies are examined once, not per
/// draft, and an unmarked title collision is never an update candidate.
#[derive(Default)]
pub struct IngestCandidates {
    by_kind: HashMap<String, HashMap<String, Vec<IngestCandidate>>>,
    max_numbers: HashMap<String, u32>,
}

impl IngestCandidates {
    /// Read candidates from the already selected owning source under the
    /// caller's workspace lock. No legacy store is opened by this method.
    /// A canonical capture must be complete; its errors are not empty results.
    pub async fn load(ctx: &MutationContext<'_>) -> anyhow::Result<Self> {
        let mut candidates = Self::default();
        match ctx.legacy_sink() {
            Some(store) => {
                for record in store.list_records(None).await? {
                    candidates.observe_number(&record.id);
                    if let Some(source_hash) = extract_existing_source_hash(&record.body)
                        && let Ok(kind) = record.kind.parse::<ArtifactKind>()
                    {
                        let path = ctx.workspace().join(kind.dir_name()).join(format!(
                            "{}-{}.md",
                            record.id,
                            projection::projection_slug(&record.title),
                        ));
                        candidates.insert(
                            &record.kind,
                            &record.title,
                            IngestCandidate {
                                id: record.id,
                                path,
                                source_hash,
                                rule_id: ingest_rule_id(&record.tags),
                            },
                        );
                    }
                }
            }
            None => {
                let read =
                    open_workspace_read(ctx.workspace(), None, CaptureLimits::default()).await?;
                for artifact in &read.captured().artifacts {
                    let relative_path = Path::new(&artifact.relative_path);
                    candidates.observe_filename(relative_path);
                    if let Some(source_hash) = extract_existing_source_hash(&artifact.body) {
                        candidates.insert(
                            &artifact.kind,
                            &artifact.title,
                            IngestCandidate {
                                id: artifact.display_id.clone(),
                                path: ctx.workspace().join(relative_path),
                                source_hash,
                                rule_id: ingest_rule_id(&artifact.tags),
                            },
                        );
                    }
                }
            }
        }
        Ok(candidates)
    }

    /// Find the exact hash across all same-kind/title candidates first.
    /// A changed import then uses its rule identity, or an older hash-bearing
    /// candidate whose source tag was not persisted. Unrelated rules and
    /// unmarked artifacts cannot mask the matching import.
    pub fn matching(
        &mut self,
        kind: &str,
        draft: &IngestArtifactDraft,
    ) -> Option<&mut IngestCandidate> {
        let bucket = self
            .by_kind
            .get_mut(kind)?
            .get_mut(&draft.title.to_ascii_lowercase())?;
        let index = bucket
            .iter()
            .position(|candidate| candidate.source_hash == draft.source_hash)
            .or_else(|| {
                bucket.iter().position(|candidate| {
                    candidate.rule_id.as_deref() == Some(draft.rule_id.as_str())
                })
            })
            .or_else(|| {
                bucket
                    .iter()
                    .position(|candidate| candidate.rule_id.is_none())
            })?;
        bucket.get_mut(index)
    }

    /// Reserve the next ID from this operation's owning-source high-water
    /// marks. The caller retains the same workspace lock through the write.
    pub fn next_id(&mut self, kind: &ArtifactKind) -> String {
        let prefix = kind.prefix().trim_end_matches('-').to_ascii_uppercase();
        let number = self.max_numbers.entry(prefix.clone()).or_default();
        *number += 1;
        format!("{prefix}-{number:03}")
    }

    /// Remember a successfully created artifact before processing another
    /// draft. Failed writes must not be inserted as existing candidates.
    pub fn record_written(
        &mut self,
        kind: &str,
        draft: &IngestArtifactDraft,
        id: String,
        path: PathBuf,
    ) {
        self.insert(
            kind,
            &draft.title,
            IngestCandidate {
                id,
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
        match bucket.binary_search_by(|existing| existing.id.cmp(&candidate.id)) {
            Ok(index) => bucket[index] = candidate,
            Err(index) => bucket.insert(index, candidate),
        }
    }

    fn observe_number(&mut self, id_or_filename: &str) {
        let Some((prefix, rest)) = id_or_filename.split_once('-') else {
            return;
        };
        let Some(number) = rest
            .split('-')
            .next()
            .and_then(|part| part.parse::<u32>().ok())
        else {
            return;
        };
        let max_number = self
            .max_numbers
            .entry(prefix.to_ascii_uppercase())
            .or_default();
        *max_number = (*max_number).max(number);
    }

    fn observe_filename(&mut self, relative_path: &Path) {
        let Some(filename) = relative_path.file_name().and_then(|name| name.to_str()) else {
            return;
        };
        let Some((prefix, _)) = filename.split_once('-') else {
            return;
        };
        let Some(kind) = ArtifactKind::from_slug_prefix(&prefix.to_ascii_lowercase()) else {
            return;
        };
        if relative_path.parent() == Some(Path::new(kind.dir_name())) {
            self.observe_number(filename);
        }
    }
}

fn ingest_rule_id(tags: &[String]) -> Option<String> {
    tags.iter()
        .find_map(|tag| tag.strip_prefix("source=ingest:").map(str::to_string))
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

    #[tokio::test]
    async fn candidate_index_loads_both_owners_and_remembers_real_writes() {
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
            let store = LanceStore::init(&ws).await.expect("legacy fixture");
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
                candidates.matching("note", &exact).expect("exact match").id,
                "NOTE-003"
            );
            assert!(candidates.matching("prd", &exact).is_none());
            assert!(candidates.matching("note", &created).is_none());

            let id = candidates.next_id(&ArtifactKind::Note);
            assert_eq!(id, "NOTE-004");
            let path = projection::create_artifact_with_projection(
                &ctx,
                &NewArtifact {
                    id: id.clone(),
                    kind: "note".to_string(),
                    title: created.title.clone(),
                    body: created.body.clone(),
                    status: "draft".to_string(),
                    depth: "tactical".to_string(),
                    author: None,
                    parent_epic: None,
                    valid_until: None,
                    tags: vec!["source=ingest:rule".to_string()],
                },
            )
            .await
            .expect("real creation");
            candidates.record_written("note", &created, id, path.clone());
            let candidate = candidates
                .matching("note", &created)
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
                .expect("existing import");
            projection::update_body_with_projection(&ctx, &candidate.id, &updated.body)
                .await
                .expect("real update");
            candidate.source_hash.clone_from(&updated.source_hash);
            assert_eq!(
                candidates
                    .matching("note", &updated)
                    .expect("updated candidate")
                    .id,
                "NOTE-003"
            );
            assert_eq!(candidates.next_id(&ArtifactKind::Note), "NOTE-005");
            assert_eq!(
                std::fs::read(unmarked_path).expect("preserved collision"),
                unmarked
            );
            drop(store);
            drop(lock);
        }
    }
}
