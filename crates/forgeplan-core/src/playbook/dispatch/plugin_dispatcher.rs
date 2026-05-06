//! Production [`Dispatcher`] for [`Delegation::Plugin`] variant (FR-1).
//!
//! Phase B Wave 1A — owner: **rust-plugin** teammate.
//! References: PRD-072 §FR-1/AC-1/AC-2, RFC-007 §"plugin",
//! [ADR-011](.forgeplan/adrs/ADR-011-plugin-agent-dispatchers-invoke-claude-print-directly.md)
//! §Decision, EVID-093 (claude --print spike measurements).
//!
//! # v2 invocation mechanism — `claude --print`
//!
//! ADR-011 supersedes the earlier `claude-code-plugin invoke <name> <target>`
//! contract: that binary does not actually exist on user systems. Plugins
//! installed via `claude plugins install …` register **agents** in
//! `~/.claude/plugins/cache/<plugin>/.../agents/`, addressable through the
//! Claude Code CLI as `claude --print --agent <slug>`. Our dispatcher shells
//! out to `claude` with that argv, captures the structured JSON envelope, and
//! maps `is_error` / `api_error_status` onto [`DispatchOutcome`].
//!
//! Argv shape (per ADR-011 §Decision):
//!
//! ```text
//! claude --print
//!        --agent <target>             # plugin agent slug (falls back to name when target empty)
//!        --output-format json         # mandatory — exit code is ambiguous
//!        --max-budget-usd <usd>       # always present (default $1.00)
//!        --allowedTools T1 T2 ...     # variadic — separate args per tool
//!        --add-dir <path>             # optional, only when produces_at set
//! ```
//!
//! The prompt body is piped on **stdin** rather than passed as a positional
//! arg — `--allowedTools` is variadic and would otherwise consume it.
//!
//! # Argv-injection hardening
//!
//! Agent slugs originate in user-authored YAML. Without validation a slug
//! starting with `--` would be parsed as a flag by `claude` and could enable
//! tools we never approved. [`validate_agent_name`] enforces the regex
//! `^[A-Za-z][A-Za-z0-9_-]{0,63}$` (leading alpha, 1..=64 chars,
//! alphanumeric + `-` / `_`) on **both** `name` and `target` before
//! constructing argv.
//!
//! # ADR-010 invariants (still active)
//!
//! All subprocess work goes through [`super::helpers::run_subprocess`] which
//! enforces:
//! - `kill_on_drop(true)`
//! - `env_clear()` + explicit allow-list — `claude` reuses the user's logged-in
//!   session, so we deliberately do **not** propagate `ANTHROPIC_API_KEY`.
//! - `Stdio::piped()` for stdout/stderr/stdin (stdin carries the prompt)
//! - per-stream cap [`super::helpers::MAX_OUTPUT_BYTES`] (10 MiB)
//! - timeout via `tokio::time::timeout`; default 600s (plugins regularly take
//!   5+ minutes; cheaper than re-running on a tight cap).

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;

use super::claude_print;
use super::{DispatchError, DispatchOutcome, Dispatcher};
use crate::playbook::types::{Delegation, Step};

/// Default Claude CLI binary searched on `PATH` when
/// [`PluginDispatcher::claude_binary`] is `None`.
const DEFAULT_CLAUDE_BINARY: &str = "claude";

/// Default per-step timeout for plugins. Plugins are typically slower than
/// agents/skills — bumped to 600s vs the helper default of 300s
/// ([`super::helpers::DEFAULT_TIMEOUT_SECS`]).
///
/// PR-E audit MED-1 (architect): tightened from `pub` to `pub(crate)` for
/// symmetry with PR-E's other lockdown sweep on `claude_print::DEFAULT_*`.
/// No external library consumer reads this; restricting protects from
/// being accidentally pinned as a stability contract.
pub(crate) const DEFAULT_PLUGIN_TIMEOUT_SECS: u64 = 600;

