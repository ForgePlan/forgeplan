//! forgeplan tag / untag — manage artifact tags.

use crate::commands::common;
use anyhow::{Context, Result};
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::projection;

/// Add tags to an artifact.
pub async fn run_add(id: &str, tags: &[String]) -> Result<()> {
    let (ws, _lock, store) = common::open_store_locked().await?;

    // Verify artifact exists
    let _existing = store
        .get_record(id)
        .await
        .with_context(|| format!("Failed to load {}", id))?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {}", id))?;

    if tags.is_empty() {
        anyhow::bail!("No tags provided. Usage: forgeplan tag <id> <tag>...");
    }

    // PRD-073 file-first: helper handles sync→add_tags→render in one shot.
    projection::add_tags_with_projection(&projection::MutationContext::new(&ws, &store), id, tags)
        .await
        .with_context(|| format!("Failed to add tags to {}", id))?;

    let updated = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact disappeared after update: {}", id))?;

    let added = updated.tags.len().saturating_sub(_existing.tags.len());
    println!("  ✓ Added {} tag(s) to {}", added, id);
    println!(
        "  Current tags: {}",
        if updated.tags.is_empty() {
            "(none)".to_string()
        } else {
            updated.tags.join(", ")
        }
    );

    // PRD-071: tags applied — surface filtered list as the verifying action.
    // w4-security-audit HIGH-1 (CWE-78): user-provided tag flows into the
    // agent-visible `Next:` hint verbatim. Mirrors decompose.rs:70 /
    // reason.rs:153 — sanitize before interpolation so shell metacharacters
    // (`;`, `$`, ``, `|`, `&`, …) cannot survive copy-paste from agent
    // output into a terminal. Empty/all-metachar input collapses to "" and
    // is replaced with the documented `<tag>` placeholder so the hint stays
    // self-explanatory.
    let primary_tag = tags
        .iter()
        .find(|t| !t.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| "<tag>".to_string());
    let safe_tag = {
        let s = forgeplan_core::artifact::sanitize::sanitize_for_hint(&primary_tag);
        if s.is_empty() { "<tag>".to_string() } else { s }
    };
    let next_hints: Vec<Hint> = vec![
        Hint::info("Tag added — list filtered artifacts")
            .with_action(format!("forgeplan list --tag {}", safe_tag)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}

/// Remove tags from an artifact.
pub async fn run_remove(id: &str, tags: &[String]) -> Result<()> {
    let (ws, _lock, store) = common::open_store_locked().await?;

    let _existing = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {}", id))?;

    if tags.is_empty() {
        anyhow::bail!("No tags provided. Usage: forgeplan untag <id> <tag>...");
    }

    // PRD-073 file-first: helper handles sync→remove_tags→render in one shot.
    projection::remove_tags_with_projection(
        &projection::MutationContext::new(&ws, &store),
        id,
        tags,
    )
    .await
    .with_context(|| format!("Failed to remove tags from {}", id))?;

    let updated = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact disappeared after update: {}", id))?;

    println!("  ✓ Removed {} tag(s) from {}", tags.len(), id);
    println!(
        "  Current tags: {}",
        if updated.tags.is_empty() {
            "(none)".to_string()
        } else {
            updated.tags.join(", ")
        }
    );

    // PRD-071: verify state after un-tagging.
    let next_hints: Vec<Hint> = vec![
        Hint::info("Tags removed — verify artifact").with_action(format!("forgeplan get {}", id)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}
