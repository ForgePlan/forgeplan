use forgeplan_core::hints::{self, Hint};
use forgeplan_core::llm::decompose;

use crate::commands::common;

pub async fn run(prd_id: &str) -> anyhow::Result<()> {
    let (_ws, _lock, store) = common::open_store_locked().await?;

    // PRD-071 contract: emit `Fix:` when LLM unavailable so the agent has a
    // deterministic remediation step instead of free-form prose.
    let llm_config = match common::require_llm_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Fix: forgeplan setup-skill");
            anyhow::bail!("LLM not configured");
        }
    };
    // PROB-060 / SPEC-005 Phase 2.6 (CD-6) — accept slug or display id.
    let prd_id = store.resolve_id(prd_id).await?.ok_or_else(|| {
        anyhow::anyhow!("Artifact '{prd_id}' not found\nFix: forgeplan list --type prd")
    })?;
    let prd_id = prd_id.as_str();
    let record = store.get_record(prd_id).await?.ok_or_else(|| {
        anyhow::anyhow!(
            "Artifact '{}' not found\n\
             Fix: forgeplan list --type prd",
            prd_id
        )
    })?;

    if record.kind != "prd" {
        eprintln!(
            "  Warning: '{}' is a {} (not a PRD). Decomposing anyway.",
            record.id, record.kind
        );
    }

    println!(
        "  Decomposing {} into RFC tasks ({}/{})...\n",
        record.id, llm_config.provider, llm_config.model
    );

    // PRD-071 contract: surface a `Fix:` line on LLM failure (rate limit, auth).
    let tasks =
        match decompose::decompose(&llm_config, &record.id, &record.title, &record.body).await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Error: decomposition failed: {}", e);
                eprintln!("Fix: forgeplan setup-skill");
                anyhow::bail!("LLM call failed");
            }
        };

    println!("{}", tasks);

    // PRD-071 ACTIONABILITY: target ID is real (`record.id`); `RFC-NNN` is the
    // allowed value-to-fill placeholder for the yet-to-exist RFC.
    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — slug pre-merge / display
    // id post-merge so the link command stays canonical for commit `Refs:`.
    //
    // **HIGH-3 (Round-1 audit, CWE-117 / prompt injection)**: even though
    // `refs_form_from_body` already drops slugs that fail SPEC-005's
    // grammar (added in this fix-1c), we apply `sanitize_for_hint` as a
    // defence-in-depth layer. This catches any future regression in the
    // slug grammar gate and aligns CLI hint emission with MCP server
    // (which has always sanitized).
    let raw_ref =
        forgeplan_core::artifact::frontmatter::refs_form_from_body(&record.body, &record.id);
    let ref_form = forgeplan_core::artifact::sanitize::sanitize_for_hint(&raw_ref);
    let hint_list = vec![
        Hint::info(format!("Create RFC for {}", ref_form)).with_action(format!(
            "forgeplan new rfc \"<task title>\" && forgeplan link RFC-NNN {} --relation refines",
            ref_form
        )),
    ];
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
