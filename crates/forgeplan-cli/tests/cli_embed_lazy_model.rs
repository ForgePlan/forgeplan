//! `embed` must not load the model when there is nothing to encode — PROB-103.
//!
//! The defect was reported by a user reading the output, because no gate runs
//! `embed` at all (PROB-102). These tests are the gate that was missing.
//!
//! They are gated on `semantic-search` for the same reason the command is, and
//! CI's `cargo nextest run` currently passes no features — so they compile in
//! CI and execute only locally until PROB-102 is closed. That is stated here
//! rather than left for the next reader to discover from a green `0 passed`.

#![cfg(feature = "semantic-search")]

use assert_cmd::Command;
use tempfile::TempDir;

fn fpl(ws: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("forgeplan").unwrap();
    cmd.current_dir(ws.path());
    cmd
}

fn init(ws: &TempDir) {
    fpl(ws).args(["init", "-y"]).output().expect("init");
}

/// An empty workspace has nothing to encode, so the model must stay on disk.
///
/// Note what this does and does not prove. It passes with the PROB-103 fix
/// reverted, because an empty workspace is caught by an older early return
/// (`No artifacts to embed.`) that predates it — verified by mutation. It is a
/// regression guard for that path, not evidence for this one. The test below
/// is the one that fails without the fix.
#[test]
fn no_artifacts_means_no_model_load() {
    let ws = TempDir::new().unwrap();
    init(&ws);

    let out = fpl(&ws).arg("embed").output().expect("embed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("Loading embedding model"),
        "an empty workspace must not load the model, got:\n{stdout}"
    );
}

/// The case the user actually hit: everything already embedded, and `embed`
/// run again. Before the fix this cost 8.22s on a 427-artifact workspace and
/// printed the loading banner every time.
#[test]
fn a_current_workspace_does_not_reload_the_model() {
    let ws = TempDir::new().unwrap();
    init(&ws);
    fpl(&ws)
        .args(["new", "note", "Something to index"])
        .output()
        .expect("new");

    // First run does the work — the banner is expected here.
    let first = fpl(&ws).arg("embed").output().expect("embed");
    let first_out = String::from_utf8_lossy(&first.stdout);
    assert!(
        first_out.contains("Loading embedding model"),
        "a workspace with unembedded artifacts must load the model, got:\n{first_out}"
    );

    // Second run has nothing left to do.
    let second = fpl(&ws).arg("embed").output().expect("embed");
    let second_out = String::from_utf8_lossy(&second.stdout);
    assert!(
        !second_out.contains("Loading embedding model"),
        "nothing changed, so the model must not be loaded again, got:\n{second_out}"
    );
    assert!(
        second_out.contains("0 embedded"),
        "the summary must still report the outcome, got:\n{second_out}"
    );
    assert!(
        second_out.contains("already current"),
        "the skipped count must still be reported, got:\n{second_out}"
    );
}
