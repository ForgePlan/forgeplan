use forgeplan_core::db::store::ArtifactRecord;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::projection;
use forgeplan_core::undo;

use crate::commands::common;

pub async fn run(id: &str, yes: bool) -> anyhow::Result<()> {
    let (ws, lock, store) = common::open_store_locked_tolerating_constellation().await?;

    // PROB-060 / SPEC-005 Phase 2.6 (CD-6) — accept slug or display id.
    //
    // The migrated branch resolves through a fresh capture rather than the
    // legacy store: the canonical markdown is the source of truth there, and
    // the derived row is whatever the last reconcile published — which is
    // behind the bytes by definition after any hand edit.
    let (record, canonical, all_relations) = match &store {
        Some(store) => {
            let resolution = common::resolve_against_workspace(&ws, id).await?;
            let (canonical, _) = common::require_found_identity(id, &resolution)?;
            let record = store.get_record(&lock, &canonical).await?.ok_or_else(|| {
                anyhow::anyhow!(
                    "Artifact '{}' not found
Fix: forgeplan list",
                    canonical
                )
            })?;
            (record, canonical, store.get_all_relations(&lock).await?)
        }
        None => {
            let read = forgeplan_core::constellation::facade::open_workspace_read_with_permit(&ws, &lock).await?;
            let resolution = common::resolve_against_capture(&read, id);
            let (canonical, _) = common::require_found_identity(id, &resolution)?;
            let record = read.get_record(&canonical).cloned().ok_or_else(|| {
                anyhow::anyhow!(
                    "Artifact '{}' not found
Fix: forgeplan list",
                    canonical
                )
            })?;
            (record, canonical, read.get_all_relations())
        }
    };
    // ADR-030: the destructive work is addressed by the canonical key the
    // resolver just returned. Re-resolving the display alias after the receipt
    // has already moved this document's file would hand the next step a
    // selector whose only remaining claimant is the SIBLING that legitimately
    // shares the alias — deleting a document the operator never named.
    let target = canonical.as_str();
    let display = record.id.as_str();

    // Check for dependents (other artifacts linking TO this one). Relation
    // endpoints are canonical keys (ADR-030), so the test is taken in that
    // space; the warning still prints the display alias a human recognises.
    let dependents: Vec<_> = all_relations
        .iter()
        .filter(|(_, endpoint, _)| endpoint.eq_ignore_ascii_case(target))
        .collect();

    if !dependents.is_empty() {
        eprintln!(
            "  WARNING: {} has {} dependent(s):",
            display,
            dependents.len()
        );
        for (source, _, rel) in &dependents {
            eprintln!("    {} --{}--> {}", source, rel, display);
        }
        if !yes {
            anyhow::bail!(
                "{} has {} dependent(s). Use --yes to confirm deletion despite dependents.",
                display,
                dependents.len()
            );
        }
        eprintln!("  Proceeding with --yes despite dependents.");
    }

    if !yes {
        anyhow::bail!(
            "About to delete {} \"{}\". This cannot be undone. Use --yes to confirm.",
            record.id,
            record.title
        );
    }
    // Confirmation and selector refusal happen before opening the index for
    // writing. Re-read the normalized v5 record and edges after preparation.
    drop(store);
    let store = common::prepare_store_locked_checked(&ws, &lock, &[(target, true)]).await?;
    let (record, all_relations) = if let Some(store) = &store {
        let record = store.get_record(&lock, target).await?
            .ok_or_else(|| anyhow::anyhow!("Artifact '{target}' disappeared during write preparation"))?;
        (record, store.get_all_relations(&lock).await?)
    } else {
        (record, all_relations)
    };

    // Cascade: delete all relations involving this artifact.
    // Count from already-fetched data to avoid double table scan.
    let relation_count = all_relations
        .iter()
        .filter(|(s, t, _)| s.eq_ignore_ascii_case(target) || t.eq_ignore_ascii_case(target))
        .count();

    // PRD-055 + audit follow-up 2026-05-01: capture soft-delete receipt
    // BEFORE mutating the store. Brings CLI into parity with MCP
    // `forgeplan_delete` which has had soft-delete since release.
    // Receipt holds full snapshot + relations so `forgeplan undo-last`
    // and `forgeplan restore <id>` recover the artifact end-to-end.
    //
    // The relations come from whichever read source produced `record`, so a
    // migrated workspace (where `artifacts.lance` is retired and must not be
    // opened) gets byte-identical receipts instead of no receipts at all.
    common::ensure_ordinary_mutation_allowed(&ws)?;
    let captured_relations = capture_relations(&record, &all_relations);
    let receipt_id = match &store {
        Some(store) => {
            undo::soft_delete_capture(&ws, &lock, store, &record, undo::DestructiveOp::Delete, None, None)
                .await
                .map_err(|e| common::describe_possible_legacy_partial_commit(true, e.into()))?
        }
        None => {
            undo::soft_delete_capture_with_relations(
                &ws,
                &lock,
                None,
                &record,
                captured_relations,
                undo::DestructiveOp::Delete,
                None,
                None,
            )
            .await?
        }
    };

    // PRD-073 file-first: helper removes the projection file (likely no-op
    // because soft_delete_capture already moved it to trash) then cascades
    // relations and the LanceDB row. Failure mid-flow is recoverable via
    // `forgeplan restore <id>` from the receipt above.
    //
    // With a `None` sink the same helper removes the canonical file and
    // stops there — the derived row retires on the reconcile pass below.
    projection::delete_artifact_with_projection(
        &projection::MutationContext::new(&ws, &lock, store.as_ref()),
        target,
    )
    .await
    .map_err(|e| common::describe_possible_legacy_partial_commit(store.is_some(), e.into()))?;

    if store.is_none() {
        common::note_constellation_owns_index();
        // Lock released before reconcile — the pass re-acquires it per storage
        // commit, and a held guard would turn the capture into a 30 s timeout.
        drop(lock);
        common::reconcile_after_file_mutation(&ws).await;
    } else {
        drop(store);
        drop(lock);
        common::drain_legacy_semantic_after_mutation(&ws).await?;
    }

    if relation_count > 0 {
        eprintln!("  Removed {} relation(s) involving {}", relation_count, id);
    }

    println!(
        "  Deleted: {} \"{}\" (receipt {receipt_id})",
        record.id, record.title
    );

    // Soft-deleted: surface the recovery path.
    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — slug pre-merge / display
    // id post-merge so the restore command keeps the canonical reference
    // form (e.g. `forgeplan restore prd-auth-system` pre-merge).
    let ref_form =
        forgeplan_core::artifact::frontmatter::refs_form_from_body(&record.body, &record.id);
    let hint_list = vec![
        Hint::info(format!(
            "Recoverable via `forgeplan restore {ref_form}` (within 30 days)"
        ))
        .with_action(format!("forgeplan restore {ref_form}")),
    ];
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}

/// Edges touching `record`, in the shape the soft-delete receipt stores.
///
/// Built from the triple set the caller already holds, so both layouts
/// capture exactly the same relations — the receipt is a recovery artifact,
/// and a receipt that silently omitted half the edges would restore an
/// artifact that no longer knows what it was linked to.
fn capture_relations(
    record: &ArtifactRecord,
    all_relations: &[(String, String, String)],
) -> Vec<undo::CapturedRelation> {
    use undo::{CapturedRelation, RelationDirection};

    let mut captured = Vec::new();
    for (source, target, relation) in all_relations {
        // The relation endpoints are canonical keys (ADR-030); the receipt
        // stores the endpoints it finds, and the artifact it belongs to is
        // named by its key.
        if source.eq_ignore_ascii_case(&record.canonical_key) {
            captured.push(CapturedRelation {
                from: record.canonical_key.clone(),
                to: target.clone(),
                relation: relation.clone(),
                direction: RelationDirection::Outgoing,
            });
        }
        if target.eq_ignore_ascii_case(&record.canonical_key) {
            captured.push(CapturedRelation {
                from: source.clone(),
                to: record.canonical_key.clone(),
                relation: relation.clone(),
                direction: RelationDirection::Incoming,
            });
        }
    }
    captured
}
