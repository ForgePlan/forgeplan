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

/// PROB-095: the remediation must name the holder, not offer to overrule them.
///
/// `--force` is the orchestrator override — it drops the claim regardless of
/// owner. PRD-071 obliges an agent to run `Fix:` as written, so offering force
/// as the primary fix told peer agents to break the coordination they had just
/// collided with. The far likelier cause is that the caller IS the holder and
/// merely omitted `--agent`, because `release` defaults to `cli/<version>`
/// instead of inheriting the identity `claim` was given.
#[test]
fn release_ownership_hint_offers_identity_first_and_force_only_as_alternative() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp);

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "owner"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["release", "PRD-001", "--agent", "stranger"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8_lossy(&out);

    let fix = stderr
        .lines()
        .find(|l| l.starts_with("Fix: "))
        .unwrap_or_else(|| panic!("no `Fix:` line in:\n{stderr}"));
    assert!(
        fix.contains("--agent owner"),
        "the primary fix must release as the holder, got: {fix}"
    );
    assert!(
        !fix.contains("--force"),
        "force must never be the primary fix — an agent runs `Fix:` verbatim, \
         and this one would drop another agent's claim: {fix}"
    );

    let alt = stderr
        .lines()
        .find(|l| l.starts_with("Or: "))
        .unwrap_or_else(|| panic!("no `Or:` line in:\n{stderr}"));
    assert!(
        alt.contains("--force"),
        "force must stay reachable for an orchestrator that means it: {alt}"
    );

    assert!(
        tmp.path().join(".forgeplan/claims/PRD-001.yaml").exists(),
        "a rejected release must not delete the claim"
    );
}

/// The JSON surface carries the same correction — agents parse `_next_action`
/// rather than reading the text lines.
#[test]
fn release_ownership_json_next_action_is_not_force() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp);

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "owner"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["release", "PRD-001", "--agent", "stranger", "--json"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value =
        serde_json::from_slice(&out).expect("error path must still emit valid JSON");

    let next = parsed["_next_action"].as_str().unwrap_or("");
    assert!(
        next.contains("--agent owner") && !next.contains("--force"),
        "_next_action must release as the holder, got: {next}"
    );
    assert!(
        parsed["_alternative_action"]
            .as_str()
            .unwrap_or("")
            .contains("--force"),
        "the override must remain discoverable, just not primary"
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
