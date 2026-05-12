//! PROB-060 Phase 2.5 — extended resolver coverage для 5 indirectly-tested
//! commands.
//!
//! Phase 2.6 (CD-6) wired resolver into 13 CLI verbs, covered by
//! `cli_resolver_wiring.rs` (26 tests). Phase 1.5b earlier wired 6 verbs
//! (get/validate/activate/deprecate/link/score), covered by their own
//! integration tests. Phase 2.5 closes the remaining gap: 5 verbs that
//! transitively use the same `LanceStore::resolve_id` через get_record /
//! lifecycle helpers, but had no explicit slug-aware regression test:
//!
//! - `phase`         — read advisory phase state по artifact ID
//! - `phase-advance` — write advisory phase transition (--to <phase>)
//! - `progress`      — read progress checkbox state по artifact ID (optional)
//! - `review`        — lifecycle review report по artifact ID
//! - `restore`       — restore soft-deleted artifact от ID
//!
//! Risk this file mitigates: future refactor that swaps shared resolver
//! implementation could regress these 5 commands silently — they all
//! share one resolver path, but no test pins their behaviour explicitly.
//!
//! Strategy: same as `cli_resolver_wiring.rs` — call command with both
//! slug form and display id form, assert success + post-action JSON
//! state matches expectations. No string-grep on error messages
//! (HIGH-7 audit lesson).

use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Init workspace + create one PRD ("PRD-001"), return tempdir.
fn workspace_with_one_prd(title: &str) -> TempDir {
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
    dir
}

/// Read canonical slug via `forgeplan get --json`.
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
        .expect("forgeplan get --json must surface slug for Phase 1+ artifact")
        .to_string()
}

/// Get JSON shape of artifact by ref.
fn get_json(workspace: &Path, id_or_slug: &str) -> Value {
    let out = forgeplan()
        .args(["get", id_or_slug, "--json"])
        .current_dir(workspace)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&out).expect("forgeplan get --json must emit valid JSON")
}

// ── phase: read advisory phase state ──────────────────────────────────

#[test]
fn phase_accepts_slug_form() {
    let dir = workspace_with_one_prd("Smoke phase slug");
    let workspace = dir.path();
    let slug = slug_for(workspace, "PRD-001");

    forgeplan()
        .args(["phase", &slug, "--json"])
        .current_dir(workspace)
        .assert()
        .success();
}

#[test]
fn phase_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Smoke phase display");
    let workspace = dir.path();

    forgeplan()
        .args(["phase", "PRD-001", "--json"])
        .current_dir(workspace)
        .assert()
        .success();
}

// ── phase-advance: write advisory phase transition ────────────────────

#[test]
fn phase_advance_accepts_slug_form() {
    let dir = workspace_with_one_prd("Smoke advance slug");
    let workspace = dir.path();
    let slug = slug_for(workspace, "PRD-001");

    // Advance к `validate` phase. New PRD starts в `shape`, so this is
    // a forward transition (advisory only — never blocks).
    forgeplan()
        .args(["phase-advance", &slug, "--to", "validate"])
        .current_dir(workspace)
        .assert()
        .success();

    // Strategy A — verify state mutation landed.
    let after = forgeplan()
        .args(["phase", &slug, "--json"])
        .current_dir(workspace)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&after).unwrap();
    assert_eq!(
        json["current_phase"], "validate",
        "phase-advance via slug must update phase state"
    );
}

#[test]
fn phase_advance_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Smoke advance display");
    let workspace = dir.path();

    forgeplan()
        .args(["phase-advance", "PRD-001", "--to", "validate"])
        .current_dir(workspace)
        .assert()
        .success();

    let after = forgeplan()
        .args(["phase", "PRD-001", "--json"])
        .current_dir(workspace)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&after).unwrap();
    assert_eq!(json["current_phase"], "validate");
}

// ── progress: read progress checkbox state ────────────────────────────

#[test]
fn progress_accepts_slug_form() {
    let dir = workspace_with_one_prd("Smoke progress slug");
    let workspace = dir.path();
    let slug = slug_for(workspace, "PRD-001");

    // `progress <ID>` reads checkbox completion. Empty PRD has 0%
    // progress но command should succeed (zero is valid state).
    forgeplan()
        .args(["progress", &slug, "--json"])
        .current_dir(workspace)
        .assert()
        .success();
}

#[test]
fn progress_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Smoke progress display");
    let workspace = dir.path();

    forgeplan()
        .args(["progress", "PRD-001", "--json"])
        .current_dir(workspace)
        .assert()
        .success();
}

// ── review: lifecycle review report ───────────────────────────────────

#[test]
fn review_accepts_slug_form() {
    let dir = workspace_with_one_prd("Smoke review slug");
    let workspace = dir.path();
    let slug = slug_for(workspace, "PRD-001");

    // `review <ID>` returns lifecycle gate status. Fresh draft passes
    // some gates, fails others — exact status doesn't matter, only
    // что resolver wired and command runs к completion.
    forgeplan()
        .args(["review", &slug])
        .current_dir(workspace)
        .assert()
        .success();
}

#[test]
fn review_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Smoke review display");
    let workspace = dir.path();

    forgeplan()
        .args(["review", "PRD-001"])
        .current_dir(workspace)
        .assert()
        .success();
}

// ── restore: recover soft-deleted artifact ────────────────────────────
//
// Both display id (`PRD-001`) and slug (`prd-foo`) forms accepted.
// PROB-060 Phase 2.5 closure: snapshots stamp the canonical slug at
// soft-delete time, and `undo::find_latest_for_slug` resolves receipts
// by it. See `crates/forgeplan-core/src/undo/mod.rs::find_latest_for_slug`.

#[test]
fn restore_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Smoke restore display");
    let workspace = dir.path();

    forgeplan()
        .args(["delete", "PRD-001", "--yes"])
        .current_dir(workspace)
        .assert()
        .success();

    forgeplan()
        .args(["restore", "PRD-001"])
        .current_dir(workspace)
        .assert()
        .success();

    let restored = get_json(workspace, "PRD-001");
    assert_eq!(restored["id"], "PRD-001");
}

#[test]
fn restore_accepts_slug_form() {
    let dir = workspace_with_one_prd("Smoke restore slug");
    let workspace = dir.path();
    let slug = slug_for(workspace, "PRD-001");

    forgeplan()
        .args(["delete", &slug, "--yes"])
        .current_dir(workspace)
        .assert()
        .success();

    forgeplan()
        .args(["restore", &slug])
        .current_dir(workspace)
        .assert()
        .success();

    // Verify the artifact is back and addressable by both forms.
    let restored = get_json(workspace, &slug);
    assert_eq!(restored["id"], "PRD-001");
}
