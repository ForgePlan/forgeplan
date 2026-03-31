use std::path::PathBuf;

use forgeplan_core::config::types::Config;
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

/// Load workspace config.
pub fn config() -> anyhow::Result<Config> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    workspace::load_config(&ws)
}

/// Open workspace store, returning only the store (most common case).
pub async fn store() -> anyhow::Result<LanceStore> {
    let (_, store) = open_store().await?;
    Ok(store)
}

/// Load and validate LLM config — fails early with actionable message if not configured.
pub fn require_llm_config() -> anyhow::Result<forgeplan_core::config::types::LlmConfig> {
    let cfg = config()?;
    let llm = cfg
        .llm
        .ok_or_else(|| {
            anyhow::anyhow!(
                "LLM not configured.\n\
                 Add to .forgeplan/config.yaml:\n\
                 llm:\n\
                   provider: gemini\n\
                   api_key_env: GEMINI_API_KEY"
            )
        })?
        .with_env_overrides();
    if llm.resolve_api_key().is_none() {
        anyhow::bail!(
            "API key not found for provider '{}'.\n\
             Set environment variable: {}",
            llm.provider,
            llm.api_key_env.as_deref().unwrap_or("(none configured)")
        );
    }
    Ok(llm)
}

/// Open storage using driver trait (new API — will replace open_store over time).
#[allow(dead_code)]
pub async fn open_driver() -> anyhow::Result<std::sync::Arc<dyn forgeplan_core::driver::StorageDriver>> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ workspace found"))?;
    let config = workspace::load_config(&ws)?;
    let storage_config = config.storage.unwrap_or_default();
    forgeplan_core::driver::factory::create_storage(&storage_config, &ws).await
}
