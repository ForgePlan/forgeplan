// PRD-056 (EPIC-005): advisory phase state machine — greenfield workflow.
//
// Per-artifact phase marker stored in `.forgeplan/state/<ID>.yaml`. Advisory
// only — no tool refuses to run. Surfaces current_phase in `_next_action`
// and health, lets agents and humans see where in the methodology cycle an
// artifact is.
//
// Design decisions (see PRD-056 + FPF ADI in the session log):
// - Per-artifact state file (not global session). Each artifact owns its
//   phase, multi-agent ready without cross-artifact locks.
// - Missing state file is NOT an error (FR-012). Treated as `unknown`.
//   No existing tool breaks when this module is added.
// - Feature-flag `phase.enabled` in Config gates everything (FR-013).
//   Default: true. False → pre-v0.23.0 semantics.
// - Atomic writes via tmp-file + rename + fsync (learned from PRD-055).
// - Symlink guards on state dir (PRD-055 audit H-2 security hardening).
// - Workflow-aware from day one: `workflow_type: greenfield` is the only
//   variant now, but the enum is extensible for future brownfield,
//   audit-hotfix, research, review-fix workflows (PRD-056 FR-014).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod store;

/// Methodology phase for the greenfield workflow.
///
/// Order: Shape → Validate → Adi → Code → Test → Audit → Evidence → Done.
/// `Unknown` is reserved for artifacts without a state file (pre-PRD-056
/// artifacts or state corruption).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Unknown,
    Shape,
    Validate,
    Adi,
    Code,
    Test,
    Audit,
    Evidence,
    Done,
}

impl Phase {
    /// Canonical string name (snake_case, matches serde output).
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Unknown => "unknown",
            Phase::Shape => "shape",
            Phase::Validate => "validate",
            Phase::Adi => "adi",
            Phase::Code => "code",
            Phase::Test => "test",
            Phase::Audit => "audit",
            Phase::Evidence => "evidence",
            Phase::Done => "done",
        }
    }

    /// Human-readable next-phase hint for `_next_action` messages.
    /// Returns None if this is a terminal state or unknown.
    pub fn suggested_next(self) -> Option<Phase> {
        match self {
            Phase::Shape => Some(Phase::Validate),
            Phase::Validate => Some(Phase::Adi),
            Phase::Adi => Some(Phase::Code),
            Phase::Code => Some(Phase::Test),
            Phase::Test => Some(Phase::Audit),
            Phase::Audit => Some(Phase::Evidence),
            Phase::Evidence => Some(Phase::Done),
            Phase::Done | Phase::Unknown => None,
        }
    }
}

/// Workflow type. Currently only `greenfield` (new artifacts from scratch).
/// Future child PRDs under EPIC-005 add other variants (brownfield,
/// audit_hotfix, research, review_fix, refactor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowType {
    #[default]
    Greenfield,
}

/// A single phase transition in the state history. Append-only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhaseTransition {
    /// Previous phase (None for the initial write).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Phase>,
    /// New phase.
    pub to: Phase,
    /// UTC timestamp (RFC3339 millis).
    pub at: DateTime<Utc>,
    /// Free-form reason / trigger (e.g. "auto: forgeplan_validate PASS").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// On-disk phase state for a single artifact. Serialized as YAML at
/// `.forgeplan/state/<artifact_id>.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhaseState {
    pub artifact_id: String,
    #[serde(default)]
    pub workflow_type: WorkflowType,
    pub current_phase: Phase,
    pub advanced_at: DateTime<Utc>,
    #[serde(default)]
    pub history: Vec<PhaseTransition>,
    /// Schema version for future migration. v1 for PRD-056.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

fn default_schema_version() -> u32 {
    1
}

/// Directory where per-artifact phase state lives, within a workspace root.
pub fn state_dir(workspace: &Path) -> PathBuf {
    workspace.join("state")
}

/// Absolute path of the state file for a given artifact id.
pub fn state_path(workspace: &Path, artifact_id: &str) -> PathBuf {
    state_dir(workspace).join(format!("{artifact_id}.yaml"))
}

