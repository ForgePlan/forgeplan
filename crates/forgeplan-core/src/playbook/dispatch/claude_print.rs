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
    pub fn render_failure_context(&self) -> String {
        let mut parts = Vec::new();
        if let Some(api_err) = &self.api_error_status {
            parts.push(format!("api_error_status={api_err}"));
        }
        if self.is_error {
            parts.push("is_error=true".to_string());
        }
        if self.total_cost_usd > 0.0 {
            parts.push(format!("cost=${:.4}", self.total_cost_usd));
        }
        if let Some(result) = &self.result {
            // Truncate long bodies to keep stderr scannable.
            let preview: String = result.chars().take(500).collect();
            parts.push(format!("result_preview={preview}"));
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
/// directory (absolute) so the agent has write permission for the target
/// location. `None` if no `produces_at` is set.
pub fn add_dir_for_produces_at(step: &Step, workspace_root: &Path) -> Option<std::path::PathBuf> {
    let rel = step.produces_at.as_ref()?;
    let abs = if rel.is_absolute() {
        rel.clone()
    } else {
        workspace_root.join(rel)
    };
    abs.parent().map(|p| p.to_path_buf())
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
        let dir = add_dir_for_produces_at(&step, &ws).unwrap();
        assert_eq!(dir, std::path::PathBuf::from("/work/repo/reports"));
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
        assert!(add_dir_for_produces_at(&step, &ws).is_none());
    }
}
