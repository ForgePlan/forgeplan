pub mod adversarial;
pub mod checks;
pub mod rules;

use crate::artifact::frontmatter::Frontmatter;
use crate::artifact::types::{ArtifactKind, Mode};

/// Severity of a validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Must,
    Should,
    Could,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Must => write!(f, "MUST"),
            Self::Should => write!(f, "SHOULD"),
            Self::Could => write!(f, "COULD"),
        }
    }
}

/// A single validation finding (issue found).
#[derive(Debug, Clone)]
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub section: Option<String>,
}

/// Result of validating one artifact.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub artifact_id: String,
    pub kind: String,
    pub depth: String,
    pub findings: Vec<Finding>,
    /// Total rules checked (findings + passed).
    pub total_rules_checked: usize,
}

impl ValidationResult {
    pub fn passed(&self) -> bool {
        !self.findings.iter().any(|f| f.severity == Severity::Must)
    }

    pub fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Must)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Should)
            .count()
    }

    /// Number of rules that passed (for F-G-R formality score).
    pub fn finding_count_passed(&self) -> usize {
        self.total_rules_checked.saturating_sub(self.findings.len())
    }
}

/// Validate an artifact given its body, frontmatter, kind, and depth.
pub fn validate(
    artifact_id: &str,
    body: &str,
    fm: &Frontmatter,
    kind: &ArtifactKind,
    depth: &Mode,
) -> ValidationResult {
    let rule_set = rules::rules_for(kind, depth);
    let total_rules_checked = rule_set.len();
    let mut findings = Vec::new();

    for (rule_id, severity, description, check_fn) in &rule_set {
        if let Some(message) = check_fn(body, fm) {
            findings.push(Finding {
                rule_id: rule_id.to_string(),
                severity: *severity,
                message,
                section: Some(description.to_string()),
            });
        }
    }

    ValidationResult {
        artifact_id: artifact_id.to_string(),
        kind: format!("{:?}", kind),
        depth: format!("{:?}", depth),
        findings,
        total_rules_checked,
    }
}
