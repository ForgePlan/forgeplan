use std::collections::BTreeMap;
use std::env;

use anyhow::Result;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::workspace::{find_workspace, load_config};

pub async fn run() -> Result<()> {
    let cwd = env::current_dir()?;
    let workspace = find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("Not in a forgeplan workspace. Run `forgeplan init` first."))?;

    let config = load_config(&workspace)?;
    let store = LanceStore::open(&workspace).await?;
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

    Ok(())
}
