use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run(
    artifact_id: Option<&str>,
    source: Option<&str>,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    let store = common::store().await?;
    let entries = store.get_change_log(artifact_id, source, limit).await?;

    // PRD-071 contract: pick the most recently changed artifact and suggest
    // viewing it. log entries are reverse-chronological (most recent first).
    let mut hints_vec: Vec<Hint> = Vec::new();
    if let Some(first) = entries.first() {
        hints_vec.push(
            Hint::info(format!(
                "Inspect most recent change on {}",
                first.artifact_id
            ))
            .with_action(format!("forgeplan get {}", first.artifact_id)),
        );
    }

    if json {
        let payload = serde_json::json!({
            "entries": entries,
            "_next_action": hints::primary_action(&hints_vec),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if entries.is_empty() {
        println!("No change log entries found.");
        let bootstrap = vec![
            Hint::suggestion("No change history yet — create or edit an artifact")
                .with_action("forgeplan list".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&bootstrap));
        return Ok(());
    }

    // Header
    println!(
        "{:<18} {:<10} {:<9} {:<8} {:<20} {:<10} COMMIT",
        "TIMESTAMP", "ARTIFACT", "ACTION", "FIELD", "CHANGE", "SOURCE"
    );
    println!("{}", "-".repeat(90));

    for entry in &entries {
        // Shorten timestamp: "2026-03-31T10:15:00+00:00" → "2026-03-31 10:15"
        let ts = entry
            .timestamp
            .replace('T', " ")
            .chars()
            .take(16)
            .collect::<String>();

        let field = entry.field.as_deref().unwrap_or("-");

        let change = match (&entry.old_value, &entry.new_value) {
            (Some(old), Some(new)) => {
                let old_short = truncate(old, 8);
                let new_short = truncate(new, 8);
                format!("{}→{}", old_short, new_short)
            }
            (None, Some(new)) => truncate(new, 18),
            (Some(old), None) => format!("(was {})", truncate(old, 14)),
            (None, None) => "-".to_string(),
        };

        let commit = entry.commit_hash.as_deref().unwrap_or("-");

        println!(
            "{:<18} {:<10} {:<9} {:<8} {:<20} {:<10} {}",
            ts, entry.artifact_id, entry.action, field, change, entry.source, commit
        );
    }

    println!();
    println!("{} entries shown", entries.len());
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
