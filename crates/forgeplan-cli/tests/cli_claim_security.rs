//! PROB-066 regression guard.
//!
//! Before the fix, `forgeplan claim --agent <STR>` accepted any non-empty
//! string as the agent identifier — including `/`, newlines, ANSI escapes,
//! and bidi-override codepoints — while the MCP path via
//! `AgentIdentity::new` rejected them. The asymmetry let an operator (or a
//! pasted payload) corrupt the on-disk YAML (newline injection) or spoof
//! the terminal when `forgeplan claims` echoed the agent string back.
//!
//! This test pins the regression by exercising the real CLI binary across
//! the rejection classes named in `validate_agent_id`:
//! - `/` (the smoke-test enshrined form `smoke-test/v1`)
//! - control char (`\n`)
//! - bidi override (`\u{202E}`)
//! - overlong (>64 bytes)

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn workspace_with_prd() -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(dir.path())
        .assert()
        .success();
    let out = forgeplan()
        .args(["new", "prd", "Security regression"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(out).expect("stdout utf8");
    // `forgeplan new prd` emits the canonical id as the first token of the
    // `Created` line — e.g. "Created PRD-001 ...".
    let id = stdout
        .lines()
        .find_map(|l| l.split_whitespace().find(|w| w.starts_with("PRD-")))
        .expect("expected PRD id in `new prd` output")
        .to_string();
    (dir, id)
}

#[test]
fn cli_claim_rejects_slash_agent() {
    // PROB-066 core case — the smoke-test enshrined form `smoke-test/v1`.
    // CLI surface must refuse before the workspace lock is acquired,
    // emitting a `Fix:` hint with a concrete remediation example.
    let (dir, id) = workspace_with_prd();
    let assertion = forgeplan()
        .args([
            "claim",
            &id,
            "--agent",
            "smoke-test/v1",
            "--ttl-minutes",
            "5",
        ])
        .current_dir(dir.path())
        .assert()
        .failure();
    let stderr = String::from_utf8(assertion.get_output().stderr.clone()).expect("stderr utf8");
    assert!(
        stderr.contains("Fix:") && stderr.contains("smoke-test-v1"),
        "expected Fix: hint with remediation example, got: {stderr}"
    );
}

#[test]
fn cli_claim_rejects_newline_agent() {
    // YAML injection vector: a `\n` in agent_id would corrupt the
    // .forgeplan/claims/<ID>.yaml body. CLI must refuse the input.
    let (dir, id) = workspace_with_prd();
    forgeplan()
        .args(["claim", &id, "--agent", "evil\nx: y", "--ttl-minutes", "5"])
        .current_dir(dir.path())
        .assert()
        .failure();
    // No claim file may have been written.
    let claim_path = dir.path().join(format!(".forgeplan/claims/{id}.yaml"));
    assert!(
        !claim_path.exists(),
        "claim file written despite invalid agent string"
    );
}

#[test]
fn cli_claim_rejects_bidi_override_agent() {
    // Terminal-spoof vector: `forgeplan claims` echoes agent_id verbatim.
    // A bidi-override (RLO) in the string flips downstream output.
    let (dir, id) = workspace_with_prd();
    let bad = "orch\u{202E}drawkcab";
    forgeplan()
        .args(["claim", &id, "--agent", bad, "--ttl-minutes", "5"])
        .current_dir(dir.path())
        .assert()
        .failure();
    let claim_path = dir.path().join(format!(".forgeplan/claims/{id}.yaml"));
    assert!(!claim_path.exists());
}

#[test]
fn cli_claim_rejects_overlong_agent() {
    // 65-byte agent string exceeds MAX_AGENT_LEN (64).
    let (dir, id) = workspace_with_prd();
    let too_long = "x".repeat(65);
    forgeplan()
        .args(["claim", &id, "--agent", &too_long, "--ttl-minutes", "5"])
        .current_dir(dir.path())
        .assert()
        .failure();
    let claim_path = dir.path().join(format!(".forgeplan/claims/{id}.yaml"));
    assert!(!claim_path.exists());
}

#[test]
fn cli_claim_accepts_hyphenated_agent() {
    // Round-trip safety: the operator-friendly form `smoke-test-v1`
    // (recommended in the CLI Fix hint) must continue to work end-to-end.
    let (dir, id) = workspace_with_prd();
    forgeplan()
        .args([
            "claim",
            &id,
            "--agent",
            "smoke-test-v1",
            "--ttl-minutes",
            "5",
        ])
        .current_dir(dir.path())
        .assert()
        .success();
    let claim_path = dir.path().join(format!(".forgeplan/claims/{id}.yaml"));
    assert!(claim_path.exists(), "expected claim file at {claim_path:?}");
    // And cleanup with release using the same agent.
    forgeplan()
        .args(["release", &id, "--agent", "smoke-test-v1"])
        .current_dir(dir.path())
        .assert()
        .success();
}
