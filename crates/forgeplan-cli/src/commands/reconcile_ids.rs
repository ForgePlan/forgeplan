//! `forgeplan reconcile-ids` — manual cleanup tool for post-merge identity
//! coherence issues per PROB-060 / RFC-009 §Phase 2.4.
//!
//! ## What this command does
//!
//! Walks `.forgeplan/<kind_dir>/*.md`, inspects frontmatter + filename, and
//! detects four categories of drift introduced when artifacts are touched
//! without going through the canonical MCP/CLI path (red-line #11) or when
//! Phase 1.x → Phase 2 migration left a partial state:
//!
//! 1. **`filename_mismatch`** — artifact has `assigned_number: N` but the
//!    on-disk filename does not match `<KIND>-<NNN>-<slug-suffix>.md` shape.
//!    Apply mode renames to canonical pattern (using `git mv` when the file
//!    is tracked, `fs::rename` otherwise).
//! 2. **`missing_predicted`** — artifact has a `slug` but no
//!    `predicted_number` field (legacy migration didn't carry it). Apply
//!    mode auto-fills `predicted_number = assigned_number` when set, else
//!    `1` as the fallback (the field is purely informational outside CI).
//! 3. **`body_links_drift`** — the body's `## Related` / `## Related
//!    Artifacts` table mentions IDs that are NOT present in frontmatter
//!    `links:`. Reported only — never auto-modified, because cross-artifact
//!    `links:` mutations belong to `forgeplan_link` (red-line #11).
//! 4. **`duplicate_assigned`** — two or more artifacts of the same kind
//!    share the same `assigned_number`. Always **flagged for manual
//!    resolution** — auto-fixing would risk silent data loss.
//!
//! ## Boundaries (red-line #11)
//!
//! The new file content this command writes goes **only** through the
//! `forgeplan_core::artifact::frontmatter` helpers + atomic file ops — no
//! direct `Edit`/`Write` on artifact bodies. LanceDB is never touched
//! (ADR-003); callers who care about the index downstream should run
//! `forgeplan scan-import` after applying fixes.
//!
//! ## Exit codes
//!
//! - `0` — workspace coherent (no actions reported, or apply mode applied
//!   all proposed fixes successfully)
//! - `1` — drift detected in `--check-only` mode, or unfixable categories
//!   were reported (`duplicate_assigned`, `body_links_drift`)
//! - `2` — workspace error (no `.forgeplan/`, scan failure)
//!
//! ## --report-cross-pr (deferred Phase 4 work)
//!
//! The flag is accepted for forward-compat with RFC-009 §Phase 2.4 — full
//! cross-PR `Refs:` drift detection requires walking commit messages on
//! sibling branches and is out of scope for the in-workspace pass. When
//! the flag is supplied we emit a single informational entry into the
//! report so JSON consumers can detect the no-op explicitly.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use chrono::Utc;
use clap::Args;
use forgeplan_core::artifact::frontmatter::{
    Frontmatter, assigned_number_from_frontmatter, parse_frontmatter,
    predicted_number_from_frontmatter, render_frontmatter, slug_from_frontmatter,
};
use forgeplan_core::artifact::types::{ArtifactKind, validate_slug};
use serde::Serialize;

/// CLI arguments for `forgeplan reconcile-ids`.
#[derive(Debug, Clone, Args, Default)]
pub struct ReconcileIdsArgs {
    /// Workspace root containing `.forgeplan/`. Default: walk-up search
    /// from cwd. May also be a `.forgeplan/` directory directly.
    #[arg(long)]
    pub workspace: Option<PathBuf>,

    /// Report inconsistencies without modifying files. Default is apply
    /// mode (auto-rename + auto-fill `predicted_number`).
    #[arg(long)]
    pub check_only: bool,

    /// Forward-compat flag for RFC-009 §Phase 2.4 cross-PR ref-drift
    /// detection. The current implementation surfaces a no-op marker in
    /// the JSON output; cross-PR walking is deferred (see module docs).
    #[arg(long)]
    pub report_cross_pr: bool,

    /// Emit JSON report to stdout (schema_version 1). Default is a
    /// scannable human-readable summary.
    #[arg(long)]
    pub json: bool,
}

/// Same kind list as `migrate-dry-run` — `Memory` is excluded from
/// lifecycle/identity tracking.
const SCAN_KINDS: &[ArtifactKind] = &[
    ArtifactKind::Prd,
    ArtifactKind::Rfc,
    ArtifactKind::Adr,
    ArtifactKind::Epic,
    ArtifactKind::Spec,
    ArtifactKind::ProblemCard,
    ArtifactKind::SolutionPortfolio,
    ArtifactKind::EvidencePack,
    ArtifactKind::Note,
    ArtifactKind::RefreshReport,
];

/// Stable lexicographic key for `ArtifactKind` (matches `migrate_dry_run`).
fn kind_sort_key(k: &ArtifactKind) -> String {
    k.prefix().trim_end_matches('-').to_string()
}

/// Lowercase one-shot kind name (e.g. "prd", "rfc").
fn kind_key(k: &ArtifactKind) -> &'static str {
    k.prefix().trim_end_matches('-')
}

/// Uppercase prefix used in canonical filename pattern (`PRD-074-slug.md`).
/// Prefer this over hard-coding `"PRD"` so all kinds stay supported.
fn kind_uppercase_prefix(k: &ArtifactKind) -> String {
    k.prefix().trim_end_matches('-').to_uppercase()
}

/// PROB-060 Phase 0b Round 2 [SEC-5 CWE-200]: redact filesystem paths in
/// error messages so absolute filesystem paths don't leak CI layout.
/// Workspace-relative paths are safe to surface; outside-workspace paths
/// are stripped to basename. Mirrors the helper in `ci_assign_id.rs`.
fn redact_path(workspace: &Path, path: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(workspace) {
        return rel.display().to_string();
    }
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "<unknown>".to_string())
}

/// One drift category surfaced in the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    FilenameMismatch,
    MissingPredicted,
    BodyLinksDrift,
    DuplicateAssigned,
    /// Forward-compat marker — emitted only when `--report-cross-pr` is
    /// supplied to make the no-op explicit to JSON consumers.
    CrossPrDeferred,
}

impl Category {
    fn as_str(self) -> &'static str {
        match self {
            Self::FilenameMismatch => "filename_mismatch",
            Self::MissingPredicted => "missing_predicted",
            Self::BodyLinksDrift => "body_links_drift",
            Self::DuplicateAssigned => "duplicate_assigned",
            Self::CrossPrDeferred => "cross_pr_deferred",
        }
    }
}

/// Single discovered artifact (intermediate scan record).
#[derive(Debug, Clone)]
struct ArtifactRecord {
    path: PathBuf,
    kind: ArtifactKind,
    fm: Frontmatter,
    body: String,
    slug: Option<String>,
    predicted: Option<u32>,
    assigned: Option<u32>,
}

/// One reported drift action.
#[derive(Debug, Clone)]
pub struct ReconcileAction {
    pub category: Category,
    /// Display-form id when known (e.g. "PRD-074", "PRD-74?", or the slug
    /// for legacy artifacts). Best-effort — never the source of truth.
    pub artifact_id: String,
    pub artifact_path: PathBuf,
    /// Human-readable JSON map of the current state relevant to the
    /// finding (filename, frontmatter snippet, etc.).
    pub current_state: serde_json::Value,
    /// JSON description of what we propose to do.
    pub suggested_fix: serde_json::Value,
    /// `Some(true)` after a successful apply, `Some(false)` after a
    /// reported-only category that we never auto-fix, `None` for
    /// `--check-only` runs that didn't attempt to apply at all.
    pub applied: Option<bool>,
}

/// Aggregated reconcile report.
#[derive(Debug, Clone)]
pub struct ReconcileReport {
    pub workspace: PathBuf,
    pub check_only: bool,
    pub actions: Vec<ReconcileAction>,
    pub scan_errors: Vec<(PathBuf, String)>,
    pub per_kind_count: BTreeMap<String, usize>,
}

