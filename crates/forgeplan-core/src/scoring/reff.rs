use std::collections::HashSet;

use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db::store::LanceStore;
use crate::scoring::evidence::preload_evidence_map;

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

// ---------------------------------------------------------------------------
// Recursive R_eff engine (Wave 1, PRD-016)
// ---------------------------------------------------------------------------

/// Maximum recursion depth for `r_eff_recursive` to prevent resource exhaustion.
pub const MAX_RECURSION_DEPTH: u8 = 10;

/// Assurance report for an artifact, including recursive dependency analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssuranceReport {
    pub artifact_id: String,
    pub r_eff: f64,
    pub self_score: f64,
    pub weakest_link: Option<String>,
    /// Number of expired evidence items found during traversal.
    pub expired_count: u32,
    pub factors: Vec<String>,
}

/// Evidence type modifier — penalty based on evidence source type.
/// Test and Measurement are highest trust (same context), Benchmark gets a
/// slight penalty, Audit (external review) gets a larger penalty.
pub fn evidence_type_to_cl_modifier(et: &EvidenceType) -> f64 {
    match et {
        EvidenceType::Test => 0.0,
        EvidenceType::Measurement => 0.0,
        EvidenceType::Benchmark => 0.1,
        EvidenceType::Audit => 0.2,
    }
}

/// Score a single evidence item with evidence-type modifier applied.
/// Used by the recursive engine; the original `score_evidence` is preserved
/// for backward compatibility with `r_eff()`.
fn score_evidence_full(e: &EvidenceItem) -> f64 {
    if is_expired(e.valid_until) {
        return 0.1;
    }
    let base = e.verdict.score();
    let penalty = cl_penalty(e.congruence_level);
    let type_mod = evidence_type_to_cl_modifier(&e.evidence_type);
    (base - penalty - type_mod).max(0.0)
}

/// Recursively compute R_eff for an artifact and its dependency chain.
///
/// Implements the weakest-link principle across the artifact's own evidence
/// and all transitive dependencies. Uses a preloaded evidence cache to avoid
/// O(N*M) DB queries. Cycle detection + max depth prevent infinite recursion
/// and resource exhaustion.
///
/// Call `preload_evidence_map()` once, then pass the result here.
pub async fn r_eff_recursive(
    artifact_id: &str,
    store: &LanceStore,
    visited: &mut HashSet<String>,
) -> anyhow::Result<AssuranceReport> {
    // Preload evidence map once at top level
    let evidence_map = preload_evidence_map(store).await?;
    r_eff_recursive_inner(artifact_id, store, visited, &evidence_map, 0).await
}

