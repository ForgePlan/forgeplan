//! PROB-060 / SPEC-005 Phase 2.6 (CD-6) — resolver wiring across CLI verbs.
//!
//! Integration tests verifying that each CLI command listed in CD-6
//! accepts BOTH the slug form (`prd-foo-bar`) and the display id form
//! (`PRD-001`) as the artifact reference argument. The resolver is a
//! single function (`LanceStore::resolve_id`); these tests catch the
//! call-site wiring — i.e. the place in each command where user input
//! flows into `get_record` / lifecycle helpers / projection helpers.
//!
//! Each command has two tests:
//!  - `<cmd>_accepts_slug_form` — input is the slug string from
//!    `forgeplan get --json`'s `slug` field.
//!  - `<cmd>_accepts_display_id_form` — input is the canonical display
//!    id (`PRD-001`).
//!
//! Both must succeed without "not found" errors, OR fail for an
//! unrelated reason (LLM not configured, gate-failure on draft, missing
//! `--by` target). Tests assert the resolver-specific error message
//! ("Artifact ... not found") is *absent*, which is a sharper signal
//! than asserting success on commands that depend on external services
//! (LLM, network).
//!
//! Reference: §2 CD-6 in `docs/sessions/2026-05-07-PROB-060-phase-2-3-4-handoff.md`.

use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Initialise a workspace and create one PRD; return the temp dir.
/// The first PRD is always assigned `PRD-001` post-merge.
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

/// Read the canonical slug for a PRD via `forgeplan get --json`. The slug
/// is the canonical body frontmatter slug (Phase 1+ artifact contract).
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

/// Run a CLI command and assert that it does NOT emit a resolver
/// "not found" error message. The command may still fail for an
/// unrelated reason (e.g. LLM unavailable, lifecycle gate); the
/// resolver-specific assertion is the one we care about for Phase 2.6.
fn assert_no_not_found(cmd: &mut Command, ref_arg: &str) {
    let output = cmd.output().expect("failed to spawn forgeplan");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}\n{stderr}");

    // The exact text from the resolver-aware error path. If this string
    // appears in the combined output, the resolver did NOT recognise
    // the input — that's the regression we're guarding against.
    let needle = format!("Artifact '{ref_arg}' not found");
    assert!(
        !combined.contains(&needle),
        "resolver failed to recognise `{ref_arg}` — full output:\n{combined}"
    );
}

// ────────────────────────────────────────────────────────────────────
// 1. update — takes `--id` arg via positional `id`
// ────────────────────────────────────────────────────────────────────

#[test]
fn update_accepts_slug_form() {
    let dir = workspace_with_one_prd("Update Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    forgeplan()
        .args(["update", &slug, "--title", "Updated Via Slug"])
        .current_dir(dir.path())
        .assert()
        .success();
}

#[test]
fn update_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Update Display Test");

    forgeplan()
        .args(["update", "PRD-001", "--title", "Updated Via Display"])
        .current_dir(dir.path())
        .assert()
        .success();
}

// ────────────────────────────────────────────────────────────────────
// 2. reason — takes `id` arg; LLM call may fail but resolver runs first
// ────────────────────────────────────────────────────────────────────

#[test]
fn reason_accepts_slug_form() {
    let dir = workspace_with_one_prd("Reason Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    let mut cmd = forgeplan();
    cmd.args(["reason", &slug]).current_dir(dir.path());
    assert_no_not_found(&mut cmd, &slug);
}

#[test]
fn reason_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Reason Display Test");

    let mut cmd = forgeplan();
    cmd.args(["reason", "PRD-001"]).current_dir(dir.path());
    assert_no_not_found(&mut cmd, "PRD-001");
}

// ────────────────────────────────────────────────────────────────────
// 3. decompose — takes `prd_id` arg; LLM may fail
// ────────────────────────────────────────────────────────────────────

#[test]
fn decompose_accepts_slug_form() {
    let dir = workspace_with_one_prd("Decompose Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    let mut cmd = forgeplan();
    cmd.args(["decompose", &slug]).current_dir(dir.path());
    assert_no_not_found(&mut cmd, &slug);
}

#[test]
fn decompose_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Decompose Display Test");

    let mut cmd = forgeplan();
    cmd.args(["decompose", "PRD-001"]).current_dir(dir.path());
    assert_no_not_found(&mut cmd, "PRD-001");
}

// ────────────────────────────────────────────────────────────────────
// 4. delete — takes `id`; needs --yes
// ────────────────────────────────────────────────────────────────────