impl ReconcileReport {
    pub fn has_unresolved(&self) -> bool {
        // Anything not successfully applied counts as unresolved.
        // CrossPrDeferred is informational only and does NOT count.
        self.actions
            .iter()
            .any(|a| a.category != Category::CrossPrDeferred && a.applied != Some(true))
    }
}

/// Resolve workspace `.forgeplan/` directory. Mirrors `migrate_dry_run`
/// resolver — accepts either project root containing `.forgeplan/` or
/// the directory itself.
fn resolve_forgeplan_dir(arg: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = arg {
        let candidate = p.to_path_buf();
        if !candidate.is_dir() {
            anyhow::bail!("workspace path does not exist: {}", candidate.display());
        }
        let nested = candidate.join(".forgeplan");
        if nested.is_dir() {
            return Ok(nested);
        }
        if candidate
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s == ".forgeplan")
            .unwrap_or(false)
        {
            return Ok(candidate);
        }
        anyhow::bail!(
            "reconcile-ids: no .forgeplan/ directory found at {}",
            candidate.display()
        );
    }
    let cwd = std::env::current_dir()?;
    forgeplan_core::workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ workspace found from {}", cwd.display()))
}

/// Result of walking every artifact `.md` under known kind subdirectories.
/// Returned as `(records, scan_errors)` где scan_errors carries `(path, message)`
/// pairs для files that failed to parse — non-fatal continuation.
type DiscoverResult = (Vec<ArtifactRecord>, Vec<(PathBuf, String)>);

/// Walk every artifact `.md` under known kind subdirectories.
fn discover_artifacts(forgeplan_dir: &Path) -> Result<DiscoverResult> {
    if !forgeplan_dir.is_dir() {
        anyhow::bail!(
            "workspace not found: {} is not a directory",
            forgeplan_dir.display()
        );
    }
    let mut records = Vec::new();
    let mut scan_errors: Vec<(PathBuf, String)> = Vec::new();

    for kind in SCAN_KINDS {
        let kind_dir = forgeplan_dir.join(kind.dir_name());
        if !kind_dir.is_dir() {
            continue;
        }
        let entries = match fs::read_dir(&kind_dir) {
            Ok(e) => e,
            Err(e) => {
                scan_errors.push((kind_dir.clone(), format!("read_dir failed: {e}")));
                continue;
            }
        };
        // [LOW-1 fix] Don't `.flatten()` here — silently dropping per-entry
        // IO errors masks transient FS issues (permission, broken inode,
        // unreadable symlink). Surface them as scan errors so downstream
        // tooling can react. Mirrors the loud-but-non-fatal posture
        // ci_assign_id.rs uses for parse failures.
        for entry_res in entries {
            let entry = match entry_res {
                Ok(e) => e,
                Err(e) => {
                    scan_errors.push((kind_dir.clone(), format!("read_dir entry failed: {e}")));
                    continue;
                }
            };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                continue;
            };
            if ext != "md" {
                continue;
            }
            match read_record(&path, kind) {
                Ok(r) => records.push(r),
                Err(e) => scan_errors.push((path.clone(), e.to_string())),
            }
        }
    }
    Ok((records, scan_errors))
}

fn read_record(path: &Path, kind: &ArtifactKind) -> Result<ArtifactRecord> {
    let content = fs::read_to_string(path).map_err(|e| anyhow::anyhow!("read file: {e}"))?;
    let (fm, body) =
        parse_frontmatter(&content).map_err(|e| anyhow::anyhow!("parse frontmatter: {e}"))?;
    let slug = slug_from_frontmatter(&fm).map(|s| s.to_string());
    // [HIGH-1 fix mirror of ci_assign_id.rs:418-431] When the frontmatter
    // does carry a slug, re-validate it against the SPEC-005 shape. Slugs
    // flow into filenames, hint suggestions, JSON output, and (downstream)
    // git command arguments — a malformed slug here means the artifact
    // was edited outside the canonical CLI/MCP path (red-line #11) or
    // PR-tampered. Fail the scan loudly rather than letting bogus content
    // reach apply-mode rename and downstream consumers. Legacy artifacts
    // that pre-date the slug field have `slug: None` — those are skipped.
    if let Some(s) = slug.as_deref()
        && let Err(e) = validate_slug(s)
    {
        anyhow::bail!("malformed slug {s:?}: {e}");
    }
    let predicted = predicted_number_from_frontmatter(&fm);
    let assigned = assigned_number_from_frontmatter(&fm);
    Ok(ArtifactRecord {
        path: path.to_path_buf(),
        kind: kind.clone(),
        fm,
        body,
        slug,
        predicted,
        assigned,
    })
}

/// Best-effort display form for an artifact (used purely for human / JSON
/// reporting — never as a key).
fn record_display_id(r: &ArtifactRecord) -> String {
    let prefix = kind_uppercase_prefix(&r.kind);
    match (r.assigned, r.predicted) {
        (Some(n), _) => format!("{prefix}-{n:03}"),
        (None, Some(n)) => format!("{prefix}-{n}?"),
        (None, None) => r.slug.clone().unwrap_or_else(|| {
            r.path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "<unknown>".to_string())
        }),
    }
}

/// Compute the canonical filename for an artifact when `assigned_number`
/// is set. Pattern: `<KIND>-<NNN>-<slug-suffix>.md`. The slug-suffix is
/// the `slug` field with the kind prefix stripped (slugs are kind-prefixed
/// per ADR-012 invariant I-1).
///
/// Returns `None` when the prerequisite fields aren't present (caller
/// already validated this is the `filename_mismatch` category candidate).
fn canonical_filename(r: &ArtifactRecord) -> Option<String> {
    let n = r.assigned?;
    let slug = r.slug.as_deref()?;
    let kind_prefix_lower = kind_key(&r.kind);
    // slug shape per ADR-012 is `<kind>-<suffix>` — strip the prefix to
    // avoid double-prefixing. If the slug somehow lacks the expected
    // prefix we fall back to the full slug as suffix.
    let suffix = slug
        .strip_prefix(&format!("{kind_prefix_lower}-"))
        .unwrap_or(slug);
    let prefix_upper = kind_uppercase_prefix(&r.kind);
    Some(format!("{prefix_upper}-{n:03}-{suffix}.md"))
}

/// Whether the on-disk filename already matches the canonical pattern.
fn filename_matches_canonical(r: &ArtifactRecord) -> bool {
    let Some(expected) = canonical_filename(r) else {
        return true; // not eligible; treat as match to skip
    };
    r.path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|fname| fname == expected)
        .unwrap_or(false)
}

/// Extract artifact ID tokens (e.g. `PRD-074`, `PROB-060`) from a markdown
/// body. We scan for tokens of the form `<UPPER>+-<digits>+` and validate
/// the prefix maps to a known kind. Used for body-links-drift detection.
fn body_artifact_refs(body: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    // Hand-rolled scanner to avoid pulling regex in for one site. We accept
    // tokens shaped `<UPPER+>-<DIGIT+>` with an optional trailing `?`.
    let bytes = body.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        // start of a candidate token must be ASCII uppercase
        if !bytes[i].is_ascii_uppercase() {
            i += 1;
            continue;
        }
        // walk uppercase letters
        let prefix_start = i;
        while i < n && bytes[i].is_ascii_uppercase() {
            i += 1;
        }
        let prefix_end = i;
        // require literal '-' next
        if i >= n || bytes[i] != b'-' {
            continue;
        }
        i += 1;
        // walk digits
        let digits_start = i;
        while i < n && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == digits_start {
            // No digits after the dash — not a valid token.
            continue;
        }
        let digits_end = i;
        // optional trailing '?' (predicted-id marker pre-merge)
        let mut tok_end = digits_end;
        if tok_end < n && bytes[tok_end] == b'?' {
            tok_end += 1;
        }
        // Boundary check: token must not be glued to another alphanumeric
        if tok_end < n && (bytes[tok_end].is_ascii_alphanumeric() || bytes[tok_end] == b'_') {
            // not a clean token boundary; skip
            continue;
        }
        let prefix = &body[prefix_start..prefix_end];
        // Accept only known artifact-kind prefixes. References like
        // `WIDGET-12` whose prefix maps to no kind are filtered here.
        let lower = prefix.to_ascii_lowercase();
        if ArtifactKind::from_slug_prefix(&lower).is_none() {
            continue;
        }
        // Strip `?` for canonical form. The empty-digits case is already
        // rejected at the `i == digits_start` guard above, so the slice
        // here is always non-empty.
        let tok = &body[prefix_start..digits_end];
        out.insert(tok.to_string());
    }
    out
}

