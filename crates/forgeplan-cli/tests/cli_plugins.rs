//! Integration tests for `forgeplan plugins {list,doctor,info}` (PRD-067).
//!
//! These tests run inside a [`TempDir`] with `HOME` overridden to a
//! known-empty path so the filesystem scanner deterministically reports
//! "no installed plugins". This keeps the test stable on CI where the
//! agent's `~/.claude/plugins/cache` may or may not exist.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn forgeplan_with_clean_home(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("forgeplan").unwrap();
    // Override HOME so the filesystem scanner sees no installed plugins.
    cmd.env("HOME", home.path());
    cmd
}

#[test]
fn plugins_list_json_returns_array() {
    let home = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();
    let output = forgeplan_with_clean_home(&home)
        .args(["plugins", "list", "--json"])
        .current_dir(cwd.path())
        .output()
        .expect("run plugins list");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(v["installed"].is_array(), "expected installed array");
    // Forgeplan synthetic entry is always present.
    let has_forgeplan = v["installed"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["info"]["name"] == "forgeplan");
    assert!(has_forgeplan, "expected forgeplan synthetic entry in: {v}");
    assert!(v["_next_action"].is_string());
}

#[test]
fn plugins_doctor_clean_home_reports_missing_and_exits_one() {
    let home = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();
    let output = forgeplan_with_clean_home(&home)
        .args(["plugins", "doctor", "--json"])
        .current_dir(cwd.path())
        .output()
        .expect("run plugins doctor");

    // With an empty HOME, every claude-plugin / agentskills entry is
    // missing → exit code 1.
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let missing = v["missing"].as_array().expect("missing array");
    assert!(
        !missing.is_empty(),
        "expected at least one missing plugin, got: {v}"
    );
    // Each missing entry must carry an actionable install_command (PRD-067 AC-6).
    for m in missing {
        assert!(
            m["install_command"].as_str().is_some(),
            "missing entry without install_command: {m}"
        );
    }
}

#[test]
fn plugins_doctor_text_emits_fix_line_when_missing() {
    let home = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();
    let output = forgeplan_with_clean_home(&home)
        .args(["plugins", "doctor"])
        .current_dir(cwd.path())
        .output()
        .expect("run plugins doctor");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Fix:"),
        "expected Fix: hint in text mode, got: {stdout}"
    );
}

#[test]
fn plugins_info_known_plugin_returns_details() {
    let home = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();
    forgeplan_with_clean_home(&home)
        .args(["plugins", "info", "c4-architecture"])
        .current_dir(cwd.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("c4-architecture"))
        .stdout(predicate::str::contains("claude plugin install"));
}

#[test]
fn plugins_info_unknown_plugin_exits_two_with_hint() {
    let home = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();
    let output = forgeplan_with_clean_home(&home)
        .args(["plugins", "info", "definitely-not-a-real-plugin"])
        .current_dir(cwd.path())
        .output()
        .expect("run plugins info");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not in registry"),
        "expected 'not in registry' in stderr, got: {stderr}"
    );
    assert!(
        stderr.contains("forgeplan plugins list"),
        "expected fallback hint, got: {stderr}"
    );
}

#[test]
fn plugins_info_unknown_plugin_json_exits_two() {
    let home = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();
    let output = forgeplan_with_clean_home(&home)
        .args(["plugins", "info", "definitely-not-a-real-plugin", "--json"])
        .current_dir(cwd.path())
        .output()
        .expect("run plugins info json");
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(v["error"].as_str().is_some());
    assert_eq!(v["_next_action"].as_str(), Some("forgeplan plugins list"));
}
