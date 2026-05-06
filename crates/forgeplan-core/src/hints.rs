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
        Self {
            level: HintLevel::Warning,
            message: message.into(),
            action: None,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            level: HintLevel::Info,
            message: message.into(),
            action: None,
        }
    }

    pub fn suggestion(message: impl Into<String>) -> Self {
        Self {
            level: HintLevel::Suggestion,
            message: message.into(),
            action: None,
        }
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
}

/// Extract the primary next-action from a list of hints — used for the `_next_action`
/// field in JSON output and the `Next:` line in CLI text output.
///
/// **PRD-071 contract**: every command's response should expose exactly ONE primary
/// next-action (or `None` for terminal states). This is a single deterministic command
/// the agent should run next, distinct from the multi-hint advisory list.
///
/// Selection rule: first hint with a non-empty `action` (Warning before Info before
/// Suggestion ordering preserved by the caller). Returns `None` if no hint has an action.
pub fn primary_action(hints: &[Hint]) -> Option<String> {
    hints
        .iter()
        .find_map(|h| h.action.as_ref().map(|a| a.to_string()))
}

/// Render the primary next-action as a `Next: <command>` line for CLI text output.
///
/// Returns empty string if no actionable hint present (terminal state). This guarantees
/// every CLI command's text output ends with either a `Next:` line or an explicit
/// terminal status — no silent gaps.
pub fn render_next_action_line(hints: &[Hint]) -> String {
    match primary_action(hints) {
        Some(cmd) => format!("\nNext: {}\n", cmd),
        None => String::new(),
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
// PRD-075 FR-009 — single canonical "reconcile parents" hint used after every
// mutator that auto-recomputes the local target. Centralizing prevents the
// hint string from drifting between mutator call sites.
// ─────────────────────────────────────────────────────────────────────────────

/// Standard hint emitted by `link` / `unlink` / `activate` after the local
/// target's R_eff has already been recomputed inline via
/// [`crate::scoring::sync_score_target`]. The remaining work is parent
/// reconciliation up the dependency chain, which is `forgeplan score-all`'s
/// responsibility (PRD-075 §Non-Goals — bounded mutator latency).
pub fn reconcile_parents_hint() -> Hint {
    Hint::info("Reconcile parents up the chain").with_action("forgeplan score-all".to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Domain-specific hint collectors
// ─────────────────────────────────────────────────────────────────────────────

/// Hints for `forgeplan score` output.
///
/// `artifact_id` — the real ID being scored (e.g. `PRD-001`). Substituted into
/// every emitted action so hints are runnable verbatim. Per PRD-071 ACTIONABILITY
/// contract: target IDs MUST be real, only "value-to-fill" placeholders allowed
/// (e.g. `<verification>` for evidence title, `EVID-NNN` for yet-to-exist evidence).
pub fn score_hints(
    artifact_id: &str,
    r_eff: f64,
    has_evidence: bool,
    evidence_cl0_count: usize,
) -> Vec<Hint> {
    let mut hints = Vec::new();

    if !has_evidence {
        hints.push(
            Hint::warning("No evidence linked — R_eff will be 0.0")
                .with_action(format!(
                    "forgeplan new evidence \"<verification>\" && forgeplan link EVID-NNN {} --relation informs",
                    artifact_id
                ))
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
                .with_action("Add more evidence or improve congruence level (CL3 = same context)"),
        );
    }

    hints
}

/// Hints for `forgeplan get` output.
/// Accepts typed enums for kind and depth for compile-time safety.
///
/// `artifact_id` — the real ID being inspected (e.g. `PRD-001`). Substituted into
/// every emitted action so hints are runnable verbatim. `<parent-id>` and
/// `<title>` remain as user-fill placeholders (values not knowable here).
pub fn get_hints(
    artifact_id: &str,
    status: &str,
    kind: &crate::artifact::types::ArtifactKind,
    has_links: bool,
    depth: &crate::artifact::types::Mode,
) -> Vec<Hint> {
    let mut hints = Vec::new();
    let kind_str = kind_to_str(kind);
    let depth_str = match depth {
        crate::artifact::types::Mode::Tactical => "tactical",
        crate::artifact::types::Mode::Standard => "standard",
        crate::artifact::types::Mode::Deep => "deep",
        crate::artifact::types::Mode::Note => "note",
    };

    if status == "draft" {
        let next = match kind_str {
            "prd" => {
                "Fill MUST sections (Problem, Goals, FR, Target Users), then: forgeplan validate"
            }
            "rfc" => {
                "Fill Summary, Motivation, Options, Implementation Phases, then: forgeplan validate"
            }
            "adr" => "Fill Context, Decision, Consequences, then: forgeplan validate",
            "evidence" => {
                "Fill Structured Fields (verdict, congruence_level, evidence_type), then: forgeplan activate"
            }
            _ => "Fill required sections, then: forgeplan validate",
        };
        hints.push(Hint::suggestion(format!(
            "Status is draft — next: {}",
            next
        )));
    }

    if !has_links && status != "deprecated" && kind_str != "memory" {
        hints.push(
            Hint::info("No links — artifact is an orphan").with_action(format!(
                "forgeplan link {} <parent-id> --relation refines",
                artifact_id
            )),
        );
    }

    if depth_str == "standard" && kind_str == "prd" && !has_links {
        hints.push(
            Hint::suggestion("Standard PRD should have linked RFC").with_action(format!(
                "forgeplan new rfc \"<title>\" && forgeplan link RFC-NNN {} --relation based_on",
                artifact_id
            )),
        );
    }

    hints
}

/// Hints for `forgeplan review` output.
///
/// `artifact_id` — the real ID being reviewed (e.g. `PRD-001`). Substituted into
/// every emitted action so hints are runnable verbatim.
pub fn review_hints(
    artifact_id: &str,
    has_evidence: bool,
    is_stub: bool,
    has_must_errors: bool,
    kind: &crate::artifact::types::ArtifactKind,
) -> Vec<Hint> {
    let mut hints = Vec::new();

    if is_stub {
        let action = match kind_to_str(kind) {
            "prd" => "Fill: Problem, Goals, Non-Goals, Target Users, FR",
            "rfc" => "Fill: Summary, Motivation, Goals, Options, Implementation Phases",
            _ => "Fill all required sections",
        };
        hints.push(
            Hint::warning("Artifact is a stub — MUST sections not filled").with_action(action),
        );
    }

    if has_must_errors {
        hints.push(
            Hint::warning("Validation has MUST errors — fix before activating").with_action(
                format!("forgeplan validate {} to see specific errors", artifact_id),
            ),
        );
    }

    if !has_evidence {
        hints.push(
            Hint::info("No evidence linked — consider adding evidence before activation")
                .with_action(format!(
                    "forgeplan new evidence \"<verification>\" && forgeplan link EVID-NNN {} --relation informs",
                    artifact_id
                ))
        );
    }

    hints
}

/// Hints for `forgeplan search` when no results found.
///
/// The `query` is shown back to the user so they know which input failed.
/// Per PRD-071 ACTIONABILITY: we don't know what a "shorter" query would be,
/// so we surface a different surface entirely (`forgeplan list`) as the
/// concrete next action — that's a runnable command with no placeholders.
pub fn search_hints(query: &str, result_count: usize) -> Vec<Hint> {
    let mut hints = Vec::new();

    if result_count == 0 {
        hints.push(Hint::info(format!("No results for \"{}\"", query)));
        hints.push(
            Hint::suggestion(format!(
                "Try broader keywords or check spelling (current query: \"{}\")",
                query
            ))
            .with_action("forgeplan list --type prd"),
        );
        hints.push(Hint::suggestion(
            "Search by kind: forgeplan list --type prd",
        ));
    }

    hints
}

/// Hints for `forgeplan activate` when activation fails.
///
/// `artifact_id` — the real ID being activated (e.g. `PRD-001`). Substituted into
/// every emitted action so hints are runnable verbatim.
pub fn activate_hints(
    artifact_id: &str,
    validation_passed: bool,
    has_evidence: bool,
    kind: &crate::artifact::types::ArtifactKind,
) -> Vec<Hint> {
    let mut hints = Vec::new();

    if !validation_passed {
        hints.push(
            Hint::warning("Validation gate failed — fix MUST errors before activating")
                .with_action(format!("forgeplan validate {}", artifact_id)),
        );
    }

    if !has_evidence
        && matches!(
            kind,
            crate::artifact::types::ArtifactKind::Prd
                | crate::artifact::types::ArtifactKind::Rfc
                | crate::artifact::types::ArtifactKind::Adr
        )
    {
        hints.push(
            Hint::suggestion("Link evidence before activating for non-zero R_eff")
                .with_action(format!(
                    "forgeplan new evidence \"<verification>\" && forgeplan link EVID-NNN {} --relation informs",
                    artifact_id
                ))
        );
    }

    hints
}

/// Hints for `forgeplan blocked` — suggest how to unblock.
pub fn blocked_hints(blocked_by: &[(String, String)]) -> Vec<Hint> {
    let mut hints = Vec::new();

    let draft_blockers: Vec<_> = blocked_by
        .iter()
        .filter(|(_, status)| status == "draft")
        .collect();

    if !draft_blockers.is_empty() {
        for (id, _) in &draft_blockers {
            hints.push(
                Hint::suggestion(format!("Activate draft dependency {}", id)).with_action(format!(
                    "forgeplan review {} && forgeplan activate {}",
                    id, id
                )),
            );
        }
    }

    hints
}

fn kind_to_str(kind: &crate::artifact::types::ArtifactKind) -> &'static str {
    use crate::artifact::types::ArtifactKind;
    match kind {
        ArtifactKind::Prd => "prd",
        ArtifactKind::Epic => "epic",
        ArtifactKind::Spec => "spec",
        ArtifactKind::Rfc => "rfc",
        ArtifactKind::Adr => "adr",
        ArtifactKind::Note => "note",
        ArtifactKind::ProblemCard => "problem",
        ArtifactKind::SolutionPortfolio => "solution",
        ArtifactKind::EvidencePack => "evidence",
        ArtifactKind::RefreshReport => "refresh",
        ArtifactKind::Memory => "memory",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_hints_no_evidence() {
        let hints = score_hints("PRD-001", 0.0, false, 0);
        assert_eq!(hints.len(), 1);
        assert!(hints[0].message.contains("No evidence"));
        assert!(hints[0].action.is_some());
        // PRD-071 ACTIONABILITY: real target ID, no `<artifact>` placeholder.
        let action = hints[0].action.as_ref().unwrap();
        assert!(
            action.contains("PRD-001"),
            "expected real ID in action: {}",
            action
        );
        assert!(!action.contains("<artifact>"));
    }

    #[test]
    fn score_hints_cl0() {
        let hints = score_hints("PRD-002", 0.7, true, 2);
        assert_eq!(hints.len(), 1);
        assert!(hints[0].message.contains("CL0"));
    }

    #[test]
    fn score_hints_low_reff() {
        let hints = score_hints("PRD-003", 0.3, true, 0);
        assert_eq!(hints.len(), 1);
        assert!(hints[0].message.contains("below 0.5"));
    }

    #[test]
    fn get_hints_draft_prd() {
        use crate::artifact::types::{ArtifactKind, Mode};
        let hints = get_hints(
            "PRD-001",
            "draft",
            &ArtifactKind::Prd,
            false,
            &Mode::Standard,
        );
        assert!(hints.len() >= 2);
        assert!(hints[0].message.contains("draft"));
        // PRD-071 ACTIONABILITY: actions on this artifact use real ID, no
        // `<this-id>` placeholder.
        for h in &hints {
            if let Some(action) = &h.action {
                assert!(
                    !action.contains("<this-id>"),
                    "action still has <this-id> placeholder: {}",
                    action
                );
            }
        }
    }

    #[test]
    fn get_hints_active_with_links() {
        use crate::artifact::types::{ArtifactKind, Mode};
        let hints = get_hints(
            "PRD-001",
            "active",
            &ArtifactKind::Prd,
            true,
            &Mode::Standard,
        );
        assert!(hints.is_empty());
    }

    #[test]
    fn review_hints_stub() {
        use crate::artifact::types::ArtifactKind;
        let hints = review_hints("PRD-001", false, true, false, &ArtifactKind::Prd);
        assert!(hints.len() >= 2);
        assert!(hints[0].message.contains("stub"));
        // PRD-071 ACTIONABILITY: target ID is real for evidence-add hint.
        for h in &hints {
            if let Some(action) = &h.action
                && action.contains("forgeplan link")
            {
                assert!(action.contains("PRD-001"), "expected real ID: {}", action);
                assert!(!action.contains("<id>"));
            }
        }
    }

    #[test]
    fn search_hints_empty() {
        let hints = search_hints("nonexistent", 0);
        assert_eq!(hints.len(), 3);
        assert!(hints[0].message.contains("No results"));
        // PRD-071 ACTIONABILITY: no `<shorter query>` placeholder anywhere.
        for h in &hints {
            if let Some(action) = &h.action {
                assert!(!action.contains("<shorter query>"));
            }
        }
    }

    #[test]
    fn activate_hints_no_evidence_prd() {
        use crate::artifact::types::ArtifactKind;
        let hints = activate_hints("PRD-001", true, false, &ArtifactKind::Prd);
        assert_eq!(hints.len(), 1);
        assert!(hints[0].message.contains("evidence"));
        // PRD-071 ACTIONABILITY: real target ID in link command.
        let action = hints[0].action.as_ref().unwrap();
        assert!(action.contains("PRD-001"), "expected real ID: {}", action);
        assert!(!action.contains("<id>"));
    }

    #[test]
    fn activate_hints_all_good() {
        use crate::artifact::types::ArtifactKind;
        let hints = activate_hints("PRD-001", true, true, &ArtifactKind::Prd);
        assert!(hints.is_empty());
    }

    #[test]
    fn blocked_hints_draft_dependency() {
        let blockers = vec![
            ("ADR-002".to_string(), "draft".to_string()),
            ("RFC-001".to_string(), "active".to_string()),
        ];
        let hints = blocked_hints(&blockers);
        assert_eq!(hints.len(), 1); // only draft blocker gets a hint
        assert!(hints[0].message.contains("ADR-002"));
    }

    #[test]
    fn primary_action_returns_first_with_action() {
        let hints = vec![
            Hint::info("info no action"),
            Hint::warning("warn with action").with_action("forgeplan score PRD-001"),
            Hint::suggestion("suggest with action").with_action("ignored"),
        ];
        assert_eq!(
            primary_action(&hints),
            Some("forgeplan score PRD-001".to_string())
        );
    }

    #[test]
    fn primary_action_none_when_no_actions() {
        let hints = vec![Hint::info("just info"), Hint::warning("just warning")];
        assert_eq!(primary_action(&hints), None);
    }

    #[test]
    fn primary_action_none_when_empty() {
        assert_eq!(primary_action(&[]), None);
    }

    #[test]
    fn render_next_action_line_with_action() {
        let hints = vec![Hint::warning("R_eff is 0").with_action("forgeplan score PRD-001")];
        let rendered = render_next_action_line(&hints);
        assert!(rendered.contains("Next: forgeplan score PRD-001"));
    }

    #[test]
    fn render_next_action_line_terminal_returns_empty() {
        let hints = vec![Hint::info("Workflow complete")];
        assert_eq!(render_next_action_line(&hints), "");
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
