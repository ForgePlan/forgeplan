//! Integration tests for `forgeplan undo-last` (PRD-070, FR-004).
//!
//! Happy-path undo is exercised by the core undo module's own unit
//! tests; here we focus on CLI-shape contract:
//! - `--help` describes the command (FR-011 parity with MCP description).
//! - empty trash exits with code 1 and a helpful error.
//! - `--json` produces parseable JSON in the no-candidate case.

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

#[test]
fn undo_last_help_describes_reversal() {
    forgeplan()
        .args(["undo-last", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reverse"))
        .stdout(predicate::str::contains("destructive"));
}

#[test]
fn undo_last_empty_trash_exits_nonzero() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    forgeplan()
        .args(["undo-last"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("No non-consumed destructive op"));
}

#[test]
fn undo_last_empty_trash_json_parses() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let out = forgeplan()
        .args(["undo-last", "--json"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
    assert_eq!(v["ok"], false);
    assert!(
        v["error"]
            .as_str()
            .unwrap_or_default()
            .contains("No non-consumed destructive op")
    );
}

#[test]
fn undo_last_within_hours_arg_accepted() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    forgeplan()
        .args(["undo-last", "--within-hours", "720"])
        .current_dir(tmp.path())
        .assert()
        .failure() // empty trash, but the flag must parse
        .stderr(predicate::str::contains("720"));
}
