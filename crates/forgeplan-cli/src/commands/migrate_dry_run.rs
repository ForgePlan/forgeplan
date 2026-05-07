//! PROB-060 / Phase 0b — EVID-C migration dry-run.
//!
//! Read-only scanner that walks `.forgeplan/<kind_dir>/*.md`, computes the
//! canonical slug each artifact would receive under SPEC-005 rules, and
//! detects per-kind collisions before Phase 4 migration.
//!
//! # Contracts
//! - Read-only — no mutation of any `.md` file (dry-run by definition).
//! - No `LanceStore::*` calls and no reads from `.forgeplan/lance/**` —
//!   the binary works directly off markdown source-of-truth (ADR-003).
//! - Hybrid resolution: default = fail-and-list (exit 1 on collision).
//!   Opt-in `--auto-suffix` adds `suggested_resolution` per collision.
//! - JSON output (when `--json`) conforms exactly to the CD-3 schema in
//!   the team-lead briefing (`schema_version: 1`).
//!
//! # Exit codes
//! - 0 — no collisions (greenlight Phase 4)
//! - 1 — collisions found
//! - 2 — scan error (no `.forgeplan/`, etc.)

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use clap::Args;
use forgeplan_core::artifact::frontmatter::{assigned_number_from_frontmatter, parse_frontmatter};
use forgeplan_core::artifact::types::{
    ArtifactKind, MAX_SLUG_LEN, MIN_SLUG_LEN, slug_from_kind_title, validate_slug,
};

/// CLI arguments for `forgeplan migrate-dry-run`.
#[derive(Debug, Clone, Args)]
pub struct MigrateDryRunArgs {
    /// Workspace root containing `.forgeplan/`. Default: current working dir
    /// (walk-up search like other commands).
    #[arg(long)]
    pub workspace: Option<PathBuf>,

    /// Include `suggested_resolution` per collision in the JSON output.
    /// Without this flag the run defaults to fail-and-list (exit 1).
    #[arg(long)]
    pub auto_suffix: bool,

    /// Also write the JSON report to this path (the same JSON as `--json`).
    /// Optional — caller chooses where to persist evidence.
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// Emit JSON to stdout. Default is a human-readable summary table.
    #[arg(long)]
    pub json: bool,
}

/// All kinds we scan. Mirrors `ArtifactKind` variants minus `Memory` —
/// memory is excluded from health/score/lifecycle and therefore not part
/// of slug-collision migration.
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

/// One artifact discovered on disk during the scan.
#[derive(Debug, Clone)]
pub struct ArtifactRecord {
    pub path: PathBuf,
    pub kind: ArtifactKind,
    pub title: String,
    pub assigned_number: Option<u32>,
    /// Existing slug from frontmatter, if present (Phase 1.x augments it).
    /// Currently unread by collision detection — kept for forward-compat
    /// with the Phase 4 migration script, which compares existing vs
    /// freshly-computed canonical slug to flag drift.
    #[allow(dead_code)]
    pub existing_slug: Option<String>,
}

/// Per-file scan failure — we record it and continue rather than abort.
#[derive(Debug, Clone)]
pub struct ScanError {
    pub path: PathBuf,
    pub reason: String,
}

/// One detected collision: ≥2 artifacts of same kind that would slugify
/// to the same canonical slug.
#[derive(Debug, Clone)]
pub struct Collision {
    pub kind: ArtifactKind,
    pub slug: String,
    pub members: Vec<ArtifactRecord>,
}

/// Aggregated collision report.
#[derive(Debug, Clone)]
pub struct CollisionReport {
    pub workspace: PathBuf,
    pub records: Vec<ArtifactRecord>,
    pub scan_errors: Vec<ScanError>,
    pub collisions: Vec<Collision>,
    /// Keyed by kind prefix (e.g. "prd", "rfc"). String keys avoid the need
    /// for `ArtifactKind: Ord` and produce stable JSON output ordering.
    pub per_kind_count: BTreeMap<String, usize>,
}

impl CollisionReport {
    pub fn total_artifacts(&self) -> usize {
        self.records.len()
    }
    pub fn total_collisions(&self) -> usize {
        self.collisions.len()
    }
    pub fn has_collisions(&self) -> bool {
        !self.collisions.is_empty()
    }
    pub fn kinds_with_collisions(&self) -> Vec<ArtifactKind> {
        let mut kinds: Vec<ArtifactKind> = self.collisions.iter().map(|c| c.kind.clone()).collect();
        kinds.sort_by_key(kind_sort_key);
        kinds.dedup();
        kinds
    }
}

