//! `forgeplan claim` — soft-coordination signal that an agent is working
//! on an artifact (PRD-057 Inc 3 + PRD-070 CLI parity).
//!
//! Mirrors the `forgeplan_claim` MCP tool: writes
//! `.forgeplan/claims/<ID>.yaml` with a TTL. Refuses (exit 1) if the claim
//! is already held by a different live agent — caller must wait, retry, or
//! ask the orchestrator to force-release.

use chrono::Duration;
use forgeplan_core::claim::{ClaimError, ClaimStore, DEFAULT_TTL};
use forgeplan_core::workspace;

const MAX_TTL_MINUTES: u32 = 1440; // 24 h — matches claim::MAX_TTL.

/// Default agent identity when caller omits `--agent`. Mirrors the MCP
/// fallback: `cli/<crate version>` so each release has a stable signature
/// while still being distinguishable from an MCP sub-agent.
fn default_agent() -> String {
    format!("cli/{}", env!("CARGO_PKG_VERSION"))
}

pub async fn run(
    id: &str,
    agent: Option<&str>,
    ttl_minutes: Option<u32>,
    note: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let agent_str = match agent.map(str::trim).filter(|a| !a.is_empty()) {
        Some(a) => a.to_string(),
        None => default_agent(),
    };

    let ttl = match ttl_minutes {
        Some(0) => anyhow::bail!("ttl-minutes must be >= 1"),
        Some(m) if m > MAX_TTL_MINUTES => anyhow::bail!(
            "ttl-minutes must be <= {MAX_TTL_MINUTES} (24 hours) — long-running work should renew \
             instead of holding a day-long claim"
        ),
        Some(m) => Duration::minutes(m as i64),
        None => DEFAULT_TTL,
    };

    let store = ClaimStore::new(&ws);
    match store
        .claim(id, &agent_str, ttl, note.map(|s| s.to_string()))
        .await
    {
        Ok(claim) => {
            if json {
                let body = serde_json::json!({
                    "id": claim.id,
                    "agent_id": claim.agent_id,
                    "claimed_at": claim.claimed_at.to_rfc3339(),
                    "expires_at": claim.expires_at.to_rfc3339(),
                    "note": claim.note,
                });
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                println!("Claimed {} for {}", claim.id, claim.agent_id);
                println!("  Expires: {}", claim.expires_at.to_rfc3339());
                if let Some(n) = &claim.note {
                    println!("  Note:    {n}");
                }
                println!(
                    "  Hint:    release with `forgeplan release {}` when done, or re-run \
                     `forgeplan claim {}` to renew before expiry.",
                    claim.id, claim.id,
                );
            }
            Ok(())
        }
        Err(ClaimError::AlreadyHeld {
            id,
            agent_id,
            expires_at,
        }) => {
            if json {
                let body = serde_json::json!({
                    "error": "already_held",
                    "id": id,
                    "agent_id": agent_id,
                    "expires_at": expires_at.to_rfc3339(),
                });
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                eprintln!("Claim for {id} already held by {agent_id} (expires {expires_at})");
                eprintln!(
                    "  Hint: wait for TTL expiry, work on a different artifact, or ask the \
                     orchestrator to force-release with `forgeplan release {id} --force`."
                );
            }
            std::process::exit(1);
        }
        Err(e) => anyhow::bail!("claim failed: {e}"),
    }
}
