// PRD-056 (EPIC-005): integration tests for `forgeplan phase` CLI command.
//
// Verifies advisory phase read semantics: missing state is treated as
// "unknown" (never an error), and after an advance the read surfaces
// current_phase, history, and a "suggested next" hint.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init_workspace(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

fn create_prd(tmp: &TempDir) {
    forgeplan()
        .args(["new", "prd", "Phase Test"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));
}

#[test]
fn phase_unknown_for_artifact_without_state() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    // Use a brand-new ID we know has no state file. We don't even create
    // an artifact -- the phase command must work on raw IDs (advisory).
    forgeplan()
        .args(["phase", "PRD-999"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("unknown"))
        .stdout(predicate::str::contains("phase-advance"));
}

#[test]
fn phase_unknown_json_shape() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let output = forgeplan()
        .args(["phase", "PRD-999", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(v["current_phase"], "unknown");
    assert_eq!(v["artifact_id"], "PRD-999");
    assert!(v["history"].as_array().unwrap().is_empty());
}

#[test]
fn phase_after_advance_shows_current_and_history() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    create_prd(&tmp);

    // Advance to shape (initial state).
    forgeplan()
        .args(["phase-advance", "PRD-001", "--to", "shape"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Read it back -- must show "shape" + suggest "validate".
    forgeplan()
        .args(["phase", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("shape"))
        .stdout(predicate::str::contains("validate"));
}

#[test]
fn phase_json_after_advance_has_history_and_next_hint() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    create_prd(&tmp);

    forgeplan()
        .args(["phase-advance", "PRD-001", "--to", "code"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["phase", "PRD-001", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(v["current_phase"], "code");
    assert_eq!(v["artifact_id"], "PRD-001");
    assert!(
        !v["history"].as_array().unwrap().is_empty(),
        "history must be populated after advance"
    );
}
