use anyhow::Result;
use console::style;

use forgeplan_core::artifact::types::{ArtifactKind, slugify};
use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::projection;

use crate::commands::common;

/// Promote a memory artifact to a full artifact of the specified kind.
/// Reads memory content, creates a new artifact, then deletes the memory.
pub async fn run(memory_id: &str, kind: &str) -> Result<()> {
    let (workspace, store) = common::open_store().await?;

    // Validate kind
    let artifact_kind: ArtifactKind = kind.parse().map_err(|e| {
        anyhow::anyhow!(
            "Unknown artifact kind '{}': {}. Use: prd, rfc, adr, note, problem, etc.",
            kind,
            e
        )
    })?;

    // Don't promote to memory (circular)
    if matches!(artifact_kind, ArtifactKind::Memory) {
        anyhow::bail!("Cannot promote memory to memory");
    }

    // Get the memory record
    let record = store
        .get_record(memory_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Memory '{}' not found", memory_id))?;

    if record.kind != "memory" {
        anyhow::bail!(
            "'{}' is not a memory (kind: {}). Only memories can be promoted.",
            memory_id,
            record.kind
        );
    }

    // Extract plain text from memory body (skip frontmatter)
    let plain_text = common::extract_plain_text(&record.body);

    // Generate new artifact ID
    let prefix = artifact_kind.prefix().trim_end_matches('-').to_uppercase();
    let new_id = store.next_id(&prefix).await?;

    // Build the new artifact body
    let title = record.title.clone();
    let body = format!(
        "# {}: {}\n\n## Content\n\n{}\n\n---\n\n*Promoted from {} on {}*\n",
        new_id,
        title,
        plain_text,
        memory_id,
        chrono::Utc::now().format("%Y-%m-%d"),
    );

    // Create the new artifact
    let artifact = NewArtifact {
        id: new_id.clone(),
        kind: artifact_kind.template_key().to_string(),
        status: "draft".to_string(),
        title: title.clone(),
        body: body.clone(),
        depth: "tactical".to_string(),
        author: record.author.clone(),
        parent_epic: None,
        valid_until: None,
    };
    store.create_artifact(&artifact).await?;

    // Write markdown projection
    projection::render_projection(
        &workspace,
        &new_id,
        artifact_kind.template_key(),
        &title,
        "draft",
        "tactical",
        record.author.as_deref(),
        None,
        None,
        &body,
        &[],
    )
    .await?;

    // Delete the original memory
    store.delete_artifact(memory_id).await?;

    // Remove memory markdown file
    let mem_slug = slugify(&record.title);
    let mem_filename = format!("{}-{}.md", memory_id, mem_slug);
    let mem_filepath = workspace
        .join(ArtifactKind::Memory.dir_name())
        .join(&mem_filename);
    if mem_filepath.exists()
        && let Err(e) = tokio::fs::remove_file(&mem_filepath).await
    {
        eprintln!(
            "  Warning: could not remove memory file {}: {}",
            mem_filepath.display(),
            e
        );
    }

    println!(
        "  Promoted {} → {} ({})",
        style(memory_id).dim(),
        style(&new_id).bold().green(),
        artifact_kind.template_key()
    );
    println!("  Title: \"{}\"", title);
    println!(
        "  Status: draft — fill required sections, then: forgeplan validate {}",
        new_id
    );

    Ok(())
}
