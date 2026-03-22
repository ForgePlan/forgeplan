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

/// Compute Granularity: content density score.
pub fn compute_granularity(body: &str) -> f64 {
    let mut score = 0.0;
    let mut max_score = 0.0;

    // Word count (0-0.3)
    max_score += 0.3;
    let word_count = body.split_whitespace().count();
    score += match word_count {
        0..=50 => 0.05,
        51..=150 => 0.1,
        151..=500 => 0.2,
        _ => 0.3,
    };

    // Section count (0-0.2)
    max_score += 0.2;
    let section_count = body.lines().filter(|l| l.starts_with("## ")).count();
    score += match section_count {
        0 => 0.0,
        1..=2 => 0.05,
        3..=5 => 0.1,
        6..=10 => 0.15,
        _ => 0.2,
    };

    // Checklist items (0-0.2)
    max_score += 0.2;
    let checklist_count = body
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("- [") || t.starts_with("* [")
        })
        .count();
    score += match checklist_count {
        0 => 0.0,
        1..=3 => 0.1,
        4..=10 => 0.15,
        _ => 0.2,
    };

    // Code blocks (0-0.15) — technical detail
    max_score += 0.15;
    let code_blocks = body.matches("```").count() / 2;
    score += match code_blocks {
        0 => 0.0,
        1 => 0.05,
        2..=3 => 0.1,
        _ => 0.15,
    };

    // Tables (0-0.15) — structured data
    max_score += 0.15;
    let has_table = body.contains("| --- ") || body.contains("|---|");
    if has_table {
        score += 0.15;
    }

    if max_score > 0.0 {
        score / max_score
    } else {
        0.0
    }
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
        assert!(g < 0.2, "Empty body should have low granularity: {g}");
    }

    #[test]
    fn granularity_rich_body_high() {
        let body = "## Summary\n\nLong description with lots of detail.\n\n\
                     ## Requirements\n\n- [ ] FR-001: First requirement\n- [ ] FR-002: Second\n\
                     - [ ] FR-003: Third\n- [ ] FR-004: Fourth\n\n\
                     ## Architecture\n\n```rust\nfn main() {}\n```\n\n\
                     ## Data Model\n\n| Field | Type |\n| --- | --- |\n| id | String |\n\n\
                     ## Timeline\n\nQ1 2026.\n\n## Risks\n\nSome risks.\n\n\
                     ## Implementation\n\nDetailed plan.";
        let g = compute_granularity(body);
        assert!(g > 0.5, "Rich body should have high granularity: {g}");
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
