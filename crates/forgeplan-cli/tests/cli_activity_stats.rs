//! Integration tests for `forgeplan activity-stats` (PRD-070, FR-002).

use assert_cmd::Command;
use chrono::Utc;
use predicates::prelude::*;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init_workspace(tmp: &TempDir) -> PathBuf {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    tmp.path().join(".forgeplan")
}

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
fn activity_stats_empty_workspace_succeeds() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    forgeplan()
        .args(["activity-stats"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No activity"));
}

#[test]
fn activity_stats_groups_by_tool() {
    let tmp = TempDir::new().unwrap();
    let ws = init_workspace(&tmp);
    for _ in 0..3 {
        seed_entry(&ws, "forgeplan_health", "ok", 10);
    }
    for _ in 0..2 {
        seed_entry(&ws, "forgeplan_score", "ok", 50);
    }

    forgeplan()
        .args(["activity-stats"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("forgeplan_health"))
        .stdout(predicate::str::contains("forgeplan_score"));
}

#[test]
fn activity_stats_json_has_aggregates() {
    let tmp = TempDir::new().unwrap();
    let ws = init_workspace(&tmp);
    seed_entry(&ws, "forgeplan_health", "ok", 10);
    seed_entry(&ws, "forgeplan_health", "ok", 30);
    seed_entry(&ws, "forgeplan_health", "tool_err", 100);

    let out = forgeplan()
        .args(["activity-stats", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
    assert_eq!(v["total_calls"], 3);
    assert_eq!(v["total_errors"], 1);
    assert_eq!(v["since_hours"], 24);
    let stats = v["stats"].as_array().unwrap();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0]["tool"], "forgeplan_health");
    assert_eq!(stats[0]["count"], 3);
    assert_eq!(stats[0]["err_count"], 1);
}
