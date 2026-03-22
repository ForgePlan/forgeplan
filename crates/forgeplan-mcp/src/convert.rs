use forgeplan_core::artifact::store::ArtifactSummary;
use forgeplan_core::db::store::ArtifactRecord;
use forgeplan_core::validation::{Finding, ValidationResult};

use crate::types::{ArtifactRecordDto, ArtifactSummaryDto, ValidationFindingDto, ValidationResultDto};

impl From<ArtifactSummary> for ArtifactSummaryDto {
    fn from(s: ArtifactSummary) -> Self {
        Self {
            id: s.id,
            kind: s.kind,
            status: s.status,
            title: s.title,
        }
    }
}

impl From<ArtifactRecord> for ArtifactRecordDto {
    fn from(r: ArtifactRecord) -> Self {
        Self {
            id: r.id,
            kind: r.kind,
            status: r.status,
            title: r.title,
            body: r.body,
            depth: r.depth,
            author: r.author,
            parent_epic: r.parent_epic,
            r_eff_score: r.r_eff_score,
            valid_until: r.valid_until,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

impl From<ArtifactRecord> for ArtifactSummaryDto {
    fn from(r: ArtifactRecord) -> Self {
        Self {
            id: r.id,
            kind: r.kind,
            status: r.status,
            title: r.title,
        }
    }
}

impl From<Finding> for ValidationFindingDto {
    fn from(f: Finding) -> Self {
        Self {
            rule_id: f.rule_id,
            severity: f.severity.to_string(),
            message: f.message,
            section: f.section,
        }
    }
}

impl From<ValidationResult> for ValidationResultDto {
    fn from(r: ValidationResult) -> Self {
        let passed = r.passed();
        let error_count = r.error_count();
        let warning_count = r.warning_count();
        Self {
            artifact_id: r.artifact_id,
            kind: r.kind,
            depth: r.depth,
            passed,
            error_count,
            warning_count,
            findings: r.findings.into_iter().map(Into::into).collect(),
        }
    }
}
