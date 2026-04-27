use anyhow::Result;
use console::style;

use forgeplan_core::db::store::ArtifactFilter;
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run(
    query: Option<&str>,
    category: Option<&str>,
    limit: usize,
    json: bool,
) -> Result<()> {
    let store = common::store().await?;

    let filter = ArtifactFilter {
        kind: Some("memory".to_string()),
        status: None,
    };
    let mut records = store.list_records(Some(&filter)).await?;

    // Filter by query (case-insensitive substring in title or body)
    if let Some(q) = query {
        let q_lower = q.to_lowercase();
        records.retain(|r| {
            r.title.to_lowercase().contains(&q_lower) || r.body.to_lowercase().contains(&q_lower)
        });
    }

    // Filter by category (from frontmatter in body)
    if let Some(cat) = category {
        let cat_lower = cat.to_lowercase();
        records.retain(|r| {
            common::extract_frontmatter_field(&r.body, "category")
                .map(|c| c.to_lowercase() == cat_lower)
                .unwrap_or(false)
        });
    }

    // Sort by created_at descending (newest first)
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Limit
    records.truncate(limit);

    let mut hints_vec: Vec<Hint> = Vec::new();

    if records.is_empty() {
        // PRD-071 contract: when no memories matched, suggest capturing one.
        hints_vec.push(
            Hint::suggestion("Capture a memory with `memory_retain`")
                .with_action("forgeplan recall --limit 5".to_string()),
        );
        if json {
            let payload = serde_json::json!({
                "memories": [],
                "_next_action": hints::primary_action(&hints_vec),
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!("  No memories found.");
            print!("{}", hints::render_next_action_line(&hints_vec));
        }
        return Ok(());
    }

    // PRD-071 contract: pick first memory id and suggest promotion. Default
    // kind=note (cheapest, always-available); user picks a real kind.
    let first_id = records[0].id.clone();
    hints_vec.push(
        Hint::suggestion(format!("Promote {} to a real artifact", first_id))
            .with_action(format!("forgeplan promote {} --kind note", first_id)),
    );

    if json {
        let json_data: Vec<_> = records
            .iter()
            .map(|r| {
                let category = common::extract_frontmatter_field(&r.body, "category")
                    .unwrap_or_else(|| "fact".to_string());
                let plain = common::extract_plain_text(&r.body);
                serde_json::json!({
                    "id": r.id,
                    "category": category,
                    "created_at": r.created_at,
                    "text": plain,
                })
            })
            .collect();
        let payload = serde_json::json!({
            "memories": json_data,
            "_next_action": hints::primary_action(&hints_vec),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    for r in &records {
        let category = common::extract_frontmatter_field(&r.body, "category")
            .unwrap_or_else(|| "fact".to_string());
        let date: String = r.created_at.chars().take(10).collect();
        let plain = common::extract_plain_text(&r.body);
        let truncated: String = plain.chars().take(80).collect();

        println!(
            "{}  {}  {}  {}",
            style(&r.id).bold(),
            style(format!("[{}]", category)).dim(),
            style(&date).dim(),
            truncated,
        );
    }

    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}
