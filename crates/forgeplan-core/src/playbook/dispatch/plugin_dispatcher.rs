//! Production [`Dispatcher`] for [`Delegation::Plugin`] variant (FR-1).
//!
//! Phase 6 Wave 1 — owner: **phase6-plugin-dispatcher** teammate.
//! References: PRD-072 §FR-1/AC-1/AC-2, RFC-007 §"plugin", ADR-010 §Decision,
//! EVID-090 (Spike-2 measurements).
//!
//! # v1 invocation mechanism
//!
//! Phase 6 v1 invokes plugins through a **prebuilt subprocess** rather than
//! the in-process Task tool API: there is no stable Rust binding for the
//! Claude Code Task tool yet (see RFC-007 §"open issues"). The dispatcher
//! shells out to a configurable `task_tool_path` binary with two CLI
//! arguments — the plugin `name` and the plugin-internal `target`. By
//! convention this binary is `claude-code-plugin invoke <name> <target>`,
//! but the path is overridable via [`PluginDispatcher::with_task_tool`] so
//! tests (and future `forgeplan-internal plugin-invoke <name> <target>`
//! shim) can inject any executable that honours the same argv contract.
//!
//! When `task_tool_path` is `None`, the dispatcher falls back to `which
//! claude-code-plugin` on `PATH`. If neither resolves, `dispatch` returns
//! [`DispatchError::DelegateMissing`] carrying the step's `fallback_hint`
//! (PRD-072 AC-2: install command surfaced to the user).
//!
//! # Invariants per ADR-010
//!
//! All subprocess work goes through [`super::helpers::run_subprocess`] which
//! enforces:
//! - `kill_on_drop(true)` (no zombie children on cancel/panic)
//! - `env_clear()` + explicit allow-list (no `FORGEPLAN_*` leak)
//! - `Stdio::null()` for stdin, `Stdio::piped()` for stdout/stderr
//! - per-stream cap [`super::helpers::MAX_OUTPUT_BYTES`] (10 MiB)
//! - timeout via `tokio::time::timeout`; default 600s here because real
//!   plugins (c4-architecture) regularly take 5+ minutes
//!
//! # Default timeout
//!
//! Plugin executions are slower than skill/agent calls — the default is
//! `600s` (10 min) when `Step.timeout_seconds` is absent (FR-8 default for
//! plugins). Override per-step via the upcoming `Step.timeout_seconds`
//! field once SPEC-003 minor-bumps to 1.1 (PRD-072 FR-8).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;

use super::helpers::{
    self, DEFAULT_TIMEOUT_SECS, SubprocessSpec, build_env_allowlist, resolve_forgeplan_binary,
};
use super::{DispatchError, DispatchOutcome, Dispatcher};
use crate::playbook::types::{Delegation, Step};

/// Default plugin executor binary searched on `PATH` when
/// [`PluginDispatcher::task_tool_path`] is `None`.
const DEFAULT_PLUGIN_BINARY: &str = "claude-code-plugin";

/// Default per-step timeout for plugins. Plugins are typically slower than
/// agents/skills — bumped to 600s vs the helper default of 300s
/// ([`super::helpers::DEFAULT_TIMEOUT_SECS`]).
pub const DEFAULT_PLUGIN_TIMEOUT_SECS: u64 = 600;

/// FR-1 production dispatcher for `Delegation::Plugin { name, target }`.
///
/// See module docs for invocation mechanism, invariants, and override
/// hooks. Construct via [`Self::new`] for production wiring or
/// [`Self::with_task_tool`] for tests/dev.
pub struct PluginDispatcher {
    /// Workspace root — passed to [`resolve_forgeplan_binary`] when probing
    /// for fallback paths and used as `cwd` for the spawned subprocess.
    workspace_root: PathBuf,
    /// Override path to the plugin executor binary. Default:
    /// `which claude-code-plugin`.
    task_tool_path: Option<PathBuf>,
    /// Default timeout if `Step.timeout_seconds` (FR-8) is unset.
    default_timeout: Duration,
}

