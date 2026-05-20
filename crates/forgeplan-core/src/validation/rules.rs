use crate::artifact::frontmatter::Frontmatter;
use crate::artifact::types::{ArtifactKind, Mode};
use crate::validation::Severity;
use crate::validation::checks;
use std::sync::LazyLock;

/// Module-level compiled regex for `{placeholder}` detection in [`check_stub`].
static PLACEHOLDER_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?:^|[^{])\{([a-zA-Z_][a-zA-Z0-9_ \-]*)\}(?:[^}]|$)")
        .expect("valid placeholder regex")
});

/// Typed result of a stub detection check.
#[derive(Debug, Clone)]
pub struct StubReport {
    pub count: usize,
    pub message: String,
}

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
        // ── Issue #287 Phase B — brownfield extraction kinds ──────────────
        // Each kind gets:
        //   1. per-kind required-section block (stable rule_id
        //      `<kind>-required-sections` so W2 coverage can surface it)
        //   2. tier-level rule (factum-must-have-source OR
        //      intent-must-have-verification-plan) applied via the
        //      `Frontmatter`'s `tier:` field with kind-default fallback.
        ArtifactKind::UseCase => {
            rules.extend(use_case_rules());
            rules.extend(tier_rules_for(kind));
        }
        ArtifactKind::Glossary => {
            rules.extend(glossary_rules());
            rules.extend(tier_rules_for(kind));
        }
        ArtifactKind::Invariant => {
            rules.extend(invariant_rules());
            rules.extend(tier_rules_for(kind));
        }
        ArtifactKind::Scenario => {
            rules.extend(scenario_rules());
            rules.extend(tier_rules_for(kind));
        }
        ArtifactKind::Hypothesis => {
            rules.extend(hypothesis_rules());
            rules.extend(tier_rules_for(kind));
        }
        ArtifactKind::DomainModel => {
            rules.extend(domain_model_rules());
            rules.extend(tier_rules_for(kind));
        }
        _ => {} // Quint-code types (Note, Problem, Solution, Evidence,
                // Refresh, Memory): base rules only.
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
        rule(
            "no-stub-content",
            Severity::Should,
            "Body must not be unfilled template",
            check_stub,
        ),
        // PROB-059 closure — flag body↔links drift across all artifact kinds.
        // SHOULD-level (not MUST) so existing artifacts с incidental drift
        // pass validation but get visible warning. Promote к --strict mode
        // в follow-up if user wants CI-fail behavior.
        rule(
            "body-links-drift",
            Severity::Should,
            "Body's `## Related Artifacts` table consistent с frontmatter `links:`",
            check_body_links_drift,
        ),
    ]
}

/// Detect template-only artifact bodies (PRD-043 FR-003).
///
/// Returns `Some(message)` when 3 or more template markers are found in `body`.
/// Markers include known Russian template phrases, generic placeholder syntax,
/// `[Actor] can [capability]` form, and 3+ consecutive section bodies that are
/// just `...`.
pub fn check_stub(body: &str, fm: &Frontmatter) -> Option<String> {
    check_stub_detailed(body, fm).map(|r| r.message)
}

/// Same as [`check_stub`] but returns a typed [`StubReport`] with marker count.
pub fn check_stub_detailed(body: &str, _fm: &Frontmatter) -> Option<StubReport> {
    const PHRASE_MARKERS: &[&str] = &[
        // Russian
        "Что мы строим и почему это важно",
        "Как проблема влияет на пользователей",
        "Что входит в минимально жизнеспособный продукт",
        "Чем наше решение отличается",
        // English
        "What we are building and why",
        "How the problem affects users",
        "What's in the MVP",
        "How our solution is different",
        "What we're building",
        "Vision: What we're building",
        // Universal
        "[Actor] can [capability]",
        "<Actor> can <capability>",
    ];

    let mut count = 0usize;

    for marker in PHRASE_MARKERS {
        if body.contains(marker) {
            count += 1;
        }
    }

    // {placeholder} markers — single-brace curly placeholders like {name}.
    // Avoid false-positives on `{{var}}` (already covered by no-placeholders)
    // and on JSON/code by requiring word characters only inside the braces.
    if PLACEHOLDER_RE.is_match(body) {
        count += 1;
    }

    // 3+ consecutive section bodies that are just "..."
    // A section body is the content between two `## ` headings (or end of file).
    let mut consecutive_dots = 0usize;
    let mut max_consecutive_dots = 0usize;
    let mut current_section: Vec<&str> = Vec::new();

    let flush = |section: &[&str], consecutive: &mut usize, max: &mut usize| {
        let content: String = section
            .iter()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if content == "..." {
            *consecutive += 1;
            if *consecutive > *max {
                *max = *consecutive;
            }
        } else if !content.is_empty() {
            *consecutive = 0;
        }
    };

    for line in body.lines() {
        if line.starts_with("## ") {
            flush(
                &current_section,
                &mut consecutive_dots,
                &mut max_consecutive_dots,
            );
            current_section.clear();
        } else {
            current_section.push(line);
        }
    }
    flush(
        &current_section,
        &mut consecutive_dots,
        &mut max_consecutive_dots,
    );

    if max_consecutive_dots >= 3 {
        count += 1;
    }

    if count >= 3 {
        Some(StubReport {
            count,
            message: format!("Body appears to be unfilled template ({count} markers found)"),
        })
    } else {
        None
    }
}

fn check_meta_id(_body: &str, fm: &Frontmatter) -> Option<String> {
    if !checks::frontmatter_has(fm, "id") {
        Some("Missing 'id' field in frontmatter".into())
    } else {
        None
    }
}

