use std::env;

use forgeplan_core::llm::route;
use forgeplan_core::workspace::{self, load_config};

pub async fn run(description: &str) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let config = load_config(&ws)?;
    let llm_config = config.llm.unwrap_or_default().with_env_overrides();

    println!(
        "  Routing task with {}/{}...\n",
        llm_config.provider, llm_config.model
    );

    let result = route::route(&llm_config, description).await?;
    println!("{}", result);

    Ok(())
}
