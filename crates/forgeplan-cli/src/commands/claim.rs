//! `forgeplan claim` — soft-coordination signal that an agent is working
//! on an artifact (PRD-057 Inc 3 + PRD-070 CLI parity).
//!
//! Mirrors the `forgeplan_claim` MCP tool: writes
//! `.forgeplan/claims/<ID>.yaml` with a TTL. Refuses (exit 1) if the claim
//! is already held by a different live agent — caller must wait, retry, or
//! ask the orchestrator to force-release.

use chrono::Duration;
use forgeplan_core::claim::{self, ClaimError, ClaimStore, DEFAULT_TTL};
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::workspace;

use crate::commands::common;

const MAX_TTL_MINUTES: u32 = 1440; // 24 h — matches claim::MAX_TTL.

/// Default agent identity when caller omits `--agent`. Uses `cli-<crate
/// version>` so each release has a stable signature while still being
/// distinguishable from an MCP sub-agent.
///
/// **Must use `-` not `/`**: the CLI surface runs the *strict*
/// [`claim::validate_agent_id`] which rejects `/` (it would let an operator
/// enshrine the canonical MCP `name/version` path-shape as a CLI argument).
/// A `cli/<version>` default therefore made bare `forgeplan claim <id>`
/// (no `--agent`) always fail validation — the dogfood v0.32.1 HIGH bug.
fn default_agent() -> String {
    format!("cli-{}", env!("CARGO_PKG_VERSION"))
}

