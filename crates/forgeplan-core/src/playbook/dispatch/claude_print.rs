//! Production helpers for invoking `claude --print` from
//! [`super::PluginDispatcher`] and [`super::AgentDispatcher`]
//! (ADR-011 / EVID-093). Single source of truth for the 9-step recipe:
//! argv build → env allowlist → prompt-via-stdin → spawn → timeout
//! → parse → render. PR-E (PROB-050 A-4..A-15) consolidated all
//! orchestration here; both dispatchers reduce to (a) variant unpack,
//! (b) name validation, (c) binary resolution, (d) call [`invoke`].
//!
//! # Public surface (intra-module only — `pub(super)` / `pub(crate)`)
//!
//! - [`invoke`] — full orchestration, called by both dispatchers.
//! - [`build_argv`] — argv construction with security gates
//!   ([`validate_allowed_tools`] + [`add_dir_for_produces_at`]).
//! - [`parse_envelope`] — UTF-8-trimmed JSON envelope decode.
//! - [`format_timeout_msg`] — uniform second/millisecond rendering for
//!   timeout diagnostics.
//! - [`DISPATCH_ENV_LOCK`] — `#[cfg(test)]` cross-dispatcher
//!   serialization mutex (PR-E audit HIGH-1: now consumed by
//!   `agent_dispatcher::tests`, `plugin_dispatcher::tests`,
//!   `helpers::tests`).
//!
//! Visibility tightened in PR-E A-7: `DEFAULT_BUDGET_USD`,
//! `DEFAULT_ALLOWED_TOOLS` are `pub(crate)`; `ClaudePrintResponse`,
//! `assemble_prompt`, `add_dir_for_produces_at`, `effective_*` are
//! `pub(super)`. External library consumers of the dispatch internals
//! must go through `AgentDispatcher` / `PluginDispatcher`.
//!
//! # `claude --print` argv contract
//!
//! - `--agent <name>` resolves the agent (plugin or top-level) by name
//! - `--print` disables the TUI; output goes to stdout
//! - `--output-format json` emits a structured envelope (cost, duration, errors)
//! - `--max-budget-usd <N>` caps spend per invocation
//! - `--allowedTools <T1> <T2> ...` is variadic — each tool is its own argv slot
//! - `--add-dir <path>` whitelists a write directory (used for `produces_at`)
//!
//! The prompt is supplied via stdin pipe (NOT positional argv) because the
//! variadic `--allowedTools` would otherwise consume it.
//!
//! # Invariants (ADR-011 §Invariants)
//!
//! - `claude` must be discoverable on PATH (`which claude`); otherwise
//!   surface `DispatchError::DelegateMissing` with install hint.
//! - JSON output is mandatory — exit code alone is ambiguous (budget cap and
//!   real error both produce exit 1).
//! - Budget cap is mandatory — every invocation passes `--max-budget-usd`.
//! - Stdin prompt is mandatory.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use regex::Regex;
use serde::Deserialize;

use crate::playbook::types::Step;

use super::{DispatchError, DispatchOutcome};

/// Cross-dispatcher serialization lock for tests that mutate process-global
/// env vars (PATH, FORGEPLAN_CLAUDE_BIN, FORGEPLAN_BIN).
///
/// PROB-050 A-6 closure: previously each dispatcher (`agent_dispatcher::tests`,
/// `plugin_dispatcher::tests`, `helpers::tests`) had its own ENV_GUARD or no
/// guard at all. `cargo test` runs tests on multiple threads in the SAME
/// process; without a shared lock, fake-claude scripts in concurrently-running
/// tests can see the temporarily-broken PATH set by `*_when_tool_absent`
/// cases and lose their ability to locate `/bin/sh` for shebang exec
/// (~1 in 5 runs flake). Round-5 audit Logic LOW-1 noted this race risk
/// for `helpers::tests::resolve_forgeplan_binary_respects_env_override`
/// which had no guard at all.
///
/// Uses `tokio::sync::Mutex` because every consumer holds the guard across
/// `await` points (clippy::await_holding_lock would fire on `std::sync::Mutex`
/// in `#[tokio::test]` contexts).
///
/// Test-only — gated behind `#[cfg(test)]` so it doesn't ship in release
/// binaries. Visible to sibling modules (`agent_dispatcher`, `plugin_dispatcher`,
/// `helpers`) via `pub(super)`.
#[cfg(test)]
pub(super) static DISPATCH_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Default per-invocation budget when `Step.budget_usd` is `None`.
/// Matches ADR-011 §Decision point 2 ("default $1.00, configurable per step").
///
/// PROB-050 A-7 closure: tightened from `pub` to `pub(crate)`. Empirically
/// (`rg DEFAULT_BUDGET_USD crates/`) no external library consumer reads this
/// constant; restricting to crate-internal protects the value from being
/// pinned as a stability contract by downstream crates that we'd then have
/// to honour across release boundaries. Widen back to `pub` when a
/// marketplace plugin author needs it.
pub(crate) const DEFAULT_BUDGET_USD: f64 = 1.00;

/// Default tool allowlist for analytic agents when `Step.allowed_tools` is
/// `None`. Least-privilege: read-only filesystem + grep + glob.
///
/// PROB-050 A-7 closure: tightened from `pub` to `pub(crate)` (same reason
/// as `DEFAULT_BUDGET_USD`).
pub(crate) const DEFAULT_ALLOWED_TOOLS: &[&str] = &["Read", "Glob", "Grep"];

