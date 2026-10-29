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
    let (record, all_relations) = match &store {
        Some(store) => {
            let id = store
                .resolve_id(id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;
            let record = store.get_record(&id).await?.ok_or_else(|| {
                anyhow::anyhow!(
                    "Artifact '{}' not found
Fix: forgeplan list",
                    id
                )
            })?;
            (record, store.get_all_relations().await?)
        }
        None => {
            let read = common::workspace_read(&ws).await?;
            let canonical = read
                .resolve_id(id)
                .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;
            let record = read.get_record(&canonical).cloned().ok_or_else(|| {
                anyhow::anyhow!(
                    "Artifact '{}' not found
Fix: forgeplan list",
                    canonical
                )
            })?;
            (record, read.get_all_relations())
        }
    };
    let id = record.id.as_str();

    // Check for dependents (other artifacts linking TO this one)
    let dependents: Vec<_> = all_relations
        .iter()
        .filter(|(_, target, _)| target.eq_ignore_ascii_case(id))
        .collect();

    if !dependents.is_empty() {
        eprintln!("  WARNING: {} has {} dependent(s):", id, dependents.len());
        for (source, _, rel) in &dependents {
            eprintln!("    {} --{}--> {}", source, rel, id);
        }
        if !yes {
            anyhow::bail!(
                "{} has {} dependent(s). Use --yes to confirm deletion despite dependents.",
                id,
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

    // Cascade: delete all relations involving this artifact.
    // Count from already-fetched data to avoid double table scan.
    let relation_count = all_relations
        .iter()
        .filter(|(s, t, _)| s.eq_ignore_ascii_case(id) || t.eq_ignore_ascii_case(id))
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
    let captured_relations = capture_relations(&record, &all_relations);
    let receipt_id = match &store {
        Some(store) => {
            undo::soft_delete_capture(&ws, store, &record, undo::DestructiveOp::Delete, None, None)
                .await?
        }
        None => {
            undo::soft_delete_capture_with_relations(
                &ws,
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
        id,
    )
    .await?;

    if store.is_none() {
        common::note_constellation_owns_index();
        // Lock released before reconcile — the pass re-acquires it per storage
        // commit, and a held guard would turn the capture into a 30 s timeout.
        drop(lock);
        common::reconcile_after_file_mutation(&ws).await;
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
