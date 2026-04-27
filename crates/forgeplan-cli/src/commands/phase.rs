// PRD-056 (EPIC-005): CLI parity for `forgeplan_phase` MCP tool.
//
// Reads advisory phase state for an artifact from
// `.forgeplan/state/<id>.yaml`. Missing state is NOT an error — it
// reports `current_phase: unknown` with a hint on how to start tracking.
// Mirrors `forgeplan_phase` semantics from the MCP server (PRD-056 FR-012).

use console::style;
use forgeplan_core::phase;
use forgeplan_core::workspace;

/// Read advisory phase state for an artifact. Returns current_phase,
/// workflow_type, timestamps, and the full append-only transition history
/// from `.forgeplan/state/<id>.yaml`. If no state file exists yet
/// (pre-PRD-056 artifact or phase tracking was disabled), returns
/// `current_phase: unknown` -- never an error. Phase tracking is advisory
/// and never blocks other tools.
pub async fn run(id: &str, json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    match phase::store::read_phase(&ws, id).await? {
        Some(state) => print_state(&state, json),
        None => print_unknown(id, json),
    }
    Ok(())
}

fn print_state(state: &phase::PhaseState, json: bool) {
    if json {
        // Full state -- history included.
        let payload = serde_json::json!({
            "artifact_id": state.artifact_id,
            "workflow_type": format!("{:?}", state.workflow_type).to_lowercase(),
            "current_phase": state.current_phase.as_str(),
            "advanced_at": state.advanced_at,
            "schema_version": state.schema_version,
            "history": state.history.iter().map(|t| serde_json::json!({
                "from": t.from.map(|p| p.as_str()),
                "to": t.to.as_str(),
                "at": t.at,
                "reason": t.reason,
            })).collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_default()
        );
        return;
    }

    let current = state.current_phase.as_str();
    let workflow = format!("{:?}", state.workflow_type).to_lowercase();
    println!(
        "{} phase: {} (workflow: {})",
        style(&state.artifact_id).bold(),
        style(current).cyan().bold(),
        workflow,
    );
    println!(
        "  last advanced: {}",
        state.advanced_at.format("%Y-%m-%d %H:%M:%S UTC")
    );

    let total = state.history.len();
    if total == 0 {
        println!("  history: (empty)");
    } else {
        let take = total.min(3);
        let skip = total - take;
        println!("  history (showing last {} of {}):", take, total);
        for t in state.history.iter().skip(skip) {
            let from = t
                .from
                .map(|p| p.as_str().to_string())
                .unwrap_or_else(|| "-".to_string());
            let reason = t
                .reason
                .as_deref()
                .map(|r| format!(" -- {}", r))
                .unwrap_or_default();
            println!(
                "    {} {} -> {}{}",
                t.at.format("%Y-%m-%d %H:%M"),
                from,
                t.to.as_str(),
                reason
            );
        }
    }

    match state.current_phase.suggested_next() {
        Some(next) => println!(
            "  suggested next: {} (override: `forgeplan phase-advance {} --to <phase>`)",
            style(next.as_str()).yellow(),
            state.artifact_id,
        ),
        None => println!("  terminal phase -- no further advancement recommended"),
    }
}

fn print_unknown(id: &str, json: bool) {
    if json {
        let payload = serde_json::json!({
            "artifact_id": id,
            "current_phase": "unknown",
            "workflow_type": "greenfield",
            "history": Vec::<serde_json::Value>::new(),
            "message": "No phase state file on disk -- advisory only, never an error",
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_default()
        );
        return;
    }
    println!(
        "{} phase: {} (no state file)",
        style(id).bold(),
        style("unknown").dim(),
    );
    println!(
        "  Typical for artifacts created before PRD-056 shipped, or when \
         `phase.enabled: false` in config."
    );
    println!(
        "  To start tracking: `forgeplan phase-advance {} --to shape`",
        id
    );
}
