//! Integration tests for `forgeplan activity` (PRD-070, FR-001).
//!
//! Spins up a tempdir workspace, seeds the activity log via real CLI
//! invocations (each command auto-writes through the MCP dispatch
//! wrapper — but here we exercise the CLI path which only reads the
//! log; for the producer side we write fixture lines directly so the
//! test is hermetic to producer-side wiring).

use assert_cmd::Command;
use chrono::Utc;
use predicates::prelude::*;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Initialize a workspace in `tmp` and return the workspace dir path
/// (`<tmp>/.forgeplan/`).
fn init_workspace(tmp: &TempDir) -> PathBuf {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    tmp.path().join(".forgeplan")
}

/// Append a fixture activity entry to the daily log file.
fn seed_entry(ws: &Path, tool: &str, status: &str, duration_ms: u64) {
    let logs_dir = ws.join("logs");
    std::fs::create_dir_all(&logs_dir).unwrap();
    let date = Utc::now().format("%Y-%m-%d");
    let path = logs_dir.join(format!("tools-{date}.jsonl"));

    let entry = serde_json::json!({
        "ts": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "tool": tool,
        "args_hash": "abc123def456",
        "duration_ms": duration_ms,
        "status": status,
        "workspace": ws.display().to_string(),
    });
    let line = format!("{}\n", serde_json::to_string(&entry).unwrap());
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .unwrap();
    f.write_all(line.as_bytes()).unwrap();
}

#[test]
fn activity_empty_workspace_succeeds() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    forgeplan()
        .args(["activity"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No tool calls"));
}

#[test]
fn activity_lists_seeded_entries() {
    let tmp = TempDir::new().unwrap();
    let ws = init_workspace(&tmp);
    seed_entry(&ws, "forgeplan_health", "ok", 12);
    seed_entry(&ws, "forgeplan_score", "ok", 45);

    forgeplan()
        .args(["activity"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("forgeplan_health"))
        .stdout(predicate::str::contains("forgeplan_score"));
}

#[test]
fn activity_json_parses_to_object() {
    let tmp = TempDir::new().unwrap();
    let ws = init_workspace(&tmp);
    seed_entry(&ws, "forgeplan_health", "ok", 7);

    let out = forgeplan()
        .args(["activity", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
    assert_eq!(v["since_hours"], 24);
    assert_eq!(v["returned"], 1);
    assert_eq!(v["entries"][0]["tool"], "forgeplan_health");
}

#[test]
fn activity_tool_filter_narrows_results() {
    let tmp = TempDir::new().unwrap();
    let ws = init_workspace(&tmp);
    seed_entry(&ws, "forgeplan_health", "ok", 1);
    seed_entry(&ws, "forgeplan_score", "ok", 1);

    let out = forgeplan()
        .args(["activity", "--tool", "forgeplan_score", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["returned"], 1);
    assert_eq!(v["entries"][0]["tool"], "forgeplan_score");
}

#[test]
fn activity_status_filter_selects_errors() {
    let tmp = TempDir::new().unwrap();
    let ws = init_workspace(&tmp);
    seed_entry(&ws, "forgeplan_health", "ok", 1);
    seed_entry(&ws, "forgeplan_reason", "tool_err", 5000);

    let out = forgeplan()
        .args(["activity", "--status", "tool_err", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["returned"], 1);
    assert_eq!(v["entries"][0]["status"], "tool_err");
}
