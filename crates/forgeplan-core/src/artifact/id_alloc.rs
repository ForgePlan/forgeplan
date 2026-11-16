// PROB-067 — Race-safe ID allocator for `forgeplan_new`.
//
// Problem: parallel `forgeplan_new evidence` invocations across separate
// git worktrees sharing the same logical workspace each compute `max(NNN)+1`
// independently and write to disk, producing duplicate `EVID-NNN` numbers
// or silent body overwrites of pre-existing siblings.
//
// Existing `WorkspaceLock` (`crates/forgeplan-core/src/workspace/lock.rs`)
// guards the local `.forgeplan/.lock` file — sufficient for parallel CLI
// invocations against the SAME working tree, but each worktree has its own
// `.forgeplan/.lock`, so cross-worktree races are not serialized.
//
// Fix combo (Option B + Option D from PROB-067):
//
// 1. **Cross-worktree lock** — when running inside a git worktree, the
//    `.git` directory is per-worktree but the *common* git dir is shared
//    across all worktrees. We place a `forgeplan-id-<kind>.lock` file
//    inside `git-common-dir`, which gives us a system-wide rendez-vous
//    point across all worktrees of the same repo. Falls back gracefully
//    to a workspace-local lock when no git common-dir is available (fresh
//    clone before first commit, non-git directories, CI sandboxes).
//
// 2. **Per-kind granularity** — different artifact kinds do not block
//    each other (parallel `new prd` + `new evidence` is fine).
//
// 3. **Canonical-key publication** — mint the key once before reserving a
//    display number, then publish `<canonical_key>.md` with create-only
//    semantics. A key collision refuses without overwriting or reminting.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;

use crate::artifact::frontmatter::parse_frontmatter;
use crate::artifact::types::ArtifactKind;
use crate::db::store::{LanceStore, NewArtifact};
use crate::projection::{self, MutationContext};
use crate::workspace::git::{git_common_dir, git_show_toplevel};
use crate::workspace::lock::{DEFAULT_LOCK_TIMEOUT, WorkspaceLock, WorkspaceReadPermit};

/// Timeout for the cross-worktree id-allocation lock. Inherits the
/// workspace-lock default (30 s) — keeps consistency with the rest of
/// the locking surface.
const ID_ALLOC_LOCK_TIMEOUT: Duration = DEFAULT_LOCK_TIMEOUT;

/// Locate the shared id-allocation lock path. Prefer `git-common-dir`
/// (shared across worktrees); fall back to the workspace `.lock` file.
///
/// `git-common-dir` differs from the per-worktree `.git/` for linked
/// worktrees: `git rev-parse --git-common-dir` returns the main repo's
/// `.git`. Placing the lock there guarantees all `forgeplan_new`
/// invocations in any worktree of the same repo see the same file.
fn id_alloc_lock_path(workspace: &Path, kind: &ArtifactKind) -> PathBuf {
    if let Some(common_dir) = git_common_dir_from_workspace(workspace) {
        let dir = common_dir.join("forgeplan");
        // best-effort: created lazily by `acquire_workspace_lock_with_timeout`
        // via `create_dir_all` on the parent before open.
        return dir.join(format!("id-{}.lock", kind.prefix().trim_end_matches('-')));
    }
    // Fallback: workspace-local. This is the existing race surface but
    // strictly no worse than today's behavior; the shared lock still
    // serializes reservations within this workspace.
    workspace.join(format!(
        ".id-alloc-{}.lock",
        kind.prefix().trim_end_matches('-')
    ))
}

/// Thin wrapper: resolve `git-common-dir` from a *workspace* path (i.e.
/// `<repo>/.forgeplan`). The underlying implementation lives in
/// [`crate::workspace::git::git_common_dir`] and operates on an arbitrary
/// `cwd`; here we derive that `cwd` from the workspace path as the id-alloc
/// convention has always required.
///
/// Factored out to `workspace::git` so the multi-worktree detector
/// ([`crate::workspace::init::detect_multi_worktree`]) can share the same
/// subprocess logic without duplicating it (PRD-078 Phase 2 / W2).
///
/// The answer is the **physical** spelling of the common directory. Git
/// reports it relative to the caller's directory in the main worktree and
/// absolute in a linked one, and both spellings depend on how the caller
/// reached the checkout — a symlinked checkout path and the real path name
/// the same directory under two different strings. Two worktrees of one
/// clone must agree on **one** lock file and **one** ledger, so the path is
/// resolved to the directory it really is before it is used.
fn git_common_dir_from_workspace(workspace: &Path) -> Option<PathBuf> {
    // Workspace is `<repo>/.forgeplan` — git ops should be rooted at
    // its parent (the actual working tree). Fall back to workspace
    // itself if it has no parent (shouldn't happen for valid workspaces).
    let cwd = workspace.parent().unwrap_or(workspace);
    let common_dir = git_common_dir(cwd)?;
    Some(canonical_shared_root(common_dir))
}

/// The physical spelling of `path`: symlinks resolved, so two spellings of
/// one directory name one file.
///
/// `std::fs::canonicalize` answers with a verbatim (`\\?\`) path on Windows,
/// which the platform's own directory helpers reject when it carries a
/// trailing separator, so the prefix is dropped again and the ordinary
/// `C:\...` spelling is kept.
fn canonical_shared_root(path: PathBuf) -> PathBuf {
    let Ok(canonical) = std::fs::canonicalize(&path) else {
        return path;
    };
    #[cfg(windows)]
    {
        let text = canonical.to_string_lossy().to_string();
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    canonical
}

/// Acquire the cross-worktree id-allocation lock for `kind`. The lock
/// file lives in `git-common-dir/forgeplan/id-<kind>.lock` when inside
/// a git repo, else in `<workspace>/.id-alloc-<kind>.lock`.
///
/// Reuses the symlink-guarded `acquire_workspace_lock_with_timeout`
/// machinery so security parity with the workspace lock is preserved.
pub async fn acquire_id_alloc_lock(
    workspace: &Path,
    kind: &ArtifactKind,
) -> anyhow::Result<WorkspaceLock> {
    let lock_path = id_alloc_lock_path(workspace, kind);
    let parent = lock_path
        .parent()
        .with_context(|| format!("id-alloc lock path has no parent: {}", lock_path.display()))?;
    // `acquire_workspace_lock_with_timeout` already calls `create_dir_all`
    // on the workspace dir it receives, and treats the lock file path as
    // `<workspace>/.lock`. To reuse it without forking, we pass the
    // parent directory of the chosen lock path, and rely on the fact
    // that for our lock the file name happens to differ from `.lock`.
    //
    // Important: the existing helper hardcodes `lock_path(workspace) =
    // workspace.join(".lock")`. We bypass this by inlining the same
    // open-and-lock primitive locally — keeps the symlink guards and
    // backoff identical.
    open_and_lock(parent, &lock_path, ID_ALLOC_LOCK_TIMEOUT).await
}

/// Cross-worktree allocator locks held continuously across a batch preflight,
/// index migration and publication. Locks are acquired in prefix order.
pub struct AllocationGuard {
    workspace: PathBuf,
    locks: Vec<(String, WorkspaceLock)>,
}

impl AllocationGuard {
    fn require(&self, workspace: &Path, kind: &ArtifactKind) -> anyhow::Result<()> {
        if self.workspace != workspace
            || !self.locks.iter().any(|(prefix, _)| prefix == kind.prefix())
        {
            anyhow::bail!(
                "allocator lock for {} is not held for this workspace",
                kind.prefix()
            );
        }
        Ok(())
    }
}

/// Acquire all distinct per-kind locks after the caller's workspace lock.
pub async fn acquire_allocation_guard(
    workspace: &Path,
    _workspace_guard: &WorkspaceLock,
    kinds: &[ArtifactKind],
) -> anyhow::Result<AllocationGuard> {
    let mut ordered = std::collections::BTreeMap::new();
    for kind in kinds {
        ordered.entry(kind.prefix().to_string()).or_insert(kind);
    }
    let mut locks = Vec::with_capacity(ordered.len());
    for (prefix, kind) in ordered {
        locks.push((prefix, acquire_id_alloc_lock(workspace, kind).await?));
    }
    Ok(AllocationGuard {
        workspace: workspace.to_path_buf(),
        locks,
    })
}

/// Open a lock file at a custom path and hold an exclusive flock with
/// the same backoff + symlink guards as `acquire_workspace_lock`. This
/// is essentially `acquire_workspace_lock_with_timeout` parameterised
/// on the lock-file path (the upstream helper hard-codes `.lock`).
async fn open_and_lock(
    parent: &Path,
    lock_path: &Path,
    timeout: Duration,
) -> anyhow::Result<WorkspaceLock> {
    use fs2::FileExt;
    use std::fs::OpenOptions;

    // Symlink guard on parent dir (parity with workspace lock C-sec #1).
    refuse_if_symlink(parent, "id-alloc lock directory").await?;
    tokio::fs::create_dir_all(parent)
        .await
        .with_context(|| format!("could not create id-alloc lock dir {}", parent.display()))?;

    // Symlink guard on lock file (parity).
    refuse_if_symlink(lock_path, "id-alloc lock file").await?;

    let path_clone = lock_path.to_path_buf();
    let file = tokio::task::spawn_blocking(move || -> anyhow::Result<std::fs::File> {
        let f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path_clone)
            .with_context(|| format!("could not open id-alloc lock {}", path_clone.display()))?;
        let start = std::time::Instant::now();
        let mut delay = Duration::from_millis(10);
        const MAX_DELAY: Duration = Duration::from_millis(1000);
        loop {
            match f.try_lock_exclusive() {
                Ok(()) => return Ok(f),
                Err(e) => {
                    let transient = e.kind() == std::io::ErrorKind::WouldBlock
                        || e.raw_os_error()
                            .zip(fs2::lock_contended_error().raw_os_error())
                            .is_some_and(|(actual, expected)| actual == expected);
                    if !transient {
                        return Err(anyhow::anyhow!(
                            "could not acquire id-alloc lock on {}: {e}",
                            path_clone.display()
                        ));
                    }
                    if start.elapsed() >= timeout {
                        anyhow::bail!(
                            "id-alloc lock {} held by another worker; timed out after {:?}",
                            path_clone.display(),
                            timeout
                        );
                    }
                    std::thread::sleep(delay);
                    delay = std::cmp::min(delay * 2, MAX_DELAY);
                }
            }
        }
    })
    .await
    .context("id-alloc lock acquisition task panicked")??;

    // Re-use the public WorkspaceLock RAII type — same Drop semantics.
    Ok(WorkspaceLock::from_file(file))
}

