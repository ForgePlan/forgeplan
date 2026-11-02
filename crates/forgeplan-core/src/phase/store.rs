// Atomic read/write of `.forgeplan/state/<ID>.yaml` phase state files.
//
// Key invariants (learned from PRD-055 audit hardening):
// 1. Symlinked state dir is refused — prevents an attacker who can
//    write `.forgeplan/state/` from redirecting writes outside the
//    workspace.
// 2. Writes use tmp-file + rename for atomicity; a crash mid-write
//    leaves either the old content or the new content, never a
//    truncated file.
// 3. Both the file and parent directory are fsync'd — ext4/xfs can
//    lose the directory entry on a hard crash otherwise.
// 4. Missing state file is NOT an error — returns `Ok(None)`. FR-012.
// 5. Invalid or unsupported state is reported without modifying the file.
//    Mutation paths must not replace state they could not read.

use chrono::Utc;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

use super::{
    MAX_HISTORY_ENTRIES, Phase, PhaseState, PhaseTransition, initial_state, state_dir, state_path,
    truncate_reason, validate_artifact_id, validate_canonical_key,
};

/// Maximum bytes read from a state file before parsing. Prevents an
/// attacker or corrupted writer from forcing us to allocate hundreds
/// of MB for parsing. Round 1 audit M1. ~1 MiB is ample — history is
/// capped at 1024 entries (see MAX_HISTORY_ENTRIES).
const MAX_STATE_FILE_BYTES: u64 = 1_048_576;

/// Current schema version. `read_phase` refuses files with
/// `schema_version` greater than this — protects forward-compat
/// from a newer writer silently losing fields when read by this
/// binary. Round 2 audit M-logic.
const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Diagnosable failures reading an existing phase sidecar.
#[derive(Debug, thiserror::Error)]
pub enum PhaseReadError {
    #[error("phase state for {canonical_key} is a symlink")]
    Symlink { canonical_key: String },
    #[error("phase state for {canonical_key} exceeds {max_bytes} bytes")]
    Oversized { canonical_key: String, max_bytes: u64 },
    #[error("phase state for {canonical_key} is malformed: {source}")]
    Malformed {
        canonical_key: String,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("phase state for {canonical_key} uses unsupported schema {version} (maximum {max_supported})")]
    FutureSchema {
        canonical_key: String,
        version: u64,
        max_supported: u32,
    },
}

/// Monotonic counter for tmp filenames. Combined with pid + nanos
/// eliminates Round 2 H-sec #3 collision window (two advances in
/// the same nanosecond within one process getting identical tmp).
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Read phase state for an artifact. Returns `Ok(None)` if the file does
/// not exist (FR-012 — missing state is not an error). Malformed, oversized
/// and future-schema files return a [`PhaseReadError`] and remain untouched.
///
/// state written before the key was escaped keeps the raw key name. Alias-
/// named legacy files are read only when a complete workspace scan proves
/// that the alias has exactly one owner; reads never migrate sidecars.
///
/// Rejects a key that cannot be a key up front: the name is escaped into
/// exactly one path component, so a tampered key cannot escape the
/// workspace (audit Round 1 C-sec #1).
pub async fn read_phase(
    workspace: &Path,
    canonical_key: &str,
) -> anyhow::Result<Option<PhaseState>> {
    validate_canonical_key(canonical_key)?;

    let dir = state_dir(workspace);
    let path =
        crate::artifact::sidecar::resolve_sidecar_path(&dir, canonical_key, canonical_key, "yaml")
            .await.map_err(sidecar_error)?;
    read_phase_at(&path, canonical_key).await
}

/// Read the phase state of a document named by its canonical key, finding an
/// alias-named legacy state only after the full workspace scan proves unique
/// ownership. This is a read-only lookup; it does not rename the file.
pub async fn read_phase_for(
    workspace: &Path,
    canonical_key: &str,
    display_alias: &str,
) -> anyhow::Result<Option<PhaseState>> {
    validate_canonical_key(canonical_key)?;

    let dir = state_dir(workspace);
    let path =
        crate::artifact::sidecar::resolve_sidecar_path(&dir, canonical_key, display_alias, "yaml")
            .await.map_err(sidecar_error)?;
    read_phase_at(&path, canonical_key).await
}

fn sidecar_error(error: std::io::Error) -> anyhow::Error {
    if let Some(resolve) = error.get_ref()
        .and_then(|inner| inner.downcast_ref::<crate::artifact::canonical_key::ResolveError>())
    {
        return resolve.clone().into();
    }
    error.into()
}

