//! `forgeplan release` — drop a claim on an artifact (PRD-057 Inc 3 +
//! PRD-070 CLI parity).
//!
//! Mirrors `forgeplan_release` MCP tool: removes the claim file and is
//! idempotent (missing claim = success). `--force` is the orchestrator
//! escape hatch to reap a crashed sub-agent's claim.

use forgeplan_core::claim::{ClaimError, ClaimStore};
use forgeplan_core::workspace;

fn default_agent() -> String {
    format!("cli/{}", env!("CARGO_PKG_VERSION"))
}

pub async fn run(id: &str, agent: Option<&str>, force: bool, json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    // Match MCP semantics: explicit agent > default agent string. With
    // `--force` and no agent, the empty string is acceptable (the core
    // path waives the agent check on force=true).
    let agent_str = match agent.map(str::trim).filter(|a| !a.is_empty()) {
        Some(a) => a.to_string(),
        None if force => String::new(),
        None => default_agent(),
    };

    let store = ClaimStore::new(&ws);
    match store.release(id, &agent_str, force).await {
        Ok(()) => {
            if json {
                let body = serde_json::json!({
                    "id": id,
                    "released": true,
                    "force": force,
                });
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                println!("Released claim on {id}");
                if force {
                    println!("  (forced — orchestrator override)");
                }
            }
            Ok(())
        }
        Err(ClaimError::NotHeldByRequester { held_by, .. }) => {
            if json {
                let body = serde_json::json!({
                    "error": "not_held_by_requester",
                    "id": id,
                    "held_by": held_by,
                });
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                eprintln!("Claim on {id} held by {held_by}, not by requester");
                eprintln!(
                    "  Hint: pass `--force` (orchestrator override) if the holder has crashed."
                );
            }
            std::process::exit(1);
        }
        Err(e) => anyhow::bail!("release failed: {e}"),
    }
}