async fn refuse_if_symlink(path: &Path, label: &str) -> anyhow::Result<()> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                anyhow::bail!(
                    "{label} at {} is a symlink — refusing to open",
                    path.display()
                );
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

// ---------------------------------------------------------------------------
// Shared per-kind allocation ledger (SPEC-009 "Shared number allocation")
// ---------------------------------------------------------------------------

/// Schema version of the per-kind allocation ledger.
pub const LEDGER_SCHEMA_VERSION: u32 = 1;

/// Per-kind allocation ledger, stored beside the allocator lock.
///
/// Local bookkeeping only: never a tracked file, never a global key.
///
/// One file per artifact kind, stored beside the per-kind allocator lock:
/// `<git-common-dir>/forgeplan/id-<KIND>.json` for a git project, or
/// `<workspace>/id-<KIND>.json` for a project without git. The ledger
/// records the highest display number already handed out for the kind, so
/// a crash between the reservation and the file write leaves a gap instead
/// of a repeated number.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdLedger {
    /// Schema version of this file. A version the allocator does not know
    /// is an error, never a silently ignored field.
    pub schema_version: u32,
    /// Highest display number handed out for the kind so far.
    pub high_watermark: u32,
}

impl IdLedger {
    /// A ledger recording `high_watermark` at the current schema version.
    pub fn new(high_watermark: u32) -> Self {
        Self {
            schema_version: LEDGER_SCHEMA_VERSION,
            high_watermark,
        }
    }

    /// Serialize to the on-disk JSON form.
    pub fn to_json(&self) -> String {
        format!(
            "{{\n  \"schema_version\": {},\n  \"high_watermark\": {}\n}}\n",
            self.schema_version, self.high_watermark
        )
    }

    /// Parse the on-disk JSON form. A damaged file is an error — never a
    /// silently skipped maximum.
    pub fn from_json(text: &str) -> anyhow::Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(text).context("allocation ledger is not valid JSON")?;
        let object = value
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("allocation ledger is not a JSON object"))?;
        let schema_version = object
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("allocation ledger has no numeric 'schema_version'"))?;
        if schema_version != u64::from(LEDGER_SCHEMA_VERSION) {
            anyhow::bail!(
                "allocation ledger schema version {schema_version} is not supported \
                 (expected {LEDGER_SCHEMA_VERSION})"
            );
        }
        let high_watermark = object
            .get("high_watermark")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("allocation ledger has no numeric 'high_watermark'"))?;
        let high_watermark = u32::try_from(high_watermark).map_err(|_| {
            anyhow::anyhow!("allocation ledger high_watermark {high_watermark} does not fit in u32")
        })?;
        Ok(Self {
            schema_version: LEDGER_SCHEMA_VERSION,
            high_watermark,
        })
    }
}

/// Path of the per-kind ledger file — beside the per-kind allocator lock.
///
/// Git project: `<git-common-dir>/forgeplan/id-<KIND>.json` (inside the
/// common dir, so it is shared by every worktree and never tracked).
/// Project without git: `<workspace>/id-<KIND>.json`, beside the existing
/// local allocator lock.
pub fn id_ledger_path(workspace: &Path, kind: &ArtifactKind) -> PathBuf {
    let lock_path = id_alloc_lock_path(workspace, kind);
    let dir = lock_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| workspace.to_path_buf());
    dir.join(format!(
        "id-{}.json",
        kind.prefix().trim_end_matches('-').to_uppercase()
    ))
}

/// Load the per-kind ledger.
///
/// `Ok(None)` when no ledger exists yet — a documented recovery: the
/// watermark starts at zero and the tree scan provides the real maximum.
/// A damaged or unreadable ledger is an **error**: skipping it would hand
/// out a number that is already in use.
pub async fn load_id_ledger(
    workspace: &Path,
    kind: &ArtifactKind,
) -> anyhow::Result<Option<IdLedger>> {
    read_ledger_file(&id_ledger_path(workspace, kind)).await
}

/// Read the ledger at an already-resolved path.
///
/// `Ok(None)` when no ledger exists — the documented recovery for a missing
/// ledger. A damaged or unreadable file is an **error**: skipping it would
/// hand out a number that is already in use.
async fn read_ledger_file(path: &Path) -> anyhow::Result<Option<IdLedger>> {
    let bytes = match tokio::fs::read(path).await {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(anyhow::anyhow!(
                "could not read allocation ledger {}: {e}",
                path.display()
            ));
        }
    };
    let text = String::from_utf8(bytes).map_err(|e| {
        anyhow::anyhow!(
            "allocation ledger {} is not valid UTF-8: {e}",
            path.display()
        )
    })?;
    let ledger = IdLedger::from_json(&text)
        .with_context(|| format!("damaged allocation ledger {}", path.display()))?;
    Ok(Some(ledger))
}

/// Persist the per-kind ledger durably.
///
/// The publication barrier is the platform's own, because the two platforms
/// do not have the same one:
///
/// * **POSIX** — a flushed temporary sibling, an atomic rename over the old
///   ledger, and a flush of the containing directory. The rename only
///   becomes durable once the directory entry is flushed, so the directory
///   flush is part of the barrier, not a nicety.
/// * **Windows** — there is no directory fsync: `File::open` on a
///   directory is refused by the platform, and flushing a directory handle
///   would return success without any documented durability meaning. The
///   platform's real barrier for a name that becomes visible is a
///   *write-through move*, which the crate's NTFS adapter already owns (a
///   flushed same-directory temporary, then
///   `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`).
///   The allocator reuses that adapter instead of borrowing the POSIX
///   shape: dropping the directory flush would silently weaken the
///   guarantee, and pretending a flush happened would be a false ACK. A
///   volume that cannot supply the barrier is refused, loudly, rather
///   than acknowledged without proof.
///
/// Either way the ledger is published *before* the document is created, so
/// a failure between the two leaves a gap rather than a repeated number.
///
/// The watermark is the shared record of every number this kind ever handed
/// out, so it may only move **forward**: publishing a lower value would let
/// the same number be handed out twice. The value already on disk is
/// therefore the floor, whatever the caller asks for.
///
/// A damaged ledger is an **error**, never something to overwrite. Its true
/// watermark is unknown, and replacing it would silently reuse numbers that
/// were already handed out; the operator has to remove it deliberately,
/// which is the documented recovery for a missing ledger.
pub async fn save_id_ledger(
    workspace: &Path,
    kind: &ArtifactKind,
    ledger: &IdLedger,
) -> anyhow::Result<()> {
    let path = id_ledger_path(workspace, kind);
    let dir = path
        .parent()
        .with_context(|| format!("allocation ledger path has no parent: {}", path.display()))?
        .to_path_buf();
    let effective = match read_ledger_file(&path).await? {
        Some(current) if current.high_watermark > ledger.high_watermark => {
            anyhow::bail!(
                "refusing to lower the allocation watermark of {} from {} to {} — \
                 the higher number was already handed out",
                path.display(),
                current.high_watermark,
                ledger.high_watermark
            );
        }
        _ => ledger.clone(),
    };
    let bytes = effective.to_json().into_bytes();
    publish_ledger(kind, &dir, &path, &bytes).await
}

