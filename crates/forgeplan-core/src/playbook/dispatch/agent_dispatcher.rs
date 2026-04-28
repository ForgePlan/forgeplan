//! Production [`Dispatcher`] for `Delegation::Agent` variant (FR-2).
//!
//! Phase 6 Wave 1 — owner: **agent-dispatcher** teammate.
//!
//! Invokes a Claude Code subagent via the Task tool subprocess. In contrast
//! to [`super::plugin_dispatcher::PluginDispatcher`] (external installed
//! plugins), agents are **subagents inside the active Claude Code agent
//! context**. For Phase 6 v1 we surface both as Task-tool subprocess
//! invocations because that is the consistent execution surface today
//! (ADR-010). The split lives at the `Delegation` enum level so that future
//! work — e.g. an in-process subagent runtime — can replace this dispatcher
//! without touching plugin or skill paths.
//!
//! Captures stdout to `Step.produces_at`, enforces timeout (default 300s —
//! tighter than the 600s plugin default since subagents are typically
//! shorter-lived), respects `kill_on_drop`. See ADR-010 §Decision and
//! [`super::helpers::run_subprocess`].
//!
//! Symmetric with [`super::plugin_dispatcher`]; intentional differences are
//! documented inline (`args` shape, default timeout, env program label).

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;

use super::helpers::{self, SubprocessSpec};
use super::{DispatchError, DispatchOutcome, Dispatcher};
use crate::playbook::types::{Delegation, Step};

/// Default timeout for agent dispatch when `Step.timeout_seconds` is not
/// set. Lower than plugin default (600s) because subagents are usually
/// quicker — see ADR-010 §Trade-offs row "subprocess timeout policy".
const DEFAULT_AGENT_TIMEOUT_SECS: u64 = 300;

/// FR-2: Production agent dispatcher.
///
/// Resolves the Task-tool binary (or honors a test-injected override),
/// builds a [`SubprocessSpec`], and delegates lifecycle to
/// [`helpers::run_subprocess`]. Errors map onto [`DispatchError`] variants:
///
/// - Wrong delegate variant         → [`DispatchError::Transport`]
/// - Tool binary not found          → [`DispatchError::DelegateMissing`]
/// - Subprocess transport failure   → [`DispatchError::Transport`]
/// - Non-zero exit / timeout / kill → [`DispatchOutcome`] with `success=false`
pub struct AgentDispatcher {
    /// Workspace root — passed to subprocess as `cwd` so relative
    /// `produces_at` paths resolve correctly.
    pub workspace_root: PathBuf,
    /// Optional explicit path to the Task tool binary. When `None`, the
    /// dispatcher resolves via `$FORGEPLAN_TASK_TOOL` env override or
    /// `which task-tool` on `$PATH`.
    pub task_tool_path: Option<PathBuf>,
    /// Default timeout applied when `Step.timeout_seconds` is not set
    /// (Step does not yet expose this field — wired in FR-8).
    pub default_timeout: Duration,
}

impl AgentDispatcher {
    /// Construct with sensible defaults: 300s timeout, auto-resolved tool path.
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            task_tool_path: None,
            default_timeout: Duration::from_secs(DEFAULT_AGENT_TIMEOUT_SECS),
        }
    }

    /// Test/dev hook — inject explicit Task tool path (bypasses PATH lookup).
    pub fn with_task_tool_path(mut self, path: PathBuf) -> Self {
        self.task_tool_path = Some(path);
        self
    }

    /// Override the default subprocess timeout.
    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Resolve task tool path: explicit override → `$FORGEPLAN_TASK_TOOL`
    /// → `which task-tool`. Returns `None` if nothing on disk.
    fn resolve_task_tool(&self) -> Option<PathBuf> {
        if let Some(p) = &self.task_tool_path
            && p.is_file()
        {
            return Some(p.clone());
        }
        if let Ok(override_path) = std::env::var("FORGEPLAN_TASK_TOOL") {
            let p = PathBuf::from(override_path);
            if p.is_file() {
                return Some(p);
            }
        }

        which_in_path("task-tool")
    }
}

