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

/// Artifact kinds that support full lifecycle (review → activate).
/// Notes and Problems are lightweight — they stay draft or get superseded directly.
const LIFECYCLE_KINDS: &[&str] = &["prd", "epic", "spec", "rfc", "adr"];

/// Check if a kind supports lifecycle operations.
pub fn supports_lifecycle(kind: &str) -> bool {
    LIFECYCLE_KINDS.contains(&kind.to_lowercase().as_str())
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

/// Result of an activation attempt, including any validation findings.
#[derive(Debug, Clone)]
pub struct ActivateResult {
    pub artifact_id: String,
    pub forced: bool,
    pub must_errors: Vec<String>,
}

/// Activate an artifact: Draft → Active (with validation gate).
/// Only PRD, Epic, Spec, RFC, ADR support activation.
/// Notes/Problems can be activated without validation gate.
///
/// If `force` is true, activation proceeds even with MUST validation errors.
/// Returns `ActivateResult` so the caller can display warnings when forced.
pub async fn activate(
    store: &LanceStore,
    artifact_id: &str,
    force: bool,
) -> anyhow::Result<ActivateResult> {
    let record = store
        .get_record(artifact_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {artifact_id}"))?;

    transitions::validate_transition(&record.status, "active")?;

    // Lightweight kinds skip validation gate
    if !supports_lifecycle(&record.kind) {
        store
            .update_artifact(artifact_id, Some("active"), None)
            .await?;
        return Ok(ActivateResult {
            artifact_id: artifact_id.to_string(),
            forced: false,
            must_errors: vec![],
        });
    }

    // Methodology enforcement: stub check (body too short = not filled)
    if record.body.trim().len() < 100 && !force {
        anyhow::bail!(
            "Cannot activate {}: body too short ({} chars). Fill required sections first.\n\
             Current state: STUB → need VALIDATED before ACTIVATED.\n\
             Run: forgeplan update {} --body \"...\"",
            artifact_id, record.body.trim().len(), artifact_id
        );
    }

    // Methodology enforcement: evidence check (no evidence = blind spot)
    if !force {
        let relations = store.get_relations(artifact_id).await.unwrap_or_default();
        let incoming = store.get_incoming_relations(artifact_id).await.unwrap_or_default();
        let has_evidence = relations.iter().any(|(_, r)| r == "informs" || r == "supports")
            || incoming.iter().any(|(source_id, _)| {
                source_id.to_uppercase().starts_with("EVID-")
            });
        if !has_evidence {
            anyhow::bail!(
                "Cannot activate {}: no evidence linked. Create evidence first.\n\
                 Current state: VALIDATED → need EVIDENCED before ACTIVATED.\n\
                 Run: forgeplan new evidence \"...\" && forgeplan link EVID-XXX {} --relation informs",
                artifact_id, artifact_id
            );
        }
    }

    // Must pass validation before activation (unless --force)
    let review_result = review(store, artifact_id).await?;
    if !review_result.can_activate && !force {
        let mut msg = format!(
            "Validation failed ({} MUST error{}):",
            review_result.must_findings.len(),
            if review_result.must_findings.len() == 1 { "" } else { "s" }
        );
        for finding in &review_result.must_findings {
            msg.push_str(&format!("\n  - {finding}"));
        }
        msg.push_str("\n\nUse --force to override.");
        anyhow::bail!("{msg}");
    }

    store
        .update_artifact(artifact_id, Some("active"), None)
        .await?;

    Ok(ActivateResult {
        artifact_id: artifact_id.to_string(),
        forced: force && !review_result.must_findings.is_empty(),
        must_errors: review_result.must_findings,
    })
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