#[test]
fn delete_accepts_slug_form() {
    let dir = workspace_with_one_prd("Delete Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    forgeplan()
        .args(["delete", &slug, "--yes"])
        .current_dir(dir.path())
        .assert()
        .success();
}

#[test]
fn delete_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Delete Display Test");

    forgeplan()
        .args(["delete", "PRD-001", "--yes"])
        .current_dir(dir.path())
        .assert()
        .success();
}

// ────────────────────────────────────────────────────────────────────
// 5. renew — takes `id`; lifecycle gate requires status=stale, so we
//    only assert resolver doesn't reject; gate failure is downstream.
// ────────────────────────────────────────────────────────────────────

#[test]
fn renew_accepts_slug_form() {
    let dir = workspace_with_one_prd("Renew Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    let mut cmd = forgeplan();
    cmd.args(["renew", &slug, "--reason", "test", "--until", "2099-12-31"])
        .current_dir(dir.path());
    assert_no_not_found(&mut cmd, &slug);
}

#[test]
fn renew_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Renew Display Test");

    let mut cmd = forgeplan();
    cmd.args([
        "renew",
        "PRD-001",
        "--reason",
        "test",
        "--until",
        "2099-12-31",
    ])
    .current_dir(dir.path());
    assert_no_not_found(&mut cmd, "PRD-001");
}

// ────────────────────────────────────────────────────────────────────
// 6. reopen — takes `id`; lifecycle gate may fail; resolver runs first
// ────────────────────────────────────────────────────────────────────

#[test]
fn reopen_accepts_slug_form() {
    let dir = workspace_with_one_prd("Reopen Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    let mut cmd = forgeplan();
    cmd.args(["reopen", &slug, "--reason", "test reopen"])
        .current_dir(dir.path());
    assert_no_not_found(&mut cmd, &slug);
}

#[test]
fn reopen_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Reopen Display Test");

    let mut cmd = forgeplan();
    cmd.args(["reopen", "PRD-001", "--reason", "test reopen"])
        .current_dir(dir.path());
    assert_no_not_found(&mut cmd, "PRD-001");
}

// ────────────────────────────────────────────────────────────────────
// 7. supersede — takes `id` + `--by`. We resolve `id`; `--by` falls
//    back to raw input mirroring `link` semantics.
// ────────────────────────────────────────────────────────────────────

#[test]
fn supersede_accepts_slug_form() {
    let dir = workspace_with_one_prd("Supersede Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    // Need a target for --by. Create a second PRD.
    forgeplan()
        .args(["new", "prd", "Supersede Target"])
        .current_dir(dir.path())
        .assert()
        .success();

    let mut cmd = forgeplan();
    cmd.args(["supersede", &slug, "--by", "PRD-002"])
        .current_dir(dir.path());
    assert_no_not_found(&mut cmd, &slug);
}

#[test]
fn supersede_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Supersede Display Test");
    forgeplan()
        .args(["new", "prd", "Supersede Display Target"])
        .current_dir(dir.path())
        .assert()
        .success();

    let mut cmd = forgeplan();
    cmd.args(["supersede", "PRD-001", "--by", "PRD-002"])
        .current_dir(dir.path());
    assert_no_not_found(&mut cmd, "PRD-001");
}

// ────────────────────────────────────────────────────────────────────
// 8. estimate — takes `id`
// ────────────────────────────────────────────────────────────────────

#[test]
fn estimate_accepts_slug_form() {
    let dir = workspace_with_one_prd("Estimate Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    let mut cmd = forgeplan();
    cmd.args(["estimate", &slug]).current_dir(dir.path());
    assert_no_not_found(&mut cmd, &slug);
}

#[test]
fn estimate_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Estimate Display Test");

    let mut cmd = forgeplan();
    cmd.args(["estimate", "PRD-001"]).current_dir(dir.path());
    assert_no_not_found(&mut cmd, "PRD-001");
}

// ────────────────────────────────────────────────────────────────────
// 9. calibrate-estimate — takes `artifact_id` + `--actual-hours`
// ────────────────────────────────────────────────────────────────────

#[test]
fn calibrate_estimate_accepts_slug_form() {
    let dir = workspace_with_one_prd("Calibrate Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    let mut cmd = forgeplan();
    cmd.args(["calibrate-estimate", &slug, "--actual-hours", "8"])
        .current_dir(dir.path());
    assert_no_not_found(&mut cmd, &slug);
}

#[test]
fn calibrate_estimate_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Calibrate Display Test");

    let mut cmd = forgeplan();
    cmd.args(["calibrate-estimate", "PRD-001", "--actual-hours", "8"])
        .current_dir(dir.path());
    assert_no_not_found(&mut cmd, "PRD-001");
}

