//! Integration tests for `forgeplan release` (PRD-070 CLI parity).

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init_with_prd(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Releaseable"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

#[test]
fn release_by_owner_succeeds() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp);

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "owner"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["release", "PRD-001", "--agent", "owner"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Released"));

    assert!(
        !tmp.path().join(".forgeplan/claims/PRD-001.yaml").exists(),
        "claim file should be gone after release"
    );
}

#[test]
fn release_by_wrong_agent_fails_without_force() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp);

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "owner"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["release", "PRD-001", "--agent", "stranger"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("owner"))
        .stderr(predicate::str::contains("--force"));

    // Claim must still exist.
    assert!(
        tmp.path().join(".forgeplan/claims/PRD-001.yaml").exists(),
        "rejected release must not delete the claim"
    );
}

#[test]
fn release_force_overrides_agent_check() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp);

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "owner"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["release", "PRD-001", "--agent", "orchestrator", "--force"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("forced"));

    assert!(
        !tmp.path().join(".forgeplan/claims/PRD-001.yaml").exists(),
        "force-release should remove the claim"
    );
}

#[test]
fn release_missing_claim_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp);

    // No claim ever made — release must still succeed (idempotent).
    forgeplan()
        .args(["release", "PRD-001", "--agent", "anyone"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Released"));
}

#[test]
fn release_json_output_parses() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp);

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "owner"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["release", "PRD-001", "--agent", "owner", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "release --json failed: {output:?}");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("expected valid JSON, got {stdout:?}: {e}"));
    assert_eq!(parsed["id"], "PRD-001");
    assert_eq!(parsed["released"], true);
    assert_eq!(parsed["force"], false);
}