pub async fn run(
    id: &str,
    agent: Option<&str>,
    ttl_minutes: Option<u32>,
    note: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let (ws, source, _read_guard) = common::open_read_source().await?;
    let resolution = match &source {
        common::ReadSource::Legacy(_) => common::resolve_against_workspace(&ws, id).await?,
        common::ReadSource::Captured(read) => common::resolve_against_capture(read, id),
    };
    let (selector, ref_form) = match &resolution.outcome {
        forgeplan_mcp::convert::Resolution::NotFound => (id.to_string(), id.to_string()),
        _ => common::require_found_identity(id, &resolution)?,
    };

    let agent_str = match agent.map(str::trim).filter(|a| !a.is_empty()) {
        Some(a) => a.to_string(),
        None => default_agent(),
    };

    // PROB-066: early CLI-side validation so the operator sees a typed
    // remediation hint *before* the workspace lock + filesystem write are
    // attempted. The same check runs again in ClaimStore::claim as
    // defense-in-depth (a future caller bypassing this surface still hits
    // the guard in core).
    if let Err(e) = claim::validate_agent_id(&agent_str) {
        anyhow::bail!(
            "Error: {e}\n\
             Fix: rename --agent to printable non-path characters only \
             (e.g. `smoke-test-v1` — alphanumerics, `-`, `_`; no `/`, controls, bidi)."
        );
    }

    let ttl = match ttl_minutes {
        Some(0) => anyhow::bail!("ttl-minutes must be >= 1"),
        Some(m) if m > MAX_TTL_MINUTES => anyhow::bail!(
            "ttl-minutes must be <= {MAX_TTL_MINUTES} (24 hours) — long-running work should renew \
             instead of holding a day-long claim"
        ),
        Some(m) => Duration::minutes(m as i64),
        None => DEFAULT_TTL,
    };

    drop(source);
    drop(_read_guard);
    let _lock = workspace::lock::acquire_workspace_lock(&ws).await?;
    let locked_source = common::read_source_for(&ws, &_lock).await?;
    let locked_resolution = match &locked_source {
        common::ReadSource::Legacy(_) => common::resolve_against_workspace(&ws, id).await?,
        common::ReadSource::Captured(read) => common::resolve_against_capture(read, id),
    };
    let locked_identity = match &locked_resolution.outcome {
        forgeplan_mcp::convert::Resolution::NotFound => (id.to_string(), id.to_string()),
        _ => common::require_found_identity(id, &locked_resolution)?,
    };
    if locked_identity != (selector.clone(), ref_form.clone()) {
        anyhow::bail!("claim selector changed while acquiring workspace lock");
    }
    common::ensure_ordinary_mutation_allowed(&ws)?;

    let store = ClaimStore::new(&ws);
    match store
        .claim_for(&selector, &ref_form, &agent_str, ttl, note.map(|s| s.to_string()))
        .await
    {
        Ok(claim) => {
            // Successful claim: next action is to start work on the
            // artifact (we use `forgeplan get` as a concrete inspection
            // step the agent should perform before editing).
            // PROB-060 (W1.B, CD-5) — emit ref_form so the inspect command
            // stays canonical for commit `Refs:`.
            let hint_list = vec![
                Hint::info(format!("Inspect claimed {}", ref_form))
                    .with_action(format!("forgeplan get {}", ref_form)),
            ];

            if json {
                let body = serde_json::json!({
                    "id": claim.id,
                    "canonical_key": claim.canonical_key,
                    "agent_id": claim.agent_id,
                    "claimed_at": claim.claimed_at.to_rfc3339(),
                    "expires_at": claim.expires_at.to_rfc3339(),
                    "note": claim.note,
                    "_next_action": hints::primary_action(&hint_list),
                    "hints": hint_list,
                });
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                println!("Claimed {} for {}", claim.id, claim.agent_id);
                // The key is the identity the claim is filed under; the alias
                // above may be shared by another document.
                if let Some(key) = claim.canonical_key.as_deref() {
                    println!("  Key:     {key}");
                }
                println!("  Expires: {}", claim.expires_at.to_rfc3339());
                if let Some(n) = &claim.note {
                    println!("  Note:    {n}");
                }
                println!(
                    "  Hint:    release with `forgeplan release {}` when done, or re-run \
                     `forgeplan claim {}` to renew before expiry.",
                    ref_form, ref_form,
                );
                print!("{}", hints::render_next_action_line(&hint_list));
            }
            Ok(())
        }
        Err(ClaimError::AlreadyHeld {
            id,
            agent_id,
            expires_at,
        }) => {
            // Conflict: orchestrator override path is the deterministic fix.
            // PROB-060 (W1.B, CD-5) — `ref_form` outer captures the canonical
            // reference for the artifact (slug pre-merge / display id
            // post-merge); the inner `id` from ClaimError is the raw filename
            // we just kept — but the outer ref_form is canonical so prefer it.
            let hint_list = vec![
                Hint::warning(format!("Claim held by {agent_id}"))
                    .with_action(format!("forgeplan release {} --force", ref_form)),
            ];

            if json {
                let body = serde_json::json!({
                    "error": "already_held",
                    "id": id,
                    "agent_id": agent_id,
                    "expires_at": expires_at.to_rfc3339(),
                    "_next_action": hints::primary_action(&hint_list),
                    "hints": hint_list,
                });
                println!("{}", serde_json::to_string_pretty(&body)?);
            } else {
                eprintln!("Claim for {id} already held by {agent_id} (expires {expires_at})");
                eprintln!("Fix: forgeplan release {ref_form} --force");
                eprintln!(
                    "  Hint: wait for TTL expiry, work on a different artifact, or ask the \
                     orchestrator to force-release with `forgeplan release {ref_form} --force`."
                );
            }
            std::process::exit(1);
        }
        Err(e) => anyhow::bail!("claim failed: {e}\nFix: forgeplan claims"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dogfood v0.32.1 HIGH fix (#1): the implicit `--agent` default MUST pass
    /// the *strict* CLI-side validator (`claim::validate_agent_id`, which
    /// rejects `/`). The old `cli/<version>` default contained a `/`, so bare
    /// `forgeplan claim <id>` always errored before this fix.
    #[test]
    fn default_agent_passes_strict_validator() {
        let agent = default_agent();
        assert!(
            !agent.contains('/'),
            "default agent must not contain '/': {agent}"
        );
        claim::validate_agent_id(&agent)
            .unwrap_or_else(|e| panic!("default agent `{agent}` rejected by validator: {e:?}"));
    }

    #[test]
    fn default_agent_has_cli_prefix_and_version() {
        let agent = default_agent();
        assert!(agent.starts_with("cli-"), "expected cli- prefix: {agent}");
        assert!(
            agent.len() > "cli-".len(),
            "expected a version suffix: {agent}"
        );
    }
}
