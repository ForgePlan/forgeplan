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
    workspace: Option<PathBuf>,
    repair_binding: Option<RepairBinding>,
}

impl std::fmt::Debug for WorkspaceLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceLock")
            .field("file", &"<held>")
            .finish()
    }
}

impl WorkspaceLock {
    /// Wrap an already-locked `File` in the RAII guard. Used by callers
    /// that need a custom lock-file path (e.g. `artifact::id_alloc` for
    /// cross-worktree per-kind locks). The caller is responsible for
    /// having taken the exclusive `flock` before constructing the guard;
    /// `Drop` will release it.
    pub fn from_file(file: File) -> Self {
        Self { file, workspace: None, repair_binding: None }
    }

    pub(crate) fn is_repair_guard_for(&self, workspace: &Path, transaction_id: &str) -> bool {
        self.repair_binding.as_ref().is_some_and(|binding| {
            !binding.semantic && !binding.blocked_legacy && binding.workspace == workspace && binding.transaction_id == transaction_id
        })
    }

    pub(crate) fn is_legacy_repair_guard_for(&self, workspace: &Path, transaction_id: &str) -> bool {
        self.repair_binding.as_ref().is_some_and(|binding| {
            !binding.semantic && binding.legacy && !binding.blocked_legacy && binding.workspace == workspace && binding.transaction_id == transaction_id
        })
    }

    pub(crate) fn legacy_repair_digest_for(&self, workspace: &Path, transaction_id: &str) -> Option<&str> {
        self.repair_binding.as_ref()
            .filter(|binding| !binding.semantic && binding.legacy && !binding.blocked_legacy && binding.workspace == workspace && binding.transaction_id == transaction_id)
            .map(|binding| binding.journal_digest.as_str())
    }

    pub(crate) fn is_blocked_legacy_repair_guard_for(&self, workspace: &Path, transaction_id: &str) -> bool {
        self.blocked_legacy_repair_digest_for(workspace, transaction_id).is_some()
    }

    pub(crate) fn blocked_legacy_repair_digest_for(&self, workspace: &Path, transaction_id: &str) -> Option<&str> {
        self.repair_binding.as_ref()
            .filter(|binding| !binding.semantic && binding.legacy && binding.blocked_legacy && binding.workspace == workspace && binding.transaction_id == transaction_id)
            .map(|binding| binding.journal_digest.as_str())
    }

    pub(crate) fn linked_legacy_repair_digests_for(
        &self, workspace: &Path, old_id: &str, new_id: &str,
    ) -> Option<(&str, &str)> {
        let binding = self.repair_binding.as_ref()?;
        if binding.semantic || !binding.legacy || binding.blocked_legacy || binding.workspace != workspace
            || binding.transaction_id != new_id
        {
            return None;
        }
        let (bound_old_id, old_digest) = binding.linked_predecessor.as_ref()?;
        (bound_old_id == old_id).then_some((old_digest.as_str(), binding.journal_digest.as_str()))
    }

