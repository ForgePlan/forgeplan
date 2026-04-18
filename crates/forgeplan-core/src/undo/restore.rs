//! Restore from a soft-delete receipt (PRD-055, increment 3).
//!
//! Given a receipt, reconstruct the artifact state it captured:
//! - recreate the LanceDB row (or reset its status for supersede/deprecate)
//! - move the projection markdown back (for Delete) or leave in place
//! - re-link relations that were captured
//! - mark the receipt consumed so undo-last doesn't re-apply it
//!
//! # Transactional semantics (PRD-055 FR-011)
//!
//! Restore either fully succeeds or leaves the trash receipt untouched.
//! We do not mark the receipt consumed until every downstream step
//! (row create, relation link, projection move) has completed. If any
//! step fails, the receipt remains available for a later retry.
//!
//! # Collision handling (PRD-055 R-3)
//!
//! If an artifact with the same ID already exists in the store (someone
//! re-created it after delete), restore refuses and returns an error
//! so the operator can resolve manually (merge / delete-and-retry).

use super::{DestructiveOp, Receipt, mark_consumed, receipt_path};
use crate::db::store::{LanceStore, NewArtifact};
use std::path::Path;

/// Outcome of a successful restore. Surfaces warnings about partial
/// issues (e.g. orphaned relation targets) so the caller can report them
/// back to the agent.
#[derive(Debug, Clone)]
pub struct RestoreReport {
    pub artifact_id: String,
    pub op: DestructiveOp,
    pub relations_restored: usize,
    pub relations_skipped: Vec<String>, // target IDs no longer in store
    pub projection_restored: bool,
    pub warnings: Vec<String>,
}

/// Restore the artifact captured in `receipt`. Returns an error if the
/// store still has a row with the same ID (collision), or if reading
/// any receipt field fails at apply time.
pub async fn apply_restore(
    workspace: &Path,
    store: &LanceStore,
    receipt: &Receipt,
) -> anyhow::Result<RestoreReport> {
    let id = receipt.snapshot.id.clone();

    // Collision check (R-3).
    if let Some(_existing) = store.get_record(&id).await? {
        match receipt.op {
            DestructiveOp::Delete => {
                anyhow::bail!(
                    "Artifact '{id}' already exists in the store — cannot restore over it. \
                     Delete or rename the current `{id}` first."
                );
            }
            DestructiveOp::Supersede | DestructiveOp::Deprecate => {
                // For status-change ops, the row is still there. We
                // reset its status and drop the supersede/deprecate
                // artifacts (new link, reason) rather than creating a
                // new row.
            }
        }
    } else if matches!(
        receipt.op,
        DestructiveOp::Supersede | DestructiveOp::Deprecate
    ) {
        // Edge case: row was deleted AFTER supersede/deprecate receipt
        // was written. Recreate it from snapshot like Delete.
    }

    let mut warnings: Vec<String> = Vec::new();

    // --- Restore artifact row ---
    match receipt.op {
        DestructiveOp::Delete => {
            let new_art = NewArtifact {
                id: receipt.snapshot.id.clone(),
                kind: receipt.snapshot.kind.clone(),
                status: receipt.snapshot.status.clone(),
                title: receipt.snapshot.title.clone(),
                body: receipt.snapshot.body.clone(),
                depth: receipt.snapshot.depth.clone(),
                author: receipt.snapshot.author.clone(),
                parent_epic: receipt.snapshot.parent_epic.clone(),
                valid_until: receipt.snapshot.valid_until.clone(),
                tags: Vec::new(),
            };
            store.create_artifact(&new_art).await?;
        }
        DestructiveOp::Supersede | DestructiveOp::Deprecate => {
            // If row exists, reset status + clear any supersede link.
            // If row was also deleted after, recreate fully.
            if store.get_record(&id).await?.is_some() {
                store
                    .update_artifact(
                        &id,
                        Some(&receipt.snapshot.status),
                        Some(&receipt.snapshot.title),
                    )
                    .await?;
                // Drop the new supersede link that lifecycle::supersede added.
                if let DestructiveOp::Supersede = receipt.op
                    && let Some(repl) = &receipt.replacement
                    && let Err(e) = store.delete_relation(&id, repl, "supersedes").await
                {
                    warnings.push(format!("could not drop supersede link {id}→{repl}: {e}"));
                }
            } else {
                // Row was deleted afterwards; recreate like Delete.
                let new_art = NewArtifact {
                    id: receipt.snapshot.id.clone(),
                    kind: receipt.snapshot.kind.clone(),
                    status: receipt.snapshot.status.clone(),
                    title: receipt.snapshot.title.clone(),
                    body: receipt.snapshot.body.clone(),
                    depth: receipt.snapshot.depth.clone(),
                    author: receipt.snapshot.author.clone(),
                    parent_epic: receipt.snapshot.parent_epic.clone(),
                    valid_until: receipt.snapshot.valid_until.clone(),
                    tags: Vec::new(),
                };
                store.create_artifact(&new_art).await?;
            }
        }
    }

    // --- Restore relations ---
    let mut relations_restored = 0usize;
    let mut relations_skipped: Vec<String> = Vec::new();

    for rel in &receipt.snapshot.relations {
        let (source, target) = (rel.from.as_str(), rel.to.as_str());
        // Outgoing: this artifact → target. Target must exist.
        // Incoming: source → this artifact. Source must exist.
        // For both we re-add the (source, target, relation) triple.
        let other_id = match rel.direction {
            super::RelationDirection::Outgoing => target,
            super::RelationDirection::Incoming => source,
        };
        // Skip orphaned links if the other end is missing.
        if store.get_record(other_id).await?.is_none() {
            relations_skipped.push(other_id.to_string());
            continue;
        }
        match store.add_relation(source, target, &rel.relation).await {
            Ok(()) => relations_restored += 1,
            Err(e) => warnings.push(format!("relation {source}→{target} skipped: {e}")),
        }
    }

    // --- Move projection back (Delete only) ---
    let mut projection_restored = false;
    if matches!(receipt.op, DestructiveOp::Delete) {
        let trashed = Path::new(&receipt.trashed_projection);
        let original = Path::new(&receipt.snapshot.projection_path);
        if trashed.exists() && !original.as_os_str().is_empty() {
            if let Some(parent) = original.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            match tokio::fs::rename(trashed, original).await {
                Ok(()) => projection_restored = true,
                Err(e) => {
                    // Try copy+remove if cross-device.
                    match tokio::fs::copy(trashed, original).await {
                        Ok(_) => {
                            let _ = tokio::fs::remove_file(trashed).await;
                            projection_restored = true;
                        }
                        Err(copy_err) => {
                            warnings.push(format!(
                                "could not restore projection to {}: {} (rename: {})",
                                original.display(),
                                copy_err,
                                e
                            ));
                        }
                    }
                }
            }
        }
    } else {
        // For supersede/deprecate the projection was never moved.
        projection_restored = true;
    }

    // --- Mark receipt consumed (prevents undo-last re-application) ---
    if let Err(e) = mark_consumed(workspace, &receipt.receipt_id).await {
        warnings.push(format!("could not mark receipt consumed: {e}"));
    }

    Ok(RestoreReport {
        artifact_id: id,
        op: receipt.op,
        relations_restored,
        relations_skipped,
        projection_restored,
        warnings,
    })
}

