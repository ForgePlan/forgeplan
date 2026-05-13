//! Auto-generate Keep-a-Changelog–shaped release notes from artifacts that
//! changed between two git refs.
//!
//! Pure side of the feature: categorisation, quality gate, and output
//! formatting. Git walking + record loading live in the CLI layer so this
//! module can be unit-tested without a real workspace / git history.
//!
//! ## Mapping (artifact kind + status → category)
//!
//! | Source artifact         | Category   | Notes                                  |
//! |-------------------------|------------|----------------------------------------|
//! | PRD (active)            | Added      | shipped feature                        |
//! | PROB (deprecated) + EVID| Fixed      | bug closed with evidence               |
//! | EVID on security PROB   | Security   | tag `security` on related PROB         |
//! | RFC/ADR (active)        | Changed    | architectural change                   |
//! | Spec/Epic               | Changed    | grouped umbrellas                      |
//! | Refresh / Note / Memory | (filtered) | excluded                               |
//!
//! Quality gate (default): only artifacts with `r_eff_score > 0` **or**
//! `status == "active"` are emitted. `--draft` waives both — useful while
//! preparing a release that has unevidenced active artifacts in flight.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::db::store::{ArtifactRecord, LanceStore};
use crate::git::validate_git_ref;

/// Top-level Keep-a-Changelog section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Added,
    Fixed,
    Security,
    Changed,
    Internal,
}

impl Category {
    /// Section heading as it appears in Keep-a-Changelog.
    pub fn heading(self) -> &'static str {
        match self {
            Self::Added => "Added",
            Self::Fixed => "Fixed",
            Self::Security => "Security",
            Self::Changed => "Changed",
            Self::Internal => "Internal",
        }
    }

    /// JSON key (snake-case) used in the structured payload — matches the
    /// `kebab-or-snake` shape the briefing spec asked for.
    pub fn json_key(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Fixed => "fixed",
            Self::Security => "security",
            Self::Changed => "changed",
            Self::Internal => "internal",
        }
    }

    /// Section ordering used by every formatter (markdown / text / JSON).
    pub fn ordered() -> [Self; 5] {
        [
            Self::Added,
            Self::Fixed,
            Self::Security,
            Self::Changed,
            Self::Internal,
        ]
    }
}

/// One row in a release-notes section.
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub id: String,
    pub title: String,
    /// Linked EvidencePack id, if a closing EVID was attached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evid: Option<String>,
    /// Short (7-char) git SHA at which the artifact was touched. Empty
    /// when the caller cannot resolve a commit (e.g. CLI test fixtures
    /// without a git history).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub commit: String,
    pub kind: String,
}

/// The full release-notes payload.
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseNotes {
    pub since: String,
    pub until: String,
    /// True when the user passed `--draft` (no quality gate applied).
    pub draft: bool,
    /// Sections keyed by category. Empty sections are still rendered as
    /// keys in JSON so downstream consumers don't need to guess.
    #[serde(flatten)]
    pub sections: BTreeMap<&'static str, Vec<Entry>>,
}

impl ReleaseNotes {
    pub fn new(since: impl Into<String>, until: impl Into<String>, draft: bool) -> Self {
        let mut sections = BTreeMap::new();
        for cat in Category::ordered() {
            sections.insert(cat.json_key(), Vec::new());
        }
        Self {
            since: since.into(),
            until: until.into(),
            draft,
            sections,
        }
    }

    pub fn push(&mut self, category: Category, entry: Entry) {
        self.sections
            .entry(category.json_key())
            .or_default()
            .push(entry);
    }

    /// Total entries across all sections.
    pub fn total(&self) -> usize {
        self.sections.values().map(Vec::len).sum()
    }

