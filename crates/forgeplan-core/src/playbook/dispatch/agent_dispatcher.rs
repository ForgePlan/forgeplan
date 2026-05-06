//! Production [`Dispatcher`] for `Delegation::Agent` variant (FR-2).
//!
//! Phase B Wave 1B — owner: **rust-agent** teammate.
//! References: PRD-072 §FR-2, ADR-011 §Decision, EVID-093 (claude --print spike).
//!
//! # Invocation mechanism (ADR-011)
//!
//! Phase B replaces the fictional `task-tool agent-invoke` shape with the real
//! `claude --print --agent <name>` headless invocation. Argv shape:
//!
//! ```text
//! claude --print --agent <name> --output-format json \
//!        --max-budget-usd <N> \
//!        --allowedTools <T1> <T2> ... \
//!        [--add-dir <abs-parent-of-produces_at>]
//! ```
//!
//! The user-visible prompt is supplied via stdin (NOT argv) because the
//! variadic `--allowedTools` would otherwise consume it. JSON output is
//! mandatory: exit-code alone cannot distinguish a budget cap from an API
//! error.
//!
//! Helpers in [`super::claude_print`] compose argv values + prompt + envelope
//! parsing. Pre-Wave 0 pinned that contract with 11 unit tests; this module
//! only orchestrates.
//!
//! # Differences vs [`super::plugin_dispatcher::PluginDispatcher`]
//!
//! Both dispatchers now shell out to `claude --print`. The split is at the
//! `Delegation` enum level so future routing (e.g. an in-process subagent
//! runtime) can replace this without touching plugin paths. The wire-level
//! difference is just the absence of a `target` argument — agents identify
//! by `name` alone.
//!
//! # Invariants
//!
//! - `claude` must be discoverable on PATH or via an explicit override
//!   ([`AgentDispatcher::with_claude_binary`]). The `$FORGEPLAN_CLAUDE_BIN`
//!   environment variable is honoured **only in test builds**
//!   (`#[cfg(test)]`) — release binaries silently ignore it. This closes
//!   CWE-426 (uncontrolled search path / binary substitution) per
//!   PROB-050 A-14 (audit S-2 escalation 2026-05-03). Otherwise
//!   [`DispatchError::DelegateMissing`].
//! - Agent name is validated against `^[A-Za-z][A-Za-z0-9_-]{0,63}$` BEFORE
//!   argv construction (argv-injection guard, ADR-011 §Security). Shared
//!   validator lives in [`super::claude_print::validate_agent_name`].
//! - Subprocess lifecycle (timeout / kill_on_drop / env allow-list) goes
//!   through [`helpers::run_subprocess`] — single source of truth (ADR-010).
//! - `--max-budget-usd` is always passed (default $1.00 from
//!   [`super::claude_print::DEFAULT_BUDGET_USD`]).
//! - Default per-step timeout is 300s (subagents shorter-lived than plugins).

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;

use super::claude_print;
use super::{DispatchError, DispatchOutcome, Dispatcher};
use crate::playbook::types::{Delegation, Step};

/// Default timeout for agent dispatch when `Step.timeout_seconds` is not
/// set. Lower than plugin default (600s) because subagents are usually
/// quicker — see ADR-010 §Trade-offs row "subprocess timeout policy".
const DEFAULT_AGENT_TIMEOUT_SECS: u64 = 300;

/// Default binary searched on `PATH` when no explicit override is provided.
/// Per ADR-011 §Decision: invoke the real Claude Code CLI.
const DEFAULT_AGENT_BINARY: &str = "claude";

