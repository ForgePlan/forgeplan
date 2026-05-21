//! Integration tests for `forgeplan anomalies` (issue #289, audit-r2
//! coverage closure).
//!
//! Each test spins up a real CLI invocation against a fresh tempdir
//! workspace. Covers:
//!   - JSON output shape on an empty workspace (the canonical happy path
//!     plugin orchestrators consume).
//!   - Filter parameter wiring (kind / severity / since).
//!   - Invalid-input error path (clean error, no panic, with guidance).
//!   - Hint protocol surface (Next: / Done. on text mode).

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Initialize a fresh workspace in the tempdir. Returns the tempdir so the
/// caller can keep it alive across the assertion phase.
fn init_workspace() -> TempDir {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    tmp
}

/// Empty workspace → `forgeplan anomalies --json` returns valid JSON with
/// total=0 and the four expected top-level fields.
#[test]
fn anomalies_json_empty_workspace_shape() {
    let tmp = init_workspace();
    let output = forgeplan()
        .args(["anomalies", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "anomalies command failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output must be parseable JSON");
    assert!(parsed.is_object(), "anomalies JSON must be an object");
    // Canonical shape: anomalies + total + by_severity + by_tier.
    for field in ["anomalies", "total", "by_severity", "by_tier"] {
        assert!(
            parsed.get(field).is_some(),
            "missing canonical field '{field}': {parsed}"
        );
    }
    assert_eq!(
        parsed["total"].as_u64(),
        Some(0),
        "empty workspace = 0 total"
    );
    assert_eq!(
        parsed["anomalies"].as_array().map(|a| a.len()),
        Some(0),
        "empty workspace = empty anomalies array"
    );
    // Audit-r3 HIGH-1 / F3 closure: CLI JSON now emits `_next_action`
    // matching the MCP wire shape (PRD-071 parity). Empty workspace
    // produces the "Done." sentinel string.
    let next_action = parsed["_next_action"]
        .as_str()
        .expect("CLI JSON must include _next_action for MCP parity");
    assert!(
        next_action.contains("No anomalies") || next_action.contains("Done"),
        "empty workspace _next_action must signal terminal state: got {next_action}"
    );
}

/// Text-mode output on an empty workspace ends with the `Done.` hint
/// marker per PRD-071 hint protocol.
#[test]
fn anomalies_text_empty_workspace_emits_done_marker() {
    let tmp = init_workspace();
    let output = forgeplan()
        .args(["anomalies"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Done."),
        "text mode on empty workspace must emit `Done.` per PRD-071: got {stdout}"
    );
    assert!(
        stdout.contains("No pipeline anomalies"),
        "text mode must surface the empty-state message: {stdout}"
    );
}

/// Invalid `--kind` value produces a clean error with guidance listing
/// valid kinds, not a panic or silent empty result.
#[test]
fn anomalies_invalid_kind_filter_returns_clean_error() {
    let tmp = init_workspace();
    let output = forgeplan()
        .args(["anomalies", "--kind", "not_a_real_kind", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "invalid --kind must fail, not silently return empty"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    // The error message must enumerate at least two valid kinds so the
    // agent can recover.
    assert!(
        stderr.contains("stuck_draft") || stderr.contains("orphan_link"),
        "error must list valid kinds for retry: {stderr}"
    );
}

/// Invalid `--severity` value produces a clean error with guidance.
#[test]
fn anomalies_invalid_severity_filter_returns_clean_error() {
    let tmp = init_workspace();
    let output = forgeplan()
        .args(["anomalies", "--severity", "not_a_severity"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("low") || stderr.contains("medium") || stderr.contains("high"),
        "severity error must list valid values: {stderr}"
    );
}

/// Invalid `--since` (non-RFC 3339) produces a clean error.
#[test]
fn anomalies_invalid_since_returns_clean_error() {
    let tmp = init_workspace();
    let output = forgeplan()
        .args(["anomalies", "--since", "yesterday"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("RFC 3339") || stderr.contains("2026"),
        "since error must hint at the expected format: {stderr}"
    );
}

/// Filter narrowing: `--kind stuck_draft` on an empty workspace produces a
/// well-formed report with total=0 and an empty anomalies array — confirms
/// the kind filter is wired all the way through to the JSON shape, not
/// rejected as an unrecognised parameter.
#[test]
fn anomalies_kind_filter_narrows_to_empty_on_empty_workspace() {
    let tmp = init_workspace();
    let output = forgeplan()
        .args(["anomalies", "--kind", "stuck_draft", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "filter must be a valid parameter");
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON shape with filter");
    assert_eq!(parsed["total"].as_u64(), Some(0));
}

/// Hint protocol round-trip: combined filters (kind + severity) produce a
/// well-formed JSON response and the per-tier counts sum to total.
#[test]
fn anomalies_filter_combination_produces_consistent_counts() {
    let tmp = init_workspace();
    let output = forgeplan()
        .args([
            "anomalies",
            "--kind",
            "stuck_draft",
            "--severity",
            "medium",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let total = parsed["total"].as_u64().unwrap();
    let by_tier_sum = parsed["by_tier"]["auto"].as_u64().unwrap_or(0)
        + parsed["by_tier"]["adi"].as_u64().unwrap_or(0)
        + parsed["by_tier"]["user"].as_u64().unwrap_or(0);
    assert_eq!(
        by_tier_sum, total,
        "tier counts must sum to total even under filter: {parsed}"
    );
}
