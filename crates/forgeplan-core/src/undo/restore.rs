//! Restore from a soft-delete receipt (PRD-055, increment 3).
//!
//! Given a receipt, reconstruct the artifact state it captured:
//! - restore canonical Markdown (move a Delete back, reset lifecycle metadata)
//! - synchronize the optional legacy row and replay captured relations
//! - mark the receipt consumed so undo-last doesn't re-apply it
//!
//! # Transactional semantics (PRD-055 FR-011)
//!
//! The receipt is not marked consumed until canonical restoration, optional
//! legacy synchronization, and relation replay have completed. An error can
//! leave some writes applied, but the receipt remains unconsumed; restore
//! never hides a failed write behind a successful recovery report.
//!
//! # Collision handling (PRD-055 R-3)
//!
//! If the canonical artifact or its legacy row already exists after Delete,
//! restore refuses so the operator can resolve the collision manually.

use super::{DestructiveOp, Receipt, mark_consumed, receipt_path};
use crate::artifact::canonical_key::canonical_key_from_document;
use crate::db::store::{LanceStore, NewArtifact};
use anyhow::Context;
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
    /// Issue #291: the status value the artifact was restored to.
    /// Mirrors `receipt.snapshot.status` (or the explicit override from
    /// [`apply_restore_with_target`]). Surfaced in tool responses so the
    /// operator can confirm which state was applied without having to
    /// re-read the artifact.
    pub restored_status: String,
}

/// Restore the artifact captured in `receipt`. Returns an error if the
/// store still has a row with the same ID (collision), or if reading
/// any receipt field fails at apply time.
/// Issue #291: restore an artifact AND override the restored status.
///
/// Same as [`apply_restore`] but accepts a `target_status` override.
/// When `target_status` is `Some(s)`, the artifact is restored to `s`.
/// Without an override, Delete preserves the exact trashed Markdown bytes;
/// lifecycle reversals apply the captured `snapshot.status`.
///
/// Valid status values: `"draft"`, `"active"`, `"deprecated"`. Any other
/// value is rejected with an error so the caller can fix the request.
///
/// The receipt's snapshot is NOT mutated — only the in-flight restore
/// writes the override. Subsequent `forgeplan_activity` on this receipt
/// will still show the original captured status.
pub async fn apply_restore_with_target(
    workspace: &Path,
    store: Option<&LanceStore>,
    receipt: &Receipt,
    target_status: Option<&str>,
) -> anyhow::Result<RestoreReport> {
    // Issue #291: validate override BEFORE any mutation so a bad value
    // doesn't half-apply a restore.
    //
    // Audit-dogfood CODE-7 fix: trim whitespace before lowercase so
    // `Some("  active  ")` doesn't fall through with an inscrutable
    // "invalid `  Active  `" message.
    let normalized_target: Option<String> = target_status.map(|t| t.trim().to_lowercase());
    if let Some(lower) = &normalized_target
        && !matches!(lower.as_str(), "draft" | "active" | "deprecated")
    {
        anyhow::bail!(
            "Invalid target_status `{}` — valid values: `draft`, `active`, `deprecated`.",
            target_status.unwrap_or("")
        );
    }

    // Explicit overrides respect the lifecycle FSM. Read canonical status
    // first, with the legacy row as a fallback when the file has no status.
    // Delete recreates an absent artifact rather than transitioning it.
    let current_status = if normalized_target.is_some() {
        let canonical_status =
            match crate::artifact::store::resolve_artifact_path(workspace, &receipt.snapshot.id)
                .await
            {
                Some(path) => {
                    let content = tokio::fs::read_to_string(path).await?;
                    let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&content)?;
                    fm.get("status").and_then(|v| v.as_str()).map(str::to_owned)
                }
                None => None,
            };
        match canonical_status {
            Some(status) => Some(status),
            None => match store {
                Some(store) => store
                    .get_record(&receipt.snapshot.id)
                    .await?
                    .map(|record| record.status),
                None => None,
            },
        }
    } else {
        None
    };
    if let Some(new_status) = &normalized_target
        && let Some(current_status) = &current_status
        && let Err(e) =
            super::super::lifecycle::transitions::validate_transition(current_status, new_status)
    {
        anyhow::bail!(
            "target_status override `{}` would violate lifecycle FSM \
             from current status `{}`: {e}. Use the canonical lifecycle \
             command (`forgeplan_activate` / `_deprecate` / `_supersede`) \
             instead of overriding restore.",
            new_status,
            current_status
        );
    }

    apply_restore_with_sink(workspace, store, receipt, normalized_target.as_deref()).await
}

pub async fn apply_restore(
    workspace: &Path,
    store: &LanceStore,
    receipt: &Receipt,
) -> anyhow::Result<RestoreReport> {
    apply_restore_with_sink(workspace, Some(store), receipt, None).await
}

/// Restore canonical Markdown without a legacy derived sink.
///
/// This uses the same canonical writes, collision checks, relation replay, and
/// receipt-consumption barrier as [`apply_restore`]. It never opens a retired
/// legacy store.
pub async fn apply_restore_file_first(
    workspace: &Path,
    receipt: &Receipt,
) -> anyhow::Result<RestoreReport> {
    apply_restore_with_sink(workspace, None, receipt, None).await
}