/// FR-2: Production agent dispatcher.
///
/// Invokes `claude --print --agent <name>` per ADR-011. Resolves the
/// binary (or honours a test-injected override), validates the agent name,
/// builds a [`SubprocessSpec`] piping the assembled prompt on stdin, and
/// delegates lifecycle to [`helpers::run_subprocess`]. Errors map onto
/// [`DispatchError`] variants:
///
/// - Wrong delegate variant            → [`DispatchError::Transport`]
/// - Agent name fails validation       → [`DispatchError::Transport`]
/// - `claude` binary not found         → [`DispatchError::DelegateMissing`]
/// - Subprocess transport failure      → [`DispatchError::Transport`]
/// - Non-zero exit / timeout / kill /  → [`DispatchOutcome`] with
///   API error in JSON envelope         `success=false`
pub struct AgentDispatcher {
    /// Workspace root — passed to subprocess as `cwd` so relative
    /// `produces_at` paths resolve correctly.
    workspace_root: PathBuf,
    /// Optional explicit path to the `claude` binary. When `None`, the
    /// dispatcher resolves via `which claude` on `$PATH`. In test builds
    /// the `$FORGEPLAN_CLAUDE_BIN` env override is also consulted ahead of
    /// `PATH` — release builds silently ignore it (CWE-426 hardening,
    /// PROB-050 A-14). Field is private (PR-E Round 6 audit fix): the
    /// only entry-point is [`Self::with_claude_binary`], itself gated to
    /// `#[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]`.
    /// Without this private + cfg-gate combo a release-build caller could
    /// write to the field directly, defeating the env-var hardening.
    claude_binary: Option<PathBuf>,
    /// Default timeout applied when `Step.timeout_seconds` is not set.
    default_timeout: Duration,
}

impl AgentDispatcher {
    /// Construct with sensible defaults: 300s timeout, auto-resolved
    /// `claude` binary path.
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            claude_binary: None,
            default_timeout: Duration::from_secs(DEFAULT_AGENT_TIMEOUT_SECS),
        }
    }

    /// Test/dev hook — inject explicit `claude` binary path (bypasses PATH lookup).
    ///
    /// **Security boundary (CWE-426 / PROB-050 A-14)**: this builder is
    /// gated to `#[cfg(any(test, all(feature = "test-helpers",
    /// debug_assertions)))]` so release binaries cannot be coerced into
    /// invoking an attacker-supplied path. Pattern mirrors
    /// `LanceStore` test-helper gating
    /// (`crates/forgeplan-core/src/db/store.rs:361-384`):
    /// `debug_assertions` in the cfg ensures that a downstream consumer
    /// who accidentally enables the `test-helpers` feature in a release
    /// (`--release`) build still gets a compile error, not a silent
    /// activation of the bypass.
    #[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
    pub fn with_claude_binary(mut self, path: PathBuf) -> Self {
        self.claude_binary = Some(path);
        self
    }

    /// Deprecated alias for [`Self::with_claude_binary`]. Pre-Phase B name
    /// (`task-tool` did not actually exist — see ADR-011). Kept for one
    /// release cycle so downstream test wiring compiles unchanged; remove
    /// in the post-Phase-B cleanup pass. Same cfg gate as
    /// `with_claude_binary` for the same security reason.
    #[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
    #[deprecated(
        since = "0.27.0",
        note = "use `with_claude_binary`; ADR-011 replaces `task-tool` with `claude --print`"
    )]
    pub fn with_task_tool_path(self, path: PathBuf) -> Self {
        self.with_claude_binary(path)
    }

    /// Override the default subprocess timeout.
    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Resolve the `claude` binary: explicit override → `which claude` on
    /// `$PATH`. Returns `None` if nothing on disk.
    ///
    /// In **test builds** (`#[cfg(test)]`) the `$FORGEPLAN_CLAUDE_BIN`
    /// environment variable is consulted between the explicit override and
    /// the `PATH` lookup, allowing test wiring to redirect spawn targets
    /// without mutating `PATH`. Release builds silently ignore this env
    /// var to close CWE-426 (binary-substitution attack surface) per
    /// PROB-050 A-14.
    fn resolve_claude_binary(&self) -> Option<PathBuf> {
        // PROB-052 Round 7 audit HIGH-1 closure: explicit `claude_binary`
        // override field MUST go through `resolve_safe_path` so the same
        // canonicalize + perm gate that closed PATH-search applies here too.
        // Pre-Round-7 a bare `is_file()` left CWE-426 hijack exploitable
        // through this branch — operator config supplied a symlink in
        // group-writable Homebrew dir → bypass.
        if let Some(p) = &self.claude_binary
            && let Ok(Some(real)) = super::helpers::resolve_safe_path(p)
        {
            return Some(real);
        }
        // PROB-050 A-14 + PROB-052 HIGH-2: test-only env override also
        // routes through resolve_safe_path so the test surface mirrors
        // production validation.
        #[cfg(test)]
        if let Ok(override_path) = std::env::var("FORGEPLAN_CLAUDE_BIN")
            && let Ok(Some(p)) = super::helpers::resolve_safe_path(&PathBuf::from(override_path))
        {
            return Some(p);
        }
        super::helpers::which_in_path(DEFAULT_AGENT_BINARY)
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

        // 2. Argv-injection guard (ADR-011 §Security). Validate BEFORE we
        //    touch the filesystem or spawn anything — a malformed name like
        //    `--allowedTools` would otherwise be parsed as a flag by claude.
        if let Err(reason) = claude_print::validate_agent_name(&agent_name) {
            return Err(DispatchError::Transport(reason));
        }

        // 3. Resolve binary — DelegateMissing carries the install hint.
        let program = match self.resolve_claude_binary() {
            Some(p) => p,
            None => {
                let hint = step.fallback_hint.clone().unwrap_or_else(|| {
                    "install Claude Code CLI (https://claude.com/claude-code)".to_string()
                });
                return Err(DispatchError::DelegateMissing {
                    delegate: format!("agent:{agent_name}"),
                    reason: format!("`claude` binary not found on PATH. Hint: {hint}"),
                });
            }
        };

        // 4-9: Resolve timeout + delegate to shared invoke().
        // PROB-050 A-4 closure: argv build + env + prompt + spawn + parse
        // + render is the same 9-step recipe both dispatchers ran. Lives
        // in `claude_print::invoke` now — see that function for the full
        // sequence. Per-step timeout override (PRD-072 FR-8) is computed
        // here because it depends on the dispatcher's own default.
        let timeout = step
            .timeout_seconds
            .map(|s| Duration::from_secs(u64::from(s)))
            .unwrap_or(self.default_timeout);
        claude_print::invoke(
            &format!("agent `{agent_name}`"),
            &agent_name,
            step,
            &self.workspace_root,
            &program,
            timeout,
        )
        .await
    }
}

