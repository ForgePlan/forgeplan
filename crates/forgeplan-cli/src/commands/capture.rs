use anyhow::Context;

use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::llm::capture;

use crate::commands::common;

pub async fn run(decision: &str, context: Option<&str>) -> anyhow::Result<()> {
    let (workspace, lock, store) = common::open_store_locked_tolerating_constellation().await?;

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

    // Title from first line of decision (truncated)
    let title = decision
        .lines()
        .next()
        .unwrap_or(decision)
        .chars()
        .take(80)
        .collect::<String>();

    // PROB-049 H-6: helpers now take `&MutationContext` instead of
    // `(workspace, store)` separately so the persistent dependencies
    // funnel through one shape.
    //
    // The shared creator mints the key before reserving the display number,
    // then publishes the key-named document. The optional legacy sink only
    // controls whether the derived row is written alongside the file.
    let captured_body = body.clone();
    let captured_title = title.clone();
    let captured_template_key = template_key.to_string();
    let build = move |id: &str, _number: u32, canonical_key: &str| -> anyhow::Result<NewArtifact> {
        Ok(NewArtifact {
            canonical_key: canonical_key.to_string(),
            id: id.to_string(),
            kind: captured_template_key.clone(),
            status: "draft".to_string(),
            title: captured_title.clone(),
            body: captured_body.clone(),
            depth: "tactical".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            // C1: fresh capture has no tags yet — user adds via `forgeplan tag` later.
            tags: Vec::new(),
        })
    };

    let created = forgeplan_core::artifact::id_alloc::create_artifact_with_minted_key(
        &workspace,
        &lock,
        store.as_ref(),
        &kind,
        3,
        &title,
        &build,
    )
    .await?;
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

    println!("  Captured: {}", filepath.display());
    println!("  ID:       {}", id);
    println!("  Key:      {}", canonical_key);
    println!("  Kind:     {} (auto-detected)", template_key);
    println!("  Title:    {}", title);

    let hint_list = vec![
        Hint::info("Review the captured artifact").with_action(format!("forgeplan get {}", id)),
    ];
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