    /// True when the gate filtered every candidate.
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

/// Decision shape returned by [`classify`].
///
/// Separated from [`Category`] so the caller knows when something was
/// filtered (Refresh / Note / Memory / draft-gated) vs. when it landed in
/// a real section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    /// Surface in the named section.
    Include(Category),
    /// Skipped — kind is not part of release-notes output (Note, Memory, Refresh).
    SkipKind,
    /// Skipped — quality gate (no evidence + not active, and `draft` false).
    SkipQuality,
}

/// Categorise one artifact record per the rules described at the module
/// top. `linked_security` is true when the caller has determined that
/// the relation graph touching this artifact has at least one neighbour
/// tagged `security` — checked at the call-site because the relation
/// scan is async.
///
/// Pure function — no I/O, no allocations beyond the returned tag.
pub fn classify(record: &ArtifactRecord, linked_security: bool, draft: bool) -> Classification {
    let kind = record.kind.as_str();

    // Filtered kinds first — they never appear in release notes.
    if matches!(kind, "note" | "memory" | "refresh") {
        return Classification::SkipKind;
    }

    // Quality gate: skip artifacts with no signal unless `--draft`.
    // An artifact "has signal" when it's active (passed validation +
    // typically has evidence) OR carries a positive R_eff score.
    let has_signal = record.status == "active" || record.r_eff_score > 0.0;
    if !draft && !has_signal {
        return Classification::SkipQuality;
    }

    let cat = match kind {
        "prd" => Category::Added,
        "problem" => Category::Fixed,
        "evidence" => {
            if linked_security {
                Category::Security
            } else {
                // Plain evidence rolls up under Internal so it doesn't
                // drown the user-facing sections, but still appears in
                // draft mode for reviewer context.
                Category::Internal
            }
        }
        "rfc" | "adr" | "spec" | "epic" | "solution" => Category::Changed,
        _ => Category::Internal,
    };
    Classification::Include(cat)
}

/// Markdown formatter — matches the Keep-a-Changelog template the project
/// uses in `CHANGELOG.md`.
pub fn format_markdown(notes: &ReleaseNotes) -> String {
    let mut out = String::new();
    let heading = format!("## [{} → {}]", notes.since, notes.until);
    out.push_str(&heading);
    out.push('\n');
    if notes.draft {
        out.push_str("> Draft mode — quality gate disabled.\n");
    }
    out.push('\n');

    for cat in Category::ordered() {
        let entries = notes
            .sections
            .get(cat.json_key())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if entries.is_empty() {
            continue;
        }
        out.push_str("### ");
        out.push_str(cat.heading());
        out.push('\n');
        for e in entries {
            out.push_str("- ");
            out.push_str(&e.title);
            out.push_str(" (");
            out.push_str(&e.id);
            if let Some(ref evid) = e.evid {
                out.push_str(", ");
                out.push_str(evid);
            }
            if !e.commit.is_empty() {
                out.push_str(", commit ");
                out.push_str(&e.commit);
            }
            out.push_str(")\n");
        }
        out.push('\n');
    }

    if notes.is_empty() {
        out.push_str("_No artifacts matched the requested range._\n");
    }
    out
}

/// Plain-text formatter — same content as markdown but without the
/// markdown adornments, for terminal previews.
pub fn format_text(notes: &ReleaseNotes) -> String {
    let mut out = String::new();
    out.push_str(&format!("{} -> {}\n", notes.since, notes.until));
    if notes.draft {
        out.push_str("(draft mode — quality gate disabled)\n");
    }
    out.push('\n');

    for cat in Category::ordered() {
        let entries = notes
            .sections
            .get(cat.json_key())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if entries.is_empty() {
            continue;
        }
        out.push_str(cat.heading());
        out.push_str(":\n");
        for e in entries {
            out.push_str("  - ");
            out.push_str(&e.title);
            out.push_str(" (");
            out.push_str(&e.id);
            if let Some(ref evid) = e.evid {
                out.push_str(", ");
                out.push_str(evid);
            }
            if !e.commit.is_empty() {
                out.push_str(", ");
                out.push_str(&e.commit);
            }
            out.push_str(")\n");
        }
        out.push('\n');
    }

    if notes.is_empty() {
        out.push_str("(no artifacts matched the requested range)\n");
    }
    out
}

