use anyhow::Result;
use console::style;

use forgeplan_core::db::store::ArtifactFilter;

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
            r.title.to_lowercase().contains(&q_lower)
                || r.body.to_lowercase().contains(&q_lower)
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

    if records.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("  No memories found.");
        }
        return Ok(());
    }

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
        println!("{}", serde_json::to_string_pretty(&json_data)?);
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

    Ok(())
}
