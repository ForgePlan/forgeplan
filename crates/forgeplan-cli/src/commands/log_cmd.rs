use crate::commands::common;

pub async fn run(
    artifact_id: Option<&str>,
    source: Option<&str>,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    let store = common::store().await?;
    let entries = store.get_change_log(artifact_id, source, limit).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if entries.is_empty() {
        println!("No change log entries found.");
        return Ok(());
    }

    // Header
    println!(
        "{:<18} {:<10} {:<9} {:<8} {:<20} {:<10} {}",
        "TIMESTAMP", "ARTIFACT", "ACTION", "FIELD", "CHANGE", "SOURCE", "COMMIT"
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
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
