//! F-G-R Scoring — Formality + Granularity + Reliability.
//!
//! Unlike quint-code where F-G-R is a textual label,
//! Forgeplan computes it from artifact content.

use crate::artifact::frontmatter::Frontmatter;
use crate::artifact::types::{ArtifactKind, Mode};
use crate::validation;

/// Computed F-G-R quality triplet for an artifact.
#[derive(Debug, Clone)]
pub struct FgrScore {
    pub artifact_id: String,
    /// Formality: schema compliance (0-1). What % of required fields/sections present?
    pub formality: f64,
    /// Granularity: detail density (0-1). How thorough is the content?
    pub granularity: f64,
    /// Reliability: trust level (0-1). Based on R_eff, evidence freshness, link count.
    pub reliability: f64,
}

impl FgrScore {
    /// Geometric mean of F, G, R — penalizes imbalance.
    pub fn overall(&self) -> f64 {
        (self.formality * self.granularity * self.reliability).cbrt()
    }

    /// Human-readable grade: A (>0.8), B (>0.6), C (>0.4), D (>0.2), F (<0.2).
    pub fn grade(&self) -> &'static str {
        let o = self.overall();
        if o > 0.8 {
            "A"
        } else if o > 0.6 {
            "B"
        } else if o > 0.4 {
            "C"
        } else if o > 0.2 {
            "D"
        } else {
            "F"
        }
    }
}

impl std::fmt::Display for FgrScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "F={:.2} G={:.2} R={:.2} ({})",
            self.formality,
            self.granularity,
            self.reliability,
            self.grade()
        )
    }
}

/// Compute Formality: % of validation rules that pass.
pub fn compute_formality(
    body: &str,
    frontmatter: &Frontmatter,
    kind: &ArtifactKind,
    depth: &Mode,
) -> f64 {
    let result = validation::validate("tmp", body, frontmatter, kind, depth);
    let total = result.findings.len() + result.finding_count_passed();
    if total == 0 {
        return 1.0;
    }
    let passed = result.finding_count_passed();
    passed as f64 / total as f64
}

/// Compute Granularity as COMPLETENESS — % of expected sections that have real content.
pub fn compute_granularity(body: &str) -> f64 {
    let mut score = 0.0;
    let mut checks = 0.0;

    // Check 1: Has Problem/Motivation section with > 20 words (with aliases)
    checks += 1.0;
    if section_has_content(
        body,
        &["Problem", "Motivation", "Problem Statement", "Background"],
        20,
    ) {
        score += 1.0;
    }

    // Check 2: Has Goals/Success Criteria (with aliases)
    checks += 1.0;
    if section_has_content(
        body,
        &["Goals", "Success Criteria", "Objectives", "Outcomes"],
        10,
    ) {
        score += 1.0;
    }

    // Check 3: Has FR/Requirements with checkboxes
    checks += 1.0;
    let has_fr = body.lines().any(|l| {
        let t = l.trim();
        t.starts_with("- [") || t.starts_with("* [")
    });
    if has_fr {
        score += 1.0;
    }

    // Check 4: Has Related/Dependencies
    checks += 1.0;
    if section_has_content(body, &["Related", "Dependencies", "Related Artifacts"], 5) {
        score += 1.0;
    }

    // Check 5: Body has substance (> 100 words total)
    checks += 1.0;
    if body.split_whitespace().count() > 100 {
        score += 1.0;
    }

    if checks > 0.0 { score / checks } else { 0.0 }
}

fn section_has_content(body: &str, headings: &[&str], min_words: usize) -> bool {
    for heading in headings {
        let pattern = format!("## {}", heading);
        if let Some(pos) = body.find(&pattern) {
            let after = &body[pos + pattern.len()..];
            let end = after.find("\n## ").unwrap_or(after.len());
            let section = &after[..end];
            if section.split_whitespace().count() >= min_words {
                return true;
            }
        }
    }
    false
}

