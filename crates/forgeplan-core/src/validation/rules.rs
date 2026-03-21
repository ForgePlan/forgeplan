use crate::artifact::frontmatter::Frontmatter;
use crate::artifact::types::{ArtifactKind, Mode};
use crate::validation::checks;
use crate::validation::Severity;

/// A rule entry: (rule_id, severity, description, check_fn).
/// check_fn returns Some(error_message) if the rule fails, None if it passes.
type CheckFn = fn(&str, &Frontmatter) -> Option<String>;
pub type RuleEntry = (&'static str, Severity, &'static str, CheckFn);

/// Get validation rules for a given artifact kind and depth.
pub fn rules_for(kind: &ArtifactKind, depth: &Mode) -> Vec<RuleEntry> {
    let mut rules = base_rules();
    match kind {
        ArtifactKind::Prd => rules.extend(prd_rules(depth)),
        ArtifactKind::Epic => rules.extend(epic_rules(depth)),
        ArtifactKind::Spec => rules.extend(spec_rules(depth)),
        ArtifactKind::Rfc => rules.extend(rfc_rules(depth)),
        ArtifactKind::Adr => rules.extend(adr_rules(depth)),
        _ => {} // Quint-code types: base rules only
    }
    rules
}

// ─── Helper: wrap check fn ──────────────────────────────────────────────────

fn rule(
    id: &'static str,
    sev: Severity,
    desc: &'static str,
    f: CheckFn,
) -> RuleEntry {
    (id, sev, desc, f)
}

// ─── Base Rules ─────────────────────────────────────────────────────────────

fn base_rules() -> Vec<RuleEntry> {
    vec![
        rule("meta-id", Severity::Must, "Frontmatter must have 'id'", check_meta_id),
        rule("meta-status", Severity::Must, "Frontmatter must have 'status'", check_meta_status),
        rule("no-placeholders", Severity::Should, "No {{placeholder}} or TODO", check_no_placeholders),
    ]
}

fn check_meta_id(_body: &str, fm: &Frontmatter) -> Option<String> {
    if !checks::frontmatter_has(fm, "id") {
        Some("Missing 'id' field in frontmatter".into())
    } else {
        None
    }
}

fn check_meta_status(_body: &str, fm: &Frontmatter) -> Option<String> {
    if !checks::frontmatter_has(fm, "status") {
        Some("Missing 'status' field in frontmatter".into())
    } else {
        None
    }
}

fn check_no_placeholders(body: &str, _fm: &Frontmatter) -> Option<String> {
    let placeholders = checks::find_placeholders(body);
    if placeholders.is_empty() {
        None
    } else {
        let details: Vec<String> = placeholders
            .iter()
            .take(3)
            .map(|(line, text)| format!("line {}: {}", line, text))
            .collect();
        Some(format!(
            "Found {} placeholder(s): {}",
            placeholders.len(),
            details.join(", ")
        ))
    }
}

// ─── PRD Rules ──────────────────────────────────────────────────────────────

fn prd_rules(depth: &Mode) -> Vec<RuleEntry> {
    let mut rules = vec![
        rule("prd-problem-exists", Severity::Must, "Problem Statement", check_prd_problem),
        rule("prd-goals-exist", Severity::Must, "Goals section", check_prd_goals),
        rule("prd-non-goals", Severity::Must, "Non-Goals section", check_prd_non_goals),
        rule("prd-fr-exist", Severity::Must, "Functional Requirements", check_prd_fr),
        rule("prd-related", Severity::Must, "Related Artifacts", check_prd_related),
    ];

    if matches!(depth, Mode::Standard | Mode::Deep) {
        let density_sev = if matches!(depth, Mode::Deep) { Severity::Must } else { Severity::Should };
        let leakage_sev = if matches!(depth, Mode::Deep) { Severity::Must } else { Severity::Should };
        rules.push(rule("prd-problem-density", density_sev, "Problem density >= 50 words", check_prd_density));
        rules.push(rule("prd-target-audience", Severity::Must, "Target Audience", check_prd_audience));
        rules.push(rule("prd-no-impl-leakage", leakage_sev, "No tech in FR", check_prd_leakage));
    }

    if matches!(depth, Mode::Deep) {
        rules.push(rule("prd-timeline", Severity::Must, "Timeline section", check_prd_timeline));
        rules.push(rule("prd-stakeholders", Severity::Must, "Stakeholders", check_prd_stakeholders));
        rules.push(rule("prd-acceptance", Severity::Must, "Acceptance Criteria", check_prd_acceptance));
    }

    rules
}

