// PRD-057 increment 1 — workspace-level advisory file lock.
//
// Serializes LanceDB write operations (`store.create_artifact`,
// `update_artifact`, `next_id`) across concurrent MCP sessions in the
// same workspace (2-5 sub-agents sharing `.forgeplan/`).
//
// Design decisions:
// - **Advisory lock via `fs2::FileExt`** — OS primitive (flock on Unix,
//   LockFileEx on Windows). Released automatically when the guard
//   drops, including on process crash (OS-level release).
// - **Single lock file per workspace** — `.forgeplan/.lock`. One lock
//   serializes all writes, which is sufficient for the 2-5 agent target
//   scale where writes are rare relative to reads.
// - **spawn_blocking wrapper** — `fs2` is synchronous; acquisition runs
//   on a blocking thread so the async runtime is not stalled.
// - **Bounded wait via `try_lock_exclusive` + backoff** — prevents
//   indefinite hang on stuck sibling agent (audit Round 1 H-1). Default
//   ceiling 30s with exponential backoff (10ms → 1000ms).
// - **Symlink guards** — workspace dir AND lock-file path must not be
//   symlinks (audit Round 1 C-sec #1, parity with PRD-055 R3 + PRD-056).
// - **Graceful error propagation** — on unlock failure in Drop, log via
//   `tracing::warn` instead of swallowing (audit Round 1 M-3).

use anyhow::Context;
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Default upper bound on lock acquisition wait. A misbehaving sibling
/// agent stuck holding the lock for longer surfaces as a clean timeout
/// error instead of an indefinite hang.
pub const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(30);

/// RAII guard for an acquired workspace lock. Releases on drop via
/// `fs2::FileExt::unlock`, logging any error through `tracing::warn!`.
#[must_use = "dropping the WorkspaceLock releases the lock immediately — bind it to a named variable for the intended scope"]
pub struct WorkspaceLock {
    file: File,
}

impl std::fmt::Debug for WorkspaceLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceLock")
            .field("file", &"<held>")
            .finish()
    }
}

impl Drop for WorkspaceLock {
    fn drop(&mut self) {
        // Explicit unlock. `File`'s own Drop also closes + releases,
        // but being explicit documents intent and survives future
        // refactors. Errors are logged rather than swallowed silently
        // (audit Round 1 — no more `let _ =`).
        if let Err(e) = self.file.unlock() {
            tracing::warn!(
                error = %e,
                "workspace lock unlock failed — OS flock is still released on file close, \
                 but explicit unlock errored (NFS / stale fd?)"
            );
        }
    }
}

/// Path to the workspace lock file under `.forgeplan/.lock`.
pub fn lock_path(workspace: &Path) -> PathBuf {
    workspace.join(".lock")
}

/// Validate that a filesystem path (workspace dir or lock file) is not
/// a symlink. Audit Round 1 C-sec #1 — parity with PRD-055 R3 and
/// PRD-056 hardening. An attacker who plants a symlink where we expect
/// a real dir / file could redirect the lock onto arbitrary targets.
async fn refuse_if_symlink(path: &Path, label: &str) -> anyhow::Result<()> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                anyhow::bail!(
                    "{label} at {} is a symlink — refusing to open (parity with PRD-055 R3)",
                    path.display()
                );
            }
            Ok(())
        }
        // Path does not exist yet — fine, caller will create it.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Acquire an exclusive workspace-level lock. Uses `try_lock_exclusive`
/// with exponential backoff up to `timeout`. Returns a typed timeout
/// error if the lock cannot be obtained in time (audit Round 1 H-1).
///
/// For 2-5 agent scale this is simple and safe.
pub async fn acquire_workspace_lock(workspace: &Path) -> anyhow::Result<WorkspaceLock> {
    acquire_workspace_lock_with_timeout(workspace, DEFAULT_LOCK_TIMEOUT).await
}

/// Same as `acquire_workspace_lock` but with a caller-specified timeout.
/// Use shorter timeouts in tight loops (e.g. 5s in dispatch) and longer
/// for interactive handlers. A timeout of `Duration::ZERO` means
/// "try once, fail fast".
pub async fn acquire_workspace_lock_with_timeout(
    workspace: &Path,
    timeout: Duration,
) -> anyhow::Result<WorkspaceLock> {
    // Symlink guard on the workspace directory itself (C-sec #1).
    refuse_if_symlink(workspace, "workspace directory").await?;
    tokio::fs::create_dir_all(workspace)
        .await
        .with_context(|| format!("could not create workspace dir {}", workspace.display()))?;

    let path = lock_path(workspace);

    // Symlink guard on the lock file (C-sec #1). Must happen BEFORE
    // `open(create:true)` which would follow a pre-planted symlink.
    refuse_if_symlink(&path, "lock file").await?;

    let path_clone = path.clone();
    let file = tokio::task::spawn_blocking(move || -> anyhow::Result<File> {
        open_and_lock_with_backoff(&path_clone, timeout)
    })
    .await
    .context("lock acquisition task panicked")??;

    let _ = path; // suppress unused-var lint when debug disabled
    Ok(WorkspaceLock { file })
}

