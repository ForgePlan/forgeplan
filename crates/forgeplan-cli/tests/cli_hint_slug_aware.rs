//! PROB-060 / SPEC-005 / ADR-012 (Phase 2 W1.B, CD-5) — slug-aware hints.
//!
//! Integration tests verifying that CLI commands emit the canonical
//! reference form in their `Next:` lines and JSON `_next_action` field:
//!
//! - **Pre-merge** artifacts (`assigned_number: null`) → slug
//!   (`prd-hint-pre-merge-fixture`).
//! - **Post-merge** artifacts (`assigned_number: 74`) → display id
//!   (`PRD-001`).
//!
//! The contract is exercised end-to-end through real subprocess invocation
//! so any regression — e.g. a new command emitting `record.id` directly
//! instead of routing through `refs_form_from_body` — surfaces here.
//!
//! Reference: `docs/methodology/agent-protocol.md`, CD-5 in
//! `docs/sessions/2026-05-07-PROB-060-phase-2-3-4-handoff.md`.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Initialise a workspace and create one PRD; return the temp dir + the
/// canonical id the CLI assigned (e.g. `PRD-001`).
fn workspace_with_one_prd(title: &str) -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(dir.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", title])
        .current_dir(dir.path())
        .assert()
        .success();
    // Phase 1: `forgeplan new` mints PRD-001 immediately; this is the
    // post-merge form (`assigned_number == predicted_number`).
    (dir, "PRD-001".to_string())
}