/// Inner recursive function with evidence cache and depth tracking.
async fn r_eff_recursive_inner(
    artifact_id: &str,
    store: &LanceStore,
    visited: &mut HashSet<String>,
    evidence_map: &std::collections::HashMap<String, Vec<EvidenceItem>>,
    depth: u8,
) -> anyhow::Result<AssuranceReport> {
    // Depth limit: return neutral score to prevent resource exhaustion.
    if depth >= MAX_RECURSION_DEPTH {
        return Ok(AssuranceReport {
            artifact_id: artifact_id.to_string(),
            r_eff: 1.0,
            self_score: 1.0,
            weakest_link: None,
            expired_count: 0,
            factors: vec![format!("Max depth ({MAX_RECURSION_DEPTH}) reached, returning neutral")],
        });
    }

    // Cycle detection: return neutral score to break the cycle.
    if visited.contains(artifact_id) {
        return Ok(AssuranceReport {
            artifact_id: artifact_id.to_string(),
            r_eff: 1.0,
            self_score: 1.0,
            weakest_link: None,
            expired_count: 0,
            factors: vec!["Cycle detected, skipping re-evaluation".to_string()],
        });
    }
    visited.insert(artifact_id.to_string());

    let mut factors: Vec<String> = Vec::new();
    let mut expired_count: u32 = 0;

    // ---- 1. Self score from cached evidence (bidirectional via preload) --

    let evidence_items = evidence_map
        .get(artifact_id)
        .cloned()
        .unwrap_or_default();

    let self_score = if evidence_items.is_empty() {
        factors.push("No evidence found (L0)".to_string());
        0.0
    } else {
        for item in &evidence_items {
            if is_expired(item.valid_until) {
                expired_count += 1;
                factors.push(format!("Evidence {} expired (Decay applied)", item.id));
            }
        }

        evidence_items
            .iter()
            .map(|e| score_evidence_full(e))
            .min_by(|a, b| a.total_cmp(b))
            .unwrap_or(0.0)
    };

    // ---- 2. Dependency scores (excluding evidence artifacts) -------------

    let relations = store.get_relations(artifact_id).await?;

    // Evidence IDs to exclude from deps (they are NOT architectural dependencies)
    let evidence_ids: HashSet<&String> = evidence_map
        .values()
        .flat_map(|items| items.iter().map(|i| &i.id))
        .collect();

    let deps: Vec<(String, String)> = relations
        .iter()
        .filter(|(target_id, rel_type)| {
            matches!(rel_type.as_str(), "informs" | "based_on" | "refines" | "depends_on")
                && !evidence_ids.contains(target_id)
        })
        .cloned()
        .collect();

    let mut min_dep_score = 1.0_f64;
    let mut weakest_link: Option<String> = None;

    for (dep_id, rel_type) in &deps {
        let dep_report = match Box::pin(r_eff_recursive_inner(
            dep_id, store, visited, evidence_map, depth + 1,
        ))
        .await
        {
            Ok(report) => report,
            Err(e) => {
                factors.push(format!("Failed to compute R_eff for {dep_id}: {e}"));
                AssuranceReport {
                    artifact_id: dep_id.clone(),
                    r_eff: 0.0,
                    self_score: 0.0,
                    weakest_link: None,
                    expired_count: 0,
                    factors: vec!["Error during recursive evaluation".to_string()],
                }
            }
        };

        // CL penalty based on relation type
        let dep_cl: u8 = match rel_type.as_str() {
            "depends_on" | "refines" => 3,
            "based_on" | "informs" => 2,
            _ => 1,
        };
        let penalty = cl_penalty(dep_cl);
        let effective_r = (dep_report.r_eff - penalty).max(0.0);

        if effective_r < min_dep_score {
            min_dep_score = effective_r;
            weakest_link = Some(dep_id.clone());
        }

        if penalty > 0.0 {
            factors.push(format!("CL penalty applied for {dep_id} (relation: {rel_type})"));
        }
    }

    // ---- 3. Weakest link principle --------------------------------------

    let final_score = if deps.is_empty() {
        self_score
    } else {
        self_score.min(min_dep_score)
    };

    Ok(AssuranceReport {
        artifact_id: artifact_id.to_string(),
        r_eff: final_score,
        self_score,
        weakest_link,
        expired_count,
        factors,
    })
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

    // === PRD-016: Evidence type modifier tests ===

    #[test]
    fn evidence_type_modifier_test_no_penalty() {
        assert_eq!(evidence_type_to_cl_modifier(&EvidenceType::Test), 0.0);
    }

    #[test]
    fn evidence_type_modifier_measurement_no_penalty() {
        assert_eq!(evidence_type_to_cl_modifier(&EvidenceType::Measurement), 0.0);
    }

    #[test]
    fn evidence_type_modifier_benchmark_slight_penalty() {
        assert!((evidence_type_to_cl_modifier(&EvidenceType::Benchmark) - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn evidence_type_modifier_audit_penalty() {
        assert!((evidence_type_to_cl_modifier(&EvidenceType::Audit) - 0.2).abs() < f64::EPSILON);
    }

    // === PRD-016: score_evidence_full with type penalty ===

    #[test]
    fn score_evidence_full_benchmark_reduces() {
        let e = EvidenceItem {
            id: "e1".into(),
            evidence_type: EvidenceType::Benchmark,
            verdict: Verdict::Supports,
            congruence_level: 3,
            valid_until: None,
        };
        // 1.0 - 0.0 (CL3) - 0.1 (Benchmark) = 0.9
        let s = score_evidence_full(&e);
        assert!((s - 0.9).abs() < f64::EPSILON, "Expected 0.9, got {s}");
    }

    #[test]
    fn score_evidence_full_audit_reduces() {
        let e = EvidenceItem {
            id: "e1".into(),
            evidence_type: EvidenceType::Audit,
            verdict: Verdict::Supports,
            congruence_level: 3,
            valid_until: None,
        };
        // 1.0 - 0.0 (CL3) - 0.2 (Audit) = 0.8
        let s = score_evidence_full(&e);
        assert!((s - 0.8).abs() < f64::EPSILON, "Expected 0.8, got {s}");
    }

    #[test]
    fn score_evidence_full_combined_penalties() {
        let e = EvidenceItem {
            id: "e1".into(),
            evidence_type: EvidenceType::Audit,
            verdict: Verdict::Supports,
            congruence_level: 2, // CL2 = 0.1
            valid_until: None,
        };
        // 1.0 - 0.1 (CL2) - 0.2 (Audit) = 0.7
        let s = score_evidence_full(&e);
        assert!((s - 0.7).abs() < f64::EPSILON, "Expected 0.7, got {s}");
    }

    #[test]
    fn score_evidence_full_clamped_to_zero() {
        let e = EvidenceItem {
            id: "e1".into(),
            evidence_type: EvidenceType::Audit,
            verdict: Verdict::Weakens, // base = 0.5
            congruence_level: 1,       // CL1 = 0.4
            valid_until: None,
        };
        // 0.5 - 0.4 - 0.2 = -0.1 → 0.0
        let s = score_evidence_full(&e);
        assert_eq!(s, 0.0, "Should clamp to 0.0, got {s}");
    }

    #[test]
    fn score_evidence_full_expired_ignores_type() {
        use chrono::NaiveDate;
        let past = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap().and_hms_opt(0, 0, 0);
        let e = EvidenceItem {
            id: "e1".into(),
            evidence_type: EvidenceType::Audit,
            verdict: Verdict::Supports,
            congruence_level: 3,
            valid_until: past,
        };
        // Expired = 0.1, type penalty irrelevant
        let s = score_evidence_full(&e);
        assert!((s - 0.1).abs() < f64::EPSILON, "Expired should be 0.1, got {s}");
    }

    // === PRD-016: AssuranceReport construction ===

    #[test]
    fn assurance_report_defaults() {
        let report = AssuranceReport {
            artifact_id: "PRD-001".into(),
            r_eff: 0.0,
            self_score: 0.0,
            weakest_link: None,
            expired_count: 0,
            factors: vec![],
        };
        assert_eq!(report.artifact_id, "PRD-001");
        assert_eq!(report.r_eff, 0.0);
        assert!(report.weakest_link.is_none());
        assert!(report.factors.is_empty());
    }

    #[test]
    fn assurance_report_with_factors() {
        let report = AssuranceReport {
            artifact_id: "RFC-001".into(),
            r_eff: 0.7,
            self_score: 0.8,
            weakest_link: Some("PRD-002".into()),
            expired_count: 0,
            factors: vec!["CL penalty applied for PRD-002".into()],
        };
        assert_eq!(report.weakest_link.as_deref(), Some("PRD-002"));
        assert_eq!(report.factors.len(), 1);
        assert!(report.r_eff < report.self_score);
    }

    // === PRD-016: r_eff with mixed types (flat mode — backward compat) ===

    #[test]
    fn r_eff_mixed_types_weakest_wins() {
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
                evidence_type: EvidenceType::Audit,
                verdict: Verdict::Supports,
                congruence_level: 2, // CL2 = 0.1
                valid_until: None,
            },
        ];
        // r_eff uses old score_evidence (no type mod):
        // e1: 1.0 - 0.0 = 1.0
        // e2: 1.0 - 0.1 = 0.9
        // min = 0.9
        let score = r_eff(&evidence);
        assert!((score - 0.9).abs() < f64::EPSILON, "Expected 0.9, got {score}");
    }
}
