//! Integration tests for `forgeplan restore <id>` (PRD-070, FR-003).
//!
//! Happy-path restore is exercised through the core undo module's own
//! unit tests; here we focus on the CLI-shape contract:
//! - `--help` describes the command (FR-011 parity with MCP description).
//! - "no receipt" prints a helpful error and exits with code 1.
//! - `--json` produces parseable JSON for the no-receipt case.

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
fn restore_help_describes_recovery() {
    forgeplan()
        .args(["restore", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("soft-deleted"))
        .stdout(predicate::str::contains("receipt"));
}

#[test]
fn restore_missing_receipt_exits_nonzero() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    forgeplan()
        .args(["restore", "PRD-999"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("No non-consumed receipt"));
}

#[test]
fn restore_missing_receipt_json_parses() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let out = forgeplan()
        .args(["restore", "PRD-999", "--json"])
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
            .contains("No non-consumed receipt")
    );
}
