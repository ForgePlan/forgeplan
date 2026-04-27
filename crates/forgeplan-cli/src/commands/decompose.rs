use forgeplan_core::hints::{self, Hint};
use forgeplan_core::llm::decompose;

use crate::commands::common;

pub async fn run(prd_id: &str) -> anyhow::Result<()> {
    let (_ws, store) = common::open_store().await?;

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
    let hint_list = vec![
        Hint::info(format!("Create RFC for {}", record.id)).with_action(format!(
            "forgeplan new rfc \"<task title>\" && forgeplan link RFC-NNN {} --relation refines",
            record.id
        )),
    ];
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
