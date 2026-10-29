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

use crate::constellation::workspace_read::WorkspaceRead;
use crate::db::store::{ArtifactRecord, LanceStore};
use crate::git::validate_git_ref;

/// Where the records a release note is built from come from.
///
/// The legacy Lance projection or one captured/pinned read view. Both answer
/// the same four questions — resolve a basename to a canonical id, load one
/// record, load its outgoing edges, load its incoming edges — so the note
/// building below runs once for both layouts instead of being written twice
/// and left to disagree.
enum Records<'a> {
    Store(&'a LanceStore),
    Read(&'a WorkspaceRead),
}

impl Records<'_> {
    /// Canonical display id for a candidate, or `None` when it resolves to
    /// nothing in this source.
    async fn resolve_id(&self, candidate: &str) -> Result<Option<String>> {
        match self {
            Self::Store(store) => store.resolve_id(candidate).await,
            Self::Read(read) => Ok(read.resolve_id(candidate)),
        }
    }

    /// One record by canonical id, or `None`.
    async fn record(&self, id: &str) -> Result<Option<ArtifactRecord>> {
        match self {
            Self::Store(store) => store.get_record(id).await,
            Self::Read(read) => Ok(read.get_record(id).cloned()),
        }
    }

    /// Whether the artifact at `id` carries tag `security`.
    ///
    /// Separate from [`Self::record`] because the neighbour probe only reads
    /// tags: on the captured path that probe must not clone a whole record
    /// body per 1-hop neighbour just to look at its tag list. The legacy
    /// path has no cheaper accessor to route through — its `get_record`
    /// already materialises the row — so both sides answer identically and
    /// only the captured side stops paying for the clone.
    async fn has_security_tag(&self, id: &str) -> Result<bool> {
        Ok(match self {
            Self::Store(store) => store
                .get_record(id)
                .await?
                .is_some_and(|r| r.tags.iter().any(|t| t.eq_ignore_ascii_case("security"))),
            Self::Read(read) => read
                .get_record(id)
                .is_some_and(|r| r.tags.iter().any(|t| t.eq_ignore_ascii_case("security"))),
        })
    }

    /// Outgoing `(target, relation)` edges of one artifact.
    async fn outgoing(&self, id: &str) -> Result<Vec<(String, String)>> {
        match self {
            Self::Store(store) => store.get_relations(id).await,
            Self::Read(read) => Ok(read.get_relations(id)),
        }
    }

    /// Incoming `(source, relation)` edges of one artifact.
    async fn incoming(&self, id: &str) -> Result<Vec<(String, String)>> {
        match self {
            Self::Store(store) => store.get_incoming_relations(id).await,
            Self::Read(read) => Ok(read.get_incoming_relations(id)),
        }
    }
}

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

