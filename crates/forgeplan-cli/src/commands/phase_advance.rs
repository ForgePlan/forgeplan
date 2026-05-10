// PRD-056 (EPIC-005): CLI parity for `forgeplan_phase_advance` MCP tool.
//
// Manually advances (or sets) the advisory phase marker for an artifact.
// Appends a transition to the history. Does NOT validate phase ordering --
// advisory layer allows out-of-order jumps. Full enforcement lands in a
// later PRD under EPIC-005.

use console::style;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::phase::{self, Phase};
use forgeplan_core::workspace;

use crate::commands::common;

/// Phases accepted on the CLI. Lower-case snake_case mirrors the MCP
/// `PhaseArg` enum and the on-disk YAML representation. `Unknown` is
/// intentionally NOT exposed as an advance target -- callers cannot
/// unset a phase.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
#[clap(rename_all = "snake_case")]
pub enum PhaseArg {
    Shape,
    Validate,
    Adi,
    Code,
    Test,
    Audit,
    Evidence,
    Done,
}

impl From<PhaseArg> for Phase {
    fn from(a: PhaseArg) -> Self {
        match a {
            PhaseArg::Shape => Phase::Shape,
            PhaseArg::Validate => Phase::Validate,
            PhaseArg::Adi => Phase::Adi,
            PhaseArg::Code => Phase::Code,
            PhaseArg::Test => Phase::Test,
            PhaseArg::Audit => Phase::Audit,
            PhaseArg::Evidence => Phase::Evidence,
            PhaseArg::Done => Phase::Done,
        }
    }
}

/// Manually advance (or set) the advisory phase marker for an artifact.
/// Appends a transition to the history. Does NOT validate phase ordering --
/// advisory layer allows out-of-order jumps (e.g. direct `done` override).
/// Full phase enforcement lands in a later PRD under EPIC-005. Use when
/// auto-advancement missed a transition or when reclassifying workflow state.
pub async fn run(id: &str, to: PhaseArg, reason: Option<&str>, json: bool) -> anyhow::Result<()> {
    // Boundary cap mirrored from MCP server.rs M-sec #2 (Audit Round 2):
    // reject MB-scale reasons before they hit the core layer (which
    // truncates as defense-in-depth at MAX_REASON_LEN=512).
    if let Some(r) = reason
        && r.len() > 4096
    {
        anyhow::bail!("reason too long: {} bytes (max: 4096)", r.len());
    }

    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    // Phase 2.5 (PROB-060) — accept slug или display id form. Phase state
    // file named by canonical id; without resolver slug input creates
    // a NEW state file at .forgeplan/state/<slug>.yaml instead of updating
    // existing canonical one — silent state divergence.
    let store = common::store().await?;
    let canonical = store
        .resolve_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found"))?;

    let target: Phase = to.into();
    let state =
        phase::store::advance_phase(&ws, &canonical, target, reason.map(|s| s.to_string())).await?;

    // PRD-071 contract: produce a single Next: action — the next suggested phase
    // advance. No suggestion when at terminal phase (Done.).
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
        let payload = serde_json::json!({
            "artifact_id": state.artifact_id,
            "current_phase": state.current_phase.as_str(),
            "workflow_type": format!("{:?}", state.workflow_type).to_lowercase(),
            "advanced_at": state.advanced_at,
            "history_entries": state.history.len(),
            "reason": reason,
            "suggested_next": state.current_phase.suggested_next().map(|p| p.as_str()),
            "_next_action": hints::primary_action(&hints_vec),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_default()
        );
        return Ok(());
    }

    let current = state.current_phase.as_str();
    println!(
        "{} advanced to {} ({} history entries)",
        style(&state.artifact_id).bold(),
        style(current).green().bold(),
        state.history.len(),
    );
    if let Some(r) = reason {
        println!("  reason: {}", r);
    }
    match state.current_phase.suggested_next() {
        Some(next) => println!("  suggested next: {}", style(next.as_str()).yellow()),
        None => println!("  terminal phase -- no further advancement recommended"),
    }

    // PRD-071 contract: terminal Next:/Done. line for CLI text consumers.
    match hints::primary_action(&hints_vec) {
        Some(cmd) => println!("\nNext: {}", cmd),
        None => println!("\nDone."),
    }
    Ok(())
}
