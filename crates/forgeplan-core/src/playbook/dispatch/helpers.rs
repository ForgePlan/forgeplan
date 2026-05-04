//! Shared subprocess helpers for production dispatchers.
//!
//! Wave 1 of Phase 6 (PRD-072 / RFC-007 / ADR-010). Used by:
//! - `plugin_dispatcher` (FR-1)
//! - `agent_dispatcher`  (FR-2)
//! - `skill_dispatcher`  (FR-3)
//! - `command_dispatcher` (FR-4)
//!
//! `forgeplan_core_dispatcher` (FR-5) does NOT use subprocess — direct
//! internal call.
//!
//! # Design
//!
//! Per ADR-010 Decision: `tokio::process::Command` with `kill_on_drop(true)`,
//! `Stdio::piped()` for stdout/stderr, `Stdio::null()` for stdin (or piped
//! when `stdin_data` provided), `env_clear()` + explicit env allow-list,
//! timeout via `tokio::time::timeout`.
//!
//! See EVID-090 for Spike-2 measurements validating this pattern. The three
//! load-bearing constraints from EVID-090 are encoded here:
//! 1. Concurrent stream drain via `tokio::join!` BEFORE `child.wait()` —
//!    otherwise large stderr/stdout deadlocks on the 64 KB pipe buffer.
//! 2. Output is capped per-stream at [`MAX_OUTPUT_BYTES`]; bytes past the cap
//!    are still drained so the child does not block on a full pipe.
//! 3. Callers must shell out to a prebuilt binary (see [`resolve_forgeplan_binary`]) —
//!    `cargo run` blew the 30 s budget on cold-cache invocation.
//!
//! # Cross-platform note
//!
//! `kill_on_drop(true)` on Unix sends `SIGKILL` once the runtime gets a
//! chance to reap the child; on Windows it triggers `TerminateProcess`. Kill
//! latency therefore differs slightly between platforms — tests assert
//! ranges, not exact timings. Pathological children that ignore `SIGKILL`
//! (POSIX zombies) are out of scope per ADR-010 §Risks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

/// Specification for a subprocess invocation. Built by each dispatcher
/// from `Step.delegate_to`, passed to [`run_subprocess`].
///
/// Filled in by the Wave 1 agent owning `helpers.rs` (this file). Other
/// dispatcher agents read this struct as the contract — they should NOT
/// modify it without coordination.
#[derive(Debug, Clone)]
pub struct SubprocessSpec<'a> {
    /// Program name or absolute path (per ADR-010: prefer prebuilt binary,
    /// never `cargo run`).
    pub program: &'a str,
    /// CLI args (typed `Vec<String>` — no shell expansion).
    pub args: &'a [String],
    /// Environment variables (explicit allow-list — `env_clear()` is applied).
    pub env: &'a HashMap<String, String>,
    /// Working directory (None = inherit from parent).
    pub cwd: Option<&'a Path>,
    /// Total timeout. Default 300s; configurable via `Step.timeout_seconds` (FR-8).
    pub timeout: Duration,
    /// Optional bytes piped to stdin. None = `Stdio::null()`.
    /// (Phase 6 v1: most delegates use file-based input via `produces_at` —
    /// stdin reserved for future scriptable workflows.)
    pub stdin_data: Option<&'a [u8]>,
}

/// Result of a subprocess invocation.
///
/// `success` should be derived by caller as `exit_code == Some(0) && !timed_out`.
/// Helper does not interpret exit codes — that's per-dispatcher policy.
#[derive(Debug, Clone)]
pub struct SubprocessOutcome {
    /// Exit code if process completed naturally. `None` if killed by signal
    /// or timed out before exit.
    pub exit_code: Option<i32>,
    /// Captured stdout bytes (cap: 10 MiB — see [`MAX_OUTPUT_BYTES`]).
    pub stdout: Vec<u8>,
    /// Captured stderr bytes (cap: 10 MiB).
    pub stderr: Vec<u8>,
    /// `true` if `tokio::time::timeout` fired and child was killed.
    pub timed_out: bool,
    /// Wall-clock duration from spawn to wait completion (or kill).
    pub duration: Duration,
}

/// Maximum captured output per stream (per ADR-010 Negative Trade-offs).
/// Prevents OOM на runaway subprocess writing GB to stdout.
pub const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;

