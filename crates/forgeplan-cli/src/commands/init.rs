use std::env;

use anyhow::Result;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::workspace::{find_workspace, init_workspace, FORGEPLAN_DIR};

pub async fn run(force: bool) -> Result<()> {
    let cwd = env::current_dir()?;

    // Check if already initialized
    if let Some(existing) = find_workspace(&cwd) {
        if !force {
            println!("  Already initialized at {}", existing.display());
            println!("  Use --force to reinitialize.");
            return Ok(());
        }
        // Remove existing workspace for reinit
        tokio::fs::remove_dir_all(&existing).await?;
        println!("  Removed existing {}", existing.display());
    }

    // Derive project name from directory name
    let project_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unnamed".into());

    let ws = init_workspace(&cwd, &project_name)?;

    // Initialize LanceDB tables (artifacts, evidence, relations)
    LanceStore::init(&ws).await?;

    println!("  Initialized {}/ in {}", FORGEPLAN_DIR, cwd.display());
    println!("  Project: {}", project_name);
    println!("  Config:  {}", ws.join("config.yaml").display());
    println!("  LanceDB: {}", ws.join("lance").display());
    Ok(())
}
