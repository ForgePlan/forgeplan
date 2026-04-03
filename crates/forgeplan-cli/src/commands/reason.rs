use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::llm::reason;
use forgeplan_core::llm::reason::ArtifactContext;

use crate::commands::common;

/// Architecture hint injected into ADI prompt so the LLM knows the tech stack.
const ARCHITECTURE_HINT: &str = "\
Forgeplan is a Rust CLI + MCP server. \
Storage: LanceDB (embedded, tables + vectors). \
Architecture: forgeplan-core (shared library) + forgeplan-cli + forgeplan-mcp. \
Driver traits: StorageDriver, EmbedDriver, MemoryDriver, LlmDriver. \
Embedding: local BGE-M3 via fastembed (no API needed). \
Files in .forgeplan/ are authoritative, LanceDB syncs from them.";

pub async fn run(id: &str, json: bool, save: bool, fpf: bool) -> anyhow::Result<()> {
    let (_ws, store) = common::open_store().await?;

    let llm_config = common::require_llm_config()?;
    let record = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", id))?;

    // Build artifact context from store metadata
    let relations = store.get_relations(&record.id).await.unwrap_or_default();
    let artifact_context = ArtifactContext {
        status: record.status.clone(),
        depth: record.depth.clone(),
        r_eff_score: record.r_eff_score,
        relations,
        architecture_hint: Some(ARCHITECTURE_HINT.to_string()),
    };

    // Build FPF context if requested
    let fpf_context = if fpf {
        match reason::build_fpf_context(&store, &record.title, &record.body).await {
            Ok(ctx) => {
                if ctx.is_some() {
                    println!("  FPF context injected into ADI prompt");
                } else {
                    println!("  No FPF sections found (run `forgeplan fpf ingest` first)");
                }
                ctx
            }
            Err(e) => {
                eprintln!("  Warning: FPF context lookup failed: {e}");
                None
            }
        }
    } else {
        None
    };

    println!(
        "  Analyzing {} with ADI cycle ({}/{})...\n",
        record.id, llm_config.provider, llm_config.model
    );

    let (analysis, adi_output) = reason::reason(
        &llm_config,
        &record.id,
        &record.title,
        &record.kind,
        &record.body,
        fpf_context.as_deref(),
        Some(&artifact_context),
    )
    .await?;

    if json {
        // Structured JSON output — use parsed AdiOutput when available
        if adi_output.raw_markdown.is_none() {
            let structured = serde_json::json!({
                "artifact_id": record.id,
                "artifact_kind": record.kind,
                "adi_output": adi_output,
                "depth": record.depth,
                "r_eff_score": record.r_eff_score,
            });
            println!("{}", serde_json::to_string_pretty(&structured)?);
        } else {
            // Fallback: raw analysis string
            let structured = serde_json::json!({
                "artifact_id": record.id,
                "artifact_kind": record.kind,
                "adi_analysis": analysis,
                "depth": record.depth,
                "r_eff_score": record.r_eff_score,
            });
            println!("{}", serde_json::to_string_pretty(&structured)?);
        }
    } else {
        println!("{}", analysis);
    }

    if save {
        let note_id = store.next_id("NOTE").await?;
        let note_title = format!("ADI analysis of {}", record.id);
        let note_body = if adi_output.raw_markdown.is_some() {
            analysis.clone()
        } else {
            serde_json::to_string_pretty(&adi_output)?
        };

        let new_artifact = NewArtifact {
            id: note_id.clone(),
            kind: "note".to_string(),
            status: "draft".to_string(),
            title: note_title,
            body: note_body,
            depth: "tactical".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
        };

        store.create_artifact(&new_artifact).await?;
        store.add_relation(&note_id, &record.id, "informs").await?;
        println!("  Saved as {} -> linked to {}", note_id, record.id);
    }

    Ok(())
}
