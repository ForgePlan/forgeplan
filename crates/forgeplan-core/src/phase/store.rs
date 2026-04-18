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
use tokio::io::AsyncWriteExt;

use super::{Phase, PhaseState, PhaseTransition, initial_state, state_dir, state_path};

/// Read phase state for an artifact. Returns `Ok(None)` if the file does
/// not exist (FR-012 — missing state is not an error). Corrupt files
/// are logged and also yield `None` to avoid breaking tool flow.
pub async fn read_phase(workspace: &Path, artifact_id: &str) -> anyhow::Result<Option<PhaseState>> {
    let path = state_path(workspace, artifact_id);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(None);
    }
    let bytes = match tokio::fs::read(&path).await {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    match serde_yaml::from_slice::<PhaseState>(&bytes) {
        Ok(s) => Ok(Some(s)),
        Err(e) => {
            tracing::warn!(
                artifact = %artifact_id,
                path = %path.display(),
                error = %e,
                "phase state file corrupted — treating as unknown"
            );
            Ok(None)
        }
    }
}

/// Atomically write phase state to disk. Creates the state directory
/// if needed, refuses to write through a symlinked state dir, writes to
/// tmp + rename, fsync's both file and parent.
pub async fn write_phase(workspace: &Path, state: &PhaseState) -> anyhow::Result<()> {
    let dir = state_dir(workspace);

    // Symlink guard (audit H-2 pattern from PRD-055 hotfix).
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
    let yaml = serde_yaml::to_string(state)?;

    // tmp + rename for atomic replacement.
    let tmp = dir.join(format!(".{}.yaml.tmp", state.artifact_id));
    let mut f = tokio::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&tmp)
        .await?;
    f.write_all(yaml.as_bytes()).await?;
    f.sync_data().await?;
    drop(f);

    tokio::fs::rename(&tmp, &target).await?;

    // Parent dir fsync so the directory entry for the new file is durable.
    if let Ok(dir_handle) = std::fs::File::open(&dir) {
        let _ = tokio::task::spawn_blocking(move || dir_handle.sync_all()).await;
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
    if let Some(existing) = read_phase(workspace, artifact_id).await? {
        return Ok(existing);
    }
    let fresh = initial_state(artifact_id, reason);
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
    let mut state = match read_phase(workspace, artifact_id).await? {
        Some(s) => s,
        None => initial_state(artifact_id, None),
    };

    let from = state.current_phase;
    // Skip recording a no-op transition (e.g. double-call of auto-advance).
    if from == to {
        return Ok(state);
    }

    let now = Utc::now();
    state.history.push(PhaseTransition {
        from: Some(from),
        to,
        at: now,
        reason,
    });
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
}
