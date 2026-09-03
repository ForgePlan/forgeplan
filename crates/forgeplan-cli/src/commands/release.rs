//! `forgeplan release` — drop a claim on an artifact (PRD-057 Inc 3 +
//! PRD-070 CLI parity).
//!
//! Mirrors `forgeplan_release` MCP tool: removes the claim file and is
//! idempotent (missing claim = success). `--force` is the orchestrator
//! escape hatch to reap a crashed sub-agent's claim.

use forgeplan_core::claim::{ClaimError, ClaimStore};
use forgeplan_core::db::store::LanceStore;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::workspace;

fn default_agent() -> String {
    format!("cli/{}", env!("CARGO_PKG_VERSION"))
}

pub async fn run(id: &str, agent: Option<&str>, force: bool, json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    // PROB-060 / SPEC-005 Phase 2.6 (CD-6) — accept slug or display id.
    // Best-effort resolve: if the artifact isn't in LanceDB (e.g. claim
    // outlived its underlying record), fall back to raw input so the
    // operator can still drop the now-orphaned claim file.
    let lance = LanceStore::open(&ws).await?;
    let canonical = lance.resolve_id(id).await?;
    let id_owned: String;
    let id: &str = if let Some(c) = canonical {
        id_owned = c;
        id_owned.as_str()
    } else {
        id
    };

    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — derive ref_form for hint
    // emission (slug pre-merge / display id post-merge). Falls back to the
    // canonical id when the artifact is missing — release is idempotent and
    // we still want a runnable suggestion.
    let ref_form: String = match lance.get_record(id).await? {
        Some(rec) => forgeplan_core::artifact::frontmatter::refs_form_from_body(&rec.body, &rec.id),
        None => id.to_string(),
    };

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
            // PRD-071: re-plan after a successful release so the orchestrator
            // (or solo agent) immediately sees the freed slot.
            let next_hints: Vec<Hint> = vec![
                Hint::info("Slot freed — re-plan dispatch")
                    .with_action("forgeplan dispatch --agents 3"),
            ];

            if json {
                let body = serde_json::json!({
                    "id": id,
                    "released": true,
                    "force": force,
                    "_next_action": hints::primary_action(&next_hints),
                });
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                println!("Released claim on {id}");
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