/// Compute Reliability: trust score based on R_eff + metadata.
pub fn compute_reliability(r_eff_score: f64, link_count: usize, is_stale: bool) -> f64 {
    let mut score = 0.0;

    // R_eff component (0-0.5)
    score += r_eff_score * 0.5;

    // Link count component (0-0.3) — connected artifacts are more reliable
    score += match link_count {
        0 => 0.0,
        1 => 0.1,
        2..=3 => 0.2,
        _ => 0.3,
    };

    // Freshness penalty (0-0.2)
    if is_stale {
        // Stale = no freshness bonus
    } else {
        score += 0.2;
    }

    score.min(1.0)
}

/// Compute full F-G-R for an artifact.
#[allow(clippy::too_many_arguments)]
pub fn compute(
    artifact_id: &str,
    body: &str,
    frontmatter: &Frontmatter,
    kind: &ArtifactKind,
    depth: &Mode,
    r_eff_score: f64,
    link_count: usize,
    is_stale: bool,
) -> FgrScore {
    FgrScore {
        artifact_id: artifact_id.to_string(),
        formality: compute_formality(body, frontmatter, kind, depth),
        granularity: compute_granularity(body),
        reliability: compute_reliability(r_eff_score, link_count, is_stale),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn granularity_empty_body_low() {
        let g = compute_granularity("");
        assert!(g == 0.0, "Empty body should have zero granularity: {g}");
    }

    #[test]
    fn granularity_rich_body_high() {
        let body = "\
## Problem\n\n\
This is a detailed problem statement that describes what we are trying to solve \
and why it matters to users and stakeholders in the project.\n\n\
## Goals\n\n\
The primary objective is to deliver a working solution that meets all criteria.\n\n\
## Requirements\n\n\
- [ ] FR-001: First requirement\n\
- [ ] FR-002: Second requirement\n\
- [ ] FR-003: Third requirement\n\n\
## Related Artifacts\n\n\
- RFC-001: Architecture proposal for this feature\n\n\
## Architecture\n\n\
The system uses a layered approach with clear separation of concerns.\n\n\
## Timeline\n\n\
Q1 2026 delivery target.";
        let g = compute_granularity(body);
        assert!(g > 0.5, "Rich body should have high granularity: {g}");
    }

    #[test]
    fn granularity_with_motivation_alias() {
        let body = "\
## Motivation\n\n\
This is a detailed motivation section that explains the background and reasoning \
behind this initiative and why it matters for the project direction.\n\n\
## Success Criteria\n\n\
Achieve 95% coverage across all modules.\n\n\
## Functional Requirements\n\n\
- [ ] FR-001: Core feature\n\n\
## Dependencies\n\n\
Requires auth service deployed first.\n\n\
## More context for substance padding to reach over one hundred words total \
in the body so that check five passes as well with enough content.";
        let g = compute_granularity(body);
        assert!(g > 0.5, "Body with aliases should score well: {g}");
    }

    #[test]
    fn section_has_content_works() {
        let body =
            "## Motivation\n\nThis is enough words to pass the check here.\n\n## Other\n\nStuff.";
        assert!(section_has_content(body, &["Motivation", "Problem"], 5));
        assert!(!section_has_content(body, &["NonExistent"], 5));
    }

    #[test]
    fn reliability_no_evidence_low() {
        let r = compute_reliability(0.0, 0, false);
        assert!(r < 0.3, "No evidence should be low reliability: {r}");
    }

    #[test]
    fn reliability_full_evidence_high() {
        let r = compute_reliability(1.0, 4, false);
        assert!(r > 0.8, "Full evidence should be high reliability: {r}");
    }

    #[test]
    fn reliability_stale_penalty() {
        let fresh = compute_reliability(0.5, 2, false);
        let stale = compute_reliability(0.5, 2, true);
        assert!(fresh > stale, "Stale should have lower reliability");
    }

    #[test]
    fn grade_boundaries() {
        let high = FgrScore {
            artifact_id: "t".into(),
            formality: 0.9,
            granularity: 0.9,
            reliability: 0.9,
        };
        assert_eq!(high.grade(), "A");

        let low = FgrScore {
            artifact_id: "t".into(),
            formality: 0.1,
            granularity: 0.1,
            reliability: 0.1,
        };
        assert_eq!(low.grade(), "F");
    }
}
