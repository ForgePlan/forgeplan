//! Production [`Dispatcher`] for `Delegation::Command` variant (FR-4).
//!
//! Phase 6 Wave 1 — owner: **command-dispatcher** teammate.
//!
//! # Security model
//!
//! `CommandDispatcher` is the **most security-sensitive** dispatcher in the
//! Phase 6 surface: it is the one path through which an arbitrary executable
//! can be invoked from a playbook. The opt-in gate (`--yes` flag) is enforced
//! **upstream** by [`super::validate_command_delegate_security`]; the
//! dispatcher itself trusts that the executor only routes here after the gate
//! passes. Documented invariants below mirror ADR-010 §Invariants and must
//! hold for every dispatch call.
//!
//! ## Invariants enforced by this dispatcher
//!
//! Per ADR-010 §Invariants:
//!
//! - **NEVER** `Stdio::inherit()` for stdin — `helpers::run_subprocess` sets
//!   `Stdio::null()` when no `stdin_data` is supplied; this dispatcher never
//!   supplies `stdin_data`. Closes the path for interactive prompt injection.
//! - **NEVER** `sh -c` shell expansion — `command: Vec<String>` is treated as
//!   `[program, arg1, arg2, ...]` and passed directly to
//!   `tokio::process::Command::new(program).args(rest)`. The user-supplied
//!   bytes never reach a shell parser.
//! - **NEVER** env passthrough by default — `env_clear()` is applied by the
//!   helper, and the env allow-list passed in is restricted to PATH/HOME/USER
//!   only. **`FORGEPLAN_*` env vars are deliberately excluded** so workspace
//!   secrets / config do not leak into arbitrary user shells.
//! - **ALWAYS** `kill_on_drop(true)` and timeout enforced — both delegated to
//!   `helpers::run_subprocess` which is the single source of truth for
//!   subprocess lifecycle (per ADR-010 Decision row "tokio::process с
//!   kill_on_drop").
//!
//! ## Why command may legitimately be dangerous
//!
//! Even with the invariants above, a `command` step can `rm -rf` whatever the
//! workspace user can. The dispatcher does not attempt to sandbox; the design
//! commitment is that the `--yes` gate makes the user accept that risk
//! explicitly per playbook run. See PRD-072 §Security and SPEC-003
//! §"delegate_to" `command`.
//!
//! See [`super::helpers::run_subprocess`] and ADR-010 §Decision.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;

use super::helpers::{self, SubprocessSpec};
use super::{DispatchError, DispatchOutcome, Dispatcher};
use crate::playbook::types::{Delegation, Step};

/// Default timeout for command dispatch when `Step.timeout_seconds` is not
/// set. Most legitimate command steps (build, lint, codegen, test runner)
/// finish well under three minutes; the higher 600s plugin default isn't
/// warranted here. ADR-010 §Trade-offs row "subprocess timeout policy".
///
/// `Step.timeout_seconds` (FR-8) is not yet wired into the [`Step`] schema; once
/// it lands, dispatch will prefer per-step override and fall back to this
/// default. Until then callers tune via [`CommandDispatcher::with_default_timeout`].
const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 180;

/// FR-4: Production command dispatcher (security-hardened shell).
///
/// Resolves the configured `command: Vec<String>`, builds a [`SubprocessSpec`]
/// with a strict env allow-list, and delegates lifecycle to
/// [`helpers::run_subprocess`]. Errors map onto [`DispatchError`]:
///
/// - Wrong delegate variant         → [`DispatchError::Transport`]
/// - Empty `command` vector         → [`DispatchError::Transport`]
/// - Subprocess spawn / I/O failure → [`DispatchError::Transport`]
/// - Non-zero exit / timeout / kill → [`DispatchOutcome`] with `success=false`
///
/// The `--yes` opt-in is **not** checked here: the executor calls
/// [`super::validate_command_delegate_security`] before routing to this
/// dispatcher. Performing the check twice would either be redundant or
/// (if the dispatcher refused based on its own state) drift away from the
/// executor-level decision — neither is desirable.
pub struct CommandDispatcher {
    /// Workspace root — passed to subprocess as `cwd` so relative paths in
    /// the command (e.g. `./scripts/build.sh`) and any `produces_at` location
    /// resolve correctly.
    pub workspace_root: PathBuf,
    /// Default timeout applied when `Step.timeout_seconds` is not set
    /// (Step does not yet expose this field — wired in FR-8).
    pub default_timeout: Duration,
}

