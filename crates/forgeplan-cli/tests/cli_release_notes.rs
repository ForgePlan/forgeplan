//! Integration tests for `forgeplan release-notes`.
//!
//! Builds a self-contained tempdir workspace + git history so the
//! end-to-end command surface (incl. `git log`) is exercised without
//! depending on the real repo state.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn git(dir: &Path) -> std::process::Command {
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(dir);
    cmd
}

fn run_git(dir: &Path, args: &[&str]) {
    let status = git(dir).args(args).output().expect("git exec");
    if !status.status.success() {
        panic!(
            "git {args:?} failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&status.stdout),
            String::from_utf8_lossy(&status.stderr),
        );
    }
}

/// Initialise a git-controlled workspace with isolated config.
fn init_workspace(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    run_git(tmp.path(), &["init", "-q", "-b", "main"]);
    // Isolated identity so we don't pull from user's .gitconfig in CI.
    run_git(
        tmp.path(),
        &["config", "user.email", "tester@forgeplan.test"],
    );
    run_git(tmp.path(), &["config", "user.name", "Tester"]);
    run_git(tmp.path(), &["config", "commit.gpgsign", "false"]);
    // commit baseline so tag points somewhere.
    run_git(tmp.path(), &["add", "-A"]);
    run_git(tmp.path(), &["commit", "-q", "-m", "chore: init workspace"]);
    run_git(tmp.path(), &["tag", "v0.30.0"]);
}

fn commit_all(dir: &Path, msg: &str) {
    run_git(dir, &["add", "-A"]);
    run_git(dir, &["commit", "-q", "-m", msg]);
}

#[test]
fn release_notes_categorises_prd_into_added() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    // Create a PRD and activate it.
    forgeplan()
        .args(["new", "prd", "Auth System"])
        .current_dir(tmp.path())
        .assert()
        .success();
    // Activation requires a filled artifact body — skip activation,
    // r_eff stays 0, but status flips after `activate` only if
    // validation passes. Status alone is the signal, so create a
    // second commit and rely on the body-as-active-artifact path
    // OR run with --draft. We use --draft so we don't depend on
    // a fully-shaped PRD here.
    commit_all(tmp.path(), "feat: add PRD-001 Auth System");

    // Markdown output, draft mode (quality gate disabled).
    let out = forgeplan()
        .args([
            "release-notes",
            "--since",
            "v0.30.0",
            "--output",
            "markdown",
            "--draft",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    assert!(
        s.contains("### Added") || s.contains("Added"),
        "expected an Added section in output:\n{s}"
    );
    assert!(s.contains("PRD-001"), "expected PRD-001 in output:\n{s}");
    assert!(s.contains("Auth System"), "title missing:\n{s}");
}

#[test]
fn release_notes_json_output_parses() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    forgeplan()
        .args(["new", "prd", "Tile dashboard"])
        .current_dir(tmp.path())
        .assert()
        .success();
    commit_all(tmp.path(), "feat: add PRD-001");

    let out = forgeplan()
        .args([
            "release-notes",
            "--since",
            "v0.30.0",
            "--output",
            "json",
            "--draft",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).expect("json output must parse");
    assert_eq!(v["since"], "v0.30.0");
    assert_eq!(v["draft"], true);
    assert!(v["added"].is_array());
    assert!(v["fixed"].is_array());
    assert!(v["security"].is_array());
    assert!(v["changed"].is_array());
    assert!(v["internal"].is_array());
    assert_eq!(v["total"].as_u64().unwrap(), 1);
}

#[test]
fn release_notes_quality_gate_filters_draft_artifacts() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    forgeplan()
        .args(["new", "prd", "Half-baked"])
        .current_dir(tmp.path())
        .assert()
        .success();
    commit_all(tmp.path(), "wip: half-baked PRD");

    // Without --draft → quality gate hides drafts that have no
    // evidence and r_eff=0.
    let out = forgeplan()
        .args(["release-notes", "--since", "v0.30.0", "--output", "json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).expect("json must parse");
    assert_eq!(
        v["total"].as_u64().unwrap(),
        0,
        "draft artifact must be filtered out without --draft, got: {v:#?}"
    );
}

#[test]
fn release_notes_text_output_has_no_markdown_chars() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    forgeplan()
        .args(["new", "prd", "Plain"])
        .current_dir(tmp.path())
        .assert()
        .success();
    commit_all(tmp.path(), "feat: plain PRD");

    let out = forgeplan()
        .args([
            "release-notes",
            "--since",
            "v0.30.0",
            "--output",
            "text",
            "--draft",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    assert!(!s.contains("###"), "text output must not contain ###:\n{s}");
    assert!(s.contains("Added:"), "expected Added: header:\n{s}");
}

#[test]
fn release_notes_rejects_invalid_output_format() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    forgeplan()
        .args(["release-notes", "--since", "v0.30.0", "--output", "yaml"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported"));
}

#[test]
fn release_notes_rejects_injection_in_since() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    // `--since '--upload-pack=…'` should be rejected by validate_git_ref.
    forgeplan()
        .args([
            "release-notes",
            "--since",
            "--upload-pack=evil",
            "--output",
            "json",
        ])
        .current_dir(tmp.path())
        .assert()
        .failure();
}
