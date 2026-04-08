//! PRD-041 Sprint 13.6: integration tests for `forgeplan fpf rules` and `forgeplan fpf check`.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init_ws(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

fn make_prd(tmp: &TempDir) {
    forgeplan()
        .args(["new", "prd", "Test PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

#[test]
fn cli_fpf_rules_shows_default_source() {
    let tmp = TempDir::new().unwrap();
    init_ws(&tmp);

    forgeplan()
        .args(["fpf", "rules"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Default"))
        .stdout(predicate::str::contains("blind-spot"));
}

#[test]
fn cli_fpf_rules_json_valid() {
    let tmp = TempDir::new().unwrap();
    init_ws(&tmp);

    let output = forgeplan()
        .args(["fpf", "rules", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    let j: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
    assert!(j.get("rules").is_some(), "has rules key");
    assert!(j["rules"].as_array().unwrap().len() > 0, "rules non-empty");
    assert!(j.get("source").is_some());
    assert!(j.get("count").is_some());
}

#[test]
fn cli_fpf_rules_flat_has_priorities() {
    let tmp = TempDir::new().unwrap();
    init_ws(&tmp);

    forgeplan()
        .args(["fpf", "rules", "--flat"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("["));
}

#[test]
fn cli_fpf_check_missing_artifact_errors() {
    let tmp = TempDir::new().unwrap();
    init_ws(&tmp);

    forgeplan()
        .args(["fpf", "check", "NOPE-999"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

#[test]
fn cli_fpf_check_existing_artifact() {
    let tmp = TempDir::new().unwrap();
    init_ws(&tmp);
    make_prd(&tmp);

    forgeplan()
        .args(["fpf", "check", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));
}

#[test]
fn cli_fpf_check_json_has_required_fields() {
    let tmp = TempDir::new().unwrap();
    init_ws(&tmp);
    make_prd(&tmp);

    let output = forgeplan()
        .args(["fpf", "check", "PRD-001", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    let j: serde_json::Value = serde_json::from_str(&s).expect("valid JSON");
    for key in [
        "artifact_id",
        "artifact_kind",
        "artifact_status",
        "matched",
        "unmatched",
    ] {
        assert!(j.get(key).is_some(), "missing key: {key}");
    }
    assert!(j["matched"].is_array());
    assert!(j["unmatched"].is_array());
}