/// The one restore implementation both layouts share. Canonical Markdown is
/// restored first; `sink` only adds legacy collision checks and derived writes.
async fn apply_restore_with_sink(
    workspace: &Path,
    sink: Option<&LanceStore>,
    receipt: &Receipt,
    target_status: Option<&str>,
) -> anyhow::Result<RestoreReport> {
    let id = &receipt.snapshot.id;
    let restored_status = target_status.unwrap_or(&receipt.snapshot.status);
    let existing_path = crate::artifact::store::resolve_artifact_path(workspace, id).await;
    let existing_record = match sink {
        Some(store) => store.get_record(id).await?,
        None => None,
    };

    // A stale or absent derived row must not let Delete overwrite a live file.
    if receipt.op == DestructiveOp::Delete && (existing_path.is_some() || existing_record.is_some())
    {
        anyhow::bail!(
            "Artifact '{id}' already exists — cannot restore over it. \
             Delete or rename the current `{id}` first."
        );
    }
    if matches!(
        receipt.op,
        DestructiveOp::Supersede | DestructiveOp::Deprecate
    ) {
        let canonical_kind = match existing_path.as_deref() {
            Some(path) => {
                let content = tokio::fs::read_to_string(path).await?;
                let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&content)?;
                fm.get("kind").and_then(|v| v.as_str()).map(str::to_owned)
            }
            None => None,
        };
        // Kind is structural; title changes are legitimate and are reversed
        // below. Preserve the legacy kind-collision refusal as well.
        for kind in canonical_kind
            .as_deref()
            .into_iter()
            .chain(existing_record.as_ref().map(|record| record.kind.as_str()))
        {
            if !kind.is_empty() && kind != receipt.snapshot.kind {
                anyhow::bail!(
                    "Artifact '{id}' has kind '{kind}' but receipt captured '{}' — \
                     refusing to overwrite. Resolve manually.",
                    receipt.snapshot.kind
                );
            }
        }
    }

    // Canonical restoration is mandatory regardless of the optional sink.
    let path = match receipt.op {
        DestructiveOp::Delete => {
            if !restore_projection_file(workspace, receipt).await? {
                anyhow::bail!(
                    "No trashed projection exists for '{id}'; canonical restore \
                     cannot complete. The receipt remains available."
                );
            }
            let path = compute_safe_projection_path(workspace, &receipt.snapshot)?;
            // Normal restore moves the exact trashed bytes. Only an explicit
            // override authorizes rewriting their status frontmatter.
            if let Some(status) = target_status {
                crate::projection::update_metadata_file_first(&path, Some(status), None, None)
                    .await?
            } else {
                path
            }
        }
        DestructiveOp::Supersede | DestructiveOp::Deprecate => match existing_path {
            Some(path) => {
                // Keep the returned path: restoring the captured title can
                // rename the file before the supersede edge is removed.
                crate::projection::update_metadata_file_first(
                    &path,
                    Some(restored_status),
                    Some(&receipt.snapshot.title),
                    None,
                )
                .await?
            }
            None => {
                render_snapshot_projection(workspace, receipt, restored_status).await?;
                compute_safe_projection_path(workspace, &receipt.snapshot)?
            }
        },
    };
    if receipt.op == DestructiveOp::Supersede
        && let Some(replacement) = &receipt.replacement
    {
        crate::link::remove_link(&path, replacement, "supersedes")
            .await
            .with_context(|| format!("could not drop supersede link {id}→{replacement}"))?;
    }

    // No store-to-file rendering here: it would replace the canonical bytes
    // just restored from trash, including fields the receipt does not capture.
    if let Some(store) = sink {
        if existing_record.is_some() {
            store
                .update_artifact(id, Some(restored_status), Some(&receipt.snapshot.title))
                .await?;
        } else {
            store
                .create_artifact(&snapshot_artifact(receipt, restored_status))
                .await?;
        }
        if receipt.op == DestructiveOp::Supersede
            && let Some(replacement) = &receipt.replacement
        {
            store
                .delete_relation(id, replacement, "supersedes")
                .await
                .with_context(|| {
                    format!("could not drop legacy supersede link {id}→{replacement}")
                })?;
        }
    }

    let mut relations_restored = 0usize;
    let mut relations_skipped = Vec::new();
    for rel in &receipt.snapshot.relations {
        let (source, target) = (rel.from.as_str(), rel.to.as_str());
        let other_id = match rel.direction {
            super::RelationDirection::Outgoing => target,
            super::RelationDirection::Incoming => source,
        };
        // Missing participants are the only explicit skip policy. Parse,
        // permission, and write errors must leave the receipt unconsumed.
        if !participant_exists(sink, workspace, other_id).await? {
            relations_skipped.push(other_id.to_string());
            continue;
        }
        replay_relation(sink, workspace, source, target, &rel.relation)
            .await
            .with_context(|| format!("could not restore relation {source}→{target}"))?;
        relations_restored += 1;
    }

    if let Err(e) = mark_consumed(workspace, &receipt.receipt_id).await {
        return Err(anyhow::anyhow!(
            "restore applied successfully but failed to mark receipt {} consumed: {e}. \
             The artifact is restored, but a subsequent undo_last would \
             re-apply this receipt. Manual intervention: edit the receipt file and set \
             `consumed: true`, or retry restore after fixing the underlying I/O error.",
            receipt.receipt_id
        ));
    }

    Ok(RestoreReport {
        artifact_id: id.clone(),
        op: receipt.op,
        relations_restored,
        relations_skipped,
        projection_restored: true,
        warnings: Vec::new(),
        restored_status: restored_status.to_string(),
    })
}

/// The `NewArtifact` a receipt's snapshot describes.
///
/// Tags are deliberately empty, matching the store path: the snapshot does
/// not capture them (they live in the projection frontmatter the trashed
/// file carries), and inventing them would claim a derivation that never
/// happened.
fn snapshot_artifact(receipt: &Receipt, status: &str) -> NewArtifact {
    NewArtifact {
        // ADR-030: the canonical key of a document is the slug its
        // frontmatter declares, and its display id when it declares none.
        canonical_key: canonical_key_from_document(
            &receipt.snapshot.id,
            &receipt.snapshot.body,
        ),
        id: receipt.snapshot.id.clone(),
        kind: receipt.snapshot.kind.clone(),
        status: status.to_string(),
        title: receipt.snapshot.title.clone(),
        body: receipt.snapshot.body.clone(),
        depth: receipt.snapshot.depth.clone(),
        author: receipt.snapshot.author.clone(),
        parent_epic: receipt.snapshot.parent_epic.clone(),
        valid_until: receipt.snapshot.valid_until.clone(),
        tags: Vec::new(),
    }
}

