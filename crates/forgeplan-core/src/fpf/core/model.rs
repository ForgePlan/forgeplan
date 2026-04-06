//! FpfContext — unified model combining trust, context, and ADI history.
//!
//! Computed on-the-fly from store data + config. Not persisted.

use super::adi::AdiSnapshot;
use super::config::FpfConfig;
use super::trust::{EvidenceInput, TrustScore};

/// Unified FPF context for a single artifact.
#[derive(Debug, Clone)]
pub struct FpfContext {
    pub artifact_id: String,
    pub trust: TrustScore,
    pub context: ContextMembership,
    pub adi_history: Vec<AdiSnapshot>,
    pub action: Option<SuggestedAction>,
}

/// Which bounded context an artifact belongs to.
#[derive(Debug, Clone)]
pub struct ContextMembership {
    /// Cluster name (e.g., "Context-1 (PRD)").
    pub cluster_name: Option<String>,
    /// Cohesion of the cluster (0-1).
    pub cohesion: f64,
    /// Number of artifacts in the same cluster.
    pub cluster_size: usize,
}

impl Default for ContextMembership {
    fn default() -> Self {
        Self {
            cluster_name: None,
            cohesion: 0.0,
            cluster_size: 0,
        }
    }
}

/// A suggested next action from explore-exploit analysis.
#[derive(Debug, Clone)]
pub struct SuggestedAction {
    pub action_type: ActionType,
    pub reason: String,
    pub priority: u8,
}

/// Explore-exploit action types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionType {
    Explore,
    Investigate,
    Exploit,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionType::Explore => write!(f, "EXPLORE"),
            ActionType::Investigate => write!(f, "INVESTIGATE"),
            ActionType::Exploit => write!(f, "EXPLOIT"),
        }
    }
}

/// Input data for building an FpfContext (no DB dependency).
#[derive(Debug, Clone)]
pub struct ArtifactData {
    pub id: String,
    pub status: String,
    pub evidence: Vec<EvidenceInput>,
    pub formality: f64,
    pub granularity: f64,
    pub link_count: usize,
    pub is_stale: bool,
}

/// Build an FpfContext from artifact data + config.
///
/// Pure function — no I/O, fully testable.
pub fn build_context(
    data: &ArtifactData,
    context: ContextMembership,
    adi_history: Vec<AdiSnapshot>,
    config: &FpfConfig,
) -> FpfContext {
    let trust = TrustScore::compute(
        &data.evidence,
        data.formality,
        data.granularity,
        data.link_count,
        data.is_stale,
        config,
    );

    let action = suggest_action(data, &trust, config);

    FpfContext {
        artifact_id: data.id.clone(),
        trust,
        context,
        adi_history,
        action,
    }
}

/// Determine explore-exploit action based on trust score and thresholds.
fn suggest_action(
    data: &ArtifactData,
    trust: &TrustScore,
    config: &FpfConfig,
) -> Option<SuggestedAction> {
    let t = &config.thresholds;

    // Skip terminal statuses
    if data.status == "superseded" || data.status == "deprecated" {
        return None;
    }

    // Rule 1: No evidence + draft → EXPLORE (highest priority)
    // H3 fix: only check r_eff, not overall — a well-formatted draft with no evidence still needs work
    if trust.r_eff < t.explore_reff && data.status == "draft" {
        return Some(SuggestedAction {
            action_type: ActionType::Explore,
            reason: format!(
                "Draft with no evidence (R_eff={:.2}). Needs evidence to validate.",
                trust.r_eff
            ),
            priority: 1,
        });
    }

    // Rule 2: Has evidence but weak → INVESTIGATE
    if trust.r_eff > 0.0 && trust.r_eff < t.investigate_reff {
        return Some(SuggestedAction {
            action_type: ActionType::Investigate,
            reason: format!(
                "R_eff={:.2} — evidence exists but weak/stale. Refresh or add stronger evidence.",
                trust.r_eff
            ),
            priority: 2,
        });
    }

    // Rule 3: Orphan (no links) + active → EXPLORE
    if data.link_count == 0 && data.status == "active" {
        return Some(SuggestedAction {
            action_type: ActionType::Explore,
            reason: "Active but no links to other artifacts. Connect it to the graph.".into(),
            priority: 3,
        });
    }

    // Rule 4: Good R_eff + good F-G-R → EXPLOIT
    if trust.r_eff >= t.exploit_reff && trust.overall >= t.exploit_fgr {
        return Some(SuggestedAction {
            action_type: ActionType::Exploit,
            reason: format!(
                "R_eff={:.2}, quality={:.2}. Ready to build on.",
                trust.r_eff, trust.overall
            ),
            priority: 5,
        });
    }

    // Rule 5 (H2 fix): Medium quality — has evidence but not strong enough for EXPLOIT
    if trust.r_eff >= t.investigate_reff && trust.r_eff < t.exploit_reff {
        return Some(SuggestedAction {
            action_type: ActionType::Investigate,
            reason: format!(
                "R_eff={:.2} — evidence moderate. Add stronger evidence or improve quality (overall={:.2}) to unlock EXPLOIT.",
                trust.r_eff, trust.overall
            ),
            priority: 4,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fpf::core::trust::Verdict;

    fn default_config() -> FpfConfig {
        FpfConfig::default()
    }

    #[test]
    fn draft_no_evidence_gets_explore() {
        let data = ArtifactData {
            id: "PRD-001".into(),
            status: "draft".into(),
            evidence: vec![],
            formality: 0.2,
            granularity: 0.2,
            link_count: 0,
            is_stale: false,
        };
        let ctx = build_context(
            &data,
            ContextMembership::default(),
            vec![],
            &default_config(),
        );
        assert!(ctx.action.is_some());
        assert_eq!(ctx.action.unwrap().action_type, ActionType::Explore);
    }

    #[test]
    fn strong_evidence_gets_exploit() {
        let data = ArtifactData {
            id: "PRD-001".into(),
            status: "active".into(),
            evidence: vec![EvidenceInput {
                verdict: Verdict::Supports,
                congruence_level: 3,
                is_expired: false,
            }],
            formality: 0.8,
            granularity: 0.8,
            link_count: 3,
            is_stale: false,
        };
        let ctx = build_context(
            &data,
            ContextMembership::default(),
            vec![],
            &default_config(),
        );
        assert!(ctx.action.is_some());
        assert_eq!(ctx.action.unwrap().action_type, ActionType::Exploit);
    }

    #[test]
    fn deprecated_gets_no_action() {
        let data = ArtifactData {
            id: "PRD-001".into(),
            status: "deprecated".into(),
            evidence: vec![],
            formality: 0.0,
            granularity: 0.0,
            link_count: 0,
            is_stale: false,
        };
        let ctx = build_context(
            &data,
            ContextMembership::default(),
            vec![],
            &default_config(),
        );
        assert!(ctx.action.is_none());
    }

    #[test]
    fn custom_thresholds_change_action() {
        let mut cfg = default_config();
        cfg.thresholds.exploit_reff = 0.5; // lower bar for exploit

        let data = ArtifactData {
            id: "PRD-001".into(),
            status: "active".into(),
            evidence: vec![EvidenceInput {
                verdict: Verdict::Supports,
                congruence_level: 2, // CL2: penalty 0.1 → score 0.9
                is_expired: false,
            }],
            formality: 0.7,
            granularity: 0.7,
            link_count: 2,
            is_stale: false,
        };
        let ctx = build_context(&data, ContextMembership::default(), vec![], &cfg);
        assert!(ctx.action.is_some());
        assert_eq!(ctx.action.unwrap().action_type, ActionType::Exploit);
    }
}