impl PluginDispatcher {
    /// Build a dispatcher rooted at `workspace_root` using the default
    /// `claude-code-plugin` resolution and the 600s default timeout.
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            task_tool_path: None,
            default_timeout: Duration::from_secs(DEFAULT_PLUGIN_TIMEOUT_SECS),
        }
    }

    /// Override the plugin executor binary. Used by tests (point at
    /// `/bin/echo` etc.) and by future bundled `forgeplan-internal
    /// plugin-invoke` shim wiring.
    pub fn with_task_tool(mut self, path: PathBuf) -> Self {
        self.task_tool_path = Some(path);
        self
    }

    /// Override the default timeout (used when `Step.timeout_seconds`
    /// is absent).
    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Resolve the binary to invoke. Order:
    /// 1. Explicit `task_tool_path` (test injection / config override)
    /// 2. `which claude-code-plugin` on `$PATH`
    ///
    /// Returns `None` if neither resolves — caller maps this to
    /// [`DispatchError::DelegateMissing`].
    fn resolve_binary(&self) -> Option<PathBuf> {
        if let Some(path) = &self.task_tool_path {
            return Some(path.clone());
        }
        which_in_path(DEFAULT_PLUGIN_BINARY)
    }
}

impl Default for PluginDispatcher {
    /// Default dispatcher rooted at the current working directory. Use
    /// [`Self::new`] when an explicit workspace root is known.
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

#[async_trait]
impl Dispatcher for PluginDispatcher {
    async fn dispatch(&self, step: &Step) -> Result<DispatchOutcome, DispatchError> {
        let (name, target) = match &step.delegate_to {
            Delegation::Plugin { name, target } => (name.clone(), target.clone()),
            _ => {
                return Err(DispatchError::Transport(format!(
                    "PluginDispatcher cannot handle non-plugin delegation for step `{}`",
                    step.id
                )));
            }
        };

        let binary = self.resolve_binary().ok_or_else(|| {
            let reason = step.fallback_hint.clone().unwrap_or_else(|| {
                "install Claude Code Task tool (`claude-code-plugin`) and ensure it is on $PATH"
                    .to_string()
            });
            DispatchError::DelegateMissing {
                delegate: format!("plugin:{name}"),
                reason,
            }
        })?;

        let program_str = binary.to_string_lossy().into_owned();
        let args: Vec<String> = vec!["invoke".to_string(), name.clone(), target.clone()];

        // Env: explicit allow-list per ADR-010 (no FORGEPLAN_* leakage).
        // PATH/HOME/USER are always passed; plugins typically resolve their
        // own helpers via PATH so we never strip it.
        let base_env: HashMap<String, String> = std::env::vars().collect();
        let env = build_env_allowlist(&[], &base_env);

        let timeout = step
            .timeout_seconds
            .map(|s| Duration::from_secs(u64::from(s)))
            .unwrap_or(self.default_timeout);

        let spec = SubprocessSpec {
            program: &program_str,
            args: &args,
            env: &env,
            cwd: Some(self.workspace_root.as_path()),
            timeout,
            stdin_data: None,
        };

        let outcome = helpers::run_subprocess(spec).await?;

        // Surface a synthetic stderr message on timeout: helpers returns
        // empty stderr/stdout when the timeout fires (child killed before
        // it could write), so without this the step failure shows up with
        // no diagnostic. Non-empty captured stderr is preferred when both
        // a partial write and a timeout coincide — keep the real bytes.
        let stderr_text = if !outcome.stderr.is_empty() {
            Some(String::from_utf8_lossy(&outcome.stderr).into_owned())
        } else if outcome.timed_out {
            Some(format!(
                "plugin `{name}/{target}` timed out after {}s",
                timeout.as_secs()
            ))
        } else {
            None
        };

        Ok(DispatchOutcome {
            success: outcome.exit_code == Some(0) && !outcome.timed_out,
            output_path: step.produces_at.clone(),
            stderr: stderr_text,
        })
    }
}

/// Local `which` re-implementation — kept private to avoid importing the
/// helpers module's private fn. Searches `$PATH` for `program`.
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

// =====================================================================
// Silence unused-import lint while `resolve_forgeplan_binary` /
// `DEFAULT_TIMEOUT_SECS` are imported for future variants.
// =====================================================================

#[allow(dead_code)]
fn _keep_imports_alive(workspace: &Path) -> Option<PathBuf> {
    let _default = DEFAULT_TIMEOUT_SECS;
    resolve_forgeplan_binary(workspace)
}

// =====================================================================
// Tests
// =====================================================================
//
// Tests cover the dispatcher's pure-logic surface (delegation matching,
// binary resolution, error mapping). Tests that exercise real subprocess
// execution route through `helpers::run_subprocess` which is owned by
// `phase6-helpers-author` and currently `unimplemented!()`. Those tests
// are gated behind `#[ignore]` so they don't panic during Wave 1
// integration; once helpers lands the ignore can be dropped.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playbook::types::{Delegation, OnError, Step};
    use tokio::sync::Mutex;