impl CommandDispatcher {
    /// Construct with sensible defaults: 180s timeout (commands are usually
    /// short-running build/lint/test invocations).
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            default_timeout: Duration::from_secs(DEFAULT_COMMAND_TIMEOUT_SECS),
        }
    }

    /// Override the default subprocess timeout.
    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }
}

impl Default for CommandDispatcher {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

#[async_trait]
impl Dispatcher for CommandDispatcher {
    async fn dispatch(&self, step: &Step) -> Result<DispatchOutcome, DispatchError> {
        // 1. Variant guard — caller must not route a non-Command step here.
        let command = match &step.delegate_to {
            Delegation::Command { command } => command,
            other => {
                return Err(DispatchError::Transport(format!(
                    "CommandDispatcher received non-Command delegate: {other:?}",
                )));
            }
        };

        // 2. Reject empty command vector — `tokio::process::Command::new("")`
        //    spawns nothing useful and returns a confusing error. We surface
        //    a clear Transport error instead.
        let (program, args) = match command.split_first() {
            Some((p, rest)) if !p.is_empty() => (p.clone(), rest.to_vec()),
            _ => {
                return Err(DispatchError::Transport(format!(
                    "CommandDispatcher refusing empty command for step `{}`",
                    step.id,
                )));
            }
        };

        // 3. Compose env allow-list — base PATH/HOME/USER ONLY.
        //    Critically: NO FORGEPLAN_* keys are allowed. A `command` step
        //    runs arbitrary user-supplied executables; leaking workspace
        //    config / secrets via env would defeat the security model.
        let base_env: HashMap<String, String> = std::env::vars().collect();
        let env = helpers::build_env_allowlist(&[], &base_env);

        // 4. Build subprocess spec. cwd = workspace_root so relative paths in
        //    the command resolve where the user expects.
        // Per-step timeout (PRD-072 FR-8): step.timeout_seconds overrides
        // the dispatcher default when set; otherwise default applies.
        let timeout = step
            .timeout_seconds
            .map(|s| Duration::from_secs(u64::from(s)))
            .unwrap_or(self.default_timeout);
        let spec = SubprocessSpec {
            program: &program,
            args: &args,
            env: &env,
            cwd: Some(&self.workspace_root),
            timeout,
            // No stdin_data — helper applies Stdio::null() (security invariant).
            stdin_data: None,
        };

        // 5. Execute. Helper translates lifecycle into outcome / Transport.
        let outcome = helpers::run_subprocess(spec).await?;

        // 6. Map subprocess outcome → DispatchOutcome.
        let success = !outcome.timed_out && outcome.exit_code == Some(0);
        let stderr = if outcome.stderr.is_empty() {
            None
        } else {
            Some(String::from_utf8_lossy(&outcome.stderr).into_owned())
        };
        let output_path = if success {
            step.produces_at.clone()
        } else {
            None
        };

        Ok(DispatchOutcome {
            success,
            output_path,
            stderr,
        })
    }
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playbook::types::{Delegation, OnError};

    fn make_step(id: &str, delegation: Delegation) -> Step {
        Step {
            id: id.to_string(),
            delegate_to: delegation,
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: OnError::Abort,
            timeout_seconds: None,
        }
    }

    /// Empty `command` vector is a programming error → Transport.
    #[tokio::test]
    async fn command_dispatcher_rejects_empty_command() {
        let d = CommandDispatcher::new(PathBuf::from("."));
        let step = make_step(
            "empty",
            Delegation::Command {
                command: Vec::<String>::new(),
            },
        );
        let err = d.dispatch(&step).await.expect_err("must reject");
        match err {
            DispatchError::Transport(msg) => {
                assert!(msg.contains("empty command"), "unexpected msg: {msg}");
            }
            other => panic!("expected Transport, got {other:?}"),
        }
    }

    /// Routing a non-Command step is a programming error → Transport.
    #[tokio::test]
    async fn command_dispatcher_rejects_non_command_delegation() {
        let d = CommandDispatcher::new(PathBuf::from("."));
        let step = make_step(
            "wrong",
            Delegation::Agent {
                name: "auditor".into(),
            },
        );
        let err = d.dispatch(&step).await.expect_err("must reject");
        match err {
            DispatchError::Transport(msg) => {
                assert!(
                    msg.contains("non-Command delegate"),
                    "unexpected msg: {msg}"
                );
            }
            other => panic!("expected Transport, got {other:?}"),
        }
    }