/// Structured envelope returned by `claude --print --output-format json`.
/// EVID-093 documented 17 fields; this struct captures the load-bearing
/// subset for dispatcher decisions. Unknown fields are ignored
/// (`#[serde(default)]` on optional ones).
///
/// Wave 1 may extend this with additional fields if dispatcher logic needs
/// them.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
/// PROB-050 A-7 closure: tightened from `pub` to `pub(super)`.
/// Used only inside the dispatch module (parse_envelope returns it,
/// invoke consumes it). External library consumers would couple to
/// claude CLI's private envelope shape — high churn risk.
pub(super) struct ClaudePrintResponse {
    /// `true` iff `claude` itself reported an error condition (API error,
    /// budget exceeded mid-flight, internal failure). The exit code alone
    /// does not disambiguate.
    pub is_error: bool,
    /// API-level error status string when present (e.g. `rate_limited`).
    /// `None` for normal runs.
    #[serde(default)]
    pub api_error_status: Option<String>,
    /// Final assistant response text. May be partial if budget cap fired
    /// mid-stream.
    #[serde(default)]
    pub result: Option<String>,
    /// Total USD cost incurred. Compare against the requested
    /// `max_budget_usd` to detect budget exhaustion.
    #[serde(default)]
    pub total_cost_usd: f64,
    /// Wall-clock duration in milliseconds.
    ///
    /// PROB-050 A-7 closure: marked `#[allow(dead_code)]` because tightening
    /// the parent struct to `pub(super)` revealed there are no current
    /// readers — the field is preserved for future telemetry / log enrichment
    /// (Round 5 deferred to PROB-051 D-class). Without `#[allow]`, clippy
    /// `-D dead_code` blocks the lockdown.
    #[serde(default)]
    #[allow(dead_code)]
    pub duration_ms: u64,
    /// Session id for downstream correlation / debugging.
    /// PROB-050 A-7: see `duration_ms` rationale.
    #[serde(default)]
    #[allow(dead_code)]
    pub session_id: Option<String>,
}

impl ClaudePrintResponse {
    /// Whether the invocation succeeded for dispatch purposes:
    /// `is_error == false` AND no `api_error_status`.
    ///
    /// PR-E audit LOW-2 (architect): tightened to `pub(super)` to match
    /// the parent struct visibility — Rust min-clamps method visibility
    /// to struct visibility so this is a no-op functionally, but the
    /// explicit attribute prevents future widening of the struct from
    /// silently re-exposing the methods.
    pub(super) fn is_success(&self) -> bool {
        !self.is_error && self.api_error_status.is_none()
    }

    /// Render a human-readable error / context string suitable for
    /// `DispatchOutcome::stderr`. Includes API error tag if present, plus
    /// any partial `result` text and cost figure.
    ///
    /// R1 audit HIGH (security H-5): `result` may carry abs paths, session
    /// ids, or partial file contents the agent encountered (logs, .env
    /// surfaced in queries). Bounded at `MAX_PREVIEW_BYTES` (500 bytes,
    /// UTF-8-safe) to limit info-leak surface flowing through MCP error
    /// JSON / Claude Desktop transcripts.
    /// PR-E audit LOW-2: tightened to `pub(super)` (same rationale as
    /// `is_success`).
    pub(super) fn render_failure_context(&self) -> String {
        let mut parts = Vec::new();
        if let Some(api_err) = &self.api_error_status {
            parts.push(format!(
                "api_error_status={}",
                truncate_for_log(api_err, MAX_VALIDATOR_ECHO_BYTES)
            ));
        }
        if self.is_error {
            parts.push("is_error=true".to_string());
        }
        if self.total_cost_usd > 0.0 {
            parts.push(format!("cost=${:.4}", self.total_cost_usd));
        }
        if let Some(result) = &self.result {
            parts.push(format!(
                "result_preview={}",
                truncate_for_log(result, MAX_PREVIEW_BYTES)
            ));
        }
        parts.join(" | ")
    }
}

/// Assemble the prompt body sent on stdin (per ADR-011 §Decision point 5).
///
/// 1. Pulls `step.input.task` (or sensible default) as the user-visible prompt body.
/// 2. If `step.produces_at` is set, appends:
///    `Write output to \`<produces_at>\` using the Write tool.`
/// 3. Returns the assembled string ready for stdin pipe.
///
/// PROB-054 closure: `produces_at` is character-validated via
/// [`validate_produces_at_chars`] BEFORE splicing. If validation fails,
/// the path is omitted from the prompt body (caller's
/// [`add_dir_for_produces_at`] returns the same error и aborts the
/// dispatch via `DispatchError::Transport`, so the prompt body is never
/// actually consumed by claude).
pub(super) fn assemble_prompt(step: &Step) -> String {
    let task = step
        .input
        .as_ref()
        .and_then(|v| v.get("task"))
        .and_then(|t| t.as_str())
        .unwrap_or("(no task provided)");

    let mut out = String::new();
    out.push_str(task);

    if let Some(path) = &step.produces_at {
        // PROB-054: char-set validate BEFORE splicing to defend against
        // prompt-injection-via-filesystem (backtick closes the markdown
        // code-fence and turns the tail of the prompt into instructions
        // the agent treats as authoritative). Validation failure → omit
        // от prompt; the symmetric `add_dir_for_produces_at` returns
        // Err и the dispatcher aborts before claude runs.
        if validate_produces_at_chars(path).is_ok() {
            out.push_str("\n\nWrite output to `");
            out.push_str(&path.to_string_lossy());
            out.push_str("` using the Write tool.\n");
        }
    }

    out
}

/// PROB-054 closure — character-set validator for `Step.produces_at`.
///
/// Defends against prompt-injection-via-filesystem (CWE-94 / OWASP A03).
/// Pre-PROB-054 the path was spliced into the natural-language prompt
/// body via `to_string_lossy()` без character validation:
///
/// ```text
/// Write output to `<produces_at>` using the Write tool.
/// ```
///
/// A path containing a backtick (`reports/`backdoor`.md`) closed the
/// markdown code-fence и turned everything after into prompt content
/// the agent treated as authoritative instruction. Same class for `$`
/// (variable expansion in some agent shells), `;` (command separator
/// hint), `\n` / `\r` (line-break injection).
///
/// The conservative allowlist is `^[A-Za-z0-9._/-]+$` — alphanumeric,
/// dot, underscore, forward slash, hyphen. Symmetric with
/// `add_dir_for_produces_at` так что the prompt-body splice и the
/// `--add-dir` argv splice fail-fast on the same input.
///
/// Out of scope: `to_string_lossy()` semantics for non-UTF-8 boundaries
/// — paths containing invalid UTF-8 are rendered as `U+FFFD` replacement
/// characters which the regex rejects (defense-in-depth, not the primary
/// guarantee — Forgeplan workspaces are UTF-8 by convention).
pub(super) fn validate_produces_at_chars(path: &Path) -> Result<(), String> {
    let s = path.to_string_lossy();
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"^[A-Za-z0-9._/-]+$").expect("produces_at allowlist regex is valid")
    });
    if re.is_match(&s) {
        Ok(())
    } else {
        Err(format!(
            "produces_at contains disallowed characters; allowed set is \
             [A-Za-z0-9._/-]+, got `{}`",
            s.escape_debug()
        ))
    }
}