fn check_prd_problem(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Problem") {
        Some("Missing '## Problem' or '## Problem Statement' section".into())
    } else {
        None
    }
}

fn check_prd_goals(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Goals") && !checks::section_exists(body, "Success Criteria") {
        Some("Missing '## Goals' or '## Success Criteria' section".into())
    } else {
        None
    }
}

fn check_prd_non_goals(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Non-Goals") && !checks::section_exists(body, "Out of Scope") {
        Some("Missing '## Non-Goals' or '## Out of Scope' section".into())
    } else {
        None
    }
}

fn check_prd_fr(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Functional Requirements") && !checks::section_exists(body, "Requirements") {
        Some("Missing '## Functional Requirements' section".into())
    } else {
        None
    }
}

fn check_prd_related(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Related Artifacts") && !checks::section_exists(body, "Related") {
        Some("Missing '## Related Artifacts' section".into())
    } else {
        None
    }
}

fn check_prd_density(body: &str, _fm: &Frontmatter) -> Option<String> {
    let wc = checks::section_word_count(body, "Problem");
    if wc > 0 && wc < 50 {
        Some(format!("Problem section has {} words (expected >= 50)", wc))
    } else {
        None
    }
}

fn check_prd_audience(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Target") && !checks::section_exists(body, "Audience") && !checks::section_exists(body, "Users") {
        Some("Missing target audience/users section (standard+ depth)".into())
    } else {
        None
    }
}

fn check_prd_leakage(body: &str, _fm: &Frontmatter) -> Option<String> {
    if let Some(start) = body.find("## Functional Requirements") {
        let fr_text = &body[start..];
        let end = fr_text[30..].find("\n## ").map(|i| i + 30).unwrap_or(fr_text.len());
        let fr_section = &fr_text[..end];
        let leaks = checks::find_tech_leakage(fr_section);
        if leaks.is_empty() {
            None
        } else {
            let names: Vec<String> = leaks.iter().map(|(_, name)| name.clone()).collect();
            Some(format!("Tech names in FR section: {}", names.join(", ")))
        }
    } else {
        None
    }
}

fn check_prd_timeline(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Timeline") {
        Some("Missing '## Timeline' section (required for deep depth)".into())
    } else {
        None
    }
}

fn check_prd_stakeholders(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Stakeholders") {
        Some("Missing '## Stakeholders' section (required for deep depth)".into())
    } else {
        None
    }
}

fn check_prd_acceptance(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Acceptance Criteria") {
        Some("Missing '## Acceptance Criteria' section (required for deep depth)".into())
    } else {
        None
    }
}

// ─── Epic Rules ─────────────────────────────────────────────────────────────

fn epic_rules(_depth: &Mode) -> Vec<RuleEntry> {
    vec![
        rule("epic-vision", Severity::Must, "Vision section", check_epic_vision),
        rule("epic-outcomes", Severity::Must, "Outcomes section", check_epic_outcomes),
        rule("epic-children", Severity::Must, "Children table", check_epic_children),
        rule("epic-phases", Severity::Must, "Phases section", check_epic_phases),
        rule("epic-progress", Severity::Must, "Progress bars", check_epic_progress),
    ]
}

fn check_epic_vision(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Vision") {
        Some("Missing '## Vision' section".into())
    } else {
        None
    }
}

fn check_epic_outcomes(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Outcomes") {
        Some("Missing '## Outcomes' section".into())
    } else {
        None
    }
}

fn check_epic_children(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Children") && !checks::section_exists(body, "Artifacts") {
        Some("Missing children/artifacts table section".into())
    } else {
        None
    }
}

fn check_epic_phases(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Phases") && !checks::section_exists(body, "Phase") {
        Some("Missing '## Phases' section".into())
    } else {
        None
    }
}

fn check_epic_progress(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Progress") {
        Some("Missing '## Progress' section with aggregated bars".into())
    } else {
        None
    }
}

// ─── Spec Rules ─────────────────────────────────────────────────────────────

