use crate::artifact::frontmatter::Frontmatter;
use crate::artifact::types::{ArtifactKind, Mode};
use crate::validation::Severity;
use crate::validation::checks;

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

fn rule(id: &'static str, sev: Severity, desc: &'static str, f: CheckFn) -> RuleEntry {
    (id, sev, desc, f)
}

// ─── Base Rules ─────────────────────────────────────────────────────────────

fn base_rules() -> Vec<RuleEntry> {
    vec![
        rule(
            "meta-id",
            Severity::Must,
            "Frontmatter must have 'id'",
            check_meta_id,
        ),
        rule(
            "meta-status",
            Severity::Must,
            "Frontmatter must have 'status'",
            check_meta_status,
        ),
        rule(
            "no-placeholders",
            Severity::Should,
            "No {{placeholder}} or TODO",
            check_no_placeholders,
        ),
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
        rule(
            "prd-problem-exists",
            Severity::Must,
            "Problem Statement",
            check_prd_problem,
        ),
        rule(
            "prd-goals-exist",
            Severity::Must,
            "Goals section",
            check_prd_goals,
        ),
        rule(
            "prd-non-goals",
            Severity::Must,
            "Non-Goals section",
            check_prd_non_goals,
        ),
        rule(
            "prd-fr-exist",
            Severity::Must,
            "Functional Requirements",
            check_prd_fr,
        ),
        rule(
            "prd-related",
            Severity::Must,
            "Related Artifacts",
            check_prd_related,
        ),
    ];

    if matches!(depth, Mode::Standard | Mode::Deep) {
        let density_sev = if matches!(depth, Mode::Deep) {
            Severity::Must
        } else {
            Severity::Should
        };
        let leakage_sev = if matches!(depth, Mode::Deep) {
            Severity::Must
        } else {
            Severity::Should
        };
        rules.push(rule(
            "prd-problem-density",
            density_sev,
            "Problem density >= 50 words",
            check_prd_density,
        ));
        rules.push(rule(
            "prd-target-audience",
            Severity::Must,
            "Target Audience",
            check_prd_audience,
        ));
        rules.push(rule(
            "prd-no-impl-leakage",
            leakage_sev,
            "No tech in FR",
            check_prd_leakage,
        ));
    }

    if matches!(depth, Mode::Deep) {
        rules.push(rule(
            "prd-timeline",
            Severity::Must,
            "Timeline section",
            check_prd_timeline,
        ));
        rules.push(rule(
            "prd-stakeholders",
            Severity::Must,
            "Stakeholders",
            check_prd_stakeholders,
        ));
        rules.push(rule(
            "prd-acceptance",
            Severity::Must,
            "Acceptance Criteria",
            check_prd_acceptance,
        ));
        rules.push(rule(
            "prd-risk-assessment",
            Severity::Must,
            "Risk Assessment",
            check_prd_risk,
        ));
        rules.push(rule(
            "prd-rollback",
            Severity::Should,
            "Rollback Plan",
            check_prd_rollback,
        ));
        rules.push(rule(
            "prd-success-metrics",
            Severity::Must,
            "Success Metrics",
            check_prd_success_metrics,
        ));
        rules.push(rule(
            "prd-dependencies",
            Severity::Should,
            "Dependencies",
            check_prd_dependencies,
        ));
    }

    // FR format check — [Actor] can [capability]
    rules.push(rule(
        "prd-fr-format",
        Severity::Could,
        "FR format: [Actor] can [capability]",
        check_prd_fr_format,
    ));

    // BMAD Step 5: Measurability checks
    rules.push(rule(
        "prd-measurability-adjectives",
        Severity::Should,
        "FR should not contain subjective adjectives without metrics",
        check_prd_measurability_adjectives,
    ));
    rules.push(rule(
        "prd-vague-quantifiers",
        Severity::Should,
        "FR should not contain vague quantifiers",
        check_prd_vague_quantifiers,
    ));

    // BMAD Step 3: Filler phrase detection
    rules.push(rule(
        "prd-filler-phrases",
        Severity::Should,
        "Body should not contain filler phrases",
        check_prd_filler_phrases,
    ));
    rules.push(rule(
        "prd-density-score",
        Severity::Could,
        "Information density should be high (filler < 5% of words)",
        check_prd_density_score,
    ));

    // BMAD Step 6: Traceability validation
    rules.push(rule(
        "prd-orphan-frs",
        Severity::Should,
        "All FRs should be referenced outside FR section",
        check_prd_orphan_frs,
    ));
    rules.push(rule(
        "prd-orphan-goals",
        Severity::Should,
        "All Goals should be supported by FRs",
        check_prd_orphan_goals,
    ));

    // BMAD Step 8: Domain classification
    rules.push(rule(
        "prd-domain-sections",
        Severity::Must,
        "Domain-specific required sections",
        check_prd_domain_sections,
    ));

    // BMAD Step 9: Project-type classification
    rules.push(rule(
        "prd-project-type-sections",
        Severity::Should,
        "Project-type recommended sections",
        check_prd_project_type_sections,
    ));

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
    if !checks::section_exists(body, "Functional Requirements")
        && !checks::section_exists(body, "Requirements")
    {
        Some("Missing '## Functional Requirements' section".into())
    } else {
        None
    }
}