/// JSON formatter — structured payload matching the briefing spec.
pub fn format_json(notes: &ReleaseNotes) -> serde_json::Value {
    serde_json::json!({
        "since": notes.since,
        "until": notes.until,
        "draft": notes.draft,
        "total": notes.total(),
        "added": notes.sections.get("added").cloned().unwrap_or_default(),
        "fixed": notes.sections.get("fixed").cloned().unwrap_or_default(),
        "security": notes.sections.get("security").cloned().unwrap_or_default(),
        "changed": notes.sections.get("changed").cloned().unwrap_or_default(),
        "internal": notes.sections.get("internal").cloned().unwrap_or_default(),
    })
}

/// Try `git describe --tags --abbrev=0` to find the latest tag. Returns
/// `None` if there are no tags or git fails — the caller decides on a
/// fallback (currently `HEAD~50` so something still gets emitted on a
/// fresh repo with no tags).
pub fn latest_tag(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if tag.is_empty() { None } else { Some(tag) }
}

/// Map basename → set of (sha) that touched it. Filenames inside
/// `.forgeplan/<kind_dir>/` only.
pub type Touched = BTreeMap<String, BTreeSet<String>>;

/// Walk `git log --name-only --pretty=%H since..until -- .forgeplan/{kinds}`
/// and group touched basenames with shas that mentioned them.
pub fn walk_artifact_changes(repo_root: &Path, since: &str, until: &str) -> Result<Touched> {
    // Allow-list of artifact kind dirs the changelog cares about. Notes,
    // memory, refresh deliberately excluded (they're filtered by classify
    // anyway, no point loading them).
    let kind_dirs = [
        ".forgeplan/prds/",
        ".forgeplan/problems/",
        ".forgeplan/evidence/",
        ".forgeplan/rfcs/",
        ".forgeplan/adrs/",
        ".forgeplan/specs/",
        ".forgeplan/epics/",
        ".forgeplan/solutions/",
    ];

    let range = format!("{since}..{until}");

    let mut args: Vec<String> = vec![
        "log".to_string(),
        "--name-only".to_string(),
        "--pretty=format:COMMIT %H".to_string(),
        "--diff-filter=AM".to_string(),
        range,
        "--".to_string(),
    ];
    for d in &kind_dirs {
        args.push((*d).to_string());
    }

    let output = Command::new("git")
        .args(&args)
        .current_dir(repo_root)
        .output()
        .with_context(|| "running `git log` — is git installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git log failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut touched: Touched = BTreeMap::new();
    let mut current_sha = String::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(sha) = line.strip_prefix("COMMIT ") {
            current_sha = sha.trim().to_string();
            continue;
        }
        if current_sha.is_empty() {
            continue;
        }
        if !line.ends_with(".md") {
            continue;
        }
        let Some(basename) = Path::new(line).file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        touched
            .entry(basename.to_string())
            .or_default()
            .insert(current_sha.clone());
    }

    Ok(touched)
}

/// Extract an id-or-slug from a `.forgeplan/<dir>/<file>.md` basename.
/// Two filename shapes per SPEC-005:
///   pre-merge:  `prd-auth-system.md` (slug)
///   post-merge: `PRD-074-auth-system.md`
pub fn id_from_basename(basename: &str) -> Option<String> {
    let stem = basename.strip_suffix(".md")?;
    // Post-merge form: `KIND-NNN-...`.
    if let Some((prefix, rest)) = stem.split_once('-')
        && let Some((digits, _)) = rest.split_once('-')
        && !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
        && prefix.chars().all(|c| c.is_ascii_alphabetic())
    {
        return Some(format!("{}-{}", prefix.to_uppercase(), digits));
    }
    // Post-merge form without trailing slug: `PRD-074.md`.
    if let Some((prefix, digits)) = stem.split_once('-')
        && !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
        && prefix.chars().all(|c| c.is_ascii_alphabetic())
    {
        return Some(format!("{}-{}", prefix.to_uppercase(), digits));
    }
    // Pre-merge slug form: pass through verbatim (lowercase).
    Some(stem.to_lowercase())
}

