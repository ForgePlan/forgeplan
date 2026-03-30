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

    // Methodology gates: same checks as activate() so review doesn't false-PASS
    let mut gates_ok = true;
    if supports_lifecycle(&record.kind) {
        // Stub check: body too short means MUST sections not filled
        if record.body.trim().len() < 100 {
            warnings.push(format!(
                "Body too short ({} chars, need 100+) — fill MUST sections before activating",
                record.body.trim().len()
            ));
            gates_ok = false;
        }

        // Evidence check: no evidence linked = blind spot
        let incoming = store.get_incoming_relations(artifact_id).await.unwrap_or_default();
        let has_evidence = relations.iter().any(|(_, r)| r == "informs" || r == "supports")
            || incoming.iter().any(|(source_id, _)| {
                source_id.to_uppercase().starts_with("EVID-")
            });
        if !has_evidence {
            warnings.push(
                "No evidence linked — create evidence and link it before activating".to_string(),
            );
            gates_ok = false;
        }
    }

    Ok(ReviewResult {
        artifact_id: artifact_id.to_string(),
        can_activate: must_findings.is_empty() && gates_ok,
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

/// Result of a supersede operation.
#[derive(Debug, Clone)]
pub struct SupersedeResult {
    /// Artifacts that depend on the superseded artifact.
    pub dependents: Vec<String>,
    /// Warnings (e.g., replacement is itself superseded/deprecated).
    pub warnings: Vec<String>,
}

/// Supersede an artifact: Active → Superseded, link to replacement.
pub async fn supersede(
    store: &LanceStore,
    artifact_id: &str,
    replacement_id: &str,
) -> anyhow::Result<SupersedeResult> {
    let record = store
        .get_record(artifact_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {artifact_id}"))?;

    // Verify replacement exists
    let replacement = store
        .get_record(replacement_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Replacement not found: {replacement_id}"))?;

    transitions::validate_transition(&record.status, "superseded")?;

    // Block if replacement is itself superseded or deprecated (chain risk)
    let mut warnings = Vec::new();
    if replacement.status == "superseded" || replacement.status == "deprecated" {
        anyhow::bail!(
            "Replacement {} is already {}. Choose an active or draft artifact as replacement.",
            replacement_id, replacement.status
        );
    }

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

    Ok(SupersedeResult {
        dependents,
        warnings,
    })
}

/// Deprecate an artifact: Active → Deprecated with reason.
pub async fn deprecate(
    store: &LanceStore,
    artifact_id: &str,
    reason: &str,
) -> anyhow::Result<Vec<String>> {
    let record = store
        .get_record(artifact_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {artifact_id}"))?;

    transitions::validate_transition(&record.status, "deprecated")?;

    // Append deprecation reason to body before status change
    let today = chrono::Utc::now().format("%Y-%m-%d");
    let deprecation_section = format!("\n\n## Deprecation\n\nReason: {reason}\nDate: {today}");
    let new_body = format!("{}{}", record.body, deprecation_section);
    store.update_body(artifact_id, &new_body).await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::store::NewArtifact;
    use tempfile::TempDir;

    async fn make_store(tmp: &TempDir) -> LanceStore {
        let ws = tmp.path().join(".forgeplan");
        LanceStore::init(&ws).await.unwrap()
    }

    fn active_note(id: &str) -> NewArtifact {
        NewArtifact {
            id: id.to_string(),
            kind: "note".to_string(),
            status: "active".to_string(),
            title: format!("Test Note {id}"),
            body: "Some body content.".to_string(),
            depth: "tactical".to_string(),
            author: Some("tester".to_string()),
            parent_epic: None,
            valid_until: None,
        }
    }

    #[tokio::test]
    async fn review_flags_stub_body_and_missing_evidence() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        // Create a PRD with short body (stub) and no evidence
        let art = NewArtifact {
            id: "PRD-001".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Stub PRD".to_string(),
            body: "Short body".to_string(),
            depth: "standard".to_string(),
            author: Some("test".to_string()),
            parent_epic: None,
            valid_until: None,
        };
        store.create_artifact(&art).await.unwrap();

        let result = review(&store, "PRD-001").await.unwrap();

        // Review must report can_activate = false
        assert!(
            !result.can_activate,
            "review should NOT pass for stub PRD without evidence"
        );

        // Warnings must mention both gates
        let warnings_joined = result.warnings.join(" | ");
        assert!(
            warnings_joined.contains("Body too short"),
            "should warn about short body, got: {warnings_joined}"
        );
        assert!(
            warnings_joined.contains("No evidence linked"),
            "should warn about missing evidence, got: {warnings_joined}"
        );
    }

    #[tokio::test]
    async fn deprecate_stores_reason_in_body() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.create_artifact(&active_note("NOTE-099")).await.unwrap();

        let reason = "no longer needed";
        deprecate(&store, "NOTE-099", reason).await.unwrap();

        let record = store.get_record("NOTE-099").await.unwrap().unwrap();
        assert_eq!(record.status, "deprecated");
        assert!(
            record.body.contains("## Deprecation"),
            "body should contain Deprecation section, got: {}",
            record.body
        );
        assert!(
            record.body.contains("Reason: no longer needed"),
            "body should contain the reason text, got: {}",
            record.body
        );
    }
}
