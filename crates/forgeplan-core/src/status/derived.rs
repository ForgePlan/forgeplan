/// Computed lifecycle state showing WHERE an artifact is in the methodology pipeline.
///
/// DerivedStatus is always computed, never stored. It reflects the current state
/// based on artifact content, validation results, evidence, and lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DerivedStatus {
    /// Created but empty — no MUST sections filled.
    Stub,
    /// MUST sections filled (e.g., Problem, Goals, FR for PRD).
    Shaped,
    /// `forgeplan validate` returns PASS (0 MUST errors).
    Validated,
    /// Has linked evidence with R_eff > 0.
    Evidenced,
    /// Lifecycle status is "active", "superseded", or "deprecated".
    Activated,
}

impl DerivedStatus {
    /// Human-readable label for display.
    pub fn label(&self) -> &str {
        match self {
            Self::Stub => "STUB",
            Self::Shaped => "SHAPED",
            Self::Validated => "VALIDATED",
            Self::Evidenced => "EVIDENCED",
            Self::Activated => "ACTIVATED",
        }
    }
}

impl std::fmt::Display for DerivedStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Compute derived status from artifact state.
///
/// This is a **pure function** — no DB calls, no async, just logic.
/// The caller is responsible for gathering the inputs.
///
/// # Arguments
/// * `status` - Lifecycle status: "draft", "active", "superseded", "deprecated"
/// * `body` - Artifact body text (markdown content after frontmatter)
/// * `kind` - Artifact kind: "prd", "rfc", "adr", "epic", "spec", "note", etc.
/// * `has_evidence` - Whether any evidence is linked to this artifact
/// * `r_eff` - R_eff score (0.0 if no evidence or evidence has CL0)
/// * `validation_passed` - Whether `forgeplan validate` returns PASS (0 MUST errors)
pub fn derive_status(
    status: &str,
    body: &str,
    kind: &str,
    has_evidence: bool,
    r_eff: f64,
    validation_passed: bool,
) -> DerivedStatus {
    // Terminal states: already activated in the lifecycle
    if matches!(status, "active" | "superseded" | "deprecated") {
        return DerivedStatus::Activated;
    }

    // Has supporting evidence with positive R_eff
    if has_evidence && r_eff > 0.0 {
        return DerivedStatus::Evidenced;
    }

    // Validation gate passed
    if validation_passed {
        return DerivedStatus::Validated;
    }

    // Check if MUST sections are filled based on kind
    if has_must_sections(body, kind) {
        return DerivedStatus::Shaped;
    }

    DerivedStatus::Stub
}

/// Check whether the body contains the required MUST sections for the given artifact kind.
///
/// This is a lightweight heuristic check — it looks for section headings, not content quality.
/// For full validation, use `forgeplan validate`.
fn has_must_sections(body: &str, kind: &str) -> bool {
    match kind {
        "prd" => {
            section_present(body, &["Problem", "Motivation", "Problem Statement", "Background"])
                && section_present(body, &["Goals", "Success Criteria", "Objectives"])
                && section_present(body, &["Functional Requirements", "FR"])
        }
        "rfc" => {
            section_present(body, &["Summary"])
                && section_present(body, &["Motivation", "Problem", "Problem Statement", "Background"])
                && section_present(body, &["Proposed", "Proposed Direction", "Proposed Solution"])
        }
        "adr" => {
            section_present(body, &["Context"])
                && section_present(body, &["Decision"])
                && section_present(body, &["Consequences"])
        }
        "epic" => {
            section_present(body, &["Vision"])
                && section_present(body, &["Outcomes"])
                && section_present(body, &["Children"])
        }
        "spec" => {
            section_present(body, &["Summary"])
                && section_present(body, &["API", "Data Model", "Contracts"])
        }
        // Lightweight types: note, problem, solution, evidence, refresh
        // These are considered SHAPED if they have any meaningful content
        _ => {
            let content = body.trim();
            // At least some non-trivial content (more than just whitespace/headers)
            let word_count = content.split_whitespace().count();
            word_count >= 10
        }
    }
}

