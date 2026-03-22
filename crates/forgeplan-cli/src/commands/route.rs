use std::env;

use forgeplan_core::routing;
use forgeplan_core::workspace;

pub async fn run(description: &str, explain: bool) -> anyhow::Result<()> {
    let _cwd = env::current_dir()?;

    // Rule-based routing (instant, offline, no LLM)
    let result = routing::route(description);
    print!("{result}");

    // Optional LLM explanation
    if explain {
        let cwd = env::current_dir()?;
        let ws = workspace::find_workspace(&cwd);
        if let Some(ws) = ws {
            let config = workspace::load_config(&ws)?;
            if let Some(llm_config) = config.llm {
                let llm_config = llm_config.with_env_overrides();
                println!("\n## AI Explanation\n");
                let explanation =
                    forgeplan_core::llm::route::route(&llm_config, description).await?;
                println!("{explanation}");
            } else {
                println!("\n(--explain requires LLM config in .forgeplan/config.yaml)");
            }
        }
    }

    Ok(())
}
