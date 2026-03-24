use std::path::PathBuf;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::workspace;

/// Open workspace store — shared boilerplate for all commands.
/// Returns (workspace_path, store).
pub async fn open_store() -> anyhow::Result<(PathBuf, LanceStore)> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    let store = LanceStore::open(&ws).await?;
    Ok((ws, store))
}

/// Open workspace store, returning only the store (most common case).
pub async fn store() -> anyhow::Result<LanceStore> {
    let (_, store) = open_store().await?;
    Ok(store)
}
