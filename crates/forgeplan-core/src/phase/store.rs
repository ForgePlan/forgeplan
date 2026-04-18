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
// 5. Corrupt state file (YAML parse error) is logged and treated as
//    `None` rather than propagating — phase tracking is advisory and
//    must never break the existing tool-call flow.

use chrono::Utc;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::AsyncWriteExt;

use super::{
    MAX_HISTORY_ENTRIES, Phase, PhaseState, PhaseTransition, initial_state, state_dir, state_path,
    truncate_reason, validate_artifact_id,
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

/// Monotonic counter for tmp filenames. Combined with pid + nanos
/// eliminates Round 2 H-sec #3 collision window (two advances in
/// the same nanosecond within one process getting identical tmp).
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Read phase state for an artifact. Returns `Ok(None)` if the file does
/// not exist (FR-012 — missing state is not an error). Corrupt files
/// are logged and also yield `None` to avoid breaking tool flow.
///
/// Rejects invalid `artifact_id` (path traversal, non-ASCII, …) up-front
/// because `state_path` would otherwise let a tampered id escape the
/// workspace (audit Round 1 C-sec #1).
pub async fn read_phase(workspace: &Path, artifact_id: &str) -> anyhow::Result<Option<PhaseState>> {
    validate_artifact_id(artifact_id)?;

    let path = state_path(workspace, artifact_id);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(None);
    }

    // Symlink guard on the target file — if the state file was replaced
    // by a symlink to `/etc/shadow`, refuse to read. Advisory parity
    // with write-side (audit Round 1 C-sec #2).
    if let Ok(meta) = tokio::fs::symlink_metadata(&path).await
        && meta.file_type().is_symlink()
    {
        tracing::warn!(
            artifact = %artifact_id,
            path = %path.display(),
            "phase state file is a symlink — refusing to read, treating as unknown"
        );
        return Ok(None);
    }

    // Size cap to bound memory on a pathological/corrupted file.
    if let Ok(meta) = tokio::fs::metadata(&path).await
        && meta.len() > MAX_STATE_FILE_BYTES
    {
        tracing::warn!(
            artifact = %artifact_id,
            size = meta.len(),
            max = MAX_STATE_FILE_BYTES,
            "phase state file exceeds size cap — treating as unknown"
        );
        return Ok(None);
    }

    let bytes = match tokio::fs::read(&path).await {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    match serde_yaml::from_slice::<PhaseState>(&bytes) {
        Ok(s) => {
            // Round 2 audit M-logic: fail-safe on unknown future schema
            // rather than silently mis-deserializing a newer writer's
            // output (which serde-default would paper over).
            if s.schema_version > CURRENT_SCHEMA_VERSION {
                tracing::warn!(
                    artifact = %artifact_id,
                    schema_version = s.schema_version,
                    max_known = CURRENT_SCHEMA_VERSION,
                    "phase state from newer schema — treating as unknown to avoid data loss"
                );
                return Ok(None);
            }
            Ok(Some(s))
        }
        Err(e) => {
            // Round 2 audit M-sec #3: preserve forensics. If YAML is
            // corrupt we'd previously let the next advance_phase clobber
            // it via initial_state(), silently wiping history. Quarantine
            // the corrupt file by renaming with a timestamp suffix so an
            // operator can recover / investigate.
            let ts = Utc::now().timestamp();
            let quarantine = path.with_extension(format!("yaml.corrupt.{ts}"));
            if let Err(e2) = tokio::fs::rename(&path, &quarantine).await {
                tracing::warn!(
                    artifact = %artifact_id,
                    path = %path.display(),
                    parse_error = %e,
                    rename_error = %e2,
                    "phase state corrupted AND quarantine failed — treating as unknown"
                );
            } else {
                tracing::warn!(
                    artifact = %artifact_id,
                    path = %path.display(),
                    quarantine = %quarantine.display(),
                    error = %e,
                    "phase state corrupted — quarantined, treating as unknown"
                );
            }
            Ok(None)
        }
    }
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

    let target = state_path(workspace, &state.artifact_id);

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
    validate_artifact_id(artifact_id)?;

    // Track whether state existed on disk — if no, we synthesized the
    // initial Shape in-memory; even when `to == Shape` we must persist
    // so subsequent reads see the materialized state (Round 2 audit
    // H-logic #2: the silent no-persist bug previously left disk empty
    // after "advance from missing" and confused callers).
    let (mut state, was_missing) = match read_phase(workspace, artifact_id).await? {
        Some(s) => (s, false),
        None => (initial_state(artifact_id, None), true),
    };

    let from = state.current_phase;
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
    async fn read_corrupt_yaml_returns_none_not_error() {
        // FR-012 + design invariant #5: corrupt state must not break
        // existing tools. We yield None so callers treat it as unknown.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = state_dir(&ws);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("PRD-X.yaml"), b"{ not valid yaml :::").unwrap();
        let got = read_phase(&ws, "PRD-X").await.unwrap();
        assert!(got.is_none(), "corrupt yaml must read as None");
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

    #[tokio::test]
    async fn history_is_capped_fifo() {
        // Audit Round 1 H1: runaway loop must not balloon history.
        // Advance back-and-forth many times; assert cap respected.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        initialize_phase(&ws, "PRD-CAP", None).await.unwrap();

        for i in 0..(MAX_HISTORY_ENTRIES + 100) {
            let p = if i % 2 == 0 { Phase::Code } else { Phase::Test };
            advance_phase(&ws, "PRD-CAP", p, None).await.unwrap();
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
                advance_phase(&ws, "PRD-CC", phase, Some(format!("concurrent-{i}"))).await
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

    #[tokio::test]
    async fn corrupt_yaml_is_quarantined_not_clobbered() {
        // Audit Round 2 M-sec #3: corrupt state file should be renamed
        // to a quarantine path (not silently clobbered), preserving
        // forensic trail. Next advance rebuilds from initial_state.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = state_dir(&ws);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("PRD-Q.yaml"), b"{ not valid :::").unwrap();

        // Read triggers quarantine.
        let _ = read_phase(&ws, "PRD-Q").await.unwrap();

        // Original file should be gone (or quarantined).
        let orig = std::fs::exists(dir.join("PRD-Q.yaml")).unwrap_or(false);
        // List dir; at least one .yaml.corrupt.* file must exist.
        let mut found_quarantine = false;
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            if entry
                .file_name()
                .to_string_lossy()
                .contains(".yaml.corrupt.")
            {
                found_quarantine = true;
                break;
            }
        }
        assert!(
            !orig,
            "corrupt yaml must be renamed away from the canonical path"
        );
        assert!(
            found_quarantine,
            "corrupt yaml must be renamed to quarantine path"
        );
    }

    #[tokio::test]
    async fn schema_version_greater_than_current_returns_none() {
        // Audit Round 2 M-logic: fail-safe on newer-schema files —
        // do not silently deserialize with defaults.
        let tmp = TempDir::new().unwrap();
        let ws = ws(&tmp);
        let dir = state_dir(&ws);
        std::fs::create_dir_all(&dir).unwrap();
        // Synthesize a valid YAML with future schema_version.
        let future_yaml = r#"artifact_id: PRD-FV
workflow_type: greenfield
current_phase: shape
advanced_at: 2026-04-18T00:00:00Z
history: []
schema_version: 9999
"#;
        std::fs::write(dir.join("PRD-FV.yaml"), future_yaml).unwrap();

        let got = read_phase(&ws, "PRD-FV").await.unwrap();
        assert!(
            got.is_none(),
            "future-schema file must be refused, not silently accepted"
        );
    }
}