/// Check if at least one of the given heading variants is present in the body.
fn section_present(body: &str, headings: &[&str]) -> bool {
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let after_hashes = trimmed.trim_start_matches('#').trim_start();
            for heading in headings {
                if after_hashes.eq_ignore_ascii_case(heading)
                    || after_hashes.to_lowercase().starts_with(&heading.to_lowercase())
                {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_empty_body() {
        let status = derive_status("draft", "", "prd", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Stub);
        assert_eq!(status.label(), "STUB");
    }

    #[test]
    fn stub_body_without_must_sections() {
        let body = "Some random text without any required sections.";
        let status = derive_status("draft", body, "prd", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Stub);
    }

    #[test]
    fn shaped_prd_with_must_sections() {
        let body = r#"
## Problem

Users need a way to track decisions.

## Goals

- Track all architecture decisions
- Provide audit trail

## Functional Requirements

- FR-001: [User] can create a new decision record
"#;
        let status = derive_status("draft", body, "prd", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Shaped);
    }

    #[test]
    fn shaped_prd_with_alias_sections() {
        let body = r#"
## Motivation

Users need a way to track decisions.

## Success Criteria

- Track all architecture decisions

## FR

- FR-001: [User] can create a new decision record
"#;
        let status = derive_status("draft", body, "prd", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Shaped);
    }

    #[test]
    fn shaped_adr_with_must_sections() {
        let body = r#"
## Context

We need to choose a database.

## Decision

We will use LanceDB.

## Consequences

Embedded DB, no server needed.
"#;
        let status = derive_status("draft", body, "adr", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Shaped);
    }

    #[test]
    fn shaped_rfc_with_must_sections() {
        let body = r#"
## Summary

New CLI architecture.

## Motivation

Current approach is ad-hoc.

## Proposed Direction

Use clap derive with subcommands.
"#;
        let status = derive_status("draft", body, "rfc", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Shaped);
    }

    #[test]
    fn shaped_epic_with_must_sections() {
        let body = r#"
## Vision

Build a complete project management tool.

## Outcomes

- Working CLI with 30+ commands

## Children

| ID | Title | Status |
|----|-------|--------|
| PRD-001 | CLI | active |
"#;
        let status = derive_status("draft", body, "epic", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Shaped);
    }

    #[test]
    fn shaped_note_with_content() {
        let body = "This is a note with enough content to be considered shaped. \
                     It has more than ten words which is the threshold for lightweight types.";
        let status = derive_status("draft", body, "note", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Shaped);
    }

    #[test]
    fn stub_note_with_minimal_content() {
        let body = "Short note.";
        let status = derive_status("draft", body, "note", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Stub);
    }

    #[test]
    fn validated_overrides_shaped() {
        let body = r#"
## Problem

Users need tracking.

## Goals

- Track decisions

## Functional Requirements

- FR-001: [User] can create records
"#;
        let status = derive_status("draft", body, "prd", false, 0.0, true);
        assert_eq!(status, DerivedStatus::Validated);
    }

    #[test]
    fn evidenced_with_positive_reff() {
        let status = derive_status("draft", "", "prd", true, 0.85, false);
        assert_eq!(status, DerivedStatus::Evidenced);
    }

    #[test]
    fn not_evidenced_with_zero_reff() {
        // has_evidence=true but r_eff=0.0 means evidence exists but scores zero (CL0 penalty)
        let status = derive_status("draft", "", "prd", true, 0.0, false);
        assert_eq!(status, DerivedStatus::Stub);
    }

    #[test]
    fn activated_from_active_status() {
        let status = derive_status("active", "", "prd", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Activated);
    }

    #[test]
    fn activated_from_superseded_status() {
        let status = derive_status("superseded", "", "prd", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Activated);
    }

    #[test]
    fn activated_from_deprecated_status() {
        let status = derive_status("deprecated", "", "rfc", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Activated);
    }

    #[test]
    fn activated_takes_priority_over_everything() {
        // Even with evidence and validation, active status wins
        let status = derive_status("active", "## Problem\n## Goals\n## FR", "prd", true, 1.0, true);
        assert_eq!(status, DerivedStatus::Activated);
    }

    #[test]
    fn evidenced_takes_priority_over_validated() {
        let status = derive_status("draft", "", "prd", true, 0.5, true);
        assert_eq!(status, DerivedStatus::Evidenced);
    }

    #[test]
    fn display_trait() {
        assert_eq!(format!("{}", DerivedStatus::Stub), "STUB");
        assert_eq!(format!("{}", DerivedStatus::Shaped), "SHAPED");
        assert_eq!(format!("{}", DerivedStatus::Validated), "VALIDATED");
        assert_eq!(format!("{}", DerivedStatus::Evidenced), "EVIDENCED");
        assert_eq!(format!("{}", DerivedStatus::Activated), "ACTIVATED");
    }

    #[test]
    fn spec_shaped_with_summary_and_api() {
        let body = r#"
## Summary

Define the artifact data model.

## API Contracts

GET /artifacts — list all
"#;
        let status = derive_status("draft", body, "spec", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Shaped);
    }

    #[test]
    fn evidence_type_shaped_with_content() {
        let body = "## Structured Fields\n\nverdict: supports\ncongruence_level: 3\n\
                     evidence_type: measurement\n\nBenchmark shows 50ms p99 latency.";
        let status = derive_status("draft", body, "evidence", false, 0.0, false);
        assert_eq!(status, DerivedStatus::Shaped);
    }
}