impl Default for AgentDispatcher {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

#[async_trait]
impl Dispatcher for AgentDispatcher {
    async fn dispatch(&self, step: &Step) -> Result<DispatchOutcome, DispatchError> {
        // 1. Variant guard — caller must not route a non-Agent step here.
        let agent_name = match &step.delegate_to {
            Delegation::Agent { name } => name.clone(),
            other => {
                return Err(DispatchError::Transport(format!(
                    "AgentDispatcher received non-Agent delegate: {other:?}",
                )));
            }
        };

        // 2. Resolve binary — DelegateMissing carries the install hint.
        let program = match self.resolve_task_tool() {
            Some(p) => p,
            None => {
                let hint = step
                    .fallback_hint
                    .clone()
                    .unwrap_or_else(|| "install Task tool runtime".to_string());
                return Err(DispatchError::DelegateMissing {
                    delegate: format!("agent:{agent_name}"),
                    reason: format!("Task tool binary not found on PATH. Hint: {hint}"),
                });
            }
        };

        // 3. Build args. Note shape distinction vs PluginDispatcher:
        //    plugin → ["plugin-invoke", &name, &target]
        //    agent  → ["agent-invoke",  &name]
        // Agents have no `target` field on the Delegation variant.
        let args: Vec<String> = vec!["agent-invoke".to_string(), agent_name.clone()];

        // 4. Compose env allow-list — base PATH/HOME/USER plus agent-specific.
        let base_env: HashMap<String, String> = std::env::vars().collect();
        let env = helpers::build_env_allowlist(&["FORGEPLAN_AGENT_CTX"], &base_env);

        // 5. Build subprocess spec. cwd = workspace_root so produces_at
        //    relative paths land where the executor expects them.
        let program_str = program.to_string_lossy().into_owned();
        let spec = SubprocessSpec {
            program: &program_str,
            args: &args,
            env: &env,
            cwd: Some(&self.workspace_root),
            timeout: self.default_timeout,
            stdin_data: None,
        };

        // 6. Execute. Helper translates lifecycle into outcome / Transport.
        let outcome = helpers::run_subprocess(spec).await?;

        // 7. Map subprocess outcome → DispatchOutcome.
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

/// Local copy of `which_in_path` — helpers::which_in_path is private. Kept
/// minimal: searches `$PATH`, returns first hit. If a third dispatcher
/// needs this we promote it to `helpers` (coordinate with helpers-author).
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
        }
    }

    /// Construction defaults: 300s timeout (vs plugin's 600s), auto path.
    #[test]
    fn new_uses_300s_default_timeout() {
        let d = AgentDispatcher::new(PathBuf::from("/tmp/ws"));
        assert_eq!(d.default_timeout, Duration::from_secs(300));
        assert!(d.task_tool_path.is_none());
        assert_eq!(d.workspace_root, PathBuf::from("/tmp/ws"));
    }

    /// Builder hooks override defaults.
    #[test]
    fn builder_hooks_override_defaults() {
        let d = AgentDispatcher::new(PathBuf::from("/ws"))
            .with_default_timeout(Duration::from_secs(42))
            .with_task_tool_path(PathBuf::from("/usr/local/bin/task-tool"));
        assert_eq!(d.default_timeout, Duration::from_secs(42));
        assert_eq!(
            d.task_tool_path.as_deref(),
            Some(std::path::Path::new("/usr/local/bin/task-tool"))
        );
    }

    /// Wrong delegate variant is a programming error → Transport.
    /// Variant guard fires before subprocess is touched, so this is safe
    /// to test even while `helpers::run_subprocess` is still a stub.
    #[tokio::test]
    async fn dispatch_rejects_non_agent_variant() {
        let d = AgentDispatcher::new(PathBuf::from("."));
        let step = make_step(
            "wrong",
            Delegation::Plugin {
                name: "p".into(),
                target: "t".into(),
            },
        );
        let err = d.dispatch(&step).await.expect_err("must reject");
        match err {
            DispatchError::Transport(msg) => {
                assert!(msg.contains("non-Agent delegate"), "unexpected msg: {msg}");
            }
            other => panic!("expected Transport, got {other:?}"),
        }
    }

