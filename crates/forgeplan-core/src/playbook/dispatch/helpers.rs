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
    /// `true` when stdout exceeded [`MAX_OUTPUT_BYTES`].
    pub stdout_truncated: bool,
    /// Captured stderr bytes (cap: 10 MiB).
    pub stderr: Vec<u8>,
    /// `true` when stderr exceeded [`MAX_OUTPUT_BYTES`].
    pub stderr_truncated: bool,
    /// `true` if `tokio::time::timeout` fired and child was killed.
    pub timed_out: bool,
    /// Wall-clock duration from spawn to wait completion (or kill).
    pub duration: Duration,
}

/// Bounded stream capture with an explicit overflow signal.
///
/// Callers that parse a final response MUST reject `truncated`; a retained
/// prefix is diagnostic data, not a complete model answer.
#[derive(Debug)]
pub(crate) struct CapturedStream {
    pub bytes: Vec<u8>,
    pub truncated: bool,
}

/// Maximum captured output per stream (per ADR-010 Negative Trade-offs).
/// Prevents OOM на runaway subprocess writing GB to stdout.
///
/// PR-E audit MED-1: tightened to `pub(crate)` (no external consumer).
pub(crate) const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;

/// Default subprocess timeout if `Step.timeout_seconds` is not set (FR-8 default).
///
/// PR-E audit MED-1: tightened to `pub(crate)` (no external consumer).
/// `#[allow(dead_code)]` because the constant is referenced only by
/// rustdoc cross-link in `plugin_dispatcher.rs::DEFAULT_PLUGIN_TIMEOUT_SECS`,
/// not by any code path. Kept rather than deleted because the cross-link
/// is load-bearing documentation (operators consult to understand the
/// 300s helper baseline vs 600s plugin override).
#[allow(dead_code)]
pub(crate) const DEFAULT_TIMEOUT_SECS: u64 = 300;

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

    let stdin = child.stdin.take();
    let stdout = child.stdout.take().expect("stdout was configured as piped");
    let stderr = child.stderr.take().expect("stderr was configured as piped");

    // The timeout covers the entire post-spawn lifecycle. In particular,
    // stdin delivery MUST run concurrently with both output drains: a child
    // that stops reading stdin may also fill stdout/stderr, and serial writes
    // would deadlock before the previous timeout even started.
    let collect = async {
        let write_stdin = async {
            if let (Some(mut stdin), Some(bytes)) = (stdin, spec.stdin_data) {
                if let Err(error) = stdin.write_all(bytes).await
                    && error.kind() != std::io::ErrorKind::BrokenPipe
                {
                    return Err(super::DispatchError::Transport(format!(
                        "subprocess stdin write failed: {error}"
                    )));
                }
            }
            Ok::<(), super::DispatchError>(())
        };

        let (stdin_res, out_res, err_res, status_res) = tokio::join!(
            write_stdin,
            read_capped(stdout, MAX_OUTPUT_BYTES),
            read_capped(stderr, MAX_OUTPUT_BYTES),
            child.wait()
        );
        stdin_res?;
        let out = out_res.map_err(|e| {
            super::DispatchError::Transport(format!("subprocess stdout drain failed: {e}"))
        })?;
        let err = err_res.map_err(|e| {
            super::DispatchError::Transport(format!("subprocess stderr drain failed: {e}"))
        })?;
        let status = status_res
            .map_err(|e| super::DispatchError::Transport(format!("subprocess wait failed: {e}")))?;
        Ok::<(CapturedStream, CapturedStream, std::process::ExitStatus), super::DispatchError>((
            out, err, status,
        ))
    };

    match tokio::time::timeout(spec.timeout, collect).await {
        Ok(Ok((stdout, stderr, status))) => Ok(SubprocessOutcome {
            exit_code: status.code(),
            stdout: stdout.bytes,
            stdout_truncated: stdout.truncated,
            stderr: stderr.bytes,
            stderr_truncated: stderr.truncated,
            timed_out: false,
            duration: started.elapsed(),
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            // `collect` is dropped before this arm: its stdin/stdout/stderr
            // handles close, then the explicit kill/wait reaps the child.
            let _ = child.kill().await;
            let _ = child.wait().await;
            Ok(SubprocessOutcome {
                exit_code: None,
                stdout: Vec::new(),
                stdout_truncated: false,
                stderr: Vec::new(),
                stderr_truncated: false,
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
///
/// ADR-017 (CR-1) widening rationale: promoted from helpers-private `fn` to
/// `pub(crate) fn` so the `claude-code` LLM provider
/// ([`crate::llm`]) reuses the identical bounded-drain pattern on the
/// `claude --print` child instead of an unbounded `read_to_end` (which the
/// auditor flagged as a real OOM vector despite the declared
/// `MAX_OUTPUT_BYTES` cap that was never enforced). Takes the reader by
/// value, so callers pass `child.stdout.take()` / `child.stderr.take()`
/// directly; dropping the returned future drops the reader, which is exactly
/// the property the provider's timeout path relies on (CR-2/CR-3) to release
/// the pipes before `kill()` + `wait()`.
pub(crate) async fn read_capped<R>(
    mut reader: R,
    max_bytes: usize,
) -> std::io::Result<CapturedStream>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut buf = Vec::with_capacity(max_bytes.min(64 * 1024));
    let mut chunk = [0u8; 8 * 1024];
    let mut overflow = [0u8; 16 * 1024];
    let mut truncated = false;
    loop {
        let remaining = max_bytes.saturating_sub(buf.len());
        if remaining == 0 {
            // Continue draining without retaining bytes, but preserve the
            // fact that the captured prefix is incomplete.
            match reader.read(&mut overflow).await {
                Ok(0) => {
                    return Ok(CapturedStream {
                        bytes: buf,
                        truncated,
                    });
                }
                Ok(_) => {
                    truncated = true;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        let to_read = remaining.min(chunk.len());
        match reader.read(&mut chunk[..to_read]).await {
            Ok(0) => {
                return Ok(CapturedStream {
                    bytes: buf,
                    truncated,
                });
            }
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

/// Resolve the active ForgePlan project root from the process CWD.
///
/// `workspace::find_workspace` returns the `.forgeplan/` directory itself.
/// Outside a ForgePlan workspace, CWD is the conservative exclusion root.
pub(crate) fn current_workspace_root() -> std::io::Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    Ok(crate::workspace::find_workspace(&cwd)
        .and_then(|forgeplan_dir| forgeplan_dir.parent().map(Path::to_path_buf))
        .unwrap_or(cwd))
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
    //
    // PROB-052 Round 7 audit HIGH-2 closure: route override branches
    // through `resolve_safe_path` so the gate that closed PATH-search
    // also covers (a) the test-only `FORGEPLAN_BIN` env override и
    // (b) the workspace-relative `target/release/forgeplan` fallback.
    // Pre-Round-7 both branches did bare `is_file()`, leaving the
    // workspace-relative path exploitable in cloned-hostile-repo flows.
    #[cfg(test)]
    if let Ok(override_path) = std::env::var("FORGEPLAN_BIN")
        && let Ok(Some(p)) = resolve_safe_path(&PathBuf::from(override_path))
    {
        return Some(p);
    }
    if let Some(p) = which_in_path("forgeplan") {
        return Some(p);
    }
    let release = workspace_root
        .join("target")
        .join("release")
        .join("forgeplan");
    if let Ok(Some(p)) = resolve_safe_path(&release) {
        return Some(p);
    }
    None
}

/// `which <program>` minimal impl — searches `$PATH`, returns first hit.
///
/// PROB-050 A-5 closure: promoted from `fn` (helpers-private) to
/// `pub(super) fn` so AgentDispatcher and PluginDispatcher can drop their
/// duplicate copies and consume the single source of truth here.
///
/// # Security (PROB-052 closure — Round 6 audit MED-1)
///
/// Pre-PROB-052 this was a CWE-367 (TOCTOU) + CWE-426 (untrusted-path
/// hijack) surface. The function did `is_file()` (which silently follows
/// symlinks), no `canonicalize()`, no executable-bit check, no
/// parent-directory-permission check. A user with write access to *any*
/// PATH directory earlier than the legitimate binary could plant a
/// hijacking executable; the window between `is_file()` and the next
/// `Command::spawn` allowed TOCTOU symlink swap on group-writable
/// directories (default Homebrew installs `/usr/local/bin` group-writable
/// под `admin`).
///
/// PROB-052 hardening:
/// 1. **Canonicalize** the resolved path so symlinks land on the real
///    target — caller spawns the resolved binary, not the link, eliminating
///    the swap window.
/// 2. **Reject group-writable / world-writable binaries** on Unix
///    (`mode & 0o022 != 0`). Windows ACL is out of scope (documented).
/// 3. **Reject group-writable / world-writable parent directories** on
///    Unix. Same Windows skip.
/// 4. **Skip non-files** after canonicalize (directories, missing targets
///    of dangling symlinks).
///
/// On Unix the implementation uses `std::os::unix::fs::MetadataExt::mode()`;
/// on Windows the permission gates are skipped by `cfg(unix)`. Cross-platform
/// behavior is preserved (PATH lookup + canonicalize) — only the Unix-only
/// permission gate is gated по `cfg(unix)`.
pub(crate) fn which_in_path(program: &str) -> Option<PathBuf> {
    which_in_path_excluding(program, None)
}

/// Resolve a stock binary while rejecting candidates inside an untrusted
/// workspace. This closes the `node_modules/.bin` / `PATH=./bin:$PATH`
/// substitution path for local LLM providers without changing the legacy
/// dispatcher lookup contract.
pub(crate) fn which_in_path_outside(program: &str, excluded_root: &Path) -> Option<PathBuf> {
    let excluded_root = std::fs::canonicalize(excluded_root).ok();
    which_in_path_excluding(program, excluded_root.as_deref())
}

/// Filter an inherited PATH before passing it to an untrusted subprocess.
///
/// Only absolute entries survive. Entries under any excluded root are removed
/// in both their lexical and canonical forms so `/tmp` → `/private/tmp`
/// aliases cannot preserve a workspace-controlled executable directory.
pub(crate) fn filtered_path_outside(path: &str, excluded_roots: &[&Path]) -> Option<String> {
    let roots = excluded_roots
        .iter()
        .flat_map(|root| {
            let canonical = std::fs::canonicalize(root).ok();
            std::iter::once((*root).to_path_buf()).chain(canonical)
        })
        .collect::<Vec<_>>();
    let entries = std::env::split_paths(std::ffi::OsStr::new(path))
        .filter(|entry| !entry.as_os_str().is_empty() && entry.is_absolute())
        .filter(|entry| {
            let canonical = std::fs::canonicalize(entry).ok();
            !roots.iter().any(|root| {
                entry.starts_with(root)
                    || canonical
                        .as_ref()
                        .is_some_and(|resolved| resolved.starts_with(root))
            })
        })
        .collect::<Vec<_>>();
    std::env::join_paths(entries)
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

fn which_in_path_excluding(program: &str, excluded_root: Option<&Path>) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        // Empty and relative entries can resolve from an attacker-controlled
        // repository. Stock CLI discovery accepts absolute install roots only.
        if dir.as_os_str().is_empty() || !dir.is_absolute() {
            continue;
        }

        #[cfg(windows)]
        let mut candidates = vec![dir.join(program)];
        #[cfg(not(windows))]
        let candidates = vec![dir.join(program)];
        #[cfg(windows)]
        if Path::new(program).extension().is_none() {
            let pathext =
                std::env::var_os("PATHEXT").unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into());
            for extension in pathext.to_string_lossy().split(';') {
                let extension = extension.trim();
                if !extension.is_empty() {
                    candidates.push(dir.join(format!("{program}{extension}")));
                }
            }
        }

        for candidate in candidates {
            match resolve_safe_path(&candidate) {
                Ok(Some(real)) if excluded_root.is_some_and(|root| real.starts_with(root)) => {
                    let candidate_safe = candidate.display().to_string().escape_debug().to_string();
                    tracing::warn!(
                        target = "playbook::dispatch::helpers",
                        candidate = %candidate_safe,
                        "which_in_path: rejected workspace-controlled candidate"
                    );
                }
                Ok(Some(real)) => return Some(real),
                Ok(None) => continue,
                Err(reason) => {
                    let candidate_safe = candidate.display().to_string().escape_debug().to_string();
                    let reason_safe = reason.escape_debug().to_string();
                    eprintln!(
                        "warning: which_in_path: rejected unsafe candidate {candidate_safe}: {reason_safe}"
                    );
                    tracing::warn!(
                        target = "playbook::dispatch::helpers",
                        candidate = %candidate_safe,
                        reason = %reason_safe,
                        "which_in_path: rejected unsafe candidate"
                    );
                }
            }
        }
    }
    None
}

/// Resolve `candidate` to a canonicalised, non-world-writable file path.
///
/// **Visibility**: `pub(super)` so dispatcher consumers (`AgentDispatcher::
/// resolve_claude_binary`, `PluginDispatcher::resolve_binary`,
/// `resolve_forgeplan_binary`) can route their explicit-override branches
/// through the same gate. Closing PROB-052 Round 7 audit HIGH-1 — pre-Round-7
/// the override branches did bare `is_file()`, leaving CWE-426 hijack
/// exploitable on the *configured-binary* path even after PATH-search was
/// hardened.
///
/// Returns:
/// - `Ok(Some(real))` if the candidate exists, canonicalises to a regular
///   file, and (на Unix) has safe permission bits on both the file and its
///   parent directory.
/// - `Ok(None)` if the candidate simply does not exist OR canonicalises to a
///   non-file (directory, special file, dangling symlink target). Caller
///   continues searching the next PATH entry.
/// - `Err(String)` if the candidate exists but is rejected on security
///   grounds (group/world-writable file or parent dir). Caller logs the
///   rejection and continues; this is NOT a fatal error because PATH may
///   contain multiple entries and the next one might be safe.
///
/// **Residual TOCTOU**: `canonicalize` + `metadata` are two separate syscalls,
/// and the eventual `Command::spawn` is yet another. The gate **shrinks** the
/// swap window from operator-time to syscall-time но не closes it fully —
/// closing requires `O_NOFOLLOW` + `fexecve`-style fd-based exec which is
/// non-portable (Linux-only). Out of scope for PROB-052; tracked в follow-up.
pub(super) fn resolve_safe_path(candidate: &Path) -> Result<Option<PathBuf>, String> {
    // PROB-052 #1: canonicalize — follows symlinks to the real target.
    // `canonicalize` returns Err if the candidate doesn't exist; treat that
    // as "not in this PATH dir, try next" rather than a security rejection.
    let real = match std::fs::canonicalize(candidate) {
        Ok(p) => p,
        Err(_) => return Ok(None),
    };

    // PROB-052 #4: must be a regular file (not a directory, not a dangling
    // symlink target). symlink_metadata is unnecessary here — canonicalize
    // already followed the chain.
    let meta = match std::fs::metadata(&real) {
        Ok(m) => m,
        Err(_) => return Ok(None),
    };
    if !meta.is_file() {
        return Ok(None);
    }

    // PROB-052 #2 + #3: Unix permission gates. Concretely the mask
    // `0o022` covers two bits: `0o020` (group-write) AND `0o002`
    // (world-write). Either set ⇒ reject. Out of scope: setuid `0o4000`,
    // setgid `0o2000`, sticky `0o1000`, POSIX ACLs, MAC labels.
    // Windows ACLs deliberately skipped via `cfg(unix)` per PRD §AC-3.
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let mode = meta.mode();
        if mode & 0o022 != 0 {
            return Err(format!(
                "binary write-bits {:03o} set (gate: mode & 0o022)",
                mode & 0o022
            ));
        }
        // Reject if parent dir is group- or world-writable (e.g. default
        // Homebrew /usr/local/bin under admin group).
        if let Some(parent) = real.parent()
            && let Ok(parent_meta) = std::fs::metadata(parent)
        {
            let pmode = parent_meta.mode();
            if pmode & 0o022 != 0 {
                return Err(format!(
                    "parent dir {} write-bits {:03o} set (gate: mode & 0o022)",
                    parent.display().to_string().escape_debug(),
                    pmode & 0o022
                ));
            }
        }
    }

    Ok(Some(real))
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

    // CR-H6 (audit closure): `#[serial_test::serial(env_path)]` adds a
    // sync, in-process lock keyed by `env_path` that any other test in
    // this crate (or this crate's test binary) can opt into without
    // taking the dispatcher-specific async mutex. Defensive supplement
    // to the existing `DISPATCH_ENV_LOCK` — both keep PATH-mutating
    // tests sequential. Caveat: serial_test locks are in-process, so
    // they do not coordinate against the `forgeplan-cli` or
    // `forgeplan-mcp` test binaries running in parallel. Cargo runs
    // each crate's tests as a separate binary, so the cross-crate race
    // risk is bounded by cargo's per-crate parallelism, which is the
    // pragmatic compromise.
    #[tokio::test]
    #[serial_test::serial(env_path)]
    #[allow(clippy::await_holding_lock)] // DISPATCH_ENV_LOCK pins env vars across spawn for test isolation
    async fn resolve_forgeplan_binary_respects_env_override() {
        // PROB-050 A-14 strengthen (Round 3 audit, test-coverage HIGH-1):
        // pre-PR-B this test asserted only `is_some()`, which any host with
        // `forgeplan` on PATH would satisfy via the `which_in_path` fallback
        // — making it insensitive to the cfg-gate. Strengthened to (a) clear
        // PATH so `which_in_path("forgeplan")` cannot accidentally satisfy
        // the assertion, and (b) compare exact PathBuf so removing or
        // widening the `#[cfg(test)]` gate breaks the test. Mirrors the
        // pattern in `agent_dispatcher::tests::dispatch_emits_delegate_missing_when_tool_absent`.
        //
        // PROB-050 A-6 closure (Round 5 LOW-1 race fix): now serialises against
        // peer dispatcher tests via the shared `DISPATCH_ENV_LOCK` instead of
        // relying on the (false) assumption that `helpers::tests` has no peer
        // mutating env. `agent_dispatcher::tests` and `plugin_dispatcher::tests`
        // share the same lock, eliminating cross-file PATH-mutation flakiness.
        let _guard = super::super::claude_print::DISPATCH_ENV_LOCK.lock().await;
        let cargo_path = which_in_path("cargo");
        let Some(cargo) = cargo_path else {
            return;
        };
        let original_path = std::env::var_os("PATH");
        // SAFETY: test-local env var manipulation; PATH save/restore guards
        // against leaking the broken PATH to subsequent tests in the same
        // `cargo test` process.
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
        assert!(outcome.stdout_truncated);
        assert!(!outcome.stderr_truncated);
        assert!(
            outcome.stdout.len() >= MAX_OUTPUT_BYTES - 64 * 1024,
            "stdout {} significantly under cap — drain stopped early?",
            outcome.stdout.len()
        );
    }

    #[tokio::test]
    async fn run_subprocess_caps_stderr_at_10mib() {
        let env = empty_env();
        let args = vec![
            "-c".to_string(),
            "yes error | head -c 12582912 >&2".to_string(),
        ];
        let spec = spec_for("sh", &args, &env, Duration::from_secs(15));
        let outcome = run_subprocess(spec).await.expect("ok");
        assert_eq!(outcome.exit_code, Some(0));
        assert!(outcome.stdout.is_empty());
        assert!(outcome.stderr.len() <= MAX_OUTPUT_BYTES);
        assert!(!outcome.stdout_truncated);
        assert!(outcome.stderr_truncated);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn timeout_reaps_the_direct_child_pid() {
        let env = empty_env();
        let temp = tempfile::tempdir().unwrap();
        let pid_file = temp.path().join("pid");
        let args = vec![
            "-c".to_string(),
            format!("echo $$ > '{}'; exec sleep 60", pid_file.display()),
        ];
        let spec = spec_for("sh", &args, &env, Duration::from_millis(500));
        let outcome = run_subprocess(spec).await.expect("timeout outcome");
        assert!(outcome.timed_out);

        let pid: libc::pid_t = std::fs::read_to_string(pid_file)
            .expect("child wrote pid")
            .trim()
            .parse()
            .expect("numeric pid");
        // SAFETY: signal 0 probes existence without delivering a signal.
        let probe = unsafe { libc::kill(pid, 0) };
        assert_eq!(probe, -1, "direct child PID {pid} still exists");
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
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

    #[tokio::test]
    async fn timeout_covers_blocked_stdin_delivery() {
        let env = empty_env();
        let args = vec!["60".to_string()];
        let payload = vec![b'x'; 1024 * 1024];
        let spec = SubprocessSpec {
            program: "sleep",
            args: &args,
            env: &env,
            cwd: None,
            timeout: Duration::from_millis(100),
            stdin_data: Some(&payload),
        };

        let outcome = run_subprocess(spec).await.expect("timeout is an outcome");
        assert!(outcome.timed_out);
        assert!(
            outcome.duration < Duration::from_secs(5),
            "stdin write escaped timeout: {:?}",
            outcome.duration
        );
    }

    /// Cancelling the runner future must trigger `kill_on_drop` and remove the
    /// direct child from the process table.
    #[cfg(unix)]
    #[tokio::test]
    async fn run_subprocess_kill_on_drop_reaps_direct_child_pid() {
        let temp = tempfile::tempdir().unwrap();
        let pid_file = temp.path().join("cancelled-child.pid");
        let env = Arc::new(empty_env());
        let env_ref = env.clone();
        let task = tokio::spawn(async move {
            let args = vec![
                "-c".to_string(),
                format!("echo $$ > '{}'; exec sleep 60", pid_file.display()),
            ];
            let spec = spec_for("sh", &args, &env_ref, Duration::from_secs(60));
            run_subprocess(spec).await
        });

        let pid = {
            let pid_file = temp.path().join("cancelled-child.pid");
            let mut observed = None;
            for _ in 0..100 {
                if let Ok(value) = std::fs::read_to_string(&pid_file)
                    && let Ok(pid) = value.trim().parse::<libc::pid_t>()
                {
                    observed = Some(pid);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            observed.expect("child wrote its PID before cancellation")
        };

        task.abort();
        let _ = task.await;

        for _ in 0..100 {
            // SAFETY: signal 0 probes existence without delivering a signal.
            let probe = unsafe { libc::kill(pid, 0) };
            if probe == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("cancelled direct child PID {pid} still exists");
    }

    // ─────────────────────────────────────────────────────────────────────
    // PROB-052 TOCTOU + symlink-follow + perm-gate hardening tests
    // ─────────────────────────────────────────────────────────────────────

    /// AC-1 — `which_in_path` MUST canonicalize the resolved path so a
    /// symlink in PATH resolves to its real target. This eliminates the
    /// TOCTOU swap window between `is_file()` and the eventual
    /// `Command::spawn` — once the canonical path is captured, swapping the
    /// symlink target swaps a different file, not the resolved one.
    ///
    /// Serializes против peer dispatcher tests via `DISPATCH_ENV_LOCK`
    /// because all PATH-mutating tests in this crate share that mutex.
    #[cfg(unix)]
    // CR-H6: PATH mutator — locked under `env_path` key.
    #[tokio::test]
    #[serial_test::serial(env_path)]
    #[allow(clippy::await_holding_lock)] // DISPATCH_ENV_LOCK pins env vars across spawn for test isolation
    async fn which_in_path_canonicalizes_symlink_to_real_target() {
        let _guard = super::super::claude_print::DISPATCH_ENV_LOCK.lock().await;
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let bin_dir = tmp.path().join("bin");
        std::fs::create_dir(&bin_dir).unwrap();
        // Tighten dir mode so the parent-dir gate accepts it. tempdir() on
        // some platforms creates 0o700 already; explicit set is safest.
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let real = bin_dir.join("real-bin");
        std::fs::write(&real, b"#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o755)).unwrap();
        let link = bin_dir.join("link-bin");
        symlink(&real, &link).unwrap();

        // SAFETY: test-local PATH manipulation. helpers::which_in_path reads
        // PATH on each call; restore at the end so subsequent tests are
        // unaffected.
        let original_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", &bin_dir);
        }
        let resolved = which_in_path("link-bin");
        unsafe {
            match original_path {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }

        let resolved = resolved.expect("symlink target must resolve");
        let real_canonical = std::fs::canonicalize(&real).unwrap();
        assert_eq!(
            resolved, real_canonical,
            "which_in_path must return canonical real path, not the symlink"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    #[allow(clippy::await_holding_lock)]
    async fn which_in_path_outside_rejects_workspace_owned_stock_name() {
        let _guard = super::super::claude_print::DISPATCH_ENV_LOCK.lock().await;
        use std::os::unix::fs::PermissionsExt;

        let workspace = tempfile::tempdir().unwrap();
        let bin_dir = workspace.path().join("node_modules/.bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let binary = bin_dir.join("codex");
        std::fs::write(&binary, b"#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&bin_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();

        let original_path = std::env::var_os("PATH");
        unsafe { std::env::set_var("PATH", &bin_dir) };
        let resolved = which_in_path_outside("codex", workspace.path());
        unsafe {
            match original_path {
                Some(path) => std::env::set_var("PATH", path),
                None => std::env::remove_var("PATH"),
            }
        }

        assert!(
            resolved.is_none(),
            "workspace-controlled stock binary must be rejected: {resolved:?}"
        );
    }

    #[test]
    fn filtered_path_removes_workspace_and_relative_entries() {
        let workspace = tempfile::tempdir().unwrap();
        let nested = workspace.path().join("node_modules/.bin");
        std::fs::create_dir_all(&nested).unwrap();
        let safe = tempfile::tempdir().unwrap();
        let raw = std::env::join_paths([
            nested.as_path(),
            std::path::Path::new("relative-bin"),
            safe.path(),
        ])
        .unwrap()
        .to_string_lossy()
        .into_owned();

        let filtered = filtered_path_outside(&raw, &[workspace.path()]).unwrap();
        let entries = std::env::split_paths(std::ffi::OsStr::new(&filtered)).collect::<Vec<_>>();
        assert_eq!(entries, vec![safe.path().to_path_buf()]);
    }

    /// AC-2 — Group/world-writable binary MUST be rejected on Unix
    /// (mode bits 0o022 set). Pre-PROB-052 this was a CWE-426 hijack vector
    /// because `is_file()` made no permission distinction.
    // CR-H6: PATH mutator — locked under `env_path` key.
    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    #[allow(clippy::await_holding_lock)]
    async fn which_in_path_rejects_group_writable_binary() {
        let _guard = super::super::claude_print::DISPATCH_ENV_LOCK.lock().await;
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let bin_dir = tmp.path().join("bin");
        std::fs::create_dir(&bin_dir).unwrap();
        std::fs::set_permissions(&bin_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let bin = bin_dir.join("hijackable");
        std::fs::write(&bin, b"#!/bin/sh\nexit 0\n").unwrap();
        // 0o775 = group-writable — the exact case Homebrew creates by
        // default on /usr/local/bin under group `admin`.
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o775)).unwrap();

        let original_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", &bin_dir);
        }
        let resolved = which_in_path("hijackable");
        unsafe {
            match original_path {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }

        assert!(
            resolved.is_none(),
            "group-writable binary must be rejected (CWE-426 hijack vector); got: {resolved:?}"
        );
    }

    /// AC-3 — Cross-platform: on Windows the Unix permission gate is
    /// skipped (Windows ACL is out of PROB-052 scope, documented in PRD).
    /// PATH lookup + canonicalize must still apply.
    // CR-H6: PATH mutator — locked under `env_path` key.
    #[cfg(not(unix))]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    #[allow(clippy::await_holding_lock)]
    async fn which_in_path_windows_skips_permission_gate() {
        let _guard = super::super::claude_print::DISPATCH_ENV_LOCK.lock().await;
        let tmp = tempfile::tempdir().unwrap();
        let bin_dir = tmp.path().join("bin");
        std::fs::create_dir(&bin_dir).unwrap();
        let bin = bin_dir.join("anybin.exe");
        std::fs::write(&bin, b"stub").unwrap();

        let original_path = std::env::var_os("PATH");
        let original_pathext = std::env::var_os("PATHEXT");
        unsafe {
            std::env::set_var("PATH", &bin_dir);
            std::env::set_var("PATHEXT", ".EXE;.CMD");
        }
        let resolved = which_in_path("anybin");
        unsafe {
            match original_path {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
            match original_pathext {
                Some(value) => std::env::set_var("PATHEXT", value),
                None => std::env::remove_var("PATHEXT"),
            }
        }

        // On Windows the permission gate is a no-op so existing-file
        // resolution still succeeds. Test asserts "found" rather than
        // mode-specific behavior.
        assert!(
            resolved.is_some(),
            "Windows must still resolve real binaries via PATH"
        );
    }

    /// Round 7 audit MED-4 — empty PATH entry (POSIX `:` interpreted as `.`)
    /// must be skipped explicitly. Implicit cwd lookup is a hijack vector
    /// when forgeplan is invoked inside a hostile cloned repo.
    // CR-H6: PATH mutator — locked under `env_path` key.
    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    #[allow(clippy::await_holding_lock)]
    async fn which_in_path_skips_empty_path_entries() {
        let _guard = super::super::claude_print::DISPATCH_ENV_LOCK.lock().await;
        let original_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", ":/nonexistent-dir-prob-052-med-4-test");
        }
        let resolved = which_in_path("ls");
        unsafe {
            match original_path {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }
        // Empty entry MUST be skipped (no cwd-relative resolve), and the
        // bogus dir doesn't exist, so result is None.
        assert!(
            resolved.is_none(),
            "empty PATH entry must NOT trigger cwd-relative resolve; got: {resolved:?}"
        );
    }

    /// Round 7 audit HIGH-1 closure — explicit `claude_binary` field on
    /// AgentDispatcher MUST go through `resolve_safe_path`. This test asserts
    /// a group-writable override is rejected just like a PATH-resolved one.
    /// Pre-Round-7 the override branch was unguarded.
    // CR-H6: this test does NOT mutate PATH directly, but it shares the
    // `DISPATCH_ENV_LOCK` with the other dispatcher tests so we tag it
    // under the same `env_path` key for predictable scheduling
    // (otherwise it could acquire DISPATCH_ENV_LOCK while a peer test
    // holds it, leaving serial_test queueing behaviour split).
    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    #[allow(clippy::await_holding_lock)]
    async fn resolve_safe_path_rejects_group_writable_override() {
        use std::os::unix::fs::PermissionsExt;
        let _guard = super::super::claude_print::DISPATCH_ENV_LOCK.lock().await;
        let tmp = tempfile::tempdir().unwrap();
        let bin_dir = tmp.path().join("bin");
        std::fs::create_dir(&bin_dir).unwrap();
        std::fs::set_permissions(&bin_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let bin = bin_dir.join("override-target");
        std::fs::write(&bin, b"#!/bin/sh\nexit 0\n").unwrap();
        // Group-writable file — same hijack vector as the PATH test, just
        // exercised through the resolve_safe_path entry point directly
        // (mirroring how dispatcher overrides will call it).
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o775)).unwrap();

        let result = resolve_safe_path(&bin);
        assert!(
            matches!(result, Err(ref s) if s.contains("write-bits")),
            "group-writable override must be rejected with mode-bit message; got: {result:?}"
        );
    }

    /// Round 7 audit HIGH-1 boundary — a safe override is accepted as the
    /// canonical path, mirroring the PATH-resolution semantics. Ensures the
    /// dispatcher consumer call sites can rely on a single canonical
    /// PathBuf rather than the original (potentially symlinked) input.
    // CR-H6: shared DISPATCH_ENV_LOCK acquirer — tagged for predictable
    // serial_test scheduling alongside peer dispatcher tests.
    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    #[allow(clippy::await_holding_lock)]
    async fn resolve_safe_path_canonicalizes_safe_override() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let _guard = super::super::claude_print::DISPATCH_ENV_LOCK.lock().await;
        let tmp = tempfile::tempdir().unwrap();
        let bin_dir = tmp.path().join("bin");
        std::fs::create_dir(&bin_dir).unwrap();
        std::fs::set_permissions(&bin_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let real = bin_dir.join("real-override");
        std::fs::write(&real, b"#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o755)).unwrap();
        let link = bin_dir.join("link-override");
        symlink(&real, &link).unwrap();

        let result = resolve_safe_path(&link);
        let resolved = match result {
            Ok(Some(p)) => p,
            other => panic!("expected Ok(Some) for safe override; got: {other:?}"),
        };
        let real_canonical = std::fs::canonicalize(&real).unwrap();
        assert_eq!(
            resolved, real_canonical,
            "safe symlinked override must canonicalize to real target"
        );
    }

    /// PROB-052 boundary — a group-writable PARENT directory must reject
    /// the binary even if the binary itself has tight 0o755 mode. This
    /// covers the default Homebrew posture where `/usr/local/bin` is
    /// 0o775 group=admin and any admin user can plant a binary that the
    /// dispatcher will then run on behalf of the workspace owner.
    // CR-H6: PATH mutator — locked under `env_path` key.
    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    #[allow(clippy::await_holding_lock)]
    async fn which_in_path_rejects_group_writable_parent_dir() {
        let _guard = super::super::claude_print::DISPATCH_ENV_LOCK.lock().await;
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let bin_dir = tmp.path().join("bin");
        std::fs::create_dir(&bin_dir).unwrap();
        // Group-writable parent.
        std::fs::set_permissions(&bin_dir, std::fs::Permissions::from_mode(0o775)).unwrap();
        let bin = bin_dir.join("safe-bin");
        std::fs::write(&bin, b"#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();

        let original_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", &bin_dir);
        }
        let resolved = which_in_path("safe-bin");
        unsafe {
            match original_path {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }

        assert!(
            resolved.is_none(),
            "group-writable parent dir must reject child binary; got: {resolved:?}"
        );
    }
}
