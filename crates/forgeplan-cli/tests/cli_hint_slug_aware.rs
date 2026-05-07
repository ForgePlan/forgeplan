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