    /// Missing task tool → DelegateMissing carrying step.fallback_hint.
    /// We force resolve_task_tool to fail by pointing the override at a
    /// non-existent path AND ensuring `task-tool` isn't on the test PATH.
    #[tokio::test]
    async fn dispatch_emits_delegate_missing_when_tool_absent() {
        // Isolate from real PATH so `which_in_path("task-tool")` is None.
        // SAFETY: test-local env mutation; we restore at the end.
        let original_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", "/nonexistent-dir-for-test-isolation");
            std::env::remove_var("FORGEPLAN_TASK_TOOL");
        }

        let d = AgentDispatcher::new(PathBuf::from("."))
            .with_task_tool_path(PathBuf::from("/no/such/binary"));
        let mut step = make_step(
            "miss",
            Delegation::Agent {
                name: "auditor".into(),
            },
        );
        step.fallback_hint = Some("brew install task-tool".to_string());

        let result = d.dispatch(&step).await;

        // Restore PATH before asserting so failure messages render correctly.
        unsafe {
            match original_path {
                Some(v) => std::env::set_var("PATH", v),
                None => std::env::remove_var("PATH"),
            }
        }

        let err = result.expect_err("must surface DelegateMissing");
        match err {
            DispatchError::DelegateMissing { delegate, reason } => {
                assert_eq!(delegate, "agent:auditor");
                assert!(
                    reason.contains("brew install task-tool"),
                    "reason: {reason}"
                );
            }
            other => panic!("expected DelegateMissing, got {other:?}"),
        }
    }

    /// Default fallback hint when step did not provide one.
    #[tokio::test]
    async fn dispatch_uses_default_hint_when_step_omits_one() {
        let original_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", "/nonexistent-dir-for-test-isolation-2");
            std::env::remove_var("FORGEPLAN_TASK_TOOL");
        }

        let d = AgentDispatcher::new(PathBuf::from("."))
            .with_task_tool_path(PathBuf::from("/no/such/binary"));
        let step = make_step(
            "miss-no-hint",
            Delegation::Agent {
                name: "reviewer".into(),
            },
        );

        let result = d.dispatch(&step).await;

        unsafe {
            match original_path {
                Some(v) => std::env::set_var("PATH", v),
                None => std::env::remove_var("PATH"),
            }
        }

        let err = result.expect_err("must surface DelegateMissing");
        match err {
            DispatchError::DelegateMissing { delegate, reason } => {
                assert_eq!(delegate, "agent:reviewer");
                assert!(
                    reason.contains("install Task tool runtime"),
                    "reason: {reason}"
                );
            }
            other => panic!("expected DelegateMissing, got {other:?}"),
        }
    }

    /// Resolution prefers the explicit `task_tool_path` when it exists on disk.
    /// We use `cargo` (always present in Rust workspace) as a stand-in to
    /// verify resolution semantics without coupling to `task-tool` install.
    #[test]
    fn resolve_task_tool_prefers_explicit_path() {
        let cargo_path = which_in_path("cargo");
        let Some(cargo) = cargo_path else {
            // Skip on environments without cargo (CI of forgeplan always has it).
            return;
        };
        let d = AgentDispatcher::new(PathBuf::from(".")).with_task_tool_path(cargo.clone());
        let resolved = d.resolve_task_tool().expect("explicit path must resolve");
        assert_eq!(resolved, cargo);
    }

    /// `Default::default` constructs without panicking and uses cwd-relative
    /// workspace root.
    #[test]
    fn default_impl_does_not_panic() {
        let d = AgentDispatcher::default();
        assert_eq!(d.workspace_root, PathBuf::from("."));
        assert_eq!(d.default_timeout, Duration::from_secs(300));
    }
}