    pub(crate) fn is_legacy_semantic_rebuild_guard_for(
        &self, workspace: &Path, transaction_id: &str, digest: &str,
    ) -> bool {
        self.repair_binding.as_ref().is_some_and(|binding| {
            binding.semantic && binding.workspace == workspace
                && binding.transaction_id == transaction_id && binding.journal_digest == digest
        })
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

/// A shared guard held for the entire materialization of a logical read.
#[must_use = "dropping the read guard releases the shared workspace lock"]
pub struct WorkspaceReadLock {
    file: File,
    workspace: PathBuf,
}

impl Drop for WorkspaceReadLock {
    fn drop(&mut self) {
        if let Err(error) = self.file.unlock() {
            tracing::warn!(%error, "workspace read lock unlock failed");
        }
    }
}

mod sealed {
    pub trait Sealed {}
}

/// Unforgeable workspace-bound proof for an ordinary read.
///
/// A writer may borrow its existing exclusive guard instead of re-locking.
/// Repair guards are deliberately excluded from ordinary reads.
pub trait WorkspaceReadPermit: sealed::Sealed + Send + Sync {
    fn matches_workspace(&self, workspace: &Path) -> anyhow::Result<()>;
}

impl sealed::Sealed for WorkspaceReadLock {}
impl WorkspaceReadPermit for WorkspaceReadLock {
    fn matches_workspace(&self, workspace: &Path) -> anyhow::Result<()> {
        if workspace.canonicalize()? != self.workspace {
            anyhow::bail!("read permit belongs to another workspace");
        }
        Ok(())
    }
}

impl sealed::Sealed for WorkspaceLock {}
impl WorkspaceReadPermit for WorkspaceLock {
    fn matches_workspace(&self, workspace: &Path) -> anyhow::Result<()> {
        if self.repair_binding.is_some() || self.workspace.as_deref() != Some(workspace.canonicalize()?.as_path()) {
            anyhow::bail!("exclusive guard is not an ordinary permit for this workspace");
        }
        crate::artifact::identity_repair::check_repair_journals(workspace, None)?;
        crate::db::migrate::check_legacy_semantic_rebuild(workspace)
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

/// Acquire ordinary read admission; retain the guard until all rows are materialized.
pub async fn acquire_workspace_read_lock(workspace: &Path) -> anyhow::Result<WorkspaceReadLock> {
    acquire_workspace_read_lock_with_timeout(workspace, DEFAULT_LOCK_TIMEOUT).await
}

async fn acquire_workspace_read_lock_with_timeout(
    workspace: &Path,
    timeout: Duration,
) -> anyhow::Result<WorkspaceReadLock> {
    refuse_if_symlink(workspace, "workspace directory").await?;
    let workspace = workspace.canonicalize()
        .with_context(|| format!("resolve workspace {}", workspace.display()))?;
    let path = lock_path(&workspace);
    refuse_if_symlink(&path, "lock file").await?;
    tokio::task::spawn_blocking(move || -> anyhow::Result<WorkspaceReadLock> {
        let file = open_lock_with_backoff(&path, timeout, true)?;
        let guard = WorkspaceReadLock { file, workspace };
        crate::artifact::identity_repair::check_repair_journals(&guard.workspace, None)?;
        crate::db::migrate::check_legacy_semantic_rebuild(&guard.workspace)?;
        if !crate::constellation::facade::workspace_is_migrated(&guard.workspace) {
            let lance = guard.workspace.join("lance");
            match std::fs::symlink_metadata(&lance) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() =>
                    anyhow::bail!("legacy index root is not a non-symlink directory"),
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            let marker_path = crate::db::migrate::identity_v5_marker_path(&lance);
            match std::fs::symlink_metadata(&marker_path) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() =>
                    anyhow::bail!("legacy identity marker is not a non-symlink file"),
                Ok(_) => { crate::db::migrate::read_identity_v5_marker(&lance)?; }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(guard)
    }).await.context("read lock acquisition task panicked")?
}

/// Synchronous ordinary-writer admission, for synchronous CLI commands.
/// Like the async entrypoint, this refuses unfinished or damaged repairs.
pub fn acquire_workspace_lock_blocking(workspace: &Path) -> anyhow::Result<WorkspaceLock> {
    let meta = std::fs::symlink_metadata(workspace)
        .with_context(|| format!("inspect workspace {}", workspace.display()))?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        anyhow::bail!("workspace {} is not a non-symlink directory", workspace.display());
    }
    let workspace = workspace.canonicalize()?;
    let file = open_and_lock_with_backoff(&lock_path(&workspace), DEFAULT_LOCK_TIMEOUT)?;
    let guard = WorkspaceLock { file, workspace: Some(workspace.clone()), repair_binding: None };
    crate::artifact::identity_repair::check_repair_journals(&workspace, None)?;
    crate::db::migrate::check_legacy_semantic_rebuild(&workspace)?;
    Ok(guard)
}

#[derive(Clone, PartialEq, Eq)]
struct RepairBinding {
    workspace: PathBuf,
    transaction_id: String,
    journal_digest: String,
    legacy: bool,
    blocked_legacy: bool,
    linked_predecessor: Option<(String, String)>,
    semantic: bool,
}

/// Opaque admission pinned to a single journal and the held physical owner.
pub(crate) struct RepairIndexAdmission<'a> {
    owner: &'a crate::constellation::publisher::IndexPublisher,
    binding: RepairBinding,
}

impl RepairIndexAdmission<'_> {
    pub(crate) fn owner(&self) -> &crate::constellation::publisher::IndexPublisher {
        self.owner
    }

    pub(crate) fn workspace(&self) -> &Path {
        &self.binding.workspace
    }

    pub(crate) fn journal_digest(&self) -> &str {
        &self.binding.journal_digest
    }
}

#[cfg(not(unix))]
fn refuse_unpinned_non_unix_recovery() -> anyhow::Result<()> {
    anyhow::bail!("recovery is unavailable on non-Unix platforms until journal and index operations use pinned, no-reparse handles with durable namespace barriers")
}

pub(crate) async fn begin_repair_index_admission<'a>(
    workspace: &Path,
    transaction_id: &str,
    owner: &'a crate::constellation::publisher::IndexPublisher,
) -> anyhow::Result<RepairIndexAdmission<'a>> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_recovery()?;
    refuse_if_symlink(workspace, "workspace directory").await?;
    let workspace = workspace.canonicalize()
        .with_context(|| format!("resolve workspace {}", workspace.display()))?;
    if owner.index_root() != workspace.join("lance") {
        anyhow::bail!("repair index owner does not belong to the requested workspace");
    }
    let path = lock_path(&workspace);
    refuse_if_symlink(&path, "lock file").await?;
    let workspace_copy = workspace.clone();
    let id = transaction_id.to_string();
    let id_copy = id.clone();
    let digest = tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
        let file = open_and_lock_with_backoff(&path, DEFAULT_LOCK_TIMEOUT)?;
        let _guard = WorkspaceLock { file, workspace: Some(workspace_copy.clone()), repair_binding: None };
        crate::db::migrate::check_legacy_semantic_rebuild(&workspace_copy)?;
        crate::artifact::identity_repair::pending_repair_digest(&workspace_copy, &id_copy)
    }).await.context("repair admission task panicked")??;
    Ok(RepairIndexAdmission {
        owner,
        binding: RepairBinding { workspace, transaction_id: id, journal_digest: digest, legacy: false, blocked_legacy: false, linked_predecessor: None, semantic: false },
    })
}

