//! TrustScore computation — pure functions, no I/O.
//!
//! Extracts and parameterizes logic from scoring/fgr.rs and scoring/reff.rs.

use super::config::FpfConfig;

/// Evidence data needed for trust computation (no DB dependency).
#[derive(Debug, Clone)]
pub struct EvidenceInput {
    pub verdict: Verdict,
    pub congruence_level: u8,
    pub is_expired: bool,
}

/// Evidence verdict.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verdict {
    Supports,
    Weakens,
    Refutes,
}

impl Verdict {
    pub fn base_score(self) -> f64 {
        match self {
            Verdict::Supports => 1.0,
            Verdict::Weakens => 0.5,
            Verdict::Refutes => 0.0,
        }
    }
}

/// Compute the effective score for a single evidence item.
fn score_single(e: &EvidenceInput, config: &FpfConfig) -> f64 {
    if e.is_expired {
        config.decay.expired_score
    } else {
        (e.verdict.base_score() - config.cl_penalties.penalty(e.congruence_level)).max(0.0)
    }
}

/// Computed trust score for a single artifact.
#[derive(Debug, Clone)]
pub struct TrustScore {
    /// R_eff: min(evidence_scores) — weakest link.
    pub r_eff: f64,
    /// Formality: schema compliance (0-1).
    pub formality: f64,
    /// Granularity: content completeness (0-1).
    pub granularity: f64,
    /// Reliability: composite of R_eff + links + freshness (0-1).
    pub reliability: f64,
    /// F-G-R overall: geometric mean.
    pub overall: f64,
    /// ID of the weakest evidence item (if any).
    pub weakest_link: Option<String>,
}

impl TrustScore {
    /// Compute R_eff from evidence list using weakest-link principle.
    pub fn compute_reff(evidence: &[EvidenceInput], config: &FpfConfig) -> f64 {
        if evidence.is_empty() {
            return 0.0;
        }

        let mut min_score = f64::MAX;

        for e in evidence {
            let score = score_single(e, config);
            if score < min_score {
                min_score = score;
            }
        }

        if min_score == f64::MAX {
            0.0
        } else {
            min_score
        }
    }

    /// Compute reliability component from R_eff + link count + staleness.
    pub fn compute_reliability(
        r_eff: f64,
        link_count: usize,
        is_stale: bool,
        config: &FpfConfig,
    ) -> f64 {
        let mut score = r_eff * config.weights.reff;

        // Link count bonus (scaled to max weight)
        let link_bonus = match link_count {
            0 => 0.0,
            1 => config.weights.links * 0.33,
            2..=3 => config.weights.links * 0.67,
            _ => config.weights.links,
        };
        score += link_bonus;

        if !is_stale {
            score += config.weights.freshness;
        }

        score.clamp(0.0, 1.0)
    }

    /// Compute full TrustScore from components.
    pub fn compute(
        evidence: &[EvidenceInput],
        formality: f64,
        granularity: f64,
        link_count: usize,
        is_stale: bool,
        config: &FpfConfig,
    ) -> Self {
        let r_eff = Self::compute_reff(evidence, config);
        let reliability = Self::compute_reliability(r_eff, link_count, is_stale, config);
        let overall = (formality * granularity * reliability).cbrt();

        let weakest_link = if evidence.is_empty() {
            None
        } else {
            // Find the evidence with lowest score
            evidence
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let sa = score_single(a, config);
                    let sb = score_single(b, config);
                    sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| format!("evidence-{i}"))
        };

        Self {
            r_eff,
            formality,
            granularity,
            reliability,
            overall,
            weakest_link,
        }
    }
}