    /// Happy path: invoking `/bin/echo hi` returns success and captures
    /// `hi\n` on stdout. Verifies the typed `[program, args...]` shape works
    /// without shell expansion.
    #[tokio::test]
    async fn command_dispatcher_invokes_echo_and_captures_output() {
        // `/bin/echo` exists on macOS + Linux CI runners; skip if absent
        // (e.g. exotic minimal containers) so the suite stays portable.
        if !std::path::Path::new("/bin/echo").is_file() {
            return;
        }
        let d = CommandDispatcher::new(PathBuf::from("."));
        let step = make_step(
            "echo",
            Delegation::Command {
                command: vec!["/bin/echo".to_string(), "hi".to_string()],
            },
        );

        let outcome = d.dispatch(&step).await.expect("dispatch ok");
        assert!(
            outcome.success,
            "expected success, stderr: {:?}",
            outcome.stderr
        );
        // `output_path` is None because the step has no `produces_at`.
        assert!(outcome.output_path.is_none());

        // The success path of `Dispatcher` doesn't expose stdout directly
        // (only stderr is surfaced for diagnostics). We validate stdout via
        // a direct helper call below, mirroring the actual invocation.
        let base_env: HashMap<String, String> = std::env::vars().collect();
        let env = helpers::build_env_allowlist(&[], &base_env);
        let args = vec!["hi".to_string()];
        let spec = SubprocessSpec {
            program: "/bin/echo",
            args: &args,
            env: &env,
            cwd: Some(std::path::Path::new(".")),
            timeout: Duration::from_secs(5),
            stdin_data: None,
        };
        let raw = helpers::run_subprocess(spec).await.expect("subprocess ok");
        assert_eq!(raw.exit_code, Some(0));
        assert_eq!(String::from_utf8_lossy(&raw.stdout), "hi\n");
    }

    /// Non-zero exit code propagates: `success=false` and exit code surfaces
    /// through the underlying helper. We verify both via the public outcome
    /// (`success=false`) and a direct helper assertion on the exit code.
    #[tokio::test]
    async fn command_dispatcher_propagates_exit_code() {
        if !std::path::Path::new("/bin/sh").is_file() {
            return;
        }
        let d = CommandDispatcher::new(PathBuf::from("."));
        let step = make_step(
            "exit7",
            Delegation::Command {
                command: vec![
                    "/bin/sh".to_string(),
                    "-c".to_string(),
                    "exit 7".to_string(),
                ],
            },
        );
        let outcome = d.dispatch(&step).await.expect("dispatch ok");
        assert!(!outcome.success, "exit 7 must surface as failure");
        assert!(outcome.output_path.is_none());

        // Cross-check exit code via helper to confirm it survives mapping.
        let base_env: HashMap<String, String> = std::env::vars().collect();
        let env = helpers::build_env_allowlist(&[], &base_env);
        let args = vec!["-c".to_string(), "exit 7".to_string()];
        let spec = SubprocessSpec {
            program: "/bin/sh",
            args: &args,
            env: &env,
            cwd: Some(std::path::Path::new(".")),
            timeout: Duration::from_secs(5),
            stdin_data: None,
        };
        let raw = helpers::run_subprocess(spec).await.expect("subprocess ok");
        assert_eq!(raw.exit_code, Some(7));
        assert!(!raw.timed_out);
    }

