use chrono::{NaiveDate, Utc};

use crate::commands::common;

pub async fn run(json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let stale_records = store.find_stale().await?;

    if stale_records.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No stale artifacts found. All valid_until dates are current.");
        }
        return Ok(());
    }

    if json {
        let today = Utc::now().date_naive();
        let data: Vec<_> = stale_records.iter().map(|r| {
            let days = r.valid_until.as_deref()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                .map(|d| (today - d).num_days())
                .unwrap_or(0);
            serde_json::json!({"id": r.id, "title": r.title, "valid_until": r.valid_until, "days_expired": days})
        }).collect();
        println!("{}", serde_json::to_string_pretty(&data)?);
        return Ok(());
    }

    println!(
        "Found {} stale artifact(s) with expired valid_until:\n",
        stale_records.len()
    );

    println!(
        "  {:<12} {:<30} {:<14} {}",
        "ID", "Title", "Expired", "Days ago"
    );
    println!("  {}", "-".repeat(70));

    let today = Utc::now().date_naive();

    for record in &stale_records {
        let title: String = if record.title.chars().count() > 28 {
            format!("{}...", record.title.chars().take(25).collect::<String>())
        } else {
            record.title.clone()
        };
        let valid_until_str = record.valid_until.as_deref().unwrap_or("?");
        let days = record
            .valid_until
            .as_deref()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .map(|d| (today - d).num_days())
            .unwrap_or(0);
        println!(
            "  {:<12} {:<30} {:<14} {} days",
            record.id, title, valid_until_str, days
        );
    }

    println!();
    println!("Hint: Use `forgeplan score <ID>` to check R_eff impact of stale evidence.");

    Ok(())
}