/// Read a receipt by ID and apply_restore. Convenience wrapper.
pub async fn apply_restore_by_id(
    workspace: &Path,
    store: &LanceStore,
    receipt_id: &str,
) -> anyhow::Result<RestoreReport> {
    let path = receipt_path(workspace, receipt_id);
    let receipt = super::read_receipt(&path).await?;
    if receipt.consumed {
        anyhow::bail!("receipt {receipt_id} is already consumed");
    }
    apply_restore(workspace, store, &receipt).await
}

#[cfg(test)]
mod tests {
    use super::super::{
        ArtifactSnapshot, CapturedRelation, DestructiveOp, Receipt, RelationDirection,
        generate_receipt_id, trashed_projection_path, write_receipt,
    };
    use super::*;
    use crate::workspace;
    use tempfile::TempDir;

    async fn fresh_ws() -> (TempDir, std::path::PathBuf, LanceStore) {
        let tmp = TempDir::new().unwrap();
        let ws = workspace::init_workspace(tmp.path(), "rt-test").unwrap();
        let store = LanceStore::init(&ws).await.unwrap();
        (tmp, ws, store)
    }

    fn build_receipt(id: &str, op: DestructiveOp, body: &str) -> Receipt {
        let rid = generate_receipt_id("prd", id);
        Receipt {
            receipt_id: rid.clone(),
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            op,
            snapshot: ArtifactSnapshot {
                id: id.into(),
                kind: "prd".into(),
                status: "active".into(),
                title: format!("Title {id}"),
                depth: "standard".into(),
                body: body.into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                relations: vec![],
                projection_path: String::new(),
            },
            reason: None,
            replacement: None,
            trashed_projection: String::new(),
            activity_log_hash: None,
            consumed: false,
        }
    }