fn check_prd_related(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Related Artifacts")
        && !checks::section_exists(body, "Related")
    {
        Some("Missing '## Related Artifacts' section".into())
    } else {
        None
    }
}

fn check_prd_density(body: &str, _fm: &Frontmatter) -> Option<String> {
    let wc = checks::section_word_count(body, "Problem");
    if wc < 50 {
        Some(format!("Problem section has {} words (expected >= 50)", wc))
    } else {
        None
    }
}

fn check_prd_audience(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Target")
        && !checks::section_exists(body, "Audience")
        && !checks::section_exists(body, "Users")
    {
        Some("Missing target audience/users section (standard+ depth)".into())
    } else {
        None
    }
}

fn check_prd_leakage(body: &str, _fm: &Frontmatter) -> Option<String> {
    let mut all_leaks: Vec<String> = Vec::new();

    // Check FR section
    if let Some(fr_section) = checks::extract_fr_section(body) {
        for (_, name) in checks::find_tech_leakage(&fr_section) {
            all_leaks.push(name);
        }
    }

    // Check NFR section
    if let Some(nfr_section) = checks::extract_nfr_section(body) {
        for (_, name) in checks::find_tech_leakage(&nfr_section) {
            all_leaks.push(name);
        }
    }

    if all_leaks.is_empty() {
        None
    } else {
        all_leaks.sort();
        all_leaks.dedup();
        Some(format!(
            "Tech names in FR/NFR sections: {}",
            all_leaks.join(", ")
        ))
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

// ─── Extended PRD Validation Rules ──────────────────────────────────────────

fn check_prd_risk(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Risk") {
        Some("Missing '## Risk Assessment' or '## Risks' section (deep depth)".into())
    } else {
        None
    }
}

fn check_prd_rollback(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Rollback") && !checks::section_exists(body, "Revert") {
        Some("Missing rollback/revert plan (deep depth)".into())
    } else {
        None
    }
}

fn check_prd_success_metrics(body: &str, _fm: &Frontmatter) -> Option<String> {
    // Use extract_section for proper heading detection (fixes audit C1)
    let section_text = checks::extract_section(body, "Success Metrics")
        .or_else(|| checks::extract_section(body, "Success Criteria"));

    match section_text {
        None => Some("Missing '## Success Metrics' or '## Success Criteria' section".into()),
        Some(text) => {
            let has_measurable = text.contains('%')
                || text.contains("< ")
                || text.contains("> ")
                || text.chars().any(|c| c.is_ascii_digit());
            if !has_measurable {
                Some(
                    "Success metrics section has no measurable values (numbers, percentages)"
                        .into(),
                )
            } else {
                None
            }
        }
    }
}

fn check_prd_dependencies(body: &str, _fm: &Frontmatter) -> Option<String> {
    if !checks::section_exists(body, "Dependencies") && !checks::section_exists(body, "Depends") {
        Some("Missing '## Dependencies' section (deep depth)".into())
    } else {
        None
    }
}

