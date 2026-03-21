use std::env;

use chrono::{NaiveDate, Utc};

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::workspace;

pub async fn run() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let stale_records = store.find_stale().await?;

    if stale_records.is_empty() {
        println!("No stale artifacts found. All valid_until dates are current.");
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
        let title = if record.title.len() > 28 {
            format!("{}...", &record.title[..25])
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
