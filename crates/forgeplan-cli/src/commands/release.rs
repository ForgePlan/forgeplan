//! `forgeplan release` — drop a claim on an artifact (PRD-057 Inc 3 +
//! PRD-070 CLI parity).
//!
//! Mirrors `forgeplan_release` MCP tool: removes the claim file and is
//! idempotent (missing claim = success). `--force` is the orchestrator
//! escape hatch to reap a crashed sub-agent's claim.

use forgeplan_core::claim::{ClaimError, ClaimStore};
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::workspace;

use crate::commands::common;

fn default_agent() -> String {
    format!("cli/{}", env!("CARGO_PKG_VERSION"))
}

pub async fn run(id: &str, agent: Option<&str>, force: bool, json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let (ws, source, _read_guard) = common::open_read_source().await?;
    let resolution = match &source {
        common::ReadSource::Legacy(_) => common::resolve_against_workspace(&ws, id).await?,
        common::ReadSource::Captured(read) => common::resolve_against_capture(read, id),
    };
    let (id_owned, ref_form) = match &resolution.outcome {
        forgeplan_mcp::convert::Resolution::NotFound => (id.to_string(), id.to_string()),
        _ => common::require_found_identity(id, &resolution)?,
    };
    let original_selector = id;
    let id = id_owned.as_str();

    // Match MCP semantics: explicit agent > default agent string. With
    // `--force` and no agent, the empty string is acceptable (the core
    // path waives the agent check on force=true).
    let agent_str = match agent.map(str::trim).filter(|a| !a.is_empty()) {
        Some(a) => a.to_string(),
        None if force => String::new(),
        None => default_agent(),
    };

    drop(source);
    drop(_read_guard);
    let _lock = workspace::lock::acquire_workspace_lock(&ws).await?;
    let locked_source = common::read_source_for(&ws, &_lock).await?;
    let locked_resolution = match &locked_source {
        common::ReadSource::Legacy(_) => common::resolve_against_workspace(&ws, original_selector).await?,
        common::ReadSource::Captured(read) => common::resolve_against_capture(read, original_selector),
    };
    let locked_identity = match &locked_resolution.outcome {
        forgeplan_mcp::convert::Resolution::NotFound => (original_selector.to_string(), original_selector.to_string()),
        _ => common::require_found_identity(original_selector, &locked_resolution)?,
    };
    if locked_identity != (id_owned.clone(), ref_form.clone()) {
        anyhow::bail!("release selector changed while acquiring workspace lock");
    }
    common::ensure_ordinary_mutation_allowed(&ws)?;

    let store = ClaimStore::new(&ws);
    match store.release_for(id, &ref_form, &agent_str, force).await {
        Ok(()) => {
            // PRD-071: re-plan after a successful release so the orchestrator
            // (or solo agent) immediately sees the freed slot.
            let next_hints: Vec<Hint> = vec![
                Hint::info("Slot freed — re-plan dispatch")
                    .with_action("forgeplan dispatch --agents 3"),
            ];

            if json {
                let body = serde_json::json!({
                    "id": ref_form,
                    "released": true,
                    "force": force,
                    "_next_action": hints::primary_action(&next_hints),
                });
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                println!("Released claim on {ref_form}");
                if force {
                    println!("  (forced — orchestrator override)");
                }
                print!("{}", hints::render_next_action_line(&next_hints));
            }
            Ok(())
        }
        Err(ClaimError::NotHeldByRequester { held_by, .. }) => {
            // PROB-095: this used to offer `--force` as the primary fix and
            // call it "the only safe escape hatch". It is neither.
            //
            // `--force` is the ORCHESTRATOR override: it drops the claim
            // regardless of who holds it. PRD-071 obliges an agent to run
            // `Fix:` as given, so pointing a peer agent at it meant the hint
            // contract instructed agents to break the very coordination they
            // had just collided with — one obedient agent away from two
            // writers in one artifact.
            //
            // The overwhelmingly common cause is far duller: the caller IS the
            // holder and simply did not say so, because `release` defaults to
            // `cli/<version>` rather than inheriting the `claim --agent`
            // identity. So the primary fix names the holder; force stays
            // reachable as an explicit `Or:` for the orchestrator that really
            // does mean to take it.
            //
            // PROB-060 (W1.B, CD-5) — emit ref_form so both commands stay
            // canonical for commit `Refs:`.
            let fix_hints: Vec<Hint> = vec![
                Hint::warning(format!(
                    "Claim held by {held_by} — release as that identity"
                ))
                .with_action(format!("forgeplan release {ref_form} --agent {held_by}")),
            ];
            let override_cmd = format!("forgeplan release {ref_form} --force");

            if json {
                let body = serde_json::json!({
                    "error": "not_held_by_requester",
                    "id": id,
                    "held_by": held_by,
                    "_next_action": hints::primary_action(&fix_hints),
                    "_alternative_action": override_cmd,
                });
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                eprintln!("Error: Claim on {id} held by {held_by}, not by requester");
                if let Some(fix) = hints::primary_action(&fix_hints) {
                    eprintln!("Fix: {}", fix);
                }
                eprintln!(
                    "Or: {override_cmd}  (orchestrator override — drops another agent's claim)"
                );
            }
            std::process::exit(1);
        }
        Err(e) => anyhow::bail!("release failed: {e}"),
    }
}
