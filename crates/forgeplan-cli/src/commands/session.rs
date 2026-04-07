//! `forgeplan session` — show or reset the methodology session state.

use console::style;

use crate::commands::common;

/// Show current session state.
pub fn run_status() {
    let session = common::load_session();

    println!();
    println!("{}", style("Session State").bold());
    println!("{}", style("─".repeat(40)).dim());
    println!("  Phase:    {}", style(session.phase.to_string()).cyan());

    if let Some(ref artifact) = session.active_artifact {
        println!("  Artifact: {}", style(artifact).bold());
    }

    if let Some(ref depth) = session.route_depth {
        let enforced = if session.is_enforced() {
            "enforced"
        } else {
            "free"
        };
        println!(
            "  Depth:    {} ({})",
            style(depth).yellow(),
            style(enforced).dim()
        );
    }

    if let Some(ref started) = session.phase_started_at {
        println!("  Since:    {}", style(started).dim());
    }

    println!();
    println!("  Next: {}", style(session.next_action_hint()).green());

    if !session.history.is_empty() {
        println!();
        println!("  History (last {}):", session.history.len());
        for t in session.history.iter().rev().take(5) {
            println!(
                "    {} → {} {}",
                style(t.from.to_string()).dim(),
                style(t.to.to_string()).cyan(),
                t.artifact
                    .as_deref()
                    .map(|a| format!("({})", a))
                    .unwrap_or_default()
            );
        }
    }
    println!();
}

/// Reset session to Idle.
pub fn run_reset() {
    let mut session = common::load_session();
    session.phase = forgeplan_core::session::Phase::Idle;
    session.active_artifact = None;
    session.route_depth = None;
    session.phase_started_at = None;
    common::save_session(&session);
    println!("  Session reset to Idle");
}