/// FR format check — [Actor] can [capability] (or - [ ] FR-NNN: ...)
fn check_prd_fr_format(body: &str, _fm: &Frontmatter) -> Option<String> {
    // First check: FR section has items at all
    if let Some(fr_content) = checks::extract_fr_section(body) {
        let fr_lines: Vec<&str> = fr_content
            .lines()
            .filter(|l| {
                let t = l.trim();
                t.starts_with("- [") || t.starts_with("* [") || t.starts_with("- FR-")
            })
            .collect();

        if fr_lines.is_empty() {
            return Some("Functional Requirements section has no FR items (use checkboxes: - [ ] FR-001: ...)".into());
        }

        // Second check: FR items follow [Actor] can [capability] format
        let bad_lines = checks::check_fr_format(body);
        if !bad_lines.is_empty() {
            let details: Vec<String> = bad_lines
                .iter()
                .take(3)
                .map(|(text, line)| format!("line {}: '{}'", line, text))
                .collect();
            return Some(format!(
                "FR items not in '[Actor] can [capability]' format: {}",
                details.join("; ")
            ));
        }

        None
    } else {
        None // No FR section — caught by check_prd_fr
    }
}

fn check_prd_measurability_adjectives(body: &str, _fm: &Frontmatter) -> Option<String> {
    let findings = checks::check_measurability_adjectives(body);
    if findings.is_empty() {
        None
    } else {
        let details: Vec<String> = findings
            .iter()
            .take(5)
            .map(|(word, line)| format!("'{}' at line {}", word, line))
            .collect();
        Some(format!(
            "Subjective adjectives in FR: {}",
            details.join(", ")
        ))
    }
}

fn check_prd_vague_quantifiers(body: &str, _fm: &Frontmatter) -> Option<String> {
    let findings = checks::check_vague_quantifiers(body);
    if findings.is_empty() {
        None
    } else {
        let details: Vec<String> = findings
            .iter()
            .take(5)
            .map(|(word, line)| format!("'{}' at line {}", word, line))
            .collect();
        Some(format!("Vague quantifiers in FR: {}", details.join(", ")))
    }
}

fn check_prd_filler_phrases(body: &str, _fm: &Frontmatter) -> Option<String> {
    let findings = checks::check_filler_phrases(body);
    if findings.is_empty() {
        None
    } else {
        let details: Vec<String> = findings
            .iter()
            .take(5)
            .map(|(phrase, replacement, line)| {
                if replacement.is_empty() {
                    format!("line {}: remove '{}'", line, phrase)
                } else {
                    format!("line {}: '{}' -> '{}'", line, phrase, replacement)
                }
            })
            .collect();
        Some(format!(
            "{} filler phrase(s): {}",
            findings.len(),
            details.join("; ")
        ))
    }
}

fn check_prd_density_score(body: &str, _fm: &Frontmatter) -> Option<String> {
    let score = checks::density_score(body);
    if score > 0.05 {
        Some(format!(
            "Density score: {:.1}% filler (threshold: 5%)",
            score * 100.0
        ))
    } else {
        None
    }
}

// ─── BMAD Step 6: Traceability ──────────────────────────────────────────────

fn check_prd_orphan_frs(body: &str, _fm: &Frontmatter) -> Option<String> {
    let orphans = checks::find_orphan_frs(body);
    if orphans.is_empty() {
        None
    } else {
        Some(format!(
            "Orphan FRs (not referenced outside FR section): {}",
            orphans.join(", ")
        ))
    }
}

fn check_prd_orphan_goals(body: &str, _fm: &Frontmatter) -> Option<String> {
    let orphans = checks::find_orphan_goals(body);
    if orphans.is_empty() {
        None
    } else {
        let details: Vec<String> = orphans.iter().take(3).map(|g| format!("'{}'", g)).collect();
        Some(format!(
            "Goals not supported by any FR: {}",
            details.join(", ")
        ))
    }
}

// ─── BMAD Step 8: Domain Classification ─────────────────────────────────────

fn check_prd_domain_sections(body: &str, fm: &Frontmatter) -> Option<String> {
    let domain = match fm.get("domain") {
        Some(serde_yml::Value::String(s)) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return None, // No domain set — skip check
    };

    let required = checks::domain_required_sections(&domain);
    if required.is_empty() {
        return None;
    }

    let missing: Vec<String> = required
        .iter()
        .filter(|(heading, _)| !checks::section_exists(body, heading))
        .map(|(_, desc)| desc.to_string())
        .collect();

    if missing.is_empty() {
        None
    } else {
        Some(format!(
            "Domain '{}' requires: {}",
            domain,
            missing.join(", ")
        ))
    }
}

// ─── BMAD Step 9: Project-Type Classification ───────────────────────────────

