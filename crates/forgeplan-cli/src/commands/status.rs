use std::collections::BTreeMap;

use anyhow::Result;

use forgeplan_core::hints::{self, Hint};
use forgeplan_core::workspace::load_config;

use crate::commands::common;

pub async fn run(json: bool) -> Result<()> {
    let (workspace, source, _read_guard) = common::open_read_source().await?;

    let config = load_config(&workspace)?;
    // Counting is over artifact summaries, which the capture produces with the
    // same `to_summary` projection the store path used — same dashboard, no
    // retired table opened on a migrated workspace.
    let artifacts = match &source {
        common::ReadSource::Legacy(store) => store.list_artifacts(&_read_guard, None).await?,
        common::ReadSource::Captured(read) => read.list_artifacts(None),
    };

    // Count by kind
    let mut by_kind: BTreeMap<String, u32> = BTreeMap::new();
    for a in &artifacts {
        *by_kind.entry(a.kind.clone()).or_insert(0) += 1;
    }

    // Count by status
    let mut by_status: BTreeMap<String, u32> = BTreeMap::new();
    for a in &artifacts {
        *by_status.entry(a.status.clone()).or_insert(0) += 1;
    }

    if json {
        // Mirrors the `forgeplan_status` MCP wire shape (StatusResponse):
        // by_kind / by_status are sorted arrays, not objects — BTreeMap
        // gives the same deterministic order.
        let total = artifacts.len();
        let draft_count = *by_status.get("draft").unwrap_or(&0) as usize;
        let active_count = *by_status.get("active").unwrap_or(&0) as usize;
        let next_action = if total == 0 {
            "Empty workspace. Determine artifact depth: `forgeplan_route description=\"...\"`."
                .to_string()
        } else if draft_count > active_count {
            format!("{draft_count} drafts pending. List them: `forgeplan_list status=draft`.")
        } else {
            format!("Workspace has {total} artifact(s). Inspect trust: `forgeplan_health`.")
        };
        let payload = serde_json::json!({
            "project": config.project_name,
            "workspace": workspace.display().to_string(),
            "total": total,
            "by_kind": by_kind.iter().map(|(kind, count)| serde_json::json!({
                "kind": kind, "count": count,
            })).collect::<Vec<_>>(),
            "by_status": by_status.iter().map(|(status, count)| serde_json::json!({
                "status": status, "count": count,
            })).collect::<Vec<_>>(),
            "_next_action": next_action,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    // Print dashboard
    println!("Forgeplan Status");
    println!("================");
    println!();
    println!("  Project:    {}", config.project_name);
    println!("  Workspace:  {}", workspace.display());
    println!("  Created:    {}", config.created_at);
    println!("  Artifacts:  {} total", artifacts.len());
    println!();

    if !by_kind.is_empty() {
        println!("  By kind:");
        for (kind, count) in &by_kind {
            println!("    {:<16} {}", kind, count);
        }
        println!();
    }

    if !by_status.is_empty() {
        println!("  By status:");
        for (status, count) in &by_status {
            println!("    {:<16} {}", status, count);
        }
    }

    if artifacts.is_empty() {
        println!("  No artifacts yet. Create one with `forgeplan new <kind> <title>`.");
    }

    // PRD-071: empty workspace → create something; populated → check health.
    let next_hints: Vec<Hint> = if artifacts.is_empty() {
        vec![Hint::info("Empty workspace").with_action("forgeplan new prd \"<title>\"".to_string())]
    } else {
        vec![Hint::info("Inspect workspace integrity").with_action("forgeplan health".to_string())]
    };
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}