/// Stable lexicographic key for `ArtifactKind`. We use the kind prefix
/// string (without trailing dash) so JSON ordering is deterministic
/// regardless of enum variant order.
fn kind_sort_key(k: &ArtifactKind) -> String {
    k.prefix().trim_end_matches('-').to_string()
}

/// Human-readable lowercase kind name used as JSON object key.
fn kind_key(k: &ArtifactKind) -> &'static str {
    k.prefix().trim_end_matches('-')
}

/// Scan all `<workspace>/<kind_dir>/*.md` files into `ArtifactRecord`s.
///
/// Tolerates per-file parse errors (records into `scan_errors`, continues).
/// Tolerates missing kind subdirectories (e.g. fresh workspace without any
/// notes yet). Returns an error only if the workspace itself is missing.
pub fn discover_artifacts(
    forgeplan_dir: &Path,
) -> anyhow::Result<(Vec<ArtifactRecord>, Vec<ScanError>)> {
    if !forgeplan_dir.is_dir() {
        anyhow::bail!(
            "workspace not found: {} is not a directory",
            forgeplan_dir.display()
        );
    }

    let mut records = Vec::new();
    let mut scan_errors = Vec::new();

    for kind in SCAN_KINDS {
        let kind_dir = forgeplan_dir.join(kind.dir_name());
        if !kind_dir.is_dir() {
            // Tolerated: not every workspace has every kind populated.
            continue;
        }
        let entries = match fs::read_dir(&kind_dir) {
            Ok(e) => e,
            Err(e) => {
                scan_errors.push(ScanError {
                    path: kind_dir.clone(),
                    reason: format!("read_dir failed: {e}"),
                });
                continue;
            }
        };
        for entry in entries.flatten() {
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
            match read_artifact_record(&path, kind) {
                Ok(r) => records.push(r),
                Err(e) => scan_errors.push(ScanError {
                    path: path.clone(),
                    reason: e.to_string(),
                }),
            }
        }
    }

    Ok((records, scan_errors))
}

/// Read a single `.md` file and extract the data needed for collision
/// detection. Title is required (collision detection is title-driven);
/// assigned_number is optional (only legacy artifacts with frontmatter
/// `assigned_number`, or filename-derived numbers, will have it).
fn read_artifact_record(path: &Path, kind: &ArtifactKind) -> anyhow::Result<ArtifactRecord> {
    let content = fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("read file {}: {e}", path.display()))?;
    let (fm, _body) = parse_frontmatter(&content)
        .map_err(|e| anyhow::anyhow!("parse frontmatter for {}: {e}", path.display()))?;

    let title = fm
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "frontmatter `title` field missing or non-string in {}",
                path.display()
            )
        })?;

    let assigned_number =
        assigned_number_from_frontmatter(&fm).or_else(|| extract_number_from_filename(path));

    let existing_slug = fm
        .get("slug")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(ArtifactRecord {
        path: path.to_path_buf(),
        kind: kind.clone(),
        title,
        assigned_number,
        existing_slug,
    })
}

/// Fallback: legacy artifacts written before SPEC-005 don't have
/// `assigned_number` in frontmatter. We extract it from the canonical
/// filename pattern `<KIND>-<NNN>-<slug>.md`. If parsing fails the field
/// is left `None` (the suggested-resolution algorithm degrades gracefully
/// — fallback to positional index suffix).
fn extract_number_from_filename(path: &Path) -> Option<u32> {
    let stem = path.file_stem()?.to_str()?;
    // Canonical layout: PRD-018-authentication. Split into 3 segments max
    // so a slug containing further dashes is preserved untouched.
    let mut parts = stem.splitn(3, '-');
    let _kind = parts.next()?; // e.g. "PRD"
    let num = parts.next()?; // e.g. "018"
    num.parse::<u32>().ok()
}