    /// Serialize tests that mutate process-global state (`PATH`,
    /// `FORGEPLAN_BIN`). `cargo test` runs cases on multiple threads, so
    /// without this guard concurrent env mutations race and produce
    /// flaky results. Async-aware so it can be held across `await`
    /// points (clippy::await_holding_lock).
    static ENV_LOCK: Mutex<()> = Mutex::const_new(());

    async fn env_guard() -> tokio::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().await
    }

    fn plugin_step(id: &str, name: &str, target: &str) -> Step {
        Step {
            id: id.to_string(),
            delegate_to: Delegation::Plugin {
                name: name.to_string(),
                target: target.to_string(),
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: OnError::Abort,
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        }
    }

    fn agent_step(id: &str) -> Step {
        Step {
            id: id.to_string(),
            delegate_to: Delegation::Agent {
                name: "alpha".to_string(),
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: OnError::Abort,
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        }
    }

    /// Defensive: PluginDispatcher refuses any non-Plugin variant with a
    /// Transport error mentioning the step ID. Executor in normal flow
    /// won't route wrong variant here, but this preserves the invariant.
    #[tokio::test]
    async fn plugin_dispatcher_rejects_non_plugin_delegation() {
        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp"));
        let step = agent_step("wrong-variant");
        let err = dispatcher
            .dispatch(&step)
            .await
            .expect_err("must reject non-Plugin");
        match err {
            DispatchError::Transport(msg) => {
                assert!(
                    msg.contains("non-plugin"),
                    "error must mention non-plugin: {msg}"
                );
                assert!(
                    msg.contains("wrong-variant"),
                    "error must include step id: {msg}"
                );
            }
            other => panic!("expected Transport, got {other:?}"),
        }
    }

    /// AC-2 PRD-072: when no Task tool binary is available on PATH and no
    /// override is set, dispatcher returns `DelegateMissing` with the
    /// step's `fallback_hint` surfaced as the reason.
    #[tokio::test]
    async fn plugin_dispatcher_returns_delegate_missing_when_task_tool_absent() {
        let _guard = env_guard().await;
        // Snapshot + clear PATH so `which claude-code-plugin` cannot find
        // anything. SAFETY: serialized via ENV_LOCK; restored before
        // dropping the guard.
        let saved_path = std::env::var_os("PATH");
        unsafe {
            std::env::remove_var("PATH");
        }

        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp"));
        let mut step = plugin_step("plug-1", "c4-architecture", "c4-code");
        step.fallback_hint = Some("install xyz via brew install xyz".to_string());

        let err = dispatcher.dispatch(&step).await.expect_err("must fail");

        // Restore PATH before any assertion that could panic.
        unsafe {
            if let Some(p) = saved_path {
                std::env::set_var("PATH", p);
            }
        }

        match err {
            DispatchError::DelegateMissing { delegate, reason } => {
                assert_eq!(delegate, "plugin:c4-architecture");
                assert!(
                    reason.contains("install xyz"),
                    "fallback_hint must propagate verbatim: {reason}"
                );
            }
            other => panic!("expected DelegateMissing, got {other:?}"),
        }
    }

    /// AC-2 PRD-072 corollary: when `fallback_hint` is `None`, the default
    /// install message is surfaced (still actionable, mentions the
    /// missing tool name).
    #[tokio::test]
    async fn plugin_dispatcher_default_missing_message_when_no_fallback_hint() {
        let _guard = env_guard().await;
        let saved_path = std::env::var_os("PATH");
        unsafe {
            std::env::remove_var("PATH");
        }

        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp"));
        let step = plugin_step("plug-2", "autoresearch", "scan");

        let err = dispatcher.dispatch(&step).await.expect_err("must fail");

        unsafe {
            if let Some(p) = saved_path {
                std::env::set_var("PATH", p);
            }
        }

        match err {
            DispatchError::DelegateMissing { delegate, reason } => {
                assert_eq!(delegate, "plugin:autoresearch");
                assert!(
                    reason.contains("claude-code-plugin"),
                    "default reason must mention the tool name: {reason}"
                );
            }
            other => panic!("expected DelegateMissing, got {other:?}"),
        }
    }

    /// `with_task_tool` injection wins over `$PATH` lookup. Verifies the
    /// resolution order documented in [`PluginDispatcher::resolve_binary`].
    #[test]
    fn plugin_dispatcher_resolve_binary_prefers_explicit_override() {
        // No env mutation here — guard not needed. /bin/echo exists on
        // every Unix dev box; on Windows skip.
        let echo = PathBuf::from("/bin/echo");
        if !echo.is_file() {
            return;
        }
        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp")).with_task_tool(echo.clone());
        let resolved = dispatcher.resolve_binary().expect("must resolve");
        assert_eq!(resolved, echo);
    }

    /// `Default` dispatcher is rooted at a real path and reuses the same
    /// 600s default timeout as `new()`. Smoke check on the constructor
    /// surface to keep them in sync.
    #[test]
    fn plugin_dispatcher_default_matches_new_defaults() {
        let dispatcher = PluginDispatcher::default();
        assert_eq!(
            dispatcher.default_timeout,
            Duration::from_secs(DEFAULT_PLUGIN_TIMEOUT_SECS)
        );
        assert!(dispatcher.task_tool_path.is_none());
    }

    /// `with_default_timeout` overrides the constructor default. Used by
    /// tests + future config wiring.
    #[test]
    fn plugin_dispatcher_with_default_timeout_overrides() {
        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp"))
            .with_default_timeout(Duration::from_secs(42));
        assert_eq!(dispatcher.default_timeout, Duration::from_secs(42));
    }

    // -----------------------------------------------------------------
    // Subprocess-touching tests
    // -----------------------------------------------------------------
    //
    // These exercise the full path through `helpers::run_subprocess` —
    // active now that phase6-helpers-author has landed the helper.

    /// AC-1 PRD-072: when the configured executor exits 0, dispatcher
    /// reports `success: true` and surfaces the step's `produces_at`.
    /// Uses `/bin/echo` as the injected `task_tool_path` — it ignores all
    /// args and exits 0, which models a fast-path successful plugin.
    #[tokio::test]
    async fn plugin_dispatcher_invokes_command_and_captures_output() {
        let echo = PathBuf::from("/bin/echo");
        if !echo.is_file() {
            return;
        }
        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp")).with_task_tool(echo);
        let mut step = plugin_step("plug-success", "noop", "noop");
        step.produces_at = Some(PathBuf::from("out.md"));
        let outcome = dispatcher.dispatch(&step).await.expect("ok");
        assert!(outcome.success, "echo exits 0 → success=true");
        assert_eq!(outcome.output_path.as_deref(), Some(Path::new("out.md")));
    }

    /// FR-9 (lifecycle): dispatcher's `default_timeout` must reach the
    /// helper. We inject a hanging shell script via `with_task_tool`
    /// (regardless of the fixed `["invoke", name, target]` argv, the
    /// script just sleeps), set a 200 ms default, and assert
    /// `success == false` because the timeout fires.
    ///
    /// Once FR-8 lands as `Step.timeout_seconds`, the dispatcher will
    /// prefer the per-step value over `default_timeout` — the
    /// [`StepTimeoutExt`] shim returns `None` today so this test
    /// currently exercises the `default_timeout` branch.
    #[tokio::test]
    async fn plugin_dispatcher_propagates_step_timeout_seconds() {
        // Build a temp script that ignores its argv and sleeps long
        // enough to reliably trip the dispatcher's timeout.
        let dir = std::env::temp_dir().join(format!(
            "forgeplan-plugin-dispatcher-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let script = dir.join("hang.sh");
        std::fs::write(&script, "#!/bin/sh\nsleep 5\n").expect("write script");
        // 0o755 — readable + executable by owner, readable + executable by all.
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("chmod +x");

        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp"))
            .with_task_tool(script.clone())
            .with_default_timeout(Duration::from_millis(200));
        let step = plugin_step("plug-timeout", "noop", "noop");
        let outcome = dispatcher.dispatch(&step).await.expect("ok");

        // Cleanup before assertions — best-effort.
        let _ = std::fs::remove_file(&script);
        let _ = std::fs::remove_dir(&dir);

        assert!(
            !outcome.success,
            "subprocess that outlives timeout must report failure"
        );
        // Synthetic diagnostic surfaces when helpers returns empty stderr
        // on timeout — without it, the step failure carries no reason.
        let stderr = outcome
            .stderr
            .as_deref()
            .expect("timed-out step must surface a stderr diagnostic");
        assert!(
            stderr.contains("timed out"),
            "stderr must carry a timeout diagnostic: {stderr}"
        );
    }
}