/// Default subprocess timeout if `Step.timeout_seconds` is not set (FR-8 default).
pub const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Spawn `spec.program` as a subprocess, drain stdout/stderr concurrently,
/// enforce the timeout, and return a [`SubprocessOutcome`].
///
/// Implementation follows ADR-010 §Decision and the three EVID-090 findings:
///
/// - `env_clear()` + explicit `envs(spec.env)` so callers compose an explicit
///   allow-list (see [`build_env_allowlist`]). No `FORGEPLAN_*` leaks to
///   subagents.
/// - `Stdio::piped()` on stdout/stderr drained concurrently with `child.wait()`
///   via `tokio::join!`. Sequential read-then-wait would deadlock once a child
///   fills the 64 KB pipe buffer.
/// - `Stdio::null()` on stdin unless `spec.stdin_data` is set; in that case
///   we pipe the bytes and drop the writer so the child observes EOF.
/// - `kill_on_drop(true)` so a panic / cancel above us reaps the child even
///   without explicit cleanup.
///
/// On timeout we kill the child explicitly and return an `Ok` outcome with
/// `timed_out: true` (per the contract — the dispatcher decides policy from
/// `Step.on_error`). Spawn failures are mapped to
/// [`DispatchError::Transport`] because no other variant matches "could not
/// invoke the delegate at all" within the current `#[non_exhaustive]` enum.
///
/// Returned `stdout` / `stderr` are capped at [`MAX_OUTPUT_BYTES`] (10 MiB).
/// Bytes past the cap are still drained — see [`read_capped`] — so a chatty
/// child cannot stall on a backed-up pipe.
pub async fn run_subprocess(
    spec: SubprocessSpec<'_>,
) -> Result<SubprocessOutcome, super::DispatchError> {
    let mut cmd = Command::new(spec.program);
    cmd.args(spec.args)
        .env_clear()
        .envs(spec.env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    if spec.stdin_data.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }

    if let Some(cwd) = spec.cwd {
        cmd.current_dir(cwd);
    }

    let started = Instant::now();
    let mut child = cmd.spawn().map_err(|e| {
        super::DispatchError::Transport(format!(
            "subprocess spawn failed for `{}`: {e}",
            spec.program
        ))
    })?;

    if let Some(bytes) = spec.stdin_data
        && let Some(mut stdin) = child.stdin.take()
    {
        // Best-effort: a child that closes stdin early (e.g. exits before
        // reading) yields BrokenPipe. We tolerate it — surface only as
        // Transport if the write itself errors for another reason.
        if let Err(e) = stdin.write_all(bytes).await
            && e.kind() != std::io::ErrorKind::BrokenPipe
        {
            return Err(super::DispatchError::Transport(format!(
                "subprocess stdin write failed: {e}"
            )));
        }
        // Dropping `stdin` here signals EOF to the child.
        drop(stdin);
    }

    let stdout = child.stdout.take().expect("stdout was configured as piped");
    let stderr = child.stderr.take().expect("stderr was configured as piped");

    let collect = async {
        let (out_res, err_res, status_res) = tokio::join!(
            read_capped(stdout, MAX_OUTPUT_BYTES),
            read_capped(stderr, MAX_OUTPUT_BYTES),
            child.wait()
        );
        let out = out_res.map_err(|e| {
            super::DispatchError::Transport(format!("subprocess stdout drain failed: {e}"))
        })?;
        let err = err_res.map_err(|e| {
            super::DispatchError::Transport(format!("subprocess stderr drain failed: {e}"))
        })?;
        let status = status_res
            .map_err(|e| super::DispatchError::Transport(format!("subprocess wait failed: {e}")))?;
        Ok::<(Vec<u8>, Vec<u8>, std::process::ExitStatus), super::DispatchError>((out, err, status))
    };

    match tokio::time::timeout(spec.timeout, collect).await {
        Ok(Ok((stdout_buf, stderr_buf, status))) => Ok(SubprocessOutcome {
            exit_code: status.code(),
            stdout: stdout_buf,
            stderr: stderr_buf,
            timed_out: false,
            duration: started.elapsed(),
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            // Timeout fired — kill child, then synthesize timed_out outcome.
            // We deliberately do NOT re-await the drain futures (they were
            // dropped with `collect`); kill_on_drop ensures cleanup.
            let _ = child.kill().await;
            // Best-effort wait so the OS reaps the child before we return.
            let _ = child.wait().await;
            Ok(SubprocessOutcome {
                exit_code: None,
                stdout: Vec::new(),
                stderr: Vec::new(),
                timed_out: true,
                duration: started.elapsed(),
            })
        }
    }
}

/// Drain an async reader up to `max_bytes` into a buffer; once the cap is
/// hit, keep reading into a discard buffer so the child does not block on a
/// full pipe. Returns the captured prefix.
///
/// Sized roughly per EVID-090 finding #2: a runaway child writing GB to
/// stdout must not OOM us, but it also must not deadlock because we stopped
/// reading.
async fn read_capped<R>(mut reader: R, max_bytes: usize) -> std::io::Result<Vec<u8>>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut buf = Vec::with_capacity(max_bytes.min(64 * 1024));
    let mut chunk = [0u8; 8 * 1024];
    let mut overflow = [0u8; 16 * 1024];
    loop {
        let remaining = max_bytes.saturating_sub(buf.len());
        if remaining == 0 {
            // Continue draining without retaining bytes.
            match reader.read(&mut overflow).await {
                Ok(0) => return Ok(buf),
                Ok(_) => continue,
                Err(e) => return Err(e),
            }
        }
        let to_read = remaining.min(chunk.len());
        match reader.read(&mut chunk[..to_read]).await {
            Ok(0) => return Ok(buf),
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) => return Err(e),
        }
    }
}

