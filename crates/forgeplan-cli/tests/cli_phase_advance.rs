// PRD-056 (EPIC-005): integration tests for `forgeplan phase-advance`
// CLI command.
//
// Verifies advisory phase advance semantics:
//   * advance writes / updates `.forgeplan/state/<id>.yaml`
//   * out-of-order jumps are allowed (advisory layer per FR)
//   * subsequent reads surface the new current_phase + cumulative history
//   * --reason is recorded
//   * --json emits a parseable payload

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
        .args(["new", "prd", "Phase Advance Test"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));
}

#[test]
fn advance_to_shape_then_code_records_history() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    create_prd(&tmp);

    // First advance: shape.
    forgeplan()
        .args(["phase-advance", "PRD-001", "--to", "shape"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("shape"));

    // Second advance: code (jump over validate/adi -- advisory allows it).
    forgeplan()
        .args([
            "phase-advance",
            "PRD-001",
            "--to",
            "code",
            "--reason",
            "skipping forward",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("code"));

    // Read it back; cumulative history must include both transitions.
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
    let history = v["history"].as_array().expect("history is array");
    assert!(
        history.len() >= 2,
        "expected at least 2 history entries, got {}",
        history.len()
    );
    let last = &history[history.len() - 1];
    assert_eq!(last["to"], "code");
    assert_eq!(last["reason"], "skipping forward");
}

#[test]
fn advance_json_emits_suggested_next() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    create_prd(&tmp);

    let output = forgeplan()
        .args(["phase-advance", "PRD-001", "--to", "validate", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(v["current_phase"], "validate");
    assert_eq!(v["suggested_next"], "adi");
    assert_eq!(v["artifact_id"], "PRD-001");
}

#[test]
fn advance_to_done_is_terminal() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    create_prd(&tmp);

    forgeplan()
        .args(["phase-advance", "PRD-001", "--to", "done"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("done"))
        .stdout(predicate::str::contains("terminal"));
}

#[test]
fn advance_rejects_invalid_phase_value() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    create_prd(&tmp);

    // clap value_enum should reject an unknown variant before our code runs.
    forgeplan()
        .args(["phase-advance", "PRD-001", "--to", "bogus"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

#[test]
fn advance_without_workspace_errors_clearly() {
    let tmp = TempDir::new().unwrap();
    // Deliberately do not init -- there is no .forgeplan/.
    forgeplan()
        .args(["phase-advance", "PRD-001", "--to", "shape"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("forgeplan init").or(predicate::str::contains(".forgeplan")),
        );
}