/// Issue #290: locate the nearest git repository for changelog generation.
///
/// Some workspaces split `.forgeplan/` and `.git/` into different
/// directories (the marketplace pattern: `.forgeplan/` at parent,
/// `.git/` in a child submodule-style repo). Without discovery,
/// `git log` runs from the workspace root and reports
/// "not a git repository".
///
/// Strategy (audit-r-dogfood SEC-1 hardening):
/// 1. If `start/.git` exists → use `start` (standard layout).
/// 2. Walk **up** from `start`, **bounded** by the nearest `.forgeplan/`
///    ancestor (don't escape workspace) or 8 hops (whichever comes
///    first) — prevents reaching `/Users/<other>/.git` on multi-user
///    systems.
/// 3. Walk **down** one level into immediate subdirectories of `start`
///    looking for `.git/`. Symlinks are explicitly rejected via
///    `symlink_metadata().is_dir()` (not the symlink-following
///    `is_dir()`) so an attacker can't plant a symlink at e.g.
///    `subdir/.git -> /private/var/.../other-user-repo/.git` to make
///    `release_notes` emit other repos' commit messages.
///    Deterministic order: entries sorted by path (issue #290 follow-up
///    CODE-8 / SEC-1).
/// 4. Fall back to `start` (preserves legacy behaviour — caller still
///    gets the original "not a git repository" error if no `.git` is
///    found anywhere).
///
/// Returns the directory to use as `current_dir` for git invocations.
pub fn find_git_root(start: &Path) -> std::path::PathBuf {
    /// Probe `dir/.git` rejecting symlinks (SEC-1 fix).
    /// `dir/.git` is allowed to be a regular file (git worktree, submodule
    /// pointer) — only symlinks are rejected because they're the vector
    /// for attacker-redirected repo discovery.
    fn has_real_dot_git(dir: &Path) -> bool {
        let dot_git = dir.join(".git");
        match std::fs::symlink_metadata(&dot_git) {
            Ok(md) => !md.file_type().is_symlink(),
            Err(_) => false,
        }
    }

    // 1 + 2. Walk up, bounded.
    //
    // Boundary: stop walking when we either (a) find a `.forgeplan/`
    // ancestor (that's the workspace edge, and the operator's repo
    // should be inside it or co-located), or (b) traverse > MAX_HOPS
    // (defensive against pathological deep nesting / fs loops).
    const MAX_HOPS: usize = 8;
    let mut current: Option<&Path> = Some(start);
    let mut hops = 0;
    while let Some(dir) = current {
        if has_real_dot_git(dir) {
            return dir.to_path_buf();
        }
        // Stop at the workspace boundary marker (`.forgeplan/`).
        if dir.join(".forgeplan").exists() && dir != start {
            break;
        }
        hops += 1;
        if hops >= MAX_HOPS {
            break;
        }
        current = dir.parent();
    }
    // 3. Walk down one level, symlink-safe, deterministic order.
    if let Ok(entries) = std::fs::read_dir(start) {
        let mut candidates: Vec<std::path::PathBuf> = entries
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                // Reject symlinks even on the directory itself.
                let md = std::fs::symlink_metadata(&p).ok()?;
                if md.file_type().is_dir() && has_real_dot_git(&p) {
                    Some(p)
                } else {
                    None
                }
            })
            .collect();
        candidates.sort();
        if let Some(p) = candidates.into_iter().next() {
            return p;
        }
    }
    // 4. Fallback (legacy behaviour).
    start.to_path_buf()
}

/// Try `git describe --tags --abbrev=0` to find the latest tag. Returns
/// `None` if there are no tags or git fails — the caller decides on a
/// fallback (currently `HEAD~50` so something still gets emitted on a
/// fresh repo with no tags).
pub fn latest_tag(repo_root: &Path) -> Option<String> {
    // Issue #290: resolve to nearest .git before invoking git.
    let git_root = find_git_root(repo_root);
    let output = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .current_dir(&git_root)
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

    // Issue #290: resolve to nearest .git before invoking git.
    let git_root = find_git_root(repo_root);
    let output = Command::new("git")
        .args(&args)
        .current_dir(&git_root)
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
    is_linked_to_security_in(&Records::Store(store), record).await
}

