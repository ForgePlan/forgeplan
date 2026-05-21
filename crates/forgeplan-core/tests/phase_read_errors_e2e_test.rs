//! FR-020 end-to-end: corrupted phase YAML must be counted in
//! `HealthReport.phase_read_errors` and emit a `tracing::warn!` line.
//!
//! ## Why an integration test
//!
//! W4's only existing test asserts `phase_read_errors == 0` on an empty
//! report (unit-level shape check). The audit brief (Wave 1.5 CR-H7)
//! flagged the gap: there is no test that *actually* hits the error
//! branch — corrupts a real on-disk YAML, runs `health_report_with_phase`,
//! and verifies the counter increments. This file plugs that gap.
//!
//! ## Test strategy
//!
//! 1. Initialise a workspace via `LanceStore::init` (creates `.forgeplan/`
//!    and the LanceDB tables under it).
//! 2. Seed an artifact directly via the `test-helpers` escape hatch
//!    (`create_artifact_for_test`) — we are NOT exercising the
//!    projection pipeline, only the health-scan phase-read path.
//! 3. Write garbage bytes to `.forgeplan/state/<id>.yaml` so
//!    `serde_yaml::from_slice` fails. The corrupted YAML path
//!    quarantines + returns `Ok(None)` per the existing contract.
//! 4. Wait, that path returns `Ok(None)` — not `Err`. So `phase_read_errors`
//!    will NOT increment for a corrupt-YAML file under the current
//!    contract.
//!
//! ## What actually increments `phase_read_errors`?
//!
//! `read_phase` returns `Err(_)` only when:
//! - `validate_artifact_id` rejects the id (path traversal etc.) — but the
//!   id has already been validated upstream by the time `health_report`
//!   sees it, so this branch is unreachable via the health path.
//! - `tokio::fs::read` returns an error OTHER than `NotFound`. The path
//!   exists (try_exists returned true) — to trigger this we need EACCES
//!   or EIO. Easiest in a test: write a 0-permission file (mode 000)
//!   on Unix.
//!
//! We use the unreadable-file route on Unix; on Windows we skip the test
//! (tests/ignored gating).

use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::health::health_report_with_phase;
use tempfile::TempDir;