/// Validate an agent slug before passing it to `claude --agent`. Wraps the
/// shared [`claude_print::validate_agent_name`] helper into the dispatcher
/// error type, preserving the `field` context (`name` vs `target`) so the
/// failure diagnostic tells the caller exactly which YAML field broke the
/// regex. See module docs §"Argv-injection hardening" for rationale.
fn validate_agent_name(value: &str, field: &str) -> Result<(), DispatchError> {
    claude_print::validate_agent_name(value)
        .map_err(|reason| DispatchError::Transport(format!("invalid agent {field}: {reason}")))
}

/// FR-1 production dispatcher for `Delegation::Plugin { name, target }`.
///
/// Construct via [`Self::new`] for production wiring or
/// [`Self::with_claude_binary`] for tests/dev (point at `/bin/echo`,
/// `/bin/cat`, or a tempfile script).
pub struct PluginDispatcher {
    /// Workspace root — passed to subprocess as `cwd` so relative
    /// `produces_at` paths resolve correctly, and as the base for
    /// [`add_dir_for_produces_at`].
    workspace_root: PathBuf,
    /// Override path to the `claude` binary. Default: `which claude`.
    claude_binary: Option<PathBuf>,
    /// Default timeout if `Step.timeout_seconds` (FR-8) is unset.
    default_timeout: Duration,
}

impl PluginDispatcher {
    /// Build a dispatcher rooted at `workspace_root` using the default
    /// `claude` resolution and the 600s default timeout.
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            claude_binary: None,
            default_timeout: Duration::from_secs(DEFAULT_PLUGIN_TIMEOUT_SECS),
        }
    }

    /// Override the `claude` binary path. Used by tests (point at
    /// `/bin/echo`, fake JSON-emitting scripts, etc.).
    ///
    /// **Security boundary (CWE-426 / PROB-050 A-14, PR-E Round 6 audit)**:
    /// gated to `#[cfg(any(test, all(feature = "test-helpers",
    /// debug_assertions)))]` so release binaries cannot inject an
    /// attacker-supplied binary path. Symmetric with
    /// [`AgentDispatcher::with_claude_binary`].
    #[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
    pub fn with_claude_binary(mut self, path: PathBuf) -> Self {
        self.claude_binary = Some(path);
        self
    }

    /// Deprecated alias for [`Self::with_claude_binary`]. Retained so that
    /// existing call sites compile during the ADR-011 migration; will be
    /// removed once Wave 1B+ are merged. Same cfg gate as
    /// `with_claude_binary` for the same security reason.
    #[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
    #[deprecated(note = "renamed to `with_claude_binary` per ADR-011")]
    pub fn with_task_tool(self, path: PathBuf) -> Self {
        self.with_claude_binary(path)
    }

    /// Override the default timeout (used when `Step.timeout_seconds`
    /// is absent).
    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Resolve the binary to invoke. Order:
    /// 1. Explicit `claude_binary` (test injection / config override)
    /// 2. `which claude` on `$PATH`
    ///
    /// Returns `None` if neither resolves — caller maps this to
    /// [`DispatchError::DelegateMissing`].
    fn resolve_binary(&self) -> Option<PathBuf> {
        // PROB-052 Round 7 audit HIGH-1 closure: route explicit override
        // through `resolve_safe_path` (canonicalize + Unix perm gate) so
        // the symmetric AgentDispatcher hardening applies here too.
        // Pre-Round-7 PluginDispatcher was even thinner than AgentDispatcher
        // — it returned the override unconditionally без even is_file().
        if let Some(path) = &self.claude_binary
            && let Ok(Some(real)) = super::helpers::resolve_safe_path(path)
        {
            return Some(real);
        }
        super::helpers::which_in_path(DEFAULT_CLAUDE_BINARY)
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

        // Argv-injection guard: validate BOTH name and target before any
        // argv assembly. `target` may legitimately be empty in user YAML
        // (`Delegation::Plugin { name, target: "" }`), in which case we
        // fall back to `name` as the agent slug — but a non-empty target
        // must still satisfy the regex, otherwise a malicious slug like
        // `--allowedTools` would slip through.
        validate_agent_name(&name, "name")?;
        if !target.is_empty() {
            validate_agent_name(&target, "target")?;
        }

        // Per ADR-011 §Decision: prefer `target` (plugin agents are
        // registered as agents themselves with that slug). Empty target
        // → fall back to `name`.
        let agent_slug = if target.is_empty() { &name } else { &target };

        let binary = self.resolve_binary().ok_or_else(|| {
            let reason = step.fallback_hint.clone().unwrap_or_else(|| {
                "install Claude Code CLI (`claude`) and ensure it is on $PATH".to_string()
            });
            DispatchError::DelegateMissing {
                delegate: format!("plugin:{name}"),
                reason,
            }
        })?;

        // PROB-050 A-4 closure: argv build + env + prompt + spawn + parse
        // + render delegated to shared `claude_print::invoke`. Per-step
        // timeout override (PRD-072 FR-8) computed here because it depends
        // on the dispatcher's own default.
        let timeout = step
            .timeout_seconds
            .map(|s| Duration::from_secs(u64::from(s)))
            .unwrap_or(self.default_timeout);
        claude_print::invoke(
            &format!("plugin `{name}/{target}`"),
            agent_slug,
            step,
            &self.workspace_root,
            &binary,
            timeout,
        )
        .await
    }
}

