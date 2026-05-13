use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::fpf::contexts;
use forgeplan_core::fpf::core::adi::AdiRecord;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::llm::reason;
use forgeplan_core::llm::reason::ArtifactContext;
use forgeplan_core::projection;

use crate::commands::common;

/// Default architecture hint when no custom file exists.
const DEFAULT_ARCHITECTURE_HINT: &str = "\
Forgeplan is a Rust CLI + MCP server. \
Storage: LanceDB (embedded, tables + vectors). \
Architecture: forgeplan-core (shared library) + forgeplan-cli + forgeplan-mcp. \
Driver traits: StorageDriver, EmbedDriver, MemoryDriver, LlmDriver. \
Embedding: local BGE-M3 via fastembed (no API needed). \
Files in .forgeplan/ are authoritative, LanceDB syncs from them.";

/// Load architecture hint: .forgeplan/prompts/architecture.md if exists, else default.
fn load_architecture_hint() -> String {
    let custom_path = std::path::Path::new(".forgeplan/prompts/architecture.md");
    if custom_path.exists()
        && let Ok(content) = std::fs::read_to_string(custom_path)
    {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    DEFAULT_ARCHITECTURE_HINT.to_string()
}

pub async fn run(id: &str, json: bool, save: bool, fpf: bool) -> anyhow::Result<()> {
    // Audit 2026-05-01 H4: do NOT hold the workspace lock across the LLM
    // call (10–60 s). Open lock-free for the read + LLM phases; re-acquire
    // only for the brief save block. Otherwise every concurrent CLI
    // mutation in a multi-agent workspace would time out at the 30 s
    // default lock timeout.
    let (_ws, store) = common::open_store().await?;

    // PRD-071 hint contract + PRD-077 FR-008: when LLM is unavailable, emit a
    // structured `Fix:` line so the agent has a deterministic next step. The
    // underlying error from `require_llm_config` already contains a `Fix:`
    // marker + copy-paste secrets.yaml snippet; we additionally surface the
    // setup-skill shortcut for agents that prefer the guided workflow.
    let llm_config = match common::require_llm_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!(
                "Fix: edit .forgeplan/config.yaml::llm or export the API key in \
                 .forgeplan/secrets.yaml (see `forgeplan setup-skill` for a guided walk-through)"
            );
            anyhow::bail!("LLM not configured");
        }
    };
    // PROB-060 / SPEC-005 Phase 2.6 (CD-6) — accept slug or display id.
    let id = store
        .resolve_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;
    let id = id.as_str();
    let record = store.get_record(id).await?.ok_or_else(|| {
        anyhow::anyhow!(
            "Artifact '{}' not found\n\
             Fix: forgeplan list",
            id
        )
    })?;

    // Build artifact context from store metadata, enriching relations with titles
    let raw_relations = store.get_relations(&record.id).await.unwrap_or_default();
    let mut relations = Vec::with_capacity(raw_relations.len());
    for (target_id, rel_type) in &raw_relations {
        let title = store
            .get_record(target_id)
            .await
            .ok()
            .flatten()
            .map(|r| r.title)
            .unwrap_or_default();
        relations.push((target_id.clone(), rel_type.clone(), title));
    }
    // Detect bounded context for this artifact
    let bounded_context = contexts::detect_for_artifact(&store, &record.id)
        .await
        .unwrap_or(None);

    let artifact_context = ArtifactContext {
        status: record.status.clone(),
        depth: record.depth.clone(),
        r_eff_score: record.r_eff_score,
        relations,
        architecture_hint: Some(load_architecture_hint()),
        bounded_context,
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

    // PRD-071 contract: LLM call failures (rate limit, auth, network) get a
    // `Fix:` marker so the agent has a deterministic next step.
    let (analysis, adi_output) = match reason::reason(
        &llm_config,
        &record.id,
        &record.title,
        &record.kind,
        &record.body,
        fpf_context.as_deref(),
        Some(&artifact_context),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            // PRD-077 FR-008: classify the failure so the Fix: hint points at
            // the actual remediation (auth → rotate key; rate-limit → wait;
            // network → check connectivity). Heuristic match on the error
            // string keeps us decoupled from LLM-provider-specific error
            // types — substring search is intentionally tolerant.
            let msg = e.to_string();
            let lower = msg.to_ascii_lowercase();
            eprintln!("Error: ADI reasoning failed: {}", msg);
            let env = llm_config
                .api_key_env
                .as_deref()
                .unwrap_or("GEMINI_API_KEY");
            if lower.contains("auth") || lower.contains("401") || lower.contains("403") {
                eprintln!(
                    "Fix: API key rejected by `{provider}` — rotate `{env}` in \
                     .forgeplan/secrets.yaml (or run `forgeplan setup-skill`)",
                    provider = llm_config.provider,
                );
            } else if lower.contains("rate") || lower.contains("429") || lower.contains("quota") {
                eprintln!(
                    "Fix: rate-limit hit on `{provider}` — wait 60 s and retry, \
                     or switch model in .forgeplan/config.yaml::llm.model",
                    provider = llm_config.provider,
                );
            } else if lower.contains("network")
                || lower.contains("timeout")
                || lower.contains("dns")
            {
                eprintln!(
                    "Fix: network error reaching `{provider}` — check connectivity, \
                     then retry `forgeplan reason {id}`",
                    provider = llm_config.provider,
                    id = record.id,
                );
            } else {
                eprintln!(
                    "Fix: verify .forgeplan/config.yaml::llm and `{env}` in \
                     .forgeplan/secrets.yaml; rerun `forgeplan reason {id}` (or \
                     `forgeplan setup-skill` for guided fix)",
                    id = record.id,
                );
            }
            anyhow::bail!("LLM call failed");
        }
    };

    // PRD-071 contract: deterministic Next: action for the agent — verify R_eff
    // after ADI. If evidence_needed is non-empty, point at evidence creation
    // first (the prerequisite for a meaningful score).
    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — emit slug pre-merge and
    // display id post-merge so the agent's next command keeps canonical
    // refs in commit messages.
    //
    // **HIGH-3 (Round-1 audit, CWE-117 / prompt injection)**: defence in
    // depth — even though `refs_form_from_body` rejects slugs failing the
    // SPEC-005 grammar, we sanitize the result before splicing it into
    // hint strings. Mirrors MCP server's existing `sanitize_for_hint`
    // discipline.
    let raw_ref =
        forgeplan_core::artifact::frontmatter::refs_form_from_body(&record.body, &record.id);
    let ref_form = forgeplan_core::artifact::sanitize::sanitize_for_hint(&raw_ref);
    let mut hints_vec: Vec<Hint> = Vec::new();
    if !adi_output.evidence_needed.is_empty() {
        hints_vec.push(
            Hint::suggestion("Add the missing evidence flagged by ADI").with_action(format!(
                "forgeplan new evidence \"<verification>\" && forgeplan link EVID-XXX {} --relation informs",
                ref_form
            )),
        );
    } else {
        hints_vec.push(
            Hint::suggestion("Verify R_eff after ADI")
                .with_action(format!("forgeplan score {}", ref_form)),
        );
    }

    if json {
        // Structured JSON output — use parsed AdiOutput when available
        if adi_output.raw_markdown.is_none() {
            let structured = serde_json::json!({
                "artifact_id": record.id,
                "artifact_kind": record.kind,
                "adi_output": adi_output,
                "depth": record.depth,
                "r_eff_score": record.r_eff_score,
                "_next_action": hints::primary_action(&hints_vec),
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
                "_next_action": hints::primary_action(&hints_vec),
            });
            println!("{}", serde_json::to_string_pretty(&structured)?);
        }
    } else {
        println!("{}", analysis);
    }

    // Suggest evidence creation for missing evidence items
    if !json && !adi_output.evidence_needed.is_empty() {
        println!("\n  --- Next steps (evidence needed) ---");
        for ev in &adi_output.evidence_needed {
            println!("  {} [{}]: {}", ev.for_hypothesis, ev.effort, ev.test);
        }
        println!(
            "\n  Tip: forgeplan new evidence \"<description>\"  # then link to {}",
            ref_form
        );
    }

    if save {
        // Re-open under the workspace lock for the brief write phase.
        // Drops the lock-free `store` first, then acquires the locked
        // store + lock guard scoped to this block only.
        drop(store);
        let (ws, _lock, store) = common::open_store_locked().await?;
        let note_id = store.next_id("NOTE").await?;

        // Convert LLM output to structured AdiRecord
        let adi_record = if adi_output.raw_markdown.is_none() {
            let model_name = format!("{}/{}", llm_config.provider, llm_config.model);
            Some(AdiRecord::from_adi_output(
                note_id.clone(),
                record.id.clone(),
                model_name,
                &adi_output,
            ))
        } else {
            None
        };

        let note_title = format!("ADI analysis of {}", record.id);
        let note_body = if let Some(ref adi_rec) = adi_record {
            // Structured: AdiRecord JSON + readable summary
            let summary = format!(
                "# ADI Record: {}\n\n\
                 **Artifact**: {} ({})\n\
                 **Model**: {}\n\
                 **Hypotheses**: {}\n\
                 **Confidence**: {}\n\
                 **Recommendation**: {}\n\n\
                 ## Structured Data\n\n\
                 ```json\n{}\n```",
                adi_rec.id,
                adi_rec.artifact_id,
                record.kind,
                adi_rec.model,
                adi_rec.hypotheses.len(),
                adi_rec.confidence,
                adi_rec.recommendation,
                adi_rec.to_json_body(),
            );
            summary
        } else {
            // Fallback: raw markdown
            analysis.clone()
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
            // C1: ADI reasoning notes are untagged; the `informs` link carries provenance.
            tags: Vec::new(),
        };

        // PRD-073 file-first: helpers handle projection writes for both
        // the new note and the bidirectional link rendering.
        let ctx = projection::MutationContext::new(&ws, &store);
        projection::create_artifact_with_projection(&ctx, &new_artifact).await?;
        projection::add_link_with_projection(&ctx, &note_id, &record.id, "informs").await?;
        println!("  Saved as {} -> linked to {}", note_id, record.id);
    }

    // PRD-071 contract: terminal Next: line in CLI text mode (json already handled).
    if !json {
        print!("{}", hints::render_next_action_line(&hints_vec));
    }

    Ok(())
}