/// Group records by `(kind, candidate_slug)` and emit a `Collision` for
/// every group with ≥2 members. Within each collision, members are sorted
/// lexicographically by path so the JSON output is deterministic.
///
/// Records whose title fails `slug_from_kind_title` (e.g. pure-non-ASCII
/// title) are dropped from collision detection — they will fail Phase 4
/// migration on a different code path and surface as scan errors there.
pub fn detect_collisions(
    records: &[ArtifactRecord],
    scan_errors: &mut Vec<ScanError>,
) -> Vec<Collision> {
    use std::collections::HashMap;

    let mut groups: HashMap<(String, String), Vec<ArtifactRecord>> = HashMap::new();

    for record in records {
        let candidate_slug = match slug_from_kind_title(&record.kind, &record.title) {
            Ok(s) => s,
            Err(e) => {
                scan_errors.push(ScanError {
                    path: record.path.clone(),
                    reason: format!("slug_from_kind_title failed: {e}"),
                });
                continue;
            }
        };
        let key = (kind_sort_key(&record.kind), candidate_slug);
        groups.entry(key).or_default().push(record.clone());
    }

    let mut collisions: Vec<Collision> = groups
        .into_iter()
        .filter_map(|((_kind_str, slug), mut members)| {
            if members.len() < 2 {
                return None;
            }
            members.sort_by(|a, b| a.path.cmp(&b.path));
            let kind = members.first()?.kind.clone();
            Some(Collision {
                kind,
                slug,
                members,
            })
        })
        .collect();

    // Stable order: by kind prefix then slug.
    collisions.sort_by(|a, b| {
        kind_sort_key(&a.kind)
            .cmp(&kind_sort_key(&b.kind))
            .then(a.slug.cmp(&b.slug))
    });
    collisions
}

/// Hybrid auto-suffix proposal. First member (lex-sorted) keeps the slug,
/// subsequent members get `<slug>-<assigned_number>` (or `<slug>-<index>`
/// when `assigned_number` is absent). Each suggestion is then validated
/// via `validate_slug`; failing suggestions carry `validation_error`
/// instead of `new_slug`.
fn render_suggestions(collision: &Collision) -> Vec<serde_json::Value> {
    collision
        .members
        .iter()
        .enumerate()
        .map(|(idx, member)| {
            let proposed = if idx == 0 {
                collision.slug.clone()
            } else {
                let suffix = member
                    .assigned_number
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| idx.to_string());
                format!("{}-{}", collision.slug, suffix)
            };
            let path_str = member.path.display().to_string();
            match validate_slug(&proposed) {
                Ok(()) => serde_json::json!({
                    "path": path_str,
                    "new_slug": proposed,
                }),
                Err(e) => serde_json::json!({
                    "path": path_str,
                    "validation_error": e.to_string(),
                    "candidate": proposed,
                }),
            }
        })
        .collect()
}