/// Build the `--add-dir` argument from `Step.produces_at`. Returns the parent
/// directory (workspace-relative absolute) so the agent has write permission
/// for the target location.
///
/// Returns:
/// - `Ok(None)` if no `produces_at` is set
/// - `Ok(Some(abs))` for valid workspace-relative paths
/// - `Err(reason)` for absolute paths or paths containing `..` segments
///
/// # R1 audit CRITICAL fix (security-expert)
///
/// Pre-fix: `workspace_root.join("../../etc/cron.d/file.md").parent()` returned
/// `<workspace>/../../etc/cron.d` verbatim — `..` segments were NOT collapsed
/// at construction time. This path was spliced into argv as `--add-dir`,
/// granting the agent Write permission outside the workspace once the OS
/// resolved the path. Combined with `Step.produces_at: "/etc/passwd"`
/// (absolute path bypassing the join), this was a sandbox-escape vector
/// (CWE-22 / OWASP A01).
///
/// Post-fix: absolute paths are rejected; relative paths with any
/// `Component::ParentDir` are rejected. Caller (PluginDispatcher /
/// AgentDispatcher) maps the error to `DispatchError::Transport` and refuses
/// to spawn `claude`.
pub(super) fn add_dir_for_produces_at(
    step: &Step,
    workspace_root: &Path,
) -> Result<Option<std::path::PathBuf>, String> {
    let Some(rel) = step.produces_at.as_ref() else {
        return Ok(None);
    };
    if rel.is_absolute() {
        return Err(format!(
            "produces_at must be workspace-relative, got absolute path `{}`",
            rel.display()
        ));
    }
    if rel
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(format!(
            "produces_at must not contain `..` segments (workspace-escape attempt), got `{}`",
            rel.display()
        ));
    }
    // PROB-054: same character validation that `assemble_prompt` applies
    // to the prompt body so the argv splice and the prompt body splice
    // fail-fast on the same input. Defense-in-depth — argv has its own
    // hardening (PROB-050 A-15) but the symmetric guard makes the
    // contract explicit.
    validate_produces_at_chars(rel)?;
    let abs = workspace_root.join(rel);
    Ok(abs.parent().map(|p| p.to_path_buf()))
}

/// Pattern enforcing argv-injection-safe `--allowedTools` entries.
/// Tool names follow the Claude Code convention: PascalCase identifier
/// starting with uppercase letter, alphanumeric only (no dashes / underscores
/// because those would let YAML smuggle e.g. `--debug-flag` past the regex
/// front-anchor).
fn tool_name_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^[A-Z][A-Za-z0-9]{0,31}$").expect("tool name regex literal is valid")
    })
}

/// Validate that `tool` is safe to splice into argv as a `--allowedTools`
/// variadic value (R1 audit CRITICAL — `Step.allowed_tools` was the second
/// YAML-controlled string flowing into argv unsanitised, allowing
/// flag-injection like `--debug-flag-x`).
///
/// Accepts: `Read`, `Glob`, `Grep`, `Write`, `Bash`, `Edit`, `WebFetch`,
/// `Task`, `BashOutput`, `KillShell`, `Skill`, etc. (PascalCase, alphanumeric,
/// 1..=32 chars).
///
/// Rejects: leading dash (`--debug`), shell metachars (`;`, `|`, `$`),
/// path separators (`/`), empty, length > 32.
pub(crate) fn validate_tool_name(tool: &str) -> Result<(), String> {
    if tool_name_regex().is_match(tool) {
        Ok(())
    } else {
        // R1 H-3 audit: bound the echoed input to prevent log-bloat on
        // malicious unbounded YAML. Uses shared truncate_for_log helper.
        let preview = truncate_for_log(tool, MAX_VALIDATOR_ECHO_BYTES);
        Err(format!(
            "tool name `{preview}` (len={}) rejected: must match `[A-Z][A-Za-z0-9]{{0,31}}`",
            tool.len()
        ))
    }
}

/// Validate every entry of an `effective_allowed_tools(step)` result.
/// Caller (dispatcher) wraps the first failure into `DispatchError::Transport`.
pub(crate) fn validate_allowed_tools(tools: &[String]) -> Result<(), String> {
    for t in tools {
        validate_tool_name(t)?;
    }
    Ok(())
}

/// Format `--max-budget-usd <N>` value with two-decimal precision so the
/// argv contract is identical between Plugin and Agent dispatchers.
/// R1 audit CRITICAL (rust + code-review): pre-fix Plugin used
/// `format!("{budget:.2}")` while Agent used `budget.to_string()`, producing
/// `"2.50"` vs `"2.5"` for the same value, divergent argv tests.
pub(crate) fn format_budget(budget: f64) -> String {
    format!("{budget:.2}")
}

/// Resolve the effective tool allowlist: `Step.allowed_tools` if `Some`,
/// otherwise [`DEFAULT_ALLOWED_TOOLS`]. Returns owned `String`s so the
/// caller can pass them to `Command::args` without lifetime gymnastics.
pub(super) fn effective_allowed_tools(step: &Step) -> Vec<String> {
    step.allowed_tools.clone().unwrap_or_else(|| {
        DEFAULT_ALLOWED_TOOLS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    })
}

