//! PROB-062 — `forgeplan init` writes canonical `.gitignore` section and
//! `forgeplan health --json` surfaces drift when files leak through.
//!
//! Two surfaces covered:
//! 1. `init -y` on a virgin directory writes the marker-bounded forgeplan
//!    section without touching pre-existing user rules.
//! 2. `health --json` populates `gitignore_drift` when a tracked file
//!    matches a canonical drift pattern (lance/, state/, etc.). The
//!    entry MUST be advisory — it does NOT promote the verdict.

use std::fs;
use std::process::Command as StdCommand;

use assert_cmd::Command;
use tempfile::TempDir;

const FORGEPLAN_BEGIN_MARKER: &str =
    "# === forgeplan workspace runtime state (managed by `forgeplan init`) ===";
const FORGEPLAN_END_MARKER: &str = "# === end forgeplan section ===";

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Init on a virgin dir → `.gitignore` created with the canonical
/// managed block + every patterned path.
#[test]
fn init_creates_canonical_gitignore() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(root)
        .assert()
        .success();

    let gitignore = fs::read_to_string(root.join(".gitignore"))
        .expect(".gitignore must be created by `forgeplan init`");
    assert!(
        gitignore.contains(FORGEPLAN_BEGIN_MARKER),
        ".gitignore missing forgeplan BEGIN marker:\n{gitignore}"
    );
    assert!(
        gitignore.contains(FORGEPLAN_END_MARKER),
        ".gitignore missing forgeplan END marker:\n{gitignore}"
    );
    // Canonical paths must all be present.
    for needed in [
        ".forgeplan/lance/",
        ".forgeplan/.fastembed_cache/",
        ".forgeplan/session.yaml",
        ".forgeplan/state/",
        ".forgeplan/trash/",
        ".forgeplan/logs/",
        ".forgeplan/locks/",
    ] {
        assert!(
            gitignore.contains(needed),
            "canonical .gitignore missing {needed}\nfull contents:\n{gitignore}"
        );
    }
}

/// Pre-existing user `.gitignore` MUST survive `init`. The managed
/// block is appended; nothing outside the markers is rewritten.
#[test]
fn init_preserves_existing_gitignore_rules() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let user_rules = "# user-authored rules\ntarget/\nnode_modules/\n";
    fs::write(root.join(".gitignore"), user_rules).unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(root)
        .assert()
        .success();

    let merged = fs::read_to_string(root.join(".gitignore")).unwrap();
    // User rules intact.
    assert!(merged.contains("# user-authored rules"));
    assert!(merged.contains("target/"));
    assert!(merged.contains("node_modules/"));
    // Managed block appended.
    assert!(merged.contains(FORGEPLAN_BEGIN_MARKER));
    assert!(merged.contains(".forgeplan/lance/"));
}

/// `health --json` MUST list every gitignored-but-tracked file under
/// the `gitignore_drift` advisory field, and the verdict MUST NOT be
/// promoted by the drift alone (advisory by design — same contract as
/// PROB-063 phase mismatches).
#[test]
fn health_reports_gitignore_drift_when_present() {
    // `git` is a hard prerequisite of this test. If the image lacks it,
    // skip rather than fail — the contract under test is "drift is
    // observed when git tracks leaked files", which is nonsensical
    // without git.
    if StdCommand::new("git").arg("--version").output().is_err() {
        eprintln!("skipping: git binary not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Bootstrap a git repo + minimal user config so `git add` works.
    let init = StdCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["init", "-q", "--initial-branch=main"])
        .status()
        .unwrap();
    assert!(init.success(), "git init failed");
    let _ = StdCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "user.email", "test@example.com"])
        .status();
    let _ = StdCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "user.name", "test"])
        .status();

    // Initialize the workspace so health has something to scan.
    forgeplan()
        .args(["init", "-y"])
        .current_dir(root)
        .assert()
        .success();

    // Simulate a contributor who accidentally committed derived state.
    fs::create_dir_all(root.join(".forgeplan/lance")).unwrap();
    fs::write(root.join(".forgeplan/lance/leaked.lance"), "x").unwrap();
    fs::write(root.join(".forgeplan/session.yaml"), "focus: PRD-001\n").unwrap();

    // Force-add despite the just-written .gitignore — the whole point
    // is to capture an already-tracked leak.
    let add = StdCommand::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "add",
            "-f",
            ".forgeplan/lance/leaked.lance",
            ".forgeplan/session.yaml",
        ])
        .status()
        .unwrap();
    assert!(add.success(), "git add of leaked files failed");

    let output = forgeplan()
        .args(["health", "--json"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "health --json failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    // `gitignore_drift` is the agreed JSON key (mirror of the Rust
    // field on `HealthReport`). If this fails the CLI surface didn't
    // pick up the new field.
    assert!(
        stdout.contains("gitignore_drift") || stdout.contains(".forgeplan/lance/leaked.lance"),
        "health --json missing drift surface:\n{stdout}"
    );

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("health --json must emit valid JSON");
    let drift = parsed
        .get("gitignore_drift")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        !drift.is_empty(),
        "expected non-empty gitignore_drift, got: {parsed}"
    );

    let paths: Vec<&str> = drift
        .iter()
        .filter_map(|e| e.get("path").and_then(|p| p.as_str()))
        .collect();
    assert!(
        paths.iter().any(|p| p.contains("lance/leaked.lance")),
        "lance leak missing from drift: {paths:?}"
    );
    assert!(
        paths.iter().any(|p| p.contains("session.yaml")),
        "session.yaml leak missing from drift: {paths:?}"
    );

    // Advisory contract: drift alone MUST NOT promote the verdict.
    // The workspace is otherwise empty (no artifacts), so the verdict
    // should still be `empty`, NOT `unhealthy`.
    let verdict = parsed
        .get("verdict")
        .and_then(|v| v.as_str())
        .unwrap_or("<missing>");
    assert_ne!(
        verdict, "unhealthy",
        "drift alone must not promote verdict to unhealthy: {parsed}"
    );
}