/// Publish the ledger bytes at `path` with the platform's durability barrier.
///
/// The old ledger is replaced atomically: a reader sees either the previous
/// watermark or the new one, never a half-written file, and the bytes of a
/// crash that happens mid-publication are discarded rather than published.
#[cfg(not(windows))]
async fn publish_ledger(
    kind: &ArtifactKind,
    dir: &Path,
    path: &Path,
    bytes: &[u8],
) -> anyhow::Result<()> {
    tokio::fs::create_dir_all(dir)
        .await
        .with_context(|| format!("could not create allocation ledger dir {}", dir.display()))?;
    let tmp_path = dir.join(format!(
        ".id-{}.json.tmp-{}-{}",
        kind.prefix().trim_end_matches('-').to_uppercase(),
        std::process::id(),
        LEDGER_TEMP_NONCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    ));
    let tmp_for_write = tmp_path.clone();
    let bytes_for_write = bytes.to_vec();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp_for_write)?;
        file.write_all(&bytes_for_write)?;
        file.sync_all()?;
        Ok(())
    })
    .await
    .context("allocation ledger write task panicked")??;
    tokio::fs::rename(&tmp_path, path)
        .await
        .with_context(|| format!("could not publish allocation ledger {}", path.display()))?;
    let dir_for_fsync = dir.to_path_buf();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let handle = std::fs::File::open(&dir_for_fsync)?;
        handle.sync_all()?;
        Ok(())
    })
    .await
    .context("allocation ledger directory fsync task panicked")??;
    Ok(())
}

/// Windows publication: the crate's NTFS write-through move.
///
/// The coordination directory is created write-through first, so its
/// namespace entry is durable before the ledger lands in it.
#[cfg(windows)]
async fn publish_ledger(
    _kind: &ArtifactKind,
    dir: &Path,
    path: &Path,
    bytes: &[u8],
) -> anyhow::Result<()> {
    let dir_for_task = dir.to_path_buf();
    let path_for_task = path.to_path_buf();
    let bytes_for_task = bytes.to_vec();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        crate::constellation::durability::ensure_dir(&dir_for_task).map_err(|error| {
            anyhow::anyhow!(
                "could not create allocation ledger dir {}: {error}",
                dir_for_task.display()
            )
        })?;
        crate::constellation::replace_mutable_file(&path_for_task, &bytes_for_task).map_err(
            |error| {
                anyhow::anyhow!(
                    "could not publish allocation ledger {}: {error}",
                    path_for_task.display()
                )
            },
        )?;
        Ok(())
    })
    .await
    .context("allocation ledger publication task panicked")??;
    Ok(())
}

/// Monotonic counter that keeps two concurrent publications of the same kind
/// from sharing one temporary name inside the shared allocator directory.
#[cfg(not(windows))]
static LEDGER_TEMP_NONCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The highest display number already used for `kind`, across every source
/// SPEC-009 names:
///
/// - the per-kind ledger watermark;
/// - the canonical frontmatter of the current tree;
/// - the frontmatter of every already-initialized linked worktree of the
///   same git common directory (discovered with
///   `git worktree list --porcelain -z`);
/// - the admissible legacy index (the LanceDB id sequence).
///
/// A missing `.forgeplan` in another worktree is admissible — that worktree
/// holds no numbers. An existing but unreadable workspace is an error, never
/// a silently skipped maximum.
pub async fn compute_max_display_number(
    workspace: &Path,
    permit: &dyn WorkspaceReadPermit,
    store: Option<&LanceStore>,
    kind: &ArtifactKind,
) -> anyhow::Result<u32> {
    permit.matches_workspace(workspace)?;
    let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
    let mut max_num: u32 = 0;

    if let Some(ledger) = load_id_ledger(workspace, kind).await? {
        max_num = max_num.max(ledger.high_watermark);
    }

    max_num = max_num.max(max_number_in_tree(&workspace.join(kind.dir_name()), kind).await?);

    for root in linked_worktree_roots(workspace).await? {
        let other_workspace = root.join(".forgeplan");
        match tokio::fs::metadata(&other_workspace).await {
            Ok(_) => {
                max_num = max_num
                    .max(max_number_in_tree(&other_workspace.join(kind.dir_name()), kind).await?);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "could not inspect linked worktree workspace {}: {e}",
                    other_workspace.display()
                ));
            }
        }
    }

    if let Some(s) = store {
        let lance_id = s.next_id(permit, &prefix).await?;
        // `next_id` returns the *next* value (max + 1); convert back to
        // the current max for comparison.
        if let Some(num_str) = lance_id.rsplit('-').next()
            && let Ok(next) = num_str.parse::<u32>()
        {
            max_num = max_num.max(next.saturating_sub(1));
        }
    }

    Ok(max_num)
}

/// The highest display number visible in one tree — from the file names
/// (`<KIND>-NNN-*.md`) and from the documents' canonical frontmatter
/// (`id`, `assigned_number`, `predicted_number`).
///
/// The frontmatter scan is what keeps the maximum honest once documents
/// are published under their canonical key instead of a numbered file name.
async fn max_number_in_tree(dir: &Path, kind: &ArtifactKind) -> anyhow::Result<u32> {
    let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
    let mut max_num: u32 = 0;
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let mut entries = match tokio::fs::read_dir(&current).await {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(anyhow::anyhow!("could not read {}: {e}", current.display()));
            }
        };
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let meta = match tokio::fs::metadata(&path).await {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => {
                    return Err(anyhow::anyhow!("could not stat {}: {e}", path.display()));
                }
            };
            if meta.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(num) = number_from_file_name(&name, &prefix) {
                max_num = max_num.max(num);
            }
            let content = tokio::fs::read_to_string(&path).await.with_context(|| {
                format!("could not read {} for number accounting", path.display())
            })?;
            let num = number_from_frontmatter(&content, &prefix).with_context(|| {
                format!("could not account for frontmatter in {}", path.display())
            })?;
            if let Some(num) = num {
                max_num = max_num.max(num);
            }
        }
    }
    Ok(max_num)
}

/// The number encoded in a legacy file name (`PRD-074-title.md`).
///
/// Only the legacy projection shape carries a display number, and it is
/// matched **case-sensitively**: the legacy writer emitted the upper-case
/// display id (`PRD-074-title.md`).
///
/// The canonical-key file name (`prd-2024-review-k<32 hex>.md`) is
/// lower-case and its title part may begin with digits. Upper-casing the
/// whole name before matching made `2024` look like a display number, which
/// pushed every later allocation of that kind past 2024 — a silent
/// inflation of the shared watermark. A canonical document's number lives in
/// its frontmatter, which is scanned separately, so nothing is lost by
/// leaving canonical file names out of this heuristic.
fn number_from_file_name(name: &str, prefix: &str) -> Option<u32> {
    let stem = name.strip_suffix(".md").unwrap_or(name);
    let rest = stem.strip_prefix(&format!("{prefix}-"))?;
    let num_str = rest.split('-').next()?;
    num_str.parse::<u32>().ok()
}

/// The number a document's canonical frontmatter claims.
fn number_from_frontmatter(content: &str, prefix: &str) -> anyhow::Result<Option<u32>> {
    let (fm, _body) = parse_frontmatter(content)?;
    let mut max_num: Option<u32> = None;
    let mut take = |n: u32| {
        max_num = Some(match max_num {
            Some(current) => current.max(n),
            None => n,
        });
    };

    for field in ["assigned_number", "predicted_number"] {
        if let Some(value) = fm.get(field)
            && !value.is_null()
        {
            let number = value
                .as_u64()
                .and_then(|number| u32::try_from(number).ok())
                .ok_or_else(|| anyhow::anyhow!("{field} must be a u32 integer"))?;
            take(number);
        }
    }

    if let Some(value) = fm.get("id")
        && !value.is_null()
    {
        let id = value
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("id must be a string"))?;
        let upper = id.to_uppercase();
        if let Some(rest) = upper.strip_prefix(&format!("{prefix}-")) {
            let number = rest
                .parse::<u32>()
                .map_err(|_| anyhow::anyhow!("id {id:?} has a malformed or overflowing number"))?;
            take(number);
        }
    }
    Ok(max_num)
}

