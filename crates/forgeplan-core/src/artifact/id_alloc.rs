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
// 3. **Post-write collision detection (Option D)** — even with the lock
//    held, after the projection write we re-check whether the target slug
//    path was racily created by an unlocked path. If yes, retry up to
//    5 times with the next-higher number, panic on cap exceeded (lock
//    is broken — diagnostic > silent corruption).
//
// 4. **Slug-based collision check** — we look at on-disk filenames AND
//    the LanceDB row set, because PROB-067 surfaced via the LanceDB row
//    overwriting an existing artifact's body. Verifying both makes the
//    allocator immune to either store being inconsistent at the moment
//    of the call.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;

use crate::artifact::types::ArtifactKind;
use crate::db::store::{LanceStore, NewArtifact};
use crate::projection::{self, MutationContext};
use crate::workspace::lock::{DEFAULT_LOCK_TIMEOUT, WorkspaceLock};

/// Maximum number of retries when collision detected post-write.
///
/// Above this cap we panic with a diagnostic — it indicates the lock is
/// broken (someone is allocating IDs OUTSIDE the locked path) rather
/// than a legitimate race the retry could resolve.
const MAX_COLLISION_RETRIES: u32 = 5;

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
    if let Some(common_dir) = git_common_dir(workspace) {
        let dir = common_dir.join("forgeplan");
        // best-effort: created lazily by `acquire_workspace_lock_with_timeout`
        // via `create_dir_all` on the parent before open.
        return dir.join(format!("id-{}.lock", kind.prefix().trim_end_matches('-')));
    }
    // Fallback: workspace-local. This is the existing race surface but
    // strictly no worse than today's behavior — added paths still
    // benefit from collision retry below.
    workspace.join(format!(
        ".id-alloc-{}.lock",
        kind.prefix().trim_end_matches('-')
    ))
}

/// Run `git rev-parse --git-common-dir` and return its path, or None
/// if not inside a git repository. Synchronous (one-shot subprocess
/// during command setup — kept blocking to avoid pulling in
/// `tokio::process` for a single-shot call). The output is small enough
/// that the call completes in microseconds in practice.
fn git_common_dir(workspace: &Path) -> Option<PathBuf> {
    // Workspace is `<repo>/.forgeplan` — git ops should be rooted at
    // its parent (the actual working tree). Fall back to workspace
    // itself if it has no parent (shouldn't happen for valid workspaces).
    let cwd = workspace.parent().unwrap_or(workspace);
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
                    let transient = e.kind() == std::io::ErrorKind::WouldBlock;
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

/// Compute the next-id for `kind` by taking the maximum of:
/// - on-disk numeric suffix in `<workspace>/<kind-dir>/<KIND>-NNN-*.md`
/// - LanceDB row IDs with the same prefix
///
/// Both sources are consulted because PROB-067 surfaced via a race where
/// one store led the other; whichever is ahead at the moment of the
/// call is the safe lower bound for the next number.
async fn compute_next_id(
    workspace: &Path,
    store: Option<&LanceStore>,
    kind: &ArtifactKind,
    digits: u32,
    bump_above: Option<u32>,
) -> anyhow::Result<(String, u32)> {
    let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
    let dir = workspace.join(kind.dir_name());

    let mut max_num: u32 = 0;

    if dir.exists() {
        let mut rd = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = rd.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            let upper = name.to_uppercase();
            if let Some(rest) = upper.strip_prefix(&format!("{}-", prefix))
                && let Some(num_str) = rest.split('-').next()
                && let Ok(num) = num_str.parse::<u32>()
            {
                max_num = max_num.max(num);
            }
        }
    }

    if let Some(s) = store {
        let lance_id = s.next_id(&prefix).await?;
        // `next_id` returns the *next* value (max + 1); convert back to
        // current max for comparison.
        if let Some(num_str) = lance_id.rsplit('-').next()
            && let Ok(next) = num_str.parse::<u32>()
        {
            let lance_max = next.saturating_sub(1);
            max_num = max_num.max(lance_max);
        }
    }

    // Audit override: caller asks us to skip past a known-bad number.
    if let Some(bump) = bump_above {
        max_num = max_num.max(bump);
    }

    let next = max_num + 1;
    let id = format!("{}-{:0>width$}", prefix, next, width = digits as usize);
    Ok((id, next))
}

/// Result of allocating an id and creating its artifact.
#[derive(Debug)]
pub struct AllocatedArtifact {
    /// Final assigned id (`PRD-074`, `EVID-119`, …).
    pub id: String,
    /// Numeric portion (`074`, `119`, …).
    pub number: u32,
    /// Path of the projection file on disk.
    pub filepath: PathBuf,
    /// The body that the build closure produced for the final id. Kept
    /// alongside the path because callers (MCP / CLI `forgeplan_new`)
    /// derive response fields (`slug`, `assigned_number`, hint refs)
    /// from the augmented frontmatter inside this rendered string, NOT
    /// from the on-disk projection (which re-renders frontmatter from
    /// `NewArtifact` fields and folds the augmented fm into the body
    /// section). Returning the canonical rendered body avoids a
    /// disk re-read + double-frontmatter parsing edge case.
    pub rendered_body: String,
}

/// Callback used by `allocate_and_create_artifact` to render a fresh
/// `NewArtifact` for a given allocated id. The callback is invoked
/// AFTER the cross-worktree lock is held, and may be called multiple
/// times if a post-write collision is detected (retry path).
pub type BuildArtifactFn = dyn Fn(&str, u32) -> anyhow::Result<NewArtifact> + Send + Sync;

