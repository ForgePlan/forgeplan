use std::collections::BTreeMap;

use anyhow::Result;

use forgeplan_core::hints::{self, Hint};
use forgeplan_core::workspace::load_config;

use crate::commands::common;

pub async fn run() -> Result<()> {
    let (workspace, store) = common::open_store().await?;

    let config = load_config(&workspace)?;
    let artifacts = store.list_artifacts(None).await?;

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
