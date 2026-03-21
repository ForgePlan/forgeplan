use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

#[test]
fn init_creates_workspace() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(".forgeplan/"));

    assert!(tmp.path().join(".forgeplan").exists());
    assert!(tmp.path().join(".forgeplan/config.yaml").exists());
    assert!(tmp.path().join(".forgeplan/prds").is_dir());
    assert!(tmp.path().join(".forgeplan/rfcs").is_dir());
}

#[test]
fn init_idempotent_without_force() {
    let tmp = TempDir::new().unwrap();

    // First init succeeds
    forgeplan()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    // Second init succeeds but warns
    forgeplan()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Already initialized"));
}

#[test]
fn new_creates_artifact() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Test Feature"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));

    let prd_dir = tmp.path().join(".forgeplan/prds");
    let entries: Vec<_> = std::fs::read_dir(&prd_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 1);
    assert!(entries[0]
        .file_name()
        .to_string_lossy()
        .contains("PRD-001"));
}

#[test]
fn new_auto_increments_id() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "rfc", "First RFC"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("RFC-001"));

    forgeplan()
        .args(["new", "rfc", "Second RFC"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("RFC-002"));
}

#[test]
fn list_shows_artifacts() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "My Feature"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .arg("list")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("My Feature"));
}

#[test]
fn status_shows_dashboard() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Feature X"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .arg("status")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("prd"));
}

#[test]
fn validate_checks_artifact() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Validation Test"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Validate the newly created PRD (template has placeholders, so should have findings)
    forgeplan()
        .args(["validate", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .stdout(predicate::str::contains("PRD-001"));
}

#[test]
fn link_creates_relationship() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "My PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "rfc", "My RFC"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["link", "RFC-001", "PRD-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Linked"));
}

#[test]
fn graph_outputs_mermaid() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "rfc", "RFC"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["link", "RFC-001", "PRD-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .arg("graph")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("graph LR"))
        .stdout(predicate::str::contains("RFC-001"))
        .stdout(predicate::str::contains("PRD-001"));
}

#[test]
fn search_finds_content() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Authentication System"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["search", "Authentication"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("Authentication"));
}

#[test]
fn stale_runs_without_error() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .arg("stale")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No stale"));
}

#[test]
fn score_without_evidence() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .arg("init")
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "adr", "Test Decision"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["score", "ADR-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No evidence"));
}

#[test]
fn no_workspace_gives_error() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .arg("list")
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("forgeplan init"));
}