pub(crate) async fn acquire_repair_workspace_lock(
    admission: &RepairIndexAdmission<'_>,
) -> anyhow::Result<WorkspaceLock> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_recovery()?;
    if admission.owner.index_root() != admission.binding.workspace.join("lance") {
        anyhow::bail!("repair index owner does not match admission");
    }
    let binding = admission.binding.clone();
    let path = lock_path(&binding.workspace);
    refuse_if_symlink(&path, "lock file").await?;
    tokio::task::spawn_blocking(move || -> anyhow::Result<WorkspaceLock> {
        let file = open_and_lock_with_backoff(&path, DEFAULT_LOCK_TIMEOUT)?;
        let guard = WorkspaceLock { file, workspace: Some(binding.workspace.clone()), repair_binding: Some(binding.clone()) };
        crate::db::migrate::check_legacy_semantic_rebuild(&binding.workspace)?;
        let digest = crate::artifact::identity_repair::pending_repair_digest(&binding.workspace, &binding.transaction_id)?;
        if digest != binding.journal_digest {
            anyhow::bail!("repair journal changed after admission");
        }
        Ok(guard)
    }).await.context("repair lock acquisition task panicked")?
}

/// Acquire the workspace lock for exactly one pending legacy-index repair.
/// This never admits an ordinary writer and never finishes the journal.
pub(crate) async fn acquire_legacy_repair_workspace_lock(
    workspace: &Path,
    transaction_id: &str,
) -> anyhow::Result<WorkspaceLock> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_recovery()?;
    refuse_if_symlink(workspace, "workspace directory").await?;
    let workspace = workspace.canonicalize()
        .with_context(|| format!("resolve workspace {}", workspace.display()))?;
    let path = lock_path(&workspace);
    refuse_if_symlink(&path, "lock file").await?;
    let transaction_id = transaction_id.to_owned();
    tokio::task::spawn_blocking(move || -> anyhow::Result<WorkspaceLock> {
        let file = open_and_lock_with_backoff(&path, DEFAULT_LOCK_TIMEOUT)?;
        let mut guard = WorkspaceLock { file, workspace: Some(workspace.clone()), repair_binding: None };
        let lance = workspace.join("lance");
        let metadata = std::fs::symlink_metadata(&lance)
            .with_context(|| format!("inspect legacy index {}", lance.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            anyhow::bail!("legacy index root is not a non-symlink directory");
        }
        if crate::constellation::facade::workspace_is_migrated(&workspace) {
            anyhow::bail!("workspace is owned by Constellation, not the legacy index");
        }
        let marker_path = crate::db::migrate::identity_v5_marker_path(&lance);
        match std::fs::symlink_metadata(&marker_path) {
            Ok(marker_meta) => {
                if marker_meta.file_type().is_symlink() || !marker_meta.is_file() {
                    anyhow::bail!("legacy identity marker is not a non-symlink file");
                }
                let marker = crate::db::migrate::read_identity_v5_marker(&lance)?
                    .ok_or_else(|| anyhow::anyhow!("legacy identity marker disappeared"))?;
                if marker.artifacts_table != crate::db::migrate::IDENTITY_V5_ARTIFACTS_TABLE
                    || marker.relations_table != crate::db::migrate::IDENTITY_V5_RELATIONS_TABLE
                    || marker.evidence_table != crate::db::migrate::IDENTITY_V5_EVIDENCE_TABLE
                {
                    anyhow::bail!("legacy identity marker names unexpected tables");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // Admission does not create a marker. The reconciler subsequently opens
                // and verifies these tables while holding this same journal-bound guard.
                for name in ["artifacts", "relations", "evidence"] {
                    let table = lance.join(format!("{name}.lance"));
                    let metadata = std::fs::symlink_metadata(&table)
                        .with_context(|| format!("inspect legacy v4 table {}", table.display()))?;
                    if metadata.file_type().is_symlink() || !metadata.is_dir() {
                        anyhow::bail!("legacy v4 table is not a non-symlink directory: {}", table.display());
                    }
                    crate::db::migrate::semantic_rebuild_tree_digest(&table)?;
                }
            }
            Err(error) => return Err(error).context("inspect legacy identity marker"),
        }
        crate::db::migrate::check_legacy_semantic_rebuild(&workspace)?;
        let journal_digest = crate::artifact::identity_repair::pending_repair_digest(
            &workspace, &transaction_id,
        )?;
        guard.repair_binding = Some(RepairBinding {
            workspace, transaction_id, journal_digest, legacy: true, blocked_legacy: false, linked_predecessor: None, semantic: false,
        });
        Ok(guard)
    }).await.context("legacy repair lock acquisition task panicked")?
}

/// Lock one blocked legacy transaction for an explicitly linked repair.
pub(crate) async fn acquire_blocked_legacy_workspace_lock(
    workspace: &Path,
    transaction_id: &str,
) -> anyhow::Result<WorkspaceLock> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_recovery()?;
    refuse_if_symlink(workspace, "workspace directory").await?;
    let workspace = workspace.canonicalize()
        .with_context(|| format!("resolve workspace {}", workspace.display()))?;
    let path = lock_path(&workspace);
    refuse_if_symlink(&path, "lock file").await?;
    let transaction_id = transaction_id.to_owned();
    tokio::task::spawn_blocking(move || -> anyhow::Result<WorkspaceLock> {
        let file = open_and_lock_with_backoff(&path, DEFAULT_LOCK_TIMEOUT)?;
        let mut guard = WorkspaceLock { file, workspace: Some(workspace.clone()), repair_binding: None };
        let lance = workspace.join("lance");
        let metadata = std::fs::symlink_metadata(&lance)
            .with_context(|| format!("inspect legacy index {}", lance.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir()
            || crate::constellation::facade::workspace_is_migrated(&workspace)
        {
            anyhow::bail!("blocked repair requires a legacy-owned, non-symlink index root");
        }
        crate::db::migrate::check_legacy_semantic_rebuild(&workspace)?;
        let journal_digest = crate::artifact::identity_repair::validate_blocked_repair_admission(
            &workspace, &transaction_id,
        )?;
        guard.repair_binding = Some(RepairBinding {
            workspace, transaction_id, journal_digest, legacy: true, blocked_legacy: true, linked_predecessor: None, semantic: false,
        });
        Ok(guard)
    }).await.context("blocked legacy repair lock acquisition task panicked")?
}

/// Lock an exact, durably linked blocked-and-successor legacy repair pair.
pub(crate) async fn acquire_linked_repair_workspace_lock(
    workspace: &Path,
    old_id: &str,
    new_id: &str,
) -> anyhow::Result<WorkspaceLock> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_recovery()?;
    refuse_if_symlink(workspace, "workspace directory").await?;
    let workspace = workspace.canonicalize()
        .with_context(|| format!("resolve workspace {}", workspace.display()))?;
    let path = lock_path(&workspace);
    refuse_if_symlink(&path, "lock file").await?;
    let old_id = old_id.to_owned();
    let new_id = new_id.to_owned();
    tokio::task::spawn_blocking(move || -> anyhow::Result<WorkspaceLock> {
        let file = open_and_lock_with_backoff(&path, DEFAULT_LOCK_TIMEOUT)?;
        let mut guard = WorkspaceLock { file, workspace: Some(workspace.clone()), repair_binding: None };
        let lance = workspace.join("lance");
        let metadata = std::fs::symlink_metadata(&lance)
            .with_context(|| format!("inspect legacy index {}", lance.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir()
            || crate::constellation::facade::workspace_is_migrated(&workspace)
        {
            anyhow::bail!("linked repair requires a legacy-owned, non-symlink index root");
        }
        crate::db::migrate::check_legacy_semantic_rebuild(&workspace)?;
        let (old_digest, journal_digest) =
            crate::artifact::identity_repair::validate_linked_repair_journals(&workspace, &old_id, &new_id)?;
        guard.repair_binding = Some(RepairBinding {
            workspace, transaction_id: new_id, journal_digest, legacy: true, blocked_legacy: false,
            linked_predecessor: Some((old_id, old_digest)),
            semantic: false,
        });
        Ok(guard)
    }).await.context("linked legacy repair lock acquisition task panicked")?
}

/// Acquire the exclusive workspace lock for exactly one semantic rebuild.
/// Ordinary readers and identity-repair admissions remain blocked.
pub(crate) async fn acquire_legacy_semantic_rebuild_lock(
    workspace: &Path,
    transaction_id: &str,
) -> anyhow::Result<WorkspaceLock> {
    #[cfg(not(unix))]
    refuse_unpinned_non_unix_recovery()?;
    refuse_if_symlink(workspace, "workspace directory").await?;
    let workspace = workspace.canonicalize()
        .with_context(|| format!("resolve workspace {}", workspace.display()))?;
    let path = lock_path(&workspace);
    refuse_if_symlink(&path, "lock file").await?;
    let transaction_id = transaction_id.to_owned();
    tokio::task::spawn_blocking(move || -> anyhow::Result<WorkspaceLock> {
        let file = open_and_lock_with_backoff(&path, DEFAULT_LOCK_TIMEOUT)?;
        let mut guard = WorkspaceLock { file, workspace: Some(workspace.clone()), repair_binding: None };
        crate::artifact::identity_repair::check_repair_journals(&workspace, None)?;
        let lance = workspace.join("lance");
        let metadata = std::fs::symlink_metadata(&lance)
            .with_context(|| format!("inspect legacy index {}", lance.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir()
            || crate::constellation::facade::workspace_is_migrated(&workspace)
        {
            anyhow::bail!("semantic rebuild requires a legacy-owned, non-symlink index root");
        }
        let journal_digest = crate::db::migrate::validate_legacy_semantic_rebuild(&workspace, &transaction_id)?;
        guard.repair_binding = Some(RepairBinding {
            workspace, transaction_id, journal_digest, legacy: true, blocked_legacy: false,
            linked_predecessor: None, semantic: true,
        });
        Ok(guard)
    }).await.context("legacy semantic rebuild lock acquisition task panicked")?
}

pub(crate) fn finish_repair_index_admission(
    admission: &RepairIndexAdmission<'_>,
    guard: &WorkspaceLock,
) -> anyhow::Result<()> {
    if guard.repair_binding.as_ref() != Some(&admission.binding) {
        anyhow::bail!("workspace guard does not belong to this repair admission");
    }
    crate::artifact::identity_repair::finish_pending_recovery(
        &admission.binding.workspace,
        &admission.binding.transaction_id,
        &admission.binding.journal_digest,
        guard,
    )
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
    let workspace = workspace.to_path_buf();
    let workspace = workspace.canonicalize()?;
    tokio::task::spawn_blocking(move || -> anyhow::Result<WorkspaceLock> {
        let file = open_and_lock_with_backoff(&path_clone, timeout)?;
        let guard = WorkspaceLock { file, workspace: Some(workspace.clone()), repair_binding: None };
        crate::artifact::identity_repair::check_repair_journals(&workspace, None)?;
        crate::db::migrate::check_legacy_semantic_rebuild(&workspace)?;
        Ok(guard)
    })
    .await
    .context("lock acquisition task panicked")?
}

/// Blocking helper: open the lock file and repeatedly `try_lock_exclusive`
/// with exponential backoff until the lock is held or the timeout elapses.
fn open_and_lock_with_backoff(path: &Path, timeout: Duration) -> anyhow::Result<File> {
    open_lock_with_backoff(path, timeout, false)
}

fn open_lock_with_backoff(path: &Path, timeout: Duration, shared: bool) -> anyhow::Result<File> {
    // Minimal OpenOptions: create if missing, write-only. Previously
    // had `.read(true).truncate(false)` — unnecessary (audit Round 1 L-1).
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let f = options
        .open(path)
        .with_context(|| format!("could not open lock file {}", path.display()))?;

    let start = std::time::Instant::now();
    let mut delay = Duration::from_millis(10);
    const MAX_DELAY: Duration = Duration::from_millis(1000);

    loop {
        match if shared { FileExt::try_lock_shared(&f) } else { f.try_lock_exclusive() } {
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

    #[cfg(not(unix))]
    #[tokio::test]
    async fn non_unix_scoped_recovery_refuses_before_opening_lockfile() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path();
        assert!(acquire_legacy_repair_workspace_lock(workspace, "txn").await.is_err());
        assert!(acquire_blocked_legacy_workspace_lock(workspace, "txn").await.is_err());
        assert!(acquire_linked_repair_workspace_lock(workspace, "old", "new").await.is_err());
        assert!(acquire_legacy_semantic_rebuild_lock(workspace, "txn").await.is_err());
        assert_eq!(std::fs::read_dir(workspace).unwrap().count(), 0);
    }

    fn ws(tmp: &TempDir) -> std::path::PathBuf {
        tmp.path().join(".forgeplan")
    }

    #[test]
    fn repair_admission_exposes_the_bound_manifest_digest() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().canonicalize().unwrap();
        let owner = crate::constellation::publisher::IndexPublisher::acquire(
            &workspace.join("lance"),
            std::sync::Arc::new(object_store::memory::InMemory::new()),
        ).unwrap();
        let admission = RepairIndexAdmission {
            owner: &owner,
            binding: RepairBinding {
                workspace,
                transaction_id: "repair-1".to_string(),
                journal_digest: "a".repeat(64),
                legacy: false,
                blocked_legacy: false,
                linked_predecessor: None,
                semantic: false,
            },
        };
        assert_eq!(admission.journal_digest(), "a".repeat(64));
    }

    #[test]
    fn repair_guard_provenance_rejects_other_workspace_transaction_and_raw_file() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().canonicalize().unwrap();
        let file = File::create(workspace.join("test.lock")).unwrap();
        let raw = WorkspaceLock::from_file(file);
        assert!(!raw.is_repair_guard_for(&workspace, "repair-1"));
        assert!(raw.matches_workspace(&workspace).is_err());
        drop(raw);
        let file = File::create(workspace.join("bound.lock")).unwrap();
        let bound = WorkspaceLock {
            file,
            workspace: Some(workspace.clone()),
            repair_binding: Some(RepairBinding {
                workspace: workspace.clone(),
                transaction_id: "repair-1".to_string(),
                journal_digest: "a".repeat(64),
                legacy: false,
                blocked_legacy: false,
                linked_predecessor: None,
                semantic: false,
            }),
        };
        assert!(bound.is_repair_guard_for(&workspace, "repair-1"));
        assert!(!bound.is_repair_guard_for(&workspace, "repair-2"));
        assert!(!bound.is_repair_guard_for(&workspace.join("other"), "repair-1"));
        assert!(!bound.is_legacy_repair_guard_for(&workspace, "repair-1"));
        assert!(bound.matches_workspace(&workspace).is_err());
        assert!(bound.legacy_repair_digest_for(&workspace, "repair-1").is_none());
    }

    #[test]
    fn legacy_guard_is_bound_to_its_workspace_and_transaction() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().canonicalize().unwrap();
        let file = File::create(workspace.join("legacy.lock")).unwrap();
        let guard = WorkspaceLock {
            file,
            workspace: Some(workspace.clone()),
            repair_binding: Some(RepairBinding {
                workspace: workspace.clone(),
                transaction_id: "repair-1".to_owned(),
                journal_digest: "a".repeat(64),
                legacy: true,
                blocked_legacy: false,
                linked_predecessor: None,
                semantic: false,
            }),
        };
        assert!(guard.is_legacy_repair_guard_for(&workspace, "repair-1"));
        assert_eq!(guard.legacy_repair_digest_for(&workspace, "repair-1"), Some("a".repeat(64).as_str()));
        assert!(guard.legacy_repair_digest_for(&workspace, "repair-2").is_none());
        assert!(!guard.is_legacy_repair_guard_for(&workspace, "repair-2"));
        assert!(guard.matches_workspace(&workspace).is_err());
        assert!(!guard.is_legacy_repair_guard_for(&workspace.join("elsewhere"), "repair-1"));
        let raw = WorkspaceLock::from_file(File::create(workspace.join("raw.lock")).unwrap());
        assert!(!raw.is_legacy_repair_guard_for(&workspace, "repair-1"));
        assert!(raw.legacy_repair_digest_for(&workspace, "repair-1").is_none());
    }

    #[test]
    fn blocked_legacy_guard_cannot_authorize_pending_or_ordinary_access() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().canonicalize().unwrap();
        let guard = WorkspaceLock {
            file: File::create(workspace.join("blocked.lock")).unwrap(),
            workspace: Some(workspace.clone()),
            repair_binding: Some(RepairBinding {
                workspace: workspace.clone(),
                transaction_id: "old".to_owned(),
                journal_digest: "digest".to_owned(),
                legacy: true,
                blocked_legacy: true,
                linked_predecessor: None,
                semantic: false,
            }),
        };
        assert!(guard.is_blocked_legacy_repair_guard_for(&workspace, "old"));
        assert_eq!(guard.blocked_legacy_repair_digest_for(&workspace, "old"), Some("digest"));
        assert!(!guard.is_blocked_legacy_repair_guard_for(&workspace, "other"));
        assert!(!guard.is_repair_guard_for(&workspace, "old"));
        assert!(!guard.is_legacy_repair_guard_for(&workspace, "old"));
        assert!(guard.legacy_repair_digest_for(&workspace, "old").is_none());
        assert!(guard.matches_workspace(&workspace).is_err());
    }

    #[test]
    fn linked_guard_binds_both_transactions_and_their_digests() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().canonicalize().unwrap();
        let guard = WorkspaceLock {
            file: File::create(workspace.join("linked.lock")).unwrap(),
            workspace: Some(workspace.clone()),
            repair_binding: Some(RepairBinding {
                workspace: workspace.clone(),
                transaction_id: "new".to_owned(),
                journal_digest: "new-digest".to_owned(),
                legacy: true,
                blocked_legacy: false,
                linked_predecessor: Some(("old".to_owned(), "old-digest".to_owned())),
                semantic: false,
            }),
        };
        assert_eq!(guard.linked_legacy_repair_digests_for(&workspace, "old", "new"), Some(("old-digest", "new-digest")));
        assert!(guard.linked_legacy_repair_digests_for(&workspace, "other", "new").is_none());
        assert!(guard.linked_legacy_repair_digests_for(&workspace, "old", "other").is_none());
        assert!(guard.is_legacy_repair_guard_for(&workspace, "new"));
        assert!(!guard.is_legacy_repair_guard_for(&workspace, "old"));
        assert!(!guard.is_blocked_legacy_repair_guard_for(&workspace, "old"));
        assert!(guard.matches_workspace(&workspace).is_err());
    }

    #[test]
    fn semantic_guard_requires_exact_workspace_transaction_and_digest() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().canonicalize().unwrap();
        let guard = WorkspaceLock {
            file: File::create(workspace.join("semantic.lock")).unwrap(),
            workspace: Some(workspace.clone()),
            repair_binding: Some(RepairBinding {
                workspace: workspace.clone(), transaction_id: "txn".to_owned(),
                journal_digest: "digest".to_owned(), legacy: true, blocked_legacy: false,
                linked_predecessor: None, semantic: true,
            }),
        };
        assert!(guard.is_legacy_semantic_rebuild_guard_for(&workspace, "txn", "digest"));
        assert!(!guard.is_legacy_semantic_rebuild_guard_for(&workspace, "other", "digest"));
        assert!(!guard.is_legacy_semantic_rebuild_guard_for(&workspace, "txn", "other"));
        assert!(!guard.is_legacy_semantic_rebuild_guard_for(&workspace.join("other"), "txn", "digest"));
        assert!(!guard.is_repair_guard_for(&workspace, "txn"));
        assert!(!guard.is_legacy_repair_guard_for(&workspace, "txn"));
        assert!(guard.matches_workspace(&workspace).is_err());
    }

    #[tokio::test]
    async fn damaged_semantic_journal_blocks_all_ordinary_admission() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().canonicalize().unwrap();
        let journal = workspace.join("lance/legacy-semantic-rebuild/txn");
        std::fs::create_dir_all(&journal).unwrap();
        std::fs::write(journal.join("manifest.json"), b"{damaged").unwrap();
        assert!(acquire_workspace_read_lock_with_timeout(&workspace, Duration::ZERO).await.is_err());
        assert!(acquire_workspace_lock_with_timeout(&workspace, Duration::ZERO).await.is_err());
        assert!(acquire_workspace_lock_blocking(&workspace).is_err());
        assert!(acquire_legacy_semantic_rebuild_lock(&workspace, "txn").await.is_err());
    }

    #[tokio::test]
    async fn shared_readers_exclude_writer_until_all_guards_drop() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().canonicalize().unwrap();
        let first = acquire_workspace_read_lock_with_timeout(&workspace, Duration::ZERO).await.unwrap();
        let second = acquire_workspace_read_lock_with_timeout(&workspace, Duration::ZERO).await.unwrap();
        assert!(first.matches_workspace(&workspace).is_ok());
        assert!(first.matches_workspace(&workspace.join("missing")).is_err());
        assert!(acquire_workspace_lock_with_timeout(&workspace, Duration::ZERO).await.is_err());
        drop(first);
        assert!(acquire_workspace_lock_with_timeout(&workspace, Duration::ZERO).await.is_err());
        drop(second);
        let writer = acquire_workspace_lock_with_timeout(&workspace, Duration::ZERO).await.unwrap();
        assert!(writer.matches_workspace(&workspace).is_ok());
        assert!(acquire_workspace_read_lock_with_timeout(&workspace, Duration::ZERO).await.is_err());
        drop(writer);
        assert!(acquire_workspace_read_lock_with_timeout(&workspace, Duration::ZERO).await.is_ok());
    }

    #[tokio::test]
    async fn shared_reader_refuses_damaged_journal_or_marker() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().canonicalize().unwrap();
        let lance = workspace.join("lance");
        std::fs::create_dir(&lance).unwrap();
        let marker = crate::db::migrate::identity_v5_marker_path(&lance);
        std::fs::write(&marker, b"{damaged").unwrap();
        assert!(acquire_workspace_read_lock(&workspace).await.is_err());
        std::fs::remove_file(marker).unwrap();
        let journal = workspace.join("identity-repair/repair-broken");
        std::fs::create_dir_all(&journal).unwrap();
        std::fs::write(journal.join("manifest.json"), b"{damaged").unwrap();
        assert!(acquire_workspace_read_lock(&workspace).await.is_err());
        assert!(acquire_workspace_lock_with_timeout(&workspace, Duration::ZERO).await.is_err());
    }

    #[tokio::test]
    async fn legacy_repair_refuses_missing_or_damaged_marker_and_damaged_journal() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().canonicalize().unwrap();
        let lance = workspace.join("lance");
        std::fs::create_dir(&lance).unwrap();
        assert!(acquire_legacy_repair_workspace_lock(&workspace, "repair-1").await.is_err());
        let marker_path = crate::db::migrate::identity_v5_marker_path(&lance);
        std::fs::write(&marker_path, b"{broken").unwrap();
        assert!(acquire_legacy_repair_workspace_lock(&workspace, "repair-1").await.is_err());
        let marker = crate::db::migrate::IdentityV5Marker {
            version: crate::db::migrate::IDENTITY_V5_VERSION,
            artifacts_table: crate::db::migrate::IDENTITY_V5_ARTIFACTS_TABLE.to_owned(),
            relations_table: crate::db::migrate::IDENTITY_V5_RELATIONS_TABLE.to_owned(),
            evidence_table: crate::db::migrate::IDENTITY_V5_EVIDENCE_TABLE.to_owned(),
            source_digest: "a".repeat(64),
            document_count: 0,
            completed: true,
            embeddings: None,
            semantic: None,
        };
        std::fs::write(&marker_path, serde_json::to_vec(&marker).unwrap()).unwrap();
        let journal = workspace.join("identity-repair/repair-1");
        std::fs::create_dir_all(&journal).unwrap();
        std::fs::write(journal.join("manifest.json"), b"{broken").unwrap();
        assert!(acquire_legacy_repair_workspace_lock(&workspace, "repair-1").await.is_err());
        assert!(acquire_workspace_lock(&workspace).await.is_err());
    }

    #[tokio::test]
    async fn acquire_creates_lock_file() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let _guard = acquire_workspace_lock(&ws).await.unwrap();
        assert!(lock_path(&ws).exists(), "lock file must be created");
    }

    #[tokio::test]
    async fn damaged_and_missing_manifests_block_normal_writers() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let journal = ws.join("identity-repair/repair-broken");
        std::fs::create_dir_all(&journal).unwrap();
        assert!(acquire_workspace_lock(&ws).await.is_err());
        std::fs::write(journal.join("manifest.json"), b"{invalid").unwrap();
        assert!(acquire_workspace_lock(&ws).await.is_err());
    }

    #[tokio::test]
    async fn symlinked_journal_root_blocks_normal_writers() {
        #[cfg(unix)]
        {
            let tmp = TempDir::new().unwrap();
            let ws = ws(&tmp);
            std::fs::create_dir_all(&ws).unwrap();
            std::os::unix::fs::symlink(tmp.path(), ws.join("identity-repair")).unwrap();
            assert!(acquire_workspace_lock(&ws).await.is_err());
        }
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

    // ── PRD-078 Phase 3: multi-worktree lock isolation (PROB-067) ──────

    /// PROB-067 surface: concurrent writes to DIFFERENT worktrees must use
    /// INDEPENDENT lock files and therefore NOT serialise each other.
    ///
    /// Two tasks each hold the lock for their respective workspace for
    /// `hold_ms` milliseconds and run concurrently. If the locks were shared
    /// (same lock file), the total elapsed time would be ≥ 2×hold_ms (serial).
    /// With independent locks the elapsed time should be close to 1×hold_ms
    /// (parallel). We use a generous upper bound (2×hold_ms – 20ms slack)
    /// to avoid flakiness on busy CI runners.
    #[tokio::test]
    async fn concurrent_writes_to_different_worktrees_use_independent_locks() {
        let tmp1 = TempDir::new().unwrap();
        let tmp2 = TempDir::new().unwrap();
        let ws1 = tmp1.path().join(".forgeplan");
        let ws2 = tmp2.path().join(".forgeplan");

        let hold_ms = 60u64;

        let ws1_c = ws1.clone();
        let ws2_c = ws2.clone();

        let started = tokio::time::Instant::now();

        // Spawn both tasks simultaneously — each acquires its own workspace lock.
        let t1 = tokio::spawn(async move {
            let _g = acquire_workspace_lock(&ws1_c).await.unwrap();
            tokio::time::sleep(Duration::from_millis(hold_ms)).await;
        });
        let t2 = tokio::spawn(async move {
            let _g = acquire_workspace_lock(&ws2_c).await.unwrap();
            tokio::time::sleep(Duration::from_millis(hold_ms)).await;
        });

        t1.await.unwrap();
        t2.await.unwrap();

        let elapsed = started.elapsed();

        // If the two locks were independent (correct behaviour), both tasks ran
        // in parallel, so total time ≈ hold_ms — well under 2×hold_ms.
        // If they were accidentally serialised (regression), total ≥ 2×hold_ms.
        let serial_lower_bound = Duration::from_millis(hold_ms * 2 - 20);
        assert!(
            elapsed < serial_lower_bound,
            "independent worktree locks must run in parallel; \
             elapsed {elapsed:?} ≥ serial lower bound {serial_lower_bound:?} \
             — locks are being shared (PROB-067 regression)"
        );

        // Also confirm the lock files are at different paths.
        assert_eq!(lock_path(&ws1), ws1.join(".lock"));
        assert_eq!(lock_path(&ws2), ws2.join(".lock"));
        assert_ne!(lock_path(&ws1), lock_path(&ws2));
    }

    /// Regression: same workspace still serialises writes (the existing
    /// `concurrent_acquirers_serialize_and_total_time` test covers this but
    /// we add an explicit named test that documents the PROB-067 invariant).
    #[tokio::test]
    async fn same_workspace_still_serialises() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);

        let hold_ms = 25u64;
        let n = 3usize;
        let in_flight = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let max_in_flight = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let started = tokio::time::Instant::now();
        let mut handles = Vec::new();
        for _ in 0..n {
            let ws = ws.clone();
            let in_flight = in_flight.clone();
            let max_in_flight = max_in_flight.clone();
            handles.push(tokio::spawn(async move {
                let _g = acquire_workspace_lock(&ws).await.unwrap();
                let current = in_flight.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                max_in_flight.fetch_max(current, std::sync::atomic::Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(hold_ms)).await;
                in_flight.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        let elapsed = started.elapsed();

        assert_eq!(
            max_in_flight.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "same workspace: at most 1 concurrent holder"
        );
        let serial_lb = Duration::from_millis(hold_ms * n as u64 - 10);
        assert!(
            elapsed >= serial_lb,
            "same workspace: elapsed {elapsed:?} < serial lb {serial_lb:?} — serialization broken"
        );
    }
}
