use chrono::{Datelike, NaiveDate, Utc};
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run(json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let stale_records = store.find_stale().await?;

    if stale_records.is_empty() {
        // PRD-071: empty stale list is terminal — no action to take.
        if json {
            let payload = serde_json::json!({
                "stale": [],
                "_next_action": null,
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!("No stale artifacts found. All valid_until dates are current.");
            println!("Done.");
        }
        return Ok(());
    }

    // PRD-071: primary action — renew the first stale artifact, providing
    // a deterministic copy-pasteable command with a real ID.
    let top_id = stale_records[0].id.clone();
    let next_year_end = format!("{}-12-31", Utc::now().date_naive().year() + 1);
    let next_hints: Vec<Hint> = vec![
        Hint::warning(format!(
            "{} stale artifact(s) — renew the most recent",
            stale_records.len()
        ))
        .with_action(format!(
            "forgeplan renew {} --reason \"<why still relevant>\" --until {}",
            top_id, next_year_end
        )),
    ];

    if json {
        let today = Utc::now().date_naive();
        let data: Vec<_> = stale_records.iter().map(|r| {
            let days = r.valid_until.as_deref()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                .map(|d| (today - d).num_days())
                .unwrap_or(0);
            serde_json::json!({"id": r.id, "title": r.title, "valid_until": r.valid_until, "days_expired": days})
        }).collect();
        let payload = serde_json::json!({
            "stale": data,
            "_next_action": hints::primary_action(&next_hints),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!(
        "Found {} stale artifact(s) with expired valid_until:\n",
        stale_records.len()
    );

    println!("  {:<12} {:<30} {:<14} Days ago", "ID", "Title", "Expired");
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

    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}