/// Blocking helper: open the lock file and repeatedly `try_lock_exclusive`
/// with exponential backoff until the lock is held or the timeout elapses.
fn open_and_lock_with_backoff(path: &Path, timeout: Duration) -> anyhow::Result<File> {
    // Minimal OpenOptions: create if missing, write-only. Previously
    // had `.read(true).truncate(false)` — unnecessary (audit Round 1 L-1).
    let f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)
        .with_context(|| format!("could not open lock file {}", path.display()))?;

    let start = std::time::Instant::now();
    let mut delay = Duration::from_millis(10);
    const MAX_DELAY: Duration = Duration::from_millis(1000);

    loop {
        match f.try_lock_exclusive() {
            Ok(()) => return Ok(f),
            Err(e) => {
                // Distinguish "already locked" from other IO errors —
                // retry on would-block, fail-fast on anything else.
                let transient = e.kind() == std::io::ErrorKind::WouldBlock;
                if !transient {
                    return Err(anyhow::anyhow!(
                        "could not acquire lock on {}: {e}",
                        path.display()
                    ));
                }
                if start.elapsed() >= timeout {
                    anyhow::bail!(
                        "workspace lock {} held by another agent; timed out after {:?}",
                        path.display(),
                        timeout
                    );
                }
                std::thread::sleep(delay);
                // Exponential backoff, capped — avoids busy loop without
                // starving latency for the typical short-held case.
                delay = std::cmp::min(delay * 2, MAX_DELAY);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn ws(tmp: &TempDir) -> std::path::PathBuf {
        tmp.path().join(".forgeplan")
    }

    #[tokio::test]
    async fn acquire_creates_lock_file() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let _guard = acquire_workspace_lock(&ws).await.unwrap();
        assert!(lock_path(&ws).exists(), "lock file must be created");
    }

    #[tokio::test]
    async fn lock_releases_on_drop() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        {
            let _g1 = acquire_workspace_lock(&ws).await.unwrap();
        }
        let _g2 = acquire_workspace_lock(&ws).await.unwrap();
    }

    #[tokio::test]
    async fn concurrent_acquirers_serialize_and_total_time() {
        // Audit Round 1: strengthen the serialize test with wall-time
        // lower-bound assertion — if serialization were broken, peak=1
        // could still accidentally hold but total time would drop below
        // the serial lower bound.
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::time::{Instant, sleep};

        let tmp = TempDir::new().unwrap();
        let ws = Arc::new(ws(&tmp));
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_in_flight = Arc::new(AtomicUsize::new(0));

        let hold_ms = 30;
        let n = 4usize;
        let started = Instant::now();

        let mut handles = Vec::new();
        for _ in 0..n {
            let ws = ws.clone();
            let in_flight = in_flight.clone();
            let max_in_flight = max_in_flight.clone();
            handles.push(tokio::spawn(async move {
                let _g = acquire_workspace_lock(&ws).await.unwrap();
                let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                max_in_flight.fetch_max(current, Ordering::SeqCst);
                sleep(Duration::from_millis(hold_ms)).await;
                in_flight.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        let elapsed = started.elapsed();
        let peak = max_in_flight.load(Ordering::SeqCst);
        assert_eq!(
            peak, 1,
            "exclusive lock: at most 1 concurrent holder, got {peak}"
        );
        // Serial lower bound: n holders × hold_ms, minus small slack for
        // test scheduling jitter.
        let expected_min = Duration::from_millis(hold_ms * n as u64 - 10);
        assert!(
            elapsed >= expected_min,
            "elapsed {elapsed:?} < serial lower bound {expected_min:?} — serialization suspect"
        );
    }

    #[tokio::test]
    async fn timeout_surfaces_when_lock_held() {
        // Audit H-1: timeout returns a clear error instead of hanging.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let _first = acquire_workspace_lock(&ws).await.unwrap();

        let started = std::time::Instant::now();
        let err = acquire_workspace_lock_with_timeout(&ws, Duration::from_millis(200))
            .await
            .unwrap_err();
        let elapsed = started.elapsed();

        assert!(
            err.to_string().contains("timed out") || err.to_string().contains("held by"),
            "timeout error must explain cause: {err}"
        );
        // Should respect timeout approximately — not instant, not 30s.
        assert!(
            elapsed >= Duration::from_millis(200) && elapsed < Duration::from_millis(2_000),
            "elapsed {elapsed:?} outside expected [200ms, 2s] timeout window"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlinked_workspace_dir_is_refused() {
        // Audit Round 1 C-sec #1: refuse pre-planted symlink as workspace.
        let tmp = TempDir::new().unwrap();
        let real = tmp.path().join("real");
        std::fs::create_dir_all(&real).unwrap();
        let sym = tmp.path().join("symlinked-ws");
        std::os::unix::fs::symlink(&real, &sym).unwrap();

        let err = acquire_workspace_lock(&sym).await.unwrap_err();
        assert!(
            err.to_string().contains("symlink"),
            "symlinked workspace must be refused: {err}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlinked_lock_file_is_refused() {
        // Audit Round 1 C-sec #1: refuse pre-planted symlink as lock file.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        std::fs::create_dir_all(&ws).unwrap();

        let victim = tmp.path().join("victim.txt");
        std::fs::write(&victim, b"sensitive").unwrap();
        std::os::unix::fs::symlink(&victim, ws.join(".lock")).unwrap();

        let err = acquire_workspace_lock(&ws).await.unwrap_err();
        assert!(
            err.to_string().contains("symlink"),
            "symlinked lock file must be refused: {err}"
        );
        // Victim untouched.
        let still = std::fs::read_to_string(&victim).unwrap();
        assert_eq!(still, "sensitive");
    }
}