/// Move the trashed projection back without re-rendering it on either layout.
///
/// The destination is the document's canonical home: the file name the
/// receipt recorded (validated — only the name is taken, never the
/// directory), falling back to workspace + kind + id + title slug. A
/// missing trashed file returns `false`; the restore cannot then consume
/// its receipt.
async fn restore_projection_file(workspace: &Path, receipt: &Receipt) -> anyhow::Result<bool> {
    let trashed = Path::new(&receipt.trashed_projection);
    let safe_original = compute_safe_projection_path(workspace, &receipt.snapshot)?;
    if !trashed.exists() {
        // Nothing in trash for this receipt — a `Delete` whose move failed, or
        // a supersede/deprecate that never moved a file. Nothing to move.
        return Ok(false);
    }
    if let Some(parent) = safe_original.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    // A restore never overwrites a different canonical document: the
    // document that already holds the path is not the one the receipt
    // captured, and replacing it would destroy another artifact's identity.
    ensure_restore_target_is_free(receipt, &safe_original).await?;
    match tokio::fs::rename(trashed, &safe_original).await {
        Ok(()) => Ok(true),
        Err(e) => {
            // Cross-device fallback, identical to the store path.
            match tokio::fs::copy(trashed, &safe_original).await {
                Ok(_) => {
                    tokio::fs::remove_file(trashed).await?;
                    Ok(true)
                }
                Err(copy_err) => Err(anyhow::anyhow!(
                    "could not restore projection to {}: {} (rename: {})",
                    safe_original.display(),
                    copy_err,
                    e
                )),
            }
        }
    }
}

/// Re-render a receipt's snapshot into its canonical projection file.
///
/// Used when a status-change receipt's artifact is gone from the workspace
/// entirely: there is no trashed body to move back (supersede/deprecate never
/// trash one), so the canonical file is written from the captured fields.
/// Every value comes from the receipt; nothing is inferred.
async fn render_snapshot_projection(
    workspace: &Path,
    receipt: &Receipt,
    status: &str,
) -> anyhow::Result<()> {
    let snapshot = &receipt.snapshot;
    let rendered = crate::projection::render_projection_with_body(
        workspace,
        &snapshot.id,
        &snapshot.kind,
        &snapshot.title,
        status,
        &snapshot.depth,
        snapshot.author.as_deref(),
        snapshot.parent_epic.as_deref(),
        snapshot.valid_until.as_deref(),
        &snapshot.body,
        &[],
    )
    .await?;
    // The receipt records the document's canonical home. A document
    // published under its canonical key is restored to that path, so the
    // restored document keeps the key it was filed under.
    let target = compute_safe_projection_path(workspace, snapshot)?;
    if target != rendered {
        ensure_restore_target_is_free(receipt, &target).await?;
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::rename(&rendered, &target).await?;
    }
    Ok(())
}

/// Refuse to restore over a document that is not the one the receipt
/// captured.
///
/// The restore target may already exist: the same document may have been
/// recreated after the delete. Restoring over the *same* canonical document
/// is the point of an undo; restoring over a *different* one would destroy
/// another artifact's identity.
async fn ensure_restore_target_is_free(receipt: &Receipt, target: &Path) -> anyhow::Result<()> {
    if !target.exists() {
        return Ok(());
    }
    let existing = tokio::fs::read_to_string(target).await?;
    let existing_key = canonical_key_from_document(&receipt.snapshot.id, &existing);
    let receipt_key = canonical_key_from_document(&receipt.snapshot.id, &receipt.snapshot.body);
    if existing_key != receipt_key {
        anyhow::bail!(
            "refusing to restore '{}': {} already holds a different canonical document \
             ('{}' instead of '{}')",
            receipt.snapshot.id,
            target.display(),
            existing_key,
            receipt_key
        );
    }
    Ok(())
}

/// Canonical participants count even if the derived row is stale or absent.
/// A legacy-only row also counts, so a missing SOURCE file becomes a replay
/// error rather than silently discarding a known participant's relation.
async fn participant_exists(
    sink: Option<&LanceStore>,
    workspace: &Path,
    id: &str,
) -> anyhow::Result<bool> {
    if crate::artifact::store::resolve_artifact_path(workspace, id)
        .await
        .is_some()
    {
        return Ok(true);
    }
    match sink {
        Some(store) => Ok(store.get_record(id).await?.is_some()),
        None => Ok(false),
    }
}