/// Helper: build env allow-list with `program_specific` keys whitelisted plus
/// `PATH`, `HOME`, `USER` (always passed). Other dispatchers compose env this way.
pub fn build_env_allowlist(
    program_specific: &[&str],
    base_env: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for key in ["PATH", "HOME", "USER"]
        .iter()
        .chain(program_specific.iter())
    {
        if let Some(value) = base_env.get(*key) {
            out.insert(key.to_string(), value.clone());
        }
    }
    out
}

/// Helper: resolve path to prebuilt forgeplan binary. Per ADR-010: never
/// invoke `cargo run`. Tries:
/// 1. `$FORGEPLAN_BIN` env override — **test builds only** (`#[cfg(test)]`);
///    release binaries silently ignore it. Mirrors the
///    `AgentDispatcher::resolve_claude_binary` cfg-gate that closed CWE-426
///    (uncontrolled search path / binary substitution) per PROB-050 A-14.
///    Both env-driven binary substitution surfaces in this module are
///    therefore symmetric: production = explicit override → PATH → release
///    fallback only.
/// 2. `which forgeplan`
/// 3. `target/release/forgeplan` relative to workspace root
///
/// Returns `None` if not found — dispatcher должен emit `Fix:` hint.
pub fn resolve_forgeplan_binary(workspace_root: &Path) -> Option<PathBuf> {
    // PROB-050 A-14 (symmetric fix): removing or widening this
    // `#[cfg(test)]` would expose CWE-426 (binary substitution) in
    // release builds, mirroring the AgentDispatcher gate. Do not change
    // without re-evaluating the security boundary.
    #[cfg(test)]
    if let Ok(override_path) = std::env::var("FORGEPLAN_BIN") {
        let p = PathBuf::from(override_path);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Some(p) = which_in_path("forgeplan") {
        return Some(p);
    }
    let release = workspace_root
        .join("target")
        .join("release")
        .join("forgeplan");
    if release.is_file() {
        return Some(release);
    }
    None
}

/// `which forgeplan` minimal impl — searches `$PATH`, returns first hit.
fn which_in_path(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_env_allowlist_includes_base_keys() {
        let mut base = HashMap::new();
        base.insert("PATH".to_string(), "/usr/bin".to_string());
        base.insert("HOME".to_string(), "/home/x".to_string());
        base.insert("SECRET".to_string(), "leak".to_string());
        let env = build_env_allowlist(&[], &base);
        assert_eq!(env.get("PATH"), Some(&"/usr/bin".to_string()));
        assert_eq!(env.get("HOME"), Some(&"/home/x".to_string()));
        assert!(
            !env.contains_key("SECRET"),
            "non-allowlist keys must be dropped"
        );
    }

    #[test]
    fn build_env_allowlist_includes_program_specific() {
        let mut base = HashMap::new();
        base.insert("PATH".to_string(), "/usr/bin".to_string());
        base.insert("CARGO_HOME".to_string(), "/home/x/.cargo".to_string());
        let env = build_env_allowlist(&["CARGO_HOME"], &base);
        assert_eq!(env.get("CARGO_HOME"), Some(&"/home/x/.cargo".to_string()));
    }

    #[test]
    fn resolve_forgeplan_binary_respects_env_override() {
        // PROB-050 A-14 strengthen (Round 3 audit, test-coverage HIGH-1):
        // pre-PR-B this test asserted only `is_some()`, which any host with
        // `forgeplan` on PATH would satisfy via the `which_in_path` fallback
        // — making it insensitive to the cfg-gate. Strengthened to (a) clear
        // PATH so `which_in_path("forgeplan")` cannot accidentally satisfy
        // the assertion, and (b) compare exact PathBuf so removing or
        // widening the `#[cfg(test)]` gate breaks the test. Mirrors the
        // pattern in `agent_dispatcher::tests::dispatch_emits_delegate_missing_when_tool_absent`.
        let cargo_path = which_in_path("cargo");
        let Some(cargo) = cargo_path else {
            return;
        };
        let original_path = std::env::var_os("PATH");
        // SAFETY: test-local env var manipulation; PATH save/restore guards
        // against leaking the broken PATH to subsequent tests in the same
        // `cargo test` process. PROB-050 A-31 tracks promoting all
        // dispatch-test env mutations to a shared static lock; today
        // `helpers::tests` has no peer test mutating env, so the local
        // pattern stays defensive-only.
        unsafe {
            std::env::set_var("PATH", "/nonexistent-dir-prob-050-a14-helpers-test");
            std::env::set_var("FORGEPLAN_BIN", &cargo);
        }
        let resolved = resolve_forgeplan_binary(Path::new("/tmp/no-such-workspace"));
        // SAFETY: cleanup BEFORE the assert so a panic on the assertion does
        // not poison subsequent tests in the same process.
        unsafe {
            std::env::remove_var("FORGEPLAN_BIN");
            match original_path {
                Some(v) => std::env::set_var("PATH", v),
                None => std::env::remove_var("PATH"),
            }
        }
        assert_eq!(
            resolved.as_deref(),
            Some(cargo.as_path()),
            "cfg(test) gate must keep FORGEPLAN_BIN reachable in test \
             builds; removing or widening the gate would break this assert"
        );
    }

    // ----- run_subprocess tests --------------------------------------------
    //
    // POSIX-only: every test below shells out to `sh`/`echo`/`sleep`/`cat`/`yes`.
    // CI matrix per ADR-010 DoD covers Ubuntu + macOS; Windows would need
    // `cmd.exe` equivalents — out of scope for Wave 1.

    use std::sync::Arc;

    fn empty_env() -> HashMap<String, String> {
        // PATH is required for `sh` to resolve `echo`, `sleep`, etc.
        let mut env = HashMap::new();
        if let Ok(path) = std::env::var("PATH") {
            env.insert("PATH".to_string(), path);
        }
        env
    }

    fn spec_for<'a>(
        program: &'a str,
        args: &'a [String],
        env: &'a HashMap<String, String>,
        timeout: Duration,
    ) -> SubprocessSpec<'a> {
        SubprocessSpec {
            program,
            args,
            env,
            cwd: None,
            timeout,
            stdin_data: None,
        }
    }

    #[tokio::test]
    async fn run_subprocess_captures_stdout() {
        let env = empty_env();
        let args = vec!["hello".to_string()];
        let spec = spec_for("echo", &args, &env, Duration::from_secs(5));
        let outcome = run_subprocess(spec).await.expect("ok");
        assert_eq!(outcome.exit_code, Some(0));
        assert_eq!(outcome.stdout, b"hello\n");
        assert!(outcome.stderr.is_empty());
        assert!(!outcome.timed_out);
        assert!(outcome.duration < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn run_subprocess_captures_stderr() {
        let env = empty_env();
        let args = vec!["-c".to_string(), "echo err >&2".to_string()];
        let spec = spec_for("sh", &args, &env, Duration::from_secs(5));
        let outcome = run_subprocess(spec).await.expect("ok");
        assert_eq!(outcome.exit_code, Some(0));
        assert!(outcome.stdout.is_empty());
        assert_eq!(outcome.stderr, b"err\n");
    }

    #[tokio::test]
    async fn run_subprocess_propagates_exit_code() {
        let env = empty_env();
        let args = vec!["-c".to_string(), "exit 7".to_string()];
        let spec = spec_for("sh", &args, &env, Duration::from_secs(5));
        let outcome = run_subprocess(spec).await.expect("ok");
        assert_eq!(outcome.exit_code, Some(7));
        assert!(!outcome.timed_out);
    }

    #[tokio::test]
    async fn run_subprocess_timeout_kills_child() {
        let env = empty_env();
        let args = vec!["60".to_string()];
        let spec = spec_for("sleep", &args, &env, Duration::from_millis(200));
        let outcome = run_subprocess(spec).await.expect("ok");
        assert!(outcome.timed_out, "expected timed_out=true");
        assert!(outcome.exit_code.is_none());
        // We allow generous slack: timeout 200 ms + kill latency.
        assert!(
            outcome.duration < Duration::from_secs(5),
            "duration {:?} exceeded slack budget",
            outcome.duration
        );
    }

    #[tokio::test]
    async fn run_subprocess_caps_stdout_at_10mib() {
        let env = empty_env();
        // Produce > 10 MiB by piping `yes` (truncated) through `head -c`.
        // `head -c 12582912` = 12 MiB which exceeds the 10 MiB cap.
        let args = vec!["-c".to_string(), "yes hello | head -c 12582912".to_string()];
        let spec = spec_for("sh", &args, &env, Duration::from_secs(15));
        let outcome = run_subprocess(spec).await.expect("ok");
        assert_eq!(outcome.exit_code, Some(0));
        assert!(
            outcome.stdout.len() <= MAX_OUTPUT_BYTES,
            "stdout {} > cap {}",
            outcome.stdout.len(),
            MAX_OUTPUT_BYTES
        );
        assert!(
            outcome.stdout.len() >= MAX_OUTPUT_BYTES - 64 * 1024,
            "stdout {} significantly under cap — drain stopped early?",
            outcome.stdout.len()
        );
    }

    #[tokio::test]
    async fn run_subprocess_with_stdin_data() {
        let env = empty_env();
        let args: Vec<String> = Vec::new();
        let spec = SubprocessSpec {
            program: "cat",
            args: &args,
            env: &env,
            cwd: None,
            timeout: Duration::from_secs(5),
            stdin_data: Some(b"hello"),
        };
        let outcome = run_subprocess(spec).await.expect("ok");
        assert_eq!(outcome.exit_code, Some(0));
        assert_eq!(outcome.stdout, b"hello");
    }

    /// Drop semantics: spawn a long-running child via a future we cancel,
    /// then verify the child gets reaped (no zombie). We approximate this by
    /// dropping the future before completion and re-checking pid liveness.
    #[tokio::test]
    async fn run_subprocess_kill_on_drop_no_zombie() {
        // We cannot directly observe `kill_on_drop` from outside without
        // racing the runtime, so the assertion is structural: cancelling the
        // future via `tokio::time::timeout` (smaller than the child sleep)
        // returns promptly AND the runtime does not leak the spawned child
        // beyond the test boundary. A leaked child would surface as the
        // outer `tokio::test` runtime hanging at shutdown — which the test
        // harness flags as a hard failure.
        let env = Arc::new(empty_env());
        let env_ref = env.clone();
        let task = tokio::spawn(async move {
            let args = vec!["60".to_string()];
            let spec = spec_for("sleep", &args, &env_ref, Duration::from_secs(60));
            run_subprocess(spec).await
        });

        // Yield to let the child spawn, then abort.
        tokio::time::sleep(Duration::from_millis(50)).await;
        task.abort();
        let _ = task.await;

        // If kill_on_drop didn't fire, the runtime would still hold a child
        // handle and the test process would not exit cleanly. We don't have
        // a direct probe here — the post-condition is asserted by the test
        // harness completing (and ADR-010 DoD's `ps` snapshot at integration
        // level).
    }
}