/// Resolve the effective budget: `Step.budget_usd` if `Some`, otherwise
/// [`DEFAULT_BUDGET_USD`].
pub(super) fn effective_budget_usd(step: &Step) -> f64 {
    step.budget_usd.unwrap_or(DEFAULT_BUDGET_USD)
}

/// Parse the JSON envelope returned by `claude --print --output-format json`.
///
/// PROB-050 A-11 closure: pre-fix PluginDispatcher used
/// `serde_json::from_str(&stdout_text)` (no trim), AgentDispatcher used
/// `serde_json::from_str(stdout_str.trim())`. The trimmed variant tolerates
/// trailing newlines + BOM that real `claude --print` can emit, so it's
/// the safer pattern. Both dispatchers now consume this helper.
///
/// # Errors
///
/// Returns `serde_json::Error` when stdout is not parseable as a
/// [`ClaudePrintResponse`]. Callers wrap in dispatcher-specific
/// diagnostics (with agent name / plugin/target context).
pub(super) fn parse_envelope(stdout: &[u8]) -> Result<ClaudePrintResponse, serde_json::Error> {
    let s = String::from_utf8_lossy(stdout);
    serde_json::from_str(s.trim())
}

/// Format the dispatcher timeout-error message.
///
/// PROB-050 A-11 closure: pre-fix PluginDispatcher used `.as_secs()` →
/// `"plugin `foo/bar` timed out after 300s"`. AgentDispatcher used `{:?}`
/// (Debug repr) → `"agent `foo` timed out after 300s"` for whole seconds
/// but `"300.500s"` for fractional — leaks Duration's internal layout into
/// user-visible diagnostics. Single source of truth, both dispatchers
/// consume this helper.
///
/// **Sub-second handling (PR-E Round 6 audit HIGH fix)**: pure
/// `.as_secs()` truncates `200ms → "0s"`, which confuses operators
/// chasing a tight-loop timeout. We render `Ns` for whole-second + and
/// `Nms` for sub-second; both branches preserve byte-stable output for
/// the common `Step.timeout_seconds = u32 ≥ 1` case (production path).
///
/// `label` is the dispatcher-specific prefix (e.g. `agent \`foo\``,
/// `plugin \`foo/bar\``). Helper returns the full sentence.
pub(super) fn format_timeout_msg(label: &str, duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs == 0 {
        format!("{label} timed out after {}ms", duration.as_millis())
    } else {
        format!("{label} timed out after {secs}s")
    }
}

/// Build the full argv for `claude --print` invocation, with all security
/// validation gates applied.
///
/// PROB-050 A-15 closure: extracted from AgentDispatcher::dispatch and
/// PluginDispatcher::dispatch (identical 11-step recipes per ADR-011
/// §Decision). Single source of truth — argv-shape tests live alongside
/// this function in `claude_print::tests` (no fake-binary indirection).
///
/// # Argv shape (per ADR-011 §Decision)
///
/// `[--print, --agent, <slug>, --output-format, json, --max-budget-usd,
///   <budget>, [--add-dir, <path>], [--allowedTools, <T1>, <T2>, ...]]`
///
/// `--allowedTools` MUST be last because it's variadic — any flag after it
/// would be consumed as a tool name.
///
/// # Security gates (run BEFORE any argv push)
///
/// - [`validate_allowed_tools`] — flag-injection guard (R1 audit C-2).
/// - [`add_dir_for_produces_at`] — path-traversal guard (R1 audit C-1).
///
/// Both produce `Err(String)` with a human-readable reason; callers wrap
/// in `DispatchError::Transport` with the step id for context.
///
/// # Errors
///
/// Returns `Err(reason: String)` when:
/// - any `Step.allowed_tools` entry fails `validate_tool_name`;
/// - `Step.produces_at` has a path-traversal segment (`..` or absolute).
///
/// # Example
///
/// ```ignore
/// let argv = build_argv("my-agent", &step, &workspace_root)?;
/// assert_eq!(argv[0], "--print");
/// assert_eq!(argv[1], "--agent");
/// assert_eq!(argv[2], "my-agent");
/// ```
pub(super) fn build_argv(
    slug: &str,
    step: &Step,
    workspace_root: &Path,
) -> Result<Vec<String>, String> {
    let budget = effective_budget_usd(step);
    let tools = effective_allowed_tools(step);

    // R1 audit CRITICAL — security-expert C-2: validate every
    // allowed_tools entry BEFORE argv construction.
    validate_allowed_tools(&tools)?;

    // R1 audit CRITICAL — security-expert C-1: canonicalise produces_at,
    // reject `..` and absolute paths to prevent workspace escape via
    // `--add-dir`.
    let add_dir = add_dir_for_produces_at(step, workspace_root)?;

    let mut args: Vec<String> = Vec::with_capacity(11 + tools.len());
    args.push("--print".to_string());
    args.push("--agent".to_string());
    args.push(slug.to_string());
    args.push("--output-format".to_string());
    args.push("json".to_string());
    args.push("--max-budget-usd".to_string());
    // Shared format_budget for argv-shape parity between Plugin/Agent
    // (pre-fix Plugin emitted "1.00", Agent emitted "1" for default).
    args.push(format_budget(budget));
    if let Some(dir) = &add_dir {
        args.push("--add-dir".to_string());
        args.push(dir.to_string_lossy().into_owned());
    }
    // `--allowedTools` is variadic and MUST be last — any later flag
    // would be consumed as a tool name.
    if !tools.is_empty() {
        args.push("--allowedTools".to_string());
        for tool in &tools {
            args.push(tool.clone());
        }
    }
    Ok(args)
}