fn spec_rules(_depth: &Mode) -> Vec<RuleEntry> {
    vec![
        rule("spec-summary", Severity::Must, "Summary section", check_spec_summary),
        rule("spec-contracts", Severity::Must, "API/Data Model", check_spec_contracts),
        rule("spec-related", Severity::Must, "Related Artifacts", check_spec_related),
    ]
}

fn check_spec_summary(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Summary") {
        Some("Missing '## Summary' section".into())
    } else {
        None
    }
}

fn check_spec_contracts(body: &str, _fm: &Frontmatter) -> Option<String> {
    let has_api = checks::section_exists(body, "API");
    let has_data = checks::section_exists(body, "Data Model");
    let has_contracts = checks::section_exists(body, "Contracts");
    if !has_api && !has_data && !has_contracts {
        Some("Missing '## API Contracts' or '## Data Models' section".into())
    } else {
        None
    }
}

fn check_spec_related(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Related") {
        Some("Missing '## Related Artifacts' section".into())
    } else {
        None
    }
}

// ─── RFC Rules ──────────────────────────────────────────────────────────────

fn rfc_rules(depth: &Mode) -> Vec<RuleEntry> {
    let mut rules = vec![
        rule("rfc-summary", Severity::Must, "Summary section", check_rfc_summary),
        rule("rfc-motivation", Severity::Must, "Motivation section", check_rfc_motivation),
        rule("rfc-options", Severity::Should, "Options Considered", check_rfc_options),
        rule("rfc-proposed", Severity::Must, "Proposed Direction", check_rfc_proposed),
        rule("rfc-phases", Severity::Should, "Implementation Phases", check_rfc_phases),
    ];

    if matches!(depth, Mode::Deep) {
        rules.push(rule("rfc-risks", Severity::Must, "Risks section", check_rfc_risks));
    }

    rules
}

fn check_rfc_summary(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Summary") {
        Some("Missing '## Summary' section".into())
    } else {
        None
    }
}

fn check_rfc_motivation(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Motivation") {
        Some("Missing '## Motivation' section".into())
    } else {
        None
    }
}

fn check_rfc_options(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Options") && !checks::section_exists(body, "Alternatives") {
        Some("Missing '## Options Considered' section".into())
    } else {
        None
    }
}

fn check_rfc_proposed(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Proposed") && !checks::section_exists(body, "Direction") && !checks::section_exists(body, "Architecture") {
        Some("Missing '## Proposed Direction' or '## Architecture' section".into())
    } else {
        None
    }
}

fn check_rfc_phases(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Implementation") && !checks::section_exists(body, "Phases") {
        Some("Missing '## Implementation Phases' section".into())
    } else {
        None
    }
}

fn check_rfc_risks(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Risks") {
        Some("Missing '## Risks' section (required for deep depth)".into())
    } else {
        None
    }
}

// ─── ADR Rules ──────────────────────────────────────────────────────────────

fn adr_rules(depth: &Mode) -> Vec<RuleEntry> {
    let mut rules = vec![
        rule("adr-context", Severity::Must, "Context section", check_adr_context),
        rule("adr-decision", Severity::Must, "Decision section", check_adr_decision),
        rule("adr-consequences", Severity::Must, "Consequences", check_adr_consequences),
    ];

    if matches!(depth, Mode::Deep) {
        rules.push(rule("adr-invariants", Severity::Should, "Invariants (DDR)", check_adr_invariants));
        rules.push(rule("adr-rollback", Severity::Should, "Rollback Plan", check_adr_rollback));
    }

    rules
}

fn check_adr_context(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Context") {
        Some("Missing '## Context' section".into())
    } else {
        None
    }
}

fn check_adr_decision(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Decision") {
        Some("Missing '## Decision' section".into())
    } else {
        None
    }
}

fn check_adr_consequences(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Consequences") {
        Some("Missing '## Consequences' section".into())
    } else {
        None
    }
}

fn check_adr_invariants(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Invariants") {
        Some("Missing '## Invariants' section (recommended for deep ADR/DDR)".into())
    } else {
        None
    }
}

fn check_adr_rollback(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Rollback") {
        Some("Missing '## Rollback Plan' section (recommended for deep ADR/DDR)".into())
    } else {
        None
    }
}