/// Build a fresh `PhaseState` at `Shape` phase for a newly created artifact.
/// Used by `initialize_phase` on `forgeplan_new` hook.
pub fn initial_state(artifact_id: &str, reason: Option<String>) -> PhaseState {
    let now = Utc::now();
    PhaseState {
        artifact_id: artifact_id.to_string(),
        workflow_type: WorkflowType::Greenfield,
        current_phase: Phase::Shape,
        advanced_at: now,
        history: vec![PhaseTransition {
            from: None,
            to: Phase::Shape,
            at: now,
            reason,
        }],
        schema_version: 1,
    }
}

/// Returns true when phase tracking is enabled in the Config.
/// Default behavior (missing `phase` block) is `true` — FR-013.
/// The flag can be flipped to false for a clean rollback to pre-v0.23.0
/// semantics without recompiling.
pub fn is_enabled(config: &crate::config::Config) -> bool {
    config.phase.as_ref().map(|p| p.enabled).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_as_str_matches_serde_repr() {
        // Guard against drift: manual as_str() must match serde output.
        let pairs = [
            (Phase::Shape, "shape"),
            (Phase::Validate, "validate"),
            (Phase::Adi, "adi"),
            (Phase::Code, "code"),
            (Phase::Test, "test"),
            (Phase::Audit, "audit"),
            (Phase::Evidence, "evidence"),
            (Phase::Done, "done"),
            (Phase::Unknown, "unknown"),
        ];
        for (p, s) in pairs {
            assert_eq!(p.as_str(), s);
            let json = serde_json::to_string(&p).unwrap();
            // serde strings include quotes around the value
            assert_eq!(json, format!("\"{s}\""));
        }
    }

    #[test]
    fn suggested_next_follows_canonical_order() {
        assert_eq!(Phase::Shape.suggested_next(), Some(Phase::Validate));
        assert_eq!(Phase::Validate.suggested_next(), Some(Phase::Adi));
        assert_eq!(Phase::Adi.suggested_next(), Some(Phase::Code));
        assert_eq!(Phase::Code.suggested_next(), Some(Phase::Test));
        assert_eq!(Phase::Test.suggested_next(), Some(Phase::Audit));
        assert_eq!(Phase::Audit.suggested_next(), Some(Phase::Evidence));
        assert_eq!(Phase::Evidence.suggested_next(), Some(Phase::Done));
        assert_eq!(Phase::Done.suggested_next(), None);
        assert_eq!(Phase::Unknown.suggested_next(), None);
    }

    #[test]
    fn workflow_type_default_is_greenfield() {
        assert_eq!(WorkflowType::default(), WorkflowType::Greenfield);
    }

    #[test]
    fn initial_state_is_shape_with_single_history_entry() {
        let s = initial_state("PRD-999", Some("unit test".to_string()));
        assert_eq!(s.artifact_id, "PRD-999");
        assert_eq!(s.current_phase, Phase::Shape);
        assert_eq!(s.workflow_type, WorkflowType::Greenfield);
        assert_eq!(s.history.len(), 1);
        assert_eq!(s.history[0].from, None);
        assert_eq!(s.history[0].to, Phase::Shape);
        assert_eq!(s.history[0].reason.as_deref(), Some("unit test"));
        assert_eq!(s.schema_version, 1);
    }

    #[test]
    fn state_path_shape() {
        let ws = Path::new("/tmp/ws/.forgeplan");
        let p = state_path(ws, "PRD-056");
        assert_eq!(p, ws.join("state").join("PRD-056.yaml"));
    }

    #[test]
    fn phase_state_yaml_roundtrip() {
        // Ensures serialize → deserialize gives identical state. Guards
        // against accidental field rename / schema break.
        let s = initial_state("PRD-001", None);
        let yaml = serde_yaml::to_string(&s).unwrap();
        let back: PhaseState = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn is_enabled_defaults_true_when_no_phase_block() {
        // Missing optional phase: block → default behavior is enabled.
        let cfg = crate::config::Config::default();
        assert!(is_enabled(&cfg));
    }
}