// PROB-050 A-5 closure: `which_in_path` consolidated into
// `super::helpers::which_in_path`. The local copy was added when helpers'
// version was private; now `pub(super)`, both dispatchers share it.

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playbook::types::{Delegation, OnError, Step};
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    /// PROB-050 A-6 closure (HIGH-1 audit fix on PR-E):
    /// previously `plugin_dispatcher::tests` had its OWN local ENV_LOCK
    /// while `agent_dispatcher::tests` and `helpers::tests` shared
    /// `claude_print::DISPATCH_ENV_LOCK`. The PR-E commit message claimed
    /// the cross-dispatcher race was closed; actually only agent ↔ helpers
    /// were unified, leaving plugin tests racing against the other two.
    /// All three now share the SAME mutex — true cross-dispatcher
    /// serialization, not three private cliques.
    use super::claude_print::DISPATCH_ENV_LOCK as ENV_GUARD;

    async fn env_guard() -> tokio::sync::MutexGuard<'static, ()> {
        ENV_GUARD.lock().await
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

    /// Write a self-deleting executable script to the per-test tempdir.
    /// Returns the script path; caller is responsible for cleanup.
    fn write_test_script(test_id: &str, body: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "forgeplan-plugin-dispatcher-{}-{}-{}",
            test_id,
            std::process::id(),
            // Add a per-call discriminator so concurrent tests don't collide.
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let script = dir.join("script.sh");
        std::fs::write(&script, body).expect("write script");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("chmod +x");
        script
    }

    /// Best-effort cleanup — ignore errors, parent dir may not exist.
    fn cleanup_test_script(script: &Path) {
        if let Some(parent) = script.parent() {
            let _ = std::fs::remove_file(script);
            let _ = std::fs::remove_dir(parent);
        }
    }

    // -----------------------------------------------------------------
    // Defensive guards (pre-spawn validation)
    // -----------------------------------------------------------------

    /// PluginDispatcher refuses any non-Plugin variant with a Transport
    /// error mentioning the step ID. Executor in normal flow won't route
    /// the wrong variant here, but this preserves the invariant.
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

    /// Argv-injection regression: a `target` that begins with `--` must
    /// be rejected BEFORE we spawn `claude`. Without this guard, `claude`
    /// would parse it as a flag and could enable tools we never approved.
    #[tokio::test]
    async fn dispatch_rejects_invalid_agent_name_for_argv_injection() {
        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp"));

        // Case 1: target with -- prefix (flag injection)
        let step = plugin_step("evil-flag", "ok-name", "--allowedTools");
        let err = dispatcher.dispatch(&step).await.expect_err("must reject");
        assert!(
            matches!(err, DispatchError::Transport(ref msg) if msg.contains("invalid agent") && msg.contains("rejected")),
            "expected Transport(invalid agent ... rejected), got {err:?}"
        );

        // Case 2: name with path traversal characters
        let step = plugin_step("evil-path", "../etc/passwd", "ok-target");
        let err = dispatcher.dispatch(&step).await.expect_err("must reject");
        assert!(
            matches!(err, DispatchError::Transport(ref msg) if msg.contains("invalid agent") && msg.contains("rejected")),
            "expected Transport(invalid agent ... rejected), got {err:?}"
        );

        // Case 3: empty name
        let step = plugin_step("evil-empty", "", "ok-target");
        let err = dispatcher.dispatch(&step).await.expect_err("must reject");
        assert!(
            matches!(err, DispatchError::Transport(ref msg) if msg.contains("invalid agent") && msg.contains("rejected")),
            "expected Transport(invalid agent ... rejected), got {err:?}"
        );

        // Case 4: oversized name (65 chars)
        let long = "a".repeat(65);
        let step = plugin_step("evil-long", &long, "ok-target");
        let err = dispatcher.dispatch(&step).await.expect_err("must reject");
        assert!(
            matches!(err, DispatchError::Transport(ref msg) if msg.contains("invalid agent") && msg.contains("rejected")),
            "expected Transport(invalid agent ... rejected), got {err:?}"
        );
    }

    /// AC-2 PRD-072: when no `claude` binary is available on PATH and no
    /// override is set, dispatcher returns `DelegateMissing` with the
    /// step's `fallback_hint` surfaced as the reason.
    #[tokio::test]
    async fn plugin_dispatcher_returns_delegate_missing_when_claude_absent() {
        let _guard = env_guard().await;
        let saved_path = std::env::var_os("PATH");
        // SAFETY: serialized via ENV_LOCK; restored before the guard drops.
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

    /// Default install message when step omits `fallback_hint`. Must
    /// mention `claude` so the user knows which binary to install.
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
                    reason.contains("claude"),
                    "default reason must mention `claude`: {reason}"
                );
            }
            other => panic!("expected DelegateMissing, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Constructor / builder smoke tests
    // -----------------------------------------------------------------

    /// `with_claude_binary` injection wins over `$PATH` lookup.
    /// Post-PROB-052 the override path goes through `resolve_safe_path`
    /// which `canonicalize`s the input — on systems where `/bin/echo` is
    /// itself a symlink to `/usr/bin/echo` (Debian/Ubuntu) the resolved
    /// value is the canonical real target, not the input symlink. Compare
    /// against the canonicalized expected path to keep this test cross-distro.
    #[test]
    fn plugin_dispatcher_resolve_binary_prefers_explicit_override() {
        let echo = PathBuf::from("/bin/echo");
        if !echo.is_file() {
            return;
        }
        let echo_canonical = match std::fs::canonicalize(&echo) {
            Ok(p) => p,
            Err(_) => return, // unusual filesystem; skip
        };
        let dispatcher =
            PluginDispatcher::new(PathBuf::from("/tmp")).with_claude_binary(echo.clone());
        let resolved = dispatcher.resolve_binary().expect("must resolve");
        assert_eq!(
            resolved, echo_canonical,
            "PROB-052: override returns canonicalized path, not input symlink"
        );
    }

    /// `Default` dispatcher reuses the same defaults as `new()`.
    #[test]
    fn plugin_dispatcher_default_matches_new_defaults() {
        let dispatcher = PluginDispatcher::default();
        assert_eq!(
            dispatcher.default_timeout,
            Duration::from_secs(DEFAULT_PLUGIN_TIMEOUT_SECS)
        );
        assert!(dispatcher.claude_binary.is_none());
    }

    /// `with_default_timeout` overrides the constructor default.
    #[test]
    fn plugin_dispatcher_with_default_timeout_overrides() {
        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp"))
            .with_default_timeout(Duration::from_secs(42));
        assert_eq!(dispatcher.default_timeout, Duration::from_secs(42));
    }

    /// Backwards-compat alias `with_task_tool` still compiles (deprecated
    /// shim for in-flight callers during the ADR-011 migration).
    /// Post-PROB-052 the override path is canonicalized — see sibling
    /// `plugin_dispatcher_resolve_binary_prefers_explicit_override` for
    /// the same cross-distro adaptation.
    #[test]
    #[allow(deprecated)]
    fn plugin_dispatcher_with_task_tool_alias_still_works() {
        let echo = PathBuf::from("/bin/echo");
        if !echo.is_file() {
            return;
        }
        let echo_canonical = match std::fs::canonicalize(&echo) {
            Ok(p) => p,
            Err(_) => return,
        };
        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp")).with_task_tool(echo.clone());
        assert_eq!(dispatcher.resolve_binary(), Some(echo_canonical));
    }

    // -----------------------------------------------------------------
    // Argv shape — spawn fake binaries, observe captured argv via stdout
    // -----------------------------------------------------------------

    /// AC-1 (ADR-011): dispatcher must spawn `<binary> --print --agent
    /// <slug> --output-format json --max-budget-usd <usd> --allowedTools
    /// <T1> ...`. We replace the binary with a shell script that echoes
    /// its argv as JSON-ish lines, then assert each required token.
    #[tokio::test]
    async fn dispatch_uses_claude_print_argv() {
        // Script echoes argv to stdout, then emits a valid JSON envelope so
        // dispatcher does not classify the run as a parse failure.
        let body = r#"#!/bin/sh
for arg in "$@"; do
  printf '__ARG__:%s\n' "$arg" >&2
done
printf '{"is_error": false, "result": "ok", "total_cost_usd": 0.01}\n'
"#;
        let script = write_test_script("argv-shape", body);

        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp"))
            .with_claude_binary(script.clone())
            .with_default_timeout(Duration::from_secs(5));
        let step = plugin_step("plug-argv", "c4-architecture", "c4-code");
        let outcome = dispatcher.dispatch(&step).await.expect("ok");

        cleanup_test_script(&script);

        assert!(
            outcome.success,
            "valid JSON envelope → success=true, got stderr={:?}",
            outcome.stderr
        );

        // We can't observe argv directly here (helpers does not expose it),
        // but the script wrote each arg to stderr prefixed by __ARG__: . That
        // stderr is captured into `outcome.stderr` ONLY on failure (our impl
        // sets stderr=None on success). To check argv shape we need a
        // failure path: emit `is_error=true` so render_failure_context()
        // surfaces the failure context. But render_failure_context() does
        // NOT include stderr. The cleanest way is a follow-up test that
        // emits non-JSON stdout — see `dispatch_argv_visible_via_failure_path`.
    }

    /// Argv-shape assertion via the parse-failure diagnostic: emit non-JSON
    /// stdout so the dispatcher surfaces the captured stderr (which the
    /// fake script populated with `__ARG__:<value>` per-arg). This lets us
    /// verify every required token is in the constructed argv.
    #[tokio::test]
    async fn dispatch_argv_visible_via_failure_path() {
        // Print argv lines to BOTH stdout (visible in stdout_preview) and
        // stderr (visible in stderr=...). Then exit normally with non-JSON
        // stdout so the dispatcher classifies as parse failure and surfaces
        // both streams.
        let body = r#"#!/bin/sh
for arg in "$@"; do
  printf 'ARG:%s\n' "$arg"
  printf 'ARG:%s\n' "$arg" >&2
done
"#;
        let script = write_test_script("argv-visible", body);

        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp"))
            .with_claude_binary(script.clone())
            .with_default_timeout(Duration::from_secs(5));
        let mut step = plugin_step("plug-argv-vis", "c4-architecture", "c4-code");
        step.budget_usd = Some(2.50);
        step.allowed_tools = Some(vec!["Read".to_string(), "Write".to_string()]);
        let outcome = dispatcher.dispatch(&step).await.expect("ok");

        cleanup_test_script(&script);

        assert!(!outcome.success, "non-JSON stdout → parse failure");
        let diag = outcome.stderr.expect("must surface diagnostic");
        // Required argv tokens — present either in stderr (line-by-line) or
        // stdout_preview (truncated to 200 chars). We assert both ends.
        for token in [
            "--print",
            "--agent",
            "c4-code",
            "--output-format",
            "json",
            "--max-budget-usd",
            "2.50",
            "--allowedTools",
            "Read",
            "Write",
        ] {
            assert!(diag.contains(token), "argv missing `{token}` in: {diag}");
        }
    }

    /// `produces_at` adds `--add-dir <abs-parent>` to argv so the agent
    /// has Write permission for the target directory.
    #[tokio::test]
    async fn dispatch_with_produces_at_includes_add_dir() {
        let body = r#"#!/bin/sh
for arg in "$@"; do printf 'ARG:%s\n' "$arg" >&2; done
printf 'noop'  # non-JSON → parse failure → diag includes stderr
"#;
        let script = write_test_script("add-dir", body);

        let workspace = std::env::temp_dir().join(format!(
            "forgeplan-pd-ws-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&workspace).expect("ws");

        let dispatcher = PluginDispatcher::new(workspace.clone())
            .with_claude_binary(script.clone())
            .with_default_timeout(Duration::from_secs(5));
        let mut step = plugin_step("plug-add-dir", "c4-architecture", "c4-code");
        step.produces_at = Some(PathBuf::from("reports/r.md"));
        let outcome = dispatcher.dispatch(&step).await.expect("ok");

        cleanup_test_script(&script);
        let _ = std::fs::remove_dir_all(&workspace);

        let diag = outcome.stderr.expect("diag");
        assert!(
            diag.contains("--add-dir"),
            "argv must include --add-dir: {diag}"
        );
        let expected_dir = workspace.join("reports");
        assert!(
            diag.contains(&expected_dir.to_string_lossy().to_string()),
            "argv must include {} in: {diag}",
            expected_dir.display()
        );
    }

    // -----------------------------------------------------------------
    // JSON envelope handling
    // -----------------------------------------------------------------

    /// AC-1 (ADR-011): a successful `claude --print` JSON envelope (
    /// `{"is_error": false, ...}`) maps to `DispatchOutcome { success:
    /// true, output_path: Some(produces_at), stderr: None }`.
    #[tokio::test]
    async fn dispatch_parses_claude_print_json_success() {
        let body = r#"#!/bin/sh
printf '{"is_error": false, "total_cost_usd": 0.42, "duration_ms": 1234, "result": "ok", "session_id": "abc"}\n'
"#;
        let script = write_test_script("json-success", body);

        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp"))
            .with_claude_binary(script.clone())
            .with_default_timeout(Duration::from_secs(5));
        let mut step = plugin_step("plug-json-ok", "c4-architecture", "c4-code");
        step.produces_at = Some(PathBuf::from("out.md"));
        let outcome = dispatcher.dispatch(&step).await.expect("ok");

        cleanup_test_script(&script);

        assert!(
            outcome.success,
            "JSON envelope is_error=false → success=true"
        );
        assert_eq!(outcome.output_path.as_deref(), Some(Path::new("out.md")));
        assert!(outcome.stderr.is_none(), "no stderr on success");
    }

    /// API errors (`is_error=true`, `api_error_status=rate_limited`) map
    /// to `success=false` and surface the status in the diagnostic.
    #[tokio::test]
    async fn dispatch_classifies_api_error_as_failure() {
        let body = r#"#!/bin/sh
printf '{"is_error": true, "api_error_status": "rate_limited", "total_cost_usd": 0.05, "result": "partial"}\n'
"#;
        let script = write_test_script("json-api-err", body);

        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp"))
            .with_claude_binary(script.clone())
            .with_default_timeout(Duration::from_secs(5));
        let step = plugin_step("plug-rate-limited", "c4-architecture", "c4-code");
        let outcome = dispatcher.dispatch(&step).await.expect("ok");

        cleanup_test_script(&script);

        assert!(!outcome.success, "is_error=true → success=false");
        assert!(outcome.output_path.is_none(), "failed run → no output_path");
        let diag = outcome.stderr.expect("must surface diagnostic");
        assert!(
            diag.contains("rate_limited"),
            "diag must mention api_error_status: {diag}"
        );
    }

    /// Non-JSON stdout (e.g. legacy binary, fixture using `/bin/echo`)
    /// surfaces a parse-failure diagnostic with raw stdout/stderr context.
    #[tokio::test]
    async fn dispatch_handles_non_json_stdout_gracefully() {
        let echo = PathBuf::from("/bin/echo");
        if !echo.is_file() {
            return;
        }
        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp"))
            .with_claude_binary(echo)
            .with_default_timeout(Duration::from_secs(5));
        let step = plugin_step("plug-nonjson", "c4-architecture", "c4-code");
        let outcome = dispatcher.dispatch(&step).await.expect("ok");
        assert!(!outcome.success);
        let diag = outcome.stderr.expect("diag");
        assert!(
            diag.contains("failed to decode"),
            "diag must surface parse failure: {diag}"
        );
    }

    // -----------------------------------------------------------------
    // Timeout handling
    // -----------------------------------------------------------------

    /// FR-9 (lifecycle): a subprocess that outlives the timeout reports
    /// `success=false` and surfaces a synthetic `timed out` diagnostic.
    #[tokio::test]
    async fn plugin_dispatcher_propagates_step_timeout_seconds() {
        let body = "#!/bin/sh\nsleep 5\n";
        let script = write_test_script("timeout", body);

        let dispatcher = PluginDispatcher::new(PathBuf::from("/tmp"))
            .with_claude_binary(script.clone())
            .with_default_timeout(Duration::from_millis(200));
        let step = plugin_step("plug-timeout", "c4-architecture", "c4-code");
        let outcome = dispatcher.dispatch(&step).await.expect("ok");

        cleanup_test_script(&script);

        assert!(
            !outcome.success,
            "subprocess that outlives timeout must report failure"
        );
        let stderr = outcome
            .stderr
            .expect("timed-out step must surface a diagnostic");
        assert!(
            stderr.contains("timed out"),
            "stderr must carry a timeout diagnostic: {stderr}"
        );
    }

    // -----------------------------------------------------------------
    // validate_agent_name unit
    // -----------------------------------------------------------------

    #[test]
    fn validate_agent_name_accepts_well_formed() {
        assert!(validate_agent_name("c4-architecture", "name").is_ok());
        assert!(validate_agent_name("c4_architecture", "name").is_ok());
        assert!(validate_agent_name("Agent", "name").is_ok());
        assert!(validate_agent_name("a", "name").is_ok());
        assert!(validate_agent_name(&"a".repeat(64), "name").is_ok());
    }

    #[test]
    fn validate_agent_name_rejects_malformed() {
        // Leading non-alpha
        assert!(validate_agent_name("1abc", "name").is_err());
        assert!(validate_agent_name("-abc", "name").is_err());
        assert!(validate_agent_name("_abc", "name").is_err());
        // Forbidden characters
        assert!(validate_agent_name("../etc/passwd", "name").is_err());
        assert!(validate_agent_name("a b", "name").is_err());
        assert!(validate_agent_name("a$b", "name").is_err());
        assert!(validate_agent_name("a\nb", "name").is_err());
        // Argv-injection vectors
        assert!(validate_agent_name("--allowedTools", "name").is_err());
        assert!(validate_agent_name("--evil", "name").is_err());
        // Boundaries
        assert!(validate_agent_name("", "name").is_err());
        assert!(validate_agent_name(&"a".repeat(65), "name").is_err());
    }
}
