use anyhow::Result;
use console::style;

use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::artifact::sanitize::sanitize_for_hint;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::projection;

use crate::commands::common;

/// Promote a memory artifact to a full artifact of the specified kind.
/// Reads memory content, creates a new artifact, then deletes the memory.
pub async fn run(memory_id: &str, kind: &str) -> Result<()> {
    let (workspace, lock, store) = common::open_store_locked_tolerating_constellation().await?;

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

    // Get the memory record. On a migrated workspace the canonical markdown
    // is the source of truth, so the record comes from a fresh capture —
    // never from re-opening the retired legacy store to look it up.
    let record = match &store {
        Some(store) => store.get_record(memory_id).await?.ok_or_else(|| {
            anyhow::anyhow!(
                "Memory '{}' not found\n\
                 Fix: forgeplan list --type memory",
                memory_id
            )
        })?,
        None => {
            let read = common::workspace_read(&workspace).await?;
            let canonical = read.resolve_id(memory_id).ok_or_else(|| {
                anyhow::anyhow!(
                    "Memory '{}' not found\n\
                     Fix: forgeplan list --type memory",
                    memory_id
                )
            })?;
            read.get_record(&canonical).cloned().ok_or_else(|| {
                anyhow::anyhow!(
                    "Memory '{}' not found\n\
                         Fix: forgeplan list --type memory",
                    memory_id
                )
            })?
        }
    };

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

    // Build the new artifact body. Shared by both layouts so the promoted
    // artifact is byte-identical whichever index owns the workspace.
    let title = record.title.as_str();
    // Retries borrow the request data; each constructed artifact owns its fields.
    let template_key = artifact_kind.template_key().to_string();
    let author = record.author.clone();
    let tags = record.tags.clone();
    let build = |id: &str, _number: u32, canonical_key: &str| -> anyhow::Result<NewArtifact> {
        let body = format!(
            "# {}: {}\n\n## Content\n\n{}\n\n---\n\n*Promoted from {} on {}*\n",
            id,
            title,
            plain_text,
            memory_id,
            chrono::Utc::now().format("%Y-%m-%d"),
        );
        Ok(NewArtifact {
            canonical_key: canonical_key.to_string(),
            id: id.to_string(),
            kind: template_key.clone(),
            status: "draft".to_string(),
            title: title.to_string(),
            body,
            depth: "tactical".to_string(),
            author: author.clone(),
            parent_epic: None,
            valid_until: None,
            // C1: propagate tags from source memory artifact, if any.
            tags: tags.clone(),
        })
    };

    // Create the new artifact.
    //
    // The shared creator mints the key before reserving a display number.
    // The optional legacy sink only controls the derived-row write.

    let created = forgeplan_core::artifact::id_alloc::create_artifact_with_minted_key(
        &workspace,
        &lock,
        store.as_ref(),
        &artifact_kind,
        3,
        title,
        &build,
    )
    .await?;
    let new_id = created.id;
    let new_key = created.canonical_key;
    let ctx = projection::MutationContext::new(&workspace, &lock, store.as_ref());

    // PRD-073 file-first: helper removes the memory's markdown file first,
    // then cascades relations and the LanceDB row. With a `None` sink it
    // removes the canonical file and stops.
    projection::delete_artifact_with_projection(&ctx, memory_id).await?;

    if store.is_none() {
        common::note_constellation_owns_index();
        drop(lock);
        common::reconcile_after_file_mutation(&workspace).await;
    }

    println!(
        "  Promoted {} → {} ({})",
        style(memory_id).dim(),
        style(&new_id).bold().green(),
        artifact_kind.template_key()
    );
    // SEC-H1 (CWE-117 / CWE-150): title came from the source memory artifact's
    // frontmatter (attacker-controllable for adversarial imports); sanitize
    // before TTY emission to neutralise ANSI/bidi/control bytes.
    println!("  Title: \"{}\"", sanitize_for_hint(title));
    println!("  Key: {}", new_key);
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
