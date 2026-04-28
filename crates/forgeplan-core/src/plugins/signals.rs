//! Project signal sniffer — populates [`ProjectSignals`] from a workspace.
//!
//! The recommendation engine consumes signals (empty repo? has docs?
//! Cargo.toml? .obsidian vault?) to decide which playbooks apply
//! (PRD-067 FR-4, FR-5).
//!
//! This module is intentionally **filesystem-light**: a single non-recursive
//! probe per signal. Heavy ops (full `git log`) are gated behind `has_git` and
//! gracefully degrade — if `git` is missing or fails, `commit_count` stays 0.
//!
//! See [`PRD-067`](../../../../.forgeplan/prds/PRD-067-plugin-detection-self-describing-hints-playbook-recommendations.md).

use std::path::Path;
use std::process::Command;

use thiserror::Error;
use tracing::debug;

use super::types::ProjectSignals;

/// Errors raised by [`detect_signals`]. Most signal-detection issues are
/// folded into "false / 0" rather than errors; only structural problems
/// (missing root, unreadable directory) escape as `Err`.
#[derive(Debug, Error)]
pub enum SignalError {
    /// The workspace root does not exist or is not a directory.
    #[error("workspace root not found: {0}")]
    RootMissing(String),

    /// I/O failure while inspecting the workspace.
    #[error("io error while detecting signals: {0}")]
    Io(#[from] std::io::Error),
}

/// Threshold below which a directory is considered "essentially empty"
/// (PRD-067 AC-3 — `forgeplan init` on an empty repo recommends greenfield).
///
/// We allow up to 5 entries so that `.git`, `.gitignore`, `README.md`, and a
/// stray editor lockfile don't disqualify a freshly cloned scaffold.
const EMPTY_REPO_ENTRY_THRESHOLD: usize = 5;

/// Detect project signals at `workspace_root`.
///
/// Behavior summary (each field is a single non-recursive existence check
/// unless noted):
///
/// | Field | Source |
/// |---|---|
/// | `empty_repo` | dir entry count ≤ [`EMPTY_REPO_ENTRY_THRESHOLD`] |
/// | `has_git` | `.git/` exists |
/// | `commit_count` | `git rev-list --count HEAD` (0 on failure) |
/// | `has_docs` | any of `docs/`, `documentation/`, `wiki/` exists |
/// | `has_obsidian` | `.obsidian/` exists |
/// | `has_package_json` | `package.json` exists |
/// | `has_cargo_toml` | `Cargo.toml` exists |
/// | `has_pyproject_toml` | `pyproject.toml` exists |
/// | `has_dockerfile` | `Dockerfile` exists |
///
/// Returns `Err(SignalError::RootMissing)` if the path does not point at an
/// existing directory.
pub fn detect_signals(workspace_root: &Path) -> Result<ProjectSignals, SignalError> {
    if !workspace_root.is_dir() {
        return Err(SignalError::RootMissing(
            workspace_root.display().to_string(),
        ));
    }

    let mut signals = signals_from_tempdir(workspace_root);

    // Augment with git metrics (only if `.git` is present).
    if signals.has_git {
        signals.commit_count = git_commit_count(workspace_root).unwrap_or(0);
    }

    Ok(signals)
}

/// Pure filesystem-only signal probe — does **not** invoke git.
///
/// Suitable for tests (which avoid running `git`) and for callers who already
/// have commit-count data from elsewhere. `has_git` is still set based on
/// `.git/` presence; only `commit_count` differs from [`detect_signals`].
pub fn signals_from_tempdir(root: &Path) -> ProjectSignals {
    let entry_count = count_entries(root);

    ProjectSignals {
        empty_repo: entry_count <= EMPTY_REPO_ENTRY_THRESHOLD,
        has_git: root.join(".git").is_dir(),
        commit_count: 0,
        has_docs: ["docs", "documentation", "wiki"]
            .iter()
            .any(|d| root.join(d).is_dir()),
        has_obsidian: root.join(".obsidian").is_dir(),
        has_package_json: root.join("package.json").is_file(),
        has_cargo_toml: root.join("Cargo.toml").is_file(),
        has_pyproject_toml: root.join("pyproject.toml").is_file(),
        has_dockerfile: root.join("Dockerfile").is_file(),
    }
}

/// Count the entries directly inside `dir`. Returns `usize::MAX` if the
/// directory is unreadable so the "empty" heuristic does not falsely fire.
fn count_entries(dir: &Path) -> usize {
    match std::fs::read_dir(dir) {
        Ok(iter) => iter.filter_map(|e| e.ok()).count(),
        Err(err) => {
            debug!(path = %dir.display(), error = %err, "read_dir failed during signal detection");
            usize::MAX
        }
    }
}

/// Invoke `git rev-list --count HEAD` and parse the result.
///
/// Returns `None` if git is missing, the command fails (e.g. no commits yet
/// in a fresh repo), or the output is unparseable.
fn git_commit_count(root: &Path) -> Option<u32> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        debug!(
            status = ?output.status,
            "git rev-list --count failed (likely empty repo or git missing)"
        );
        return None;
    }

    let text = String::from_utf8(output.stdout).ok()?;
    text.trim().parse::<u32>().ok()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn empty_repo_detected_when_dir_is_almost_empty() {
        let tmp = TempDir::new().unwrap();
        // Zero entries → empty.
        let signals = signals_from_tempdir(tmp.path());
        assert!(signals.empty_repo);
        assert!(!signals.has_git);
        assert_eq!(signals.commit_count, 0);
    }

    #[test]
    fn cargo_toml_detected() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let signals = signals_from_tempdir(tmp.path());
        assert!(signals.has_cargo_toml);
        assert!(!signals.has_package_json);
    }

    #[test]
    fn docs_directory_detected() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("docs")).unwrap();
        let signals = signals_from_tempdir(tmp.path());
        assert!(signals.has_docs);
    }

    #[test]
    fn documentation_directory_alias_detected() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("documentation")).unwrap();
        let signals = signals_from_tempdir(tmp.path());
        assert!(signals.has_docs);
    }

    #[test]
    fn obsidian_marker_detected() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".obsidian")).unwrap();
        let signals = signals_from_tempdir(tmp.path());
        assert!(signals.has_obsidian);
    }

    #[test]
    fn package_json_detected() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("package.json"), "{}").unwrap();
        let signals = signals_from_tempdir(tmp.path());
        assert!(signals.has_package_json);
    }

    #[test]
    fn pyproject_toml_detected() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("pyproject.toml"), "[project]").unwrap();
        let signals = signals_from_tempdir(tmp.path());
        assert!(signals.has_pyproject_toml);
    }

    #[test]
    fn dockerfile_detected() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Dockerfile"), "FROM scratch\n").unwrap();
        let signals = signals_from_tempdir(tmp.path());
        assert!(signals.has_dockerfile);
    }

    #[test]
    fn multiple_flags_combine() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        fs::create_dir_all(tmp.path().join("docs")).unwrap();
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        fs::write(tmp.path().join("Dockerfile"), "FROM rust:1\n").unwrap();
        // Add filler entries to push past the empty threshold.
        for i in 0..6 {
            fs::write(tmp.path().join(format!("file_{i}")), "").unwrap();
        }

        let signals = signals_from_tempdir(tmp.path());
        assert!(signals.has_cargo_toml);
        assert!(signals.has_docs);
        assert!(signals.has_git);
        assert!(signals.has_dockerfile);
        assert!(!signals.empty_repo);
    }

    #[test]
    fn detect_signals_returns_error_on_missing_root() {
        let bogus = Path::new("/nonexistent/forgeplan/test/path/zzz");
        let result = detect_signals(bogus);
        assert!(matches!(result, Err(SignalError::RootMissing(_))));
    }

    #[test]
    fn detect_signals_works_on_existing_dir_without_git() {
        let tmp = TempDir::new().unwrap();
        // No .git → commit_count stays 0, no git invocation.
        let signals = detect_signals(tmp.path()).expect("detect should succeed");
        assert!(!signals.has_git);
        assert_eq!(signals.commit_count, 0);
    }
}
