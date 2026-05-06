use forgeplan_core::hints::{self, Hint};

use crate::commands::common;
use crate::ui;

pub async fn run(id: &str, json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    // PROB-060 / SPEC-005 Phase 1.5 — `forgeplan get` accepts both display
    // id (`PRD-074`, `prd-074`, `Prd-74`) and slug form (`prd-auth-system`).
    // Resolver maps either form → canonical DB id; None means no match by
    // any path. Audit Phase 1.5 H1: removed redundant `unwrap_or_else(id)`
    // fallback — resolver already covers all legitimate paths.
    // ADR-012 invariants I-3 + I-4 enforcement.
    let record = match store.resolve_id(id).await? {
        Some(canonical) => store.get_record(&canonical).await?.ok_or_else(|| {
            anyhow::anyhow!(
                "Artifact resolved to '{canonical}' but get_record missed (race?)\nFix: forgeplan list"
            )
        })?,
        None => anyhow::bail!("Artifact '{id}' not found\nFix: forgeplan list"),
    };

    // Contextual hints — compute up front so both text and JSON paths emit them.
    // Use canonical id everywhere downstream so relation lookups also work
    // when caller passed slug.
    let relations = store.get_relations(&record.id).await.unwrap_or_default();
    let incoming = store
        .get_incoming_relations(&record.id)
        .await
        .unwrap_or_default();
    let has_links = !relations.is_empty() || !incoming.is_empty();
    let kind: forgeplan_core::artifact::types::ArtifactKind = record
        .kind
        .parse()
        .unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
    let depth: forgeplan_core::artifact::types::Mode = record
        .depth
        .parse()
        .unwrap_or(forgeplan_core::artifact::types::Mode::Standard);

    let mut hints_vec: Vec<Hint> =
        hints::get_hints(&record.id, &record.status, &kind, has_links, &depth);

    // Top-level Next: hint per status — full command, real ID.
    let primary = match record.status.as_str() {
        "draft" => Some(
            Hint::suggestion("Validate after filling MUST sections")
                .with_action(format!("forgeplan validate {}", record.id)),
        ),
        "active" if record.r_eff_score < 0.5 => Some(
            Hint::warning("R_eff below 0.5 — score and add evidence")
                .with_action(format!("forgeplan score {}", record.id)),
        ),
        _ => None,
    };
    if let Some(h) = primary {
        hints_vec.insert(0, h);
    }

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
            "_next_action": hints::primary_action(&hints_vec),
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
    if let Some(ref epic) = record.parent_epic
        && !epic.is_empty()
    {
        ui::kv("Parent Epic", epic);
    }
    if let Some(ref vu) = record.valid_until {
        ui::kv("Valid Until", vu);
    }
    ui::kv("R_eff", &ui::styled_reff(record.r_eff_score));
    ui::kv("Created", &record.created_at);
    ui::kv("Updated", &record.updated_at);
    println!();
    println!("{}", record.body);

    if !hints_vec.is_empty() {
        print!("{}", hints::format_hints(&hints_vec));
    }
    print!("{}", hints::render_next_action_line(&hints_vec));

    Ok(())
}