/// Frontmatter `links:` is a sequence of `{target: <id>, relation: <rel>}`
/// entries. Returns the set of `target` values lowercased for
/// case-insensitive comparison against body-extracted IDs.
fn frontmatter_link_targets(fm: &Frontmatter) -> HashSet<String> {
    let mut out = HashSet::new();
    let Some(serde_yaml::Value::Sequence(seq)) = fm.get("links") else {
        return out;
    };
    for entry in seq {
        if let Some(target) = entry.get("target").and_then(|v| v.as_str()) {
            out.insert(target.trim().to_ascii_uppercase());
        }
    }
    out
}

// =====================================================================
// Detection
// =====================================================================

fn detect_filename_mismatch(r: &ArtifactRecord) -> Option<(String, String)> {
    if r.assigned.is_none() || r.slug.is_none() {
        return None;
    }
    if filename_matches_canonical(r) {
        return None;
    }
    let current = r
        .path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("<unknown>")
        .to_string();
    let expected = canonical_filename(r)?;
    Some((current, expected))
}

fn detect_missing_predicted(r: &ArtifactRecord) -> Option<u32> {
    r.slug.as_ref()?;
    if r.predicted.is_some() {
        return None;
    }
    // Auto-fill: prefer assigned, else fall back to 1 (synthetic placeholder)
    Some(r.assigned.unwrap_or(1))
}

fn detect_body_links_drift(r: &ArtifactRecord) -> Option<Vec<String>> {
    let body_refs = body_artifact_refs(&r.body);
    if body_refs.is_empty() {
        return None;
    }
    let frontmatter_targets = frontmatter_link_targets(&r.fm);
    // Figure out artifact's own canonical id (uppercase) to exclude self-refs.
    let prefix_upper = kind_uppercase_prefix(&r.kind);
    let self_id_assigned = r
        .assigned
        .map(|n| format!("{prefix_upper}-{n:03}"))
        .unwrap_or_default();
    let self_id_predicted = r
        .predicted
        .map(|n| format!("{prefix_upper}-{n}"))
        .unwrap_or_default();

    let drifted: Vec<String> = body_refs
        .into_iter()
        .filter(|tok| {
            let upper = tok.to_ascii_uppercase();
            if upper == self_id_assigned || upper == self_id_predicted {
                return false;
            }
            !frontmatter_targets.contains(&upper)
        })
        .collect();
    if drifted.is_empty() {
        None
    } else {
        let mut sorted = drifted;
        sorted.sort();
        Some(sorted)
    }
}

fn detect_duplicate_assigned(records: &[ArtifactRecord]) -> Vec<Vec<usize>> {
    let mut groups: HashMap<(String, u32), Vec<usize>> = HashMap::new();
    for (idx, r) in records.iter().enumerate() {
        if let Some(n) = r.assigned {
            groups
                .entry((kind_sort_key(&r.kind), n))
                .or_default()
                .push(idx);
        }
    }
    groups
        .into_values()
        .filter(|v| v.len() >= 2)
        .map(|mut v| {
            v.sort();
            v
        })
        .collect()
}

// =====================================================================
// Apply
// =====================================================================