/// Re-add an edge to its canonical SOURCE file, then the optional legacy sink.
///
/// Only the typed `LinkExists` error is a canonical no-op. Legacy duplicates
/// are confirmed by reading the existing triple, not by matching error text.
async fn replay_relation(
    sink: Option<&LanceStore>,
    workspace: &Path,
    source: &str,
    target: &str,
    relation: &str,
) -> anyhow::Result<()> {
    let path = crate::artifact::store::resolve_artifact_path(workspace, source)
        .await
        .ok_or_else(|| anyhow::anyhow!("source artifact '{source}' not found"))?;
    match crate::link::add_link_verbatim(&path, target, relation).await {
        Err(error)
            if matches!(
                error.downcast_ref::<crate::error::ForgeplanError>(),
                Some(crate::error::ForgeplanError::LinkExists { .. })
            ) => {}
        result => result?,
    }
    if let Some(store) = sink {
        let duplicate = store
            .get_relations(source)
            .await?
            .iter()
            .any(|(t, r)| t.eq_ignore_ascii_case(target) && r == relation);
        if !duplicate {
            store.add_relation(source, target, relation).await?;
        }
    }
    Ok(())
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
    use crate::artifact::types::ArtifactKind;
    let kind: ArtifactKind = snapshot
        .kind
        .parse()
        .map_err(|e| anyhow::anyhow!("receipt has unknown kind '{}': {e}", snapshot.kind))?;
    // The receipt records where the document lived. That file name is the
    // document's canonical home: a document published under its canonical
    // key is restored to that name, and a legacy document keeps the name it
    // had. Only the file NAME is taken from the receipt — never its
    // directory — and the name is validated, so a tampered receipt cannot
    // aim the restore outside the kind directory.
    let filename = recorded_file_name(snapshot).unwrap_or_else(|| {
        let slug = crate::projection::projection_slug(&snapshot.title);
        format!("{}-{}.md", snapshot.id, slug)
    });
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

/// The file name the receipt recorded for the document, when it is usable.
///
/// Only the name is taken — never the directory — and it must be a safe
/// canonical file name. A tampered receipt therefore cannot aim a restore
/// outside the kind directory.
fn recorded_file_name(snapshot: &super::ArtifactSnapshot) -> Option<String> {
    let recorded = Path::new(&snapshot.projection_path);
    let name = recorded.file_name()?.to_str()?.trim();
    if !name.ends_with(".md") {
        return None;
    }
    let stem = &name[..name.len() - 3];
    // The name must be one this build could have written.
    crate::artifact::store::canonical_filename(stem)
        .ok()
        .map(|_| name.to_string())
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

    async fn stage_deleted_projection(workspace: &Path, receipt: &mut Receipt) {
        render_snapshot_projection(workspace, receipt, &receipt.snapshot.status)
            .await
            .unwrap();
        let original = compute_safe_projection_path(workspace, &receipt.snapshot).unwrap();
        let trashed = trashed_projection_path(workspace, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::rename(&original, &trashed).await.unwrap();
        receipt.snapshot.projection_path = original.display().to_string();
        receipt.trashed_projection = trashed.display().to_string();
    }

    #[tokio::test]
    async fn restore_preserves_the_key_from_the_receipt() {
        let (_tmp, ws, store) = fresh_ws().await;
        let key = "prd-auth-system-k0123456789abcdef0123456789abcdef";
        let body = format!("---\nid: PRD-042\nkind: prd\nslug: {key}\n---\n\n# body\n");
        let mut receipt = build_receipt("PRD-042", DestructiveOp::Delete, &body);
        receipt.snapshot.slug = Some(key.to_string());
        let target = ws.join("prds").join(format!("{key}.md"));
        receipt.snapshot.projection_path = target.display().to_string();

        // The document is in the trash; its canonical path is free.
        let trashed = trashed_projection_path(&ws, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&trashed, &body).await.unwrap();
        receipt.trashed_projection = trashed.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore(&ws, &store, &receipt).await.unwrap();
        assert_eq!(report.artifact_id, "PRD-042");
        assert!(
            target.exists(),
            "the document must come back at its canonical path: {}",
            target.display()
        );
        let text = tokio::fs::read_to_string(&target).await.unwrap();
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
        assert_eq!(
            fm.get("slug").and_then(|value| value.as_str()),
            Some(key),
            "the restored document keeps the key the receipt captured"
        );
    }

    #[tokio::test]
    async fn restore_refuses_to_overwrite_a_different_canonical_document() {
        let (_tmp, ws, store) = fresh_ws().await;
        let key = "prd-auth-system-k0123456789abcdef0123456789abcdef";
        let body = format!("---\nid: PRD-042\nkind: prd\nslug: {key}\n---\n\n# body\n");
        let mut receipt = build_receipt("PRD-042", DestructiveOp::Delete, &body);
        receipt.snapshot.slug = Some(key.to_string());
        let target = ws.join("prds").join(format!("{key}.md"));
        receipt.snapshot.projection_path = target.display().to_string();

        // A DIFFERENT document already holds the canonical path.
        tokio::fs::create_dir_all(target.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(
            &target,
            "---\nid: PRD-099\nkind: prd\nslug: prd-other-k11111111111111111111111111111111\n---\n\n# other\n",
        )
        .await
        .unwrap();

        let trashed = trashed_projection_path(&ws, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&trashed, &body).await.unwrap();
        receipt.trashed_projection = trashed.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        let err = apply_restore(&ws, &store, &receipt)
            .await
            .expect_err("restore must refuse to overwrite a different document");
        assert!(
            err.to_string().contains("different canonical document"),
            "unexpected error: {err:#}"
        );
        let after = tokio::fs::read_to_string(&target).await.unwrap();
        assert!(
            after.contains("PRD-099"),
            "the other document must stay untouched"
        );
    }

    async fn assert_canonical_metadata(workspace: &Path, id: &str, status: &str, title: &str) {
        let path = crate::artifact::store::resolve_artifact_path(workspace, id)
            .await
            .expect("canonical artifact must exist");
        let text = tokio::fs::read_to_string(path).await.unwrap();
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
        assert_eq!(fm.get("id").and_then(|value| value.as_str()), Some(id));
        assert_eq!(
            fm.get("status").and_then(|value| value.as_str()),
            Some(status)
        );
        assert_eq!(
            fm.get("title").and_then(|value| value.as_str()),
            Some(title)
        );
    }

    #[tokio::test]
    async fn restore_delete_recreates_row() {
        let (_tmp, ws, store) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-001", DestructiveOp::Delete, "# body");
        stage_deleted_projection(&ws, &mut receipt).await;
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
        assert!(report.projection_restored);
        assert_canonical_metadata(&ws, "PRD-001", "active", "Title PRD-001").await;
        assert!(
            super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    #[tokio::test]
    async fn restore_refuses_if_id_collision_on_delete() {
        let (_tmp, ws, store) = fresh_ws().await;

        // Create PRD-001 currently in store (someone re-created it).
        let existing = NewArtifact {
            canonical_key: "PRD-001".into(),
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
                canonical_key: "PRD-002".into(),
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
        crate::projection::render_after_mutation(&ws, &store, "PRD-002")
            .await
            .unwrap();
        assert_canonical_metadata(&ws, "PRD-002", "deprecated", "To deprecate").await;

        // Receipt captured state BEFORE deprecate (status=active).
        let mut receipt = build_receipt("PRD-002", DestructiveOp::Deprecate, "b");
        receipt.snapshot.title = "To deprecate".into();
        receipt.reason = Some("testing".into());
        write_receipt(&ws, &receipt).await.unwrap();

        apply_restore(&ws, &store, &receipt).await.unwrap();

        let r = store.get_record("PRD-002").await.unwrap().unwrap();
        assert_eq!(r.status, "active");
        assert_canonical_metadata(&ws, "PRD-002", "active", "To deprecate").await;
        assert!(
            super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    #[tokio::test]
    async fn restore_marks_receipt_consumed() {
        let (_tmp, ws, store) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-003", DestructiveOp::Delete, "x");
        stage_deleted_projection(&ws, &mut receipt).await;
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
        stage_deleted_projection(&ws, &mut receipt).await;
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore(&ws, &store, &receipt).await.unwrap();
        assert_eq!(report.relations_restored, 0);
        assert_eq!(report.relations_skipped, vec!["EVID-999".to_string()]);
        assert!(report.warnings.is_empty());
        assert_canonical_metadata(&ws, "PRD-004", "active", "Title PRD-004").await;
        assert!(
            super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
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
        assert_eq!(
            tokio::fs::read(&original).await.unwrap(),
            b"# original body\n"
        );
        assert_eq!(
            store.get_record("PRD-005").await.unwrap().unwrap().status,
            "active"
        );
        assert!(
            super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
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
        // Remove the persisted receipt after preparing a real trashed
        // projection, so the failure occurs at the consumption barrier.
        let (_tmp, ws, store) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-MC", DestructiveOp::Delete, "body");
        stage_deleted_projection(&ws, &mut receipt).await;
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
        assert_canonical_metadata(&ws, "PRD-MC", "active", "Title PRD-MC").await;
        assert!(store.get_record("PRD-MC").await.unwrap().is_some());
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
                canonical_key: "PRD-CONFLICT".into(),
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

    // ─── Issue #291: target_status override + restored_status surface ─────

    /// Default behaviour (no override): restored_status equals captured
    /// snapshot.status. RestoreReport surfaces it.
    #[tokio::test]
    async fn restore_report_surfaces_captured_status() {
        let (_tmp, ws, store) = fresh_ws().await;
        store
            .create_artifact(&NewArtifact {
                canonical_key: "PRD-291A".into(),
                id: "PRD-291A".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "T".into(),
                body: "b".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        store
            .update_artifact("PRD-291A", Some("deprecated"), None)
            .await
            .unwrap();
        crate::projection::render_after_mutation(&ws, &store, "PRD-291A")
            .await
            .unwrap();
        let mut receipt = build_receipt("PRD-291A", DestructiveOp::Deprecate, "b");
        receipt.snapshot.status = "active".into();
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore(&ws, &store, &receipt).await.unwrap();
        assert_eq!(report.restored_status, "active");
        assert_canonical_metadata(&ws, "PRD-291A", "active", "Title PRD-291A").await;
    }

    /// `apply_restore_with_target` explicit override wins over snapshot
    /// when the override is FSM-valid from the current state.
    ///
    /// Audit-dogfood SEC-2: override now respects lifecycle FSM. This
    /// test uses `draft → active` (a legal transition); the negative
    /// case (forbidden transition) is pinned in
    /// `restore_with_target_rejects_fsm_invalid_transition` below.
    #[tokio::test]
    async fn restore_with_target_override_applies_explicit_status() {
        let (_tmp, ws, store) = fresh_ws().await;
        store
            .create_artifact(&NewArtifact {
                canonical_key: "PRD-291B".into(),
                id: "PRD-291B".into(),
                kind: "prd".into(),
                status: "draft".into(),
                title: "T".into(),
                body: "b".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        crate::projection::render_after_mutation(&ws, &store, "PRD-291B")
            .await
            .unwrap();
        // Receipt captured active (the prior_status before some earlier op);
        // current row is draft (a valid `draft → active` transition).
        let mut receipt = build_receipt("PRD-291B", DestructiveOp::Deprecate, "b");
        receipt.snapshot.status = "draft".into();
        write_receipt(&ws, &receipt).await.unwrap();

        // Override to `active` — FSM-valid from `draft`.
        let report = apply_restore_with_target(&ws, Some(&store), &receipt, Some("active"))
            .await
            .unwrap();
        assert_eq!(report.restored_status, "active", "override wins");

        let r = store.get_record("PRD-291B").await.unwrap().unwrap();
        assert_eq!(r.status, "active", "store reflects override");
        assert_canonical_metadata(&ws, "PRD-291B", "active", "Title PRD-291B").await;
        let persisted = super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
            .await
            .unwrap();
        assert_eq!(persisted.snapshot.status, "draft");
        assert!(persisted.consumed);
    }

    /// Audit-dogfood SEC-2: target_status override that violates the
    /// lifecycle FSM is rejected before any mutation. Example: trying
    /// to restore a `deprecated` artifact to `draft` — that's not a
    /// legal forward transition. Operator must use the canonical
    /// lifecycle commands.
    #[tokio::test]
    async fn restore_with_target_rejects_fsm_invalid_transition() {
        let (_tmp, ws, store) = fresh_ws().await;
        store
            .create_artifact(&NewArtifact {
                canonical_key: "PRD-291E".into(),
                id: "PRD-291E".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "T".into(),
                body: "b".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        store
            .update_artifact("PRD-291E", Some("deprecated"), None)
            .await
            .unwrap();
        crate::projection::render_after_mutation(&ws, &store, "PRD-291E")
            .await
            .unwrap();
        let mut receipt = build_receipt("PRD-291E", DestructiveOp::Deprecate, "b");
        receipt.snapshot.status = "active".into();
        write_receipt(&ws, &receipt).await.unwrap();

        // `deprecated → draft` is FSM-invalid — must be rejected.
        let err = apply_restore_with_target(&ws, Some(&store), &receipt, Some("draft"))
            .await
            .expect_err("FSM should reject deprecated->draft override");
        assert!(
            err.to_string().contains("FSM"),
            "rejection must cite FSM: {err}"
        );
        // Store still deprecated — no partial mutation.
        let r = store.get_record("PRD-291E").await.unwrap().unwrap();
        assert_eq!(r.status, "deprecated", "no partial mutation on FSM reject");
        assert_canonical_metadata(&ws, "PRD-291E", "deprecated", "T").await;
        assert!(
            !super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    /// Invalid target_status value is rejected BEFORE mutation.
    #[tokio::test]
    async fn restore_with_target_rejects_invalid_status() {
        let (_tmp, ws, store) = fresh_ws().await;
        store
            .create_artifact(&NewArtifact {
                canonical_key: "PRD-291C".into(),
                id: "PRD-291C".into(),
                kind: "prd".into(),
                status: "deprecated".into(),
                title: "T".into(),
                body: "b".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        crate::projection::render_after_mutation(&ws, &store, "PRD-291C")
            .await
            .unwrap();
        let mut receipt = build_receipt("PRD-291C", DestructiveOp::Deprecate, "b");
        receipt.snapshot.status = "active".into();
        write_receipt(&ws, &receipt).await.unwrap();

        let err = apply_restore_with_target(&ws, Some(&store), &receipt, Some("bogus"))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("Invalid target_status"),
            "bad override must error: {err}"
        );
        // Store unchanged — error happened before mutation.
        let r = store.get_record("PRD-291C").await.unwrap().unwrap();
        assert_eq!(r.status, "deprecated", "no partial restore on bad input");
        assert_canonical_metadata(&ws, "PRD-291C", "deprecated", "T").await;
        assert!(
            !super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    /// `None` override = legacy behaviour identical to `apply_restore`.
    #[tokio::test]
    async fn restore_with_target_none_matches_legacy() {
        let (_tmp, ws, store) = fresh_ws().await;
        store
            .create_artifact(&NewArtifact {
                canonical_key: "PRD-291D".into(),
                id: "PRD-291D".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "T".into(),
                body: "b".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        store
            .update_artifact("PRD-291D", Some("deprecated"), None)
            .await
            .unwrap();
        crate::projection::render_after_mutation(&ws, &store, "PRD-291D")
            .await
            .unwrap();
        let mut receipt = build_receipt("PRD-291D", DestructiveOp::Deprecate, "b");
        receipt.snapshot.status = "active".into();
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore_with_target(&ws, Some(&store), &receipt, None)
            .await
            .unwrap();
        assert_eq!(
            report.restored_status, "active",
            "None override uses captured snapshot.status"
        );
        assert_canonical_metadata(&ws, "PRD-291D", "active", "Title PRD-291D").await;
    }

    #[tokio::test]
    async fn legacy_delete_restore_preserves_bytes_and_replays_canonical_relations() {
        let (_tmp, ws, store) = fresh_ws().await;
        let target = build_receipt("PRD-412", DestructiveOp::Deprecate, "Target body.\n");
        let incoming = build_receipt("PRD-413", DestructiveOp::Deprecate, "Incoming body.\n");
        for participant in [&target, &incoming] {
            render_snapshot_projection(&ws, participant, "active")
                .await
                .unwrap();
            store
                .create_artifact(&snapshot_artifact(participant, "active"))
                .await
                .unwrap();
        }
        let target_path = compute_safe_projection_path(&ws, &target.snapshot).unwrap();
        let target_bytes = tokio::fs::read(&target_path).await.unwrap();
        let mut receipt = build_receipt("PRD-411", DestructiveOp::Delete, "Original body.\n");
        receipt.snapshot.relations = vec![
            CapturedRelation {
                from: "PRD-411".into(),
                to: "PRD-412".into(),
                relation: "based_on".into(),
                direction: RelationDirection::Outgoing,
            },
            CapturedRelation {
                from: "PRD-413".into(),
                to: "PRD-411".into(),
                relation: "informs".into(),
                direction: RelationDirection::Incoming,
            },
        ];
        // A surviving derived triple must be an exact duplicate no-op, not a
        // swallowed relation error. Its missing canonical link still needs replay.
        store
            .add_relation("PRD-413", "PRD-411", "informs")
            .await
            .unwrap();
        let original = "---\n# Keep operator formatting and fields not captured in the receipt.\nid: PRD-411\nkind: prd\nstatus: active\ntitle: 'Title PRD-411'\ndepth: standard\ntags: [kept]\ncustom: 'unchanged'\nlinks:\n  - target: PRD-412\n    relation: based_on\n---\n\nOriginal body.\n";
        let trashed = trashed_projection_path(&ws, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&trashed, original).await.unwrap();
        receipt.trashed_projection = trashed.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore(&ws, &store, &receipt).await.unwrap();
        let restored_path = compute_safe_projection_path(&ws, &receipt.snapshot).unwrap();
        assert_eq!(
            tokio::fs::read(&restored_path).await.unwrap(),
            original.as_bytes()
        );
        assert!(!trashed.exists());
        assert!(report.projection_restored);
        assert_eq!(report.relations_restored, 2);
        assert!(report.relations_skipped.is_empty());
        assert!(report.warnings.is_empty());
        assert_canonical_metadata(&ws, "PRD-411", "active", "Title PRD-411").await;
        let incoming_path = compute_safe_projection_path(&ws, &incoming.snapshot).unwrap();
        let text = tokio::fs::read_to_string(incoming_path).await.unwrap();
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
        assert_eq!(
            crate::link::list_links(&fm),
            vec![("PRD-411".to_string(), "informs".to_string())]
        );
        assert_eq!(
            store.get_relations("PRD-411").await.unwrap(),
            vec![("PRD-412".to_string(), "based_on".to_string())]
        );
        assert_eq!(
            store.get_relations("PRD-413").await.unwrap(),
            vec![("PRD-411".to_string(), "informs".to_string())]
        );
        assert_eq!(
            store.get_record("PRD-411").await.unwrap().unwrap().status,
            "active"
        );
        assert_eq!(tokio::fs::read(target_path).await.unwrap(), target_bytes);
        assert!(
            super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    #[tokio::test]
    async fn legacy_deleted_artifact_restore_applies_explicit_status() {
        let (_tmp, ws, store) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-441", DestructiveOp::Delete, "Original body.\n");
        stage_deleted_projection(&ws, &mut receipt).await;
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore_with_target(&ws, Some(&store), &receipt, Some(" draft "))
            .await
            .unwrap();

        assert_canonical_metadata(&ws, "PRD-441", "draft", "Title PRD-441").await;
        assert_eq!(
            store.get_record("PRD-441").await.unwrap().unwrap().status,
            "draft"
        );
        assert_eq!(report.restored_status, "draft");
        let persisted = super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
            .await
            .unwrap();
        assert_eq!(persisted.snapshot.status, "active");
        assert!(persisted.consumed);
        assert!(!Path::new(&receipt.trashed_projection).exists());
    }

    #[tokio::test]
    async fn restore_supersede_reverses_title_rename_and_edge_on_both_layouts() {
        for legacy in [false, true] {
            let tmp = TempDir::new().unwrap();
            let ws = workspace::init_workspace(tmp.path(), "restore-renamed-supersede").unwrap();
            let sink = if legacy {
                Some(LanceStore::init(&ws).await.unwrap())
            } else {
                None
            };
            let mut receipt = build_receipt("PRD-421", DestructiveOp::Supersede, "Source body.\n");
            let replacement =
                build_receipt("PRD-422", DestructiveOp::Deprecate, "Replacement body.\n");
            for participant in [&receipt, &replacement] {
                render_snapshot_projection(&ws, participant, "active")
                    .await
                    .unwrap();
                if let Some(store) = sink.as_ref() {
                    store
                        .create_artifact(&snapshot_artifact(participant, "active"))
                        .await
                        .unwrap();
                }
            }
            let original_path = compute_safe_projection_path(&ws, &receipt.snapshot).unwrap();
            let replacement_path =
                compute_safe_projection_path(&ws, &replacement.snapshot).unwrap();
            receipt.replacement = Some("PRD-422".into());
            write_receipt(&ws, &receipt).await.unwrap();
            let guard = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
            let ctx = crate::projection::MutationContext::new(&ws, &guard, sink.as_ref());
            let read = crate::constellation::facade::open_workspace_read(
                &ws,
                None,
                crate::constellation::capture::CaptureLimits::default(),
            )
            .await
            .unwrap();
            crate::lifecycle::canonical::supersede_captured(&ctx, &read, "PRD-421", "PRD-422")
                .await
                .unwrap();
            crate::projection::update_metadata_with_projection(
                &ctx,
                "PRD-421",
                None,
                Some("Renamed after supersede"),
            )
            .await
            .unwrap();
            let renamed_path = crate::artifact::store::resolve_artifact_path(&ws, "PRD-421")
                .await
                .unwrap();
            assert_ne!(renamed_path, original_path);
            assert!(!original_path.exists());
            let superseded_text = tokio::fs::read_to_string(&renamed_path).await.unwrap();
            let (fm, _) =
                crate::artifact::frontmatter::parse_frontmatter(&superseded_text).unwrap();
            assert_eq!(
                fm.get("status").and_then(|value| value.as_str()),
                Some("superseded")
            );
            assert!(
                crate::link::list_links(&fm)
                    .contains(&("PRD-422".to_string(), "supersedes".to_string()))
            );
            let replacement_bytes = tokio::fs::read(&replacement_path).await.unwrap();

            let report = match sink.as_ref() {
                Some(store) => apply_restore(&ws, store, &receipt).await.unwrap(),
                None => apply_restore_file_first(&ws, &receipt).await.unwrap(),
            };

            let restored_path = crate::artifact::store::resolve_artifact_path(&ws, "PRD-421")
                .await
                .unwrap();
            assert_eq!(restored_path, original_path);
            assert!(!renamed_path.exists());
            assert_canonical_metadata(&ws, "PRD-421", "active", "Title PRD-421").await;
            let text = tokio::fs::read_to_string(restored_path).await.unwrap();
            let (fm, body) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
            assert!(crate::link::list_links(&fm).is_empty());
            assert_eq!(body.trim(), "Source body.");
            assert!(report.projection_restored);
            assert!(report.warnings.is_empty());
            assert_eq!(report.restored_status, "active");
            if let Some(store) = sink.as_ref() {
                let record = store.get_record("PRD-421").await.unwrap().unwrap();
                assert_eq!(record.status, "active");
                assert_eq!(record.title, "Title PRD-421");
                assert!(store.get_relations("PRD-421").await.unwrap().is_empty());
            } else {
                assert!(!ws.join("lance/artifacts.lance").exists());
            }
            assert_eq!(
                tokio::fs::read(replacement_path).await.unwrap(),
                replacement_bytes
            );
            assert!(
                super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                    .await
                    .unwrap()
                    .consumed
            );
        }
    }

    #[tokio::test]
    async fn restore_relation_write_failure_keeps_receipt_retryable_on_both_layouts() {
        for legacy in [false, true] {
            let tmp = TempDir::new().unwrap();
            let ws = workspace::init_workspace(tmp.path(), "restore-relation-failure").unwrap();
            let sink = if legacy {
                Some(LanceStore::init(&ws).await.unwrap())
            } else {
                None
            };
            let mut receipt =
                build_receipt("PRD-431", DestructiveOp::Deprecate, "Restored body.\n");
            let incoming = build_receipt("PRD-432", DestructiveOp::Deprecate, "Incoming body.\n");
            render_snapshot_projection(&ws, &receipt, "deprecated")
                .await
                .unwrap();
            let source_path = compute_safe_projection_path(&ws, &incoming.snapshot).unwrap();
            // This participant exists, but the real canonical link writer
            // cannot append to its malformed links field. It must not count
            // as an orphan, duplicate, or successful recovery.
            let malformed = "---\nid: PRD-432\nkind: prd\nstatus: active\ntitle: Title PRD-432\nlinks: not-a-sequence\n---\n\nIncoming body.\n";
            tokio::fs::write(&source_path, malformed).await.unwrap();
            if let Some(store) = sink.as_ref() {
                store
                    .create_artifact(&snapshot_artifact(&receipt, "deprecated"))
                    .await
                    .unwrap();
                store
                    .create_artifact(&snapshot_artifact(&incoming, "active"))
                    .await
                    .unwrap();
            }
            receipt.snapshot.relations.push(CapturedRelation {
                from: "PRD-432".into(),
                to: "PRD-431".into(),
                relation: "informs".into(),
                direction: RelationDirection::Incoming,
            });
            write_receipt(&ws, &receipt).await.unwrap();
            let receipt_file = receipt_path(&ws, &receipt.receipt_id);
            let receipt_bytes = tokio::fs::read(&receipt_file).await.unwrap();

            let error = apply_restore_with_target(&ws, sink.as_ref(), &receipt, None)
                .await
                .expect_err("a failed incoming relation write cannot consume the receipt");

            assert!(matches!(
                error.downcast_ref::<crate::error::ForgeplanError>(),
                Some(crate::error::ForgeplanError::Frontmatter(_))
            ));
            assert_canonical_metadata(&ws, "PRD-431", "active", "Title PRD-431").await;
            assert_eq!(
                tokio::fs::read(&source_path).await.unwrap(),
                malformed.as_bytes()
            );
            assert_eq!(tokio::fs::read(&receipt_file).await.unwrap(), receipt_bytes);
            assert!(
                !super::super::read_receipt(&receipt_file)
                    .await
                    .unwrap()
                    .consumed
            );
            if let Some(store) = sink.as_ref() {
                assert_eq!(
                    store.get_record("PRD-431").await.unwrap().unwrap().status,
                    "active"
                );
                assert!(store.get_relations("PRD-432").await.unwrap().is_empty());
            }

            render_snapshot_projection(&ws, &incoming, "active")
                .await
                .unwrap();
            let report = apply_restore_with_target(&ws, sink.as_ref(), &receipt, None)
                .await
                .unwrap();
            let text = tokio::fs::read_to_string(&source_path).await.unwrap();
            let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
            assert_eq!(
                crate::link::list_links(&fm),
                vec![("PRD-431".to_string(), "informs".to_string())]
            );
            assert_eq!(report.relations_restored, 1);
            assert!(report.relations_skipped.is_empty());
            assert!(report.warnings.is_empty());
            assert!(
                super::super::read_receipt(&receipt_file)
                    .await
                    .unwrap()
                    .consumed
            );
            if let Some(store) = sink.as_ref() {
                assert_eq!(
                    store.get_relations("PRD-432").await.unwrap(),
                    vec![("PRD-431".to_string(), "informs".to_string())]
                );
            } else {
                assert!(!ws.join("lance/artifacts.lance").exists());
            }
        }
    }

    #[tokio::test]
    async fn restore_delete_without_trash_keeps_receipt_unconsumed_on_both_layouts() {
        for legacy in [false, true] {
            let tmp = TempDir::new().unwrap();
            let ws = workspace::init_workspace(tmp.path(), "restore-without-trash").unwrap();
            let sink = if legacy {
                Some(LanceStore::init(&ws).await.unwrap())
            } else {
                None
            };
            let receipt = build_receipt("PRD-451", DestructiveOp::Delete, "Body without trash.\n");
            write_receipt(&ws, &receipt).await.unwrap();

            let error = apply_restore_with_target(&ws, sink.as_ref(), &receipt, None)
                .await
                .unwrap_err();

            assert!(error.to_string().contains("No trashed projection"));
            assert!(
                crate::artifact::store::resolve_artifact_path(&ws, "PRD-451")
                    .await
                    .is_none()
            );
            assert!(
                !super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                    .await
                    .unwrap()
                    .consumed
            );
            if let Some(store) = sink.as_ref() {
                assert!(store.get_record("PRD-451").await.unwrap().is_none());
            } else {
                assert!(!ws.join("lance/artifacts.lance").exists());
            }
        }
    }

    #[tokio::test]
    async fn legacy_restore_refuses_canonical_collision_even_without_a_derived_row() {
        let (_tmp, ws, store) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-461", DestructiveOp::Delete, "Original body.\n");
        stage_deleted_projection(&ws, &mut receipt).await;
        let mut collider = build_receipt("PRD-461", DestructiveOp::Deprecate, "Current body.\n");
        collider.snapshot.title = "Current canonical file".into();
        render_snapshot_projection(&ws, &collider, "active")
            .await
            .unwrap();
        let collider_path = compute_safe_projection_path(&ws, &collider.snapshot).unwrap();
        let current_bytes = tokio::fs::read(&collider_path).await.unwrap();
        write_receipt(&ws, &receipt).await.unwrap();
        assert!(store.get_record("PRD-461").await.unwrap().is_none());

        let error = apply_restore(&ws, &store, &receipt).await.unwrap_err();

        assert!(error.to_string().contains("already exists"));
        assert_eq!(tokio::fs::read(collider_path).await.unwrap(), current_bytes);
        assert!(Path::new(&receipt.trashed_projection).exists());
        assert!(store.get_record("PRD-461").await.unwrap().is_none());
        assert!(
            !super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    #[tokio::test]
    async fn canonical_delete_restore_preserves_bytes_and_existing_outgoing_edge() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace::init_workspace(tmp.path(), "canonical-restore").unwrap();
        let mut receipt = build_receipt("PRD-401", DestructiveOp::Delete, "Original body.\n");
        let original = "---\nid: PRD-401\nkind: prd\nstatus: active\ntitle: Title PRD-401\ndepth: standard\nlinks:\n  - target: EVID-401\n    relation: based_on\n---\n\nOriginal body.\n";
        tokio::fs::write(
            ws.join("evidence/EVID-401-target.md"),
            "---\nid: EVID-401\nkind: evidence\nstatus: active\ntitle: Target\n---\n\nEvidence.\n",
        )
        .await
        .unwrap();
        receipt.snapshot.relations.push(CapturedRelation {
            from: "PRD-401".into(),
            to: "EVID-401".into(),
            relation: "based_on".into(),
            direction: RelationDirection::Outgoing,
        });
        let trashed = trashed_projection_path(&ws, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&trashed, original).await.unwrap();
        receipt.trashed_projection = trashed.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore_with_target(&ws, None, &receipt, None)
            .await
            .unwrap();
        let restored = crate::artifact::store::resolve_artifact_path(&ws, "PRD-401")
            .await
            .unwrap();
        assert_eq!(tokio::fs::read_to_string(restored).await.unwrap(), original);
        assert_eq!(report.relations_restored, 1);
        assert!(report.relations_skipped.is_empty());
        assert!(report.warnings.is_empty());
        assert!(
            super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
        assert!(!ws.join("lance/artifacts.lance").exists());
    }

    #[tokio::test]
    async fn canonical_deleted_artifact_restore_applies_explicit_status() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace::init_workspace(tmp.path(), "canonical-override").unwrap();
        let mut receipt = build_receipt("PRD-402", DestructiveOp::Delete, "Original body.\n");
        let trashed = trashed_projection_path(&ws, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(
            &trashed,
            "---\nid: PRD-402\nkind: prd\nstatus: active\ntitle: Title PRD-402\n---\n\nOriginal body.\n",
        )
        .await
        .unwrap();
        receipt.trashed_projection = trashed.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore_with_target(&ws, None, &receipt, Some(" draft "))
            .await
            .unwrap();
        let restored = crate::artifact::store::resolve_artifact_path(&ws, "PRD-402")
            .await
            .unwrap();
        let text = tokio::fs::read_to_string(restored).await.unwrap();
        let (fm, body) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
        assert_eq!(
            fm.get("status").and_then(|value| value.as_str()),
            Some("draft")
        );
        assert_eq!(body.trim(), "Original body.");
        assert_eq!(report.restored_status, "draft");
        assert!(!ws.join("lance/artifacts.lance").exists());
    }
}
