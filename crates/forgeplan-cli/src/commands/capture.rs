use anyhow::Context;

use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::llm::capture;
use forgeplan_core::projection;

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
    // Both layouts allocate through the same `build` closure so the written
    // bytes are identical; only the id source and the derived sink differ.
    // A migrated workspace takes its id from the canonical markdown under
    // the cross-worktree id-alloc lock (with collision retry) and writes no
    // legacy row — opening one would resurrect a retired table.
    let captured_body = body.clone();
    let captured_title = title.clone();
    let captured_template_key = template_key.to_string();
    let build = move |id: &str, _number: u32| -> anyhow::Result<NewArtifact> {
        Ok(NewArtifact {
            // ADR-030: no minted key yet — the document declares no slug, so
            // its canonical key is its display id.
            canonical_key: id.to_string(),
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

    let (id, filepath) = match &store {
        Some(store) => {
            let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
            let id = store.next_id(&prefix).await?;
            let artifact = build(&id, 0)?;
            let filepath = projection::create_artifact_with_projection(
                &projection::MutationContext::new(&workspace, &lock, Some(store)),
                &artifact,
            )
            .await?;
            (id, filepath)
        }
        None => {
            let allocated =
                forgeplan_core::artifact::id_alloc::allocate_and_create_artifact_file_first(
                    &workspace, &lock, &kind, 3, &build,
                )
                .await?;
            (allocated.id, allocated.filepath)
        }
    };

    if store.is_none() {
        common::note_constellation_owns_index();
        // Lock must drop before reconcile — it re-acquires the same lock per
        // storage commit, and a held guard would time the capture out.
        drop(lock);
        common::reconcile_after_file_mutation(&workspace).await;
    }

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
