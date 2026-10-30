// PRD-056 (EPIC-005): CLI parity for `forgeplan_phase_advance` MCP tool.
//
// Manually advances (or sets) the advisory phase marker for an artifact.
// Appends a transition to the history. Does NOT validate phase ordering --
// advisory layer allows out-of-order jumps. Full enforcement lands in a
// later PRD under EPIC-005.

use console::style;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::phase::{self, Phase};

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

    // Serialize the phase store's read-modify-write without opening an
    // artifact index merely to find the workspace.
    let (ws, _lock) = common::open_workspace_locked().await?;
    let (_, source) = common::open_read_source().await?;

    // Phase 2.5 (PROB-060) — accept slug or display id form. Phase state
    // file named by canonical id; without resolver slug input creates
    // a NEW state file at .forgeplan/state/<slug>.yaml instead of updating
    // existing canonical one — silent state divergence.
    let (canonical, display) = common::resolve_identity(&source, &ws, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found"))?;
    // A state file written before the key existed carries the display id in
    // its name. The name resolved to exactly one document, so the file moves
    // to the key's name.
    phase::store::migrate_legacy_state(&ws, &display, &canonical).await?;

    let target: Phase = to.into();
    let state =
        // #330 — the explicit operator path may move a phase in either
        // direction; the guard belongs on automation, not on a deliberate
        // command someone typed.
        phase::store::advance_phase_unchecked_for(
            &ws,
            &canonical,
            &display,
            target,
            reason.map(|s| s.to_string()),
        )
        .await?;

    // PRD-071 contract: produce a single Next: action — the next suggested phase
    // advance. No suggestion when at terminal phase (Done.).
    let mut hints_vec: Vec<Hint> = Vec::new();
    if let Some(next) = state.current_phase.suggested_next() {
        hints_vec.push(
            Hint::suggestion(format!("Advance to {}", next.as_str())).with_action(format!(
                "forgeplan phase-advance {} --to {}",
                display,
                next.as_str()
            )),
        );
    }

    if json {
        let payload = serde_json::json!({
            "artifact_id": display,
            "canonical_key": canonical,
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
        style(&display).bold(),
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
