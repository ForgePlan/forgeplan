use std::env;

use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::llm::reason;
use forgeplan_core::workspace::{self, load_config};

pub async fn run(id: &str, json: bool, save: bool) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let config = load_config(&ws)?;
    let llm_config = config.llm.unwrap_or_default().with_env_overrides();

    let store = LanceStore::open(&ws).await?;
    let record = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", id))?;

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
