//! Artifact lifecycle — state machine with guarded transitions.
//!
//! Draft → Active → Superseded/Deprecated
//! Each transition has validation gates.

pub mod transitions;

use crate::db::store::LanceStore;
use crate::validation::{self, Severity};

/// Result of a review operation.
#[derive(Debug, Clone)]
pub struct ReviewResult {
    pub artifact_id: String,
    pub can_activate: bool,
    pub must_findings: Vec<String>,
    pub should_findings: Vec<String>,
    pub warnings: Vec<String>,
}

impl std::fmt::Display for ReviewResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.can_activate {
            writeln!(f, "  Review PASSED — ready to activate")?;
        } else {
            writeln!(f, "  Review FAILED — fix MUST issues first")?;
        }
        writeln!(f)?;

        if !self.must_findings.is_empty() {
            writeln!(f, "MUST fix:")?;
            for item in &self.must_findings {
                writeln!(f, "  [x] {item}")?;
            }
        }

        if !self.should_findings.is_empty() {
            writeln!(f, "SHOULD fix:")?;
            for item in &self.should_findings {
                writeln!(f, "  [ ] {item}")?;
            }
        }

        if !self.warnings.is_empty() {
            writeln!(f, "Warnings:")?;
            for w in &self.warnings {
                writeln!(f, "  ! {w}")?;
            }
        }

        if self.can_activate {
            writeln!(f)?;
            writeln!(f, "Next: forgeplan activate {}", self.artifact_id)?;
        }

        Ok(())
    }
}

/// Review an artifact: run validation and check lifecycle warnings.
pub async fn review(store: &LanceStore, artifact_id: &str) -> anyhow::Result<ReviewResult> {
    let record = store
        .get_record(artifact_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {artifact_id}"))?;

    let kind = record.kind.parse()?;
    let depth = record.depth.parse().unwrap_or(crate::artifact::types::Mode::Standard);
    let fm = record.frontmatter_map();

    let result = validation::validate(artifact_id, &record.body, &fm, &kind, &depth);

    let must_findings: Vec<String> = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Must)
        .map(|f| format!("{}: {}", f.rule_id, f.message))
        .collect();

    let should_findings: Vec<String> = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Should)
        .map(|f| format!("{}: {}", f.rule_id, f.message))
        .collect();

    // Lifecycle warnings: check build-on-draft
    let mut warnings = Vec::new();
    let relations = store.get_relations(artifact_id).await.unwrap_or_default();
    for (target_id, _relation) in &relations {
        if let Ok(Some(target)) = store.get_record(target_id).await {
            if target.status == "draft" {
                warnings.push(format!(
                    "build-on-draft: depends on {} which is still Draft",
                    target_id
                ));
            }
        }
    }

    Ok(ReviewResult {
        artifact_id: artifact_id.to_string(),
        can_activate: must_findings.is_empty(),
        must_findings,
        should_findings,
        warnings,
    })
}

/// Activate an artifact: Draft → Active (with validation gate).
pub async fn activate(store: &LanceStore, artifact_id: &str) -> anyhow::Result<()> {
    let record = store
        .get_record(artifact_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {artifact_id}"))?;

    transitions::validate_transition(&record.status, "active")?;

    // Must pass validation before activation
    let review_result = review(store, artifact_id).await?;
    if !review_result.can_activate {
        let issues = review_result.must_findings.join("; ");
        anyhow::bail!("Cannot activate — MUST issues: {issues}");
    }

    store
        .update_artifact(artifact_id, Some("active"), None)
        .await?;

    Ok(())
}

/// Supersede an artifact: Active → Superseded, link to replacement.
pub async fn supersede(
    store: &LanceStore,
    artifact_id: &str,
    replacement_id: &str,
) -> anyhow::Result<Vec<String>> {
    let record = store
        .get_record(artifact_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {artifact_id}"))?;

    // Verify replacement exists
    store
        .get_record(replacement_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Replacement not found: {replacement_id}"))?;

    transitions::validate_transition(&record.status, "superseded")?;

    // Create supersedes link
    store
        .add_relation(artifact_id, replacement_id, "supersedes")
        .await?;

    // Update status
    store
        .update_artifact(artifact_id, Some("superseded"), None)
        .await?;

    // Find dependents and warn
    let all_relations = store.get_all_relations().await.unwrap_or_default();
    let dependents: Vec<String> = all_relations
        .iter()
        .filter(|(_src, tgt, _rel)| tgt == artifact_id)
        .map(|(src, _, _)| src.clone())
        .collect();

    Ok(dependents)
}

/// Deprecate an artifact: Active → Deprecated with reason.
pub async fn deprecate(
    store: &LanceStore,
    artifact_id: &str,
    _reason: &str,
) -> anyhow::Result<Vec<String>> {
    let record = store
        .get_record(artifact_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {artifact_id}"))?;

    transitions::validate_transition(&record.status, "deprecated")?;

    store
        .update_artifact(artifact_id, Some("deprecated"), None)
        .await?;

    // Find dependents and warn
    let all_relations = store.get_all_relations().await.unwrap_or_default();
    let dependents: Vec<String> = all_relations
        .iter()
        .filter(|(_src, tgt, _rel)| tgt == artifact_id)
        .map(|(src, _, _)| src.clone())
        .collect();

    Ok(dependents)
}
