//! `forgeplan activity-stats` — aggregate per-tool stats (PRD-054).
//!
//! CLI parity for the `forgeplan_activity_stats` MCP tool. Computes
//! count, error count, p50/p95/total duration grouped by tool name
//! over a time window. Used to attribute LLM-token spend and identify
//! slow tools.

use chrono::Duration;
use console::style;
use forgeplan_core::activity::query::{QueryFilter, compute_stats, query};
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::workspace;

/// Run the stats query. Mirrors `ActivityStatsParams`:
/// - `since_hours` clamped 1..=720 (default 24).
pub async fn run(since_hours: u32, json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let since = since_hours.clamp(1, 720);
    let filter = QueryFilter {
        since: Some(Duration::hours(since as i64)),
        tools: vec![],
        statuses: vec![],
        limit: None,
    };

    let result = query(&ws, &filter).await?;
    let stats = compute_stats(&result.entries);
    let total_calls: usize = stats.iter().map(|s| s.count).sum();
    let total_errors: usize = stats.iter().map(|s| s.err_count).sum();
    let total_ms: u64 = stats.iter().map(|s| s.total_ms).sum();

    let mut hint_list: Vec<Hint> = Vec::new();
    if stats.is_empty() {
        hint_list.push(
            Hint::info("Try a longer window")
                .with_action("forgeplan activity-stats --since-hours 720".to_string()),
        );
    } else {
        hint_list.push(
            Hint::info("See raw entries")
                .with_action(format!("forgeplan activity --since-hours {since}")),
        );
    }

    if json {
        let payload = serde_json::json!({
            "stats": stats,
            "total_calls": total_calls,
            "total_errors": total_errors,
            "total_ms": total_ms,
            "since_hours": since,
            "_next_action": hints::primary_action(&hint_list),
            "hints": hint_list,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if stats.is_empty() {
        println!(
            "No activity in the last {since} hour(s). Try a longer window: \
             `--since-hours 720` for 30 days."
        );
        print!("{}", hints::render_next_action_line(&hint_list));
        return Ok(());
    }

    println!(
        "{}",
        style(format!(
            "Activity stats — last {since}h, {total_calls} call(s), {total_errors} error(s), \
             {total_ms} ms total"
        ))
        .bold()
    );
    println!("{}", style("─".repeat(80)).dim());
    println!(
        "{:<32}  {:>6}  {:>6}  {:>8}  {:>8}  {:>10}",
        style("TOOL").dim(),
        style("COUNT").dim(),
        style("ERR").dim(),
        style("P50_MS").dim(),
        style("P95_MS").dim(),
        style("TOTAL_MS").dim(),
    );
    for s in &stats {
        let err_styled = if s.err_count > 0 {
            style(s.err_count).red().to_string()
        } else {
            format!("{}", s.err_count)
        };
        println!(
            "{:<32}  {:>6}  {:>6}  {:>8}  {:>8}  {:>10}",
            s.tool, s.count, err_styled, s.p50_ms, s.p95_ms, s.total_ms
        );
    }

    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
