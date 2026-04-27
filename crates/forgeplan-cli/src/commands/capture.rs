use anyhow::Context;

use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::llm::capture;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(decision: &str, context: Option<&str>) -> anyhow::Result<()> {
    let (workspace, store) = common::open_store().await?;

    // PRD-071 contract: emit `Fix:` when LLM unavailable.
    let llm_config = match common::require_llm_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Fix: forgeplan setup-skill");
            anyhow::bail!("LLM not configured");
        }
    };

    println!(
        "  Capturing decision with {}/{}...",
        llm_config.provider, llm_config.model
    );

    // PRD-071 contract: surface `Fix:` on LLM call failure (rate limit, auth).
    let (kind_str, body) = match capture::capture(&llm_config, decision, context)
        .await
        .with_context(|| "LLM capture failed")
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Fix: forgeplan setup-skill");
            anyhow::bail!("LLM call failed");
        }
    };

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
        // C1: fresh capture has no tags yet — user adds via `forgeplan tag` later.
        tags: Vec::new(),
    };

    store.create_artifact(&artifact).await?;

    let filepath = projection::render_projection(
        &workspace,
        &id,
        template_key,
        &title,
        "draft",
        "tactical",
        None,
        None,
        None,
        &body,
        &[],
    )
    .await?;

    println!("  Captured: {}", filepath.display());
    println!("  ID:       {}", id);
    println!("  Kind:     {} (auto-detected)", template_key);
    println!("  Title:    {}", title);

    let hint_list = vec![
        Hint::info("Review the captured artifact").with_action(format!("forgeplan get {}", id)),
    ];
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