    #[tokio::test]
    async fn restore_delete_recreates_row() {
        let (_tmp, ws, store) = fresh_ws().await;
        let receipt = build_receipt("PRD-001", DestructiveOp::Delete, "# body");
        write_receipt(&ws, &receipt).await.unwrap();

        // Simulate prior delete: no row in store.
        assert!(store.get_record("PRD-001").await.unwrap().is_none());

        let report = apply_restore(&ws, &store, &receipt).await.unwrap();
        assert_eq!(report.artifact_id, "PRD-001");

        let restored = store
            .get_record("PRD-001")
            .await
            .unwrap()
            .expect("row back");
        assert_eq!(restored.body, "# body");
        assert_eq!(restored.status, "active");
    }

    #[tokio::test]
    async fn restore_refuses_if_id_collision_on_delete() {
        let (_tmp, ws, store) = fresh_ws().await;

        // Create PRD-001 currently in store (someone re-created it).
        let existing = NewArtifact {
            id: "PRD-001".into(),
            kind: "prd".into(),
            status: "active".into(),
            title: "Collider".into(),
            body: "different body".into(),
            depth: "standard".into(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        };
        store.create_artifact(&existing).await.unwrap();

        // A delete-receipt for the same ID.
        let receipt = build_receipt("PRD-001", DestructiveOp::Delete, "original body");
        write_receipt(&ws, &receipt).await.unwrap();

        let err = apply_restore(&ws, &store, &receipt).await.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("already exists"),
            "expected collision error, got: {msg}"
        );

        // Existing row untouched.
        let r = store.get_record("PRD-001").await.unwrap().unwrap();
        assert_eq!(r.body, "different body");
    }

    #[tokio::test]
    async fn restore_deprecate_resets_status() {
        let (_tmp, ws, store) = fresh_ws().await;
        // Create as active in store.
        store
            .create_artifact(&NewArtifact {
                id: "PRD-002".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "To deprecate".into(),
                body: "b".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        // Change to deprecated in store.
        store
            .update_artifact("PRD-002", Some("deprecated"), None)
            .await
            .unwrap();

        // Receipt captured state BEFORE deprecate (status=active).
        let mut receipt = build_receipt("PRD-002", DestructiveOp::Deprecate, "b");
        receipt.snapshot.title = "To deprecate".into();
        receipt.reason = Some("testing".into());
        write_receipt(&ws, &receipt).await.unwrap();

        apply_restore(&ws, &store, &receipt).await.unwrap();

        let r = store.get_record("PRD-002").await.unwrap().unwrap();
        assert_eq!(r.status, "active");
    }

    #[tokio::test]
    async fn restore_marks_receipt_consumed() {
        let (_tmp, ws, store) = fresh_ws().await;
        let receipt = build_receipt("PRD-003", DestructiveOp::Delete, "x");
        write_receipt(&ws, &receipt).await.unwrap();
        apply_restore(&ws, &store, &receipt).await.unwrap();

        // Second try via apply_restore_by_id should refuse (consumed).
        let err = apply_restore_by_id(&ws, &store, &receipt.receipt_id)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("already consumed"));
    }

    #[tokio::test]
    async fn restore_skips_orphaned_relations() {
        let (_tmp, ws, store) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-004", DestructiveOp::Delete, "b");
        // Outgoing relation to EVID-999 which doesn't exist in store.
        receipt.snapshot.relations.push(CapturedRelation {
            from: "PRD-004".into(),
            to: "EVID-999".into(),
            relation: "informs".into(),
            direction: RelationDirection::Outgoing,
        });
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore(&ws, &store, &receipt).await.unwrap();
        assert_eq!(report.relations_restored, 0);
        assert_eq!(report.relations_skipped, vec!["EVID-999".to_string()]);
    }

    #[tokio::test]
    async fn restore_moves_projection_back_for_delete() {
        let (_tmp, ws, store) = fresh_ws().await;

        // Prepare a trashed projection file.
        let receipt_id = generate_receipt_id("prd", "PRD-005");
        let trashed = trashed_projection_path(&ws, &receipt_id);
        let parent = trashed.parent().unwrap();
        tokio::fs::create_dir_all(parent).await.unwrap();
        tokio::fs::write(&trashed, b"# original body\n")
            .await
            .unwrap();

        // Target path for restore.
        let original = ws.join("prds").join("PRD-005-t.md");

        let receipt = Receipt {
            receipt_id: receipt_id.clone(),
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            op: DestructiveOp::Delete,
            snapshot: ArtifactSnapshot {
                id: "PRD-005".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "t".into(),
                depth: "standard".into(),
                body: "# original body\n".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                relations: vec![],
                projection_path: original.display().to_string(),
            },
            reason: None,
            replacement: None,
            trashed_projection: trashed.display().to_string(),
            activity_log_hash: None,
            consumed: false,
        };
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore(&ws, &store, &receipt).await.unwrap();
        assert!(report.projection_restored);
        assert!(
            original.exists(),
            "projection should be back at {}",
            original.display()
        );
        assert!(!trashed.exists(), "trashed copy should be gone");
    }
}