/// Allocate an id and write the projection file atomically with respect
/// to other concurrent `forgeplan_new` invocations — even those running
/// in sibling git worktrees of the same repo.
///
/// Workflow:
/// 1. Acquire cross-worktree id-alloc lock for `kind` (per-kind).
/// 2. Compute candidate next-id from filesystem + LanceDB.
/// 3. Call `build` to render the `NewArtifact` for that id.
/// 4. Pre-write existence check (`<kind>/{ID}-*.md`).
///    On collision: bump and retry (loop).
/// 5. `create_artifact_with_projection`.
/// 6. Post-write verify: assert the file we just wrote is the unique
///    holder of `{ID}-` prefix in `<kind>/`. On collision: bump and
///    retry (loop, ≤ MAX_COLLISION_RETRIES).
///
/// The `build` closure receives `(id, number)`; callers use these to
/// fill template placeholders + augment frontmatter with `slug` /
/// `predicted_number`. Re-running `build` for the retry case yields a
/// fresh artifact with the bumped id — no stale-state bleed.
pub async fn allocate_and_create_artifact(
    workspace: &Path,
    store: &LanceStore,
    kind: &ArtifactKind,
    digits: u32,
    build: &BuildArtifactFn,
) -> anyhow::Result<AllocatedArtifact> {
    let _lock = acquire_id_alloc_lock(workspace, kind)
        .await
        .with_context(|| format!("acquire id-alloc lock for {}", kind.prefix()))?;

    let mut bump_above: Option<u32> = None;
    for attempt in 0..MAX_COLLISION_RETRIES {
        let (id, number) = compute_next_id(workspace, Some(store), kind, digits, bump_above)
            .await
            .context("compute next-id under id-alloc lock")?;

        // Pre-write: file with `{ID}-` prefix must not exist on disk yet.
        if id_exists_on_disk(workspace, kind, &id).await? {
            tracing::warn!(
                kind = kind.prefix(),
                id = %id,
                attempt,
                "id-alloc pre-write collision — file already exists, bumping"
            );
            bump_above = Some(number);
            continue;
        }

        let artifact = build(&id, number).with_context(|| format!("build artifact for {id}"))?;
        // Sanity: caller must respect the passed-in id.
        if artifact.id != id {
            anyhow::bail!(
                "internal: build() returned mismatched id {:?}, expected {id}",
                artifact.id
            );
        }
        let rendered_body = artifact.body.clone();

        let ctx = MutationContext::new(workspace, store);
        let filepath = projection::create_artifact_with_projection(&ctx, &artifact)
            .await
            .with_context(|| format!("create projection for {id}"))?;

        // Post-write: re-confirm `{ID}-` prefix uniqueness. If a parallel
        // unlocked path slipped in between compute_next_id and projection,
        // the duplicate is visible now and we bump+retry.
        if !is_unique_owner_of_id(workspace, kind, &id, &filepath).await? {
            tracing::warn!(
                kind = kind.prefix(),
                id = %id,
                attempt,
                "id-alloc post-write collision — non-unique owner, removing and retrying"
            );
            // Best-effort rollback of the file we just wrote so we don't
            // leave an orphan when the retry bumps to a higher id. The
            // LanceDB row is left in place — reindex will reconcile on
            // next list operation. Caller could call `delete_artifact`
            // but at this scale it would race with the parallel writer.
            let _ = tokio::fs::remove_file(&filepath).await;
            bump_above = Some(number);
            continue;
        }

        return Ok(AllocatedArtifact {
            id,
            number,
            filepath,
            rendered_body,
        });
    }

    panic!(
        "id-alloc exceeded MAX_COLLISION_RETRIES ({MAX_COLLISION_RETRIES}) for kind {} — \
         the cross-worktree lock is not serializing allocations. Investigate \
         git-common-dir resolution and `.git/forgeplan/id-*.lock` files.",
        kind.prefix()
    );
}

/// Return `true` if any markdown file in `<workspace>/<kind-dir>/`
/// starts with `{id}-`. Case-insensitive on the id portion (mirror
/// `next_id` scanning).
async fn id_exists_on_disk(
    workspace: &Path,
    kind: &ArtifactKind,
    id: &str,
) -> anyhow::Result<bool> {
    let dir = workspace.join(kind.dir_name());
    if !dir.exists() {
        return Ok(false);
    }
    let upper_prefix = format!("{}-", id.to_uppercase());
    let mut rd = tokio::fs::read_dir(&dir).await?;
    while let Some(entry) = rd.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.to_uppercase().starts_with(&upper_prefix) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Confirm that the file we just wrote is the SOLE owner of `{id}-`
/// prefix in `<kind-dir>`. Used as post-write collision detection
/// (PROB-067 Option D).
async fn is_unique_owner_of_id(
    workspace: &Path,
    kind: &ArtifactKind,
    id: &str,
    our_path: &Path,
) -> anyhow::Result<bool> {
    let dir = workspace.join(kind.dir_name());
    if !dir.exists() {
        return Ok(true);
    }
    let upper_prefix = format!("{}-", id.to_uppercase());
    let our_name = our_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut rd = tokio::fs::read_dir(&dir).await?;
    while let Some(entry) = rd.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == our_name {
            continue;
        }
        if name.to_uppercase().starts_with(&upper_prefix) {
            return Ok(false);
        }
    }
    Ok(true)
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
}