impl std::fmt::Display for TrustScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "R_eff={:.2} F={:.2} G={:.2} R={:.2} overall={:.2}",
            self.r_eff, self.formality, self.granularity, self.reliability, self.overall
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> FpfConfig {
        FpfConfig::default()
    }

    #[test]
    fn reff_empty_evidence_is_zero() {
        assert!((TrustScore::compute_reff(&[], &default_config()) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn reff_single_supports_cl3_is_one() {
        let evidence = vec![EvidenceInput {
            verdict: Verdict::Supports,
            congruence_level: 3,
            is_expired: false,
        }];
        let r = TrustScore::compute_reff(&evidence, &default_config());
        assert!((r - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn reff_weakest_link_wins() {
        let evidence = vec![
            EvidenceInput {
                verdict: Verdict::Supports,
                congruence_level: 3,
                is_expired: false,
            },
            EvidenceInput {
                verdict: Verdict::Weakens,
                congruence_level: 1,
                is_expired: false,
            },
        ];
        let r = TrustScore::compute_reff(&evidence, &default_config());
        // Weakens (0.5) - CL1 penalty (0.4) = 0.1
        assert!((r - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn reff_expired_evidence_uses_decay() {
        let evidence = vec![EvidenceInput {
            verdict: Verdict::Supports,
            congruence_level: 3,
            is_expired: true,
        }];
        let r = TrustScore::compute_reff(&evidence, &default_config());
        assert!((r - 0.1).abs() < f64::EPSILON); // expired_score default
    }

    #[test]
    fn reff_refutes_is_zero() {
        let evidence = vec![EvidenceInput {
            verdict: Verdict::Refutes,
            congruence_level: 3,
            is_expired: false,
        }];
        let r = TrustScore::compute_reff(&evidence, &default_config());
        assert!((r - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn reliability_scales_with_config_weights() {
        let cfg = FpfConfig {
            weights: super::super::config::ReliabilityWeights {
                reff: 0.6,
                links: 0.2,
                freshness: 0.2,
            },
            ..Default::default()
        };
        let r = TrustScore::compute_reliability(1.0, 4, false, &cfg);
        // 1.0 * 0.6 + 0.2 (max links) + 0.2 (fresh) = 1.0
        assert!((r - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn full_trust_score_computation() {
        let evidence = vec![EvidenceInput {
            verdict: Verdict::Supports,
            congruence_level: 3,
            is_expired: false,
        }];
        let score = TrustScore::compute(&evidence, 0.8, 0.6, 2, false, &default_config());
        assert!(score.r_eff > 0.9);
        assert!(score.overall > 0.5);
        assert!(score.weakest_link.is_some());
    }

    #[test]
    fn custom_config_changes_behavior() {
        let mut cfg = FpfConfig::default();
        cfg.cl_penalties.cl3 = 0.5; // harsh penalty even for same context

        let evidence = vec![EvidenceInput {
            verdict: Verdict::Supports,
            congruence_level: 3,
            is_expired: false,
        }];
        let r = TrustScore::compute_reff(&evidence, &cfg);
        // 1.0 - 0.5 = 0.5
        assert!((r - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn reff_all_expired_returns_decay_score() {
        let evidence = vec![
            EvidenceInput {
                verdict: Verdict::Supports,
                congruence_level: 3,
                is_expired: true,
            },
            EvidenceInput {
                verdict: Verdict::Supports,
                congruence_level: 3,
                is_expired: true,
            },
        ];
        let r = TrustScore::compute_reff(&evidence, &default_config());
        assert!((r - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn reff_mixed_expired_picks_weakest() {
        let evidence = vec![
            EvidenceInput {
                verdict: Verdict::Supports,
                congruence_level: 3,
                is_expired: false, // score = 1.0
            },
            EvidenceInput {
                verdict: Verdict::Supports,
                congruence_level: 3,
                is_expired: true, // score = 0.1 (expired)
            },
        ];
        let r = TrustScore::compute_reff(&evidence, &default_config());
        assert!((r - 0.1).abs() < f64::EPSILON); // expired wins as weakest
    }

    #[test]
    fn reliability_stale_excludes_freshness_bonus() {
        let cfg = default_config();
        let fresh = TrustScore::compute_reliability(0.5, 2, false, &cfg);
        let stale = TrustScore::compute_reliability(0.5, 2, true, &cfg);
        assert!(fresh > stale);
        // Difference should be exactly the freshness weight (0.2)
        assert!((fresh - stale - cfg.weights.freshness).abs() < f64::EPSILON);
    }

    #[test]
    fn reliability_zero_links() {
        let cfg = default_config();
        let r = TrustScore::compute_reliability(0.5, 0, false, &cfg);
        // 0.5 * 0.5 + 0.0 (no links) + 0.2 (fresh) = 0.45
        assert!((r - 0.45).abs() < f64::EPSILON);
    }

    #[test]
    fn reliability_one_link() {
        let cfg = default_config();
        let r = TrustScore::compute_reliability(0.5, 1, false, &cfg);
        // 0.5 * 0.5 + 0.3 * 0.33 + 0.2 = 0.549
        assert!((r - (0.25 + 0.3 * 0.33 + 0.2)).abs() < 0.01);
    }

    #[test]
    fn reliability_clamped_to_zero() {
        // Even with extreme negative-ish scenarios, reliability >= 0
        let r = TrustScore::compute_reliability(0.0, 0, true, &default_config());
        assert!(r >= 0.0);
    }

    #[test]
    fn score_single_clamps_negative_to_zero() {
        // Weakens (0.5) - CL0 penalty (0.9) = -0.4 → clamped to 0.0
        let e = EvidenceInput {
            verdict: Verdict::Weakens,
            congruence_level: 0,
            is_expired: false,
        };
        let s = score_single(&e, &default_config());
        assert!((s - 0.0).abs() < f64::EPSILON);
    }
}
