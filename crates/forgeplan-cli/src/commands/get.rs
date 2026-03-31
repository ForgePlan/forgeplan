use crate::commands::common;
use crate::ui;

pub async fn run(id: &str, json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let record = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", id))?;

    if json {
        let json_data = serde_json::json!({
            "id": record.id,
            "kind": record.kind,
            "status": record.status,
            "title": record.title,
            "depth": record.depth,
            "author": record.author,
            "parent_epic": record.parent_epic,
            "valid_until": record.valid_until,
            "r_eff": record.r_eff_score,
            "created_at": record.created_at,
            "updated_at": record.updated_at,
            "body": record.body,
        });
        println!("{}", serde_json::to_string_pretty(&json_data)?);
        return Ok(());
    }

    ui::header(&record.id, &record.title);
    ui::kv("Kind", &record.kind);
    ui::kv("Status", &ui::styled_status(&record.status));
    ui::kv("Depth", &ui::styled_depth(&record.depth));
    if let Some(ref author) = record.author {
        ui::kv("Author", author);
    }
    if let Some(ref epic) = record.parent_epic {
        if !epic.is_empty() {
            ui::kv("Parent Epic", epic);
        }
    }
    if let Some(ref vu) = record.valid_until {
        ui::kv("Valid Until", vu);
    }
    ui::kv("R_eff", &ui::styled_reff(record.r_eff_score));
    ui::kv("Created", &record.created_at);
    ui::kv("Updated", &record.updated_at);
    println!();
    println!("{}", record.body);

    // Contextual hints
    let relations = store.get_relations(id).await.unwrap_or_default();
    let incoming = store.get_incoming_relations(id).await.unwrap_or_default();
    let has_links = !relations.is_empty() || !incoming.is_empty();
    let kind: forgeplan_core::artifact::types::ArtifactKind = record.kind.parse().unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
    let depth: forgeplan_core::artifact::types::Mode = record.depth.parse().unwrap_or(forgeplan_core::artifact::types::Mode::Standard);
    let get_hints = forgeplan_core::hints::get_hints(&record.status, &kind, has_links, &depth);
    if !get_hints.is_empty() {
        print!("{}", forgeplan_core::hints::format_hints(&get_hints));
    }

    Ok(())
}
