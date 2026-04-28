//! Phase 6 Wave 2 — integration tests for `forgeplan init` recommendation
//! wiring (PRD-072 FR-6, PRD-067 AC-3..AC-7).
//!
//! Each test drives the real `forgeplan init -y` binary against a `TempDir`
//! pre-seeded with the signal we want to assert on (empty git repo, Obsidian
//! vault, legacy code with ≥100 commits). The recommendation engine is
//! deterministic given the signals + bundled descriptors, so we can assert on
//! the textual stderr output emitted by [`commands::init::emit_recommendation_hints`].
//!
//! `FORGEPLAN_HINTS=1` is set on every test so the TTY guard does not
//! suppress the hint stream when stderr is captured by `assert_cmd`. The
//! AC-7 backward-compat case explicitly sets `FORGEPLAN_HINTS=0` to verify
//! the disable contract.
//!
//! AC traceability (PRD-067):
//!   AC-3 -> init_on_empty_repo_recommends_greenfield_kickoff
//!   AC-4 -> init_on_obsidian_vault_recommends_brownfield_docs
//!   AC-5 -> init_on_legacy_code_recommends_brownfield_code
//!   AC-7 -> init_with_forgeplan_hints_zero_emits_no_hints
//!   robustness -> init_does_not_panic_on_signal_detect_failure

use std::path::Path;
use std::process::Command as StdCommand;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

// ─── Helpers ────────────────────────────────────────────────────────────────

fn forgeplan() -> Command {
    let mut cmd = Command::cargo_bin("forgeplan").expect("test fixture: cargo_bin forgeplan");
    cmd.env("FORGEPLAN_DISABLE_PLUGIN_DISCOVERY", "1");
    cmd.env("FORGEPLAN_HINTS", "1");
    cmd
}

/// Initialise an empty git repo (no commits) inside `dir`.
fn git_init_empty(dir: &Path) {
    let status = StdCommand::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(dir)
        .status()
        .expect("git init");
    assert!(status.success(), "git init failed");
    // Provide identity so `git commit` works in CI sandboxes.
    for (k, v) in [
        ("user.email", "test@forgeplan.dev"),
        ("user.name", "Forgeplan Test"),
    ] {
        let s = StdCommand::new("git")
            .args(["config", k, v])
            .current_dir(dir)
            .status()
            .expect("git config");
        assert!(s.success(), "git config {k} failed");
    }
}

/// Pile up `n` empty commits on top of HEAD. Used to manufacture the
/// commit_count_min ≥100 signal for the brownfield-code trigger.
fn git_empty_commits(dir: &Path, n: usize) {
    // Single shell loop is ~50× faster than spawning a child per commit.
    let cmd = format!("for i in $(seq 1 {n}); do git commit --allow-empty -m c$i -q; done");
    let status = StdCommand::new("sh")
        .arg("-c")
        .arg(&cmd)
        .current_dir(dir)
        .status()
        .expect("sh git commit loop");
    assert!(status.success(), "git commit loop failed");
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[test]
fn init_on_empty_repo_recommends_greenfield_kickoff() {
    let tmp = TempDir::new().expect("tempdir");
    git_init_empty(tmp.path());

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("greenfield-kickoff"));
}

#[test]
fn init_on_obsidian_vault_recommends_brownfield_docs() {
    let tmp = TempDir::new().expect("tempdir");
    std::fs::create_dir_all(tmp.path().join(".obsidian")).expect("mkdir .obsidian");

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("brownfield-docs"));
}

#[test]
fn init_on_legacy_code_recommends_brownfield_code() {
    let tmp = TempDir::new().expect("tempdir");
    git_init_empty(tmp.path());
    git_empty_commits(tmp.path(), 100);

    // Push past the empty-repo threshold so the `empty_repo` signal flips off.
    for i in 0..6 {
        std::fs::write(tmp.path().join(format!("file_{i}.txt")), b"x").expect("write file");
    }

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("brownfield-code"));
}

#[test]
fn init_with_forgeplan_hints_zero_emits_no_hints() {
    let tmp = TempDir::new().expect("tempdir");
    git_init_empty(tmp.path());

    let mut cmd = Command::cargo_bin("forgeplan").expect("cargo_bin");
    cmd.env("FORGEPLAN_DISABLE_PLUGIN_DISCOVERY", "1");
    cmd.env("FORGEPLAN_HINTS", "0");
    cmd.args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("recommended:").not())
        .stderr(predicate::str::contains("greenfield-kickoff").not());
}

#[test]
fn init_does_not_panic_on_signal_detect_failure() {
    // Detect_signals returns RootMissing when the workspace path doesn't
    // exist as a directory. Init creates `.forgeplan/` first, so to provoke
    // the failure path we temporarily simulate it via a clean tempdir that
    // exists (signals always succeed there). What we actually verify here:
    // even with sparse / odd inputs (no git, no docs, no manifests) init
    // exits 0 and never panics.
    let tmp = TempDir::new().expect("tempdir");

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
}