/// Read the on-disk markdown body for a known PRD created via `forgeplan new`.
/// The filename pattern is `<KIND>-<NNN>-<title-slug>.md`.
fn read_prd_file(workspace: &Path) -> (std::path::PathBuf, String) {
    let prd_dir = workspace.join(".forgeplan").join("prds");
    let mut entries: Vec<_> = fs::read_dir(&prd_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    entries.sort();
    let path = entries
        .into_iter()
        .next()
        .expect("expected at least one PRD file");
    let body = fs::read_to_string(&path).unwrap();
    (path, body)
}

/// Force `assigned_number: null` on an existing PRD so subsequent commands
/// see it as pre-merge. Mutates the markdown directly (test-only — the
/// production path goes through CI bot's `set_assigned_number`). After this
/// runs we re-import via `forgeplan scan-import` so LanceDB matches disk.
///
/// PRD-073 file layout: `forgeplan new` writes a *two-block* markdown —
/// the projection layer (synthetic, top) and the canonical body (bottom,
/// with `slug` + `assigned_number`). We mutate every `assigned_number:`
/// line so both blocks stay consistent and `parse_frontmatter` sees a
/// pre-merge artifact regardless of which block it picks up.
fn make_pre_merge(workspace: &Path) {
    let (path, body) = read_prd_file(workspace);
    let mut updated = String::new();
    for line in body.lines() {
        if line.starts_with("assigned_number:") {
            updated.push_str("assigned_number: null\n");
        } else {
            updated.push_str(line);
            updated.push('\n');
        }
    }
    fs::write(&path, updated).unwrap();
    // `reindex` propagates body changes from disk into LanceDB. `scan-import`
    // alone reports "no changes" because filename + id are unchanged — only
    // the canonical body's `assigned_number` line flipped.
    forgeplan()
        .arg("reindex")
        .current_dir(workspace)
        .assert()
        .success();
}

/// Read the canonical slug for a PRD via `forgeplan get --json` (the slug
/// lives in the canonical body's frontmatter, which is the *second* block
/// of the rendered markdown — `parse_frontmatter` on the file would pick
/// the projection-layer block which has no slug; the JSON path reads
/// LanceDB which stores the canonical body).
fn slug_for(workspace: &Path, id_or_slug: &str) -> String {
    let out = forgeplan()
        .args(["get", id_or_slug, "--json"])
        .current_dir(workspace)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    json["slug"]
        .as_str()
        .expect("forgeplan get --json must surface the slug for Phase 1+ artifacts")
        .to_string()
}

#[test]
fn get_emits_display_id_for_post_merge_artifact() {
    let (dir, id) = workspace_with_one_prd("Hint Post Merge Fixture");

    let out = forgeplan()
        .args(["get", &id])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    let next_lines: Vec<&str> = s
        .lines()
        .filter(|l| l.trim_start().starts_with("Next:"))
        .collect();
    assert_eq!(
        next_lines.len(),
        1,
        "expected exactly one Next: line, got: {s}"
    );
    let next = next_lines[0];
    assert!(
        next.contains("PRD-001"),
        "post-merge Next: must reference display id, got: {next}"
    );
}

#[test]
fn get_emits_slug_for_pre_merge_artifact() {
    let (dir, _id) = workspace_with_one_prd("Hint Pre Merge Fixture");
    make_pre_merge(dir.path());
    let slug = slug_for(dir.path(), "PRD-001");

    // Reach the artifact via slug — the resolver must accept either form
    // (Phase 1.5b precondition for this test).
    let out = forgeplan()
        .args(["get", &slug])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    let next_lines: Vec<&str> = s
        .lines()
        .filter(|l| l.trim_start().starts_with("Next:"))
        .collect();
    assert_eq!(
        next_lines.len(),
        1,
        "expected exactly one Next: line, got: {s}"
    );
    let next = next_lines[0];
    assert!(
        next.contains(&slug),
        "pre-merge Next: must reference slug `{slug}`, got: {next}"
    );
    assert!(
        !next.contains("PRD-001"),
        "pre-merge Next: must NOT reference the (unstable) display id, got: {next}"
    );
}

#[test]
fn get_json_next_action_uses_slug_pre_merge() {
    let (dir, _id) = workspace_with_one_prd("Hint JSON Pre Merge Fixture");
    make_pre_merge(dir.path());
    let slug = slug_for(dir.path(), "PRD-001");

    let out = forgeplan()
        .args(["get", &slug, "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let next = json["_next_action"]
        .as_str()
        .expect("_next_action must be a string for this fixture");
    assert!(
        next.contains(&slug),
        "pre-merge _next_action must reference slug `{slug}`, got: {next}"
    );
    assert!(
        !next.contains("PRD-001"),
        "pre-merge _next_action must NOT reference display id, got: {next}"
    );
}

#[test]
fn list_first_get_hint_uses_slug_pre_merge() {
    let (dir, _id) = workspace_with_one_prd("Hint List First Pre Merge");
    make_pre_merge(dir.path());
    let slug = slug_for(dir.path(), "PRD-001");

    let out = forgeplan()
        .arg("list")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    let next_lines: Vec<&str> = s
        .lines()
        .filter(|l| l.trim_start().starts_with("Next:"))
        .collect();
    assert_eq!(
        next_lines.len(),
        1,
        "expected exactly one Next: line, got:\n{s}"
    );
    let next = next_lines[0];
    assert!(
        next.contains(&slug),
        "pre-merge list Next: must reference slug `{slug}`, got: {next}"
    );
}

// ── PROB-060 Phase 2 audit closure (CRIT-3): W3 commands. ──────────────
//
// The W3 batch (Phase 2.6 / CD-6) wired `resolve_id` into 13 CLI commands
// but several of them still emitted `record.id` directly into their hint
// `Next:` line — the audit caught the regression. The tests below are the
// regression guard: each command must emit `slug` pre-merge and the
// display id post-merge.

fn extract_next_line(stdout: &[u8]) -> String {
    let s = String::from_utf8(stdout.to_vec()).unwrap();
    let next_lines: Vec<&str> = s
        .lines()
        .filter(|l| l.trim_start().starts_with("Next:"))
        .collect();
    assert_eq!(
        next_lines.len(),
        1,
        "expected exactly one Next: line, got:\n{s}"
    );
    next_lines[0].to_string()
}

#[test]
fn update_emits_slug_pre_merge_for_validate_hint() {
    // CRIT-3 — `forgeplan update` was using `record.id` for the
    // `forgeplan validate` next-action. Verify the slug-aware refactor.
    let (dir, _id) = workspace_with_one_prd("Hint Update Pre Merge");
    make_pre_merge(dir.path());
    let slug = slug_for(dir.path(), "PRD-001");

    let out = forgeplan()
        .args(["update", &slug, "--title", "Updated Title"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    assert!(
        next.contains(&slug),
        "pre-merge update Next: must reference slug `{slug}`, got: {next}"
    );
    assert!(
        !next.contains("PRD-001"),
        "pre-merge update Next: must NOT reference display id, got: {next}"
    );
}

#[test]
fn update_emits_display_id_post_merge() {
    // Counter-test: post-merge artifact (assigned_number set) routes
    // through `record.id` fallback → display id wins.
    let (dir, id) = workspace_with_one_prd("Hint Update Post Merge");
    let out = forgeplan()
        .args(["update", &id, "--title", "Updated Title"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    assert!(
        next.contains("PRD-001"),
        "post-merge update Next: must reference display id, got: {next}"
    );
}

#[test]
fn renew_emits_slug_pre_merge_for_score_hint() {
    // CRIT-3 — `forgeplan renew` previously used `id` directly. After the
    // fix the score command must use slug pre-merge.
    let (dir, _id) = workspace_with_one_prd("Hint Renew Pre Merge");
    // Ensure stale state via lifecycle: the artifact is fresh-created
    // (status=draft, valid_until=None). renew refreshes valid_until from
    // any state per lifecycle::renew. We force pre-merge after.
    make_pre_merge(dir.path());
    let slug = slug_for(dir.path(), "PRD-001");

    let out = forgeplan()
        .args([
            "renew",
            &slug,
            "--reason",
            "extending review window",
            "--until",
            "2099-01-01",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    assert!(
        next.contains(&slug),
        "pre-merge renew Next: must reference slug `{slug}`, got: {next}"
    );
    assert!(
        !next.contains("PRD-001"),
        "pre-merge renew Next: must NOT reference display id, got: {next}"
    );
}

#[test]
fn estimate_emits_slug_pre_merge_for_calibrate_hint() {
    // CRIT-3 — `forgeplan estimate` emitted `record.id` for the
    // `calibrate-estimate` follow-up. With the fix the slug must win.
    let (dir, _id) = workspace_with_one_prd("Hint Estimate Pre Merge");
    make_pre_merge(dir.path());
    let slug = slug_for(dir.path(), "PRD-001");

    // estimate may emit empty-items hint (`forgeplan get …`) if there are
    // no FR/Phase items in the body — both paths must be slug-aware. The
    // fixture has no FR sections so we exercise the empty-items branch.
    let out = forgeplan()
        .args(["estimate", &slug])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    assert!(
        next.contains(&slug),
        "pre-merge estimate Next: must reference slug `{slug}`, got: {next}"
    );
    assert!(
        !next.contains("PRD-001"),
        "pre-merge estimate Next: must NOT reference display id, got: {next}"
    );
}

#[test]
fn fgr_emits_slug_pre_merge_for_lowest_hint() {
    // CRIT-3 — `forgeplan fgr` lowest-grade hint emitted `record.id`.
    // Verify slug-aware after the fix on text path.
    let (dir, _id) = workspace_with_one_prd("Hint FGR Pre Merge");
    make_pre_merge(dir.path());
    let slug = slug_for(dir.path(), "PRD-001");

    let out = forgeplan()
        .args(["fgr"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    assert!(
        next.contains(&slug),
        "pre-merge fgr Next: must reference slug `{slug}`, got: {next}"
    );
    assert!(
        !next.contains("PRD-001"),
        "pre-merge fgr Next: must NOT reference display id, got: {next}"
    );
}

#[test]
fn delete_emits_slug_pre_merge_for_restore_hint() {
    // CRIT-3 — `forgeplan delete` referenced raw `id` in its restore
    // follow-up. With the slug-aware fix the restore command stays
    // canonical pre-merge.
    let (dir, _id) = workspace_with_one_prd("Hint Delete Pre Merge");
    make_pre_merge(dir.path());
    let slug = slug_for(dir.path(), "PRD-001");

    let out = forgeplan()
        .args(["delete", &slug, "--yes"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    assert!(
        next.contains(&slug),
        "pre-merge delete Next: must reference slug `{slug}`, got: {next}"
    );
    assert!(
        !next.contains("PRD-001"),
        "pre-merge delete Next: must NOT reference display id, got: {next}"
    );
}

// ── PROB-060 Phase 2.1 audit closure (Round 2 Code FINDING-2): six W3 commands
// missing slug-aware regression coverage. ─────────────────────────────────
//
// Round 1 CRIT-3 fix touched 13 commands but the test suite only covered 7.
// The tests below close that gap: supersede, reopen, claim, release,
// calibrate-estimate, import. For at least 2 of them we also add a
// post-merge counterpart so the display-id branch stays exercised.

/// Force `assigned_number: null` on EVERY PRD on disk (multi-artifact
/// pre-merge fixture for supersede / link-style commands). Mirrors
/// `make_pre_merge` but iterates all `.md` files в `.forgeplan/prds/`.
fn make_all_prds_pre_merge(workspace: &Path) {
    let prd_dir = workspace.join(".forgeplan").join("prds");
    let entries: Vec<_> = fs::read_dir(&prd_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    for path in entries {
        let body = fs::read_to_string(&path).unwrap();
        let mut updated = String::new();
        for line in body.lines() {
            if line.starts_with("assigned_number:") {
                updated.push_str("assigned_number: null\n");
            } else {
                updated.push_str(line);
                updated.push('\n');
            }
        }
        fs::write(&path, updated).unwrap();
    }
    forgeplan()
        .arg("reindex")
        .current_dir(workspace)
        .assert()
        .success();
}

/// Create a workspace with two PRDs (`PRD-001`, `PRD-002`) — used by
/// supersede where both source и target need to exist.
fn workspace_with_two_prds(title_a: &str, title_b: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(dir.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", title_a])
        .current_dir(dir.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", title_b])
        .current_dir(dir.path())
        .assert()
        .success();
    dir
}

/// Force activation via `forgeplan activate --force` (bypasses the
/// validation gate's MUST-section check; the lifecycle state machine
/// for supersede / reopen only requires `status=active|stale`).
fn force_active(workspace: &Path, ids: &[&str]) {
    for id in ids {
        forgeplan()
            .args(["activate", id, "--force"])
            .current_dir(workspace)
            .assert()
            .success();
    }
}

#[test]
fn supersede_emits_slug_pre_merge_for_successor_hint() {
    // CRIT-3 / Round 2 FINDING-2 — `forgeplan supersede` emits a `Next:
    // forgeplan get <by>` hint that must reference the canonical form for
    // the successor (slug pre-merge, display id post-merge). Both source
    // и target must be slug-aware.
    //
    // supersede requires `status=active` on the source per lifecycle gate —
    // we force-activate by mutating disk + reindex (test-only bypass).
    let dir = workspace_with_two_prds("Supersede Source Foxglove", "Supersede Target Periwinkle");
    force_active(dir.path(), &["PRD-001", "PRD-002"]);
    make_all_prds_pre_merge(dir.path());
    let source_slug = slug_for(dir.path(), "PRD-001");
    let target_slug = slug_for(dir.path(), "PRD-002");
    assert_ne!(source_slug, target_slug, "fixtures must produce two slugs");

    let out = forgeplan()
        .args(["supersede", &source_slug, "--by", &target_slug])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    assert!(
        next.contains(&target_slug),
        "pre-merge supersede Next: must reference target slug `{target_slug}`, got: {next}"
    );
    assert!(
        !next.contains("PRD-002"),
        "pre-merge supersede Next: must NOT reference display id of target, got: {next}"
    );
}

#[test]
fn supersede_emits_display_id_post_merge_for_successor_hint() {
    // Counter-test: post-merge artifacts have stable `assigned_number`
    // matching their `predicted_number`, so refs_form_from_body falls
    // back to the display id. The successor hint must use it.
    //
    // Use distinct titles (cosine similarity gate refuses near-duplicates
    // в a non-interactive shell). Force-activate to satisfy supersede gate.
    let dir = workspace_with_two_prds("Supersede Post Boatswain", "Supersede Post Quartermaster");
    force_active(dir.path(), &["PRD-001", "PRD-002"]);

    let out = forgeplan()
        .args(["supersede", "PRD-001", "--by", "PRD-002"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    assert!(
        next.contains("PRD-002"),
        "post-merge supersede Next: must reference target display id, got: {next}"
    );
}

#[test]
fn reopen_emits_slug_pre_merge_for_validate_hint() {
    // CRIT-3 / Round 2 FINDING-2 — `forgeplan reopen` mints a new draft
    // and emits `forgeplan validate <new>` as the next action. reopen
    // requires source status active|stale per lifecycle gate; we
    // force-activate then flip to pre-merge to exercise the slug-aware
    // path on the source.
    //
    // The freshly-minted child has its body assembled by lifecycle::reopen
    // and includes a slug for the new id (templates write slug into the
    // canonical body). After reopen we read the new artifact's slug via
    // refs_form_from_body — the validate hint must reference it (or
    // fall back to the display id when no slug is present yet).
    let (dir, _id) = workspace_with_one_prd("Reopen Pre Merge Cinquefoil");
    force_active(dir.path(), &["PRD-001"]);
    make_pre_merge(dir.path());
    let source_slug = slug_for(dir.path(), "PRD-001");

    let out = forgeplan()
        .args(["reopen", &source_slug, "--reason", "audit cycle"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    // The new draft is `PRD-002` (next id in sequence). The Next: hint
    // points to validate of the new draft. The pre-merge source slug
    // must not leak through (otherwise the operator would validate the
    // wrong artifact). The hint must reference either the new artifact's
    // slug (if assigned_number is null in the freshly-rendered body)
    // OR its display id (PRD-002) — both forms are canonical for the
    // new artifact, both are NOT the source slug.
    assert!(
        !next.contains(&source_slug),
        "reopen Next: must NOT reference source pre-merge slug `{source_slug}`, got: {next}"
    );
    assert!(
        next.contains("PRD-002") || next.contains("prd-"),
        "reopen Next: must reference the new draft (PRD-002 or slug), got: {next}"
    );
}

#[test]
fn claim_emits_slug_pre_merge_for_inspect_hint() {
    // Round 2 FINDING-2 — `forgeplan claim` emits a `Next: forgeplan get
    // <ref_form>` hint that must use the slug pre-merge.
    let (dir, _id) = workspace_with_one_prd("Hint Claim Pre Merge");
    make_pre_merge(dir.path());
    let slug = slug_for(dir.path(), "PRD-001");

    let out = forgeplan()
        .args(["claim", &slug, "--agent", "test/cli", "--ttl-minutes", "5"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    assert!(
        next.contains(&slug),
        "pre-merge claim Next: must reference slug `{slug}`, got: {next}"
    );
    assert!(
        !next.contains("PRD-001"),
        "pre-merge claim Next: must NOT reference display id, got: {next}"
    );
}

#[test]
fn claim_already_held_emits_slug_pre_merge_in_release_hint() {
    // Round 2 FINDING-2 — AlreadyHeld branch on stderr must surface a
    // `Fix: forgeplan release <ref_form> --force` line that uses the
    // slug pre-merge. Since claim conflict prints the override hint to
    // stderr (and exits 1), we read stderr instead of stdout.
    let (dir, _id) = workspace_with_one_prd("Hint Claim Held Pre Merge");
    make_pre_merge(dir.path());
    let slug = slug_for(dir.path(), "PRD-001");

    // First claim succeeds.
    forgeplan()
        .args([
            "claim",
            &slug,
            "--agent",
            "first/agent",
            "--ttl-minutes",
            "30",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // Second claim with a different agent triggers AlreadyHeld → exit 1.
    let output = forgeplan()
        .args([
            "claim",
            &slug,
            "--agent",
            "second/agent",
            "--ttl-minutes",
            "30",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "claim conflict must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let fix_line = stderr
        .lines()
        .find(|l| l.trim_start().starts_with("Fix:"))
        .unwrap_or_else(|| panic!("expected `Fix:` line on AlreadyHeld stderr, got:\n{stderr}"));
    assert!(
        fix_line.contains(&slug),
        "pre-merge claim conflict Fix: must reference slug `{slug}`, got: {fix_line}"
    );
    assert!(
        !fix_line.contains("PRD-001"),
        "pre-merge claim conflict Fix: must NOT reference display id, got: {fix_line}"
    );
}

#[test]
fn release_emits_dispatch_hint_pre_merge_without_id_leak() {
    // Round 2 FINDING-2 — `forgeplan release` happy-path emits a
    // `Next: forgeplan dispatch …` hint that does NOT reference any
    // single artifact (it asks the orchestrator to re-plan). The
    // regression we guard: the success branch must not accidentally
    // leak the display id into the printed text body either, because
    // the operator is steering by slug pre-merge.
    let (dir, _id) = workspace_with_one_prd("Hint Release Pre Merge");
    make_pre_merge(dir.path());
    let slug = slug_for(dir.path(), "PRD-001");

    // Claim then release with the SAME agent — happy path.
    forgeplan()
        .args([
            "claim",
            &slug,
            "--agent",
            "owner/cli",
            "--ttl-minutes",
            "30",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["release", &slug, "--agent", "owner/cli"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    // Dispatch hint never carries an artifact id — guard that it didn't
    // accidentally inherit a display-id substring from a stale code path.
    assert!(
        next.contains("forgeplan dispatch"),
        "release Next: must point at dispatch, got: {next}"
    );
    assert!(
        !next.contains("PRD-001"),
        "release Next: must NOT leak display id of released artifact, got: {next}"
    );
}

#[test]
fn release_not_held_emits_slug_pre_merge_in_force_hint() {
    // Round 2 FINDING-2 — NotHeldByRequester branch on stderr emits a
    // `Fix: forgeplan release <ref_form> --force` line. The pre-merge
    // ref_form is the slug; verify the AlreadyHeld → release fix path
    // stays canonical so an operator can copy-paste без display-id leak.
    let (dir, _id) = workspace_with_one_prd("Hint Release Not Held Pre Merge");
    make_pre_merge(dir.path());
    let slug = slug_for(dir.path(), "PRD-001");

    // First agent claims.
    forgeplan()
        .args([
            "claim",
            &slug,
            "--agent",
            "first/cli",
            "--ttl-minutes",
            "30",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // Different agent tries to release without --force → NotHeldByRequester.
    let output = forgeplan()
        .args(["release", &slug, "--agent", "intruder/cli"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "release-by-non-holder must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let fix_line = stderr
        .lines()
        .find(|l| l.trim_start().starts_with("Fix:"))
        .unwrap_or_else(|| panic!("expected `Fix:` line on NotHeld stderr, got:\n{stderr}"));
    assert!(
        fix_line.contains(&slug),
        "pre-merge release Fix: must reference slug `{slug}`, got: {fix_line}"
    );
    assert!(
        !fix_line.contains("PRD-001"),
        "pre-merge release Fix: must NOT reference display id, got: {fix_line}"
    );
}

/// Inject an FR table (3 rows) into a PRD body so calibrate-estimate
/// can produce a non-zero estimate and we exercise the success-path
/// hint chain. Mirrors the `make_pre_merge` mutation pattern: edit the
/// markdown file directly + `reindex` so LanceDB matches disk.
fn inject_fr_table(workspace: &Path) {
    let (path, body) = read_prd_file(workspace);
    let mut updated = body;
    let fr_block = "\n\n## Functional Requirements\n\n\
        | ID | Category | Priority | Requirement | Journey |\n\
        |----|----------|----------|-------------|---------|\n\
        | FR-001 | Core | Must | User can do X | Journey 1 |\n\
        | FR-002 | Core | Must | User can do Y | Journey 1 |\n\
        | FR-003 | UI | Should | Display Z | Journey 2 |\n";
    updated.push_str(fr_block);
    fs::write(&path, updated).unwrap();
    forgeplan()
        .arg("reindex")
        .current_dir(workspace)
        .assert()
        .success();
}

#[test]
fn calibrate_estimate_emits_slug_pre_merge_for_followup_hint() {
    // Round 2 FINDING-2 — `forgeplan calibrate-estimate` success path
    // emits one of three follow-up hints (`forgeplan estimate … --my-grade`,
    // `forgeplan score …`) all of which must use slug pre-merge. To
    // exercise the success path we inject FR rows so the estimator
    // produces a non-zero value, then force pre-merge form.
    let (dir, _id) = workspace_with_one_prd("Calibrate Pre Merge Saxifrage");
    inject_fr_table(dir.path());
    make_pre_merge(dir.path());
    let slug = slug_for(dir.path(), "PRD-001");

    let out = forgeplan()
        .args(["calibrate-estimate", &slug, "--actual-hours", "8"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    assert!(
        next.contains(&slug),
        "pre-merge calibrate-estimate Next: must reference slug `{slug}`, got: {next}"
    );
    assert!(
        !next.contains("PRD-001"),
        "pre-merge calibrate-estimate Next: must NOT reference display id, got: {next}"
    );
}

#[test]
fn calibrate_estimate_emits_display_id_post_merge_for_followup_hint() {
    // Counter-test: post-merge artifact reuses display id form on the
    // success-path follow-up hint.
    let (dir, id) = workspace_with_one_prd("Calibrate Post Merge Tarragon");
    inject_fr_table(dir.path());

    let out = forgeplan()
        .args(["calibrate-estimate", &id, "--actual-hours", "8"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    assert!(
        next.contains("PRD-001"),
        "post-merge calibrate-estimate Next: must reference display id, got: {next}"
    );
}

#[test]
fn import_post_run_hint_does_not_leak_display_id_pre_merge() {
    // Round 2 FINDING-2 — `forgeplan import` emits a single
    // `Next: forgeplan health` hint after re-rendering each imported
    // artifact's projection. The re-render path is per-artifact and
    // routes through `projection::create_artifact_with_projection`, so
    // there's no per-id hint at the import boundary itself — the guard
    // here is that the post-run hint (a) exists and (b) does NOT
    // accidentally embed any display id from the imported artifact set.
    //
    // Setup: create a PRD, force pre-merge, export → fresh workspace,
    // import. The imported artifact carries `assigned_number: null` so
    // any future hint chain in the workspace surfaces slug refs.
    let (origin_dir, _id) = workspace_with_one_prd("Hint Import Pre Merge");
    make_pre_merge(origin_dir.path());
    let slug = slug_for(origin_dir.path(), "PRD-001");

    // Export to JSON.
    let export_path = origin_dir.path().join("backup.json");
    forgeplan()
        .args(["export", "--output", export_path.to_str().unwrap()])
        .current_dir(origin_dir.path())
        .assert()
        .success();
    let exported = fs::read_to_string(&export_path).unwrap();
    assert!(
        exported.contains(&slug),
        "export must carry slug `{slug}` для downstream import re-render"
    );

    // Fresh workspace + import.
    let dest = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(dest.path())
        .assert()
        .success();
    let dest_export = dest.path().join("backup.json");
    fs::copy(&export_path, &dest_export).unwrap();

    let out = forgeplan()
        .args(["import", dest_export.to_str().unwrap(), "--force"])
        .current_dir(dest.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let next = extract_next_line(&out);
    assert!(
        next.contains("forgeplan health"),
        "import Next: must point at health, got: {next}"
    );
    // Negative guard: import's success message printed before Next:
    // sometimes echoes counts; the Next: line specifically must stay
    // free of any artifact id leak (display or slug).
    assert!(
        !next.contains("PRD-"),
        "import Next: must NOT leak any display id of imported artifact, got: {next}"
    );
}
