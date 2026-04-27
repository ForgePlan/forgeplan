//! Integration tests for `forgeplan claims` (PRD-070 CLI parity).

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

#[test]
fn claims_empty_workspace_reports_no_claims() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    forgeplan()
        .arg("claims")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No active claims"));
}

#[test]
fn claims_lists_two_active_entries() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    forgeplan()
        .args(["new", "prd", "First"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "rfc", "Second"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "alpha"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["claim", "RFC-001", "--agent", "beta"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .arg("claims")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("RFC-001"))
        .stdout(predicate::str::contains("alpha"))
        .stdout(predicate::str::contains("beta"))
        .stdout(predicate::str::contains("2 active claim"));
}

#[test]
fn claims_json_output_parses() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    forgeplan()
        .args(["new", "prd", "JSON Claim"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["claim", "PRD-001", "--agent", "json-agent"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["claims", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "claims --json failed: {output:?}");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("expected valid JSON, got {stdout:?}: {e}"));
    assert_eq!(parsed["count"], 1);
    assert_eq!(parsed["skipped"], 0);
    let claims = parsed["claims"].as_array().expect("claims is array");
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0]["id"], "PRD-001");
    assert_eq!(claims[0]["agent_id"], "json-agent");
}