fn check_prd_project_type_sections(body: &str, fm: &Frontmatter) -> Option<String> {
    let project_type = match fm.get("project_type") {
        Some(serde_yml::Value::String(s)) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return None, // No project_type set — skip check
    };

    let recommended = checks::project_type_recommended_sections(&project_type);
    if recommended.is_empty() {
        return None;
    }

    let missing: Vec<String> = recommended
        .iter()
        .filter(|(heading, _)| !checks::section_exists(body, heading))
        .map(|(_, desc)| desc.to_string())
        .collect();

    if missing.is_empty() {
        None
    } else {
        Some(format!(
            "Project type '{}' recommends: {}",
            project_type,
            missing.join(", ")
        ))
    }
}

// ─── Epic Rules ─────────────────────────────────────────────────────────────

fn epic_rules(_depth: &Mode) -> Vec<RuleEntry> {
    vec![
        rule(
            "epic-vision",
            Severity::Must,
            "Vision section",
            check_epic_vision,
        ),
        rule(
            "epic-outcomes",
            Severity::Must,
            "Outcomes section",
            check_epic_outcomes,
        ),
        rule(
            "epic-children",
            Severity::Must,
            "Children table",
            check_epic_children,
        ),
        rule(
            "epic-phases",
            Severity::Must,
            "Phases section",
            check_epic_phases,
        ),
        rule(
            "epic-progress",
            Severity::Must,
            "Progress bars",
            check_epic_progress,
        ),
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
        rule(
            "spec-summary",
            Severity::Must,
            "Summary section",
            check_spec_summary,
        ),
        rule(
            "spec-contracts",
            Severity::Must,
            "API/Data Model",
            check_spec_contracts,
        ),
        rule(
            "spec-related",
            Severity::Must,
            "Related Artifacts",
            check_spec_related,
        ),
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
        rule(
            "rfc-summary",
            Severity::Must,
            "Summary section",
            check_rfc_summary,
        ),
        rule(
            "rfc-motivation",
            Severity::Must,
            "Motivation section",
            check_rfc_motivation,
        ),
        rule(
            "rfc-options",
            Severity::Should,
            "Options Considered",
            check_rfc_options,
        ),
        rule(
            "rfc-proposed",
            Severity::Must,
            "Proposed Direction",
            check_rfc_proposed,
        ),
        rule(
            "rfc-phases",
            Severity::Should,
            "Implementation Phases",
            check_rfc_phases,
        ),
    ];

    if matches!(depth, Mode::Deep) {
        rules.push(rule(
            "rfc-risks",
            Severity::Must,
            "Risks section",
            check_rfc_risks,
        ));
    }

    // Decision Contract rules for RFC (Could — RFC is a proposal, not a decision)
    rules.push(rule(
        "rfc-invariants",
        Severity::Could,
        "RFC could define invariants",
        check_adr_invariants,
    ));
    rules.push(rule(
        "rfc-rollback",
        Severity::Could,
        "RFC could define rollback plan",
        check_adr_rollback,
    ));

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
    if !checks::section_exists(body, "Proposed")
        && !checks::section_exists(body, "Direction")
        && !checks::section_exists(body, "Architecture")
    {
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
        rule(
            "adr-context",
            Severity::Must,
            "Context section",
            check_adr_context,
        ),
        rule(
            "adr-decision",
            Severity::Must,
            "Decision section",
            check_adr_decision,
        ),
        rule(
            "adr-consequences",
            Severity::Must,
            "Consequences",
            check_adr_consequences,
        ),
    ];

    // Decision Contract rules — severity scales with depth
    let (inv_sev, roll_sev) = match depth {
        Mode::Deep => (Severity::Must, Severity::Must),
        _ => (Severity::Should, Severity::Should),
    };

    rules.push(rule(
        "adr-invariants",
        inv_sev,
        "Invariants — what must never be violated",
        check_adr_invariants,
    ));
    rules.push(rule(
        "adr-rollback",
        roll_sev,
        "Rollback plan — what to do if decision fails",
        check_adr_rollback,
    ));
    rules.push(rule(
        "adr-preconditions",
        Severity::Could,
        "Preconditions — what must be true before implementing",
        check_adr_preconditions,
    ));
    rules.push(rule(
        "adr-postconditions",
        Severity::Could,
        "Postconditions — what must be true after implementing",
        check_adr_postconditions,
    ));
    rules.push(rule(
        "adr-affected-files",
        Severity::Should,
        "Affected files/modules — scope of the decision",
        check_adr_affected_files,
    ));

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
    if checks::section_exists(body, "Invariants") {
        None
    } else {
        Some(
            "Missing '## Invariants' section — what must NEVER be violated by this decision".into(),
        )
    }
}

