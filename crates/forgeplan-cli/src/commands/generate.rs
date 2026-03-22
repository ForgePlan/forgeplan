use std::env;

use anyhow::Context;

use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::config::LlmConfig;
use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::llm::generate::generate_body;
use forgeplan_core::projection;
use forgeplan_core::workspace::{self, load_config};

pub async fn run(kind_str: &str, description: &str) -> anyhow::Result<()> {
    let kind: ArtifactKind = kind_str
        .parse()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let cwd = env::current_dir()?;
    let workspace = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("Not in a forgeplan workspace. Run `forgeplan init` first."))?;

    let config = load_config(&workspace)?;
    let llm_config = config.llm.unwrap_or_default();

    let store = LanceStore::open(&workspace).await?;

    // Generate title from first line of description (truncated)
    let title = description
        .lines()
        .next()
        .unwrap_or(description)
        .chars()
        .take(80)
        .collect::<String>();

    let template_key = kind.template_key();

    println!("  Generating {} with {} ({})...", template_key, llm_config.provider, llm_config.model);

    // Generate body via LLM
    let body = generate_body(&llm_config, template_key, description, &title)
        .await
        .with_context(|| format!("LLM generation failed (provider: {})", llm_config.provider))?;

    // Create artifact with generated body
    let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
    let id = store.next_id(&prefix).await?;

    let artifact = NewArtifact {
        id: id.clone(),
        kind: template_key.to_string(),
        status: "draft".to_string(),
        title: title.clone(),
        body: body.clone(),
        depth: "standard".to_string(),
        author: None,
        parent_epic: None,
        valid_until: None,
    };

    store
        .create_artifact(&artifact)
        .await
        .with_context(|| format!("Failed to create artifact {} in LanceDB", id))?;

    let filepath = projection::render_projection(
        &workspace, &id, template_key, &title, "draft", "standard",
        None, None, None, &body, &[],
    )
    .await
    .with_context(|| format!("Failed to write projection for {}", id))?;

    println!("  Created: {}", filepath.display());
    println!("  ID:      {}", id);
    println!("  Kind:    {}", template_key);
    println!("  Title:   {}", title);
    println!("  Source:  AI-generated ({}/{})", llm_config.provider, llm_config.model);

    Ok(())
}