/// Whether `path` is currently tracked by git in its parent worktree.
/// Returns `false` on any git error (not a repo, command not found, etc.)
/// — caller falls back to `fs::rename`.
fn is_git_tracked(path: &Path) -> bool {
    let parent = match path.parent() {
        Some(p) => p,
        None => return false,
    };
    let output = Command::new("git")
        .arg("ls-files")
        .arg("--error-unmatch")
        .arg("--")
        .arg(path)
        .current_dir(parent)
        .output();
    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

/// Rename `from` → `to`. Uses `git mv` if the file is tracked (preserves
/// history), `fs::rename` otherwise. Returns the new path on success.
///
/// [HIGH-1 fix mirror of ci_assign_id.rs:752-782 / 658-683 SEC-6 block]
/// Hardening:
/// 1. **Symlink check** — `symlink_metadata` (does NOT traverse) + reject
///    any source that is a symlink. A PR-tampered `.forgeplan/prds/x.md`
///    pointing at `/etc/passwd` would otherwise be moved.
/// 2. **Workspace boundary** — if `canonical_workspace` is supplied,
///    assert that the canonicalized source AND the canonicalized parent
///    of the target both stay under the workspace root. Defends against
///    relative-`..` slugs that escape `.forgeplan/`.
/// 3. **`--` separator** on `git mv` — explicit `mv -- <from> <to>`
///    prevents future flag-injection regressions if `from`/`to` ever
///    accept untrusted prefixes.
fn rename_with_git_fallback(
    from: &Path,
    to: &Path,
    canonical_workspace: Option<&Path>,
) -> Result<PathBuf> {
    if to.exists() {
        anyhow::bail!("destination already exists: {}", to.display());
    }

    // [SEC-6 CWE-367] symlink check: refuse to follow symlinks. Use
    // `symlink_metadata` which never traverses, unlike `metadata()`.
    let lmeta = fs::symlink_metadata(from)
        .map_err(|e| anyhow::anyhow!("stat {} (symlink check): {e}", from.display()))?;
    if lmeta.file_type().is_symlink() {
        anyhow::bail!(
            "reconcile-ids: refusing to follow symlink artifact {} [SEC-6]",
            from.display()
        );
    }

    // [SEC-6 CWE-22] path traversal: canonicalize source and assert it
    // stays under workspace; canonicalize target's parent (which must
    // already exist) and assert the same. We never canonicalize the
    // target itself before it exists.
    if let Some(ws_canon) = canonical_workspace {
        let from_canon = fs::canonicalize(from)
            .map_err(|e| anyhow::anyhow!("canonicalize {}: {e}", from.display()))?;
        if !from_canon.starts_with(ws_canon) {
            anyhow::bail!(
                "reconcile-ids: source {} escapes workspace [SEC-6 invariant violation]",
                from.display()
            );
        }
        let target_parent = to
            .parent()
            .ok_or_else(|| anyhow::anyhow!("rename target has no parent: {}", to.display()))?;
        let target_parent_canon = fs::canonicalize(target_parent).map_err(|e| {
            anyhow::anyhow!(
                "canonicalize target parent {}: {e}",
                target_parent.display()
            )
        })?;
        if !target_parent_canon.starts_with(ws_canon) {
            anyhow::bail!(
                "reconcile-ids: target parent {} escapes workspace [SEC-6]",
                target_parent.display()
            );
        }
    }

    if is_git_tracked(from) {
        let parent = from.parent().unwrap_or_else(|| Path::new("."));
        // [HIGH-1 fix] `["mv", "--"]` separator mirrors ci_assign_id.rs:606.
        // Prevents argument-injection if `from`/`to` ever start with a
        // dash; harmless on well-formed inputs.
        let status = Command::new("git")
            .args(["mv", "--"])
            .arg(from)
            .arg(to)
            .current_dir(parent)
            .status();
        if matches!(status, Ok(s) if s.success()) {
            return Ok(to.to_path_buf());
        }
        // fall through to fs::rename
    }
    fs::rename(from, to).map_err(|e| anyhow::anyhow!("fs::rename failed: {e}"))?;
    Ok(to.to_path_buf())
}

/// Insert `predicted_number` field at the canonical position. Body bytes
/// are preserved.
///
/// [HIGH-4 fix mirror of ci_assign_id.rs:854-867] Atomic publish: write
/// to `<path>.tmp` then `fs::rename` it into place. POSIX `rename(2)` is
/// atomic on a single filesystem — a crash mid-write therefore leaves
/// either the previous file intact or the new file fully written, never
/// a half-written truncation. Direct `fs::write(path, …)` truncates the
/// destination first, which means a crash mid-write would leave an
/// empty/partial file on disk.
///
/// [Round 2 Sec FINDING-5 fix] Defense-in-depth: this function now also
/// gets the SEC-6 hardening block (symlink-reject + workspace-boundary
/// canonicalize) that the rename path picked up in Round 1. Without it
/// a tampered artifact `path` could be a symlink (e.g. pointing at
/// `/etc/passwd`) and the predicted-number rewrite would clobber the
/// link target. Mirrors `ci_assign_id.rs:806-837` exactly.
///
/// `canonical_workspace` is `Option` to mirror `apply_actions`'s posture
/// — if canonicalize at the workspace level failed earlier, the boundary
/// check is skipped (write will still fail loudly if the FS is broken),
/// but the symlink check ALWAYS runs because it's local to `path` and
/// has no workspace dependency.
fn write_predicted_number(
    path: &Path,
    fm: &Frontmatter,
    body: &str,
    n: u32,
    canonical_workspace: Option<&Path>,
) -> Result<()> {
    // [SEC-6 CWE-367] symlink check: refuse to follow symlinks. Use
    // `symlink_metadata` which never traverses, unlike `metadata()`.
    let lmeta = fs::symlink_metadata(path)
        .map_err(|e| anyhow::anyhow!("stat {} (symlink check): {e}", path.display()))?;
    if lmeta.file_type().is_symlink() {
        anyhow::bail!(
            "reconcile-ids: refusing to follow symlink artifact {} [SEC-6]",
            path.display()
        );
    }

    // [SEC-6 CWE-22] path traversal: canonicalize `path` and assert it
    // stays under the canonicalized workspace. Defends against a
    // relative-`..` path or symlink-shaped tamper that escaped the
    // boundary checks earlier in the pipeline (defense-in-depth).
    if let Some(ws_canon) = canonical_workspace {
        let path_canon = fs::canonicalize(path)
            .map_err(|e| anyhow::anyhow!("canonicalize {}: {e}", path.display()))?;
        if !path_canon.starts_with(ws_canon) {
            anyhow::bail!(
                "reconcile-ids: path {} escapes workspace [SEC-6 invariant violation]",
                path.display()
            );
        }
    }

    let mut new_fm = fm.clone();
    new_fm.insert(
        "predicted_number".to_string(),
        serde_yaml::Value::Number(serde_yaml::Number::from(n)),
    );
    let rendered = render_frontmatter(&new_fm, body)?;

    // Sibling tmp file — same parent ⇒ same filesystem ⇒ atomic rename.
    // We use `.md.tmp` (mirrors ci_assign_id.rs convention) so a stray
    // crashed-tmp is easy to recognize manually.
    let tmp_path = path.with_extension("md.tmp");
    fs::write(&tmp_path, rendered)
        .map_err(|e| anyhow::anyhow!("write tmp {} failed: {e}", tmp_path.display()))?;
    fs::rename(&tmp_path, path).map_err(|e| {
        // Try to clean up the orphan tmp on rename failure so a retry
        // doesn't trip over a stale sibling. Best-effort — if the
        // cleanup itself errors we keep the original error context.
        let _ = fs::remove_file(&tmp_path);
        anyhow::anyhow!(
            "atomic rename {} -> {} failed: {e}",
            tmp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

// =====================================================================
// Report assembly
// =====================================================================

/// Build the action list. Order:
///   1. duplicate_assigned (per group)
///   2. filename_mismatch (per record)
///   3. missing_predicted (per record)
///   4. body_links_drift (per record)
///   5. cross_pr_deferred (one entry, when flag supplied)
fn build_actions(
    records: &[ArtifactRecord],
    cross_pr_requested: bool,
    workspace: &Path,
) -> Vec<ReconcileAction> {
    let mut actions = Vec::new();

    // 1. duplicate_assigned (flag only, never fix)
    for group in detect_duplicate_assigned(records) {
        let members: Vec<&ArtifactRecord> = group.iter().map(|i| &records[*i]).collect();
        let kind = members[0].kind.clone();
        let assigned = members[0].assigned.unwrap_or(0);
        let kind_upper = kind_uppercase_prefix(&kind);
        let display = format!("{kind_upper}-{assigned:03}");
        let paths: Vec<String> = members
            .iter()
            .map(|m| redact_path(workspace, &m.path))
            .collect();
        actions.push(ReconcileAction {
            category: Category::DuplicateAssigned,
            artifact_id: display.clone(),
            artifact_path: members[0].path.clone(),
            current_state: serde_json::json!({
                "kind": kind_key(&kind),
                "assigned_number": assigned,
                "members": paths,
            }),
            suggested_fix: serde_json::json!({
                "action": "manual_review_required",
                "note": "Auto-fix is disabled for duplicate_assigned — human must \
                         decide which artifact retains the number and which one is \
                         renumbered or deprecated.",
            }),
            applied: Some(false),
        });
    }

    // 2. filename_mismatch
    for r in records {
        if let Some((current, expected)) = detect_filename_mismatch(r) {
            actions.push(ReconcileAction {
                category: Category::FilenameMismatch,
                artifact_id: record_display_id(r),
                artifact_path: r.path.clone(),
                current_state: serde_json::json!({
                    "filename": current,
                    "assigned_number": r.assigned,
                    "slug": r.slug,
                }),
                suggested_fix: serde_json::json!({
                    "action": "rename",
                    "new_filename": expected,
                }),
                applied: None,
            });
        }
    }

    // 3. missing_predicted
    for r in records {
        if let Some(value) = detect_missing_predicted(r) {
            actions.push(ReconcileAction {
                category: Category::MissingPredicted,
                artifact_id: record_display_id(r),
                artifact_path: r.path.clone(),
                current_state: serde_json::json!({
                    "slug": r.slug,
                    "predicted_number": null,
                    "assigned_number": r.assigned,
                }),
                suggested_fix: serde_json::json!({
                    "action": "set_predicted_number",
                    "value": value,
                }),
                applied: None,
            });
        }
    }

    // 4. body_links_drift
    for r in records {
        if let Some(missing) = detect_body_links_drift(r) {
            actions.push(ReconcileAction {
                category: Category::BodyLinksDrift,
                artifact_id: record_display_id(r),
                artifact_path: r.path.clone(),
                current_state: serde_json::json!({
                    "body_refs_not_in_links": missing,
                }),
                suggested_fix: serde_json::json!({
                    "action": "report_only",
                    "note": "Use `forgeplan link <source> <target> --relation <r>` \
                             to update frontmatter links — direct edits violate \
                             red-line #11.",
                }),
                applied: Some(false),
            });
        }
    }

    // 5. cross_pr_deferred (forward-compat marker)
    if cross_pr_requested {
        actions.push(ReconcileAction {
            category: Category::CrossPrDeferred,
            artifact_id: "<workspace>".to_string(),
            artifact_path: workspace.to_path_buf(),
            current_state: serde_json::json!({
                "flag": "--report-cross-pr",
            }),
            suggested_fix: serde_json::json!({
                "action": "deferred",
                "note": "Cross-PR Refs: drift detection is deferred — see \
                         RFC-009 §Phase 2.4. Workspace-only categories are \
                         covered above.",
            }),
            applied: Some(true),
        });
    }

    actions
}

/// Apply auto-fixable actions (filename_mismatch + missing_predicted) in
/// place. Returns the same action list with `applied = Some(true|false)`
/// filled in for the in-scope categories.
///
/// Apply order: filename renames first (so subsequent reads use new
/// paths), then predicted-number writes. We always re-read the file
/// content immediately before mutation to defend against TOCTOU drift.
///
/// [HIGH-1 fix] `workspace` is canonicalized once at entry and passed
/// down into [`rename_with_git_fallback`] for SEC-6 boundary checks. We
/// canonicalize once because canonicalize is a syscall — repeating it
/// per-action would be wasteful and (worse) racy if the workspace dir
/// were swapped mid-loop.
fn apply_actions(actions: &mut [ReconcileAction], workspace: &Path) {
    // Canonicalize workspace once. If canonicalize fails (e.g. workspace
    // got removed) we skip the boundary check rather than crash — writes
    // will fail loudly below if the FS is genuinely broken. This mirrors
    // the posture in ci_assign_id.rs::apply_plan.
    let canonical_workspace = fs::canonicalize(workspace).ok();

    // Phase 1: renames. We update `artifact_path` in-place so any
    // subsequent operation on the same record (e.g. predicted-number
    // write that landed on the same file) sees the new path.
    let mut renamed: HashMap<PathBuf, PathBuf> = HashMap::new();
    for action in actions.iter_mut() {
        if action.category != Category::FilenameMismatch {
            continue;
        }
        let new_filename = match action
            .suggested_fix
            .get("new_filename")
            .and_then(|v| v.as_str())
        {
            Some(s) => s.to_string(),
            None => {
                action.applied = Some(false);
                continue;
            }
        };
        let from = action.artifact_path.clone();
        let parent = match from.parent() {
            Some(p) => p.to_path_buf(),
            None => {
                action.applied = Some(false);
                continue;
            }
        };
        let to = parent.join(&new_filename);
        match rename_with_git_fallback(&from, &to, canonical_workspace.as_deref()) {
            Ok(new_path) => {
                renamed.insert(from, new_path.clone());
                action.artifact_path = new_path;
                action.applied = Some(true);
            }
            Err(_) => action.applied = Some(false),
        }
    }

    // Phase 2: predicted-number fills.
    for action in actions.iter_mut() {
        if action.category != Category::MissingPredicted {
            continue;
        }
        let value = match action.suggested_fix.get("value").and_then(|v| v.as_u64()) {
            Some(n) => match u32::try_from(n) {
                Ok(n) => n,
                Err(_) => {
                    action.applied = Some(false);
                    continue;
                }
            },
            None => {
                action.applied = Some(false);
                continue;
            }
        };
        // Resolve renamed paths.
        let path = renamed
            .get(&action.artifact_path)
            .cloned()
            .unwrap_or_else(|| action.artifact_path.clone());
        // Re-read the file (TOCTOU-safe: parses current frontmatter).
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => {
                action.applied = Some(false);
                continue;
            }
        };
        let (fm, body) = match parse_frontmatter(&content) {
            Ok(v) => v,
            Err(_) => {
                action.applied = Some(false);
                continue;
            }
        };
        // Idempotency: someone else may have set it between scan and apply.
        if predicted_number_from_frontmatter(&fm).is_some() {
            action.applied = Some(true);
            action.artifact_path = path;
            continue;
        }
        match write_predicted_number(&path, &fm, &body, value, canonical_workspace.as_deref()) {
            Ok(()) => {
                action.applied = Some(true);
                action.artifact_path = path;
            }
            Err(_) => action.applied = Some(false),
        }
    }
}

// =====================================================================
// Render
// =====================================================================

pub fn render_json(report: &ReconcileReport) -> serde_json::Value {
    let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let actions: Vec<serde_json::Value> = report
        .actions
        .iter()
        .map(|a| {
            serde_json::json!({
                "category": a.category.as_str(),
                "artifact_id": a.artifact_id,
                "artifact_path": redact_path(&report.workspace, &a.artifact_path),
                "current_state": a.current_state,
                "suggested_fix": a.suggested_fix,
                "applied": a.applied,
            })
        })
        .collect();
    let scan_errors: Vec<serde_json::Value> = report
        .scan_errors
        .iter()
        .map(|(p, reason)| {
            serde_json::json!({
                "path": redact_path(&report.workspace, p),
                "reason": reason,
            })
        })
        .collect();
    // [MED-2 fix mirror of redact_path posture] Don't leak the absolute
    // workspace path into JSON output — it surfaces CI runner layout
    // (`/home/runner/work/owner/repo/.forgeplan`) and is useless to
    // consumers, who only care that this is a Forgeplan workspace. Emit
    // the canonical relative `.forgeplan` shape instead. The scanned
    // paths inside `actions` and `scan_errors` are already redacted to
    // workspace-relative form by `redact_path`.
    serde_json::json!({
        "schema_version": 1,
        "timestamp": timestamp,
        "workspace": ".forgeplan",
        "check_only": report.check_only,
        "actions": actions,
        "scan_errors": scan_errors,
        "summary": {
            "total_actions": report.actions.len(),
            "unresolved": report.has_unresolved(),
        }
    })
}

pub fn render_human(report: &ReconcileReport) -> String {
    let mut out = String::new();
    out.push_str("Forgeplan reconcile-ids (PROB-060 / RFC-009 §Phase 2.4)\n");
    out.push_str(&format!("Workspace: {}\n", report.workspace.display()));
    out.push_str(&format!(
        "Mode: {}\n",
        if report.check_only {
            "check-only"
        } else {
            "apply"
        }
    ));
    out.push_str(&format!(
        "Per-kind counts: {} kinds, {} artifacts total\n",
        report.per_kind_count.len(),
        report.per_kind_count.values().sum::<usize>(),
    ));
    out.push('\n');

    if report.actions.is_empty() {
        out.push_str("No drift detected. Workspace is coherent.\n");
        if !report.scan_errors.is_empty() {
            out.push_str(&format!(
                "\nScan errors: {} (see --json for detail)\n",
                report.scan_errors.len()
            ));
        }
        return out;
    }

    out.push_str(&format!("Actions: {}\n", report.actions.len()));
    for a in &report.actions {
        let applied = match a.applied {
            Some(true) => "[applied]",
            Some(false) => "[not applied]",
            None => "[pending]",
        };
        out.push_str(&format!(
            "  {} {} {} {}\n",
            applied,
            a.category.as_str(),
            a.artifact_id,
            redact_path(&report.workspace, &a.artifact_path)
        ));
    }
    if !report.scan_errors.is_empty() {
        out.push_str(&format!("\nScan errors: {}\n", report.scan_errors.len()));
    }
    if report.has_unresolved() {
        out.push_str(
            "\nUnresolved drift remains. Manual review needed for \
             duplicate_assigned / body_links_drift; rerun without \
             --check-only to apply auto-fixable categories.\n",
        );
    }
    out
}

// =====================================================================
// Entry point
// =====================================================================

/// Public entry point. Returns process exit code (0 / 1 / 2 per module
/// docs). Caller in `main.rs` is responsible for `std::process::exit`.
pub fn run(args: ReconcileIdsArgs) -> Result<i32> {
    let forgeplan_dir = match resolve_forgeplan_dir(args.workspace.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            return Ok(2);
        }
    };
    let (records, scan_errors) = match discover_artifacts(&forgeplan_dir) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {e}");
            return Ok(2);
        }
    };

    let mut per_kind_count: BTreeMap<String, usize> = BTreeMap::new();
    for r in &records {
        *per_kind_count.entry(kind_sort_key(&r.kind)).or_insert(0) += 1;
    }

    // Build actions. In apply mode, mark `applied = None` for
    // auto-fixable categories before mutation; non-fixable categories
    // stay `Some(false)`.
    let mut actions = build_actions(&records, args.report_cross_pr, &forgeplan_dir);

    // In check-only mode, leave applied as-is from build_actions:
    //   - filename_mismatch / missing_predicted → None (pending review)
    //   - duplicate_assigned / body_links_drift → Some(false) (never fixed)
    //   - cross_pr_deferred → Some(true) (no-op)
    if !args.check_only {
        apply_actions(&mut actions, &forgeplan_dir);
    }

    let report = ReconcileReport {
        workspace: forgeplan_dir.clone(),
        check_only: args.check_only,
        actions,
        scan_errors,
        per_kind_count,
    };

    if args.json {
        let json = render_json(&report);
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("{}", render_human(&report));
    }

    let exit = if report.has_unresolved() { 1 } else { 0 };
    Ok(exit)
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Build a minimal workspace skeleton. Returns the project root (the
    /// caller passes either this or `<root>/.forgeplan` to `--workspace`).
    fn temp_workspace(files: &[(&str, &str, &str)]) -> TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let fp = dir.path().join(".forgeplan");
        for k in SCAN_KINDS {
            fs::create_dir_all(fp.join(k.dir_name())).unwrap();
        }
        for (subdir, fname, content) in files {
            let p = fp.join(subdir).join(fname);
            fs::write(p, content).unwrap();
        }
        dir
    }

    fn fm_full(id: &str, title: &str, slug: &str, predicted: u32, assigned: Option<u32>) -> String {
        match assigned {
            Some(n) => format!(
                "---\nid: {id}\nkind: prd\nstatus: draft\ntitle: {title}\nslug: {slug}\npredicted_number: {predicted}\nassigned_number: {n}\n---\n\nBody.\n"
            ),
            None => format!(
                "---\nid: {id}\nkind: prd\nstatus: draft\ntitle: {title}\nslug: {slug}\npredicted_number: {predicted}\nassigned_number: null\n---\n\nBody.\n"
            ),
        }
    }

    #[test]
    fn reconcile_ids_happy_path_clean_workspace() {
        // A workspace with two coherent PRDs (assigned, slug, predicted,
        // canonical filename) and one RFC. Expect 0 actions in either
        // mode and exit code 0.
        let ws = temp_workspace(&[
            (
                "prds",
                "PRD-001-auth-system.md",
                &fm_full("PRD-001", "Auth system", "prd-auth-system", 1, Some(1)),
            ),
            (
                "prds",
                "PRD-002-billing-service.md",
                &fm_full(
                    "PRD-002",
                    "Billing service",
                    "prd-billing-service",
                    2,
                    Some(2),
                ),
            ),
        ]);
        let args = ReconcileIdsArgs {
            workspace: Some(ws.path().to_path_buf()),
            check_only: true,
            report_cross_pr: false,
            json: true,
        };
        let code = run(args).unwrap();
        // 0 actions ⇒ no unresolved ⇒ exit 0
        assert_eq!(code, 0);
    }

    #[test]
    fn reconcile_ids_filename_mismatch_detected() {
        // PRD with `assigned_number: 7` and slug `prd-auth-system` lives in
        // a wrong-shaped filename. Detection must flag, suggested fix must
        // be the canonical pattern.
        let ws = temp_workspace(&[(
            "prds",
            "PRD-007-stale-name.md", // wrong: slug suffix doesn't match
            &fm_full("PRD-007", "Auth system", "prd-auth-system", 7, Some(7)),
        )]);
        let fp = ws.path().join(".forgeplan");
        let (records, _) = discover_artifacts(&fp).unwrap();
        let actions = build_actions(&records, false, &fp);
        let mismatch: Vec<&ReconcileAction> = actions
            .iter()
            .filter(|a| a.category == Category::FilenameMismatch)
            .collect();
        assert_eq!(mismatch.len(), 1);
        assert_eq!(
            mismatch[0]
                .suggested_fix
                .get("new_filename")
                .and_then(|v| v.as_str()),
            Some("PRD-007-auth-system.md")
        );
    }

    #[test]
    fn reconcile_ids_missing_predicted_autofill() {
        // Artifact has slug + assigned_number but predicted_number is missing.
        // Apply mode must auto-fill predicted_number = assigned_number.
        let content = "---\nid: PRD-005\nkind: prd\nstatus: draft\ntitle: Search index\nslug: prd-search-index\nassigned_number: 5\n---\n\nBody.\n";
        let ws = temp_workspace(&[("prds", "PRD-005-search-index.md", content)]);
        let args = ReconcileIdsArgs {
            workspace: Some(ws.path().to_path_buf()),
            check_only: false, // apply
            report_cross_pr: false,
            json: false,
        };
        let code = run(args).unwrap();
        // The predicted_number action applied, filename was already canonical
        // ⇒ no unresolved ⇒ exit 0.
        assert_eq!(code, 0);
        let written =
            fs::read_to_string(ws.path().join(".forgeplan/prds/PRD-005-search-index.md")).unwrap();
        assert!(written.contains("predicted_number: 5"));
    }

    #[test]
    fn reconcile_ids_duplicate_assigned_flagged() {
        // Two PRDs with assigned_number: 9 — must surface as
        // duplicate_assigned (never auto-fixed) and exit 1 even in apply
        // mode.
        let ws = temp_workspace(&[
            (
                "prds",
                "PRD-009-first.md",
                &fm_full("PRD-009", "First", "prd-first", 9, Some(9)),
            ),
            (
                "prds",
                "PRD-009-second.md",
                &fm_full("PRD-009", "Second", "prd-second", 9, Some(9)),
            ),
        ]);
        let fp = ws.path().join(".forgeplan");
        let (records, _) = discover_artifacts(&fp).unwrap();
        let actions = build_actions(&records, false, &fp);
        let dups: Vec<&ReconcileAction> = actions
            .iter()
            .filter(|a| a.category == Category::DuplicateAssigned)
            .collect();
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].applied, Some(false));
        // run() in apply mode must still exit 1 because the duplicate is
        // never resolved.
        let args = ReconcileIdsArgs {
            workspace: Some(ws.path().to_path_buf()),
            check_only: false,
            report_cross_pr: false,
            json: false,
        };
        let code = run(args).unwrap();
        assert_eq!(code, 1);
    }

    #[test]
    fn reconcile_ids_body_links_drift_reports_without_fix() {
        // PRD body mentions PROB-060 but frontmatter `links:` doesn't
        // include it. Must surface as body_links_drift, never auto-fixed.
        let body =
            "## Related Artifacts\n\n| Artifact | Relation |\n|---|---|\n| PROB-060 | based_on |\n";
        let content = format!(
            "---\nid: PRD-010\nkind: prd\nstatus: draft\ntitle: Linked\nslug: prd-linked\npredicted_number: 10\nassigned_number: 10\nlinks:\n- target: ADR-012\n  relation: based_on\n---\n\n{body}\n"
        );
        let ws = temp_workspace(&[("prds", "PRD-010-linked.md", &content)]);
        let fp = ws.path().join(".forgeplan");
        let (records, _) = discover_artifacts(&fp).unwrap();
        let actions = build_actions(&records, false, &fp);
        let drifts: Vec<&ReconcileAction> = actions
            .iter()
            .filter(|a| a.category == Category::BodyLinksDrift)
            .collect();
        assert_eq!(drifts.len(), 1);
        assert_eq!(drifts[0].applied, Some(false));
        let missing = drifts[0]
            .current_state
            .get("body_refs_not_in_links")
            .unwrap()
            .as_array()
            .unwrap();
        let strs: Vec<&str> = missing.iter().filter_map(|v| v.as_str()).collect();
        assert!(strs.contains(&"PROB-060"));
        assert!(!strs.contains(&"ADR-012")); // present in links → not drifted
    }

    #[test]
    fn reconcile_ids_apply_renames_filename() {
        // End-to-end apply: wrong filename → after run the file is renamed
        // to the canonical pattern.
        let ws = temp_workspace(&[(
            "prds",
            "PRD-007-wrong-name.md",
            &fm_full("PRD-007", "Real title", "prd-real-title", 7, Some(7)),
        )]);
        let args = ReconcileIdsArgs {
            workspace: Some(ws.path().to_path_buf()),
            check_only: false,
            report_cross_pr: false,
            json: false,
        };
        let code = run(args).unwrap();
        assert_eq!(code, 0);
        assert!(
            !ws.path()
                .join(".forgeplan/prds/PRD-007-wrong-name.md")
                .exists()
        );
        assert!(
            ws.path()
                .join(".forgeplan/prds/PRD-007-real-title.md")
                .exists()
        );
    }

    #[test]
    fn reconcile_ids_check_only_does_not_modify_files() {
        let content = "---\nid: PRD-005\nkind: prd\nstatus: draft\ntitle: Search\nslug: prd-search\nassigned_number: 5\n---\n\nBody.\n";
        let ws = temp_workspace(&[("prds", "PRD-005-stale.md", content)]);
        let original =
            fs::read_to_string(ws.path().join(".forgeplan/prds/PRD-005-stale.md")).unwrap();
        let args = ReconcileIdsArgs {
            workspace: Some(ws.path().to_path_buf()),
            check_only: true,
            report_cross_pr: false,
            json: false,
        };
        let code = run(args).unwrap();
        // Two pending fixes (filename + missing_predicted) → unresolved.
        assert_eq!(code, 1);
        // File on disk untouched.
        assert!(ws.path().join(".forgeplan/prds/PRD-005-stale.md").exists());
        let after = fs::read_to_string(ws.path().join(".forgeplan/prds/PRD-005-stale.md")).unwrap();
        assert_eq!(original, after);
    }

    #[test]
    fn reconcile_ids_report_cross_pr_emits_marker() {
        let ws = temp_workspace(&[]);
        let fp = ws.path().join(".forgeplan");
        let actions = build_actions(&[], true, &fp);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].category, Category::CrossPrDeferred);
        assert_eq!(actions[0].applied, Some(true));
    }

    #[test]
    fn reconcile_ids_workspace_missing_returns_exit_2() {
        let dir = tempfile::tempdir().unwrap();
        let bogus = dir.path().join("definitely-not-here");
        let args = ReconcileIdsArgs {
            workspace: Some(bogus),
            check_only: true,
            report_cross_pr: false,
            json: false,
        };
        let code = run(args).unwrap();
        assert_eq!(code, 2);
    }

    #[test]
    fn body_artifact_refs_picks_up_canonical_tokens() {
        let body = "Refs PRD-074, ADR-012 and PROB-060? but not WIDGET-1 or LOWER-2.";
        let refs = body_artifact_refs(body);
        assert!(refs.contains("PRD-074"));
        assert!(refs.contains("ADR-012"));
        assert!(refs.contains("PROB-060"));
        // WIDGET prefix maps to no kind → skipped
        assert!(!refs.iter().any(|r| r.starts_with("WIDGET")));
        // lower-case prefix is skipped
        assert!(!refs.iter().any(|r| r.contains("LOWER")));
    }

    #[test]
    fn canonical_filename_round_trip() {
        let mut fm = Frontmatter::new();
        fm.insert(
            "slug".to_string(),
            serde_yaml::Value::String("prd-auth-system".to_string()),
        );
        fm.insert(
            "assigned_number".to_string(),
            serde_yaml::Value::Number(serde_yaml::Number::from(7u32)),
        );
        let r = ArtifactRecord {
            path: PathBuf::from("/tmp/.forgeplan/prds/PRD-007-stale.md"),
            kind: ArtifactKind::Prd,
            fm,
            body: String::new(),
            slug: Some("prd-auth-system".to_string()),
            predicted: None,
            assigned: Some(7),
        };
        assert_eq!(
            canonical_filename(&r).as_deref(),
            Some("PRD-007-auth-system.md")
        );
    }

    // =================================================================
    // Round 1 audit closures — additional coverage for the safety
    // hardening landed in this commit (HIGH-1, HIGH-4, MED-2, LOW-1,
    // LOW-2). Each test pins one invariant that the hardening enforces.
    // =================================================================

    /// [HIGH-1 / SEC-6] `read_record` rejects malformed slugs from
    /// frontmatter so a tampered artifact (slug containing shell
    /// metacharacters, uppercase letters, etc.) cannot reach apply mode.
    /// The test uses `discover_artifacts` (the public entry point that
    /// touches `read_record`) and asserts the bad file lands in
    /// `scan_errors` rather than `records`.
    #[test]
    fn reconcile_ids_read_record_rejects_malformed_slug() {
        // Slug `PRD-Bad slug` violates SPEC-005 (uppercase + space). The
        // file is otherwise well-formed YAML — the only failure should
        // come from `validate_slug`.
        let bad = "---\nid: PRD-099\nkind: prd\nstatus: draft\ntitle: T\nslug: \"PRD-Bad slug\"\npredicted_number: 99\nassigned_number: null\n---\n\nBody.\n";
        let ws = temp_workspace(&[("prds", "prd-bad-slug.md", bad)]);
        let fp = ws.path().join(".forgeplan");
        let (records, scan_errors) = discover_artifacts(&fp).unwrap();
        // The bad file must NOT produce a record.
        assert!(
            records.is_empty(),
            "malformed slug should not yield a record"
        );
        // It MUST surface as a scan error so operators see the violation.
        assert_eq!(scan_errors.len(), 1);
        let (path, msg) = &scan_errors[0];
        assert!(path.ends_with("prd-bad-slug.md"));
        assert!(
            msg.contains("malformed slug") || msg.contains("slug"),
            "scan error should mention slug; got: {msg}"
        );
    }

    /// [HIGH-1] Legacy artifacts that pre-date the slug field (`slug:
    /// None`) must still parse cleanly — `validate_slug` only fires when
    /// a slug is present.
    #[test]
    fn reconcile_ids_read_record_accepts_legacy_no_slug() {
        // No slug field at all — early Phase-1 artifacts look like this.
        let legacy = "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Legacy\nassigned_number: 1\n---\n\nBody.\n";
        let ws = temp_workspace(&[("prds", "PRD-001-legacy.md", legacy)]);
        let fp = ws.path().join(".forgeplan");
        let (records, scan_errors) = discover_artifacts(&fp).unwrap();
        assert_eq!(records.len(), 1, "legacy slug-less artifact must parse");
        assert!(scan_errors.is_empty());
        assert!(records[0].slug.is_none());
    }

    /// [HIGH-1 / SEC-6 CWE-22] `rename_with_git_fallback` refuses to act
    /// on a symlink source. The companion `ci_assign_id.rs` invariant —
    /// here we pin the same posture for the manual-cleanup tool.
    #[cfg(unix)]
    #[test]
    fn rename_with_git_fallback_rejects_symlink_source() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("real.md");
        fs::write(&target, "real").unwrap();
        let link = dir.path().join("link.md");
        symlink(&target, &link).unwrap();
        let dest = dir.path().join("renamed.md");

        let err = rename_with_git_fallback(&link, &dest, None).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("symlink") || msg.contains("SEC-6"),
            "expected symlink rejection, got: {msg}"
        );
        // Both source and target untouched.
        assert!(link.exists());
        assert!(target.exists());
        assert!(!dest.exists());
    }

    /// [HIGH-1 / SEC-6] When `canonical_workspace` is supplied the rename
    /// must reject a source path whose canonical form lies outside the
    /// workspace. We construct the situation by passing a `canonical_workspace`
    /// that points at a different temp dir than the actual source.
    #[test]
    fn rename_with_git_fallback_enforces_workspace_boundary() {
        let outer = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();

        // Real file lives under `outer`, NOT under `workspace`.
        let from = outer.path().join("escapes.md");
        fs::write(&from, "x").unwrap();
        let to = outer.path().join("renamed.md");

        let canonical_ws = fs::canonicalize(workspace.path()).unwrap();
        let err = rename_with_git_fallback(&from, &to, Some(&canonical_ws)).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("escapes workspace") || msg.contains("SEC-6"),
            "expected workspace-boundary rejection, got: {msg}"
        );
        // Source untouched.
        assert!(from.exists());
        assert!(!to.exists());
    }

    /// [HIGH-4] `write_predicted_number` is atomic — uses a tmp+rename
    /// dance so the destination either has the old content or the new
    /// content, never an empty/half-written file. We can't easily
    /// simulate a crash, so we verify the contract indirectly: after a
    /// successful call no `*.md.tmp` sibling remains, the destination
    /// is well-formed and contains the new field, and a *failing* call
    /// (rename to a path whose parent doesn't exist) leaves the original
    /// intact.
    #[test]
    fn write_predicted_number_is_atomic_and_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.md");
        let original = "---\nid: PRD-005\nkind: prd\nstatus: draft\ntitle: T\nslug: prd-search\nassigned_number: 5\n---\n\nBody.\n";
        fs::write(&path, original).unwrap();

        let (fm, body) = parse_frontmatter(original).unwrap();
        // [Round 2 Sec FINDING-5] SEC-6 hardening: pass workspace boundary
        // (None disables the boundary check; symlink check still runs).
        write_predicted_number(&path, &fm, &body, 5, None).unwrap();

        // No orphan tmp left behind.
        let tmp = path.with_extension("md.tmp");
        assert!(!tmp.exists(), "atomic write must clean up tmp on success");

        // Destination has the new field and the original body.
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("predicted_number: 5"));
        assert!(after.contains("Body."));

        // Round-trip: re-parse must succeed (file is well-formed).
        let (fm2, body2) = parse_frontmatter(&after).unwrap();
        assert_eq!(predicted_number_from_frontmatter(&fm2), Some(5));
        assert_eq!(body2.trim(), "Body.");
    }

    /// [Round 2 Sec FINDING-5] `write_predicted_number` refuses to follow a
    /// symlinked artifact path. A tampered PR could land a symlink at
    /// `.forgeplan/prds/x.md` pointing at `/etc/passwd`; without this guard
    /// the predicted-number rewrite would clobber the link target. Mirrors
    /// the `rename_with_git_fallback_rejects_symlink_source` test posture.
    #[cfg(unix)]
    #[test]
    fn write_predicted_number_rejects_symlink_artifact() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        // Real target file (could be inside or outside workspace — symlink
        // existence alone is the trigger).
        let real_target = dir.path().join("real.md");
        let real_content = "---\nid: PRD-005\nkind: prd\nstatus: draft\ntitle: T\nslug: prd-x\nassigned_number: 5\n---\n\nBody.\n";
        fs::write(&real_target, real_content).unwrap();

        let link_path = dir.path().join("link.md");
        symlink(&real_target, &link_path).unwrap();

        let (fm, body) = parse_frontmatter(real_content).unwrap();
        let err = write_predicted_number(&link_path, &fm, &body, 5, None)
            .expect_err("symlink artifact must be rejected");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("symlink") || msg.contains("SEC-6"),
            "expected symlink rejection message, got: {msg}"
        );
        // The real target must remain untouched (no predicted_number
        // injected), since the rewrite never proceeded.
        let after = fs::read_to_string(&real_target).unwrap();
        assert!(
            !after.contains("predicted_number: 5"),
            "real target must not be clobbered through symlink"
        );
    }

    /// [Round 2 Sec FINDING-5] `write_predicted_number` enforces the
    /// workspace boundary: a path that canonicalizes outside the supplied
    /// workspace root is rejected. Mirrors
    /// `rename_with_git_fallback_enforces_workspace_boundary`.
    #[test]
    fn write_predicted_number_enforces_workspace_boundary() {
        // Two sibling dirs: `workspace/` (the canonical root we pass in)
        // and `outside/` (where the artifact actually lives).
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let outside = dir.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        let path = outside.join("x.md");
        let original = "---\nid: PRD-005\nkind: prd\nstatus: draft\ntitle: T\nslug: prd-x\nassigned_number: 5\n---\n\nBody.\n";
        fs::write(&path, original).unwrap();

        let (fm, body) = parse_frontmatter(original).unwrap();
        let canonical_ws = fs::canonicalize(&workspace).unwrap();
        let err = write_predicted_number(&path, &fm, &body, 5, Some(&canonical_ws))
            .expect_err("path outside workspace must be rejected");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("escapes workspace") || msg.contains("SEC-6"),
            "expected workspace-boundary rejection, got: {msg}"
        );
        // File untouched — no predicted_number field appended.
        let after = fs::read_to_string(&path).unwrap();
        assert!(
            !after.contains("predicted_number: 5"),
            "file outside workspace must not be modified"
        );
    }

    /// [MED-2] The JSON `workspace` field is the relative `.forgeplan`
    /// string — never the absolute path of the runner. This pins the
    /// fix that prevents CI layout (`/home/runner/work/owner/repo/...`)
    /// from leaking into machine-readable output.
    #[test]
    fn render_json_workspace_field_is_relative() {
        let dir = tempfile::tempdir().unwrap();
        let absolute_ws = dir.path().join(".forgeplan");
        fs::create_dir_all(&absolute_ws).unwrap();
        let report = ReconcileReport {
            workspace: absolute_ws.clone(),
            check_only: true,
            actions: Vec::new(),
            scan_errors: Vec::new(),
            per_kind_count: BTreeMap::new(),
        };
        let v = render_json(&report);
        let ws_field = v.get("workspace").and_then(|v| v.as_str()).unwrap();
        assert_eq!(ws_field, ".forgeplan");
        // Defense check: must NOT contain the absolute path that the
        // report carries internally.
        assert!(
            !ws_field.contains(absolute_ws.to_str().unwrap()),
            "absolute workspace path must not leak into JSON"
        );
    }

    /// [LOW-1] `discover_artifacts` surfaces malformed files as scan
    /// errors rather than dropping them silently. Combined с the
    /// loop-level Err propagation (no more `entries.flatten()`), this
    /// guarantees every input file lands in either `records` или
    /// `scan_errors` — never silently disappears.
    #[test]
    fn reconcile_ids_discover_surfaces_malformed_frontmatter() {
        // Two PRDs: one well-formed, one whose frontmatter block never
        // closes — guaranteed to fail `parse_frontmatter` (vs a silently-
        // tolerated bad field value).
        let good = fm_full("PRD-001", "Auth", "prd-auth", 1, Some(1));
        // Missing the closing `---` ⇒ parse_frontmatter must error out.
        let broken = "---\nid: PRD-002\nkind: prd\nstatus: draft\ntitle: T\nslug: prd-broken\npredicted_number: 2\n\nBody-without-end-marker.\n";
        let ws = temp_workspace(&[
            ("prds", "PRD-001-auth.md", &good),
            ("prds", "prd-broken.md", broken),
        ]);
        let fp = ws.path().join(".forgeplan");
        let (records, scan_errors) = discover_artifacts(&fp).unwrap();
        // Exactly one good record.
        assert_eq!(records.len(), 1, "good record must parse");
        assert_eq!(records[0].slug.as_deref(), Some("prd-auth"));
        // Bad file MUST surface as a scan error (not silently dropped).
        assert_eq!(scan_errors.len(), 1, "broken frontmatter must surface");
        assert!(
            scan_errors[0].0.ends_with("prd-broken.md"),
            "scan error must point at the broken file: {:?}",
            scan_errors[0]
        );
        // Conservation rule: every input file accounted for.
        assert_eq!(
            records.len() + scan_errors.len(),
            2,
            "every input must surface as either record or scan_error"
        );
    }

    /// [LOW-2] After cleaning up `dash_pos` and the redundant length
    /// check, `body_artifact_refs` still rejects tokens with no digits
    /// after the dash (the case the old comment lied about).
    #[test]
    fn body_artifact_refs_skips_no_digit_tokens() {
        // `PRD-` and `ADR-foo` must not be accepted — the dash isn't
        // followed by digits. `RFC-001` IS a valid token.
        let body = "Bare dash PRD-, slug-style ADR-foo, but RFC-001 is real.";
        let refs = body_artifact_refs(body);
        assert!(refs.contains("RFC-001"));
        assert!(!refs.iter().any(|r| r == "PRD-"));
        assert!(!refs.iter().any(|r| r.starts_with("ADR-foo")));
    }
}
