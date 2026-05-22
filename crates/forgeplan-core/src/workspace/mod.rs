pub(crate) mod git;
pub mod init;
pub mod lock;

pub use init::detect_multi_worktree;
pub use init::{ARTIFACT_DIRS, FORGEPLAN_DIR, find_workspace, init_workspace, load_config};
pub use lock::{WorkspaceLock, acquire_workspace_lock, lock_path};

/// Return the best-effort suggested workspace path for an MCP error message
/// when multi-worktree detection fires and the caller provided no explicit
/// `workspace` parameter.
///
/// Returns `git show-toplevel` (canonicalized) if available, or `cwd` as a
/// fallback.  The result is always an absolute path suitable for inclusion in
/// a `workspace=` hint in the error response.
pub fn suggest_workspace_path(cwd: &std::path::Path) -> std::path::PathBuf {
    git::git_show_toplevel(cwd)
        .and_then(|p| std::fs::canonicalize(&p).ok())
        .unwrap_or_else(|| cwd.to_path_buf())
}
