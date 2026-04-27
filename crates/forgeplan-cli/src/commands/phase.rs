// PRD-056 (EPIC-005): CLI parity for `forgeplan_phase` MCP tool.
//
// Reads advisory phase state for an artifact from
// `.forgeplan/state/<id>.yaml`. Missing state is NOT an error — it
// reports `current_phase: unknown` with a hint on how to start tracking.
// Mirrors `forgeplan_phase` semantics from the MCP server (PRD-056 FR-012).

use console::style;
use forgeplan_core::hints::{self, Hint};
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
    // PRD-071 contract: derive a single Next: action from the state. If a
    // suggested next phase exists, point at the full advance command. Else
    // (terminal phase) emit no action — handled as Done. terminal status.
    let mut hints_vec: Vec<Hint> = Vec::new();
    if let Some(next) = state.current_phase.suggested_next() {
        hints_vec.push(
            Hint::suggestion(format!("Advance to {}", next.as_str())).with_action(format!(
                "forgeplan phase-advance {} --to {}",
                state.artifact_id,
                next.as_str()
            )),
        );
    }

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
            "_next_action": hints::primary_action(&hints_vec),
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

    // PRD-071 contract: terminal Next:/Done line for the CLI text surface.
    match hints::primary_action(&hints_vec) {
        Some(cmd) => println!("\nNext: {}", cmd),
        None => println!("\nDone."),
    }
}

fn print_unknown(id: &str, json: bool) {
    // PRD-071 contract: bootstrap the phase tracking by suggesting a `shape` advance.
    let hints_vec = vec![
        Hint::suggestion("Start phase tracking")
            .with_action(format!("forgeplan phase-advance {} --to shape", id)),
    ];

    if json {
        let payload = serde_json::json!({
            "artifact_id": id,
            "current_phase": "unknown",
            "workflow_type": "greenfield",
            "history": Vec::<serde_json::Value>::new(),
            "message": "No phase state file on disk -- advisory only, never an error",
            "_next_action": hints::primary_action(&hints_vec),
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
    print!("{}", hints::render_next_action_line(&hints_vec));
}
