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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub kind: String,
    pub depth: String,
    pub evidence: Vec<EvidenceInput>,
    pub formality: f64,
    pub granularity: f64,
    pub link_count: usize,
    pub is_stale: bool,
    /// Pre-computed trust score (available after build_context or set externally).
    pub trust: TrustScore,
}

/// Build an FpfContext from artifact data + config.
///
/// Pure function — no I/O, fully testable.
pub fn build_context(
    data: &mut ArtifactData,
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

    // Update ArtifactData with computed trust for downstream use (rule engine)
    data.trust = trust.clone();

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

    // Rule 2: No/weak evidence on non-draft → INVESTIGATE
    // Covers: active with r_eff=0 and links (M1 audit fix), or weak evidence 0<r_eff<0.5
    if data.status != "draft" && trust.r_eff < t.investigate_reff {
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

    fn make_data(
        status: &str,
        evidence: Vec<EvidenceInput>,
        f: f64,
        g: f64,
        links: usize,
        stale: bool,
    ) -> ArtifactData {
        ArtifactData {
            id: "PRD-001".into(),
            status: status.into(),
            kind: "prd".into(),
            depth: "standard".into(),
            evidence,
            formality: f,
            granularity: g,
            link_count: links,
            is_stale: stale,
            trust: TrustScore {
                r_eff: 0.0,
                formality: f,
                granularity: g,
                reliability: 0.0,
                overall: 0.0,
                weakest_link: None,
            },
        }
    }

    #[test]
    fn draft_no_evidence_gets_explore() {
        let mut data = make_data("draft", vec![], 0.2, 0.2, 0, false);
        let ctx = build_context(
            &mut data,
            ContextMembership::default(),
            vec![],
            &default_config(),
        );
        assert!(ctx.action.is_some());
        assert_eq!(ctx.action.unwrap().action_type, ActionType::Explore);
    }

    #[test]
    fn strong_evidence_gets_exploit() {
        let mut data = make_data(
            "active",
            vec![EvidenceInput {
                verdict: Verdict::Supports,
                congruence_level: 3,
                is_expired: false,
            }],
            0.8,
            0.8,
            3,
            false,
        );
        let ctx = build_context(
            &mut data,
            ContextMembership::default(),
            vec![],
            &default_config(),
        );
        assert!(ctx.action.is_some());
        assert_eq!(ctx.action.unwrap().action_type, ActionType::Exploit);
    }

    #[test]
    fn deprecated_gets_no_action() {
        let mut data = make_data("deprecated", vec![], 0.0, 0.0, 0, false);
        let ctx = build_context(
            &mut data,
            ContextMembership::default(),
            vec![],
            &default_config(),
        );
        assert!(ctx.action.is_none());
    }

    #[test]
    fn active_zero_reff_with_links_gets_investigate() {
        let mut data = make_data("active", vec![], 0.7, 0.7, 3, false);
        let ctx = build_context(
            &mut data,
            ContextMembership::default(),
            vec![],
            &default_config(),
        );
        let action = ctx.action.unwrap();
        assert_eq!(action.action_type, ActionType::Investigate);
    }

    #[test]
    fn custom_thresholds_change_action() {
        let mut cfg = default_config();
        cfg.thresholds.exploit_reff = 0.5;
        let mut data = make_data(
            "active",
            vec![EvidenceInput {
                verdict: Verdict::Supports,
                congruence_level: 2,
                is_expired: false,
            }],
            0.7,
            0.7,
            2,
            false,
        );
        let ctx = build_context(&mut data, ContextMembership::default(), vec![], &cfg);
        assert!(ctx.action.is_some());
        assert_eq!(ctx.action.unwrap().action_type, ActionType::Exploit);
    }

    #[test]
    fn well_formatted_draft_no_evidence_gets_explore() {
        let mut data = make_data("draft", vec![], 0.9, 0.9, 3, false);
        let ctx = build_context(
            &mut data,
            ContextMembership::default(),
            vec![],
            &default_config(),
        );
        assert!(ctx.action.is_some());
        assert_eq!(ctx.action.unwrap().action_type, ActionType::Explore);
    }

    #[test]
    fn medium_quality_gets_investigate() {
        let mut data = make_data(
            "active",
            vec![EvidenceInput {
                verdict: Verdict::Supports,
                congruence_level: 1,
                is_expired: false,
            }],
            0.6,
            0.6,
            2,
            false,
        );
        let ctx = build_context(
            &mut data,
            ContextMembership::default(),
            vec![],
            &default_config(),
        );
        let action = ctx.action.unwrap();
        assert_eq!(action.action_type, ActionType::Investigate);
        assert_eq!(action.priority, 4);
    }

    #[test]
    fn orphan_active_gets_explore() {
        let mut data = make_data(
            "active",
            vec![EvidenceInput {
                verdict: Verdict::Supports,
                congruence_level: 3,
                is_expired: false,
            }],
            0.7,
            0.7,
            0,
            false,
        );
        let ctx = build_context(
            &mut data,
            ContextMembership::default(),
            vec![],
            &default_config(),
        );
        let action = ctx.action.unwrap();
        assert_eq!(action.action_type, ActionType::Explore);
        assert_eq!(action.priority, 3);
    }

    #[test]
    fn superseded_gets_no_action() {
        let mut data = make_data("superseded", vec![], 0.0, 0.0, 0, false);
        let ctx = build_context(
            &mut data,
            ContextMembership::default(),
            vec![],
            &default_config(),
        );
        assert!(ctx.action.is_none());
    }
}