/// Convert a Git path payload without changing its representation.
///
/// POSIX paths are arbitrary non-NUL byte strings. Other platforms can only
/// represent paths that Git emitted as valid UTF-8 through this interface.
#[cfg(unix)]
fn path_from_git_bytes(bytes: &[u8]) -> anyhow::Result<PathBuf> {
    use std::os::unix::ffi::OsStrExt;

    if bytes.is_empty() {
        anyhow::bail!("git worktree list returned an empty worktree path");
    }
    Ok(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
}

#[cfg(not(unix))]
fn path_from_git_bytes(bytes: &[u8]) -> anyhow::Result<PathBuf> {
    let path = std::str::from_utf8(bytes)
        .context("git worktree path is not representable as UTF-8 on this platform")?;
    if path.is_empty() {
        anyhow::bail!("git worktree list returned an empty worktree path");
    }
    Ok(PathBuf::from(path))
}

/// Parse NUL-delimited records from `git worktree list --porcelain -z`.
fn parse_linked_worktree_roots(output: &[u8]) -> anyhow::Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    for record in output.split(|byte| *byte == 0) {
        if let Some(path) = record.strip_prefix(b"worktree ") {
            let path = path_from_git_bytes(path)?;
            if !roots.contains(&path) {
                roots.push(path);
            }
        }
    }
    Ok(roots)
}

/// The roots of every worktree that shares the git common directory of
/// `workspace`.
///
/// Discovered with `git worktree list --porcelain -z` — never by scanning
/// neighbouring directories, which would pick up unrelated checkouts.
///
/// The command runs in the **current worktree**, not next to the git
/// directory. Those are different places whenever the git directory is not
/// `<repo>/.git`: a submodule's git directory lives under the superproject's
/// `.git/modules/<name>`, and `--separate-git-dir` puts it anywhere at all.
/// Deriving the root from the git directory's parent then either fails
/// outright or — worse — silently enumerates the *superproject's* worktrees,
/// which are not this repository's at all. `git worktree list` run inside a
/// worktree always enumerates the worktrees of the repository that worktree
/// belongs to, which is exactly the set the accounting needs.
async fn linked_worktree_roots(workspace: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let cwd = workspace.parent().unwrap_or(workspace);
    let Some(repo_root) = git_show_toplevel(cwd) else {
        return Ok(Vec::new());
    };
    let output = tokio::task::spawn_blocking(move || -> anyhow::Result<std::process::Output> {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&repo_root)
            .args(["worktree", "list", "--porcelain", "-z"])
            .output()
            .context("could not run `git worktree list`")
    })
    .await
    .context("git worktree list task panicked")?;
    let output = match output {
        Ok(output) => output,
        Err(e) => return Err(e).context("could not discover linked worktrees"),
    };
    if !output.status.success() {
        anyhow::bail!("git worktree list failed with {}", output.status);
    }
    parse_linked_worktree_roots(&output.stdout)
}

/// A reserved display number and the ledger state that records it.
#[derive(Debug, Clone)]
pub struct ReservedNumber {
    /// The reserved display id (`PRD-074`).
    pub id: String,
    /// The numeric part of the reserved id.
    pub number: u32,
    /// The ledger persisted for this reservation.
    pub ledger: IdLedger,
}

/// Reserve the next display number for `kind` and persist it to the
/// per-kind ledger **before** any document is written, so a crash between
/// the reservation and the write leaves a gap rather than a repeated
/// number.
///
/// The caller must already hold the workspace mutation lock: the lock order
/// is workspace lock first, per-kind allocator lock second. This function
/// takes the allocator lock itself and releases it before returning.
pub async fn reserve_display_number(
    workspace: &Path,
    guard: &WorkspaceLock,
    store: Option<&LanceStore>,
    kind: &ArtifactKind,
    digits: u32,
) -> anyhow::Result<ReservedNumber> {
    guard.matches_workspace(workspace)?;
    let _lock = acquire_id_alloc_lock(workspace, kind)
        .await
        .with_context(|| format!("acquire id-alloc lock for {}", kind.prefix()))?;
    reserve_display_number_locked(workspace, guard, store, kind, digits, None).await
}

/// The reservation step, for callers that already hold the per-kind
/// allocator lock.
async fn reserve_display_number_locked(
    workspace: &Path,
    guard: &WorkspaceLock,
    store: Option<&LanceStore>,
    kind: &ArtifactKind,
    digits: u32,
    bump_above: Option<u32>,
) -> anyhow::Result<ReservedNumber> {
    let (id, number) = compute_next_id(workspace, guard, store, kind, digits, bump_above)
        .await
        .context("compute next-id under id-alloc lock")?;
    let ledger = IdLedger::new(number);
    save_id_ledger(workspace, kind, &ledger)
        .await
        .with_context(|| format!("persist allocation ledger for {}", kind.prefix()))?;
    Ok(ReservedNumber { id, number, ledger })
}

/// A document created through the shared creation mechanism.
#[derive(Debug)]
pub struct CreatedArtifact {
    /// The allocated display id (`PRD-074`).
    pub id: String,
    /// The numeric part of the display id.
    pub number: u32,
    /// The minted canonical key — the document's permanent identity.
    pub canonical_key: String,
    /// Path of the published document.
    pub filepath: PathBuf,
    /// The body the build closure produced.
    pub rendered_body: String,
}

/// Callback used by [`create_artifact_with_minted_key`] to render a fresh
/// `NewArtifact`. It receives the allocated id, its numeric part, and the
/// minted canonical key.
pub type BuildMintedArtifactFn<'a> =
    dyn Fn(&str, u32, &str) -> anyhow::Result<NewArtifact> + Send + Sync + 'a;

/// The one creation mechanism.
///
/// Every producer mints its canonical key here, and the order is fixed
/// (SPEC-009 "Numbering and canonical key are independent"):
///
/// 1. the canonical key is minted **once**, before any attempt to reserve a
///    display number — a repeated command is a new operation, never a
///    hidden retry that changes identity;
/// 2. the display number is reserved under the shared per-kind ledger and
///    persisted before anything is written, so a crash leaves a gap rather
///    than a repeated number;
/// 3. the document is published under `<canonical_key>.md` and never over
///    an existing file.
///
/// `guard` is the caller's already-held workspace lock: the lock order is
/// workspace lock first, per-kind allocator lock second.
pub async fn create_artifact_with_minted_key(
    workspace: &Path,
    guard: &WorkspaceLock,
    store: Option<&LanceStore>,
    kind: &ArtifactKind,
    digits: u32,
    title: &str,
    build: &BuildMintedArtifactFn<'_>,
) -> anyhow::Result<CreatedArtifact> {
    create_artifact_with_minted_key_using(
        MintedKeyRequest {
            workspace,
            guard,
            store,
            kind,
            digits,
            title,
            build,
        },
        crate::artifact::canonical_key::mint_artifact_key,
    )
    .await
}

/// Borrowed inputs for publishing a key minted during read-only batch planning.
///
/// The caller must retain the workspace lock from planning through execution;
/// the usual allocator lock and ledger order remain unchanged.
pub struct PreparedKeyRequest<'a> {
    pub workspace: &'a Path,
    pub guard: &'a WorkspaceLock,
    pub allocation: &'a AllocationGuard,
    pub store: Option<&'a LanceStore>,
    pub kind: &'a ArtifactKind,
    pub digits: u32,
    pub canonical_key: String,
    pub expected_id: &'a str,
    pub build: &'a BuildMintedArtifactFn<'a>,
}

pub async fn create_artifact_with_prepared_key(
    request: PreparedKeyRequest<'_>,
) -> anyhow::Result<CreatedArtifact> {
    let PreparedKeyRequest {
        workspace,
        guard,
        allocation,
        store,
        kind,
        digits,
        canonical_key,
        expected_id,
        build,
    } = request;
    guard.matches_workspace(workspace)?;
    if !crate::artifact::types::is_canonical_key(&canonical_key) {
        anyhow::bail!("invalid prepared canonical key '{canonical_key}'");
    }
    let candidates = crate::artifact::documents::workspace_candidates(workspace).await?;
    if candidates
        .iter()
        .any(|candidate| candidate.canonical_key == canonical_key)
    {
        anyhow::bail!("canonical key '{canonical_key}' is already claimed");
    }
    if let Some(store) = store
        && store
            .list_records(guard, None)
            .await?
            .iter()
            .any(|record| record.canonical_key == canonical_key)
    {
        anyhow::bail!("canonical key '{canonical_key}' is already indexed");
    }
    create_artifact_with_key_locked(LockedKeyRequest {
        workspace,
        guard,
        store,
        kind,
        digits,
        canonical_key,
        expected_id: Some(expected_id),
        allocation: Some(allocation),
        build,
    })
    .await
}

