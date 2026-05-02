//! Helpers for invoking `claude --print` from PluginDispatcher / AgentDispatcher
//! (ADR-011 / EVID-093). Phase B Pre-Wave 0 skeleton — Wave 1 sub-agents fill
//! in implementations.
//!
//! # Design
//!
//! `claude --print` is the headless invocation mode of the Claude Code CLI:
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

use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;

use crate::playbook::types::Step;

/// Default per-invocation budget when `Step.budget_usd` is `None`.
/// Matches ADR-011 §Decision point 2 ("default $1.00, configurable per step").
pub const DEFAULT_BUDGET_USD: f64 = 1.00;

/// Default tool allowlist for analytic agents when `Step.allowed_tools` is
/// `None`. Least-privilege: read-only filesystem + grep + glob.
/// Wave 1 may revise based on real-step requirements (e.g. add `Write` for
/// produces_at flows).
pub const DEFAULT_ALLOWED_TOOLS: &[&str] = &["Read", "Glob", "Grep"];

/// Structured envelope returned by `claude --print --output-format json`.
/// EVID-093 documented 17 fields; this struct captures the load-bearing
/// subset for dispatcher decisions. Unknown fields are ignored
/// (`#[serde(default)]` on optional ones).
///
/// Wave 1 may extend this with additional fields if dispatcher logic needs
/// them.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ClaudePrintResponse {
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
    #[serde(default)]
    pub duration_ms: u64,
    /// Session id for downstream correlation / debugging.
    #[serde(default)]
    pub session_id: Option<String>,
}

impl ClaudePrintResponse {
    /// Whether the invocation succeeded for dispatch purposes:
    /// `is_error == false` AND no `api_error_status`.
    pub fn is_success(&self) -> bool {
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
    pub fn render_failure_context(&self) -> String {
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

/// Assemble the prompt body sent on stdin. Wave 1 fills in details based on
/// `Step.input` shape and `produces_at` convention from ADR-011 §Decision
/// point 5.
///
/// Current stub returns a placeholder so type-checks pass for Wave 1
/// dispatcher rewrites. The real implementation:
/// 1. Pulls `step.input.task` (or sensible default) as the user-visible prompt body.
/// 2. If `step.produces_at` is set, appends:
///    `Write output to \`<produces_at>\` using the Write tool.`
/// 3. Returns the assembled string ready for stdin pipe.
pub fn assemble_prompt(step: &Step) -> String {
    let task = step
        .input
        .as_ref()
        .and_then(|v| v.get("task"))
        .and_then(|t| t.as_str())
        .unwrap_or("(no task provided)");

    let mut out = String::new();
    out.push_str(task);

    if let Some(path) = &step.produces_at {
        out.push_str("\n\nWrite output to `");
        out.push_str(&path.to_string_lossy());
        out.push_str("` using the Write tool.\n");
    }

    out
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
pub fn add_dir_for_produces_at(
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
pub fn effective_allowed_tools(step: &Step) -> Vec<String> {
    step.allowed_tools.clone().unwrap_or_else(|| {
        DEFAULT_ALLOWED_TOOLS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    })
}

/// Resolve the effective budget: `Step.budget_usd` if `Some`, otherwise
/// [`DEFAULT_BUDGET_USD`].
pub fn effective_budget_usd(step: &Step) -> f64 {
    step.budget_usd.unwrap_or(DEFAULT_BUDGET_USD)
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
}
