use std::env;
use std::fs;

use anyhow::Result;

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
        fs::remove_dir_all(&existing)?;
        println!("  Removed existing {}", existing.display());
    }

    // Derive project name from directory name
    let project_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unnamed".into());

    let ws = init_workspace(&cwd, &project_name)?;
    println!("  Initialized {}/ in {}", FORGEPLAN_DIR, cwd.display());
    println!("  Project: {}", project_name);
    println!("  Config:  {}", ws.join("config.yaml").display());
    Ok(())
}
