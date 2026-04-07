//! Session state machine for methodology enforcement.
//!
//! Tracks the current development phase (Idle → Routing → Shaping → Coding → Evidence → PR)
//! and blocks out-of-order actions.
//!
//! State is persisted in `.forgeplan/session.yaml` and read by MCP tools
//! (`forgeplan_phase`, `forgeplan_guard`) to enforce methodology.

use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Development phase in the methodology lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    /// No active work — ready for next task.
    #[default]
    Idle,
    /// `forgeplan route` executed — determining depth and pipeline.
    Routing,
    /// Creating/filling artifact (PRD, RFC, etc.) — Shape phase.
    Shaping,
    /// Artifact validated, coding in progress.
    Coding,
    /// Code done, creating evidence and linking.
    Evidence,
    /// Evidence linked, preparing PR.
    Pr,
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::Idle => write!(f, "idle"),
            Phase::Routing => write!(f, "routing"),
            Phase::Shaping => write!(f, "shaping"),
            Phase::Coding => write!(f, "coding"),
            Phase::Evidence => write!(f, "evidence"),
            Phase::Pr => write!(f, "pr"),
        }
    }
}

/// Persisted session state in `.forgeplan/session.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionState {
    /// Current methodology phase.
    pub phase: Phase,
    /// Active artifact being worked on (e.g., "PRD-035").
    pub active_artifact: Option<String>,
    /// Depth from `forgeplan route` (tactical/standard/deep/critical).
    pub route_depth: Option<String>,
    /// When the current phase was entered.
    pub phase_started_at: Option<String>,
    /// History of phase transitions (last N).
    #[serde(default)]
    pub history: Vec<PhaseTransition>,
}

/// A recorded phase transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseTransition {
    pub from: Phase,
    pub to: Phase,
    pub artifact: Option<String>,
    pub timestamp: String,
}

const SESSION_FILE: &str = "session.yaml";
const MAX_HISTORY: usize = 20;

impl SessionState {
    /// Load session state from workspace directory.
    /// Returns default (Idle) if file doesn't exist.
    pub fn load(workspace: &Path) -> Self {
        let path = workspace.join(SESSION_FILE);
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|content| serde_yaml::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Save session state to workspace directory.
    pub fn save(&self, workspace: &Path) -> anyhow::Result<()> {
        let path = workspace.join(SESSION_FILE);
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(&path, yaml)?;
        Ok(())
    }

    /// Transition to a new phase. Records history.
    /// Returns error if transition is not allowed.
    pub fn transition(&mut self, to: Phase) -> Result<(), String> {
        validate_transition(self.phase, to, self.route_depth.as_deref())?;

        let transition = PhaseTransition {
            from: self.phase,
            to,
            artifact: self.active_artifact.clone(),
            timestamp: Utc::now().to_rfc3339(),
        };

        self.history.push(transition);
        if self.history.len() > MAX_HISTORY {
            self.history.drain(..self.history.len() - MAX_HISTORY);
        }

        self.phase = to;
        self.phase_started_at = Some(Utc::now().to_rfc3339());

        // Reset artifact on Idle
        if to == Phase::Idle {
            self.active_artifact = None;
            self.route_depth = None;
        }

        Ok(())
    }

    /// Check if a proposed transition is allowed without performing it.
    pub fn can_transition(&self, to: Phase) -> Result<(), String> {
        validate_transition(self.phase, to, self.route_depth.as_deref())
    }

    /// Get a hint for what to do next based on current phase.
    pub fn next_action_hint(&self) -> &'static str {
        match self.phase {
            Phase::Idle => "Run `forgeplan route \"task description\"` to start",
            Phase::Routing => "Create artifact: `forgeplan new prd \"Title\"`",
            Phase::Shaping => "Fill MUST sections, then: `forgeplan validate <ID>`",
            Phase::Coding => "Implement the feature, then: `forgeplan new evidence \"desc\"`",
            Phase::Evidence => "Link evidence: `forgeplan link EVID-X <artifact>`, then prepare PR",
            Phase::Pr => "Push and create PR: `git push && gh pr create --base dev`",
        }
    }

    /// Check if the current depth requires strict enforcement.
    /// Tactical = no enforcement (free flow).
    pub fn is_enforced(&self) -> bool {
        match self.route_depth.as_deref() {
            Some("tactical") => false,
            None => false, // No route = no enforcement
            _ => true,     // standard, deep, critical
        }
    }
}

/// Validate a phase transition.
fn validate_transition(from: Phase, to: Phase, depth: Option<&str>) -> Result<(), String> {
    // Tactical depth = no enforcement
    if depth == Some("tactical") {
        return Ok(());
    }

    // Always allowed: reset to Idle
    if to == Phase::Idle {
        return Ok(());
    }

    // Always allowed: same phase (idempotent)
    if from == to {
        return Ok(());
    }

    // Allowed transitions
    let allowed = matches!(
        (from, to),
        (Phase::Idle, Phase::Routing)
            | (Phase::Routing, Phase::Shaping)
            | (Phase::Shaping, Phase::Coding)
            | (Phase::Coding, Phase::Evidence)
            | (Phase::Evidence, Phase::Pr)
            // Skip forward (tactical in Standard+ is allowed with explicit phase set)
            | (Phase::Idle, Phase::Shaping) // direct new prd without route
            | (Phase::Idle, Phase::Coding) // direct coding (must have artifact)
            | (Phase::Shaping, Phase::Evidence) // skip coding (docs-only change)
            | (Phase::Coding, Phase::Pr) // skip evidence (tactical fix within Standard+ sprint)
    );

    if !allowed {
        return Err(format!(
            "Cannot go from '{}' to '{}'. {}",
            from,
            to,
            hint_for_transition(from, to)
        ));
    }

    Ok(())
}

