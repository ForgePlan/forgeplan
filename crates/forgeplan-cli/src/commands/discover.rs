//! forgeplan discover — brownfield discovery CLI (PRD-035 FR-007)
//!
//! Starts a discovery session and prints the protocol for an AI agent to follow.
//! Agent reads the protocol, analyzes the codebase, and reports findings via MCP.
//! This CLI is primarily a human-facing status/inspection tool.

use anyhow::{Context, Result};
use forgeplan_core::discover::{
    DiscoverSession, Phase, Protocol, SessionStatus, list_sessions, load_session, save_session,
};
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

/// Start a new discovery session and print the protocol.
pub async fn run_start(name: &str) -> Result<()> {
    let (workspace, _store) = common::open_store().await?;

    let session = DiscoverSession::new(name);
    save_session(&workspace, &session)
        .with_context(|| format!("Failed to save session {}", session.id))?;

    let protocol = Protocol::default();

    println!();
    println!("  Discovery session started");
    println!("  ─────────────────────────");
    println!("  Session ID:   {}", session.id);
    println!("  Project:      {}", session.project_name);
    println!(
        "  Started:      {}",
        session.started_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!();
    println!("  Protocol (v{}) — 7 phases:", protocol.version);
    println!();

    for phase_inst in &protocol.phases {
        println!(
            "  {}. {} ({})",
            phase_inst.order,
            phase_inst.name.to_uppercase(),
            phase_inst.phase.name()
        );
        println!("     {}", phase_inst.instructions);
        println!();
    }

    println!("  Source Tier Rules:");
    println!(
        "    Tier 1 (truth):       {}",
        protocol.source_tier_rules.t1.join(", ")
    );
    println!(
        "    Tier 2 (extracted):   {}",
        protocol.source_tier_rules.t2.join(", ")
    );
    println!(
        "    Tier 3 (supplement):  {}",
        protocol.source_tier_rules.t3.join(", ")
    );
    println!();
    println!("  → AI Agent: follow the phases in order, call forgeplan_discover_finding for each");
    println!("    discovery, then forgeplan_discover_complete when done.");
    println!();
    println!(
        "  → Human: track progress with `forgeplan discover show {}`",
        session.id
    );
    println!();

    let hint_list = vec![
        Hint::info(format!("Track session {}", session.id))
            .with_action(format!("forgeplan discover show {}", session.id)),
    ];
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}

/// List all discovery sessions.
pub async fn run_list() -> Result<()> {
    let (workspace, _store) = common::open_store().await?;

    let sessions = list_sessions(&workspace)?;

    if sessions.is_empty() {
        println!("  No discovery sessions found.");
        println!("  Start one: forgeplan discover start <project-name>");
        let hint_list = vec![
            Hint::info("Start a discovery session")
                .with_action("forgeplan discover start <project-name>".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&hint_list));
        return Ok(());
    }

    println!();
    println!(
        "  {:<25} {:<20} {:<12} {:<10} {:<10}",
        "Session ID", "Project", "Status", "Phase", "Findings"
    );
    println!("  {}", "─".repeat(80));

    for s in &sessions {
        println!(
            "  {:<25} {:<20} {:<12} {:<10} {:<10}",
            s.id,
            truncate(&s.project_name, 18),
            format!("{:?}", s.status).to_lowercase(),
            s.current_phase.name(),
            s.findings.len()
        );
    }

    println!();
    println!("  {} session(s) total", sessions.len());
    println!();

    // Pick the first in-progress session as the next-action target.
    let active = sessions
        .iter()
        .find(|s| s.status != SessionStatus::Completed)
        .or_else(|| sessions.first());
    let hint_list = if let Some(s) = active {
        vec![
            Hint::info(format!("Inspect session {}", s.id))
                .with_action(format!("forgeplan discover show {}", s.id)),
        ]
    } else {
        Vec::new()
    };
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}

/// Show details of a specific session.
pub async fn run_show(session_id: &str) -> Result<()> {
    let (workspace, _store) = common::open_store().await?;

    let session = load_session(&workspace, session_id)
        .with_context(|| format!("Session '{}' not found", session_id))?;

    println!();
    println!("  {}", session.id);
    println!("  {}", "─".repeat(session.id.len()));
    println!("  Project:       {}", session.project_name);
    println!("  Status:        {:?}", session.status);
    println!("  Current phase: {}", session.current_phase.name());
    println!(
        "  Started:       {}",
        session.started_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    if let Some(completed) = session.completed_at {
        println!(
            "  Completed:     {}",
            completed.format("%Y-%m-%d %H:%M:%S UTC")
        );
    }
    println!();

    // Phase counts
    let phase_counts = session.phase_counts();
    if !phase_counts.is_empty() {
        println!("  Findings by phase:");
        for phase in Phase::all() {
            let count = phase_counts.get(phase).copied().unwrap_or(0);
            if count > 0 {
                println!("    {:<12} {:>3}", phase.name(), count);
            }
        }
        println!();
    }

    // Tier counts
    let tier_counts = session.tier_counts();
    if !tier_counts.is_empty() {
        println!("  Findings by tier:");
        for tier in 1..=3u8 {
            let count = tier_counts.get(&tier).copied().unwrap_or(0);
            if count > 0 {
                println!("    Tier {} ({:>3}): CL{}", tier, count, 4 - tier);
            }
        }
        println!();
    }

    // Recent findings
    if !session.findings.is_empty() {
        println!("  Recent findings (last 10):");
        for f in session.findings.iter().rev().take(10) {
            let artifact_ref = f.artifact_id.as_deref().unwrap_or("(pending)");
            println!(
                "    [{}] {:<6} {} — {}",
                f.phase.name(),
                format!("T{}", f.tier),
                artifact_ref,
                truncate(&f.title, 50)
            );
        }
        println!();
    }

    println!("  Total findings: {}", session.findings.len());
    println!();

    let hint_list = if session.status == SessionStatus::Completed {
        vec![
            Hint::info("Validate post-discovery health")
                .with_action("forgeplan health".to_string()),
        ]
    } else {
        vec![
            Hint::info(format!("Complete session {}", session.id))
                .with_action(format!("forgeplan discover complete {}", session.id)),
        ]
    };
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}

/// Mark a session as completed.
pub async fn run_complete(session_id: &str) -> Result<()> {
    let (workspace, _store) = common::open_store().await?;

    let mut session = load_session(&workspace, session_id)
        .with_context(|| format!("Session '{}' not found", session_id))?;

    if session.status == SessionStatus::Completed {
        println!("  Session {} already completed.", session_id);
        let hint_list = vec![
            Hint::info("Validate post-discovery health")
                .with_action("forgeplan health".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&hint_list));
        return Ok(());
    }

    session.complete();
    save_session(&workspace, &session)?;

    println!();
    println!("  ✓ Session {} completed", session.id);
    println!("  Total findings: {}", session.findings.len());
    if let Some(completed) = session.completed_at {
        println!(
            "  Completed at:   {}",
            completed.format("%Y-%m-%d %H:%M:%S UTC")
        );
    }
    println!();
    println!("  Next: forgeplan health  (validate discovery)");
    println!();

    let hint_list =
        vec![Hint::info("Validate discovery output").with_action("forgeplan health".to_string())];
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}