/// Read one state file, guarding against a symlinked target and an
/// oversized one.
async fn read_phase_at(path: &Path, canonical_key: &str) -> anyhow::Result<Option<PhaseState>> {
    let meta = match tokio::fs::symlink_metadata(path).await {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    if meta.file_type().is_symlink() {
        return Err(PhaseReadError::Symlink {
            canonical_key: canonical_key.to_owned(),
        }.into());
    }
    if meta.len() > MAX_STATE_FILE_BYTES {
        return Err(PhaseReadError::Oversized {
            canonical_key: canonical_key.to_owned(),
            max_bytes: MAX_STATE_FILE_BYTES,
        }.into());
    }

    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let mut bytes = Vec::new();
    file.take(MAX_STATE_FILE_BYTES + 1).read_to_end(&mut bytes).await?;
    if bytes.len() as u64 > MAX_STATE_FILE_BYTES {
        return Err(PhaseReadError::Oversized {
            canonical_key: canonical_key.to_owned(),
            max_bytes: MAX_STATE_FILE_BYTES,
        }.into());
    }
    let malformed = |source| PhaseReadError::Malformed {
        canonical_key: canonical_key.to_owned(),
        source,
    };
    let value: serde_yaml::Value = serde_yaml::from_slice(&bytes).map_err(malformed)?;
    if let Some(version) = value.get("schema_version").and_then(serde_yaml::Value::as_u64)
        && version > u64::from(CURRENT_SCHEMA_VERSION)
    {
        return Err(PhaseReadError::FutureSchema {
            canonical_key: canonical_key.to_owned(),
            version,
            max_supported: CURRENT_SCHEMA_VERSION,
        }.into());
    }
    let state: PhaseState = serde_yaml::from_value(value).map_err(malformed)?;
    Ok(Some(state))
}

/// Migrate a legacy alias-named phase file only after revalidating its unique
/// owner under the workspace mutation lock. The keyed name is published with
/// `hard_link`, which fails rather than replacing a concurrently created file.
pub async fn migrate_legacy_state(
    workspace: &Path,
    legacy_id: &str,
    canonical_key: &str,
) -> anyhow::Result<bool> {
    migrate_legacy_state_held(workspace, None, legacy_id, canonical_key).await
}

/// The same migration for a caller that ALREADY holds the workspace lock.
///
/// The CLI takes the lock before it resolves a phase command, and the lock is
/// not reentrant: acquiring it again inside the migration deadlocked the
/// command until its 30 s timeout. Passing the held guard keeps one lock for
/// the whole read-modify-write.
pub async fn migrate_legacy_state_held(
    workspace: &Path,
    held: Option<&crate::workspace::lock::WorkspaceLock>,
    legacy_id: &str,
    canonical_key: &str,
) -> anyhow::Result<bool> {
    let _acquired;
    let _lock = match held {
        Some(guard) => guard,
        None => {
            _acquired = crate::workspace::lock::acquire_workspace_lock(workspace).await?;
            &_acquired
        }
    };
    let candidates = crate::artifact::documents::workspace_candidates(workspace).await?;
    if let Err(error @ crate::artifact::canonical_key::ResolveError::ConflictingCanonicalKey { .. }) =
        crate::artifact::canonical_key::resolve_artifact(canonical_key, &candidates)
    {
        return Err(error.into());
    }
    let dir = state_dir(workspace);
    let target = state_path(workspace, canonical_key);
    if legacy_id == canonical_key || tokio::fs::try_exists(&target).await? {
        return Ok(false);
    }
    let resolved = crate::artifact::canonical_key::resolve_artifact(legacy_id, &candidates)?;
    let Some(resolved) = resolved else {
        return Ok(false);
    };
    if resolved.identity.canonical_key != canonical_key {
        anyhow::bail!(
            "legacy phase alias {legacy_id:?} does not uniquely identify canonical key {canonical_key:?}"
        );
    }

    for legacy in
        crate::artifact::sidecar::legacy_sidecar_paths(&dir, canonical_key, legacy_id, "yaml")
    {
        if legacy == target || !tokio::fs::try_exists(&legacy).await? {
            continue;
        }
        let meta = tokio::fs::symlink_metadata(&legacy).await?;
        if meta.file_type().is_symlink() {
            anyhow::bail!("refusing to migrate symlinked phase state {}", legacy.display());
        }
        match tokio::fs::hard_link(&legacy, &target).await {
            Ok(()) => {
                tokio::fs::remove_file(&legacy).await?;
                return Ok(true);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(false),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(false)
}

/// Atomically write phase state to disk. Creates the state directory
/// if needed, refuses to write through a symlinked state dir or target,
/// writes to a per-process-unique tmp, renames, and fsync's file +
/// parent directory.
pub async fn write_phase(workspace: &Path, state: &PhaseState) -> anyhow::Result<()> {
    validate_artifact_id(&state.artifact_id)?;

    let dir = state_dir(workspace);

    // Symlink guard on dir (audit H-2 pattern from PRD-055 hotfix).
    if dir.exists() {
        let meta = tokio::fs::symlink_metadata(&dir).await?;
        if meta.file_type().is_symlink() {
            anyhow::bail!(
                "phase state directory {} is a symlink — refusing to write",
                dir.display()
            );
        }
    }
    tokio::fs::create_dir_all(&dir).await?;

    // The state file is named after the document's canonical key: the
    // identity, not the alias a second document may share. A state written
    // before the key was recorded keeps the display id in its name.
    let file_name = state
        .canonical_key
        .as_deref()
        .unwrap_or(state.artifact_id.as_str());
    validate_canonical_key(file_name)?;
    let target = state_path(workspace, file_name);

    // Symlink guard on the target file too: an attacker with write
    // access to `.forgeplan/state/` could have pre-planted a symlink
    // to `/etc/passwd`. rename-over-symlink behavior is platform-
    // dependent; refuse up-front. Round 1 audit C-sec #2.
    if tokio::fs::try_exists(&target).await.unwrap_or(false)
        && let Ok(meta) = tokio::fs::symlink_metadata(&target).await
        && meta.file_type().is_symlink()
    {
        anyhow::bail!(
            "phase state target {} is a symlink — refusing to overwrite",
            target.display()
        );
    }

    let yaml = serde_yaml::to_string(state)?;

    // Per-process + nanos + monotonic-counter tmp name so concurrent
    // `advance_phase` calls on the same artifact never collide on the
    // tmp path. Round 2 audit H-sec #3 / H-logic — nanos alone can
    // collide on clocks with sub-nanosecond equality in the same tokio
    // scheduler tick; AtomicU64 closes the window. rename() remains
    // atomic on POSIX — last rename wins, but neither writer is
    // partially visible.
    let pid = std::process::id();
    let nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let counter = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!(
        ".{}.yaml.{}.{}.{}.tmp",
        state.artifact_id, pid, nanos, counter
    ));

    // `create_new(true)` on the (near-unique) tmp name means a hostile
    // symlink placed at that exact path would cause EEXIST rather than
    // being silently followed.
    let mut f = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&tmp)
        .await?;
    f.write_all(yaml.as_bytes()).await?;
    f.sync_data().await?;
    drop(f);

    tokio::fs::rename(&tmp, &target).await?;

    // Parent dir fsync so the directory entry for the new file is
    // durable on ext4/xfs. Round 2 audit H-logic / H-rust — move the
    // blocking `std::fs::File::open` inside the spawn_blocking closure
    // so the async worker thread is never blocked by filesystem IO,
    // not even for directory open which can stall on NFS/contended
    // inodes. Propagate failures via tracing::warn (Round 1 H4).
    let dir_for_task = dir.clone();
    let dir_for_log = dir.clone();
    let join = tokio::task::spawn_blocking(move || {
        std::fs::File::open(&dir_for_task).and_then(|h| h.sync_all())
    })
    .await;
    match join {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!(
            dir = %dir_for_log.display(),
            error = %e,
            "parent-dir fsync failed — state write is durable in page cache but not on disk"
        ),
        Err(e) => tracing::warn!(
            dir = %dir_for_log.display(),
            error = %e,
            "parent-dir fsync task panicked"
        ),
    }
    Ok(())
}

/// First-time initialization: write a fresh Shape state for an artifact.
/// Idempotent — if a state file already exists this returns the existing
/// state without overwriting (callers who need to force a reset should
/// delete the file first).
pub async fn initialize_phase(
    workspace: &Path,
    artifact_id: &str,
    reason: Option<String>,
) -> anyhow::Result<PhaseState> {
    validate_artifact_id(artifact_id)?;
    if let Some(existing) = read_phase(workspace, artifact_id).await? {
        return Ok(existing);
    }
    let fresh = initial_state(artifact_id, truncate_reason(reason));
    write_phase(workspace, &fresh).await?;
    Ok(fresh)
}

/// Advance the phase marker for an artifact. If no state file exists,
/// one is created first (at `Shape`) and then advanced. Append-only
/// history guarantees a record of each transition.
///
/// Returns the new state after the transition. Does NOT validate that
/// the target phase follows the "canonical" order — advisory layer
/// allows out-of-order advances (e.g. direct jump to `done` from manual
/// override). Full-enforcement PRD (follow-up) will add validation.
pub async fn advance_phase(
    workspace: &Path,
    artifact_id: &str,
    to: Phase,
    reason: Option<String>,
) -> anyhow::Result<PhaseState> {
    advance_phase_inner(workspace, artifact_id, artifact_id, to, reason, false).await
}

/// Move the phase anywhere, including backwards — for deliberate correction.
///
/// #330. The checked [`advance_phase`] refuses a step down the ladder, because
/// its own name promises it will not take one. An operator running
/// `forgeplan phase-advance --to shape` on an artifact that reached `done` is
/// making a decision, not tripping over automation, and that path stays open.
pub async fn advance_phase_unchecked(
    workspace: &Path,
    artifact_id: &str,
    to: Phase,
    reason: Option<String>,
) -> anyhow::Result<PhaseState> {
    advance_phase_inner(workspace, artifact_id, artifact_id, to, reason, true).await
}

/// Advance the phase of a document whose canonical key is known.
///
/// The state file is named after the canonical key; the state itself keeps
/// the display id — the name a reader knows the document by.
pub async fn advance_phase_for(
    workspace: &Path,
    canonical_key: &str,
    display_id: &str,
    to: Phase,
    reason: Option<String>,
) -> anyhow::Result<PhaseState> {
    advance_phase_inner(
        workspace,
        canonical_key,
        display_id,
        to,
        reason,
        false,
    )
    .await
}

/// Move the phase of a document whose canonical key is known, in either
/// direction — the deliberate correction path.
pub async fn advance_phase_unchecked_for(
    workspace: &Path,
    canonical_key: &str,
    display_id: &str,
    to: Phase,
    reason: Option<String>,
) -> anyhow::Result<PhaseState> {
    advance_phase_inner(
        workspace,
        canonical_key,
        display_id,
        to,
        reason,
        true,
    )
    .await
}

async fn advance_phase_inner(
    workspace: &Path,
    canonical_key: &str,
    artifact_id: &str,
    to: Phase,
    reason: Option<String>,
    allow_regression: bool,
) -> anyhow::Result<PhaseState> {
    validate_artifact_id(artifact_id)?;
    validate_canonical_key(canonical_key)?;

    // Track whether state existed on disk — if no, we synthesized the
    // initial Shape in-memory; even when `to == Shape` we must persist
    // so subsequent reads see the materialized state (Round 2 audit
    // H-logic #2: the silent no-persist bug previously left disk empty
    // after "advance from missing" and confused callers).
    let (mut state, was_missing) = match read_phase_for(workspace, canonical_key, artifact_id).await? {
        Some(s) => (s, false),
        None => (initial_state(artifact_id, None), true),
    };
    // The state names the document by its display id and carries the key the
    // file is named after.
    state.artifact_id = artifact_id.to_string();
    state.canonical_key = Some(canonical_key.to_string());

    let from = state.current_phase;

    // #330. There was no monotonicity guard at all, and MCP `forgeplan_validate`
    // calls this with `Phase::Validate` on every PASS — so validating an
    // artifact that had already reached `done` walked it back to `validate`,
    // and `forgeplan_health` then reported a phase mismatch the artifact did
    // not have until someone checked it. A function named `advance` that
    // silently reverses is the same defect class as a gate that reports a
    // result it never computed.
    //
    // Refusing leaves state untouched and tells the caller why; the auto-advance
    // path in MCP treats phase tracking as advisory and logs the refusal without
    // failing the tool call.
    if !allow_regression && to.rank() < from.rank() {
        anyhow::bail!(
            "Phase for {artifact_id} is already `{}`; refusing to move back to `{}`\nFix: forgeplan phase-advance {artifact_id} --to {}",
            from.as_str(),
            to.as_str(),
            to.as_str()
        );
    }

    // Skip recording a no-op transition (e.g. double-call of auto-advance).
    if from == to {
        if was_missing {
            // First-time materialization — persist even though the
            // logical transition is a no-op.
            write_phase(workspace, &state).await?;
        }
        return Ok(state);
    }

    let now = Utc::now();
    state.history.push(PhaseTransition {
        from: Some(from),
        to,
        at: now,
        reason: truncate_reason(reason),
    });

    // Cap history to prevent unbounded disk growth from a
    // runaway-agent loop (audit Round 1 H1). FIFO drop — keep the
    // initial entries (likely auto-init) and the most recent ones.
    if state.history.len() > MAX_HISTORY_ENTRIES {
        let excess = state.history.len() - MAX_HISTORY_ENTRIES;
        state.history.drain(1..=excess);
    }

    state.current_phase = to;
    state.advanced_at = now;

    write_phase(workspace, &state).await?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn ws(tmp: &TempDir) -> std::path::PathBuf {
        let p = tmp.path().join(".forgeplan");
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[tokio::test]
    async fn read_missing_returns_none() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let got = read_phase(&ws, "PRD-999").await.unwrap();
        assert!(got.is_none(), "missing state must be Ok(None), not error");
        assert!(!state_dir(&ws).exists());
    }

    #[tokio::test]
    async fn write_then_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let s = initial_state("PRD-100", Some("seed".into()));
        write_phase(&ws, &s).await.unwrap();
        let back = read_phase(&ws, "PRD-100").await.unwrap().unwrap();
        assert_eq!(back, s);
    }

    #[tokio::test]
    async fn initialize_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let first = initialize_phase(&ws, "PRD-A", Some("first".into()))
            .await
            .unwrap();
        let again = initialize_phase(&ws, "PRD-A", Some("SHOULD BE IGNORED".into()))
            .await
            .unwrap();
        assert_eq!(first, again, "second init must return existing state");
    }

    #[tokio::test]
    async fn advance_appends_history_and_updates_current() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        initialize_phase(&ws, "PRD-B", None).await.unwrap();

        let s1 = advance_phase(&ws, "PRD-B", Phase::Validate, Some("validate PASS".into()))
            .await
            .unwrap();
        assert_eq!(s1.current_phase, Phase::Validate);
        assert_eq!(s1.history.len(), 2); // init + validate

        let s2 = advance_phase(&ws, "PRD-B", Phase::Adi, None).await.unwrap();
        assert_eq!(s2.current_phase, Phase::Adi);
        assert_eq!(s2.history.len(), 3);
        assert_eq!(s2.history.last().unwrap().from, Some(Phase::Validate));
    }

    #[tokio::test]
    async fn advance_from_missing_state_initializes_then_advances() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        // No prior state; advance directly.
        let s = advance_phase(&ws, "PRD-C", Phase::Code, Some("jump".into()))
            .await
            .unwrap();
        assert_eq!(s.current_phase, Phase::Code);
        // History: initial Shape synthesized + Code transition.
        assert!(!s.history.is_empty());
        assert_eq!(s.history.last().unwrap().to, Phase::Code);
    }

    #[tokio::test]
    async fn advance_no_op_when_target_equals_current() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        initialize_phase(&ws, "PRD-D", None).await.unwrap();
        let before = read_phase(&ws, "PRD-D").await.unwrap().unwrap();
        let after = advance_phase(&ws, "PRD-D", Phase::Shape, Some("noop".into()))
            .await
            .unwrap();
        assert_eq!(before, after, "no-op advance must not mutate state");
    }

    #[tokio::test]
    async fn corrupt_state_is_diagnosed_and_not_overwritten() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = state_dir(&ws);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("prd-example.yaml");
        let bytes = b"{ not valid yaml :::";
        std::fs::write(&path, bytes).unwrap();

        for err in [
            read_phase(&ws, "prd-example").await.unwrap_err(),
            read_phase_for(&ws, "prd-example", "PRD-001").await.unwrap_err(),
            advance_phase_for(&ws, "prd-example", "PRD-001", Phase::Code, None)
                .await.unwrap_err(),
        ] {
            assert!(matches!(err.downcast_ref::<PhaseReadError>(),
                Some(PhaseReadError::Malformed { canonical_key, .. }) if canonical_key == "prd-example"));
        }
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let names: Vec<_> = std::fs::read_dir(&dir).unwrap()
            .map(|entry| entry.unwrap().file_name()).collect();
        assert_eq!(names, vec![path.file_name().unwrap().to_os_string()]);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn read_refuses_symlink_without_touching_it_or_target() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = state_dir(&ws);
        std::fs::create_dir_all(&dir).unwrap();
        let target = tmp.path().join("target");
        std::fs::write(&target, b"private").unwrap();
        let path = dir.join("prd-symlink.yaml");
        std::os::unix::fs::symlink(&target, &path).unwrap();
        let err = read_phase(&ws, "prd-symlink").await.unwrap_err();
        assert!(matches!(err.downcast_ref::<PhaseReadError>(),
            Some(PhaseReadError::Symlink { canonical_key }) if canonical_key == "prd-symlink"));
        assert_eq!(std::fs::read_link(&path).unwrap(), target);
        assert_eq!(std::fs::read(&target).unwrap(), b"private");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlinked_state_dir_is_refused() {
        // Audit hardening parity with PRD-055: writing through a symlink
        // would let an attacker redirect writes outside the workspace.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, ws.join("state")).unwrap();

        let s = initial_state("PRD-SEC", None);
        let err = write_phase(&ws, &s).await.unwrap_err();
        assert!(
            err.to_string().contains("symlink"),
            "symlinked state dir must be refused: {err}"
        );
    }

    #[tokio::test]
    async fn history_is_append_only_not_truncated() {
        // Regression guard: advancing many times must keep full history,
        // not replace on each write.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        initialize_phase(&ws, "PRD-H", None).await.unwrap();
        for phase in [
            Phase::Validate,
            Phase::Adi,
            Phase::Code,
            Phase::Test,
            Phase::Audit,
            Phase::Evidence,
            Phase::Done,
        ] {
            advance_phase(&ws, "PRD-H", phase, None).await.unwrap();
        }
        let s = read_phase(&ws, "PRD-H").await.unwrap().unwrap();
        assert_eq!(s.current_phase, Phase::Done);
        assert_eq!(s.history.len(), 8, "init + 7 advances = 8 entries");
    }

    // ── Audit Round 1 regression tests (hotfix) ──────────────────

    #[tokio::test]
    async fn read_rejects_path_traversal_id() {
        // Audit C-sec #1: an attacker-controlled id with path traversal
        // must be refused up-front — state_path must not escape the
        // workspace even if such an id somehow reached the module.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let err = read_phase(&ws, "../../etc/passwd").await.unwrap_err();
        assert!(
            err.to_string().contains("invalid character") || err.to_string().contains("must start"),
            "traversal id must be rejected: {err}"
        );
    }

    #[tokio::test]
    async fn advance_rejects_path_traversal_id() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let err = advance_phase(&ws, "../evil", Phase::Done, None)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("must start with an ASCII letter") || msg.contains("invalid character"),
            "id must be rejected by the validator: {msg}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn write_refuses_symlinked_target_file() {
        // Audit C-sec #2: if an attacker with write access to
        // `.forgeplan/state/` pre-plants a symlink at the target path,
        // a subsequent write must refuse — do not allow clobbering
        // arbitrary files via symlink.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = state_dir(&ws);
        std::fs::create_dir_all(&dir).unwrap();

        // Plant a symlink at state/PRD-SL.yaml pointing somewhere else.
        let outside = tmp.path().join("victim.txt");
        std::fs::write(&outside, b"sensitive").unwrap();
        std::os::unix::fs::symlink(&outside, dir.join("PRD-SL.yaml")).unwrap();

        let s = initial_state("PRD-SL", None);
        let err = write_phase(&ws, &s).await.unwrap_err();
        assert!(
            err.to_string().contains("symlink"),
            "symlinked target must be refused: {err}"
        );
        // Victim file untouched.
        let still = std::fs::read_to_string(&outside).unwrap();
        assert_eq!(still, "sensitive");
    }

    /// #330. MCP `forgeplan_validate` auto-advances to `Validate` on every
    /// PASS. Run against an artifact that already reached `done`, that walked
    /// the phase backwards and made `forgeplan_health` report a mismatch the
    /// artifact did not have until someone validated it.
    #[tokio::test]
    async fn validation_of_a_finished_artifact_does_not_walk_the_phase_back() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        initialize_phase(&ws, "PRD-MONO", None).await.unwrap();
        advance_phase_unchecked(&ws, "PRD-MONO", Phase::Done, None)
            .await
            .unwrap();

        let err = advance_phase(&ws, "PRD-MONO", Phase::Validate, Some("auto".into()))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("refusing to move back"),
            "the refusal must say what it refused, got: {err}"
        );

        let after = read_phase(&ws, "PRD-MONO").await.unwrap().unwrap();
        assert_eq!(
            after.current_phase,
            Phase::Done,
            "a refused transition must leave state untouched"
        );
    }

    /// Forward motion and re-stating the current phase both stay legal — the
    /// guard must not turn into a tripwire on the normal path.
    #[tokio::test]
    async fn forward_and_no_op_transitions_are_unaffected() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        initialize_phase(&ws, "PRD-FWD", None).await.unwrap();

        advance_phase(&ws, "PRD-FWD", Phase::Code, None)
            .await
            .expect("forward jump is allowed");
        advance_phase(&ws, "PRD-FWD", Phase::Code, None)
            .await
            .expect("re-stating the current phase is a no-op, not a regression");

        let s = read_phase(&ws, "PRD-FWD").await.unwrap().unwrap();
        assert_eq!(s.current_phase, Phase::Code);
    }

    /// The deliberate operator path keeps both directions — that is the whole
    /// reason the guard lives on `advance_phase` and not inside `write_phase`.
    #[tokio::test]
    async fn the_explicit_path_may_still_correct_a_phase_downward() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        initialize_phase(&ws, "PRD-FIX", None).await.unwrap();
        advance_phase_unchecked(&ws, "PRD-FIX", Phase::Done, None)
            .await
            .unwrap();

        advance_phase_unchecked(&ws, "PRD-FIX", Phase::Shape, Some("mis-marked".into()))
            .await
            .expect("an operator correcting a mistake is not a regression");

        let s = read_phase(&ws, "PRD-FIX").await.unwrap().unwrap();
        assert_eq!(s.current_phase, Phase::Shape);
    }

    #[tokio::test]
    async fn history_is_capped_fifo() {
        // Audit Round 1 H1: runaway loop must not balloon history.
        // Advance back-and-forth many times; assert cap respected.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        initialize_phase(&ws, "PRD-CAP", None).await.unwrap();

        for i in 0..(MAX_HISTORY_ENTRIES + 100) {
            let p = if i % 2 == 0 { Phase::Code } else { Phase::Test };
            // #330: this oscillates on purpose to exercise the FIFO cap, which
            // is a different question from whether automation may reverse a
            // phase. The unchecked entry point is the honest one here.
            advance_phase_unchecked(&ws, "PRD-CAP", p, None)
                .await
                .unwrap();
        }
        let s = read_phase(&ws, "PRD-CAP").await.unwrap().unwrap();
        assert!(
            s.history.len() <= MAX_HISTORY_ENTRIES,
            "history should be capped at {MAX_HISTORY_ENTRIES}, got {}",
            s.history.len()
        );
        // First entry should still be the Shape init (FIFO drops 1..=excess,
        // not index 0 — preserves provenance of initial state).
        assert_eq!(s.history[0].to, Phase::Shape);
    }

    #[tokio::test]
    async fn reason_is_truncated_on_write() {
        // Audit Round 1 H3 / related to H-sec #2: reason stored on disk
        // is bounded; runaway agent cannot dump MB into a single entry.
        use super::super::MAX_REASON_LEN;

        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        initialize_phase(&ws, "PRD-R", None).await.unwrap();

        let huge = "x".repeat(MAX_REASON_LEN * 10);
        advance_phase(&ws, "PRD-R", Phase::Validate, Some(huge))
            .await
            .unwrap();

        let s = read_phase(&ws, "PRD-R").await.unwrap().unwrap();
        let stored = s.history.last().unwrap().reason.as_ref().unwrap();
        assert!(stored.len() <= MAX_REASON_LEN);
    }

    // ── Audit Round 2 regression tests ──────────────────────────

    #[tokio::test]
    async fn advance_from_missing_with_target_shape_persists() {
        // Audit Round 2 H-logic #2: previously `advance_phase` on a
        // missing state with `to=Shape` hit the `from==to` early return
        // and never wrote to disk. Next read saw None. Bug.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let s = advance_phase(&ws, "PRD-MS", Phase::Shape, Some("manual".into()))
            .await
            .unwrap();
        assert_eq!(s.current_phase, Phase::Shape);

        // Disk state must now be persisted — next read returns Some, not None.
        let back = read_phase(&ws, "PRD-MS").await.unwrap();
        assert!(
            back.is_some(),
            "advance from missing + to=Shape must persist state"
        );
        assert_eq!(back.unwrap().current_phase, Phase::Shape);
    }

    #[tokio::test]
    async fn concurrent_advances_all_succeed() {
        // Audit Round 2 H-sec #3: tmp filename uniqueness under
        // concurrent calls in the same tokio runtime tick. The
        // AtomicU64 counter (vs just nanos) should guarantee no two
        // tmp paths ever collide.
        use futures::future::join_all;
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        initialize_phase(&ws, "PRD-CC", None).await.unwrap();

        let mut tasks = Vec::new();
        for i in 0..16 {
            let ws = ws.clone();
            let phase = if i % 2 == 0 { Phase::Code } else { Phase::Test };
            tasks.push(tokio::spawn(async move {
                // #330: the question here is tmp-filename collision under
                // concurrent writes, not whether a phase may move backwards.
                // The oscillation between Code and Test is just a way to make
                // every task write; with the monotonicity guard in place the
                // losers of that race are refused, which would turn a
                // write-safety test into an ordering test by accident.
                advance_phase_unchecked(&ws, "PRD-CC", phase, Some(format!("concurrent-{i}"))).await
            }));
        }
        let results = join_all(tasks).await;
        for r in results {
            r.expect("task panicked").expect("advance_phase failed");
        }
        // State exists and is one of the expected phases.
        let s = read_phase(&ws, "PRD-CC").await.unwrap().unwrap();
        assert!(matches!(s.current_phase, Phase::Code | Phase::Test));
    }

    /// #330, the half the rewrite above would otherwise have dropped: under
    /// concurrency the CHECKED entry point must refuse cleanly rather than
    /// corrupt state or panic. Sixteen tasks all advancing forward to the same
    /// phase — every one is either a real transition or a no-op, none is a
    /// regression, so all must succeed and the state must land exactly there.
    #[tokio::test]
    async fn concurrent_checked_advances_are_safe() {
        use futures::future::join_all;
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        initialize_phase(&ws, "PRD-CCC", None).await.unwrap();

        let mut tasks = Vec::new();
        for i in 0..16 {
            let ws = ws.clone();
            tasks.push(tokio::spawn(async move {
                advance_phase(&ws, "PRD-CCC", Phase::Code, Some(format!("fwd-{i}"))).await
            }));
        }
        for r in join_all(tasks).await {
            r.expect("task panicked")
                .expect("a forward transition must never be refused");
        }

        let s = read_phase(&ws, "PRD-CCC").await.unwrap().unwrap();
        assert_eq!(s.current_phase, Phase::Code);
    }

    #[tokio::test]
    async fn oversized_and_future_state_are_diagnosed_without_mutation() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = state_dir(&ws);
        std::fs::create_dir_all(&dir).unwrap();
        let oversized = vec![b'x'; MAX_STATE_FILE_BYTES as usize + 1];
        let future = b"artifact_id: PRD-FV\nworkflow_type: future_workflow\ncurrent_phase: future_phase\nschema_version: 9999\n";
        let big_path = dir.join("prd-large.yaml");
        let future_path = dir.join("prd-future.yaml");
        std::fs::write(&big_path, &oversized).unwrap();
        std::fs::write(&future_path, future).unwrap();

        let err = read_phase_for(&ws, "prd-large", "PRD-001").await.unwrap_err();
        assert!(matches!(err.downcast_ref::<PhaseReadError>(),
            Some(PhaseReadError::Oversized { canonical_key, max_bytes })
                if canonical_key == "prd-large" && *max_bytes == MAX_STATE_FILE_BYTES));
        let err = read_phase_for(&ws, "prd-future", "PRD-002").await.unwrap_err();
        assert!(matches!(err.downcast_ref::<PhaseReadError>(),
            Some(PhaseReadError::FutureSchema { canonical_key, version: 9999, .. })
                if canonical_key == "prd-future"));
        assert!(advance_phase_for(&ws, "prd-large", "PRD-001", Phase::Code, None).await.is_err());
        assert!(advance_phase_for(&ws, "prd-future", "PRD-002", Phase::Code, None).await.is_err());
        assert_eq!(std::fs::read(&big_path).unwrap(), oversized);
        assert_eq!(std::fs::read(&future_path).unwrap(), future);
        let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap()
            .map(|entry| entry.unwrap().file_name()).collect();
        names.sort();
        assert_eq!(names, vec![future_path.file_name().unwrap().to_os_string(), big_path.file_name().unwrap().to_os_string()]);
    }

    // ── ADR-030 — the state file is addressed by the canonical key ────────

    #[tokio::test]
    async fn two_documents_sharing_an_alias_keep_their_own_state() {
        // The state file is named after the document's key, so two
        // documents that share a display alias never share a state file.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);

        advance_phase_for(&ws, "prd-first-document", "PRD-001", Phase::Shape, None)
            .await
            .unwrap();
        advance_phase_for(&ws, "prd-second-document", "PRD-001", Phase::Validate, None)
            .await
            .unwrap();

        let dir = state_dir(&ws);
        assert!(dir.join("prd-first-document.yaml").exists());
        assert!(dir.join("prd-second-document.yaml").exists());
        assert!(
            !dir.join("PRD-001.yaml").exists(),
            "the shared alias names no state file"
        );

        let first = read_phase_for(&ws, "prd-first-document", "PRD-001")
            .await
            .unwrap()
            .unwrap();
        let second = read_phase_for(&ws, "prd-second-document", "PRD-001")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first.current_phase, Phase::Shape);
        assert_eq!(second.current_phase, Phase::Validate);
    }

    #[tokio::test]
    async fn a_read_does_not_attribute_or_migrate_an_alias_state_file() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = state_dir(&ws);
        std::fs::create_dir_all(&dir).unwrap();
        let legacy = "artifact_id: PRD-001\nworkflow_type: greenfield\ncurrent_phase: validate\n\
             advanced_at: 2026-04-18T00:00:00Z\nhistory: []\nschema_version: 1\n";
        std::fs::write(dir.join("PRD-001.yaml"), legacy).unwrap();

        let found = read_phase_for(&ws, "prd-auth-system", "PRD-001")
            .await
            .unwrap();
        assert!(found.is_none());
        assert_eq!(
            std::fs::read_to_string(dir.join("PRD-001.yaml")).unwrap(),
            legacy
        );
        assert!(!dir.join("prd-auth-system.yaml").exists());
    }

    #[tokio::test]
    async fn legacy_phase_migration_refuses_shared_alias_without_mutation() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = state_dir(&ws);
        std::fs::create_dir_all(&dir).unwrap();
        let prds = ws.join("prds");
        std::fs::create_dir_all(&prds).unwrap();
        for (filename, slug) in [
            ("alpha.md", "prd-alpha-k11111111111111111111111111111111"),
            ("beta.md", "prd-beta-k22222222222222222222222222222222"),
        ] {
            std::fs::write(
                prds.join(filename),
                format!("---\nid: PRD-001\nslug: {slug}\ntitle: {slug}\nkind: prd\n---\n"),
            )
            .unwrap();
        }
        let legacy_path = dir.join("PRD-001.yaml");
        let bytes = b"legacy-state-bytes";
        std::fs::write(&legacy_path, bytes).unwrap();

        assert!(
            migrate_legacy_state(
                &ws,
                "PRD-001",
                "prd-alpha-k11111111111111111111111111111111",
            )
            .await
            .is_err()
        );
        assert_eq!(std::fs::read(&legacy_path).unwrap(), bytes);
        assert!(
            !state_path(&ws, "prd-alpha-k11111111111111111111111111111111").exists()
        );
    }

    #[tokio::test]
    async fn legacy_phase_migration_moves_only_a_uniquely_owned_alias() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = state_dir(&ws);
        std::fs::create_dir_all(&dir).unwrap();
        let prds = ws.join("prds");
        std::fs::create_dir_all(&prds).unwrap();
        std::fs::write(
            prds.join("alpha.md"),
            "---\nid: PRD-001\nslug: prd-alpha-k11111111111111111111111111111111\n\
             title: Alpha\nkind: prd\n---\n",
        )
        .unwrap();
        let legacy_path = dir.join("PRD-001.yaml");
        let bytes = b"legacy-state-bytes";
        std::fs::write(&legacy_path, bytes).unwrap();
        let canonical_key = "prd-alpha-k11111111111111111111111111111111";

        assert!(
            migrate_legacy_state(&ws, "PRD-001", canonical_key)
                .await
                .unwrap()
        );
        assert_eq!(std::fs::read(state_path(&ws, canonical_key)).unwrap(), bytes);
        assert!(!legacy_path.exists());
    }

    #[tokio::test]
    async fn a_state_file_name_escapes_the_key_it_is_named_after() {
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);

        advance_phase_for(
            &ws,
            "forgeplan/plugins/x--PRD-074",
            "PRD-074",
            Phase::Shape,
            None,
        )
        .await
        .unwrap();

        let dir = state_dir(&ws);
        assert!(
            dir.join("forgeplan%2Fplugins%2Fx--PRD-074.yaml").exists(),
            "the key is escaped into one file name"
        );
        assert!(
            !dir.join("forgeplan").exists(),
            "no directory is created for the separator"
        );
    }
}
