//! Shared hints infrastructure — actionable suggestions across all commands.
//!
//! Hints are warnings, info messages, and suggestions that help users
//! understand what's missing, what to do next, and how to improve quality.
//! They appear in CLI output and are included in JSON for AI agents.

use serde::Serialize;

/// A hint attached to any command output.
#[derive(Debug, Clone, Serialize)]
pub struct Hint {
    pub level: HintLevel,
    pub message: String,
    /// Suggested next action (forgeplan command or description)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HintLevel {
    Warning,
    Info,
    Suggestion,
}

impl Hint {
    pub fn warning(message: impl Into<String>) -> Self {
        Self { level: HintLevel::Warning, message: message.into(), action: None }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self { level: HintLevel::Info, message: message.into(), action: None }
    }

    pub fn suggestion(message: impl Into<String>) -> Self {
        Self { level: HintLevel::Suggestion, message: message.into(), action: None }
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
}

/// Format hints for terminal output.
pub fn format_hints(hints: &[Hint]) -> String {
    if hints.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n");
    for hint in hints {
        let prefix = match hint.level {
            HintLevel::Warning => "!",
            HintLevel::Info => "i",
            HintLevel::Suggestion => "*",
        };
        out.push_str(&format!("  {} {}\n", prefix, hint.message));
        if let Some(ref action) = hint.action {
            out.push_str(&format!("    -> {}\n", action));
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Domain-specific hint collectors
// ─────────────────────────────────────────────────────────────────────────────

/// Hints for `forgeplan score` output.
pub fn score_hints(
    r_eff: f64,
    has_evidence: bool,
    evidence_cl0_count: usize,
) -> Vec<Hint> {
    let mut hints = Vec::new();

    if !has_evidence {
        hints.push(
            Hint::warning("No evidence linked — R_eff will be 0.0")
                .with_action("forgeplan new evidence \"<what you verified>\" && forgeplan link EVID-XXX <artifact> --relation informs")
        );
    }

    if evidence_cl0_count > 0 {
        hints.push(
            Hint::warning(format!(
                "{} evidence(s) have CL0 — check that body contains 'congruence_level: 3' (not template placeholder)",
                evidence_cl0_count
            ))
            .with_action("Edit evidence body: add '## Structured Fields' with verdict/congruence_level/evidence_type")
        );
    }

    if r_eff > 0.0 && r_eff < 0.5 {
        hints.push(
            Hint::info("R_eff below 0.5 — decision is weakly supported")
                .with_action("Add more evidence or improve congruence level (CL3 = same context)")
        );
    }

    hints
}

/// Hints for `forgeplan get` output.
pub fn get_hints(
    status: &str,
    kind: &str,
    has_links: bool,
    depth: &str,
) -> Vec<Hint> {
    let mut hints = Vec::new();

    if status == "draft" {
        let next = match kind {
            "prd" => "Fill MUST sections (Problem, Goals, FR, Target Users), then: forgeplan validate",
            "rfc" => "Fill Summary, Motivation, Options, Implementation Phases, then: forgeplan validate",
            "adr" => "Fill Context, Decision, Consequences, then: forgeplan validate",
            "evidence" => "Fill Structured Fields (verdict, congruence_level, evidence_type), then: forgeplan activate",
            _ => "Fill required sections, then: forgeplan validate",
        };
        hints.push(Hint::suggestion(format!("Status is draft — next: {}", next)));
    }

    if !has_links && status != "deprecated" && kind != "memory" {
        hints.push(
            Hint::info("No links — artifact is an orphan")
                .with_action(format!("forgeplan link <this-id> <parent-id> --relation refines"))
        );
    }

    if depth == "standard" && (kind == "prd") && !has_links {
        hints.push(
            Hint::suggestion("Standard PRD should have linked RFC")
                .with_action("forgeplan new rfc \"<title>\" && forgeplan link RFC-XXX <this-id> --relation based_on")
        );
    }

    hints
}

/// Hints for `forgeplan review` output.
pub fn review_hints(
    has_evidence: bool,
    is_stub: bool,
    has_must_errors: bool,
    kind: &str,
) -> Vec<Hint> {
    let mut hints = Vec::new();

    if is_stub {
        hints.push(
            Hint::warning("Artifact is a stub — MUST sections not filled")
                .with_action(match kind {
                    "prd" => "Fill: Problem, Goals, Non-Goals, Target Users, FR",
                    "rfc" => "Fill: Summary, Motivation, Goals, Options, Implementation Phases",
                    _ => "Fill all required sections",
                })
        );
    }

    if has_must_errors {
        hints.push(
            Hint::warning("Validation has MUST errors — fix before activating")
                .with_action("forgeplan validate <id> to see specific errors")
        );
    }

    if !has_evidence {
        hints.push(
            Hint::info("No evidence linked — consider adding evidence before activation")
                .with_action("forgeplan new evidence \"<verification>\" && forgeplan link EVID-XXX <id> --relation informs")
        );
    }

    hints
}

/// Hints for `forgeplan search` when no results found.
pub fn search_hints(query: &str, result_count: usize) -> Vec<Hint> {
    let mut hints = Vec::new();

    if result_count == 0 {
        hints.push(Hint::info(format!("No results for \"{}\"", query)));
        hints.push(
            Hint::suggestion("Try broader keywords or check spelling")
                .with_action("forgeplan search \"<shorter query>\"")
        );
        hints.push(
            Hint::suggestion("Search by kind: forgeplan list --type prd")
        );
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_hints_no_evidence() {
        let hints = score_hints(0.0, false, 0);
        assert_eq!(hints.len(), 1);
        assert!(hints[0].message.contains("No evidence"));
        assert!(hints[0].action.is_some());
    }

    #[test]
    fn score_hints_cl0() {
        let hints = score_hints(0.7, true, 2);
        assert_eq!(hints.len(), 1);
        assert!(hints[0].message.contains("CL0"));
    }

    #[test]
    fn score_hints_low_reff() {
        let hints = score_hints(0.3, true, 0);
        assert_eq!(hints.len(), 1);
        assert!(hints[0].message.contains("below 0.5"));
    }

    #[test]
    fn get_hints_draft_prd() {
        let hints = get_hints("draft", "prd", false, "standard");
        assert!(hints.len() >= 2); // draft + orphan + maybe RFC suggestion
        assert!(hints[0].message.contains("draft"));
    }

    #[test]
    fn get_hints_active_with_links() {
        let hints = get_hints("active", "prd", true, "standard");
        assert!(hints.is_empty());
    }

    #[test]
    fn review_hints_stub() {
        let hints = review_hints(false, true, false, "prd");
        assert!(hints.len() >= 2); // stub + no evidence
        assert!(hints[0].message.contains("stub"));
    }

    #[test]
    fn search_hints_empty() {
        let hints = search_hints("nonexistent", 0);
        assert_eq!(hints.len(), 3);
        assert!(hints[0].message.contains("No results"));
    }

    #[test]
    fn format_hints_output() {
        let hints = vec![
            Hint::warning("Problem").with_action("Fix it"),
            Hint::suggestion("Try this"),
        ];
        let output = format_hints(&hints);
        assert!(output.contains("! Problem"));
        assert!(output.contains("-> Fix it"));
        assert!(output.contains("* Try this"));
    }
}