/// Check whether `record` or any of its 1-hop neighbours (in either
/// relation direction) carries tag `security`.
pub async fn is_linked_to_security(store: &LanceStore, record: &ArtifactRecord) -> Result<bool> {
    if record
        .tags
        .iter()
        .any(|t| t.eq_ignore_ascii_case("security"))
    {
        return Ok(true);
    }
    let outgoing = store.get_relations(&record.id).await.unwrap_or_default();
    for (target, _rel) in &outgoing {
        if let Ok(Some(t)) = store.get_record(target).await
            && t.tags.iter().any(|t| t.eq_ignore_ascii_case("security"))
        {
            return Ok(true);
        }
    }
    let incoming = store
        .get_incoming_relations(&record.id)
        .await
        .unwrap_or_default();
    for (source, _rel) in &incoming {
        if let Ok(Some(s)) = store.get_record(source).await
            && s.tags.iter().any(|t| t.eq_ignore_ascii_case("security"))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// For PROB / PRD / RFC etc., return an EvidencePack id that points to
/// this artifact (incoming relation from EVID is the canonical
/// attachment in our methodology — CLAUDE.md "EvidencePack" section).
pub async fn pick_closing_evid(
    store: &LanceStore,
    record: &ArtifactRecord,
) -> Result<Option<String>> {
    if record.kind == "evidence" {
        return Ok(None);
    }
    let incoming = store
        .get_incoming_relations(&record.id)
        .await
        .unwrap_or_default();
    for (source_id, _rel) in incoming {
        if source_id.starts_with("EVID-") || source_id.to_lowercase().starts_with("evid-") {
            return Ok(Some(source_id));
        }
    }
    Ok(None)
}

/// Build a [`ReleaseNotes`] payload by loading touched records via the
/// store and classifying each one. Resolution is by canonical id only —
/// records that no longer exist are silently dropped.
pub async fn build_release_notes(
    store: &LanceStore,
    touched: &Touched,
    since: &str,
    until: &str,
    draft: bool,
) -> Result<ReleaseNotes> {
    let mut notes = ReleaseNotes::new(since, until, draft);

    let mut seen_ids: BTreeSet<String> = BTreeSet::new();

    for (basename, shas) in touched {
        let Some(candidate_id) = id_from_basename(basename) else {
            continue;
        };
        let canonical = match store.resolve_id(&candidate_id).await {
            Ok(Some(c)) => c,
            _ => continue,
        };
        if !seen_ids.insert(canonical.clone()) {
            continue;
        }
        let record = match store.get_record(&canonical).await? {
            Some(r) => r,
            None => continue,
        };
        let security = is_linked_to_security(store, &record).await?;
        let class = classify(&record, security, draft);
        let cat = match class {
            Classification::Include(c) => c,
            _ => continue,
        };
        let evid = pick_closing_evid(store, &record).await?;
        let commit = shas
            .iter()
            .next()
            .map(|s| s.chars().take(7).collect::<String>())
            .unwrap_or_default();

        notes.push(
            cat,
            Entry {
                id: canonical.clone(),
                title: record.title.clone(),
                evid,
                commit,
                kind: record.kind.clone(),
            },
        );
    }

    Ok(notes)
}

/// End-to-end helper: resolve refs (validating user input), walk git,
/// load records, build notes. Returns the in-memory representation,
/// callers choose markdown/text/json themselves.
pub async fn generate(
    store: &LanceStore,
    repo_root: &Path,
    since: Option<&str>,
    until: Option<&str>,
    draft: bool,
) -> Result<ReleaseNotes> {
    let since_resolved = match since {
        Some(s) => {
            validate_git_ref(s)?;
            s.to_string()
        }
        None => latest_tag(repo_root).unwrap_or_else(|| "HEAD~50".to_string()),
    };
    let until_resolved = match until {
        Some(u) => {
            validate_git_ref(u)?;
            u.to_string()
        }
        None => "HEAD".to_string(),
    };

    let touched = walk_artifact_changes(repo_root, &since_resolved, &until_resolved)?;
    let notes =
        build_release_notes(store, &touched, &since_resolved, &until_resolved, draft).await?;
    Ok(notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(kind: &str, status: &str, r_eff: f64, tags: &[&str]) -> ArtifactRecord {
        ArtifactRecord {
            id: format!("{}-001", kind.to_uppercase()),
            kind: kind.to_string(),
            status: status.to_string(),
            title: format!("Test {kind}"),
            body: String::new(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            r_eff_score: r_eff,
            valid_until: None,
            created_at: "2026-01-01T00:00:00".to_string(),
            updated_at: "2026-01-01T00:00:00".to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            body_hash: None,
            embedding: None,
        }
    }

    #[test]
    fn prd_active_lands_in_added() {
        let r = rec("prd", "active", 0.0, &[]);
        assert_eq!(
            classify(&r, false, false),
            Classification::Include(Category::Added)
        );
    }

    #[test]
    fn prob_active_with_score_lands_in_fixed() {
        let r = rec("problem", "active", 0.8, &[]);
        assert_eq!(
            classify(&r, false, false),
            Classification::Include(Category::Fixed)
        );
    }

    #[test]
    fn rfc_adr_active_land_in_changed() {
        for kind in ["rfc", "adr", "spec", "epic", "solution"] {
            let r = rec(kind, "active", 0.5, &[]);
            assert_eq!(
                classify(&r, false, false),
                Classification::Include(Category::Changed),
                "kind={kind}"
            );
        }
    }

    #[test]
    fn evid_with_security_link_lands_in_security() {
        let r = rec("evidence", "active", 0.9, &[]);
        assert_eq!(
            classify(&r, true, false),
            Classification::Include(Category::Security)
        );
    }

    #[test]
    fn evid_without_security_link_lands_in_internal() {
        let r = rec("evidence", "active", 0.5, &[]);
        assert_eq!(
            classify(&r, false, false),
            Classification::Include(Category::Internal)
        );
    }

    #[test]
    fn note_memory_refresh_are_skipped() {
        for kind in ["note", "memory", "refresh"] {
            let r = rec(kind, "active", 1.0, &[]);
            assert_eq!(
                classify(&r, false, false),
                Classification::SkipKind,
                "kind={kind}"
            );
        }
    }

    #[test]
    fn quality_gate_skips_draft_without_signal() {
        // draft status + r_eff=0 + not active → filtered.
        let r = rec("prd", "draft", 0.0, &[]);
        assert_eq!(classify(&r, false, false), Classification::SkipQuality);
    }

    #[test]
    fn draft_flag_waives_quality_gate() {
        let r = rec("prd", "draft", 0.0, &[]);
        assert_eq!(
            classify(&r, false, true),
            Classification::Include(Category::Added)
        );
    }

    #[test]
    fn quality_gate_keeps_active_even_without_score() {
        // Many active artifacts have r_eff still being computed; status
        // alone counts as signal.
        let r = rec("prd", "active", 0.0, &[]);
        assert_eq!(
            classify(&r, false, false),
            Classification::Include(Category::Added)
        );
    }

    #[test]
    fn quality_gate_keeps_score_even_without_active_status() {
        let r = rec("problem", "deprecated", 0.6, &[]);
        assert_eq!(
            classify(&r, false, false),
            Classification::Include(Category::Fixed)
        );
    }

    #[test]
    fn release_notes_push_and_total() {
        let mut n = ReleaseNotes::new("v0.30.0", "HEAD", false);
        assert!(n.is_empty());
        n.push(
            Category::Added,
            Entry {
                id: "PRD-001".to_string(),
                title: "Auth".to_string(),
                evid: None,
                commit: "abc1234".to_string(),
                kind: "prd".to_string(),
            },
        );
        assert_eq!(n.total(), 1);
        assert!(!n.is_empty());
    }

    #[test]
    fn markdown_format_contains_sections() {
        let mut n = ReleaseNotes::new("v0.30.0", "HEAD", false);
        n.push(
            Category::Added,
            Entry {
                id: "PRD-074".to_string(),
                title: "Cache aside".to_string(),
                evid: Some("EVID-100".to_string()),
                commit: "deadbee".to_string(),
                kind: "prd".to_string(),
            },
        );
        n.push(
            Category::Fixed,
            Entry {
                id: "PROB-067".to_string(),
                title: "Counter race".to_string(),
                evid: Some("EVID-101".to_string()),
                commit: "feedfa".to_string(),
                kind: "problem".to_string(),
            },
        );
        let md = format_markdown(&n);
        assert!(md.contains("## [v0.30.0 → HEAD]"));
        assert!(md.contains("### Added"));
        assert!(md.contains("### Fixed"));
        assert!(md.contains("PRD-074"));
        assert!(md.contains("EVID-100"));
        assert!(md.contains("commit deadbee"));
    }

    #[test]
    fn markdown_empty_renders_friendly_marker() {
        let n = ReleaseNotes::new("v0.30.0", "HEAD", false);
        let md = format_markdown(&n);
        assert!(md.contains("No artifacts matched"));
    }

    #[test]
    fn markdown_draft_renders_banner() {
        let n = ReleaseNotes::new("v0.30.0", "HEAD", true);
        let md = format_markdown(&n);
        assert!(md.contains("Draft mode"));
    }

    #[test]
    fn json_format_has_all_section_keys() {
        let n = ReleaseNotes::new("v0.30.0", "HEAD", false);
        let val = format_json(&n);
        assert_eq!(val["since"], "v0.30.0");
        assert_eq!(val["until"], "HEAD");
        assert_eq!(val["draft"], false);
        assert_eq!(val["total"], 0);
        for key in ["added", "fixed", "security", "changed", "internal"] {
            assert!(val.get(key).is_some(), "missing key {key}");
            assert!(val[key].is_array(), "{key} is not array");
        }
    }

    #[test]
    fn id_from_basename_post_merge_form() {
        assert_eq!(
            id_from_basename("PRD-074-auth-system.md").as_deref(),
            Some("PRD-074")
        );
        assert_eq!(
            id_from_basename("EVID-101-foo.md").as_deref(),
            Some("EVID-101")
        );
        assert_eq!(
            id_from_basename("PROB-009-bar.md").as_deref(),
            Some("PROB-009")
        );
    }

    #[test]
    fn id_from_basename_post_merge_without_suffix() {
        assert_eq!(id_from_basename("PRD-074.md").as_deref(), Some("PRD-074"));
    }

    #[test]
    fn id_from_basename_pre_merge_slug() {
        assert_eq!(
            id_from_basename("prd-auth-system.md").as_deref(),
            Some("prd-auth-system")
        );
        assert_eq!(
            id_from_basename("evid-cache-aside.md").as_deref(),
            Some("evid-cache-aside")
        );
    }

    #[test]
    fn id_from_basename_returns_none_for_garbage() {
        assert_eq!(id_from_basename(""), None);
        assert_eq!(id_from_basename("missing-extension"), None);
    }

    #[test]
    fn text_format_no_markdown_chars() {
        let mut n = ReleaseNotes::new("v0.30.0", "HEAD", false);
        n.push(
            Category::Added,
            Entry {
                id: "PRD-074".to_string(),
                title: "X".to_string(),
                evid: None,
                commit: String::new(),
                kind: "prd".to_string(),
            },
        );
        let t = format_text(&n);
        assert!(!t.contains("##"));
        assert!(!t.contains("###"));
        assert!(t.contains("Added:"));
    }
}
