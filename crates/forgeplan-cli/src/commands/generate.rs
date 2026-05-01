use anyhow::Context;

use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::llm::generate::generate_body;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(kind_str: &str, description: &str) -> anyhow::Result<()> {
    let kind: ArtifactKind = kind_str.parse().map_err(|e| {
        anyhow::anyhow!(
            "{}\n\
             Fix: forgeplan generate prd \"<description>\"",
            e
        )
    })?;

    let (workspace, store, _lock) = common::open_store_locked().await?;

    // PRD-071 contract: emit `Fix:` when LLM unavailable so the agent has a
    // deterministic remediation step.
    let llm_config = match common::require_llm_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Fix: forgeplan setup-skill");
            anyhow::bail!("LLM not configured");
        }
    };

    // Generate title from first line of description (truncated)
    let title = description
        .lines()
        .next()
        .unwrap_or(description)
        .chars()
        .take(80)
        .collect::<String>();

    let template_key = kind.template_key();

    println!(
        "  Generating {} with {} ({})...",
        template_key, llm_config.provider, llm_config.model
    );

    // Generate body via LLM. PRD-071 contract: emit `Fix:` on failure.
    let body = match generate_body(&llm_config, template_key, description, &title)
        .await
        .with_context(|| format!("LLM generation failed (provider: {})", llm_config.provider))
    {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Fix: forgeplan setup-skill");
            anyhow::bail!("LLM call failed");
        }
    };

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
        // C1: AI-generated artifacts start untagged; users can tag post-review.
        tags: Vec::new(),
    };

    // PRD-073 file-first: helper writes file FIRST then syncs to LanceDB.
    let filepath = projection::create_artifact_with_projection(&workspace, &store, &artifact)
        .await
        .with_context(|| format!("Failed to create artifact {} (file-first)", id))?;

    println!("  Created: {}", filepath.display());
    println!("  ID:      {}", id);
    println!("  Kind:    {}", template_key);
    println!("  Title:   {}", title);
    println!(
        "  Source:  AI-generated ({}/{})",
        llm_config.provider, llm_config.model
    );

    // PRD-071 contract: the AI draft still needs a human pass before validate
    // — but the canonical Next: action is validate, since validation surfaces
    // any MUST gaps the LLM left.
    let hints_vec = vec![
        Hint::suggestion(format!("Review AI draft, then validate {}", id))
            .with_action(format!("forgeplan validate {}", id)),
    ];
    print!("{}", hints::render_next_action_line(&hints_vec));

    Ok(())
}