// ────────────────────────────────────────────────────────────────────
// 10. fgr — takes optional `id`; we always pass it
// ────────────────────────────────────────────────────────────────────

#[test]
fn fgr_accepts_slug_form() {
    let dir = workspace_with_one_prd("FGR Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    let mut cmd = forgeplan();
    cmd.args(["fgr", &slug]).current_dir(dir.path());
    assert_no_not_found(&mut cmd, &slug);
}

#[test]
fn fgr_accepts_display_id_form() {
    let dir = workspace_with_one_prd("FGR Display Test");

    let mut cmd = forgeplan();
    cmd.args(["fgr", "PRD-001"]).current_dir(dir.path());
    assert_no_not_found(&mut cmd, "PRD-001");
}

// ────────────────────────────────────────────────────────────────────
// 11. claim — takes `id`; resolver is best-effort (fallback to raw)
// ────────────────────────────────────────────────────────────────────

#[test]
fn claim_accepts_slug_form() {
    let dir = workspace_with_one_prd("Claim Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    forgeplan()
        .args(["claim", &slug, "--agent", "test-agent"])
        .current_dir(dir.path())
        .assert()
        .success();
}

#[test]
fn claim_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Claim Display Test");

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "test-agent"])
        .current_dir(dir.path())
        .assert()
        .success();
}

// ────────────────────────────────────────────────────────────────────
// 12. release — takes `id`; resolver best-effort. Idempotent (missing
//     claim = success), so we exercise the post-claim release path.
// ────────────────────────────────────────────────────────────────────

#[test]
fn release_accepts_slug_form() {
    let dir = workspace_with_one_prd("Release Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    forgeplan()
        .args(["claim", &slug, "--agent", "test-agent"])
        .current_dir(dir.path())
        .assert()
        .success();

    forgeplan()
        .args(["release", &slug, "--agent", "test-agent"])
        .current_dir(dir.path())
        .assert()
        .success();
}

#[test]
fn release_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Release Display Test");

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "test-agent"])
        .current_dir(dir.path())
        .assert()
        .success();

    forgeplan()
        .args(["release", "PRD-001", "--agent", "test-agent"])
        .current_dir(dir.path())
        .assert()
        .success();
}

// ────────────────────────────────────────────────────────────────────
// 13. import — bulk-resolve in payload loop. Test verifies that an
//     export-then-reimport with --force round-trips a slug-typed id
//     correctly via the per-item resolver.
// ────────────────────────────────────────────────────────────────────

#[test]
fn import_accepts_slug_in_payload() {
    let dir = workspace_with_one_prd("Import Slug Payload");
    let slug = slug_for(dir.path(), "PRD-001");

    // Hand-craft a minimal export-shaped JSON file that uses the slug
    // (not display id) as the artifact's primary id. Resolver in
    // import_cmd should normalise it onto the canonical PRD-001 row.
    let payload = serde_json::json!({
        "artifacts": [{
            "id": &slug,
            "kind": "prd",
            "status": "draft",
            "title": "Re-imported via slug",
            "body": "## Goal\n\nRe-import smoke test.\n",
            "depth": "standard",
            "tags": [],
        }],
        "relations": []
    });
    let payload_path = dir.path().join("payload.json");
    std::fs::write(&payload_path, payload.to_string()).unwrap();

    forgeplan()
        .args(["import", payload_path.to_str().unwrap(), "--force"])
        .current_dir(dir.path())
        .assert()
        .success();
}

#[test]
fn import_accepts_display_id_in_payload() {
    let dir = workspace_with_one_prd("Import Display Payload");

    let payload = serde_json::json!({
        "artifacts": [{
            "id": "PRD-001",
            "kind": "prd",
            "status": "draft",
            "title": "Re-imported via display id",
            "body": "## Goal\n\nRe-import smoke test.\n",
            "depth": "standard",
            "tags": [],
        }],
        "relations": []
    });
    let payload_path = dir.path().join("payload.json");
    std::fs::write(&payload_path, payload.to_string()).unwrap();

    forgeplan()
        .args(["import", payload_path.to_str().unwrap(), "--force"])
        .current_dir(dir.path())
        .assert()
        .success();
}