/// Render the report as CD-3 JSON.
pub fn render_json(report: &CollisionReport, auto_suffix: bool) -> serde_json::Value {
    let scanned_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let exit_code = if report.has_collisions() { 1 } else { 0 };

    // Per-kind blocks. Iterate over SCAN_KINDS so output ordering is stable.
    // Kinds with zero artifacts are omitted from the JSON for compactness.
    let mut kinds_obj = serde_json::Map::new();
    for kind in SCAN_KINDS {
        let count = report
            .per_kind_count
            .get(&kind_sort_key(kind))
            .copied()
            .unwrap_or(0);
        if count == 0 {
            continue;
        }
        let collisions_for_kind: Vec<&Collision> = report
            .collisions
            .iter()
            .filter(|c| kind_sort_key(&c.kind) == kind_sort_key(kind))
            .collect();

        let mut collisions_json = Vec::new();
        for c in &collisions_for_kind {
            let mut entry = serde_json::Map::new();
            entry.insert(
                "slug".to_string(),
                serde_json::Value::String(c.slug.clone()),
            );
            entry.insert(
                "count".to_string(),
                serde_json::Value::Number(serde_json::Number::from(c.members.len())),
            );
            entry.insert(
                "artifacts".to_string(),
                serde_json::Value::Array(
                    c.members
                        .iter()
                        .map(|m| {
                            serde_json::json!({
                                "path": m.path.display().to_string(),
                                "assigned_number": m.assigned_number,
                                "title": m.title,
                            })
                        })
                        .collect(),
                ),
            );
            if auto_suffix {
                entry.insert(
                    "suggested_resolution".to_string(),
                    serde_json::Value::Array(render_suggestions(c)),
                );
            }
            collisions_json.push(serde_json::Value::Object(entry));
        }

        kinds_obj.insert(
            kind_key(kind).to_string(),
            serde_json::json!({
                "count": count,
                "collisions": collisions_json,
            }),
        );
    }

    let kinds_with_collisions: Vec<String> = report
        .kinds_with_collisions()
        .iter()
        .map(|k| kind_key(k).to_string())
        .collect();

    // PROB-060 Phase 0b Round 2 [SEC-5 CWE-200]: prefer workspace-relative
    // paths in scan_errors. Falls back to basename for paths that strip
    // empty (e.g. paths that aren't a child of `report.workspace`).
    let scan_errors_json: Vec<serde_json::Value> = report
        .scan_errors
        .iter()
        .map(|e| {
            let path_display = match e.path.strip_prefix(&report.workspace) {
                Ok(rel) => rel.display().to_string(),
                Err(_) => e
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "<unknown>".to_string()),
            };
            serde_json::json!({
                "path": path_display,
                "reason": e.reason,
            })
        })
        .collect();

    serde_json::json!({
        "schema_version": 1,
        "scanned_at": scanned_at,
        "workspace": report.workspace.display().to_string(),
        "total_artifacts": report.total_artifacts(),
        "kinds": serde_json::Value::Object(kinds_obj),
        "scan_errors": scan_errors_json,
        "summary": {
            "total_collisions": report.total_collisions(),
            "kinds_with_collisions": kinds_with_collisions,
            "exit_code": exit_code,
        }
    })
}