// PROB-050 A-5 closure: `which_in_path` consolidated into
// `super::helpers::which_in_path` (was: 3 identical copies, one per
// dispatcher). All call sites now use the shared `pub(super) fn`.

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playbook::types::{Delegation, OnError};

    /// PROB-050 A-6 closure: serialization lock moved to
    /// `super::claude_print::DISPATCH_ENV_LOCK` so `helpers::tests` and
    /// `plugin_dispatcher::tests` share the same guard. This alias keeps
    /// existing test bodies (`ENV_GUARD.lock().await`) compiling without
    /// rewrite.
    use super::claude_print::DISPATCH_ENV_LOCK as ENV_GUARD;

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
            budget_usd: None,
            allowed_tools: None,
        }
    }

    /// Construction defaults: 300s timeout (vs plugin's 600s), auto path.
    #[test]
    fn new_uses_300s_default_timeout() {
        let d = AgentDispatcher::new(PathBuf::from("/tmp/ws"));
        assert_eq!(d.default_timeout, Duration::from_secs(300));
        assert!(d.claude_binary.is_none());
        assert_eq!(d.workspace_root, PathBuf::from("/tmp/ws"));
    }

    /// Builder hooks override defaults.
    #[test]
    fn builder_hooks_override_defaults() {
        let d = AgentDispatcher::new(PathBuf::from("/ws"))
            .with_default_timeout(Duration::from_secs(42))
            .with_claude_binary(PathBuf::from("/usr/local/bin/claude"));
        assert_eq!(d.default_timeout, Duration::from_secs(42));
        assert_eq!(
            d.claude_binary.as_deref(),
            Some(std::path::Path::new("/usr/local/bin/claude"))
        );
    }

    /// Deprecated alias still routes through to `with_claude_binary`.
    #[test]
    #[allow(deprecated)]
    fn deprecated_with_task_tool_path_still_sets_claude_binary() {
        let d = AgentDispatcher::new(PathBuf::from("/ws"))
            .with_task_tool_path(PathBuf::from("/usr/local/bin/claude"));
        assert_eq!(
            d.claude_binary.as_deref(),
            Some(std::path::Path::new("/usr/local/bin/claude"))
        );
    }

    /// Wrong delegate variant is a programming error → Transport.
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

    /// ARGV-INJECTION GUARD: a name like `--allowedTools` must be rejected
    /// BEFORE any spawn. We assert that we get `Transport`, not
    /// `DelegateMissing` — the validator fires before binary resolution.
    #[tokio::test]
    async fn dispatch_rejects_invalid_agent_name_for_argv_injection() {
        // Point claude_binary at a real file so resolve_claude_binary
        // would succeed if we got that far — proves the validator fires
        // ahead of resolution.
        let cargo = super::super::helpers::which_in_path("cargo")
            .unwrap_or_else(|| PathBuf::from("/bin/sh"));
        let d = AgentDispatcher::new(PathBuf::from(".")).with_claude_binary(cargo);
        let step = make_step(
            "evil",
            Delegation::Agent {
                name: "--allowedTools".into(),
            },
        );
        let err = d.dispatch(&step).await.expect_err("must reject");
        match err {
            DispatchError::Transport(msg) => {
                assert!(
                    msg.contains("--allowedTools") && msg.contains("argv-injection"),
                    "unexpected msg: {msg}"
                );
            }
            other => panic!("expected Transport (validator failure), got {other:?}"),
        }
    }

    /// Missing claude binary → DelegateMissing carrying step.fallback_hint.
    #[allow(clippy::await_holding_lock)] // ENV_GUARD pins env vars across spawn for test isolation
    #[tokio::test]
    async fn dispatch_emits_delegate_missing_when_tool_absent() {
        let _guard = ENV_GUARD.lock().await;
        // Isolate from real PATH so `which_in_path("claude")` is None.
        let original_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", "/nonexistent-dir-for-test-isolation");
            std::env::remove_var("FORGEPLAN_CLAUDE_BIN");
        }

        let d = AgentDispatcher::new(PathBuf::from("."))
            .with_claude_binary(PathBuf::from("/no/such/binary"));
        let mut step = make_step(
            "miss",
            Delegation::Agent {
                name: "auditor".into(),
            },
        );
        step.fallback_hint = Some("brew install claude-code".to_string());

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
                assert_eq!(delegate, "agent:auditor");
                assert!(
                    reason.contains("brew install claude-code"),
                    "reason: {reason}"
                );
            }
            other => panic!("expected DelegateMissing, got {other:?}"),
        }
    }

    /// Default fallback hint when step did not provide one.
    #[allow(clippy::await_holding_lock)] // ENV_GUARD pins env vars across spawn for test isolation
    #[tokio::test]
    async fn dispatch_uses_default_hint_when_step_omits_one() {
        let _guard = ENV_GUARD.lock().await;
        let original_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", "/nonexistent-dir-for-test-isolation-2");
            std::env::remove_var("FORGEPLAN_CLAUDE_BIN");
        }

        let d = AgentDispatcher::new(PathBuf::from("."))
            .with_claude_binary(PathBuf::from("/no/such/binary"));
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
                assert!(reason.contains("Claude Code CLI"), "reason: {reason}");
            }
            other => panic!("expected DelegateMissing, got {other:?}"),
        }
    }

    /// Resolution prefers the explicit `claude_binary` when it exists on disk.
    #[test]
    fn resolve_claude_binary_prefers_explicit_path() {
        let cargo_path = super::super::helpers::which_in_path("cargo");
        let Some(cargo) = cargo_path else {
            return;
        };
        let d = AgentDispatcher::new(PathBuf::from(".")).with_claude_binary(cargo.clone());
        let resolved = d
            .resolve_claude_binary()
            .expect("explicit path must resolve");
        assert_eq!(resolved, cargo);
    }

    /// PROB-050 A-14 cfg-gate guard: in test builds the
    /// `$FORGEPLAN_CLAUDE_BIN` env override is honoured ahead of `PATH`
    /// when no explicit `with_claude_binary` is set. This pins the
    /// behaviour so a future refactor that accidentally removes or widens
    /// the `#[cfg(test)]` gate (or deletes the whole branch) breaks a test
    /// rather than silently regressing the surface that tests rely on.
    ///
    /// Counterpart for the **release-build** half of the contract (env
    /// MUST be ignored) is enforced compile-time by `#[cfg(test)]` itself
    /// — there is no runtime test we can write that exercises a release
    /// binary without orchestrating an external `cargo build --release`,
    /// which is too expensive for unit tests.
    #[tokio::test]
    async fn resolve_claude_binary_honours_env_override_in_test_builds() {
        let _guard = ENV_GUARD.lock().await;
        let cargo_path = super::super::helpers::which_in_path("cargo");
        let Some(cargo) = cargo_path else {
            return; // CI without cargo on PATH — skip rather than fail.
        };
        // Isolate from any developer-shell-exported var, then set ours.
        // SAFETY: ENV_GUARD serialises tests that mutate process-global env.
        unsafe {
            std::env::set_var("FORGEPLAN_CLAUDE_BIN", cargo.as_os_str());
        }
        let d = AgentDispatcher::new(PathBuf::from("."));
        let resolved = d.resolve_claude_binary();
        // SAFETY: cleanup before any other test runs; mirrors the pattern
        // at lines ~502/~545 where remove_var defends against pollution.
        unsafe {
            std::env::remove_var("FORGEPLAN_CLAUDE_BIN");
        }
        assert_eq!(
            resolved.as_deref(),
            Some(cargo.as_path()),
            "cfg(test) gate must keep env override reachable in test builds"
        );
    }

    /// `Default::default` constructs without panicking.
    #[test]
    fn default_impl_does_not_panic() {
        let d = AgentDispatcher::default();
        assert_eq!(d.workspace_root, PathBuf::from("."));
        assert_eq!(d.default_timeout, Duration::from_secs(300));
    }

    // =================================================================
    // ADR-011 argv shape + JSON envelope tests via fake `claude` binary.
    //
    // helpers::run_subprocess applies `env_clear()` + an allow-list, so
    // the child only sees PATH/HOME/USER. We can't pass capture targets
    // via env — instead each test writes a shell script with the target
    // paths spliced in literally. The script:
    //   1. Records argv (one arg per line) at a known path.
    //   2. Drains stdin (optionally to a known path).
    //   3. Prints a configurable JSON envelope on stdout.
    // =================================================================

    fn make_executable(path: &std::path::Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms).unwrap();
        }
        #[cfg(not(unix))]
        let _ = path;
    }

    /// AC: argv carries `--print --agent <name> --output-format json
    /// --max-budget-usd <N> --allowedTools <T>...`.
    #[allow(clippy::await_holding_lock)] // ENV_GUARD pins env vars across spawn for test isolation
    #[tokio::test]
    async fn dispatch_uses_claude_print_argv() {
        let _guard = ENV_GUARD.lock().await;
        let tmp = tempfile::tempdir().expect("tmpdir");
        let argv_out = tmp.path().join("argv.txt");
        let stdin_out = tmp.path().join("stdin.txt");
        let script = format!(
            r#"#!/bin/sh
: > "{argv}"
for a in "$@"; do
  printf '%s\n' "$a" >> "{argv}"
done
cat > "{stdin}"
printf '%s' '{json}'
"#,
            argv = argv_out.display(),
            stdin = stdin_out.display(),
            json =
                r#"{"is_error": false, "result": "ok", "total_cost_usd": 0.42, "duration_ms": 10}"#,
        );
        let fake = tmp.path().join("fake-claude.sh");
        std::fs::write(&fake, script).unwrap();
        make_executable(&fake);

        let d = AgentDispatcher::new(tmp.path().to_path_buf()).with_claude_binary(fake);
        let yaml = serde_yaml::from_str("task: \"investigate auth\"").unwrap();
        let step = Step {
            id: "s1".into(),
            delegate_to: Delegation::Agent {
                name: "auditor".into(),
            },
            input: Some(yaml),
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: OnError::Abort,
            timeout_seconds: None,
            budget_usd: Some(2.50),
            allowed_tools: Some(vec!["Read".into(), "Grep".into()]),
        };

        let outcome = d.dispatch(&step).await.expect("dispatch ok");
        assert!(outcome.success, "expected success, got {outcome:?}");

        let argv = std::fs::read_to_string(&argv_out).expect("argv recorded");
        let lines: Vec<&str> = argv.lines().collect();
        assert_eq!(lines[0], "--print", "argv: {argv}");
        assert_eq!(lines[1], "--agent");
        assert_eq!(lines[2], "auditor");
        assert_eq!(lines[3], "--output-format");
        assert_eq!(lines[4], "json");
        assert_eq!(lines[5], "--max-budget-usd");
        assert_eq!(lines[6], "2.50");
        let tool_idx = lines
            .iter()
            .position(|l| *l == "--allowedTools")
            .expect("--allowedTools present");
        assert_eq!(lines[tool_idx + 1], "Read");
        assert_eq!(lines[tool_idx + 2], "Grep");

        let stdin_body = std::fs::read_to_string(&stdin_out).expect("stdin recorded");
        assert!(
            stdin_body.contains("investigate auth"),
            "stdin missing prompt body: {stdin_body}"
        );
    }

    /// AC: when `produces_at` is set, argv must contain `--add-dir <abs>`.
    #[allow(clippy::await_holding_lock)] // ENV_GUARD pins env vars across spawn for test isolation
    #[tokio::test]
    async fn dispatch_with_produces_at_includes_add_dir() {
        let _guard = ENV_GUARD.lock().await;
        let tmp = tempfile::tempdir().expect("tmpdir");
        let argv_out = tmp.path().join("argv.txt");
        let script = format!(
            r#"#!/bin/sh
: > "{argv}"
for a in "$@"; do
  printf '%s\n' "$a" >> "{argv}"
done
cat > /dev/null
printf '{json}'
"#,
            argv = argv_out.display(),
            json = r#"{"is_error": false, "result": "ok"}"#,
        );
        let fake = tmp.path().join("fake-claude.sh");
        std::fs::write(&fake, script).unwrap();
        make_executable(&fake);

        let workspace = tmp.path().to_path_buf();
        let d = AgentDispatcher::new(workspace.clone()).with_claude_binary(fake);
        let step = Step {
            id: "s1".into(),
            delegate_to: Delegation::Agent {
                name: "writer".into(),
            },
            input: None,
            produces_at: Some(PathBuf::from("reports/out.md")),
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: OnError::Abort,
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        };

        let _ = d.dispatch(&step).await.expect("dispatch ok");

        let argv = std::fs::read_to_string(&argv_out).expect("argv recorded");
        let lines: Vec<&str> = argv.lines().collect();
        let idx = lines
            .iter()
            .position(|l| *l == "--add-dir")
            .expect("--add-dir present");
        let abs_expected = workspace.join("reports");
        assert_eq!(
            std::path::Path::new(lines[idx + 1]),
            abs_expected.as_path(),
            "argv: {argv}"
        );
    }

    /// AC: stdout JSON envelope with `is_error: false` → success.
    #[allow(clippy::await_holding_lock)] // ENV_GUARD pins env vars across spawn for test isolation
    #[tokio::test]
    async fn dispatch_parses_success_json() {
        let _guard = ENV_GUARD.lock().await;
        let tmp = tempfile::tempdir().expect("tmpdir");
        let script = r#"#!/bin/sh
cat > /dev/null
printf '{"is_error": false, "result": "all good", "total_cost_usd": 0.01, "duration_ms": 7, "session_id": "sess-1"}'
"#;
        let fake = tmp.path().join("fake-claude.sh");
        std::fs::write(&fake, script).unwrap();
        make_executable(&fake);

        let d = AgentDispatcher::new(tmp.path().to_path_buf()).with_claude_binary(fake);
        let step = make_step(
            "s1",
            Delegation::Agent {
                name: "auditor".into(),
            },
        );
        let outcome = d.dispatch(&step).await.expect("dispatch ok");
        assert!(outcome.success);
        assert!(outcome.output_path.is_none());
        assert!(outcome.stderr.is_none() || outcome.stderr.as_deref() == Some(""));
    }

    /// AC: `is_error: true` with `api_error_status: rate_limited` → failure;
    /// stderr must mention `rate_limited`.
    #[allow(clippy::await_holding_lock)] // ENV_GUARD pins env vars across spawn for test isolation
    #[tokio::test]
    async fn dispatch_classifies_api_error_as_failure() {
        let _guard = ENV_GUARD.lock().await;
        let tmp = tempfile::tempdir().expect("tmpdir");
        let script = r#"#!/bin/sh
cat > /dev/null
printf '{"is_error": true, "api_error_status": "rate_limited", "result": "partial", "total_cost_usd": 0.05, "duration_ms": 3}'
"#;
        let fake = tmp.path().join("fake-claude.sh");
        std::fs::write(&fake, script).unwrap();
        make_executable(&fake);

        let d = AgentDispatcher::new(tmp.path().to_path_buf()).with_claude_binary(fake);
        let step = make_step(
            "s1",
            Delegation::Agent {
                name: "auditor".into(),
            },
        );
        let outcome = d.dispatch(&step).await.expect("dispatch returns Ok");
        assert!(!outcome.success, "API-error envelope must fail");
        let stderr = outcome.stderr.expect("stderr populated");
        assert!(
            stderr.contains("rate_limited"),
            "stderr should mention api_error_status: {stderr}"
        );
    }
}
