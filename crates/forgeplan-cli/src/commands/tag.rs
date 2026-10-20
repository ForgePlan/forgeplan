//! forgeplan tag / untag — manage artifact tags.

use crate::commands::common;
use anyhow::{Context, Result};
use forgeplan_core::artifact::store::resolve_artifact_path;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::projection;
use forgeplan_core::workspace::lock::WorkspaceLock;

/// Tags currently on the artifact's canonical markdown.
///
/// File-first and store-backed paths must answer the SAME question or the
/// two print different "Current tags:" lines for the same workspace layout.
/// The projection frontmatter is the source of truth on both, so both read
/// it here rather than one reading a cached column.
async fn current_tags(path: &std::path::Path) -> Result<Vec<String>> {
    let content = tokio::fs::read_to_string(path).await?;
    let (fm, _) = forgeplan_core::artifact::frontmatter::parse_frontmatter(&content)?;
    Ok(forgeplan_core::artifact::frontmatter::tags_from_frontmatter(&fm))
}

/// Add tags to an artifact.
pub async fn run_add(id: &str, tags: &[String]) -> Result<()> {
    let (ws, _lock, store) = common::open_store_locked_tolerating_constellation().await?;
    let Some(store) = store else {
        return run_add_file_first(ws, _lock, id, tags).await;
    };

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
    projection::add_tags_with_projection(
        &projection::MutationContext::new(&ws, &_lock, Some(&store)),
        id,
        tags,
    )
    .await
    .with_context(|| format!("Failed to add tags to {}", id))?;

    let updated = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact disappeared after update: {}", id))?;

    let added = updated.tags.len().saturating_sub(_existing.tags.len());
    println!("  ✓ Added {} tag(s) to {}", added, id);
    print_current_tags(&updated.tags);

    print_tag_added_hint(tags);
    Ok(())
}

/// File-first `tag` for a constellation-migrated workspace: there is no
/// `LanceStore`, so the tag set is edited in the projection frontmatter
/// (`update_tags_file_first`, reached through the same
/// `add_tags_with_projection` helper with a `None` sink) and one reconcile
/// pass publishes the change to `constellation_*_v1`.
///
/// Same shape as the store path: usage check → resolve → mutate → report the
/// resulting set → hint. The count is measured on the file, before and
/// after, so "Added N tag(s)" cannot claim work the bytes do not show.
async fn run_add_file_first(
    ws: std::path::PathBuf,
    lock: WorkspaceLock,
    id: &str,
    tags: &[String],
) -> Result<()> {
    let path = resolve_artifact_path(&ws, id)
        .await
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {}", id))?;
    if tags.is_empty() {
        anyhow::bail!("No tags provided. Usage: forgeplan tag <id> <tag>...");
    }
    let before = current_tags(&path).await?;

    projection::add_tags_with_projection(
        &projection::MutationContext::new(&ws, &lock, None),
        id,
        tags,
    )
    .await
    .with_context(|| format!("Failed to add tags to {}", id))?;

    // Re-read the canonical file: the count and the printed set must come
    // from the bytes that were just written, not from a pre-mutation view.
    let after = current_tags(&path).await?;
    let added = after.len().saturating_sub(before.len());
    println!("  ✓ Added {} tag(s) to {}", added, id);
    print_current_tags(&after);

    common::note_constellation_owns_index();
    drop(lock);
    common::reconcile_after_file_mutation(&ws).await;

    print_tag_added_hint(tags);
    Ok(())
}

/// Remove tags from an artifact.
pub async fn run_remove(id: &str, tags: &[String]) -> Result<()> {
    let (ws, _lock, store) = common::open_store_locked_tolerating_constellation().await?;
    let Some(store) = store else {
        return run_remove_file_first(ws, _lock, id, tags).await;
    };

    let _existing = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {}", id))?;

    if tags.is_empty() {
        anyhow::bail!("No tags provided. Usage: forgeplan untag <id> <tag>...");
    }

    // PRD-073 file-first: helper handles sync→remove_tags→render in one shot.
    projection::remove_tags_with_projection(
        &projection::MutationContext::new(&ws, &_lock, Some(&store)),
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
    print_current_tags(&updated.tags);

    // PRD-071: verify state after un-tagging.
    let next_hints: Vec<Hint> = vec![
        Hint::info("Tags removed — verify artifact").with_action(format!("forgeplan get {}", id)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}

/// File-first `untag`. Mirrors `run_add_file_first` — same resolution, same
/// usage refusal, same "Removed N tag(s)" wording (N counts the tags the
/// user named, exactly as the store path does), same reconcile.
async fn run_remove_file_first(
    ws: std::path::PathBuf,
    lock: WorkspaceLock,
    id: &str,
    tags: &[String],
) -> Result<()> {
    let path = resolve_artifact_path(&ws, id)
        .await
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {}", id))?;

    if tags.is_empty() {
        anyhow::bail!("No tags provided. Usage: forgeplan untag <id> <tag>...");
    }

    projection::remove_tags_with_projection(
        &projection::MutationContext::new(&ws, &lock, None),
        id,
        tags,
    )
    .await
    .with_context(|| format!("Failed to remove tags from {}", id))?;

    let after = current_tags(&path).await?;
    println!("  ✓ Removed {} tag(s) from {}", tags.len(), id);
    print_current_tags(&after);

    common::note_constellation_owns_index();
    drop(lock);
    common::reconcile_after_file_mutation(&ws).await;

    let next_hints: Vec<Hint> = vec![
        Hint::info("Tags removed — verify artifact").with_action(format!("forgeplan get {}", id)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));
    Ok(())
}

fn print_current_tags(tags: &[String]) {
    println!(
        "  Current tags: {}",
        if tags.is_empty() {
            "(none)".to_string()
        } else {
            tags.join(", ")
        }
    );
}

/// PRD-071: tags applied — surface filtered list as the verifying action.
///
/// w4-security-audit HIGH-1 (CWE-78): user-provided tag flows into the
/// agent-visible `Next:` hint verbatim. Mirrors decompose.rs:70 /
/// reason.rs:153 — sanitize before interpolation so shell metacharacters
/// (`;`, `$`, ``, `|`, `&`, …) cannot survive copy-paste from agent
/// output into a terminal. Empty/all-metachar input collapses to "" and
/// is replaced with the documented `<tag>` placeholder so the hint stays
/// self-explanatory.
fn print_tag_added_hint(tags: &[String]) {
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
}