/// Preview the next allocator result while retaining the per-kind lock.
/// `bump_above` accounts for earlier planned creations.
pub async fn preview_display_number(
    workspace: &Path,
    guard: &WorkspaceLock,
    allocation: &AllocationGuard,
    store: Option<&LanceStore>,
    kind: &ArtifactKind,
    digits: u32,
    bump_above: Option<u32>,
) -> anyhow::Result<(String, u32)> {
    guard.matches_workspace(workspace)?;
    allocation.require(workspace, kind)?;
    compute_next_id(workspace, guard, store, kind, digits, bump_above).await
}

/// Reserve a previously previewed number under the shared per-kind lock.
/// A changed high-water mark fails before the ledger is advanced.
pub async fn reserve_prepared_display_number(
    workspace: &Path,
    guard: &WorkspaceLock,
    allocation: &AllocationGuard,
    store: Option<&LanceStore>,
    kind: &ArtifactKind,
    digits: u32,
    expected_id: &str,
) -> anyhow::Result<()> {
    guard.matches_workspace(workspace)?;
    allocation.require(workspace, kind)?;
    let (actual, _) = compute_next_id(workspace, guard, store, kind, digits, None).await?;
    if actual != expected_id {
        anyhow::bail!(
            "planned display id '{expected_id}' changed to '{actual}'; ledger was not advanced"
        );
    }
    reserve_display_number_locked(workspace, guard, store, kind, digits, None).await?;
    Ok(())
}

struct MintedKeyRequest<'a> {
    workspace: &'a Path,
    guard: &'a WorkspaceLock,
    store: Option<&'a LanceStore>,
    kind: &'a ArtifactKind,
    digits: u32,
    title: &'a str,
    build: &'a BuildMintedArtifactFn<'a>,
}

async fn create_artifact_with_minted_key_using<M>(
    request: MintedKeyRequest<'_>,
    mint: M,
) -> anyhow::Result<CreatedArtifact>
where
    M: FnOnce(&ArtifactKind, &str) -> anyhow::Result<String>,
{
    // Mint exactly once before reserving a number. Entropy failures leave no
    // file or index row and consume no display number.
    let canonical_key = mint(request.kind, request.title)?;
    create_artifact_with_key_locked(LockedKeyRequest {
        workspace: request.workspace,
        guard: request.guard,
        store: request.store,
        kind: request.kind,
        digits: request.digits,
        canonical_key,
        expected_id: None,
        allocation: None,
        build: request.build,
    })
    .await
}

struct LockedKeyRequest<'a> {
    workspace: &'a Path,
    guard: &'a WorkspaceLock,
    store: Option<&'a LanceStore>,
    kind: &'a ArtifactKind,
    digits: u32,
    canonical_key: String,
    expected_id: Option<&'a str>,
    allocation: Option<&'a AllocationGuard>,
    build: &'a BuildMintedArtifactFn<'a>,
}

async fn create_artifact_with_key_locked(
    request: LockedKeyRequest<'_>,
) -> anyhow::Result<CreatedArtifact> {
    let LockedKeyRequest {
        workspace,
        guard,
        store,
        kind,
        digits,
        canonical_key,
        expected_id,
        allocation,
        build,
    } = request;
    let _lock = match allocation {
        Some(guard) => {
            guard.require(workspace, kind)?;
            None
        }
        None => Some(
            acquire_id_alloc_lock(workspace, kind)
                .await
                .with_context(|| format!("acquire id-alloc lock for {}", kind.prefix()))?,
        ),
    };
    if let Some(expected) = expected_id {
        let (actual, _) = compute_next_id(workspace, guard, store, kind, digits, None).await?;
        if actual != expected {
            anyhow::bail!(
                "planned display id '{expected}' changed to '{actual}'; ledger was not advanced"
            );
        }
    }
    let reserved =
        reserve_display_number_locked(workspace, guard, store, kind, digits, None).await?;

    let mut artifact = build(&reserved.id, reserved.number, &canonical_key)
        .with_context(|| format!("build artifact for {}", reserved.id))?;
    if artifact.id != reserved.id {
        anyhow::bail!(
            "internal: build() returned mismatched id {:?}, expected {}",
            artifact.id,
            reserved.id
        );
    }
    artifact.canonical_key = canonical_key.clone();

    let ctx = MutationContext::new(workspace, guard, store);
    let filepath = projection::create_artifact_with_canonical_key(&ctx, &artifact)
        .await
        .with_context(|| format!("publish {}", canonical_key))?;

    Ok(CreatedArtifact {
        id: reserved.id,
        number: reserved.number,
        canonical_key,
        filepath,
        rendered_body: artifact.body,
    })
}

