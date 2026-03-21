use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    Measurement,
    Test,
    Benchmark,
    Audit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Supports,
    Weakens,
    Refutes,
}

impl Verdict {
    pub fn score(&self) -> f64 {
        match self {
            Self::Supports => 1.0,
            Self::Weakens => 0.5,
            Self::Refutes => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub id: String,
    pub evidence_type: EvidenceType,
    pub verdict: Verdict,
    /// Congruence Level 0-3. Higher = more congruent.
    pub congruence_level: u8,
    pub valid_until: Option<NaiveDateTime>,
}

/// Congruence Level penalty. CL3 = no penalty, CL0 = almost disqualified.
fn cl_penalty(cl: u8) -> f64 {
    match cl {
        0 => 0.9,
        1 => 0.4,
        2 => 0.1,
        3 => 0.0,
        _ => 0.0,
    }
}

fn is_expired(valid_until: Option<NaiveDateTime>) -> bool {
    match valid_until {
        Some(dt) => Utc::now().naive_utc() > dt,
        None => false,
    }
}

/// Score a single evidence item.
fn score_evidence(e: &EvidenceItem) -> f64 {
    // Expired evidence = 0.1 (stale, not absent)
    if is_expired(e.valid_until) {
        return 0.1;
    }
    let base = e.verdict.score();
    let penalty = cl_penalty(e.congruence_level);
    (base - penalty).max(0.0)
}

/// R_eff = min(evidence_scores) — trust equals the weakest link, NEVER average.
pub fn r_eff(evidence: &[EvidenceItem]) -> f64 {
    if evidence.is_empty() {
        return 0.0;
    }
    evidence
        .iter()
        .map(|e| score_evidence(e))
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_evidence_returns_zero() {
        assert_eq!(r_eff(&[]), 0.0);
    }

    #[test]
    fn single_supporting_cl3_returns_one() {
        let evidence = vec![EvidenceItem {
            id: "e1".into(),
            evidence_type: EvidenceType::Test,
            verdict: Verdict::Supports,
            congruence_level: 3,
            valid_until: None,
        }];
        assert_eq!(r_eff(&evidence), 1.0);
    }

    #[test]
    fn weakest_link_wins() {
        let evidence = vec![
            EvidenceItem {
                id: "e1".into(),
                evidence_type: EvidenceType::Test,
                verdict: Verdict::Supports,
                congruence_level: 3,
                valid_until: None,
            },
            EvidenceItem {
                id: "e2".into(),
                evidence_type: EvidenceType::Benchmark,
                verdict: Verdict::Weakens,
                congruence_level: 3,
                valid_until: None,
            },
        ];
        assert_eq!(r_eff(&evidence), 0.5);
    }

    #[test]
    fn cl_penalty_reduces_score() {
        let evidence = vec![EvidenceItem {
            id: "e1".into(),
            evidence_type: EvidenceType::Test,
            verdict: Verdict::Supports,
            congruence_level: 0, // CL0 = 0.9 penalty
            valid_until: None,
        }];
        let score = r_eff(&evidence);
        assert!((score - 0.1).abs() < f64::EPSILON);
    }
}
