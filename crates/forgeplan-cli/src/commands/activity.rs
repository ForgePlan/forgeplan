//! `forgeplan activity` — query the activity log (PRD-054).
//!
//! CLI parity for the `forgeplan_activity` MCP tool. Streams the
//! append-only JSONL records at `.forgeplan/logs/tools-YYYY-MM-DD.jsonl`,
//! applies time / tool / status filters, and prints either pretty JSON
//! or a compact human-readable table.

use chrono::Duration;
use console::style;
use forgeplan_core::activity::query::{QueryFilter, query};
use forgeplan_core::workspace;

/// Run the activity query. Mirrors `ActivityQueryParams` in the MCP server:
/// - `since_hours` clamped 1..=720 (default 24)
/// - `tool` is comma-separated; empty list = no filter
/// - `status` is one of `ok` / `tool_err` / `rpc_err`; empty = all
/// - `limit` clamped 1..=5000 (default 500)
pub async fn run(
    since_hours: u32,
    tool: Option<&str>,
    status: Option<&str>,
    limit: u32,
    json: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let since = since_hours.clamp(1, 720);
    let limit = limit.clamp(1, 5000) as usize;

    let tools: Vec<String> = tool
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let statuses: Vec<String> = status
        .filter(|s| !s.is_empty())
        .map(|s| vec![s.to_string()])
        .unwrap_or_default();

    let filter = QueryFilter {
        since: Some(Duration::hours(since as i64)),
        tools,
        statuses,
        limit: Some(limit),
    };

    let result = query(&ws, &filter).await?;

    if json {
        let payload = serde_json::json!({
            "entries": result.entries,
            "total_scanned": result.total_scanned,
            "returned": result.entries.len(),
            "warnings": result.warnings,
            "since_hours": since,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    // Human-readable table — Forge tone, no emoji.
    if result.entries.is_empty() {
        println!(
            "No tool calls in the last {since} hour(s). Try a wider window: \
             `--since-hours 720` for the last 30 days."
        );
        if !result.warnings.is_empty() {
            println!();
            println!("{}", style("Warnings:").yellow().bold());
            for w in &result.warnings {
                println!("  - {w}");
            }
        }
        return Ok(());
    }

    println!(
        "{}",
        style(format!(
            "Activity log — last {since}h, {} entries (scanned {})",
            result.entries.len(),
            result.total_scanned
        ))
        .bold()
    );
    println!("{}", style("─".repeat(80)).dim());
    println!(
        "{:<24}  {:<32}  {:<10}  {:>8}",
        style("TIMESTAMP").dim(),
        style("TOOL").dim(),
        style("STATUS").dim(),
        style("MS").dim(),
    );
    for e in &result.entries {
        let status_styled = match e.status.as_str() {
            "ok" => style(&e.status).green().to_string(),
            "tool_err" | "rpc_err" => style(&e.status).red().to_string(),
            _ => e.status.clone(),
        };
        // Trim ts to seconds for compactness.
        let ts_short = e.ts.split('.').next().unwrap_or(&e.ts).to_string();
        println!(
            "{:<24}  {:<32}  {:<10}  {:>8}",
            ts_short, e.tool, status_styled, e.duration_ms
        );
    }

    if !result.warnings.is_empty() {
        println!();
        println!("{}", style("Warnings:").yellow().bold());
        for w in &result.warnings {
            println!("  - {w}");
        }
    }

    Ok(())
}