fn check_adr_rollback(body: &str, _fm: &Frontmatter) -> Option<String> {
    if checks::section_exists(body, "Rollback")
        || checks::section_exists(body, "Rollback Plan")
        || checks::section_exists(body, "Mitigation")
    {
        None
    } else {
        Some("Missing '## Rollback Plan' section — what to do if this decision fails".into())
    }
}

fn check_adr_preconditions(body: &str, _fm: &Frontmatter) -> Option<String> {
    if checks::section_exists(body, "Preconditions")
        || checks::section_exists(body, "Pre-conditions")
        || checks::section_exists(body, "Prerequisites")
    {
        None
    } else {
        Some("Missing '## Preconditions' section".into())
    }
}

fn check_adr_postconditions(body: &str, _fm: &Frontmatter) -> Option<String> {
    if checks::section_exists(body, "Postconditions")
        || checks::section_exists(body, "Post-conditions")
        || checks::section_exists(body, "Expected Outcome")
    {
        None
    } else {
        Some("Missing '## Postconditions' section".into())
    }
}

fn check_adr_affected_files(body: &str, _fm: &Frontmatter) -> Option<String> {
    if checks::section_exists(body, "Affected Files")
        || checks::section_exists(body, "Affected Scope")
        || checks::section_exists(body, "Scope")
    {
        None
    } else {
        Some(
            "Missing '## Affected Files' section — which files/modules does this decision affect"
                .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{Severity, validate};

    fn make_fm(id: &str, status: &str) -> Frontmatter {
        let mut fm = Frontmatter::new();
        fm.insert("id".into(), serde_yml::Value::String(id.into()));
        fm.insert("status".into(), serde_yml::Value::String(status.into()));
        fm
    }

    // ─── 1. rules_for returns correct count per kind ────────────────────────

    #[test]
    fn rules_for_prd_tactical_returns_base_only() {
        let rules = rules_for(&ArtifactKind::Prd, &Mode::Tactical);
        let base_count = base_rules().len();
        let prd_base = 5; // problem, goals, non-goals, fr, related
        let fr_format = 1; // fr-format check (all depths)
        let measurability = 2; // adjectives + vague quantifiers (all depths)
        let density_detection = 2; // filler-phrases + density-score (all depths)
        let traceability = 2; // orphan-frs + orphan-goals (all depths)
        let classification = 2; // domain-sections + project-type-sections (all depths)
        assert_eq!(
            rules.len(),
            base_count
                + prd_base
                + fr_format
                + measurability
                + density_detection
                + traceability
                + classification
        );
    }

    #[test]
    fn rules_for_prd_standard_includes_audience_density_leakage() {
        let rules = rules_for(&ArtifactKind::Prd, &Mode::Standard);
        let base_count = base_rules().len();
        let prd_base = 5;
        let standard_extra = 3; // density, audience, leakage
        let fr_format = 1;
        let measurability = 2; // adjectives + vague quantifiers
        let density_detection = 2; // filler-phrases + density-score
        let traceability = 2; // orphan-frs + orphan-goals
        let classification = 2; // domain-sections + project-type-sections
        assert_eq!(
            rules.len(),
            base_count
                + prd_base
                + standard_extra
                + fr_format
                + measurability
                + density_detection
                + traceability
                + classification
        );

        let ids: Vec<&str> = rules.iter().map(|(id, _, _, _)| *id).collect();
        assert!(ids.contains(&"prd-problem-density"));
        assert!(ids.contains(&"prd-target-audience"));
        assert!(ids.contains(&"prd-no-impl-leakage"));
    }

    #[test]
    fn rules_for_prd_deep_includes_timeline_stakeholders_acceptance() {
        let rules = rules_for(&ArtifactKind::Prd, &Mode::Deep);
        let base_count = base_rules().len();
        let prd_base = 5;
        let standard_extra = 3;
        let deep_extra = 7; // timeline, stakeholders, acceptance, risk, rollback, success_metrics, dependencies
        let fr_format = 1;
        let measurability = 2; // adjectives + vague quantifiers
        let density_detection = 2; // filler-phrases + density-score
        let traceability = 2; // orphan-frs + orphan-goals
        let classification = 2; // domain-sections + project-type-sections
        assert_eq!(
            rules.len(),
            base_count
                + prd_base
                + standard_extra
                + deep_extra
                + fr_format
                + measurability
                + density_detection
                + traceability
                + classification
        );

        let ids: Vec<&str> = rules.iter().map(|(id, _, _, _)| *id).collect();
        assert!(ids.contains(&"prd-timeline"));
        assert!(ids.contains(&"prd-stakeholders"));
        assert!(ids.contains(&"prd-acceptance"));
    }

    #[test]
    fn rules_for_epic_returns_base_plus_5() {
        let rules = rules_for(&ArtifactKind::Epic, &Mode::Standard);
        let base_count = base_rules().len();
        assert_eq!(rules.len(), base_count + 5);

        let ids: Vec<&str> = rules.iter().map(|(id, _, _, _)| *id).collect();
        assert!(ids.contains(&"epic-vision"));
        assert!(ids.contains(&"epic-outcomes"));
        assert!(ids.contains(&"epic-children"));
        assert!(ids.contains(&"epic-phases"));
        assert!(ids.contains(&"epic-progress"));
    }

    #[test]
    fn rules_for_spec_returns_base_plus_3() {
        let rules = rules_for(&ArtifactKind::Spec, &Mode::Standard);
        let base_count = base_rules().len();
        assert_eq!(rules.len(), base_count + 3);

        let ids: Vec<&str> = rules.iter().map(|(id, _, _, _)| *id).collect();
        assert!(ids.contains(&"spec-summary"));
        assert!(ids.contains(&"spec-contracts"));
        assert!(ids.contains(&"spec-related"));
    }

    #[test]
    fn rules_for_rfc_standard_returns_base_plus_7_with_contracts() {
        let rules = rules_for(&ArtifactKind::Rfc, &Mode::Standard);
        let base_count = base_rules().len();
        // 5 base RFC + 2 contract rules (invariants, rollback)
        assert_eq!(rules.len(), base_count + 7);

        let ids: Vec<&str> = rules.iter().map(|(id, _, _, _)| *id).collect();
        assert!(!ids.contains(&"rfc-risks"));
        assert!(ids.contains(&"rfc-invariants"));
        assert!(ids.contains(&"rfc-rollback"));

        // RFC contract rules are Could severity
        let inv = rules
            .iter()
            .find(|(id, _, _, _)| *id == "rfc-invariants")
            .unwrap();
        assert_eq!(inv.1, Severity::Could);
    }

    #[test]
    fn rules_for_rfc_deep_returns_base_plus_8_with_risks_and_contracts() {
        let rules = rules_for(&ArtifactKind::Rfc, &Mode::Deep);
        let base_count = base_rules().len();
        // 5 base RFC + 1 risks + 2 contract rules
        assert_eq!(rules.len(), base_count + 8);

        let ids: Vec<&str> = rules.iter().map(|(id, _, _, _)| *id).collect();
        assert!(ids.contains(&"rfc-risks"));
        assert!(ids.contains(&"rfc-invariants"));
        assert!(ids.contains(&"rfc-rollback"));
    }

    #[test]
    fn rules_for_adr_standard_returns_base_plus_8_with_contracts() {
        let rules = rules_for(&ArtifactKind::Adr, &Mode::Standard);
        let base_count = base_rules().len();
        // 3 base ADR + 5 contract rules (invariants, rollback, preconditions, postconditions, affected-files)
        assert_eq!(rules.len(), base_count + 8);

        let ids: Vec<&str> = rules.iter().map(|(id, _, _, _)| *id).collect();
        assert!(ids.contains(&"adr-invariants"));
        assert!(ids.contains(&"adr-rollback"));
        assert!(ids.contains(&"adr-preconditions"));
        assert!(ids.contains(&"adr-postconditions"));
        assert!(ids.contains(&"adr-affected-files"));

        // At standard depth, invariants and rollback are Should (not Must)
        let inv = rules
            .iter()
            .find(|(id, _, _, _)| *id == "adr-invariants")
            .unwrap();
        assert_eq!(inv.1, Severity::Should);
    }

    #[test]
    fn rules_for_adr_deep_returns_base_plus_8_with_must_contracts() {
        let rules = rules_for(&ArtifactKind::Adr, &Mode::Deep);
        let base_count = base_rules().len();
        assert_eq!(rules.len(), base_count + 8);

        let ids: Vec<&str> = rules.iter().map(|(id, _, _, _)| *id).collect();
        assert!(ids.contains(&"adr-invariants"));
        assert!(ids.contains(&"adr-rollback"));

        // At deep depth, invariants and rollback are Must
        let inv = rules
            .iter()
            .find(|(id, _, _, _)| *id == "adr-invariants")
            .unwrap();
        assert_eq!(inv.1, Severity::Must);
        let roll = rules
            .iter()
            .find(|(id, _, _, _)| *id == "adr-rollback")
            .unwrap();
        assert_eq!(roll.1, Severity::Must);
    }

    #[test]
    fn rules_for_note_returns_base_only() {
        let rules = rules_for(&ArtifactKind::Note, &Mode::Tactical);
        let base_count = base_rules().len();
        assert_eq!(rules.len(), base_count);
    }

    // ─── 2. PRD validation on complete document ─────────────────────────────

    #[test]
    fn prd_complete_document_passes_deep_validation() {
        let fm = make_fm("prd-001", "draft");

        // Problem section with >= 50 words
        let problem_text = "This is a significant problem that affects many users \
            across the platform. The current workflow is inefficient and error-prone, \
            leading to frequent mistakes and wasted time. Users have reported frustration \
            with the existing process, and our metrics show a decline in engagement. \
            We need a comprehensive solution that addresses the root causes and provides \
            a streamlined experience for all user segments.";

        let body = format!(
            "## Problem\n\n{problem_text}\n\n\
             ## Goals\n\nImprove user satisfaction by 20%.\n\n\
             ## Non-Goals\n\nWe will not rebuild the entire platform.\n\n\
             ## Functional Requirements\n\n- [Actor] can [capability]\n\n\
             ## Related Artifacts\n\n- RFC-001\n\n\
             ## Target Users\n\nDevelopers and project managers.\n\n\
             ## Timeline\n\nQ1 2026.\n\n\
             ## Stakeholders\n\n- Engineering lead\n- Product manager\n\n\
             ## Acceptance Criteria\n\n- All FR implemented\n- Tests pass\n\n\
             ## Risk Assessment\n\n- Migration may break existing integrations\n\n\
             ## Dependencies\n\n- Auth service must be deployed first\n\n\
             ## Success Metrics\n\n- 20% improvement in task completion rate\n"
        );

        let result = validate("prd-001", &body, &fm, &ArtifactKind::Prd, &Mode::Deep);
        let must_findings: Vec<_> = result
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Must)
            .collect();
        assert!(
            must_findings.is_empty(),
            "Expected 0 Must findings on complete PRD, got {}: {:?}",
            must_findings.len(),
            must_findings.iter().map(|f| &f.rule_id).collect::<Vec<_>>()
        );
        assert!(result.passed());
    }

    // ─── 3. PRD validation on incomplete document ───────────────────────────

    #[test]
    fn prd_incomplete_document_has_multiple_must_findings() {
        let fm = make_fm("prd-002", "draft");
        let body = "## Problem\n\nShort.";

        let result = validate("prd-002", body, &fm, &ArtifactKind::Prd, &Mode::Deep);
        let must_count = result.error_count();

        // Missing: Goals, Non-Goals, FR, Related, Audience, Timeline, Stakeholders,
        //          Acceptance, density < 50
        assert!(
            must_count >= 5,
            "Expected at least 5 Must findings on incomplete PRD, got {}",
            must_count
        );
        assert!(!result.passed());
    }

    // ─── 4. Density check fires on missing/short section ────────────────────

    #[test]
    fn density_check_fires_when_no_problem_section() {
        let fm = make_fm("prd-003", "draft");
        let body = "## Goals\n\nSome goals here.\n";

        let result = check_prd_density(body, &fm);
        assert!(
            result.is_some(),
            "Density check should fire when Problem section is missing"
        );
        let msg = result.unwrap();
        assert!(msg.contains("0 words") || msg.contains("words"));
    }

    #[test]
    fn density_check_fires_when_problem_section_too_short() {
        let fm = make_fm("prd-004", "draft");
        let body = "## Problem\n\nToo short.\n\n## Goals\n\nGoals here.\n";

        let result = check_prd_density(body, &fm);
        assert!(
            result.is_some(),
            "Density check should fire when Problem < 50 words"
        );
    }

    #[test]
    fn density_check_passes_when_problem_section_long_enough() {
        let fm = make_fm("prd-005", "draft");
        let long_problem = (0..60)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let body = format!("## Problem\n\n{long_problem}\n\n## Goals\n\nGoals.\n");

        let result = check_prd_density(&body, &fm);
        assert!(
            result.is_none(),
            "Density check should pass when Problem >= 50 words"
        );
    }

    // ─── 5. ADR deep rules include DDR fields ───────────────────────────────

    #[test]
    fn adr_standard_passes_with_context_decision_consequences() {
        let fm = make_fm("adr-001", "active");
        let body = "## Context\n\nWe need to choose a database.\n\n\
                     ## Decision\n\nUse LanceDB for embedded vector storage.\n\n\
                     ## Consequences\n\nSimplifies deployment, limits horizontal scaling.\n";

        let result = validate("adr-001", body, &fm, &ArtifactKind::Adr, &Mode::Standard);
        assert!(
            result.passed(),
            "ADR with Context+Decision+Consequences should pass at Standard depth, findings: {:?}",
            result
                .findings
                .iter()
                .map(|f| &f.rule_id)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn adr_deep_has_must_findings_for_invariants_and_rollback() {
        let fm = make_fm("adr-002", "active");
        let body = "## Context\n\nWe need to choose a database.\n\n\
                     ## Decision\n\nUse LanceDB for embedded vector storage.\n\n\
                     ## Consequences\n\nSimplifies deployment, limits horizontal scaling.\n";

        let result = validate("adr-002", body, &fm, &ArtifactKind::Adr, &Mode::Deep);

        // Should NOT pass — invariants and rollback are Must at Deep depth
        assert!(
            !result.passed(),
            "ADR should fail at Deep depth without Invariants/Rollback"
        );

        let must_ids: Vec<&str> = result
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Must)
            .map(|f| f.rule_id.as_str())
            .collect();
        assert!(
            must_ids.contains(&"adr-invariants"),
            "Expected Must finding for adr-invariants at Deep depth"
        );
        assert!(
            must_ids.contains(&"adr-rollback"),
            "Expected Must finding for adr-rollback at Deep depth"
        );
    }

    #[test]
    fn adr_deep_passes_with_full_contract() {
        let fm = make_fm("adr-003", "active");
        let body = "## Context\n\nWe need to choose a database.\n\n\
                     ## Decision\n\nUse LanceDB for embedded vector storage.\n\n\
                     ## Consequences\n\nSimplifies deployment, limits horizontal scaling.\n\n\
                     ## Invariants\n\n- Single-file deployment must be preserved.\n\n\
                     ## Rollback Plan\n\n- Revert to SQLite.\n\n\
                     ## Affected Files\n\n- crates/forgeplan-core/src/db/\n";

        let result = validate("adr-003", body, &fm, &ArtifactKind::Adr, &Mode::Deep);
        assert!(
            result.passed(),
            "ADR with full contract should pass at Deep depth, findings: {:?}",
            result
                .findings
                .iter()
                .map(|f| (&f.rule_id, &f.severity))
                .collect::<Vec<_>>()
        );
    }

    // ─── 6. Base rules - no-placeholders ────────────────────────────────────

    #[test]
    fn no_placeholders_fires_on_mustache_placeholder() {
        let fm = make_fm("test-001", "draft");
        let body = "## Summary\n\nThis has a {{placeholder}} in it.\n";

        let result = check_no_placeholders(body, &fm);
        assert!(result.is_some(), "Should detect {{placeholder}}");
    }

    #[test]
    fn no_placeholders_fires_on_todo() {
        let fm = make_fm("test-002", "draft");
        let body = "## Summary\n\nTODO fill this section.\n";

        let result = check_no_placeholders(body, &fm);
        assert!(result.is_some(), "Should detect TODO");
    }

    #[test]
    fn no_placeholders_ignores_todo_inside_code_fence() {
        let fm = make_fm("test-003", "draft");
        let body =
            "## Summary\n\nSome text.\n\n```rust\n// TODO: implement later\n```\n\nMore text.\n";

        let result = check_no_placeholders(body, &fm);
        assert!(result.is_none(), "Should NOT detect TODO inside code fence");
    }

    #[test]
    fn no_placeholders_passes_on_clean_body() {
        let fm = make_fm("test-004", "draft");
        let body = "## Summary\n\nThis is a clean document with no issues.\n";

        let result = check_no_placeholders(body, &fm);
        assert!(
            result.is_none(),
            "Clean body should pass no-placeholders check"
        );
    }
}