/// Full `claude --print` invocation orchestration: argv build + env
/// allowlist + prompt-via-stdin + spawn + timeout + parse + render.
///
/// PROB-050 A-4 closure: extracted from AgentDispatcher::dispatch and
/// PluginDispatcher::dispatch (identical 9-step recipe per ADR-011
/// §Decision). Both dispatchers reduce to (a) variant unpack,
/// (b) compute slug + label, (c) resolve binary, (d) call invoke.
///
/// # Arguments
///
/// - `label` — dispatcher-prefix string for diagnostics
///   (`"agent \`foo\`"` or `"plugin \`foo/bar\`"`)
/// - `slug` — value for `--agent <slug>` argv slot
/// - `step` — playbook step (provides budget, allowed_tools, produces_at, …)
/// - `workspace_root` — cwd for the subprocess (relative `produces_at`
///   paths land here) and root for path-traversal checks
/// - `binary` — pre-resolved path to the `claude` executable
/// - `timeout` — pre-resolved per-invocation timeout (already considers
///   `Step.timeout_seconds` override vs dispatcher default)
///
/// # Errors
///
/// Returns `DispatchError::Transport` when:
/// - `build_argv` rejects the step (flag-injection or path-traversal)
/// - `run_subprocess` cannot spawn the binary (Transport-level)
///
/// Returns `Ok(DispatchOutcome { success: false, ... })` when:
/// - subprocess timed out
/// - `claude --print` returned a non-success JSON envelope (api error,
///   budget cap, partial result)
/// - stdout is not parseable as a [`ClaudePrintResponse`]
///
/// Returns `Ok(DispatchOutcome { success: true, ... })` when claude's
/// JSON envelope decoded with `is_error: false`.
pub(super) async fn invoke(
    label: &str,
    slug: &str,
    step: &Step,
    workspace_root: &Path,
    binary: &Path,
    timeout: Duration,
) -> Result<DispatchOutcome, DispatchError> {
    // 1. Build argv (with security gates: validate_allowed_tools +
    //    add_dir_for_produces_at).
    let args = build_argv(slug, step, workspace_root)
        .map_err(|reason| DispatchError::Transport(format!("{label}: {reason}")))?;

    // 2. Compose env allow-list — base PATH/HOME/USER only. We
    //    deliberately do NOT forward `ANTHROPIC_API_KEY` etc. — `claude`
    //    relies on its existing keychain session.
    let base_env: HashMap<String, String> = std::env::vars().collect();
    let env = super::helpers::build_env_allowlist(&[], &base_env);

    // 3. Assemble prompt for stdin. produces_at hint is appended by
    //    `assemble_prompt` itself.
    let prompt = assemble_prompt(step);
    let stdin_bytes = prompt.into_bytes();

    // 4. Build subprocess spec.
    let program_str = binary.to_string_lossy().into_owned();
    let spec = super::helpers::SubprocessSpec {
        program: &program_str,
        args: &args,
        env: &env,
        cwd: Some(workspace_root),
        timeout,
        stdin_data: Some(&stdin_bytes),
    };

    // 5. Execute. Helper translates lifecycle into outcome / Transport.
    let outcome = super::helpers::run_subprocess(spec).await?;

    // 6. Map subprocess outcome → DispatchOutcome.
    if outcome.timed_out {
        return Ok(DispatchOutcome {
            success: false,
            output_path: None,
            stderr: Some(format_timeout_msg(label, outcome.duration)),
        });
    }

    let stderr_str = if outcome.stderr.is_empty() {
        String::new()
    } else {
        String::from_utf8_lossy(&outcome.stderr).into_owned()
    };

    match parse_envelope(&outcome.stdout) {
        Ok(response) => {
            let success = response.is_success();
            let stderr = if success {
                if stderr_str.is_empty() {
                    None
                } else {
                    Some(stderr_str)
                }
            } else {
                let mut combined = response.render_failure_context();
                if !stderr_str.is_empty() {
                    combined.push_str(" | stderr=");
                    combined.push_str(&stderr_str);
                }
                Some(combined)
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
        Err(parse_err) => {
            // Unparseable stdout: JSON envelope is mandatory per ADR-011.
            // Diagnostic combines exit_code (agent's pattern) + stdout
            // preview (plugin's pattern) — union of pre-A-4 dispatcher
            // diagnostics. Prefix uses "failed to decode" wording so the
            // pre-A-4 plugin test assertion `diag.contains("failed to
            // decode")` still passes; agent test wasn't string-asserting
            // this path.
            let mut diag =
                format!("{label} failed to decode claude --print JSON envelope: {parse_err}");
            if let Some(code) = outcome.exit_code {
                diag.push_str(&format!(" | exit_code={code}"));
            }
            if !stderr_str.is_empty() {
                diag.push_str(" | stderr=");
                diag.push_str(stderr_str.trim_end());
            }
            if !outcome.stdout.is_empty() {
                // PR-E audit MED-2 (security + logic): pre-fix used
                // `chars().take(MAX_PREVIEW_BYTES)` which silently became a
                // CHAR count, not bytes — multi-byte UTF-8 (CJK/emoji) could
                // inflate preview to ~2KB despite the 500-name. Use the
                // shared byte-bounded UTF-8-safe helper that
                // `render_failure_context` already uses for identical
                // info-disclosure surface across both rendering paths.
                let stdout_str = String::from_utf8_lossy(&outcome.stdout);
                let preview = truncate_for_log(&stdout_str, MAX_PREVIEW_BYTES);
                diag.push_str(" | stdout_preview=");
                diag.push_str(preview.trim_end());
            }
            Ok(DispatchOutcome {
                success: false,
                output_path: None,
                stderr: Some(diag),
            })
        }
    }
}

/// Pattern enforcing argv-injection-safe agent / plugin / target names:
/// must start with a letter, then up to 63 additional `[A-Za-z0-9_-]` chars
/// (total length 1..=64). Compiled lazily and cached.
fn agent_name_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^[A-Za-z][A-Za-z0-9_-]{0,63}$").expect("agent name regex literal is valid")
    })
}