/// Render a short, scannable human summary.
pub fn render_human_summary(report: &CollisionReport) -> String {
    let mut out = String::new();
    out.push_str("Forgeplan migration dry-run (PROB-060 / EVID-C)\n");
    out.push_str(&format!("Workspace: {}\n", report.workspace.display()));
    out.push_str(&format!(
        "Scanned: {} artifacts across {} kinds\n",
        report.total_artifacts(),
        report.per_kind_count.len()
    ));
    out.push('\n');

    out.push_str("Per-kind counts:\n");
    for kind in SCAN_KINDS {
        if let Some(c) = report.per_kind_count.get(&kind_sort_key(kind))
            && *c > 0
        {
            out.push_str(&format!("  {:6} {}\n", kind_key(kind), c));
        }
    }
    out.push('\n');

    if report.scan_errors.is_empty() {
        out.push_str("Scan errors: 0\n");
    } else {
        out.push_str(&format!("Scan errors: {}\n", report.scan_errors.len()));
        for e in &report.scan_errors {
            out.push_str(&format!("  {}: {}\n", e.path.display(), e.reason));
        }
    }
    out.push('\n');

    if !report.has_collisions() {
        out.push_str("No collisions detected. Greenlight Phase 4.\n");
        return out;
    }

    out.push_str(&format!(
        "Collisions: {} (across kinds: {})\n",
        report.total_collisions(),
        report
            .kinds_with_collisions()
            .iter()
            .map(|k| kind_key(k))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    for c in &report.collisions {
        out.push_str(&format!(
            "  [{}] {} ({} members)\n",
            kind_key(&c.kind),
            c.slug,
            c.members.len()
        ));
        for m in &c.members {
            let n = m
                .assigned_number
                .map(|n| format!("#{n}"))
                .unwrap_or_else(|| "#?".to_string());
            out.push_str(&format!(
                "    - {} {} \"{}\"\n",
                n,
                m.path.display(),
                m.title
            ));
        }
    }
    out.push('\n');
    out.push_str("Run with --auto-suffix --json to emit suggested resolution slugs.\n");
    out
}

/// Resolve workspace `.forgeplan/` directory. `--workspace <PATH>` may
/// point either at the project root (containing `.forgeplan/`) OR at the
/// `.forgeplan/` directory itself.
///
/// PROB-060 Phase 0b Round 2 [E2E-3]: previously a `--workspace /tmp/empty`
/// argument was accepted as-is (returning the bare path), and the scan
/// silently produced 0 artifacts + exit 0. CD-3 contract requires exit 2
/// for scan errors, including missing `.forgeplan/`. We now require the
/// resolved path to *be* a `.forgeplan/` directory or to contain one.
fn resolve_forgeplan_dir(arg: Option<&Path>) -> anyhow::Result<PathBuf> {
    if let Some(p) = arg {
        let candidate = p.to_path_buf();
        if !candidate.is_dir() {
            anyhow::bail!("workspace path does not exist: {}", candidate.display());
        }
        // Project root containing .forgeplan/.
        let nested = candidate.join(".forgeplan");
        if nested.is_dir() {
            return Ok(nested);
        }
        // Direct .forgeplan/ pass-through (recognized by the directory
        // basename — we accept any directory literally named ".forgeplan").
        if candidate
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s == ".forgeplan")
            .unwrap_or(false)
        {
            return Ok(candidate);
        }
        anyhow::bail!(
            "migrate-dry-run: no .forgeplan/ directory found at {}",
            candidate.display()
        );
    }
    let cwd = std::env::current_dir()?;
    forgeplan_core::workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ workspace found from {}", cwd.display()))
}

/// Aggregate the report from raw scan data.
fn build_report(
    workspace: PathBuf,
    records: Vec<ArtifactRecord>,
    mut scan_errors: Vec<ScanError>,
) -> CollisionReport {
    let mut per_kind_count: BTreeMap<String, usize> = BTreeMap::new();
    for r in &records {
        *per_kind_count.entry(kind_sort_key(&r.kind)).or_insert(0) += 1;
    }
    let collisions = detect_collisions(&records, &mut scan_errors);
    CollisionReport {
        workspace,
        records,
        scan_errors,
        collisions,
        per_kind_count,
    }
}

/// Public entry point. Returns the desired process exit code.
///
/// Callers in `main.rs` should `std::process::exit(code)` after awaiting
/// this future so the shell sees the code (clap-derived `anyhow::Result<()>`
/// alone collapses 1/2 to a generic 1).
pub async fn run(args: MigrateDryRunArgs) -> anyhow::Result<i32> {
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

    let report = build_report(forgeplan_dir, records, scan_errors);
    let json = render_json(&report, args.auto_suffix);

    if args.json {
        let s = serde_json::to_string_pretty(&json)?;
        println!("{s}");
    } else {
        println!("{}", render_human_summary(&report));
    }

    if let Some(out_path) = args.output.as_deref() {
        let s = serde_json::to_string_pretty(&json)?;
        if let Some(parent) = out_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(out_path, s)?;
        eprintln!("Wrote JSON report to {}", out_path.display());
    }

    let exit_code = if report.has_collisions() { 1 } else { 0 };
    Ok(exit_code)
}

// Build-time sanity assertions.
const _: () = {
    assert!(MIN_SLUG_LEN >= 3);
    assert!(MAX_SLUG_LEN >= MIN_SLUG_LEN + 4);
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn temp_workspace(files: &[(&str, &str, &str)]) -> TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let fp = dir.path().join(".forgeplan");
        fs::create_dir_all(fp.join("prds")).unwrap();
        fs::create_dir_all(fp.join("rfcs")).unwrap();
        fs::create_dir_all(fp.join("notes")).unwrap();
        for (subdir, fname, content) in files {
            let p = fp.join(subdir).join(fname);
            fs::write(p, content).unwrap();
        }
        dir
    }

    fn frontmatter(id: &str, title: &str, assigned: Option<u32>) -> String {
        match assigned {
            Some(n) => format!(
                "---\nid: {id}\nkind: prd\nstatus: draft\ntitle: {title}\nassigned_number: {n}\n---\n\nBody.\n"
            ),
            None => {
                format!("---\nid: {id}\nkind: prd\nstatus: draft\ntitle: {title}\n---\n\nBody.\n")
            }
        }
    }

    #[test]
    fn discover_happy_path_no_collisions() {
        let ws = temp_workspace(&[
            (
                "prds",
                "PRD-001-auth.md",
                &frontmatter("PRD-001", "Auth System", Some(1)),
            ),
            (
                "prds",
                "PRD-002-billing.md",
                &frontmatter("PRD-002", "Billing service", Some(2)),
            ),
            (
                "rfcs",
                "RFC-001-tls.md",
                &frontmatter("RFC-001", "TLS Rollout", Some(1)),
            ),
        ]);
        let fp = ws.path().join(".forgeplan");
        let (records, errors) = discover_artifacts(&fp).unwrap();
        assert_eq!(records.len(), 3);
        assert!(errors.is_empty());
        let mut errs2 = Vec::new();
        let collisions = detect_collisions(&records, &mut errs2);
        assert!(collisions.is_empty());
        assert!(errs2.is_empty());
    }

    #[test]
    fn detect_collision_two_artifacts_same_title() {
        let ws = temp_workspace(&[
            (
                "prds",
                "PRD-018-auth.md",
                &frontmatter("PRD-018", "Authentication", Some(18)),
            ),
            (
                "prds",
                "PRD-042-auth-v2.md",
                &frontmatter("PRD-042", "Authentication", Some(42)),
            ),
        ]);
        let fp = ws.path().join(".forgeplan");
        let (records, _) = discover_artifacts(&fp).unwrap();
        let mut errs = Vec::new();
        let collisions = detect_collisions(&records, &mut errs);
        assert_eq!(collisions.len(), 1);
        assert_eq!(collisions[0].slug, "prd-authentication");
        assert_eq!(collisions[0].members.len(), 2);
        assert!(
            collisions[0].members[0]
                .path
                .to_string_lossy()
                .contains("PRD-018")
        );
        assert!(
            collisions[0].members[1]
                .path
                .to_string_lossy()
                .contains("PRD-042")
        );
    }

    #[test]
    fn auto_suffix_adds_suggested_resolution() {
        let ws = temp_workspace(&[
            (
                "prds",
                "PRD-018-auth.md",
                &frontmatter("PRD-018", "Authentication", Some(18)),
            ),
            (
                "prds",
                "PRD-042-auth-v2.md",
                &frontmatter("PRD-042", "Authentication", Some(42)),
            ),
        ]);
        let fp = ws.path().join(".forgeplan");
        let (records, errs) = discover_artifacts(&fp).unwrap();
        let report = build_report(fp, records, errs);
        let json = render_json(&report, true);
        let collisions = json["kinds"]["prd"]["collisions"].as_array().unwrap();
        assert_eq!(collisions.len(), 1);
        let resolution = collisions[0]["suggested_resolution"].as_array().unwrap();
        assert_eq!(resolution.len(), 2);
        assert_eq!(resolution[0]["new_slug"], "prd-authentication");
        assert_eq!(resolution[1]["new_slug"], "prd-authentication-42");

        let json_no_suffix = render_json(&report, false);
        let collisions_ns = json_no_suffix["kinds"]["prd"]["collisions"]
            .as_array()
            .unwrap();
        assert!(collisions_ns[0].get("suggested_resolution").is_none());
    }

    #[test]
    fn parse_error_tolerated_recorded_as_scan_error() {
        let ws = temp_workspace(&[
            (
                "prds",
                "PRD-001-good.md",
                &frontmatter("PRD-001", "Good", Some(1)),
            ),
            (
                "prds",
                "PRD-002-broken.md",
                "no frontmatter at all just text\n",
            ),
        ]);
        let fp = ws.path().join(".forgeplan");
        let (records, errors) = discover_artifacts(&fp).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].title, "Good");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].path.to_string_lossy().contains("PRD-002-broken"));
    }

    #[test]
    fn missing_workspace_returns_error_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        let nonexistent = dir.path().join(".forgeplan-does-not-exist");
        let err = discover_artifacts(&nonexistent).unwrap_err();
        assert!(err.to_string().contains("workspace not found"));
    }

    #[test]
    fn missing_kind_subdirs_tolerated() {
        let dir = tempfile::tempdir().unwrap();
        let fp = dir.path().join(".forgeplan");
        fs::create_dir_all(fp.join("prds")).unwrap();
        fs::write(
            fp.join("prds").join("PRD-001-x.md"),
            frontmatter("PRD-001", "X", Some(1)),
        )
        .unwrap();
        let (records, errors) = discover_artifacts(&fp).unwrap();
        assert_eq!(records.len(), 1);
        assert!(errors.is_empty());
    }

    #[test]
    fn extract_number_from_filename_canonical() {
        let p = PathBuf::from("/x/y/PRD-018-authentication.md");
        assert_eq!(extract_number_from_filename(&p), Some(18));
        let p = PathBuf::from("/x/y/RFC-009-migration-rollout.md");
        assert_eq!(extract_number_from_filename(&p), Some(9));
    }

    #[test]
    fn extract_number_from_filename_non_canonical_returns_none() {
        let p = PathBuf::from("/x/y/prd-auth-system.md");
        assert_eq!(extract_number_from_filename(&p), None);
        let p = PathBuf::from("/x/y/no-dashes.md");
        assert_eq!(extract_number_from_filename(&p), None);
    }

    #[test]
    fn render_human_summary_no_collisions_says_greenlight() {
        let ws = temp_workspace(&[(
            "prds",
            "PRD-001-x.md",
            &frontmatter("PRD-001", "X", Some(1)),
        )]);
        let fp = ws.path().join(".forgeplan");
        let (records, errs) = discover_artifacts(&fp).unwrap();
        let report = build_report(fp, records, errs);
        let s = render_human_summary(&report);
        assert!(s.contains("Greenlight Phase 4"));
        assert!(s.contains("prd"));
    }

    #[test]
    fn render_json_schema_v1_shape() {
        let ws = temp_workspace(&[(
            "prds",
            "PRD-001-x.md",
            &frontmatter("PRD-001", "X", Some(1)),
        )]);
        let fp = ws.path().join(".forgeplan");
        let (records, errs) = discover_artifacts(&fp).unwrap();
        let report = build_report(fp, records, errs);
        let json = render_json(&report, false);
        assert_eq!(json["schema_version"], 1);
        assert!(json["scanned_at"].is_string());
        assert!(json["workspace"].is_string());
        assert_eq!(json["total_artifacts"], 1);
        assert_eq!(json["summary"]["exit_code"], 0);
        assert_eq!(json["summary"]["total_collisions"], 0);
        assert!(json["kinds"]["prd"].is_object());
    }

    #[test]
    fn collision_resolution_falls_back_to_index_when_assigned_missing() {
        let ws = temp_workspace(&[
            (
                "prds",
                "prd-a.md",
                "---\nid: PRD-001\ntitle: Cat\nstatus: draft\n---\nB.\n",
            ),
            (
                "prds",
                "prd-b.md",
                "---\nid: PRD-002\ntitle: Cat\nstatus: draft\n---\nB.\n",
            ),
        ]);
        let fp = ws.path().join(".forgeplan");
        let (records, errs) = discover_artifacts(&fp).unwrap();
        let report = build_report(fp, records, errs);
        let json = render_json(&report, true);
        let collisions = json["kinds"]["prd"]["collisions"].as_array().unwrap();
        assert_eq!(collisions.len(), 1);
        let res = collisions[0]["suggested_resolution"].as_array().unwrap();
        assert_eq!(res[0]["new_slug"], "prd-cat");
        assert_eq!(res[1]["new_slug"], "prd-cat-1");
    }

    /// PROB-060 Phase 0b Round 2 [E2E-3]: a workspace path that exists
    /// as a directory but has no `.forgeplan/` subdir must be rejected
    /// with exit 2 (scan error), not silently treated as «0 artifacts».
    #[test]
    fn run_returns_exit_2_when_forgeplan_dir_missing() {
        let dir = tempfile::tempdir().unwrap();
        // dir.path() exists but contains no `.forgeplan/`.
        let args = MigrateDryRunArgs {
            workspace: Some(dir.path().to_path_buf()),
            auto_suffix: false,
            output: None,
            json: true,
        };
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let exit = rt.block_on(async { run(args).await.unwrap() });
        assert_eq!(exit, 2, "missing .forgeplan/ must return exit 2");
    }

    #[test]
    fn report_exit_code_one_when_collisions_present() {
        let ws = temp_workspace(&[
            (
                "prds",
                "PRD-001-foo.md",
                &frontmatter("PRD-001", "Same Title", Some(1)),
            ),
            (
                "prds",
                "PRD-002-bar.md",
                &frontmatter("PRD-002", "Same Title", Some(2)),
            ),
        ]);
        let fp = ws.path().join(".forgeplan");
        let (records, errs) = discover_artifacts(&fp).unwrap();
        let report = build_report(fp, records, errs);
        let json = render_json(&report, false);
        assert_eq!(json["summary"]["exit_code"], 1);
        assert_eq!(json["summary"]["total_collisions"], 1);
    }
}
