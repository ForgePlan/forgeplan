use crate::artifact::types::Mode;
use crate::db::store::ArtifactRecord;

/// Result of depth calibration for a single artifact.
#[derive(Debug, Clone)]
pub struct CalibrationResult {
    pub artifact_id: String,
    pub artifact_title: String,
    pub current_depth: String,
    pub suggested_depth: Mode,
    pub signals: Vec<DepthSignal>,
    pub escalation_needed: bool,
}

/// A signal that contributed to the depth suggestion.
#[derive(Debug, Clone)]
pub struct DepthSignal {
    pub name: String,
    pub value: String,
    pub minimum_depth: Mode,
}

/// Suggest depth level for an artifact based on heuristic analysis of its content.
pub fn suggest_depth(record: &ArtifactRecord, link_count: usize) -> CalibrationResult {
    let mut signals = Vec::new();
    let body_lower = record.body.to_lowercase();

    // Signal 1: Functional Requirements count
    let fr_count = record
        .body
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("- [") || t.starts_with("* [")
        })
        .count();

    if fr_count > 10 {
        signals.push(DepthSignal {
            name: "functional_requirements".into(),
            value: format!("{fr_count} items"),
            minimum_depth: Mode::Deep,
        });
    } else if fr_count > 3 {
        signals.push(DepthSignal {
            name: "functional_requirements".into(),
            value: format!("{fr_count} items"),
            minimum_depth: Mode::Standard,
        });
    }

    // Signal 2: Security / Breaking Changes sections → auto-escalate
    let has_security = body_lower.contains("## security")
        || body_lower.contains("## compliance")
        || body_lower.contains("## authentication")
        || body_lower.contains("security considerations");

    if has_security {
        signals.push(DepthSignal {
            name: "security_section".into(),
            value: "present".into(),
            minimum_depth: Mode::Deep,
        });
    }

    let has_breaking = body_lower.contains("breaking change")
        || body_lower.contains("## migration")
        || body_lower.contains("backwards compatibility");

    if has_breaking {
        signals.push(DepthSignal {
            name: "breaking_changes".into(),
            value: "detected".into(),
            minimum_depth: Mode::Deep,
        });
    }

    // Signal 3: Link count → impact proxy
    if link_count > 5 {
        signals.push(DepthSignal {
            name: "dependency_links".into(),
            value: format!("{link_count} links"),
            minimum_depth: Mode::Deep,
        });
    } else if link_count > 2 {
        signals.push(DepthSignal {
            name: "dependency_links".into(),
            value: format!("{link_count} links"),
            minimum_depth: Mode::Standard,
        });
    }

    // Signal 4: Parent epic → part of strategy
    if record.parent_epic.as_ref().is_some_and(|p| !p.is_empty()) {
        signals.push(DepthSignal {
            name: "parent_epic".into(),
            value: record.parent_epic.clone().unwrap_or_default(),
            minimum_depth: Mode::Standard,
        });
    }

    // Signal 5: Body complexity (section count)
    let section_count = record.body.lines().filter(|l| l.starts_with("## ")).count();

    if section_count > 8 {
        signals.push(DepthSignal {
            name: "section_count".into(),
            value: format!("{section_count} sections"),
            minimum_depth: Mode::Deep,
        });
    } else if section_count > 4 {
        signals.push(DepthSignal {
            name: "section_count".into(),
            value: format!("{section_count} sections"),
            minimum_depth: Mode::Standard,
        });
    }

    // Signal 6: Body length
    let line_count = record.body.lines().count();
    if line_count > 200 {
        signals.push(DepthSignal {
            name: "body_length".into(),
            value: format!("{line_count} lines"),
            minimum_depth: Mode::Deep,
        });
    } else if line_count > 50 {
        signals.push(DepthSignal {
            name: "body_length".into(),
            value: format!("{line_count} lines"),
            minimum_depth: Mode::Standard,
        });
    }

    // Determine suggested depth = max(all signal minimums)
    let suggested = signals
        .iter()
        .map(|s| &s.minimum_depth)
        .max_by_key(|m| depth_rank(m))
        .cloned()
        .unwrap_or(Mode::Tactical);

    let current = record.depth.parse::<Mode>().unwrap_or(Mode::Standard);
    let escalation_needed = depth_rank(&suggested) > depth_rank(&current);

    CalibrationResult {
        artifact_id: record.id.clone(),
        artifact_title: record.title.clone(),
        current_depth: record.depth.clone(),
        suggested_depth: suggested,
        signals,
        escalation_needed,
    }
}

fn depth_rank(mode: &Mode) -> u8 {
    match mode {
        Mode::Note => 0,
        Mode::Tactical => 1,
        Mode::Standard => 2,
        Mode::Deep => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::store::ArtifactRecord;

    fn make_record(body: &str, depth: &str, parent_epic: Option<&str>) -> ArtifactRecord {
        ArtifactRecord {
            id: "TEST-001".into(),
            kind: "prd".into(),
            status: "draft".into(),
            title: "Test Artifact".into(),
            body: body.into(),
            depth: depth.into(),
            author: None,
            parent_epic: parent_epic.map(|s| s.into()),
            r_eff_score: 0.0,
            valid_until: None,
            created_at: "2026-01-01T00:00:00".into(),
            updated_at: "2026-01-01T00:00:00".into(),
            tags: Vec::new(),
            body_hash: None,
            embedding: None,
        }
    }

    #[test]
    fn minimal_artifact_suggests_tactical() {
        let record = make_record("# Simple note\nJust a thought.", "tactical", None);
        let result = suggest_depth(&record, 0);
        assert_eq!(result.suggested_depth, Mode::Tactical);
        assert!(!result.escalation_needed);
    }

    #[test]
    fn security_section_escalates_to_deep() {
        let record = make_record(
            "# Auth Design\n## Security\nMust handle OIDC.\n## Implementation\nTBD.",
            "standard",
            None,
        );
        let result = suggest_depth(&record, 0);
        assert_eq!(result.suggested_depth, Mode::Deep);
        assert!(result.escalation_needed);
    }

    #[test]
    fn many_links_escalates() {
        let record = make_record("# Feature\n## Summary\nDoes things.", "tactical", None);
        let result = suggest_depth(&record, 6);
        assert_eq!(result.suggested_depth, Mode::Deep);
        assert!(result.escalation_needed);
    }

    #[test]
    fn parent_epic_at_least_standard() {
        let record = make_record("# Task\nSmall fix.", "tactical", Some("EPIC-001"));
        let result = suggest_depth(&record, 0);
        assert_eq!(result.suggested_depth, Mode::Standard);
        assert!(result.escalation_needed);
    }

    #[test]
    fn already_deep_no_escalation() {
        let record = make_record(
            "# Complex\n## Security\nImportant.\n## Migration\nBreaking.",
            "deep",
            None,
        );
        let result = suggest_depth(&record, 3);
        assert_eq!(result.suggested_depth, Mode::Deep);
        assert!(!result.escalation_needed);
    }
}
