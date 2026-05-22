/// Shared git-interrogation helpers used by the workspace module.
///
/// Both functions are synchronous: they invoke `git rev-parse` via
/// `std::process::Command` — a short one-shot subprocess that completes
/// in microseconds in practice. This deliberately avoids pulling in
/// `tokio::process` for single-shot detection calls (mirrors the original
/// `id_alloc` convention).
///
/// Errors (git not installed, not inside a repo, non-UTF8 output) are
/// returned as `None` so callers degrade gracefully.
use std::path::{Path, PathBuf};

/// Run `git rev-parse --git-common-dir` from `cwd` and return the
/// canonicalized absolute path.
///
/// For a linked worktree the common-dir differs from the per-worktree
/// `.git` file/dir and points to the main repo's `.git` directory.
/// For a plain repository the result equals `<repo-root>/.git`.
///
/// Returns `None` when:
/// - `git` is not on the PATH
/// - `cwd` is not inside a git repository
/// - the output is empty or cannot be parsed
pub(crate) fn git_common_dir(cwd: &Path) -> Option<PathBuf> {
    let out = std::process::Command::new("git")
        .args([
            "-C",
            &cwd.to_string_lossy(),
            "rev-parse",
            "--git-common-dir",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if raw.is_empty() {
        return None;
    }
    let p = PathBuf::from(&raw);
    // `git rev-parse --git-common-dir` returns a relative path when
    // invoked inside a worktree; canonicalize against `cwd`.
    let abs = if p.is_absolute() { p } else { cwd.join(p) };
    Some(abs)
}

/// Run `git rev-parse --show-toplevel` from `cwd` and return the
/// canonicalized absolute path of the working tree root.
///
/// For a linked worktree this returns the linked worktree's own top-level
/// directory (i.e. the directory containing the `.git` file that points to
/// the common dir), NOT the main worktree. This is the property that makes
/// the `common-dir-parent ≠ show-toplevel` comparison reliable for
/// multi-worktree detection.
///
/// Returns `None` under the same conditions as [`git_common_dir`].
pub(crate) fn git_show_toplevel(cwd: &Path) -> Option<PathBuf> {
    let out = std::process::Command::new("git")
        .args(["-C", &cwd.to_string_lossy(), "rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if raw.is_empty() {
        return None;
    }
    let p = PathBuf::from(&raw);
    let abs = if p.is_absolute() { p } else { cwd.join(p) };
    Some(abs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_git_repo(dir: &Path) {
        Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(dir)
            .output()
            .expect("git init");
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "root"])
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "t@t.t")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "t@t.t")
            .output()
            .expect("git commit");
    }

    #[test]
    fn git_common_dir_returns_none_outside_repo() {
        let tmp = TempDir::new().unwrap();
        // Plain temp dir with no git repo — must return None gracefully.
        let result = git_common_dir(tmp.path());
        assert!(
            result.is_none(),
            "expected None outside a git repo, got {:?}",
            result
        );
    }

    #[test]
    fn git_show_toplevel_returns_none_outside_repo() {
        let tmp = TempDir::new().unwrap();
        let result = git_show_toplevel(tmp.path());
        assert!(
            result.is_none(),
            "expected None outside a git repo, got {:?}",
            result
        );
    }

    #[test]
    fn git_common_dir_returns_some_inside_repo() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        let result = git_common_dir(tmp.path());
        assert!(
            result.is_some(),
            "expected Some inside a git repo, got None"
        );
        let p = result.unwrap();
        // Should end with `.git`
        assert_eq!(
            p.file_name().and_then(|n| n.to_str()),
            Some(".git"),
            "common_dir should end with .git, got {}",
            p.display()
        );
    }

    #[test]
    fn git_show_toplevel_returns_repo_root() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        let result = git_show_toplevel(tmp.path());
        assert!(result.is_some(), "expected Some inside a git repo");
        let top = result.unwrap();
        // Canonicalize both for comparison (macOS /var → /private/var etc.).
        let canon_tmp = std::fs::canonicalize(tmp.path()).unwrap();
        let canon_top = std::fs::canonicalize(&top).unwrap();
        assert_eq!(canon_top, canon_tmp, "show-toplevel should equal repo root");
    }
}
