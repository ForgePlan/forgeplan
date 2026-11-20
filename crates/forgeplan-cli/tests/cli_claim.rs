//! Integration tests for `forgeplan claim` (PRD-070 CLI parity).

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init_with_prd(tmp: &TempDir, title: &str) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", title])
        .current_dir(tmp.path())
        .assert()
        .success();
}

#[test]
fn claim_writes_file_under_claims_dir() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp, "Claim Subject");

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "agent-A"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Claimed PRD-001"));

    // The sidecar is addressed by the document's canonical key, not by the
    // display alias two documents may share (ADR-030).
    let claims_dir = tmp.path().join(".forgeplan/claims");
    let entries: Vec<_> = std::fs::read_dir(&claims_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(entries.len(), 1, "one claim, one sidecar: {entries:?}");
    let claim_path = entries[0].clone();
    assert!(
        claim_path.to_str().unwrap().ends_with(".yaml"),
        "the sidecar is a yaml file named after the key: {claim_path:?}"
    );
    assert!(
        !claim_path.ends_with("PRD-001.yaml"),
        "the sidecar is not named by the alias: {claim_path:?}"
    );
    let body = std::fs::read_to_string(&claim_path).unwrap();
    assert!(
        body.contains("canonical_key"),
        "the sidecar declares the document it is about: {body}"
    );
    assert!(
        body.contains("agent-A"),
        "claim file should mention agent-A: {body}"
    );
}

#[test]
fn claim_rejects_when_already_held_by_other_agent() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp, "Contention Subject");

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "agent-A"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Second agent attempts the same claim — must fail with exit 1 and a
    // hint about --force on release.
    let assert_b = forgeplan()
        .args(["claim", "PRD-001", "--agent", "agent-B"])
        .current_dir(tmp.path())
        .assert()
        .failure();
    assert_b
        .stderr(predicate::str::contains("agent-A"))
        .stderr(predicate::str::contains("--force"));
}

#[test]
fn claim_renew_by_same_agent_succeeds() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp, "Renewable");

    forgeplan()
        .args([
            "claim",
            "PRD-001",
            "--agent",
            "agent-A",
            "--ttl-minutes",
            "5",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args([
            "claim",
            "PRD-001",
            "--agent",
            "agent-A",
            "--ttl-minutes",
            "10",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();
}

#[test]
fn claim_json_output_parses() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp, "JSON Subject");

    let output = forgeplan()
        .args(["claim", "PRD-001", "--agent", "json-agent", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "claim --json failed: {output:?}");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("expected valid JSON, got {stdout:?}: {e}"));
    assert_eq!(parsed["id"], "PRD-001");
    assert_eq!(parsed["agent_id"], "json-agent");
    assert!(parsed["expires_at"].is_string());
}

#[test]
fn claim_rejects_zero_ttl() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp, "Zero TTL");

    forgeplan()
        .args([
            "claim",
            "PRD-001",
            "--agent",
            "agent-A",
            "--ttl-minutes",
            "0",
        ])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("ttl-minutes"));
}