/// Compute the next-id for `kind` by taking the maximum of:
/// - the per-kind ledger watermark,
/// - the canonical frontmatter of the current tree,
/// - the frontmatter of every linked worktree of the same git common dir,
/// - LanceDB row IDs with the same prefix
///
/// Every source is consulted because PROB-067 surfaced via a race where
/// one store led the other; whichever is ahead at the moment of
/// the call is the safe lower bound for the next number.
async fn compute_next_id(
    workspace: &Path,
    permit: &dyn WorkspaceReadPermit,
    store: Option<&LanceStore>,
    kind: &ArtifactKind,
    digits: u32,
    bump_above: Option<u32>,
) -> anyhow::Result<(String, u32)> {
    let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
    let mut max_num = compute_max_display_number(workspace, permit, store, kind).await?;

    // Audit override: caller asks us to skip past a known-bad number.
    if let Some(bump) = bump_above {
        max_num = max_num.max(bump);
    }

    // A `u32` overflow is an error, never a wrap.
    let next = max_num.checked_add(1).ok_or_else(|| {
        anyhow::anyhow!("display number for kind {prefix} overflows u32 — refusing to wrap around")
    })?;
    let id = format!("{}-{:0>width$}", prefix, next, width = digits as usize);
    Ok((id, next))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn ws(tmp: &TempDir) -> PathBuf {
        let ws = tmp.path().join(".forgeplan");
        std::fs::create_dir_all(&ws).unwrap();
        ws
    }

    #[tokio::test]
    async fn id_alloc_lock_path_uses_per_kind_naming() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let p_evid = id_alloc_lock_path(&ws, &ArtifactKind::EvidencePack);
        let p_prd = id_alloc_lock_path(&ws, &ArtifactKind::Prd);
        // Two different kinds must resolve to different lock files
        // (per-kind granularity invariant).
        assert_ne!(
            p_evid,
            p_prd,
            "per-kind lock paths must differ: {} vs {}",
            p_evid.display(),
            p_prd.display()
        );
        // Each path's filename includes the kind prefix (lowercased
        // by `kind.prefix()`).
        let name_evid = p_evid.file_name().unwrap().to_string_lossy().to_lowercase();
        assert!(
            name_evid.contains("evid"),
            "EVID lock filename should include 'evid', got {}",
            p_evid.display()
        );
        let name_prd = p_prd.file_name().unwrap().to_string_lossy().to_lowercase();
        assert!(
            name_prd.contains("prd"),
            "PRD lock filename should include 'prd', got {}",
            p_prd.display()
        );
    }

    #[tokio::test]
    async fn lock_serializes_concurrent_acquirers() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::time::{Duration as TDuration, sleep};

        let tmp = TempDir::new().unwrap();
        let ws = Arc::new(ws(&tmp));
        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..4 {
            let ws = ws.clone();
            let in_flight = in_flight.clone();
            let peak = peak.clone();
            handles.push(tokio::spawn(async move {
                let _g = acquire_id_alloc_lock(&ws, &ArtifactKind::EvidencePack)
                    .await
                    .unwrap();
                let cur = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(cur, Ordering::SeqCst);
                sleep(TDuration::from_millis(20)).await;
                in_flight.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(
            peak.load(Ordering::SeqCst),
            1,
            "id-alloc lock must serialize concurrent holders"
        );
    }

    #[tokio::test]
    async fn different_kinds_do_not_block_each_other() {
        // Per-kind granularity check: holding EVID lock must not block
        // a PRD allocation.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let _g_evid = acquire_id_alloc_lock(&ws, &ArtifactKind::EvidencePack)
            .await
            .unwrap();
        // Should succeed immediately — different lock file.
        let _g_prd = tokio::time::timeout(
            Duration::from_secs(2),
            acquire_id_alloc_lock(&ws, &ArtifactKind::Prd),
        )
        .await
        .expect("PRD lock acquisition must not block on EVID lock")
        .unwrap();
    }

    #[tokio::test]
    async fn borrowed_builder_allocates_distinct_canonical_artifacts_without_a_store() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let title = String::from("Borrowed request title");
        let build = |id: &str, _: u32, key: &str| -> anyhow::Result<NewArtifact> {
            Ok(NewArtifact {
                canonical_key: key.to_owned(),
                id: id.to_owned(),
                kind: "prd".into(),
                status: "draft".into(),
                title: title.clone(),
                body: "## Summary\n\nConsumer-visible request content.\n".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: vec!["request-context".into(), "retain-on-promote".into()],
            })
        };
        let first = create_artifact_with_minted_key(
            &ws,
            &guard,
            None,
            &ArtifactKind::Prd,
            3,
            &title,
            &build,
        )
        .await
        .unwrap();
        let first_bytes = tokio::fs::read(&first.filepath).await.unwrap();
        let second = create_artifact_with_minted_key(
            &ws,
            &guard,
            None,
            &ArtifactKind::Prd,
            3,
            &title,
            &build,
        )
        .await
        .unwrap();
        assert_eq!(first.id, "PRD-001");
        assert_eq!(second.id, "PRD-002");
        assert_ne!(first.canonical_key, second.canonical_key);
        assert_ne!(first.filepath, second.filepath);
        assert_eq!(tokio::fs::read(&first.filepath).await.unwrap(), first_bytes);
        assert_eq!(
            first.filepath.file_name().unwrap().to_str().unwrap(),
            format!("{}.md", first.canonical_key)
        );
        assert_eq!(
            second.filepath.file_name().unwrap().to_str().unwrap(),
            format!("{}.md", second.canonical_key)
        );
        let text = tokio::fs::read_to_string(&second.filepath).await.unwrap();
        let (frontmatter, body) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
        assert_eq!(
            frontmatter.get("title").and_then(|v| v.as_str()),
            Some(title.as_str())
        );
        assert_eq!(
            frontmatter.get("tags"),
            Some(&serde_yaml::to_value(["request-context", "retain-on-promote"]).unwrap())
        );
        assert!(body.contains("Consumer-visible request content."));
        assert!(!ws.join("lance/artifacts.lance").exists());
    }

    fn git_is_available() -> bool {
        std::process::Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn git(cwd: &Path, args: &[&str]) -> std::process::Output {
        std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_AUTHOR_NAME", "allocator")
            .env("GIT_AUTHOR_EMAIL", "allocator@example.com")
            .env("GIT_COMMITTER_NAME", "allocator")
            .env("GIT_COMMITTER_EMAIL", "allocator@example.com")
            .output()
            .expect("git invocation failed")
    }

    #[tokio::test]
    async fn two_documents_with_the_same_title_get_different_keys_and_files() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let title = String::from("Identical title");
        let build = |id: &str, _: u32, key: &str| -> anyhow::Result<NewArtifact> {
            Ok(NewArtifact {
                canonical_key: key.to_owned(),
                id: id.to_owned(),
                kind: "prd".into(),
                status: "draft".into(),
                title: title.clone(),
                body: "## Summary\n\nSame title, different document.\n".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: vec![],
            })
        };
        let first = create_artifact_with_minted_key(
            &ws,
            &guard,
            None,
            &ArtifactKind::Prd,
            3,
            &title,
            &build,
        )
        .await
        .unwrap();
        let second = create_artifact_with_minted_key(
            &ws,
            &guard,
            None,
            &ArtifactKind::Prd,
            3,
            &title,
            &build,
        )
        .await
        .unwrap();

        assert_ne!(
            first.canonical_key, second.canonical_key,
            "two documents with the same title must not share a key"
        );
        assert_ne!(first.filepath, second.filepath);
        assert_eq!(
            first
                .filepath
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            format!("{}.md", first.canonical_key)
        );
        assert_eq!(
            second
                .filepath
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            format!("{}.md", second.canonical_key)
        );
        assert!(first.filepath.exists());
        assert!(second.filepath.exists());
        assert_ne!(first.id, second.id);

        for created in [&first, &second] {
            let text = std::fs::read_to_string(&created.filepath).unwrap();
            let (fm, body) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
            assert_eq!(
                fm.get("slug").and_then(|v| v.as_str()),
                Some(created.canonical_key.as_str())
            );
            assert_eq!(
                fm.get("title").and_then(|v| v.as_str()),
                Some(title.as_str())
            );
            assert_eq!(
                body.trim_end(),
                "## Summary\n\nSame title, different document."
            );
        }
    }

    #[tokio::test]
    async fn cyrillic_title_creation_publishes_the_key_and_preserves_file_bytes() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let title = "Еженедельный обзор";
        let expected_body = "## Summary\n\nCyrillic title body.\n";
        let build = |id: &str, _: u32, key: &str| -> anyhow::Result<NewArtifact> {
            Ok(NewArtifact {
                canonical_key: key.to_owned(),
                id: id.to_owned(),
                kind: "prd".into(),
                status: "draft".into(),
                title: title.to_string(),
                body: expected_body.to_string(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: vec![],
            })
        };

        let first = create_artifact_with_minted_key(
            &ws,
            &guard,
            None,
            &ArtifactKind::Prd,
            3,
            title,
            &build,
        )
        .await
        .unwrap();
        let first_bytes = tokio::fs::read(&first.filepath).await.unwrap();
        let second = create_artifact_with_minted_key(
            &ws,
            &guard,
            None,
            &ArtifactKind::Prd,
            3,
            title,
            &build,
        )
        .await
        .unwrap();

        assert_ne!(first.canonical_key, second.canonical_key);
        assert_ne!(first.filepath, second.filepath);
        assert_eq!(tokio::fs::read(&first.filepath).await.unwrap(), first_bytes);
        for created in [&first, &second] {
            assert!(created.canonical_key.starts_with("prd-document-k"));
            assert_eq!(
                created.filepath.file_name().unwrap().to_str().unwrap(),
                format!("{}.md", created.canonical_key)
            );
            let bytes = tokio::fs::read(&created.filepath).await.unwrap();
            let text = std::str::from_utf8(&bytes).unwrap();
            let (frontmatter, body) =
                crate::artifact::frontmatter::parse_frontmatter(text).unwrap();
            assert_eq!(
                frontmatter.get("slug").and_then(|value| value.as_str()),
                Some(created.canonical_key.as_str())
            );
            assert_eq!(
                frontmatter.get("title").and_then(|value| value.as_str()),
                Some(title)
            );
            assert_eq!(body.trim_end(), expected_body.trim_end());
        }
    }

    #[tokio::test]
    async fn entropy_failure_creates_no_file_and_reserves_no_number() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let build = |id: &str, _: u32, key: &str| -> anyhow::Result<NewArtifact> {
            Ok(NewArtifact {
                canonical_key: key.to_owned(),
                id: id.to_owned(),
                kind: "prd".into(),
                status: "draft".into(),
                title: "Entropy failure".into(),
                body: "Content.".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: vec![],
            })
        };

        let error = create_artifact_with_minted_key_using(
            MintedKeyRequest {
                workspace: &ws,
                guard: &guard,
                store: None,
                kind: &ArtifactKind::Prd,
                digits: 3,
                title: "Entropy failure",
                build: &build,
            },
            |_, _| anyhow::bail!("entropy unavailable"),
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("entropy unavailable"));
        assert!(!ws.join("prds").exists());
        assert!(!id_ledger_path(&ws, &ArtifactKind::Prd).exists());
    }

    #[tokio::test]
    async fn canonical_key_collision_refuses_without_overwriting_or_reminting() {
        use std::cell::Cell;

        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let key = "prd-collision-k0123456789abcdef0123456789abcdef";
        let mint_calls = Cell::new(0);
        let mint = |_: &ArtifactKind, _: &str| {
            mint_calls.set(mint_calls.get() + 1);
            Ok(key.to_string())
        };
        let build = |id: &str, _: u32, key: &str| -> anyhow::Result<NewArtifact> {
            Ok(NewArtifact {
                canonical_key: key.to_owned(),
                id: id.to_owned(),
                kind: "prd".into(),
                status: "draft".into(),
                title: "Collision".into(),
                body: "Original bytes.".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: vec![],
            })
        };

        let first = create_artifact_with_minted_key_using(
            MintedKeyRequest {
                workspace: &ws,
                guard: &guard,
                store: None,
                kind: &ArtifactKind::Prd,
                digits: 3,
                title: "Collision",
                build: &build,
            },
            &mint,
        )
        .await
        .unwrap();
        let before = tokio::fs::read(&first.filepath).await.unwrap();
        let second = create_artifact_with_minted_key_using(
            MintedKeyRequest {
                workspace: &ws,
                guard: &guard,
                store: None,
                kind: &ArtifactKind::Prd,
                digits: 3,
                title: "Collision",
                build: &build,
            },
            &mint,
        )
        .await;

        assert!(second.is_err());
        assert_eq!(mint_calls.get(), 2);
        assert_eq!(tokio::fs::read(&first.filepath).await.unwrap(), before);
        let mut entries = tokio::fs::read_dir(first.filepath.parent().unwrap())
            .await
            .unwrap();
        let mut files = Vec::new();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            files.push(entry.file_name());
        }
        assert_eq!(files, [std::ffi::OsString::from(format!("{key}.md"))]);
    }

    #[tokio::test]
    async fn created_row_and_file_store_the_same_canonical_key() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = LanceStore::init_new(&ws, &guard).await.unwrap();
        let title = "Stored identity";
        let build = |id: &str, _: u32, key: &str| -> anyhow::Result<NewArtifact> {
            Ok(NewArtifact {
                canonical_key: key.to_owned(),
                id: id.to_owned(),
                kind: "prd".into(),
                status: "draft".into(),
                title: title.to_string(),
                body: "## Summary\n\nStored row content.\n".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: vec![],
            })
        };

        let created = create_artifact_with_minted_key(
            &ws,
            &guard,
            Some(&store),
            &ArtifactKind::Prd,
            3,
            title,
            &build,
        )
        .await
        .unwrap();
        let row = store
            .get_record(&guard, &created.canonical_key)
            .await
            .unwrap()
            .unwrap();
        let file = tokio::fs::read_to_string(&created.filepath).await.unwrap();
        let (frontmatter, _body) = crate::artifact::frontmatter::parse_frontmatter(&file).unwrap();

        assert_eq!(row.canonical_key, created.canonical_key);
        assert_eq!(
            frontmatter.get("slug").and_then(|value| value.as_str()),
            Some(created.canonical_key.as_str())
        );
        assert_eq!(
            created.filepath.file_name().unwrap().to_str().unwrap(),
            format!("{}.md", created.canonical_key)
        );
        assert_eq!(std::fs::read_to_string(&created.filepath).unwrap(), file);
    }

    #[test]
    fn ledger_json_round_trips_and_rejects_damage() {
        let ledger = IdLedger::new(42);
        let parsed = IdLedger::from_json(&ledger.to_json()).unwrap();
        assert_eq!(parsed, ledger);
        // Damaged: not JSON at all.
        assert!(IdLedger::from_json("{ not json").is_err());
        // Damaged: unknown schema version.
        assert!(IdLedger::from_json(r#"{"schema_version": 99, "high_watermark": 3}"#).is_err());
        // Damaged: no high watermark.
        assert!(IdLedger::from_json(r#"{"schema_version": 1}"#).is_err());
    }

    #[tokio::test]
    async fn damaged_ledger_is_an_error_not_a_skipped_maximum() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        std::fs::write(
            id_ledger_path(&ws, &ArtifactKind::Prd),
            "{ this is not a ledger",
        )
        .unwrap();
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let err = compute_max_display_number(&ws, &guard, None, &ArtifactKind::Prd)
            .await
            .expect_err("a damaged ledger must be an error, not a skipped maximum");
        assert!(
            err.to_string().contains("damaged allocation ledger"),
            "unexpected error: {err:#}"
        );
    }

    #[tokio::test]
    async fn ledger_watermark_bounds_the_next_number() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        save_id_ledger(&ws, &ArtifactKind::Prd, &IdLedger::new(7))
            .await
            .unwrap();
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let max = compute_max_display_number(&ws, &guard, None, &ArtifactKind::Prd)
            .await
            .unwrap();
        assert_eq!(max, 7);
        let reserved = reserve_display_number(&ws, &guard, None, &ArtifactKind::Prd, 3)
            .await
            .unwrap();
        assert_eq!(reserved.number, 8);
        assert_eq!(reserved.id, "PRD-008");
    }

    #[tokio::test]
    async fn a_reserved_number_is_never_handed_out_twice_after_a_crash() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        // Reserve, then "crash": no document is written to disk.
        let first = reserve_display_number(&ws, &guard, None, &ArtifactKind::Prd, 3)
            .await
            .unwrap();
        assert_eq!(first.number, 1);
        // The reservation is durable — the ledger is on disk before any
        // document could be written.
        let ledger = load_id_ledger(&ws, &ArtifactKind::Prd)
            .await
            .unwrap()
            .expect("ledger must exist after a reservation");
        assert_eq!(ledger.high_watermark, 1);
        drop(guard);
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        // The next allocation must not reuse the reserved number.
        let second = reserve_display_number(&ws, &guard, None, &ArtifactKind::Prd, 3)
            .await
            .unwrap();
        assert_eq!(second.number, 2);
        assert_ne!(first.id, second.id);
    }

    #[tokio::test]
    async fn u32_overflow_is_an_error_not_a_wrap() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        save_id_ledger(&ws, &ArtifactKind::Prd, &IdLedger::new(u32::MAX))
            .await
            .unwrap();
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let err = reserve_display_number(&ws, &guard, None, &ArtifactKind::Prd, 3)
            .await
            .expect_err("an overflow must be an error, never a wrap");
        assert!(
            format!("{err:#}").contains("overflow"),
            "unexpected error: {err:#}"
        );
    }

    #[tokio::test]
    async fn canonical_frontmatter_of_the_current_tree_bounds_the_maximum() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = ws.join("prds");
        std::fs::create_dir_all(&dir).unwrap();
        // A document published under its canonical key: the number lives in
        // the frontmatter, not in the file name.
        std::fs::write(
            dir.join("prd-weekly-review-k0123456789abcdef0123456789abcdef.md"),
            "---\nid: PRD-042\nkind: prd\ntitle: Weekly review\n---\n\nBody.\n",
        )
        .unwrap();
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let max = compute_max_display_number(&ws, &guard, None, &ArtifactKind::Prd)
            .await
            .unwrap();
        assert_eq!(max, 42);
    }

    #[tokio::test]
    async fn a_number_reserved_in_one_linked_worktree_is_not_handed_out_in_another() {
        if !git_is_available() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q"]);
        std::fs::write(repo.join("README.md"), "seed\n").unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-q", "-m", "init"]);
        let linked = tmp.path().join("linked");
        let out = git(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                linked.to_str().unwrap(),
                "-b",
                "linked",
            ],
        );
        assert!(
            out.status.success(),
            "git worktree add failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let ws_main = repo.join(".forgeplan");
        let ws_linked = linked.join(".forgeplan");
        std::fs::create_dir_all(&ws_main).unwrap();
        std::fs::create_dir_all(&ws_linked).unwrap();
        let guard_main = crate::workspace::acquire_workspace_lock(&ws_main)
            .await
            .unwrap();
        let guard_linked = crate::workspace::acquire_workspace_lock(&ws_linked)
            .await
            .unwrap();

        let first = reserve_display_number(&ws_main, &guard_main, None, &ArtifactKind::Prd, 3)
            .await
            .unwrap();
        let second = reserve_display_number(&ws_linked, &guard_linked, None, &ArtifactKind::Prd, 3)
            .await
            .unwrap();
        assert_ne!(
            first.number, second.number,
            "a number reserved in one worktree was handed out again in another"
        );
        assert_eq!(first.number, 1);
        assert_eq!(second.number, 2);
    }

    #[tokio::test]
    async fn the_ledger_lives_beside_the_allocator_lock() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let lock = id_alloc_lock_path(&ws, &ArtifactKind::Prd);
        let ledger = id_ledger_path(&ws, &ArtifactKind::Prd);
        assert_eq!(
            lock.parent(),
            ledger.parent(),
            "ledger and lock must sit in the same directory"
        );
        assert_eq!(
            ledger.file_name().unwrap().to_string_lossy(),
            "id-PRD.json",
            "ledger file name must carry the kind"
        );
    }
    #[tokio::test]
    async fn malformed_document_frontmatter_aborts_allocation_accounting() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = ws.join(ArtifactKind::Prd.dir_name());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("imported.md"),
            "---\nid: PRD-900\nkind: prd\nslug: prd-imported-k0123456789abcdef0123456789abcdef\n",
        )
        .unwrap();
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();

        let error = compute_max_display_number(&ws, &guard, None, &ArtifactKind::Prd)
            .await
            .expect_err("a malformed source must not be omitted from accounting");
        assert!(
            error
                .to_string()
                .contains("could not account for frontmatter")
        );
    }

    #[test]
    fn worktree_porcelain_paths_preserve_exact_posix_bytes() {
        let stdout = b"worktree /tmp/linked \xff \n\0HEAD deadbeef\0\0";
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let roots = parse_linked_worktree_roots(stdout).unwrap();
            assert_eq!(roots.len(), 1);
            assert_eq!(roots[0].as_os_str().as_bytes(), b"/tmp/linked \xff \n");
        }
        #[cfg(not(unix))]
        assert!(parse_linked_worktree_roots(stdout).is_err());
    }

    #[tokio::test]
    async fn allocation_accounting_keeps_large_u32_hints_and_rejects_invalid_numbers() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = ws.join(ArtifactKind::Prd.dir_name());
        std::fs::create_dir_all(&dir).unwrap();
        let document = dir.join("imported.md");
        std::fs::write(
            &document,
            "---\nid: PRD-7\nkind: prd\nassigned_number: 1000001\n---\nBody.\n",
        )
        .unwrap();
        assert_eq!(
            max_number_in_tree(&dir, &ArtifactKind::Prd).await.unwrap(),
            1_000_001
        );
        std::fs::write(
            &document,
            "---\nid: PRD-7\nkind: prd\npredicted_number: 4294967295\n---\nBody.\n",
        )
        .unwrap();
        assert_eq!(
            max_number_in_tree(&dir, &ArtifactKind::Prd).await.unwrap(),
            u32::MAX
        );

        for frontmatter in [
            "id: PRD-7\nkind: prd\nassigned_number: 4294967296",
            "id: PRD-7\nkind: prd\npredicted_number: malformed",
            "id: PRD-4294967296\nkind: prd",
            "id: PRD-not-a-number\nkind: prd",
        ] {
            std::fs::write(&document, format!("---\n{frontmatter}\n---\nBody.\n")).unwrap();
            assert!(
                max_number_in_tree(&dir, &ArtifactKind::Prd).await.is_err(),
                "invalid allocation number was silently ignored: {frontmatter}"
            );
        }
    }

    /// A canonical file name carries the *title*, not the display number —
    /// and a title may start with digits. `prd-2024-review-k<hex>.md` must
    /// not push the shared watermark from 7 to 2024.
    #[tokio::test]
    async fn a_canonical_file_name_with_a_numeric_title_does_not_inflate_the_maximum() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = ws.join(ArtifactKind::Prd.dir_name());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("prd-2024-review-k0123456789abcdef0123456789abcdef.md"),
            "---\nid: PRD-007\nkind: prd\ntitle: 2024 review\n---\n\nBody.\n",
        )
        .unwrap();
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let max = compute_max_display_number(&ws, &guard, None, &ArtifactKind::Prd)
            .await
            .unwrap();
        assert_eq!(
            max, 7,
            "the title part of a canonical file name must not be read as a display number"
        );
    }

    /// The legacy projection shape `<ID>-<slug>.md` still bounds the maximum,
    /// including the extension-only form `<ID>.md`.
    ///
    /// The frontmatter of these files carries no display id, so the number is
    /// accounted for **only** through the file name — which is exactly the
    /// path the canonical-name rule must not break.
    #[tokio::test]
    async fn legacy_file_names_still_bound_the_maximum() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = ws.join(ArtifactKind::Prd.dir_name());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("PRD-074-title.md"),
            "---\nkind: prd\ntitle: Legacy\n---\n\nBody.\n",
        )
        .unwrap();
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let max = compute_max_display_number(&ws, &guard, None, &ArtifactKind::Prd)
            .await
            .unwrap();
        assert_eq!(max, 74, "a legacy file name must still bound the maximum");

        // The extension-only form `<ID>.md` counts too — the number must not
        // be read out of the name with its extension attached.
        std::fs::write(
            dir.join("PRD-090.md"),
            "---\nkind: prd\ntitle: Legacy\n---\n\nBody.\n",
        )
        .unwrap();
        let max = compute_max_display_number(&ws, &guard, None, &ArtifactKind::Prd)
            .await
            .unwrap();
        assert_eq!(
            max, 90,
            "an extension-only legacy file name must still bound the maximum"
        );
    }

    /// SPEC V12: a damaged ledger is an explicit diagnosable error, never a
    /// silently skipped maximum. Every damage shape is checked.
    #[tokio::test]
    async fn every_damaged_ledger_shape_is_an_explicit_error() {
        let shapes: [(&str, &str); 4] = [
            ("empty", ""),
            ("garbage", "{ this is not a ledger"),
            (
                "unknown schema version",
                "{\"schema_version\": 99, \"high_watermark\": 3}",
            ),
            (
                "watermark beyond u32",
                "{\"schema_version\": 1, \"high_watermark\": 4294967296}",
            ),
        ];
        for (label, content) in shapes {
            let tmp = TempDir::new().unwrap();
            let ws = ws(&tmp);
            std::fs::write(id_ledger_path(&ws, &ArtifactKind::Prd), content).unwrap();
            let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
            let error = compute_max_display_number(&ws, &guard, None, &ArtifactKind::Prd)
                .await
                .expect_err("a damaged ledger must be an error, not a skipped maximum");
            assert!(
                error.to_string().contains("damaged allocation ledger"),
                "unexpected error for {label}: {error:#}"
            );
        }
    }

    /// The ledger is the shared record of every number a kind ever handed
    /// out, so a later write may never lower it.
    #[tokio::test]
    async fn a_saved_watermark_never_moves_backwards() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        save_id_ledger(&ws, &ArtifactKind::Prd, &IdLedger::new(7))
            .await
            .unwrap();
        let error = save_id_ledger(&ws, &ArtifactKind::Prd, &IdLedger::new(3))
            .await
            .expect_err("a stale write must not lower the shared watermark");
        assert!(
            format!("{error:#}").contains("refusing to lower"),
            "unexpected error: {error:#}"
        );
        let ledger = load_id_ledger(&ws, &ArtifactKind::Prd)
            .await
            .unwrap()
            .expect("ledger must exist");
        assert_eq!(
            ledger.high_watermark, 7,
            "the shared watermark must survive a refused write"
        );
    }

    /// A publication that fails after the reservation was persisted leaves a
    /// gap, never a repeated number.
    #[tokio::test]
    async fn a_failed_publication_still_consumes_the_reserved_number() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let build = |_id: &str, _number: u32, _key: &str| -> anyhow::Result<NewArtifact> {
            anyhow::bail!("publication failed after the reservation")
        };
        let error = create_artifact_with_minted_key_using(
            MintedKeyRequest {
                workspace: &ws,
                guard: &guard,
                store: None,
                kind: &ArtifactKind::Prd,
                digits: 3,
                title: "Reserved then failed",
                build: &build,
            },
            crate::artifact::canonical_key::mint_artifact_key,
        )
        .await
        .expect_err("the failing publication must surface its error");
        assert!(
            format!("{error:#}").contains("publication failed"),
            "unexpected error: {error:#}"
        );

        let ledger = load_id_ledger(&ws, &ArtifactKind::Prd)
            .await
            .unwrap()
            .expect("the reservation must be persisted before the document write");
        assert_eq!(ledger.high_watermark, 1);
        assert!(
            !ws.join(ArtifactKind::Prd.dir_name()).exists(),
            "no document may exist for the failed publication"
        );

        let next = reserve_display_number(&ws, &guard, None, &ArtifactKind::Prd, 3)
            .await
            .unwrap();
        assert_eq!(
            next.number, 2,
            "the reserved number must not be handed out again"
        );
    }

    /// Two worktrees of one clone must name **one** lock and **one** ledger,
    /// or the shared accounting is two independent accountings.
    #[tokio::test]
    async fn one_clone_shares_one_lock_and_one_ledger_across_its_worktrees() {
        if !git_is_available() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q"]);
        std::fs::write(repo.join("README.md"), "seed\n").unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-q", "-m", "init"]);
        let linked = tmp.path().join("linked");
        let out = git(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                linked.to_str().unwrap(),
                "-b",
                "linked",
            ],
        );
        assert!(
            out.status.success(),
            "git worktree add failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let ws_main = repo.join(".forgeplan");
        let ws_linked = linked.join(".forgeplan");
        std::fs::create_dir_all(&ws_main).unwrap();
        std::fs::create_dir_all(&ws_linked).unwrap();

        assert_eq!(
            id_alloc_lock_path(&ws_main, &ArtifactKind::Prd),
            id_alloc_lock_path(&ws_linked, &ArtifactKind::Prd),
            "linked worktrees must contend on the same allocator lock"
        );
        assert_eq!(
            id_ledger_path(&ws_main, &ArtifactKind::Prd),
            id_ledger_path(&ws_linked, &ArtifactKind::Prd),
            "linked worktrees must share one allocation ledger"
        );
        let ledger = id_ledger_path(&ws_main, &ArtifactKind::Prd);
        assert!(
            !ledger.starts_with(&ws_main) && !ledger.starts_with(&ws_linked),
            "the ledger is local bookkeeping and must live outside every workspace"
        );
    }
}
