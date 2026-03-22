//! Pipeline mapping: Depth → Vec<ArtifactKind>.
//!
//! Defines what artifacts to create for each depth level.

use crate::artifact::types::{ArtifactKind, Mode};

/// Return the artifact pipeline for a given depth.
pub fn for_depth(depth: &Mode) -> Vec<ArtifactKind> {
    match depth {
        Mode::Note => vec![],
        Mode::Tactical => vec![],
        Mode::Standard => vec![ArtifactKind::Prd, ArtifactKind::Rfc],
        Mode::Deep => vec![
            ArtifactKind::Prd,
            ArtifactKind::Spec,
            ArtifactKind::Rfc,
            ArtifactKind::Adr,
        ],
    }
}

/// Check pipeline completion status for a given depth and existing artifacts.
pub fn completion(depth: &Mode, existing_kinds: &[ArtifactKind]) -> PipelineStatus {
    let required = for_depth(depth);
    if required.is_empty() {
        return PipelineStatus {
            total: 0,
            completed: 0,
            remaining: vec![],
            percentage: 100.0,
        };
    }

    let completed = required
        .iter()
        .filter(|k| existing_kinds.contains(k))
        .count();
    let remaining: Vec<ArtifactKind> = required
        .iter()
        .filter(|k| !existing_kinds.contains(k))
        .cloned()
        .collect();
    let total = required.len();
    let percentage = if total > 0 {
        (completed as f64 / total as f64) * 100.0
    } else {
        100.0
    };

    PipelineStatus {
        total,
        completed,
        remaining,
        percentage,
    }
}

/// Pipeline completion status.
#[derive(Debug, Clone)]
pub struct PipelineStatus {
    /// Total artifacts required by this depth.
    pub total: usize,
    /// Artifacts already created.
    pub completed: usize,
    /// Artifact types still needed.
    pub remaining: Vec<ArtifactKind>,
    /// Completion percentage (0-100).
    pub percentage: f64,
}

impl std::fmt::Display for PipelineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.total == 0 {
            write!(f, "No pipeline (tactical)")
        } else {
            write!(
                f,
                "{}/{} ({:.0}%)",
                self.completed, self.total, self.percentage
            )?;
            if !self.remaining.is_empty() {
                let names: Vec<&str> = self
                    .remaining
                    .iter()
                    .map(|k| k.template_key())
                    .collect();
                write!(f, " — remaining: {}", names.join(", "))?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tactical_pipeline_is_empty() {
        assert!(for_depth(&Mode::Tactical).is_empty());
    }

    #[test]
    fn standard_pipeline_is_prd_rfc() {
        let p = for_depth(&Mode::Standard);
        assert_eq!(p, vec![ArtifactKind::Prd, ArtifactKind::Rfc]);
    }

    #[test]
    fn deep_pipeline_has_four_artifacts() {
        let p = for_depth(&Mode::Deep);
        assert_eq!(p.len(), 4);
        assert!(p.contains(&ArtifactKind::Spec));
        assert!(p.contains(&ArtifactKind::Adr));
    }

    #[test]
    fn completion_all_done() {
        let status = completion(
            &Mode::Standard,
            &[ArtifactKind::Prd, ArtifactKind::Rfc],
        );
        assert_eq!(status.completed, 2);
        assert_eq!(status.remaining.len(), 0);
        assert!((status.percentage - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn completion_partial() {
        let status = completion(&Mode::Deep, &[ArtifactKind::Prd]);
        assert_eq!(status.completed, 1);
        assert_eq!(status.total, 4);
        assert_eq!(status.remaining.len(), 3);
        assert!((status.percentage - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn completion_tactical_always_100() {
        let status = completion(&Mode::Tactical, &[]);
        assert!((status.percentage - 100.0).abs() < f64::EPSILON);
    }
}
