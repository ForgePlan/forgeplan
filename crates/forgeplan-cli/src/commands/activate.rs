use forgeplan_core::lifecycle;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(id: &str, force: bool) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    // Capture old status before transition
    let old_status = store
        .get_record(id)
        .await?
        .map(|r| r.status)
        .unwrap_or_else(|| "draft".to_string());

    // Sync file→LanceDB before lifecycle call (preserve user edits)
    if let Some(record) = store.get_record(id).await? {
        projection::sync_file_to_store(&store, &ws, &record).await?;
    }

    let result = lifecycle::activate(&store, id, force).await?;

    // Re-render projection with updated status
    if let Some(record) = store.get_record(id).await? {
        let links = store.get_relations(id).await.unwrap_or_default();
        projection::render_projection(
            &ws,
            &record.id,
            &record.kind,
            &record.title,
            &record.status,
            &record.depth,
            record.author.as_deref(),
            record.parent_epic.as_deref(),
            record.valid_until.as_deref(),
            &record.body,
            &links,
        )
        .await?;
    }

    common::log_change_field(
        &store,
        id,
        "update",
        "status",
        Some(&old_status),
        Some("active"),
        "cli",
    )
    .await;

    if result.forced {
        println!("  Activated {id} ({old_status} → active)");
        println!(
            "  Warning: Activated with {} validation error{}",
            result.must_errors.len(),
            if result.must_errors.len() == 1 {
                ""
            } else {
                "s"
            }
        );
    } else {
        println!("  Activated {id} ({old_status} → active)");
    }

    // Hints: suggest evidence if not linked
    if let Some(record) = store.get_record(id).await? {
        let rels = store.get_relations(id).await.unwrap_or_default();
        let incoming = store.get_incoming_relations(id).await.unwrap_or_default();
        let has_evidence = rels
            .iter()
            .chain(incoming.iter())
            .any(|(t, _)| t.to_uppercase().starts_with("EVID-"));
        let kind: forgeplan_core::artifact::types::ArtifactKind = record
            .kind
            .parse()
            .unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
        let hints = forgeplan_core::hints::activate_hints(true, has_evidence, &kind);
        if !hints.is_empty() {
            print!("{}", forgeplan_core::hints::format_hints(&hints));
        }
    }

    Ok(())
}