/// Validate that `name` is safe to splice into argv as an agent / plugin /
/// target identifier (ADR-011 §Security). Both `AgentDispatcher` and
/// `PluginDispatcher` MUST call this before spawning `claude --print`.
///
/// Reject reasons (non-exhaustive):
/// - empty
/// - leading dash (`--allowedTools` would be parsed as a flag by claude)
/// - shell metacharacters (`;`, `|`, `&`, `$`, backtick, spaces, etc.)
/// - exceeds 64 chars
///
/// Maximum byte size for `result_preview` and stderr fragments embedded in
/// `DispatchOutcome.stderr` / error messages. Bounds info-leak surface
/// (R1 audit HIGH — security H-5: claude session output may carry abs paths
/// / session ids / file contents through into MCP error JSON).
pub(crate) const MAX_PREVIEW_BYTES: usize = 500;

/// Maximum byte size for the validator's echo of the rejected user input
/// (R1 audit HIGH — security H-3: a multi-MB malicious YAML name would
/// produce a multi-MB error string flowing into logs / MCP envelope).
pub(crate) const MAX_VALIDATOR_ECHO_BYTES: usize = 80;

/// Truncate `s` at `max_bytes` boundary, respecting UTF-8 char boundaries
/// so the truncated string remains valid. Appends "…" suffix when truncated.
pub(crate) fn truncate_for_log(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

/// Caller wraps the returned `String` in its own `DispatchError::Transport`
/// (or analogous) so the surrounding context (agent vs plugin) is preserved.
pub(crate) fn validate_agent_name(name: &str) -> Result<(), String> {
    if agent_name_regex().is_match(name) {
        Ok(())
    } else {
        // R1 audit HIGH (security H-3 + code-review H-4): bound the echoed
        // input. A 4MB malicious YAML name would otherwise produce a 4MB
        // error string flowing into MCP error JSON / structured logs.
        let preview = truncate_for_log(name, MAX_VALIDATOR_ECHO_BYTES);
        Err(format!(
            "agent name `{preview}` (len={}) rejected: must match \
             ^[A-Za-z][A-Za-z0-9_-]{{0,63}}$ (argv-injection guard, ADR-011 §Security)",
            name.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json_response(is_error: bool, api_err: Option<&str>, cost: f64) -> String {
        let api = api_err
            .map(|e| format!("\"{e}\""))
            .unwrap_or_else(|| "null".to_string());
        format!(
            r#"{{"is_error": {is_error}, "api_error_status": {api}, "result": "ok", "total_cost_usd": {cost}, "duration_ms": 1234, "session_id": "abc-123"}}"#
        )
    }

    #[test]
    fn parses_successful_response() {
        let json = json_response(false, None, 0.42);
        let resp: ClaudePrintResponse = serde_json::from_str(&json).unwrap();
        assert!(resp.is_success());
        assert_eq!(resp.total_cost_usd, 0.42);
        assert_eq!(resp.session_id.as_deref(), Some("abc-123"));
    }

    #[test]
    fn parses_api_error_response() {
        let json = json_response(true, Some("rate_limited"), 0.05);
        let resp: ClaudePrintResponse = serde_json::from_str(&json).unwrap();
        assert!(!resp.is_success());
        let ctx = resp.render_failure_context();
        assert!(ctx.contains("rate_limited"));
        assert!(ctx.contains("is_error=true"));
        assert!(ctx.contains("cost=$0.0500"));
    }

    #[test]
    fn parses_minimal_response_with_defaults() {
        // Unknown fields ignored, optional fields default.
        let json = r#"{"is_error": false}"#;
        let resp: ClaudePrintResponse = serde_json::from_str(json).unwrap();
        assert!(resp.is_success());
        assert_eq!(resp.total_cost_usd, 0.0);
        assert_eq!(resp.duration_ms, 0);
    }

    #[test]
    fn assemble_prompt_uses_input_task() {
        let yaml = serde_yaml::from_str("task: \"Analyze the auth module\"").unwrap();
        let step = Step {
            id: "s1".to_string(),
            delegate_to: crate::playbook::types::Delegation::Plugin {
                name: "p".to_string(),
                target: "t".to_string(),
            },
            input: Some(yaml),
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: Default::default(),
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        };
        let prompt = assemble_prompt(&step);
        assert!(prompt.contains("Analyze the auth module"));
        assert!(!prompt.contains("Write output"));
    }

    #[test]
    fn assemble_prompt_appends_write_tool_hint_for_produces_at() {
        let yaml = serde_yaml::from_str("task: \"Make report\"").unwrap();
        let step = Step {
            id: "s1".to_string(),
            delegate_to: crate::playbook::types::Delegation::Agent {
                name: "n".to_string(),
            },
            input: Some(yaml),
            produces_at: Some(std::path::PathBuf::from("reports/r.md")),
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: Default::default(),
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        };
        let prompt = assemble_prompt(&step);
        assert!(prompt.contains("Make report"));
        assert!(prompt.contains("Write output to `reports/r.md`"));
        assert!(prompt.contains("Write tool"));
    }

    #[test]
    fn format_timeout_msg_renders_seconds_for_whole_durations() {
        // PR-E Round 6 audit HIGH fix: production path is u32 seconds,
        // must keep "Ns" rendering byte-stable.
        assert_eq!(
            format_timeout_msg("agent `foo`", std::time::Duration::from_secs(300)),
            "agent `foo` timed out after 300s"
        );
        assert_eq!(
            format_timeout_msg("plugin `a/b`", std::time::Duration::from_secs(1)),
            "plugin `a/b` timed out after 1s"
        );
    }

    #[test]
    fn format_timeout_msg_renders_milliseconds_for_sub_second() {
        // PR-E Round 6 audit HIGH fix: pre-fix as_secs() truncated 200ms
        // → "0s", confusing operators chasing tight-loop timeouts.
        assert_eq!(
            format_timeout_msg("agent `foo`", std::time::Duration::from_millis(200)),
            "agent `foo` timed out after 200ms"
        );
        assert_eq!(
            format_timeout_msg("plugin `a/b`", std::time::Duration::from_millis(750)),
            "plugin `a/b` timed out after 750ms"
        );
        // Edge: exactly 0ms — unreachable in production (clamp at u32),
        // but the helper must not panic.
        assert_eq!(
            format_timeout_msg("x", std::time::Duration::from_millis(0)),
            "x timed out after 0ms"
        );
    }

    #[test]
    fn effective_budget_uses_step_override() {
        let step = Step {
            id: "x".to_string(),
            delegate_to: crate::playbook::types::Delegation::Agent {
                name: "n".to_string(),
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: Default::default(),
            timeout_seconds: None,
            budget_usd: Some(2.50),
            allowed_tools: None,
        };
        assert_eq!(effective_budget_usd(&step), 2.50);
    }

    #[test]
    fn effective_budget_falls_back_to_default() {
        let step = Step {
            id: "x".to_string(),
            delegate_to: crate::playbook::types::Delegation::Agent {
                name: "n".to_string(),
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: Default::default(),
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        };
        assert_eq!(effective_budget_usd(&step), DEFAULT_BUDGET_USD);
    }

    #[test]
    fn effective_allowed_tools_uses_step_override() {
        let step = Step {
            id: "x".to_string(),
            delegate_to: crate::playbook::types::Delegation::Agent {
                name: "n".to_string(),
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: Default::default(),
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: Some(vec!["Bash".to_string(), "Write".to_string()]),
        };
        let tools = effective_allowed_tools(&step);
        assert_eq!(tools, vec!["Bash".to_string(), "Write".to_string()]);
    }

    #[test]
    fn effective_allowed_tools_falls_back_to_default() {
        let step = Step {
            id: "x".to_string(),
            delegate_to: crate::playbook::types::Delegation::Agent {
                name: "n".to_string(),
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: Default::default(),
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        };
        let tools = effective_allowed_tools(&step);
        assert_eq!(tools.len(), 3);
        assert!(tools.contains(&"Read".to_string()));
        assert!(tools.contains(&"Glob".to_string()));
        assert!(tools.contains(&"Grep".to_string()));
    }

    #[test]
    fn add_dir_resolves_relative_produces_at() {
        let step = Step {
            id: "x".to_string(),
            delegate_to: crate::playbook::types::Delegation::Agent {
                name: "n".to_string(),
            },
            input: None,
            produces_at: Some(std::path::PathBuf::from("reports/foo.md")),
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: Default::default(),
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        };
        let ws = std::path::PathBuf::from("/work/repo");
        let dir = add_dir_for_produces_at(&step, &ws)
            .expect("relative produces_at must be accepted")
            .expect("reports/ has a parent");
        assert_eq!(dir, std::path::PathBuf::from("/work/repo/reports"));
    }

    #[test]
    fn validate_agent_name_accepts_typical_identifiers() {
        for ok in [
            "auditor",
            "code-reviewer",
            "rust_expert",
            "Agent1",
            "a",
            "A1_2-3",
        ] {
            validate_agent_name(ok).unwrap_or_else(|e| panic!("must accept `{ok}`: {e}"));
        }
    }

    #[test]
    fn validate_agent_name_rejects_argv_injection_forms() {
        for bad in [
            "",
            "--allowedTools",
            "-x",
            "a b",
            "a;b",
            "a|b",
            "a$b",
            "a`b",
            "a&b",
            "a/b",
            "../etc",
            "1leading-digit",
        ] {
            assert!(
                validate_agent_name(bad).is_err(),
                "must reject `{bad}` as argv-injection unsafe"
            );
        }
    }

    #[test]
    fn validate_agent_name_enforces_length_cap() {
        let ok = "a".to_string() + &"b".repeat(63); // 64 chars total
        assert!(validate_agent_name(&ok).is_ok());
        let too_long = "a".to_string() + &"b".repeat(64); // 65 chars
        assert!(validate_agent_name(&too_long).is_err());
    }

    #[test]
    fn add_dir_returns_none_for_no_produces_at() {
        let step = Step {
            id: "x".to_string(),
            delegate_to: crate::playbook::types::Delegation::Agent {
                name: "n".to_string(),
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: Default::default(),
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        };
        let ws = std::path::PathBuf::from("/work/repo");
        assert!(
            add_dir_for_produces_at(&step, &ws)
                .expect("None produces_at is accepted")
                .is_none()
        );
    }

    /// R1 audit CRITICAL (security-expert C-1): path-traversal via `..`
    /// rejected before argv construction.
    #[test]
    fn add_dir_rejects_parent_dir_segments() {
        let yaml = serde_yaml::from_str("task: \"x\"").unwrap();
        let step = Step {
            id: "x".to_string(),
            delegate_to: crate::playbook::types::Delegation::Plugin {
                name: "p".into(),
                target: "t".into(),
            },
            input: Some(yaml),
            produces_at: Some(std::path::PathBuf::from("../../etc/passwd_replacement.md")),
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: Default::default(),
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        };
        let ws = std::path::PathBuf::from("/work/repo");
        let err = add_dir_for_produces_at(&step, &ws).expect_err("must reject `..` in produces_at");
        assert!(err.contains("workspace-escape"), "reason: {err}");
    }

    /// R1 audit CRITICAL (security-expert C-1): absolute paths rejected
    /// even when no `..` (would otherwise grant `--add-dir /etc`).
    #[test]
    fn add_dir_rejects_absolute_paths() {
        let yaml = serde_yaml::from_str("task: \"x\"").unwrap();
        let step = Step {
            id: "x".to_string(),
            delegate_to: crate::playbook::types::Delegation::Plugin {
                name: "p".into(),
                target: "t".into(),
            },
            input: Some(yaml),
            produces_at: Some(std::path::PathBuf::from("/etc/passwd")),
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: Default::default(),
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        };
        let ws = std::path::PathBuf::from("/work/repo");
        let err = add_dir_for_produces_at(&step, &ws).expect_err("must reject absolute path");
        assert!(err.contains("absolute path"), "reason: {err}");
    }

    /// R1 audit CRITICAL (security-expert C-2): tool-name validator
    /// rejects flag-injection forms.
    #[test]
    fn validate_tool_name_rejects_argv_injection() {
        for bad in [
            "--debug",
            "--allowedTools",
            "Bash;rm -rf /",
            "Read|cat",
            "Read$EVIL",
            "/Read",
            "../Read",
            "",
            "lowercase",
            "1Read",
            "_Read",
        ] {
            assert!(
                validate_tool_name(bad).is_err(),
                "must reject `{bad}` as tool name",
            );
        }
    }

    /// R1 audit CRITICAL (security-expert C-2): tool-name validator
    /// accepts known-good Claude Code tools.
    #[test]
    fn validate_tool_name_accepts_known_tools() {
        for ok in [
            "Read",
            "Glob",
            "Grep",
            "Write",
            "Edit",
            "Bash",
            "WebFetch",
            "Task",
            "BashOutput",
            "KillShell",
        ] {
            validate_tool_name(ok)
                .unwrap_or_else(|e| panic!("must accept `{ok}` as tool name: {e}"));
        }
    }

    /// R1 audit HIGH (security H-3): validator error message bounded so a
    /// multi-MB malicious tool name doesn't bloat logs.
    #[test]
    fn validate_tool_name_truncates_long_input_in_error() {
        let huge = "A".repeat(10_000);
        let err = validate_tool_name(&huge).expect_err("32-char limit must reject 10KB");
        // Error message itself is bounded: preview cap + ellipsis.
        assert!(
            err.len() < 500,
            "error msg must be bounded, got len={}",
            err.len()
        );
        assert!(err.contains("len=10000"));
    }

    /// R1 audit CRITICAL (rust+code-review C-1/C-2): shared format_budget
    /// gives stable two-decimal argv formatting across Plugin and Agent.
    #[test]
    fn format_budget_pads_to_two_decimals() {
        assert_eq!(format_budget(1.0), "1.00");
        assert_eq!(format_budget(0.5), "0.50");
        assert_eq!(format_budget(2.345), "2.35"); // 4 -> 5 round half-up via std
        assert_eq!(format_budget(10.999), "11.00");
    }

    // ─────────────────────────────────────────────────────────────────────
    // PROB-054 produces_at character-set validator tests
    // ─────────────────────────────────────────────────────────────────────

    /// PROB-054 happy path — typical workspace-relative path with slashes,
    /// dots, hyphens, underscores must validate cleanly.
    #[test]
    fn validate_produces_at_chars_accepts_typical_path() {
        let p = std::path::Path::new("reports/audit-2026-05-06.md");
        assert!(validate_produces_at_chars(p).is_ok());
        let p = std::path::Path::new("docs/operations/QUALITY-GATES.ru.md");
        assert!(validate_produces_at_chars(p).is_ok());
        let p = std::path::Path::new("a_b_c.txt");
        assert!(validate_produces_at_chars(p).is_ok());
    }

    /// PROB-054 main attack — backtick closes the markdown code-fence
    /// in the prompt body and turns the tail into agent instructions.
    /// MUST be rejected.
    #[test]
    fn validate_produces_at_chars_rejects_backtick() {
        let p = std::path::Path::new("reports/`backdoor`.md");
        let err = validate_produces_at_chars(p).expect_err("backtick must reject");
        assert!(
            err.contains("disallowed characters"),
            "error must explain disallowed chars: {err}"
        );
        // escape_debug renders backtick as-is (it's printable ASCII), but
        // the assert verifies the offending input is reflected for ops debug
        assert!(err.contains("backdoor"), "error must reflect input: {err}");
    }

    /// PROB-054 — dollar sign is reserved in some agent shells / template
    /// renderers (`$VAR`, `$(cmd)` substitution); reject for symmetry
    /// with the threat model described in the PROB body.
    #[test]
    fn validate_produces_at_chars_rejects_dollar_sign() {
        let p = std::path::Path::new("reports/$(whoami).md");
        let err = validate_produces_at_chars(p).expect_err("dollar must reject");
        assert!(err.contains("disallowed characters"));
    }

    /// PROB-054 — semicolon ranges from "command separator hint" to
    /// nothing depending on agent rendering; conservative reject.
    #[test]
    fn validate_produces_at_chars_rejects_semicolon() {
        let p = std::path::Path::new("reports/file;rm -rf.md");
        let err = validate_produces_at_chars(p).expect_err("semicolon must reject");
        assert!(err.contains("disallowed characters"));
    }

    /// PROB-054 — newline / carriage-return inject literal line breaks
    /// into the prompt body; the agent reads multiple "instructions"
    /// where one was intended. Reject.
    #[test]
    fn validate_produces_at_chars_rejects_newline() {
        let p = std::path::Path::new("reports/inject\nLine.md");
        let err = validate_produces_at_chars(p).expect_err("newline must reject");
        assert!(err.contains("disallowed characters"));
    }

    /// PROB-054 — `add_dir_for_produces_at` MUST surface the same
    /// validator failure path so the dispatcher aborts before claude
    /// runs (symmetric guard with assemble_prompt).
    #[test]
    fn add_dir_for_produces_at_rejects_disallowed_chars() {
        let mut step = make_step_for_test();
        step.produces_at = Some(std::path::PathBuf::from("reports/`evil`.md"));
        let err = add_dir_for_produces_at(&step, std::path::Path::new("/tmp/ws"))
            .expect_err("disallowed chars must reject");
        assert!(
            err.contains("disallowed characters"),
            "error must come from the validator: {err}"
        );
    }

    /// Helper: build a minimal Step fixture for produces_at tests.
    fn make_step_for_test() -> super::super::super::types::Step {
        use super::super::super::types::{Delegation, Step};
        Step {
            id: "test-step".to_string(),
            delegate_to: Delegation::Agent {
                name: "test-agent".to_string(),
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: Default::default(),
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        }
    }
}