/// PROB-059 — body↔links drift detection.
///
/// Compares IDs mentioned в `## Related Artifacts` table rows against
/// frontmatter `links:` array. If body table mentions an ID that has no
/// corresponding `links:` entry → SHOULD-level warning. Self-references
/// и table-row IDs that are the artifact's own id are ignored.
///
/// Implementation strategy (Option A from PROB-059): strict parser
/// targeting only the `## Related Artifacts` section. Free-text "see
/// also PRD-005" mentions elsewhere в body are intentionally NOT
/// flagged — incidental mentions shouldn't trigger drift warnings.
fn check_body_links_drift(body: &str, fm: &Frontmatter) -> Option<String> {
    let body_ids = checks::extract_related_artifacts_table_ids(body);
    if body_ids.is_empty() {
        return None;
    }
    let link_targets: std::collections::BTreeSet<String> =
        checks::extract_frontmatter_link_targets(fm)
            .into_iter()
            .map(|s| s.to_uppercase())
            .collect();
    let self_id = fm
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_uppercase())
        .unwrap_or_default();
    let mut missing: Vec<String> = Vec::new();
    for id in &body_ids {
        let id_upper = id.to_uppercase();
        if id_upper == self_id {
            continue;
        }
        if !link_targets.contains(&id_upper) {
            missing.push(id.clone());
        }
    }
    if missing.is_empty() {
        None
    } else {
        Some(format!(
            "Body's `## Related Artifacts` table mentions {} but frontmatter `links:` array doesn't reference \
             them. Run: forgeplan link <this-id> <target> --relation <informs|based_on|refines|...> \
             OR remove the table row if the mention is incidental.",
            missing.join(", ")
        ))
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
        Some(serde_yaml::Value::String(s)) if !s.trim().is_empty() => s.trim().to_string(),
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
        Some(serde_yaml::Value::String(s)) if !s.trim().is_empty() => s.trim().to_string(),
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

// ─── Issue #287 Phase B — Brownfield validation ─────────────────────────────
//
// Two-tier discipline:
//   • Factum tier (UseCase, Glossary, Invariant, Scenario, DomainModel)
//     MUST cite its `## Source` so the artifact remains traceable to
//     the codebase it was extracted from. Stable rule_id:
//     `factum-must-have-source`.
//   • Intent tier (Hypothesis) MUST publish a falsifiability plan —
//     `## How To Verify`, `## Evidence For`, `## Evidence Against` all
//     present. Stable rule_id: `intent-must-have-verification-plan`.
//
// Per-kind required-section lists (public constants below) double as
// W2's source of truth for "expected_sections" in coverage reporting.
// Mutation of these lists is a breaking contract change — update W2 too.
//
// FPF: chose public `&'static [&'static str]` over a method on
// `ArtifactKind` to keep the validation module the single owner of
// section policy and avoid pulling `ArtifactKind` into a circular
// dependency for downstream consumers that want the metadata without
// importing the entire validation surface.

/// Required `##` headings for a UseCase factum artifact.
///
/// W2 imports this to compute `expected_sections` totals in coverage.
pub const REQUIRED_SECTIONS_USE_CASE: &[&str] = &[
    "Problem",
    "Actor",
    "Main Flow",
    "Acceptance Criteria",
    "Source",
];

/// Required `##` headings for a Glossary factum artifact.
pub const REQUIRED_SECTIONS_GLOSSARY: &[&str] = &["Canonical Term", "Definition", "Source"];

/// Required `##` headings for an Invariant factum artifact.
pub const REQUIRED_SECTIONS_INVARIANT: &[&str] =
    &["Statement", "Scope", "Rationale", "Verification", "Source"];

/// Required `##` headings for a Scenario factum artifact.
pub const REQUIRED_SECTIONS_SCENARIO: &[&str] =
    &["Given", "When", "Then", "Demonstrates", "Source"];

/// Required `##` headings for a Hypothesis intent artifact.
pub const REQUIRED_SECTIONS_HYPOTHESIS: &[&str] = &[
    "Hypothesis",
    "Lifecycle State",
    "Evidence For",
    "Evidence Against",
    "How To Verify",
    "Source",
];

/// Required `##` headings for a DomainModel factum artifact.
pub const REQUIRED_SECTIONS_DOMAIN_MODEL: &[&str] = &["Domain", "Composition", "Source"];

/// Minimum word count for a Factum's `## Source` section. Below this
/// the citation is treated as a stub (cannot legitimately point at
/// codebase / docs in <10 words).
const FACTUM_SOURCE_MIN_WORDS: usize = 10;

/// Default tier per artifact kind when frontmatter's `tier:` field is
/// missing or unrecognised.
///
/// Phase A templates pre-set `tier: factum` for UseCase, Glossary,
/// Invariant, Scenario, DomainModel and `tier: intent` for Hypothesis.
/// This helper exists so downstream callers (W2 coverage) can infer
/// the tier without re-parsing every template.
///
/// Returns `"factum"` for non-brownfield kinds — conservative default,
/// downstream consumers should check `ArtifactKind::is_brownfield()`
/// first if they care.
pub fn tier_for_kind(kind: &ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Hypothesis => "intent",
        // All other brownfield kinds + non-brownfield kinds default to
        // factum (the more conservative "must cite source" interpretation).
        _ => "factum",
    }
}

/// Extract the artifact's tier.
///
/// Audit-r4 SEC-C1 closure: tier is determined exclusively by
/// [`ArtifactKind`]. The frontmatter `tier:` field is informational only
/// (template default for human readers) — it CANNOT override enforcement.
/// Earlier revisions trusted the frontmatter, which let an operator flip
/// a Hypothesis to `tier: factum` and silently skip the
/// falsifiability-plan gate, or flip an Invariant to `tier: intent`
/// and skip the `## Source` citation gate.
///
/// `fm` is kept in the signature for forward-compatibility (e.g. if a
/// future tier dispatch needs additional metadata) but is unused today.
fn extract_tier(fm: &Frontmatter, kind: &ArtifactKind) -> &'static str {
    let _ = fm; // intentionally ignored — tier is kind-derived (SEC-C1)
    tier_for_kind(kind)
}

