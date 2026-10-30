use anyhow::Context;

use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::llm::generate::generate_body;

use crate::commands::common;

pub async fn run(kind_str: &str, description: &str) -> anyhow::Result<()> {
    let kind: ArtifactKind = kind_str.parse().map_err(|e| {
        anyhow::anyhow!(
            "{}\n\
             Fix: forgeplan generate prd \"<description>\"",
            e
        )
    })?;

    let (workspace, lock, store) = common::open_store_locked_tolerating_constellation().await?;

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

    // Create artifact with generated body.
    //
    // The shared creator mints the key before reserving the display number,
    // then publishes the key-named document. The optional legacy sink only
    // controls whether the derived row is written alongside the file.
    let generated_body = body.clone();
    let generated_title = title.clone();
    let generated_template_key = template_key.to_string();
    let build = move |id: &str, _number: u32, canonical_key: &str| -> anyhow::Result<NewArtifact> {
        Ok(NewArtifact {
            canonical_key: canonical_key.to_string(),
            id: id.to_string(),
            kind: generated_template_key.clone(),
            status: "draft".to_string(),
            title: generated_title.clone(),
            body: generated_body.clone(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            // C1: AI-generated artifacts start untagged; users can tag post-review.
            tags: Vec::new(),
        })
    };

    // PRD-073 file-first: helper writes file FIRST then syncs to LanceDB.
    // PROB-049 H-6: persistent deps now flow through `MutationContext`.

    let created = forgeplan_core::artifact::id_alloc::create_artifact_with_minted_key(
        &workspace,
        &lock,
        store.as_ref(),
        &kind,
        3,
        &title,
        &build,
    )
    .await
    .with_context(|| format!("Failed to create {} {} (key-file)", template_key, title))?;
    let id = created.id;
    let canonical_key = created.canonical_key;
    let filepath = created.filepath;
    if store.is_none() {
        common::note_constellation_owns_index();
        // Lock must drop before reconcile — it re-acquires the same lock per
        // storage commit, and a held guard would time the capture out.
        drop(lock);
        common::reconcile_after_file_mutation(&workspace).await;
    }

    println!("  Created: {}", filepath.display());
    println!("  ID:      {}", id);
    println!("  Key:     {}", canonical_key);
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