fn hint_for_transition(from: Phase, to: Phase) -> &'static str {
    match (from, to) {
        (Phase::Routing, Phase::Coding) => "Create an artifact first: forgeplan new prd",
        (Phase::Routing, Phase::Evidence) => "Create artifact and code first",
        (Phase::Routing, Phase::Pr) => "Complete shaping → coding → evidence first",
        (Phase::Shaping, Phase::Pr) => "Code and create evidence first",
        (Phase::Evidence, Phase::Coding) => "Evidence phase complete — prepare PR or reset",
        (Phase::Pr, _) => "PR phase — merge or reset to idle",
        _ => "Follow: route → shape → code → evidence → pr",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_idle() {
        let s = SessionState::default();
        assert_eq!(s.phase, Phase::Idle);
        assert!(s.active_artifact.is_none());
    }

    #[test]
    fn happy_path_transitions() {
        let mut s = SessionState {
            route_depth: Some("standard".into()),
            ..Default::default()
        };

        assert!(s.transition(Phase::Routing).is_ok());
        assert_eq!(s.phase, Phase::Routing);

        assert!(s.transition(Phase::Shaping).is_ok());
        assert_eq!(s.phase, Phase::Shaping);

        assert!(s.transition(Phase::Coding).is_ok());
        assert_eq!(s.phase, Phase::Coding);

        assert!(s.transition(Phase::Evidence).is_ok());
        assert_eq!(s.phase, Phase::Evidence);

        assert!(s.transition(Phase::Pr).is_ok());
        assert_eq!(s.phase, Phase::Pr);

        assert!(s.transition(Phase::Idle).is_ok());
        assert_eq!(s.phase, Phase::Idle);
    }

    #[test]
    fn blocked_skip_routing_to_coding() {
        let mut s = SessionState {
            phase: Phase::Routing,
            route_depth: Some("standard".into()),
            ..Default::default()
        };

        let result = s.transition(Phase::Coding);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot go from"));
    }

    #[test]
    fn tactical_allows_anything() {
        let mut s = SessionState {
            phase: Phase::Routing,
            route_depth: Some("tactical".into()),
            ..Default::default()
        };

        // Tactical can skip directly to PR
        assert!(s.transition(Phase::Pr).is_ok());
    }

    #[test]
    fn no_depth_no_enforcement() {
        let mut s = SessionState::default();
        // No route_depth = no enforcement
        assert!(s.transition(Phase::Coding).is_ok());
    }

    #[test]
    fn idle_reset_clears_state() {
        let mut s = SessionState {
            phase: Phase::Evidence,
            active_artifact: Some("PRD-035".into()),
            route_depth: Some("deep".into()),
            ..Default::default()
        };

        assert!(s.transition(Phase::Idle).is_ok());
        assert!(s.active_artifact.is_none());
        assert!(s.route_depth.is_none());
    }

    #[test]
    fn history_recorded() {
        let mut s = SessionState {
            route_depth: Some("standard".into()),
            ..Default::default()
        };

        s.transition(Phase::Routing).unwrap();
        s.transition(Phase::Shaping).unwrap();

        assert_eq!(s.history.len(), 2);
        assert_eq!(s.history[0].from, Phase::Idle);
        assert_eq!(s.history[0].to, Phase::Routing);
        assert_eq!(s.history[1].from, Phase::Routing);
        assert_eq!(s.history[1].to, Phase::Shaping);
    }

    #[test]
    fn idempotent_same_phase() {
        let mut s = SessionState {
            phase: Phase::Coding,
            route_depth: Some("standard".into()),
            ..Default::default()
        };

        assert!(s.transition(Phase::Coding).is_ok());
        assert_eq!(s.phase, Phase::Coding);
    }

    #[test]
    fn allowed_skip_shaping_to_evidence() {
        let mut s = SessionState {
            phase: Phase::Shaping,
            route_depth: Some("standard".into()),
            ..Default::default()
        };

        // Docs-only change: skip coding
        assert!(s.transition(Phase::Evidence).is_ok());
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = SessionState {
            phase: Phase::Coding,
            active_artifact: Some("PRD-035".into()),
            route_depth: Some("deep".into()),
            ..Default::default()
        };
        s.transition(Phase::Evidence).unwrap();
        s.save(dir.path()).unwrap();

        let loaded = SessionState::load(dir.path());
        assert_eq!(loaded.phase, Phase::Evidence);
        assert_eq!(loaded.active_artifact.as_deref(), Some("PRD-035"));
        assert_eq!(loaded.history.len(), 1);
    }

    #[test]
    fn next_action_hints() {
        assert!(
            SessionState {
                phase: Phase::Idle,
                ..Default::default()
            }
            .next_action_hint()
            .contains("route")
        );
        assert!(
            SessionState {
                phase: Phase::Shaping,
                ..Default::default()
            }
            .next_action_hint()
            .contains("validate")
        );
        assert!(
            SessionState {
                phase: Phase::Coding,
                ..Default::default()
            }
            .next_action_hint()
            .contains("evidence")
        );
    }

    #[test]
    fn is_enforced_by_depth() {
        assert!(
            !SessionState {
                route_depth: None,
                ..Default::default()
            }
            .is_enforced()
        );
        assert!(
            !SessionState {
                route_depth: Some("tactical".into()),
                ..Default::default()
            }
            .is_enforced()
        );
        assert!(
            SessionState {
                route_depth: Some("standard".into()),
                ..Default::default()
            }
            .is_enforced()
        );
        assert!(
            SessionState {
                route_depth: Some("deep".into()),
                ..Default::default()
            }
            .is_enforced()
        );
        assert!(
            SessionState {
                route_depth: Some("critical".into()),
                ..Default::default()
            }
            .is_enforced()
        );
    }
}
