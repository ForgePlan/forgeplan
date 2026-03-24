use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::workspace;

pub async fn run() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    println!("  Running schema migrations...");
    let _store = LanceStore::open(&ws).await?; // open() runs migrations
    println!("  Migrations complete. Schema up to date.");

    Ok(())
}
