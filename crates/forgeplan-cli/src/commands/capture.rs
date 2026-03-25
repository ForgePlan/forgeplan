use anyhow::Context;

use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::llm::capture;
use forgeplan_core::projection;
use forgeplan_core::workspace::load_config;

use crate::commands::common;

pub async fn run(decision: &str, context: Option<&str>) -> anyhow::Result<()> {
    let (workspace, store) = common::open_store().await?;

    let config = load_config(&workspace)?;
    let llm_config = config.llm.unwrap_or_default().with_env_overrides();

    println!(
        "  Capturing decision with {}/{}...",
        llm_config.provider, llm_config.model
    );

    let (kind_str, body) = capture::capture(&llm_config, decision, context)
        .await
        .with_context(|| "LLM capture failed")?;

    let kind: ArtifactKind = kind_str.parse().unwrap_or(ArtifactKind::Note);
    let template_key = kind.template_key();
    let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
    let id = store.next_id(&prefix).await?;

    // Title from first line of decision (truncated)
    let title = decision
        .lines()
        .next()
        .unwrap_or(decision)
        .chars()
        .take(80)
        .collect::<String>();

    let artifact = NewArtifact {
        id: id.clone(),
        kind: template_key.to_string(),
        status: "draft".to_string(),
        title: title.clone(),
        body: body.clone(),
        depth: "tactical".to_string(),
        author: None,
        parent_epic: None,
        valid_until: None,
    };

    store.create_artifact(&artifact).await?;

    let filepath = projection::render_projection(
        &workspace, &id, template_key, &title, "draft", "tactical",
        None, None, None, &body, &[],
    )
    .await?;

    println!("  Captured: {}", filepath.display());
    println!("  ID:       {}", id);
    println!("  Kind:     {} (auto-detected)", template_key);
    println!("  Title:    {}", title);

    Ok(())
}