/// Tier-level rules for a brownfield artifact. Returned set depends on
/// the artifact's tier (resolved via [`extract_tier`] at check time, not
/// dispatch time — frontmatter overrides default).
///
/// FPF: chose to register BOTH potential tier-rules and let the inner
/// check inspect frontmatter, rather than splitting dispatch by tier.
/// Reason — `rules_for()` doesn't currently receive frontmatter, and
/// threading it would balloon the signature for every existing kind.
/// The per-check `extract_tier` call is O(1) lookup in a BTreeMap.
fn tier_rules_for(kind: &ArtifactKind) -> Vec<RuleEntry> {
    // Both rules registered; each check short-circuits to None when
    // the artifact's effective tier is not its own.
    let _ = kind; // suppress unused; kept for symmetry / future per-kind tier tweaks.
    vec![
        rule(
            "factum-must-have-source",
            Severity::Must,
            "Factum: '## Source' citation",
            check_factum_source,
        ),
        rule(
            "intent-must-have-verification-plan",
            Severity::Must,
            "Intent: '## How To Verify' + Evidence For/Against",
            check_intent_verification_plan,
        ),
    ]
}

/// Resolve the artifact's kind from frontmatter (`kind:` field) so a
/// tier-rule can know which default to apply. Falls back to factum-mode
/// when the kind is missing or unrecognised — that mirrors the
/// conservative default and lets a malformed artifact still get the
/// "must cite source" nag.
fn fm_kind_or_factum_default(fm: &Frontmatter) -> ArtifactKind {
    fm.get("kind")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<ArtifactKind>().ok())
        // UseCase is an arbitrary factum representative for the
        // fallback path; the tier rule only branches on tier value,
        // not the specific kind.
        .unwrap_or(ArtifactKind::UseCase)
}

/// Factum tier MUST cite its `## Source`.
///
/// Fires only when the effective tier is `factum`. Empty / placeholder
/// citations (< [`FACTUM_SOURCE_MIN_WORDS`] words) are flagged as if
/// the section were missing — a one-word stub like "TBD" does not
/// fulfil the contract.
fn check_factum_source(body: &str, fm: &Frontmatter) -> Option<String> {
    let kind = fm_kind_or_factum_default(fm);
    if extract_tier(fm, &kind) != "factum" {
        return None;
    }
    if !checks::section_exists(body, "Source") {
        return Some(
            "Factum artifact must have a '## Source' section citing the codebase or document \
             where this fact was extracted from."
                .into(),
        );
    }
    let wc = checks::section_word_count(body, "Source");
    if wc < FACTUM_SOURCE_MIN_WORDS {
        return Some(format!(
            "Factum '## Source' citation is too short ({} words, expected >= {}). \
             Cite the codebase path, doc URL, or interview reference.",
            wc, FACTUM_SOURCE_MIN_WORDS
        ));
    }
    None
}

