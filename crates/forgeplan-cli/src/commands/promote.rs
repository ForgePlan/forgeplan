use anyhow::Result;
use console::style;

use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::projection;

use crate::commands::common;

/// Promote a memory artifact to a full artifact of the specified kind.
/// Reads memory content, creates a new artifact, then deletes the memory.
pub async fn run(memory_id: &str, kind: &str) -> Result<()> {
    let (workspace, store, _lock) = common::open_store_locked().await?;

    // Validate kind. PRD-071 contract: error path emits a `Fix:` marker line.
    let artifact_kind: ArtifactKind = kind.parse().map_err(|e| {
        anyhow::anyhow!(
            "Unknown artifact kind '{}': {}. Use: prd, rfc, adr, note, problem, etc.\n\
             Fix: forgeplan promote {} --kind prd",
            kind,
            e,
            memory_id
        )
    })?;

    // Don't promote to memory (circular)
    if matches!(artifact_kind, ArtifactKind::Memory) {
        anyhow::bail!(
            "Cannot promote memory to memory\n\
             Fix: forgeplan promote {} --kind note",
            memory_id
        );
    }

    // Get the memory record
    let record = store.get_record(memory_id).await?.ok_or_else(|| {
        anyhow::anyhow!(
            "Memory '{}' not found\n\
             Fix: forgeplan list --type memory",
            memory_id
        )
    })?;

    if record.kind != "memory" {
        anyhow::bail!(
            "'{}' is not a memory (kind: {}). Only memories can be promoted.\n\
             Fix: forgeplan list --type memory",
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
        // C1: propagate tags from source memory artifact, if any.
        tags: record.tags.clone(),
    };
    // PRD-073 file-first: helper writes the markdown projection first, then
    // syncs to LanceDB. If LanceDB insert fails, reindex recovers.
    projection::create_artifact_with_projection(&workspace, &store, &artifact).await?;

    // PRD-073 file-first: helper removes the memory's markdown file first,
    // then cascades relations and the LanceDB row.
    projection::delete_artifact_with_projection(&workspace, &store, memory_id).await?;

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

    // PRD-071 contract: terminal Next: line — promoted artifact is in draft,
    // user has to fill MUST sections then validate.
    let hints_vec = vec![
        Hint::suggestion(format!("Validate {} after filling MUST sections", new_id))
            .with_action(format!("forgeplan validate {}", new_id)),
    ];
    print!("{}", hints::render_next_action_line(&hints_vec));

    Ok(())
}
