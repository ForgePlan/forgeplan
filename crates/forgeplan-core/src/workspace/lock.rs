// PRD-057 increment 1 — workspace-level advisory file lock.
//
// Serializes LanceDB write operations (`store.create_artifact`,
// `update_artifact`, `next_id`) across concurrent MCP sessions in the
// same workspace (2-5 sub-agents sharing `.forgeplan/`).
//
// Design decisions:
// - **Advisory lock via `fs2::FileExt::lock_exclusive`** — OS primitive
//   (flock on Unix, LockFileEx on Windows). Blocks until acquired (no
//   busy loop). Released automatically when the `LockFile` guard drops,
//   including on process crash (OS-level release).
// - **Single lock file per workspace** — `.forgeplan/.lock`. One lock
//   serializes all writes, which is sufficient for the 2-5 agent target
//   scale where writes are rare relative to reads.
// - **spawn_blocking wrapper** — `fs2` is synchronous; we wrap acquisition
//   in `tokio::task::spawn_blocking` so the async runtime is not stalled
//   while waiting for the lock.
// - **Graceful degradation on lock-file IO errors** — if the lock file
//   cannot be created (e.g. read-only FS), we return the error so the
//   caller can decide to proceed unlocked or bail. LanceDB internal
//   locking still provides some safety.

use anyhow::Context;
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

/// RAII guard for an acquired workspace lock. Releases on drop via
/// `fs2::FileExt::unlock` (implicit when the file handle is closed by
/// Rust's Drop impl, belt-and-suspenders here).
pub struct WorkspaceLock {
    _file: File,
    _path: PathBuf,
}

impl Drop for WorkspaceLock {
    fn drop(&mut self) {
        // Explicit unlock — `File`'s Drop will also close + release,
        // but being explicit documents the intent and survives
        // future refactors that might stash `_file`.
        let _ = self._file.unlock();
    }
}

/// Path to the workspace lock file under `.forgeplan/.lock`.
pub fn lock_path(workspace: &Path) -> PathBuf {
    workspace.join(".lock")
}

/// Acquire an exclusive workspace-level lock. Blocking — uses
/// `tokio::task::spawn_blocking` so async callers don't stall the
/// runtime. Returns a guard that releases on drop.
///
/// For 2-5 agent scale this is simple and safe. For larger scale
/// (10+ agents) a per-artifact or per-operation lock scheme would
/// be needed, but that is out of scope for PRD-057.
pub async fn acquire_workspace_lock(workspace: &Path) -> anyhow::Result<WorkspaceLock> {
    let path = lock_path(workspace);
    // Ensure workspace dir exists — the lock file sits under it.
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("could not create lock parent dir {}", parent.display()))?;
    }

    let path_clone = path.clone();
    let file = tokio::task::spawn_blocking(move || -> anyhow::Result<File> {
        let f = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path_clone)
            .with_context(|| format!("could not open lock file {}", path_clone.display()))?;
        f.lock_exclusive().with_context(|| {
            format!(
                "could not acquire exclusive lock on {}",
                path_clone.display()
            )
        })?;
        Ok(f)
    })
    .await
    .context("lock acquisition task panicked")??;

    Ok(WorkspaceLock {
        _file: file,
        _path: path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn acquire_creates_lock_file() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().to_path_buf();
        let _guard = acquire_workspace_lock(&ws).await.unwrap();
        assert!(lock_path(&ws).exists(), "lock file must be created");
    }

    #[tokio::test]
    async fn lock_releases_on_drop() {
        // Two sequential acquisitions from the same async task succeed —
        // the first guard is dropped before the second tries.
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().to_path_buf();
        {
            let _g1 = acquire_workspace_lock(&ws).await.unwrap();
        } // drop here
        let _g2 = acquire_workspace_lock(&ws).await.unwrap();
    }

    #[tokio::test]
    async fn concurrent_acquirers_serialize() {
        // Two concurrent tasks both acquire — second must wait for first
        // to release. We detect serialization by checking that at most
        // one guard is alive at any observable checkpoint.
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::time::{Duration, sleep};

        let tmp = TempDir::new().unwrap();
        let ws = Arc::new(tmp.path().to_path_buf());
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_in_flight = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..4 {
            let ws = ws.clone();
            let in_flight = in_flight.clone();
            let max_in_flight = max_in_flight.clone();
            handles.push(tokio::spawn(async move {
                let _g = acquire_workspace_lock(&ws).await.unwrap();
                let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                max_in_flight.fetch_max(current, Ordering::SeqCst);
                sleep(Duration::from_millis(20)).await;
                in_flight.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        let peak = max_in_flight.load(Ordering::SeqCst);
        assert_eq!(
            peak, 1,
            "exclusive lock: at most 1 concurrent holder, got {peak}"
        );
    }
}
