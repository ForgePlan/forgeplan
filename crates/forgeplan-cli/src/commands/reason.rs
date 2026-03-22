use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::llm::reason;
use forgeplan_core::workspace::{self, load_config};

pub async fn run(id: &str) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let config = load_config(&ws)?;
    let llm_config = config.llm.unwrap_or_default().with_env_overrides();

    let store = LanceStore::open(&ws).await?;
    let record = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", id))?;

    println!(
        "  Analyzing {} with ADI cycle ({}/{})...\n",
        record.id, llm_config.provider, llm_config.model
    );

    let analysis = reason::reason(
        &llm_config,
        &record.id,
        &record.title,
        &record.kind,
        &record.body,
    )
    .await?;

    println!("{}", analysis);

    Ok(())
}
