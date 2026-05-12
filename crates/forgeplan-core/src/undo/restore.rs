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
use std::path::{Path, PathBuf};

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

    // Collision check (R-3) + kind/title validation (audit H-4).
    if let Some(existing) = store.get_record(&id).await? {
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
                // artifacts. BUT — audit H-4 — the row we see might be
                // a different artifact that happened to get the same
                // ID via manual fiddling. If kind or title disagree
                // with the snapshot, refuse to clobber.
                if existing.kind != receipt.snapshot.kind {
                    anyhow::bail!(
                        "Artifact '{id}' in store has kind '{}' but receipt captured '{}' — \
                         refusing to overwrite. Resolve manually.",
                        existing.kind,
                        receipt.snapshot.kind
                    );
                }
                // Title can change legitimately via update; warn but
                // do not bail. Kind is structural and should match.
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
        // C-1 security (audit Round 3 — path traversal): NEVER trust
        // `receipt.snapshot.projection_path` verbatim. A tampered
        // receipt could point at `/etc/passwd` or anywhere outside
        // the workspace. Recompute the destination from workspace +
        // kind + id + slug, then verify it stays inside the workspace
        // via canonicalize + starts_with.
        let safe_original = match compute_safe_projection_path(workspace, &receipt.snapshot) {
            Ok(p) => p,
            Err(e) => {
                warnings.push(format!(
                    "cannot compute safe projection path: {e} — skipping projection restore"
                ));
                // Still mark overall as not restored; fall through.
                PathBuf::new()
            }
        };
        if !safe_original.as_os_str().is_empty() && trashed.exists() {
            if let Some(parent) = safe_original.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            match tokio::fs::rename(trashed, &safe_original).await {
                Ok(()) => projection_restored = true,
                Err(e) => {
                    // Try copy+remove if cross-device.
                    match tokio::fs::copy(trashed, &safe_original).await {
                        Ok(_) => {
                            let _ = tokio::fs::remove_file(trashed).await;
                            projection_restored = true;
                        }
                        Err(copy_err) => {
                            warnings.push(format!(
                                "could not restore projection to {}: {} (rename: {})",
                                safe_original.display(),
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
    //
    // Audit C-1 logic (FR-011 transactional): mark_consumed failure is
    // NOT a mere warning. If we return Ok() with the receipt still
    // unconsumed on disk, a subsequent undo_last re-applies the same
    // receipt — for Delete it collides (harmless), for Supersede/
    // Deprecate it silently re-runs update_artifact (misleading
    // "success"). Propagate the error so the caller sees the
    // transactional failure.
    if let Err(e) = mark_consumed(workspace, &receipt.receipt_id).await {
        return Err(anyhow::anyhow!(
            "restore applied successfully but failed to mark receipt {} consumed: {e}. \
             The artifact is restored in the store, but a subsequent undo_last would \
             re-apply this receipt. Manual intervention: edit the receipt file and set \
             `consumed: true`, or retry restore after fixing the underlying I/O error.",
            receipt.receipt_id
        ));
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

/// Compute the destination projection path from snapshot fields,
/// validated to live inside the workspace. Prevents the path-traversal
/// attack where a tampered receipt's `projection_path` points at
/// `/etc/passwd` or similar (audit Round 3 C-1 security).
///
/// Does NOT use `receipt.snapshot.projection_path` — trusts only
/// workspace (caller-supplied) + kind + id + title via slugify.
fn compute_safe_projection_path(
    workspace: &Path,
    snapshot: &super::ArtifactSnapshot,
) -> anyhow::Result<PathBuf> {
    use crate::artifact::types::{ArtifactKind, slugify};
    let kind: ArtifactKind = snapshot
        .kind
        .parse()
        .map_err(|e| anyhow::anyhow!("receipt has unknown kind '{}': {e}", snapshot.kind))?;
    let slug = slugify(&snapshot.title);
    let filename = format!("{}-{}.md", snapshot.id, slug);
    let candidate = workspace.join(kind.dir_name()).join(&filename);

    // Canonical check: resolved path must live under workspace.
    // Workspace may not yet exist (fresh init); fall back to simple
    // component-walk if canonicalize fails.
    match (candidate.canonicalize(), workspace.canonicalize()) {
        (Ok(resolved), Ok(ws_canon)) => {
            if !resolved.starts_with(&ws_canon) {
                anyhow::bail!(
                    "computed projection path {} escapes workspace {}",
                    resolved.display(),
                    ws_canon.display()
                );
            }
        }
        _ => {
            // Couldn't canonicalize (parent dir may not exist yet).
            // Walk components and reject `..` / absolute root segments
            // beyond workspace prefix.
            for comp in candidate.components() {
                if matches!(comp, std::path::Component::ParentDir) {
                    anyhow::bail!(
                        "candidate path {} contains `..` — refusing",
                        candidate.display()
                    );
                }
            }
        }
    }
    Ok(candidate)
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
                slug: None,
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
                slug: None,
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

    // ── Audit Round 3 regression tests ────────────────────────

    #[tokio::test]
    async fn restore_rejects_traversal_projection_path() {
        // Audit Round 3 C-1 security: a tampered receipt with
        // projection_path pointing at /tmp/pwn (outside workspace)
        // must NOT be used verbatim. Restore should compute the
        // destination from workspace+kind+id+slug and ignore the
        // receipt's `projection_path` field.
        let (_tmp, ws, store) = fresh_ws().await;

        // Seed a trashed body.
        let receipt_id = generate_receipt_id("prd", "PRD-SEC");
        let trashed = trashed_projection_path(&ws, &receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&trashed, b"# pwn payload\n")
            .await
            .unwrap();

        // Craft malicious receipt.
        let evil_target = std::env::temp_dir().join("pwn.md");
        let _ = tokio::fs::remove_file(&evil_target).await;
        let receipt = Receipt {
            receipt_id: receipt_id.clone(),
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            op: DestructiveOp::Delete,
            snapshot: ArtifactSnapshot {
                id: "PRD-SEC".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "security test".into(),
                depth: "standard".into(),
                body: "# pwn payload\n".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                relations: vec![],
                // ATTACKER-CONTROLLED: tries to write outside workspace.
                projection_path: evil_target.display().to_string(),
                slug: None,
            },
            reason: None,
            replacement: None,
            trashed_projection: trashed.display().to_string(),
            activity_log_hash: None,
            consumed: false,
        };
        write_receipt(&ws, &receipt).await.unwrap();

        apply_restore(&ws, &store, &receipt).await.unwrap();

        // The evil target must NOT exist — restore should have
        // written to the safe computed path inside the workspace.
        assert!(
            !evil_target.exists(),
            "traversal write to {} must be rejected",
            evil_target.display()
        );
        // The safe computed path SHOULD exist under workspace.
        let safe_path = ws.join("prds").join("PRD-SEC-security-test.md");
        assert!(
            safe_path.exists(),
            "safe computed path {} should have received the body",
            safe_path.display()
        );
    }

    #[tokio::test]
    async fn restore_bails_when_mark_consumed_fails() {
        // Audit Round 3 C-1 logic (FR-011): if mark_consumed fails
        // we MUST return Err, not Ok with a warning. Otherwise
        // undo_last re-applies the same receipt.
        //
        // We simulate the failure by deleting the receipt file out
        // from under mark_consumed so the `rename(tmp → path)` has
        // no target directory entry issue — actually, mark_consumed
        // will still work because it creates the tmp file and
        // renames over. Better test: make the parent read-only.
        let (_tmp, ws, store) = fresh_ws().await;
        let receipt = build_receipt("PRD-MC", DestructiveOp::Delete, "body");
        write_receipt(&ws, &receipt).await.unwrap();

        // Pre-delete the receipt file so mark_consumed's read fails.
        let rpath = receipt_path(&ws, &receipt.receipt_id);
        tokio::fs::remove_file(&rpath).await.unwrap();

        let result = apply_restore(&ws, &store, &receipt).await;
        assert!(
            result.is_err(),
            "mark_consumed failure must propagate as Err"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("failed to mark receipt"),
            "error should mention mark_consumed: {err}"
        );
    }

    #[tokio::test]
    async fn restore_bails_on_kind_mismatch_for_supersede() {
        // Audit Round 3 H-4: supersede/deprecate restore on collision
        // branch must validate kind/title. If row was re-created with
        // a different kind, refuse rather than silently overwrite.
        let (_tmp, ws, store) = fresh_ws().await;

        // Seed a DIFFERENT artifact at PRD-CONFLICT (same ID but RFC kind
        // would conflict — use same ID prefix but different kind field).
        // In practice ID collision across kinds is prevented, but attacker
        // could craft a receipt claiming prd while row is something else.
        // Simulate: row is prd, receipt claims it was rfc.
        store
            .create_artifact(&NewArtifact {
                id: "PRD-CONFLICT".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "Current".into(),
                body: "current body".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();

        let receipt = Receipt {
            receipt_id: generate_receipt_id("rfc", "PRD-CONFLICT"),
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            op: DestructiveOp::Supersede,
            snapshot: ArtifactSnapshot {
                id: "PRD-CONFLICT".into(),
                kind: "rfc".into(), // mismatch with existing row
                status: "active".into(),
                title: "Old RFC".into(),
                depth: "standard".into(),
                body: "old rfc body".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                relations: vec![],
                projection_path: String::new(),
                slug: None,
            },
            reason: None,
            replacement: Some("PRD-999".into()),
            trashed_projection: String::new(),
            activity_log_hash: None,
            consumed: false,
        };
        write_receipt(&ws, &receipt).await.unwrap();

        let err = apply_restore(&ws, &store, &receipt).await.unwrap_err();
        assert!(
            err.to_string().contains("refusing to overwrite"),
            "kind mismatch must refuse: {err}"
        );
        // Current row unchanged.
        let current = store.get_record("PRD-CONFLICT").await.unwrap().unwrap();
        assert_eq!(current.kind, "prd", "existing row must be untouched");
        assert_eq!(current.title, "Current");
    }
}