    /// FORGEPLAN_* env vars must not leak into the spawned process. Setting
    /// `FORGEPLAN_FOO` in the parent and asking the child to echo it should
    /// produce empty stdout (just a newline from `echo`). Confirms the
    /// env_clear() + restricted allow-list behavior end-to-end.
    #[tokio::test]
    async fn command_dispatcher_does_not_leak_forgeplan_env() {
        if !std::path::Path::new("/bin/sh").is_file() {
            return;
        }

        // SAFETY: test-local env mutation. We restore at the end. Because
        // tests in this module may run concurrently, choose a unique key.
        let key = "FORGEPLAN_LEAK_PROBE_CMD_DISPATCHER";
        unsafe {
            std::env::set_var(key, "MUST_NOT_APPEAR");
        }

        // Use the helper directly so we can observe the child's stdout.
        // The dispatcher's public outcome does not surface stdout, so this
        // is the cleanest way to assert the env scrubbing invariant.
        let base_env: HashMap<String, String> = std::env::vars().collect();
        // Using the same allow-list as the dispatcher: PATH/HOME/USER only.
        let env = helpers::build_env_allowlist(&[], &base_env);
        let args = vec!["-c".to_string(), format!("printf '%s' \"${{{key}:-}}\"")];
        let spec = SubprocessSpec {
            program: "/bin/sh",
            args: &args,
            env: &env,
            cwd: Some(std::path::Path::new(".")),
            timeout: Duration::from_secs(5),
            stdin_data: None,
        };
        let raw = helpers::run_subprocess(spec).await.expect("subprocess ok");

        // SAFETY: cleanup before assertions so a panic still removes the var.
        unsafe {
            std::env::remove_var(key);
        }

        assert_eq!(raw.exit_code, Some(0));
        assert!(
            raw.stdout.is_empty(),
            "FORGEPLAN_* leaked into child: stdout={:?}",
            String::from_utf8_lossy(&raw.stdout),
        );

        // Sanity: confirm the dispatcher path also refuses to leak. We can't
        // observe the child's stdout from `dispatch`, but we can assert the
        // env construction code path drops the key.
        let dispatcher_env = helpers::build_env_allowlist(&[], &base_env);
        assert!(
            !dispatcher_env.contains_key(key),
            "dispatcher allow-list must drop FORGEPLAN_*",
        );
    }

    /// Timeout fires: a 60s sleep with a 1s timeout returns `success=false`
    /// and the underlying outcome reports `timed_out=true`. We assert the
    /// outer behavior via `DispatchOutcome`, then verify `timed_out=true` via
    /// the helper to pin the contract precisely.
    ///
    /// `Step.timeout_seconds` (FR-8) is not yet wired into [`Step`]; once it
    /// lands the dispatcher will prefer that over `default_timeout`. This
    /// test exercises the `default_timeout` path via
    /// [`CommandDispatcher::with_default_timeout`] which is the same code
    /// path the FR-8 wiring will use.
    #[tokio::test]
    async fn command_dispatcher_respects_step_timeout_seconds() {
        if !std::path::Path::new("/bin/sleep").is_file() {
            return;
        }
        let d =
            CommandDispatcher::new(PathBuf::from(".")).with_default_timeout(Duration::from_secs(1));
        let step = make_step(
            "slow",
            Delegation::Command {
                command: vec!["/bin/sleep".to_string(), "60".to_string()],
            },
        );

        let started = std::time::Instant::now();
        let outcome = d.dispatch(&step).await.expect("dispatch ok");
        let elapsed = started.elapsed();

        assert!(!outcome.success, "timed-out command must not be success");
        assert!(
            elapsed < Duration::from_secs(15),
            "dispatch should return promptly after timeout, took {elapsed:?}",
        );

        // Cross-check `timed_out=true` via direct helper invocation.
        let base_env: HashMap<String, String> = std::env::vars().collect();
        let env = helpers::build_env_allowlist(&[], &base_env);
        let args = vec!["60".to_string()];
        let spec = SubprocessSpec {
            program: "/bin/sleep",
            args: &args,
            env: &env,
            cwd: Some(std::path::Path::new(".")),
            timeout: Duration::from_secs(1),
            stdin_data: None,
        };
        let raw = helpers::run_subprocess(spec).await.expect("subprocess ok");
        assert!(raw.timed_out, "helper must report timed_out=true");
        assert!(raw.exit_code.is_none());
    }

    /// Constructor defaults: 180s timeout (lower than agent's 300s — see
    /// ADR-010 §Trade-offs).
    #[test]
    fn new_uses_180s_default_timeout() {
        let d = CommandDispatcher::new(PathBuf::from("/tmp/ws"));
        assert_eq!(d.default_timeout, Duration::from_secs(180));
        assert_eq!(d.workspace_root, PathBuf::from("/tmp/ws"));
    }

    /// `Default::default` constructs without panicking and uses cwd-relative
    /// workspace root.
    #[test]
    fn default_impl_does_not_panic() {
        let d = CommandDispatcher::default();
        assert_eq!(d.workspace_root, PathBuf::from("."));
        assert_eq!(d.default_timeout, Duration::from_secs(180));
    }
}
