use forgeplan_core::llm::decompose;
use forgeplan_core::workspace::load_config;

use crate::commands::common;

pub async fn run(prd_id: &str) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    let config = load_config(&ws)?;
    let llm_config = config.llm.unwrap_or_default().with_env_overrides();
    let record = store
        .get_record(prd_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", prd_id))?;

    if record.kind != "prd" {
        eprintln!(
            "  Warning: '{}' is a {} (not a PRD). Decomposing anyway.",
            record.id, record.kind
        );
    }

    println!(
        "  Decomposing {} into RFC tasks ({}/{})...\n",
        record.id, llm_config.provider, llm_config.model
    );

    let tasks = decompose::decompose(
        &llm_config,
        &record.id,
        &record.title,
        &record.body,
    )
    .await?;

    println!("{}", tasks);

    Ok(())
}