async fn is_linked_to_security_in(records: &Records<'_>, record: &ArtifactRecord) -> Result<bool> {
    if record
        .tags
        .iter()
        .any(|t| t.eq_ignore_ascii_case("security"))
    {
        return Ok(true);
    }
    let outgoing = records.outgoing(&record.id).await.unwrap_or_default();
    for (target, _rel) in &outgoing {
        // Tag-only probe: the neighbour's body is never read here.
        if records.has_security_tag(target).await.unwrap_or(false) {
            return Ok(true);
        }
    }
    let incoming = records.incoming(&record.id).await.unwrap_or_default();
    for (source, _rel) in &incoming {
        if records.has_security_tag(source).await.unwrap_or(false) {
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
    pick_closing_evid_in(&Records::Store(store), record).await
}

async fn pick_closing_evid_in(
    records: &Records<'_>,
    record: &ArtifactRecord,
) -> Result<Option<String>> {
    if record.kind == "evidence" {
        return Ok(None);
    }
    let incoming = records.incoming(&record.id).await.unwrap_or_default();
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
    build_release_notes_in(&Records::Store(store), touched, since, until, draft).await
}

/// [`build_release_notes`] over one captured/pinned read view.
///
/// Git still answers "which artifacts did this range touch" from the same
/// repository; only the record lookup moves to the capture, so a migrated
/// workspace gets the same categories, the same closing-evidence picks and
/// the same security classification without opening a legacy table.
async fn build_release_notes_in(
    records: &Records<'_>,
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
        let canonical = match records.resolve_id(&candidate_id).await {
            Ok(Some(c)) => c,
            _ => continue,
        };
        if !seen_ids.insert(canonical.clone()) {
            continue;
        }
        let record = match records.record(&canonical).await? {
            Some(r) => r,
            None => continue,
        };
        let security = is_linked_to_security_in(records, &record).await?;
        let class = classify(&record, security, draft);
        let cat = match class {
            Classification::Include(c) => c,
            _ => continue,
        };
        let evid = pick_closing_evid_in(records, &record).await?;
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

/// End-to-end release notes over one captured/pinned read view.
///
/// The same ref resolution, the same `git log` walk over the same
/// repository and the same classification as [`generate`] — only the record
/// lookup reads from this capture. A migrated workspace therefore produces
/// the same sections, entries, commit shas and closing-evidence picks, and
/// does so without opening the legacy projection.
pub async fn generate_captured(
    read: &WorkspaceRead,
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
    let notes = build_release_notes_in(
        &Records::Read(read),
        &touched,
        &since_resolved,
        &until_resolved,
        draft,
    )
    .await?;
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

    // ─── Issue #290: find_git_root discovery ──────────────────────────────

    use tempfile::TempDir;

    /// Standard layout: `.git/` directly inside the start directory.
    #[test]
    fn find_git_root_standard_layout() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        assert_eq!(find_git_root(tmp.path()), tmp.path().to_path_buf());
    }

    /// Walk-up: `.git/` in a parent directory, start in a subdir.
    #[test]
    fn find_git_root_walks_up_to_parent() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        let sub = tmp.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        assert_eq!(find_git_root(&sub), tmp.path().to_path_buf());
    }

    /// Issue #290 reproduction: `.git/` in a CHILD subdirectory, NOT in
    /// any parent. Start dir is the workspace root containing `.forgeplan/`.
    /// The discovery must walk DOWN one level to find the repo.
    #[test]
    fn find_git_root_walks_down_for_split_repo() {
        let tmp = TempDir::new().unwrap();
        // .forgeplan/ at root (no .git here)
        std::fs::create_dir(tmp.path().join(".forgeplan")).unwrap();
        // .git/ in child repo subdir
        let repo = tmp.path().join("forgeplan-marketplace");
        std::fs::create_dir(&repo).unwrap();
        std::fs::create_dir(repo.join(".git")).unwrap();

        let resolved = find_git_root(tmp.path());
        assert_eq!(
            resolved, repo,
            "issue #290: .git/ in child subdir must be discovered (not fallback to start)"
        );
    }

    /// No git anywhere → falls back to start (legacy behaviour, caller
    /// gets the same "not a git repository" error as before).
    #[test]
    fn find_git_root_fallback_when_no_git() {
        let tmp = TempDir::new().unwrap();
        let resolved = find_git_root(tmp.path());
        // tmp.path() may walk up to a real .git/ on dev machines (cargo
        // workspace root); assert resolved is either start OR an ancestor.
        assert!(
            resolved == tmp.path() || tmp.path().starts_with(&resolved),
            "fallback or walked up to ancestor .git/: {resolved:?}"
        );
    }

    /// Prefer walk-up over walk-down: if both ancestor AND child have
    /// `.git/`, the ancestor (standard) wins because step 2 runs before
    /// step 3.
    #[test]
    fn find_git_root_prefers_ancestor_over_child() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        let child = tmp.path().join("child");
        std::fs::create_dir(&child).unwrap();
        std::fs::create_dir(child.join(".git")).unwrap();
        // Start = tmp.path() itself — should return tmp.path() (has .git).
        assert_eq!(find_git_root(tmp.path()), tmp.path().to_path_buf());
    }

    /// Audit-r-dogfood SEC-1: walk-down MUST reject symlinks at the
    /// `.git` entry. Without this, an attacker who can write a subdir
    /// in the workspace can plant a symlinked `.git` pointing at another
    /// user's repo, causing `forgeplan_release_notes` to emit commit
    /// messages from that other repo.
    #[cfg(unix)]
    #[test]
    fn find_git_root_rejects_symlinked_dot_git_in_subdir() {
        use std::os::unix::fs::symlink;
        let tmp = TempDir::new().unwrap();
        // `.forgeplan/` at root marks workspace boundary so walk-up
        // doesn't escape into the cargo workspace's `.git/`.
        std::fs::create_dir(tmp.path().join(".forgeplan")).unwrap();
        // Plant a real repo "somewhere else" — simulates another user's repo.
        let other_repo = tmp.path().join("other-user-repo");
        std::fs::create_dir(&other_repo).unwrap();
        std::fs::create_dir(other_repo.join(".git")).unwrap();
        // Attacker plants a child dir with a SYMLINKED `.git/` pointing
        // at the other-user repo.
        let child = tmp.path().join("evil-subdir");
        std::fs::create_dir(&child).unwrap();
        symlink(other_repo.join(".git"), child.join(".git")).unwrap();

        let resolved = find_git_root(tmp.path());
        assert_ne!(
            resolved, child,
            "SEC-1: symlinked .git must NOT be selected — got {resolved:?}"
        );
    }

    /// Walk-up must stop at `.forgeplan/` boundary so it can't escape
    /// the workspace and reach `/Users/<other>/.git` on multi-user host.
    #[test]
    fn find_git_root_walk_up_bounded_by_forgeplan_marker() {
        let tmp = TempDir::new().unwrap();
        // No .git at tmp, but .forgeplan/ here = workspace boundary.
        std::fs::create_dir(tmp.path().join(".forgeplan")).unwrap();
        let sub = tmp.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        // Subdir has no .git either. find_git_root from sub must NOT
        // walk past tmp.path() (which has .forgeplan) into cargo workspace's
        // real .git on the dev machine.
        let resolved = find_git_root(&sub);
        // Should fall back to `start` (sub) since no .git found within bound.
        // Critical: must NOT return some ancestor outside the workspace.
        assert!(
            resolved == sub || resolved == tmp.path(),
            "walk-up must stay within workspace bound: got {resolved:?}"
        );
    }

    /// Walk-down is deterministic when multiple subdirs have `.git/`
    /// (issue #290 CODE-8 / SEC-1).
    #[test]
    fn find_git_root_walk_down_deterministic_order() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".forgeplan")).unwrap();
        for name in ["zeta-repo", "alpha-repo", "middle-repo"] {
            let dir = tmp.path().join(name);
            std::fs::create_dir(&dir).unwrap();
            std::fs::create_dir(dir.join(".git")).unwrap();
        }
        let resolved = find_git_root(tmp.path());
        assert_eq!(
            resolved.file_name().and_then(|n| n.to_str()),
            Some("alpha-repo"),
            "alphabetic order: 'alpha' must win, not fs::read_dir() insertion order — got {resolved:?}"
        );
    }

    // ── Captured read view ────────────────────────────────────────────

    use std::path::PathBuf;
    use std::process::Command;

    fn git(repo: &std::path::Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    struct ArtifactFixture<'a> {
        id: &'a str,
        kind: &'a str,
        status: &'a str,
        tags: &'a [&'a str],
        links: &'a [(&'a str, &'a str)],
    }

    fn write_artifact(
        ws: &std::path::Path,
        dir: &str,
        file: &str,
        fixture: ArtifactFixture<'_>,
    ) -> PathBuf {
        let ArtifactFixture {
            id,
            kind,
            status,
            tags,
            links,
        } = fixture;
        let mut fm = format!(
            "---\nid: {id}\ntitle: Fixture {id}\nkind: {kind}\nstatus: {status}\n\
             depth: standard\ncreated_at: 2026-01-01T00:00:00Z\n"
        );
        if !tags.is_empty() {
            fm.push_str("tags:\n");
            for tag in tags {
                fm.push_str(&format!("  - {tag}\n"));
            }
        }
        if !links.is_empty() {
            fm.push_str("links:\n");
            for (target, relation) in links {
                fm.push_str(&format!("  - target: {target}\n    relation: {relation}\n"));
            }
        }
        let path = ws.join(dir).join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            format!("{fm}---\n\n## Context\n\nRelease-notes fixture body.\n"),
        )
        .unwrap();
        path
    }

    /// A tagged release with two commits: `v0.1.0` holds the original
    /// decisions, and the second commit edits two ADRs and adds a PRD and
    /// two evidence packs — the range a release note is built from.
    fn release_repo() -> (TempDir, PathBuf, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q", "--initial-branch=main"]);
        git(&repo, &["config", "user.email", "fixture@example.test"]);
        git(&repo, &["config", "user.name", "Release Fixture"]);

        let ws = repo.join(".forgeplan");
        for dir in crate::workspace::ARTIFACT_DIRS {
            std::fs::create_dir_all(ws.join(dir)).unwrap();
        }

        let adr1 = write_artifact(
            &ws,
            "adrs",
            "ADR-001-first.md",
            ArtifactFixture {
                id: "ADR-001",
                kind: "adr",
                status: "active",
                tags: &[],
                links: &[],
            },
        );
        let adr2 = write_artifact(
            &ws,
            "adrs",
            "ADR-002-security.md",
            ArtifactFixture {
                id: "ADR-002",
                kind: "adr",
                status: "active",
                tags: &["security"],
                links: &[],
            },
        );
        let evid2 = write_artifact(
            &ws,
            "evidence",
            "EVID-002-pack.md",
            ArtifactFixture {
                id: "EVID-002",
                kind: "evidence",
                status: "active",
                tags: &[],
                links: &[("ADR-002", "informs")],
            },
        );
        write_artifact(
            &ws,
            "prds",
            "PRD-001-first.md",
            ArtifactFixture {
                id: "PRD-001",
                kind: "prd",
                status: "active",
                tags: &[],
                links: &[],
            },
        );
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-q", "-m", "v0.1.0"]);
        git(&repo, &["tag", "v0.1.0"]);

        // Second commit: edits both ADRs and the pack that informs the
        // security-tagged one, and adds an active PRD, an unevidenced draft
        // PRD and a fresh pack — the range a release note is built from.
        for path in [&adr1, &adr2, &evid2] {
            let mut body = std::fs::read_to_string(path).unwrap();
            body.push_str("\nEdited in the second release commit.\n");
            std::fs::write(path, body).unwrap();
        }
        // Never active and never scored: the quality gate drops it unless
        // the caller asks for a draft.
        write_artifact(
            &ws,
            "prds",
            "PRD-002-second.md",
            ArtifactFixture {
                id: "PRD-002",
                kind: "prd",
                status: "draft",
                tags: &[],
                links: &[],
            },
        );
        write_artifact(
            &ws,
            "prds",
            "PRD-003-second.md",
            ArtifactFixture {
                id: "PRD-003",
                kind: "prd",
                status: "active",
                tags: &[],
                links: &[],
            },
        );
        write_artifact(
            &ws,
            "evidence",
            "EVID-003-pack.md",
            ArtifactFixture {
                id: "EVID-003",
                kind: "evidence",
                status: "active",
                tags: &[],
                links: &[],
            },
        );
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-q", "-m", "second release"]);

        (tmp, repo, ws)
    }

    async fn open_read(ws: &std::path::Path) -> WorkspaceRead {
        crate::constellation::facade::open_workspace_read(
            ws,
            None,
            crate::constellation::capture::CaptureLimits::default(),
        )
        .await
        .expect("the fixture workspace captures completely")
    }

    async fn legacy_store_for(read: &WorkspaceRead) -> LanceStore {
        let store = LanceStore::init(read.workspace()).await.unwrap();
        // From `read.records()` (scores recomputed by `from_parts`), not
        // from `record_from_parsed` whose output is 0.0 for every record —
        // `classify`'s quality gate reads that score directly.
        for record in read.records() {
            store
                .create_artifact(&crate::db::store::NewArtifact {
                    id: record.id.clone(),
                    kind: record.kind.clone(),
                    status: record.status.clone(),
                    title: record.title.clone(),
                    body: record.body.clone(),
                    depth: record.depth.clone(),
                    author: record.author.clone(),
                    parent_epic: record.parent_epic.clone(),
                    valid_until: record.valid_until.clone(),
                    tags: record.tags.clone(),
                })
                .await
                .unwrap();
            store.replace_record(record).await.unwrap();
        }
        for (source, target, relation) in read.get_all_relations() {
            store
                .add_relation(&source, &target, &relation)
                .await
                .unwrap();
        }
        store
    }

    fn ids_in(notes: &ReleaseNotes, category: Category) -> Vec<String> {
        notes
            .sections
            .get(category.json_key())
            .map(|entries| entries.iter().map(|e| e.id.clone()).collect())
            .unwrap_or_default()
    }

    #[tokio::test]
    async fn captured_release_notes_classify_the_git_range_over_the_capture() {
        let (_tmp, repo, ws) = release_repo();
        let read = open_read(&ws).await;

        let notes = generate_captured(&read, &repo, Some("v0.1.0"), Some("HEAD"), false)
            .await
            .unwrap();

        assert_eq!(notes.since, "v0.1.0");
        assert_eq!(notes.until, "HEAD");
        assert!(!notes.draft);

        // Both edited ADRs land under Changed, in basename order.
        assert_eq!(
            ids_in(&notes, Category::Changed),
            vec!["ADR-001", "ADR-002"]
        );
        // EVID-002 links to a security-tagged decision, so it is Security,
        // not Internal; EVID-003 has no such neighbour and stays Internal.
        assert_eq!(ids_in(&notes, Category::Security), vec!["EVID-002"]);
        assert_eq!(ids_in(&notes, Category::Internal), vec!["EVID-003"]);
        // The new active PRD is Added; the draft one is gated out.
        assert_eq!(ids_in(&notes, Category::Added), vec!["PRD-003"]);
        assert!(notes.sections.get("fixed").unwrap().is_empty());

        // ADR-002 has an incoming evidence pack, so the note carries it.
        let adr2 = notes
            .sections
            .get("changed")
            .unwrap()
            .iter()
            .find(|e| e.id == "ADR-002")
            .unwrap();
        assert_eq!(adr2.evid.as_deref(), Some("EVID-002"));
        assert_eq!(adr2.commit.len(), 7, "the touching commit is shortened");
        assert_eq!(adr2.kind, "adr");

        // ADR-001 was edited in the same commit and carries no evidence.
        let adr1 = notes
            .sections
            .get("changed")
            .unwrap()
            .iter()
            .find(|e| e.id == "ADR-001")
            .unwrap();
        assert_eq!(adr1.evid, None);
        assert_eq!(adr1.commit, adr2.commit);
    }

    #[tokio::test]
    async fn captured_release_notes_draft_flag_waives_the_quality_gate() {
        let (_tmp, repo, ws) = release_repo();
        let read = open_read(&ws).await;

        let notes = generate_captured(&read, &repo, Some("v0.1.0"), Some("HEAD"), true)
            .await
            .unwrap();

        assert!(notes.draft);
        // PRD-002 is only reachable in draft mode: the release gate drops
        // artifacts with neither an active status nor a positive score.
        assert_eq!(ids_in(&notes, Category::Added), vec!["PRD-002", "PRD-003"]);
    }

    #[tokio::test]
    async fn captured_release_notes_default_to_the_latest_tag_and_head() {
        let (_tmp, repo, ws) = release_repo();
        let read = open_read(&ws).await;

        let notes = generate_captured(&read, &repo, None, None, false)
            .await
            .unwrap();

        // `since: None` resolves through `git describe --tags --abbrev=0`,
        // not a hardcoded range.
        assert_eq!(notes.since, "v0.1.0");
        assert_eq!(notes.until, "HEAD");
        assert!(
            notes.total() > 0,
            "the tagged range must resolve to entries"
        );
    }

    #[tokio::test]
    async fn captured_release_notes_reject_an_injected_ref() {
        let (_tmp, repo, ws) = release_repo();
        let read = open_read(&ws).await;

        let err = generate_captured(&read, &repo, Some("v0.1.0; rm -rf /"), None, false)
            .await
            .expect_err("a shell fragment is not a git ref");
        assert!(
            format!("{err}").contains("git"),
            "the validation error must name the ref contract, got: {err}"
        );
    }

    #[tokio::test]
    async fn captured_release_notes_match_the_legacy_projection_entry_for_entry() {
        let (_tmp, repo, ws) = release_repo();
        let read = open_read(&ws).await;
        let store = legacy_store_for(&read).await;

        for draft in [false, true] {
            let legacy = generate(&store, &repo, Some("v0.1.0"), Some("HEAD"), draft)
                .await
                .unwrap();
            let captured = generate_captured(&read, &repo, Some("v0.1.0"), Some("HEAD"), draft)
                .await
                .unwrap();

            assert_eq!(
                (&legacy.since, &legacy.until, legacy.draft),
                (&captured.since, &captured.until, captured.draft)
            );
            assert_eq!(legacy.total(), captured.total(), "draft={draft}");
            for category in Category::ordered() {
                assert_eq!(
                    ids_in(&legacy, category),
                    ids_in(&captured, category),
                    "category={} draft={draft}",
                    category.json_key()
                );
            }
            let legacy_changed = legacy.sections.get("changed").unwrap();
            let captured_changed = captured.sections.get("changed").unwrap();
            for (l, c) in legacy_changed.iter().zip(captured_changed) {
                assert_eq!(
                    (
                        l.id.as_str(),
                        l.title.as_str(),
                        l.kind.as_str(),
                        l.commit.as_str(),
                        l.evid.as_deref()
                    ),
                    (
                        c.id.as_str(),
                        c.title.as_str(),
                        c.kind.as_str(),
                        c.commit.as_str(),
                        c.evid.as_deref()
                    )
                );
            }
        }
    }
}