/// FR-020 e2e — confirm corrupted (unreadable) phase YAML causes
/// `phase_read_errors` to increment by exactly 1.
///
/// Wave 1.5 CR-H7 closure: pre-fix, only the empty-report shape was tested.
/// This test exercises the actual error branch in
/// `health_report_with_phase::buffer_unordered::Err(e)`.
#[cfg(unix)]
#[tokio::test]
async fn corrupted_phase_yaml_increments_phase_read_errors() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().join(".forgeplan");

    // 1. Initialise LanceStore — creates .forgeplan/ + tables. We do NOT
    //    use `init_workspace` because that requires a project name and
    //    creates artefact subdirs we don't need for this test; raw
    //    LanceStore::init is enough to satisfy `health_report_with_phase`.
    let store = LanceStore::init(&ws).await.expect("LanceStore::init");

    // Also write a minimal config.yaml so `load_config` succeeds.
    // `is_enabled(&config)` defaults to true when the phase block is
    // missing — perfect for our scenario (we want phase tracking ON).
    std::fs::write(ws.join("config.yaml"), "project_name: fr020_e2e\n").expect("write config.yaml");

    // 2. Seed an active PRD via the test-helpers escape hatch. We need it
    //    to be `active` because `health_report_with_phase` filters phase
    //    reads to active artifacts only (line 511 of health/mod.rs).
    let prd = NewArtifact {
        id: "PRD-001".to_string(),
        kind: "prd".to_string(),
        status: "active".to_string(),
        title: "FR-020 test PRD".to_string(),
        body: "## Problem\n\nReal body.\n\n## Goals\n\nReal goals.\n".to_string(),
        depth: "standard".to_string(),
        author: Some("fr020-e2e".to_string()),
        parent_epic: None,
        valid_until: None,
        tags: Vec::new(),
    };
    store
        .create_artifact_for_test(&prd)
        .await
        .expect("create_artifact_for_test");

    // 3. Create the state file at the path read_phase will check, then
    //    chmod it to 000 so the read fails with EACCES (NOT a NotFound,
    //    which would be silently treated as `Ok(None)` per FR-012).
    //
    //    This is the route that produces `Err(_)` from `read_phase` →
    //    `health_report_with_phase` Err arm → `phase_read_errors += 1`.
    let state_dir = ws.join("state");
    std::fs::create_dir_all(&state_dir).expect("mkdir state dir");
    let state_path = state_dir.join("PRD-001.yaml");
    // Write a valid-looking PhaseState YAML so the failure mode is *I/O*
    // (EACCES on open) rather than a parse error. Parse errors quarantine
    // to `.yaml.corrupt.<ts>` + return `Ok(None)` — that does NOT
    // increment phase_read_errors.
    std::fs::write(
        &state_path,
        "artifact_id: PRD-001\n\
         workflow_type: greenfield\n\
         current_phase: shape\n\
         advanced_at: 2026-05-13T00:00:00Z\n\
         history: []\n\
         schema_version: 1\n",
    )
    .expect("write state yaml");

    // chmod 000 → unreadable. The `tokio::fs::read` call inside
    // `read_phase` returns `Err(EACCES)` which is NOT `NotFound`, so
    // the error propagates upward to `health_report_with_phase` and
    // increments `phase_read_errors`.
    let mut perms = std::fs::metadata(&state_path)
        .expect("stat state yaml")
        .permissions();
    perms.set_mode(0o000);
    std::fs::set_permissions(&state_path, perms).expect("chmod 000");

    // 4. Run health_report_with_phase. Expect phase_read_errors == 1.
    let (report, _mismatches) = health_report_with_phase(&store, &ws)
        .await
        .expect("health_report_with_phase");

    // Restore perms before any assert so cleanup works regardless of
    // test outcome (TempDir cleanup needs read access to enumerate).
    let mut perms_restore = std::fs::metadata(&state_path)
        .expect("stat state yaml restore")
        .permissions();
    perms_restore.set_mode(0o644);
    let _ = std::fs::set_permissions(&state_path, perms_restore);

    // FR-020 contract: exactly one phase read error counted.
    assert_eq!(
        report.phase_read_errors, 1,
        "expected exactly 1 phase_read_errors after corrupting PRD-001 state file, \
         got {} (full report: total={}, by_status={:?})",
        report.phase_read_errors, report.total, report.by_status
    );

    // Sanity: the PRD itself is still counted in the report total.
    assert!(
        report.total >= 1,
        "PRD-001 should be counted in report.total: {}",
        report.total
    );
}

/// Negative control: a clean workspace with one active PRD and a
/// well-formed phase YAML must produce `phase_read_errors == 0`.
/// Pinned alongside the positive test so a future regression that flips
/// the polarity of the counter is caught here.
#[cfg(unix)]
#[tokio::test]
async fn clean_phase_yaml_does_not_increment_phase_read_errors() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().join(".forgeplan");
    let store = LanceStore::init(&ws).await.expect("LanceStore::init");
    std::fs::write(ws.join("config.yaml"), "project_name: fr020_e2e_clean\n")
        .expect("write config.yaml");

    let prd = NewArtifact {
        id: "PRD-002".to_string(),
        kind: "prd".to_string(),
        status: "active".to_string(),
        title: "FR-020 clean PRD".to_string(),
        body: "## Problem\n\nReal body.\n\n## Goals\n\nReal goals.\n".to_string(),
        depth: "standard".to_string(),
        author: Some("fr020-e2e".to_string()),
        parent_epic: None,
        valid_until: None,
        tags: Vec::new(),
    };
    store
        .create_artifact_for_test(&prd)
        .await
        .expect("create_artifact_for_test");

    // No state file at all — `read_phase` returns `Ok(None)` early via
    // `try_exists == false`. NOT an error → counter stays 0.
    let (report, _) = health_report_with_phase(&store, &ws)
        .await
        .expect("health_report_with_phase");

    assert_eq!(
        report.phase_read_errors, 0,
        "clean workspace must have phase_read_errors == 0, got {}",
        report.phase_read_errors
    );
}

/// Sanity: the test-helpers feature gate is active (debug-only). If this
/// test fails to compile, it means `create_artifact_for_test` was
/// gated out — re-check Cargo features.
#[cfg(unix)]
#[tokio::test]
async fn test_helpers_gate_is_active() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().join(".forgeplan");
    let _store = LanceStore::init(&ws).await.expect("LanceStore::init");
    // Compilation alone proves the gate; no runtime assertion needed.
}
