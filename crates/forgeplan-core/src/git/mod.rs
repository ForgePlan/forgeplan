//! Git integration — detect changed .forgeplan/ files from git operations.

use std::path::Path;
use std::process::Command;

/// Files changed in .forgeplan/ between two git refs.
#[derive(Debug, Clone)]
pub struct GitChangedFile {
    /// Relative path from repo root (e.g., ".forgeplan/prds/PRD-001-auth.md")
    pub path: String,
    /// Git change status: A (added), M (modified), D (deleted)
    pub status: char,
}

/// Get the current HEAD commit hash (short, 7 chars).
pub fn head_commit_hash(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Get the merge base (common ancestor) between HEAD and a ref.
pub fn merge_base(repo_root: &Path, ref_name: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["merge-base", "HEAD", ref_name])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Detect .forgeplan/ files changed between two git refs (or since last N commits).
/// Returns list of changed files with their status.
pub fn changed_artifact_files(
    repo_root: &Path,
    since_ref: &str,
) -> anyhow::Result<Vec<GitChangedFile>> {
    let output = Command::new("git")
        .args([
            "diff",
            "--name-status",
            since_ref,
            "HEAD",
            "--",
            ".forgeplan/",
        ])
        .current_dir(repo_root)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git (is git installed?): {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git diff failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(2, '\t').collect();
        if parts.len() == 2 {
            let status = parts[0].chars().next().unwrap_or('M');
            let path = parts[1].to_string();
            // Only .md files in artifact dirs
            if path.ends_with(".md") {
                files.push(GitChangedFile { path, status });
            }
        }
    }

    Ok(files)
}

/// Get the ORIG_HEAD ref (set after git pull/merge/rebase).
/// Returns None if ORIG_HEAD doesn't exist (no recent merge).
pub fn orig_head(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "ORIG_HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn head_commit_hash_returns_7_chars() {
        // Run in current repo which is a git repo
        let hash = head_commit_hash(Path::new("."));
        assert!(hash.is_some(), "should find HEAD in git repo");
        let h = hash.unwrap();
        assert!(h.len() >= 7 && h.len() <= 12, "short hash should be 7-12 chars, got {}", h.len());
    }

    #[test]
    fn head_commit_hash_returns_none_for_non_repo() {
        let tmp = TempDir::new().unwrap();
        let hash = head_commit_hash(tmp.path());
        assert!(hash.is_none());
    }

    #[test]
    fn changed_artifact_files_no_diff() {
        // HEAD..HEAD = no changes
        let files = changed_artifact_files(Path::new("."), "HEAD");
        assert!(files.is_ok());
        assert!(files.unwrap().is_empty());
    }
}