/// Intent tier MUST publish a falsifiability plan.
///
/// Fires only when the effective tier is `intent`. All three sections
/// must be present — a hypothesis without a verification plan is
/// indistinguishable from a guess.
fn check_intent_verification_plan(body: &str, fm: &Frontmatter) -> Option<String> {
    let kind = fm_kind_or_factum_default(fm);
    if extract_tier(fm, &kind) != "intent" {
        return None;
    }
    let mut missing: Vec<&str> = Vec::new();
    if !checks::section_exists(body, "How To Verify") {
        missing.push("How To Verify");
    }
    if !checks::section_exists(body, "Evidence For") {
        missing.push("Evidence For");
    }
    if !checks::section_exists(body, "Evidence Against") {
        missing.push("Evidence Against");
    }
    if missing.is_empty() {
        None
    } else {
        Some(format!(
            "Intent (hypothesis) must publish a falsifiability plan. Missing: {}.",
            missing
                .iter()
                .map(|s| format!("'## {s}'"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

// ─── Per-kind required-section rules ────────────────────────────────────────
//
// FPF: chose ONE rule_id per kind (e.g. `use-case-required-sections`) over
// one-rule-per-section. Reason — for coverage reporting (W2) it's the
// kind-level pass/fail that matters; granular rule_ids would multiply
// without giving the reader more actionable info than the per-finding
// message (which already lists exactly which headings are missing).

fn use_case_rules() -> Vec<RuleEntry> {
    vec![rule(
        "use-case-required-sections",
        Severity::Must,
        "UseCase required sections",
        check_use_case_sections,
    )]
}

fn check_use_case_sections(body: &str, _fm: &Frontmatter) -> Option<String> {
    check_required_sections(body, REQUIRED_SECTIONS_USE_CASE, "UseCase")
}

fn glossary_rules() -> Vec<RuleEntry> {
    vec![rule(
        "glossary-required-sections",
        Severity::Must,
        "Glossary required sections",
        check_glossary_sections,
    )]
}

fn check_glossary_sections(body: &str, _fm: &Frontmatter) -> Option<String> {
    check_required_sections(body, REQUIRED_SECTIONS_GLOSSARY, "Glossary")
}

fn invariant_rules() -> Vec<RuleEntry> {
    vec![rule(
        "invariant-required-sections",
        Severity::Must,
        "Invariant required sections",
        check_invariant_sections,
    )]
}

fn check_invariant_sections(body: &str, _fm: &Frontmatter) -> Option<String> {
    check_required_sections(body, REQUIRED_SECTIONS_INVARIANT, "Invariant")
}

fn scenario_rules() -> Vec<RuleEntry> {
    vec![rule(
        "scenario-required-sections",
        Severity::Must,
        "Scenario required sections",
        check_scenario_sections,
    )]
}

fn check_scenario_sections(body: &str, _fm: &Frontmatter) -> Option<String> {
    check_required_sections(body, REQUIRED_SECTIONS_SCENARIO, "Scenario")
}

fn hypothesis_rules() -> Vec<RuleEntry> {
    vec![rule(
        "hypothesis-required-sections",
        Severity::Must,
        "Hypothesis required sections",
        check_hypothesis_sections,
    )]
}

fn check_hypothesis_sections(body: &str, _fm: &Frontmatter) -> Option<String> {
    check_required_sections(body, REQUIRED_SECTIONS_HYPOTHESIS, "Hypothesis")
}

fn domain_model_rules() -> Vec<RuleEntry> {
    vec![rule(
        "domain-model-required-sections",
        Severity::Must,
        "DomainModel required sections",
        check_domain_model_sections,
    )]
}

fn check_domain_model_sections(body: &str, _fm: &Frontmatter) -> Option<String> {
    check_required_sections(body, REQUIRED_SECTIONS_DOMAIN_MODEL, "DomainModel")
}

/// Generic helper — reports all missing required sections at once so
/// the author sees a single actionable finding instead of N nags.
fn check_required_sections(
    body: &str,
    required: &[&'static str],
    kind_label: &str,
) -> Option<String> {
    let missing: Vec<&'static str> = required
        .iter()
        .copied()
        .filter(|heading| !checks::section_exists(body, heading))
        .collect();
    if missing.is_empty() {
        None
    } else {
        Some(format!(
            "{kind_label} is missing required section(s): {}.",
            missing
                .iter()
                .map(|h| format!("'## {h}'"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{Severity, validate};

    fn make_fm(id: &str, status: &str) -> Frontmatter {
        let mut fm = Frontmatter::new();
        fm.insert("id".into(), serde_yaml::Value::String(id.into()));
        fm.insert("status".into(), serde_yaml::Value::String(status.into()));
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

    // ─── no-stub-content (PRD-043 FR-003) ──────────────────────────────────

    #[test]
    fn test_check_stub_detailed_returns_count() {
        let body = r#"## Problem
Что мы строим и почему это важно
Как проблема влияет на пользователей

## Goals
Что входит в минимально жизнеспособный продукт
"#;
        let fm = make_fm("PRD-001", "draft");
        let report = check_stub_detailed(body, &fm).expect("should be flagged as stub");
        assert!(
            report.count >= 3,
            "count should be >= 3, got {}",
            report.count
        );
        assert!(report.message.contains("unfilled template"));
        assert!(report.message.contains(&report.count.to_string()));
    }

    #[test]
    fn test_check_stub_detects_template() {
        let body = r#"## Problem
Что мы строим и почему это важно
Как проблема влияет на пользователей

## Goals
Что входит в минимально жизнеспособный продукт
Чем наше решение отличается

## FR
- FR-001: [Actor] can [capability]
- FR-002: {placeholder}
"#;
        let fm = make_fm("PRD-001", "draft");
        let result = check_stub(body, &fm);
        assert!(
            result.is_some(),
            "Template-only body must be flagged as stub"
        );
        let msg = result.unwrap();
        assert!(msg.contains("unfilled template"), "msg: {}", msg);
    }

    #[test]
    fn test_check_stub_passes_filled_artifact() {
        let body = r#"## Problem
Users cannot validate artifact bodies for stub content. This causes
methodology drift: empty PRDs leak into the active set without resistance.

## Goals
Detect template-only bodies in the validation pipeline so reviewers see a
SHOULD finding before activation.

## FR
- FR-001: Validator emits a warning when 3+ template markers present.
- FR-002: Threshold is configurable per project.
"#;
        let fm = make_fm("PRD-001", "draft");
        let result = check_stub(body, &fm);
        assert!(
            result.is_none(),
            "Filled artifact body must not be flagged: {:?}",
            result
        );
    }

    #[test]
    fn test_check_stub_at_threshold() {
        let three = r#"## A
Что мы строим и почему это важно
Как проблема влияет на пользователей
Чем наше решение отличается
"#;
        let fm = make_fm("PRD-001", "draft");
        assert!(
            check_stub(three, &fm).is_some(),
            "3 markers must trigger stub detection"
        );

        let two = r#"## A
Что мы строим и почему это важно
Как проблема влияет на пользователей

Real content describing the rest of the story in detail.
"#;
        assert!(
            check_stub(two, &fm).is_none(),
            "2 markers must not trigger stub detection"
        );
    }

    #[test]
    fn test_check_stub_detects_english_template() {
        let body = r#"## Problem
What we are building and why
How the problem affects users

## Goals
What's in the MVP
How our solution is different

## FR
- FR-001: [Actor] can [capability]
"#;
        let fm = make_fm("PRD-001", "draft");
        let result = check_stub(body, &fm);
        assert!(
            result.is_some(),
            "English template body must be flagged as stub"
        );
        let msg = result.unwrap();
        assert!(msg.contains("unfilled template"), "msg: {}", msg);
    }

    #[test]
    fn test_check_stub_consecutive_dots_count_as_marker() {
        let body = r#"## A
Что мы строим и почему это важно

## B
...

## C
...

## D
...

## E
Чем наше решение отличается
"#;
        let fm = make_fm("PRD-001", "draft");
        assert!(
            check_stub(body, &fm).is_some(),
            "3 consecutive '...' sections + 2 phrases must trigger"
        );
    }
}

// ─── Issue #287 Phase B — brownfield validation tests ───────────────────────
//
// Two-tier discipline + per-kind required-sections.
//
// Coverage matrix (14 tests minimum):
//   • 6 kinds × {positive, negative} = 12 required-section tests
//   • 1 factum-tier-missing-source test (UseCase as representative)
//   • 1 intent-tier-missing-verification-plan test (Hypothesis)
//
// Additional tests cover the `extract_tier` fallback, the `tier_for_kind`
// helper, the `Source`-too-short edge case, and the `tier_rules_for`
// dispatch arms (one per kind).
#[cfg(test)]
mod tests_brownfield {
    use super::*;
    use crate::validation::{Severity, validate};

    fn fm_with_kind_and_tier(id: &str, kind: &str, tier: Option<&str>) -> Frontmatter {
        let mut fm = Frontmatter::new();
        fm.insert("id".into(), serde_yaml::Value::String(id.into()));
        fm.insert("status".into(), serde_yaml::Value::String("draft".into()));
        fm.insert("kind".into(), serde_yaml::Value::String(kind.into()));
        if let Some(t) = tier {
            fm.insert("tier".into(), serde_yaml::Value::String(t.into()));
        }
        fm
    }

    fn must_finding<'a>(
        result: &'a crate::validation::ValidationResult,
        rule_id: &str,
    ) -> Option<&'a crate::validation::Finding> {
        result
            .findings
            .iter()
            .find(|f| f.rule_id == rule_id && f.severity == Severity::Must)
    }

    // ─── 1. tier_for_kind helper ────────────────────────────────────────────

    #[test]
    fn tier_for_kind_hypothesis_is_intent() {
        assert_eq!(tier_for_kind(&ArtifactKind::Hypothesis), "intent");
    }

    #[test]
    fn tier_for_kind_other_brownfield_kinds_are_factum() {
        assert_eq!(tier_for_kind(&ArtifactKind::UseCase), "factum");
        assert_eq!(tier_for_kind(&ArtifactKind::Glossary), "factum");
        assert_eq!(tier_for_kind(&ArtifactKind::Invariant), "factum");
        assert_eq!(tier_for_kind(&ArtifactKind::Scenario), "factum");
        assert_eq!(tier_for_kind(&ArtifactKind::DomainModel), "factum");
    }

    // ─── 2. UseCase positive + negative ────────────────────────────────────

    #[test]
    fn use_case_complete_body_passes_required_sections() {
        let fm = fm_with_kind_and_tier("uc-001", "use_case", Some("factum"));
        let body = "\
## Problem\nCustomer needs refund flow.\n\n\
## Actor\nSupport agent.\n\n\
## Main Flow\n1. Open ticket\n2. Issue refund\n\n\
## Acceptance Criteria\n- [ ] Refund completes\n\n\
## Source\nExtracted from `src/refund.rs:42` during 2026-05 audit pass.\n";

        let result = validate("uc-001", body, &fm, &ArtifactKind::UseCase, &Mode::Tactical);
        assert!(
            must_finding(&result, "use-case-required-sections").is_none(),
            "complete UseCase should have no required-sections finding: {:?}",
            result
                .findings
                .iter()
                .map(|f| &f.rule_id)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn use_case_missing_actor_fires_required_sections_finding() {
        let fm = fm_with_kind_and_tier("uc-002", "use_case", Some("factum"));
        // Drop the `## Actor` heading.
        let body = "\
## Problem\nCustomer needs refund flow.\n\n\
## Main Flow\n1. Open ticket\n2. Issue refund\n\n\
## Acceptance Criteria\n- [ ] Refund completes\n\n\
## Source\nExtracted from `src/refund.rs:42` during 2026-05 audit pass.\n";

        let result = validate("uc-002", body, &fm, &ArtifactKind::UseCase, &Mode::Tactical);
        let finding = must_finding(&result, "use-case-required-sections")
            .expect("missing Actor must produce use-case-required-sections finding");
        assert!(finding.message.contains("'## Actor'"));
    }

    // ─── 3. Glossary positive + negative ───────────────────────────────────

    #[test]
    fn glossary_complete_body_passes_required_sections() {
        let fm = fm_with_kind_and_tier("glos-001", "glossary", Some("factum"));
        let body = "\
## Canonical Term\nTenant\n\n\
## Definition\nA single billing account.\n\n\
## Source\nDiscovered in `src/auth/tenant.rs` and onboarding wiki.\n";

        let result = validate(
            "glos-001",
            body,
            &fm,
            &ArtifactKind::Glossary,
            &Mode::Tactical,
        );
        assert!(must_finding(&result, "glossary-required-sections").is_none());
    }

    #[test]
    fn glossary_missing_definition_fires_required_sections_finding() {
        let fm = fm_with_kind_and_tier("glos-002", "glossary", Some("factum"));
        let body = "\
## Canonical Term\nTenant\n\n\
## Source\nDiscovered in `src/auth/tenant.rs` and onboarding wiki.\n";

        let result = validate(
            "glos-002",
            body,
            &fm,
            &ArtifactKind::Glossary,
            &Mode::Tactical,
        );
        let finding = must_finding(&result, "glossary-required-sections")
            .expect("missing Definition must produce glossary-required-sections finding");
        assert!(finding.message.contains("'## Definition'"));
    }

    // ─── 4. Invariant positive + negative ──────────────────────────────────

    #[test]
    fn invariant_complete_body_passes_required_sections() {
        let fm = fm_with_kind_and_tier("inv-001", "invariant", Some("factum"));
        let body = "\
## Statement\nA refund cannot exceed the original payment.\n\n\
## Scope\nPayments domain.\n\n\
## Rationale\nPrevents double-spend.\n\n\
## Verification\nUnit test in `tests/refund_bounds.rs`.\n\n\
## Source\nExtracted from `src/payments/refund.rs` audit 2026-05.\n";

        let result = validate(
            "inv-001",
            body,
            &fm,
            &ArtifactKind::Invariant,
            &Mode::Tactical,
        );
        assert!(must_finding(&result, "invariant-required-sections").is_none());
    }

    #[test]
    fn invariant_missing_rationale_fires_required_sections_finding() {
        let fm = fm_with_kind_and_tier("inv-002", "invariant", Some("factum"));
        let body = "\
## Statement\nA refund cannot exceed the original payment.\n\n\
## Scope\nPayments domain.\n\n\
## Verification\nUnit test in `tests/refund_bounds.rs`.\n\n\
## Source\nExtracted from `src/payments/refund.rs` audit 2026-05.\n";

        let result = validate(
            "inv-002",
            body,
            &fm,
            &ArtifactKind::Invariant,
            &Mode::Tactical,
        );
        let finding = must_finding(&result, "invariant-required-sections")
            .expect("missing Rationale must produce invariant-required-sections finding");
        assert!(finding.message.contains("'## Rationale'"));
    }

    // ─── 5. Scenario positive + negative ───────────────────────────────────

    #[test]
    fn scenario_complete_body_passes_required_sections() {
        let fm = fm_with_kind_and_tier("scen-001", "scenario", Some("factum"));
        let body = "\
## Given\nA payment of $100.\n\n\
## When\nAgent issues refund of $150.\n\n\
## Then\nSystem rejects the refund with code REFUND_EXCEEDS_PAYMENT.\n\n\
## Demonstrates\n- INV-001 (refund ceiling)\n\n\
## Source\nExtracted from incident-2026-04-12 postmortem and `tests/refund_bounds.rs`.\n";

        let result = validate(
            "scen-001",
            body,
            &fm,
            &ArtifactKind::Scenario,
            &Mode::Tactical,
        );
        assert!(must_finding(&result, "scenario-required-sections").is_none());
    }

    #[test]
    fn scenario_missing_then_fires_required_sections_finding() {
        let fm = fm_with_kind_and_tier("scen-002", "scenario", Some("factum"));
        let body = "\
## Given\nA payment of $100.\n\n\
## When\nAgent issues refund of $150.\n\n\
## Demonstrates\n- INV-001 (refund ceiling)\n\n\
## Source\nExtracted from incident-2026-04-12 postmortem and `tests/refund_bounds.rs`.\n";

        let result = validate(
            "scen-002",
            body,
            &fm,
            &ArtifactKind::Scenario,
            &Mode::Tactical,
        );
        let finding = must_finding(&result, "scenario-required-sections")
            .expect("missing Then must produce scenario-required-sections finding");
        assert!(finding.message.contains("'## Then'"));
    }

    // ─── 6. Hypothesis positive + negative ─────────────────────────────────

    #[test]
    fn hypothesis_complete_body_passes_required_sections() {
        let fm = fm_with_kind_and_tier("hyp-001", "hypothesis", Some("intent"));
        let body = "\
## Hypothesis\nRefunds over $1000 require manager approval.\n\n\
## Lifecycle State\n**Current**: inferred\n\n\
## Evidence For\n- `src/refund_policy.rs:88` references `requires_approval` flag.\n\n\
## Evidence Against\n- No unit test covers the path; could be dead code.\n\n\
## How To Verify\nRun a $1500 refund in staging and observe approval queue.\n\n\
## Source\nObserved during code walk on 2026-05-14.\n";

        let result = validate(
            "hyp-001",
            body,
            &fm,
            &ArtifactKind::Hypothesis,
            &Mode::Tactical,
        );
        assert!(must_finding(&result, "hypothesis-required-sections").is_none());
    }

    #[test]
    fn hypothesis_missing_lifecycle_state_fires_required_sections_finding() {
        let fm = fm_with_kind_and_tier("hyp-002", "hypothesis", Some("intent"));
        let body = "\
## Hypothesis\nRefunds over $1000 require manager approval.\n\n\
## Evidence For\n- `src/refund_policy.rs:88` references `requires_approval` flag.\n\n\
## Evidence Against\n- No unit test covers the path; could be dead code.\n\n\
## How To Verify\nRun a $1500 refund in staging and observe approval queue.\n\n\
## Source\nObserved during code walk on 2026-05-14.\n";

        let result = validate(
            "hyp-002",
            body,
            &fm,
            &ArtifactKind::Hypothesis,
            &Mode::Tactical,
        );
        let finding = must_finding(&result, "hypothesis-required-sections")
            .expect("missing Lifecycle State must produce hypothesis-required-sections finding");
        assert!(finding.message.contains("'## Lifecycle State'"));
    }

    // ─── 7. DomainModel positive + negative ────────────────────────────────

    #[test]
    fn domain_model_complete_body_passes_required_sections() {
        let fm = fm_with_kind_and_tier("dm-001", "domain_model", Some("factum"));
        let body = "\
## Domain\nPayments subdomain.\n\n\
## Composition\n- GLOS-001 (Tenant)\n- INV-001 (Refund ceiling)\n\n\
## Source\nAggregated from Discover Agent v3.2 pass on 2026-05-14.\n";

        let result = validate(
            "dm-001",
            body,
            &fm,
            &ArtifactKind::DomainModel,
            &Mode::Tactical,
        );
        assert!(must_finding(&result, "domain-model-required-sections").is_none());
    }

    #[test]
    fn domain_model_missing_composition_fires_required_sections_finding() {
        let fm = fm_with_kind_and_tier("dm-002", "domain_model", Some("factum"));
        let body = "\
## Domain\nPayments subdomain.\n\n\
## Source\nAggregated from Discover Agent v3.2 pass on 2026-05-14.\n";

        let result = validate(
            "dm-002",
            body,
            &fm,
            &ArtifactKind::DomainModel,
            &Mode::Tactical,
        );
        let finding = must_finding(&result, "domain-model-required-sections")
            .expect("missing Composition must produce domain-model-required-sections finding");
        assert!(finding.message.contains("'## Composition'"));
    }

    // ─── 8. Tier-level rules ───────────────────────────────────────────────

    #[test]
    fn factum_without_source_section_fires_factum_must_have_source() {
        // UseCase body with all other required sections except Source.
        let fm = fm_with_kind_and_tier("uc-003", "use_case", Some("factum"));
        let body = "\
## Problem\nCustomer needs refund flow.\n\n\
## Actor\nSupport agent.\n\n\
## Main Flow\n1. Open ticket\n2. Issue refund\n\n\
## Acceptance Criteria\n- [ ] Refund completes\n";

        let result = validate("uc-003", body, &fm, &ArtifactKind::UseCase, &Mode::Tactical);
        // Both the per-kind required-sections rule and the tier rule
        // should fire — Source is in BOTH lists by design (factum-tier
        // = source citation; per-kind required-sections enumerates it
        // explicitly).
        let tier_finding = must_finding(&result, "factum-must-have-source")
            .expect("factum without Source must trigger factum-must-have-source");
        assert!(tier_finding.message.contains("Source"));
    }

    #[test]
    fn factum_with_short_source_section_fires_factum_must_have_source() {
        // `## Source\nTBD\n` — 1 word, below FACTUM_SOURCE_MIN_WORDS.
        let fm = fm_with_kind_and_tier("uc-004", "use_case", Some("factum"));
        let body = "\
## Problem\nCustomer needs refund flow.\n\n\
## Actor\nSupport agent.\n\n\
## Main Flow\n1. Open ticket\n2. Issue refund\n\n\
## Acceptance Criteria\n- [ ] Refund completes\n\n\
## Source\nTBD\n";

        let result = validate("uc-004", body, &fm, &ArtifactKind::UseCase, &Mode::Tactical);
        let finding = must_finding(&result, "factum-must-have-source")
            .expect("Source with <10 words must trigger factum-must-have-source");
        assert!(
            finding.message.contains("too short"),
            "expected 'too short' in: {}",
            finding.message
        );
    }

    #[test]
    fn intent_hypothesis_without_how_to_verify_fires_intent_rule() {
        let fm = fm_with_kind_and_tier("hyp-003", "hypothesis", Some("intent"));
        // Has Hypothesis + Evidence For + Evidence Against + Source, no `## How To Verify`.
        let body = "\
## Hypothesis\nRefunds over $1000 require manager approval.\n\n\
## Lifecycle State\n**Current**: inferred\n\n\
## Evidence For\n- `src/refund_policy.rs:88` references `requires_approval`.\n\n\
## Evidence Against\n- No unit test covers the path.\n\n\
## Source\nObserved during code walk on 2026-05-14.\n";

        let result = validate(
            "hyp-003",
            body,
            &fm,
            &ArtifactKind::Hypothesis,
            &Mode::Tactical,
        );
        let finding = must_finding(&result, "intent-must-have-verification-plan")
            .expect("missing How To Verify must trigger intent-must-have-verification-plan");
        assert!(finding.message.contains("How To Verify"));
    }

    // ─── 9. Tier inference fallback ────────────────────────────────────────

    #[test]
    fn missing_tier_frontmatter_falls_back_to_kind_default_factum() {
        // No `tier:` field — UseCase defaults to factum, so the factum
        // rule must still apply and (correctly) fire when Source is empty.
        let mut fm = Frontmatter::new();
        fm.insert("id".into(), serde_yaml::Value::String("uc-005".into()));
        fm.insert("status".into(), serde_yaml::Value::String("draft".into()));
        fm.insert("kind".into(), serde_yaml::Value::String("use_case".into()));

        let body = "\
## Problem\nCustomer needs refund flow.\n\n\
## Actor\nSupport agent.\n\n\
## Main Flow\n1. Open ticket\n\n\
## Acceptance Criteria\n- [ ] Refund completes\n";
        // No `## Source` heading at all.

        let result = validate("uc-005", body, &fm, &ArtifactKind::UseCase, &Mode::Tactical);
        assert!(
            must_finding(&result, "factum-must-have-source").is_some(),
            "UseCase without tier: in fm should still default to factum and require Source"
        );
    }

    #[test]
    fn missing_tier_frontmatter_falls_back_to_kind_default_intent() {
        // No `tier:` field on a Hypothesis — defaults to intent, so the
        // intent verification plan rule must fire when the plan is missing.
        let mut fm = Frontmatter::new();
        fm.insert("id".into(), serde_yaml::Value::String("hyp-004".into()));
        fm.insert("status".into(), serde_yaml::Value::String("draft".into()));
        fm.insert(
            "kind".into(),
            serde_yaml::Value::String("hypothesis".into()),
        );

        let body = "\
## Hypothesis\nRefunds over $1000 require manager approval.\n\n\
## Lifecycle State\n**Current**: inferred\n\n\
## Source\nObserved during code walk.\n";
        // No `## How To Verify`, no `## Evidence For/Against`.

        let result = validate(
            "hyp-004",
            body,
            &fm,
            &ArtifactKind::Hypothesis,
            &Mode::Tactical,
        );
        assert!(
            must_finding(&result, "intent-must-have-verification-plan").is_some(),
            "Hypothesis without tier: should still default to intent and require verification plan"
        );
    }

    // ─── 9b. SEC-C1 (audit-r4) — frontmatter tier override is ignored ──────

    #[test]
    fn sec_c1_hypothesis_with_factum_override_still_requires_verification_plan() {
        // Audit-r4 SEC-C1: an operator MUST NOT be able to disable the
        // intent-tier falsifiability gate by writing `tier: factum` in a
        // Hypothesis frontmatter. The gate is kind-derived.
        let fm = fm_with_kind_and_tier("hyp-secc1", "hypothesis", Some("factum"));

        let body = "\
## Hypothesis\nRefunds over $1000 require manager approval.\n\n\
## Lifecycle State\n**Current**: inferred\n\n\
## Source\nCode walk in commands/refund.rs.\n";
        // No `## How To Verify` — should still trigger intent rule
        // because Hypothesis is intent regardless of frontmatter override.

        let result = validate(
            "hyp-secc1",
            body,
            &fm,
            &ArtifactKind::Hypothesis,
            &Mode::Tactical,
        );
        assert!(
            must_finding(&result, "intent-must-have-verification-plan").is_some(),
            "tier: factum in frontmatter must NOT bypass intent-tier enforcement on Hypothesis"
        );
    }

    #[test]
    fn sec_c1_invariant_with_intent_override_still_requires_source() {
        // Audit-r4 SEC-C1: same direction the other way — an operator
        // MUST NOT be able to flip an Invariant to intent and skip the
        // mandatory Source citation.
        let fm = fm_with_kind_and_tier("inv-secc1", "invariant", Some("intent"));

        let body = "\
## Invariant\nUser email must be unique per organisation.\n\n\
## Scope\nOrganisations table.\n\n\
## Enforcement\nDatabase unique constraint.\n\n\
## Violation Detection\nMigration test.\n";
        // No `## Source` — should still trigger factum rule
        // because Invariant is factum regardless of frontmatter override.

        let result = validate(
            "inv-secc1",
            body,
            &fm,
            &ArtifactKind::Invariant,
            &Mode::Tactical,
        );
        assert!(
            must_finding(&result, "factum-must-have-source").is_some(),
            "tier: intent in frontmatter must NOT bypass factum-tier enforcement on Invariant"
        );
    }

    // ─── 10. Required-section constants size contract ──────────────────────

    #[test]
    fn required_sections_constants_match_spec_size() {
        // W2 imports these. A future refactor that resizes any of them
        // is a contract change — bump the expectation here to make the
        // breakage visible and intentional.
        assert_eq!(REQUIRED_SECTIONS_USE_CASE.len(), 5);
        assert_eq!(REQUIRED_SECTIONS_GLOSSARY.len(), 3);
        assert_eq!(REQUIRED_SECTIONS_INVARIANT.len(), 5);
        assert_eq!(REQUIRED_SECTIONS_SCENARIO.len(), 5);
        assert_eq!(REQUIRED_SECTIONS_HYPOTHESIS.len(), 6);
        assert_eq!(REQUIRED_SECTIONS_DOMAIN_MODEL.len(), 3);
    }

    #[test]
    fn factum_required_sections_all_include_source() {
        // Source citation is the defining factum-tier requirement.
        // Every factum kind's required-sections list MUST include it.
        for (label, list) in [
            ("UseCase", REQUIRED_SECTIONS_USE_CASE),
            ("Glossary", REQUIRED_SECTIONS_GLOSSARY),
            ("Invariant", REQUIRED_SECTIONS_INVARIANT),
            ("Scenario", REQUIRED_SECTIONS_SCENARIO),
            ("DomainModel", REQUIRED_SECTIONS_DOMAIN_MODEL),
        ] {
            assert!(
                list.contains(&"Source"),
                "factum kind {label}'s required-sections list omits 'Source'"
            );
        }
    }

    #[test]
    fn hypothesis_required_sections_include_intent_plan() {
        // The intent tier plan is verification-focused; the kind's
        // required-sections must enumerate the plan headings too so a
        // single finding can point at all of them.
        for needed in ["How To Verify", "Evidence For", "Evidence Against"] {
            assert!(
                REQUIRED_SECTIONS_HYPOTHESIS.contains(&needed),
                "Hypothesis required-sections missing intent-plan heading {needed:?}"
            );
        }
    }

    // ─── 11. rules_for dispatch covers all brownfield kinds ────────────────

    #[test]
    fn rules_for_use_case_includes_required_and_tier_rules() {
        let rules = rules_for(&ArtifactKind::UseCase, &Mode::Tactical);
        let ids: Vec<&str> = rules.iter().map(|(id, _, _, _)| *id).collect();
        assert!(ids.contains(&"use-case-required-sections"));
        assert!(ids.contains(&"factum-must-have-source"));
        assert!(ids.contains(&"intent-must-have-verification-plan"));
    }

    #[test]
    fn rules_for_hypothesis_includes_required_and_tier_rules() {
        let rules = rules_for(&ArtifactKind::Hypothesis, &Mode::Tactical);
        let ids: Vec<&str> = rules.iter().map(|(id, _, _, _)| *id).collect();
        assert!(ids.contains(&"hypothesis-required-sections"));
        assert!(ids.contains(&"factum-must-have-source"));
        assert!(ids.contains(&"intent-must-have-verification-plan"));
    }

    #[test]
    fn rules_for_glossary_invariant_scenario_domain_model_dispatch() {
        for kind in [
            ArtifactKind::Glossary,
            ArtifactKind::Invariant,
            ArtifactKind::Scenario,
            ArtifactKind::DomainModel,
        ] {
            let rules = rules_for(&kind, &Mode::Tactical);
            let ids: Vec<&str> = rules.iter().map(|(id, _, _, _)| *id).collect();
            assert!(
                ids.iter().any(|id| id.ends_with("-required-sections")),
                "kind {kind:?} dispatch missing required-sections rule"
            );
            assert!(
                ids.contains(&"factum-must-have-source"),
                "kind {kind:?} dispatch missing factum-must-have-source"
            );
        }
    }
}
