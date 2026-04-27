//! `forgeplan session` — show or reset the methodology session state.

use console::style;
use forgeplan_core::hints::{self, Hint};

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

    // PRD-071 contract: emit a single primary next-action keyed off the
    // current methodology phase. Idle → routing; everything else nudges
    // toward the next gate using the active artifact ID when available.
    use forgeplan_core::session::Phase;
    let next_hints: Vec<Hint> = match session.phase {
        Phase::Idle => vec![
            Hint::info("Session idle").with_action("forgeplan route \"<your task>\"".to_string()),
        ],
        Phase::Routing => vec![
            Hint::info("Routed — start shaping")
                .with_action("forgeplan new prd \"<title>\"".to_string()),
        ],
        Phase::Shaping => match session.active_artifact.as_deref() {
            Some(a) => vec![
                Hint::info("Validate the active artifact")
                    .with_action(format!("forgeplan validate {}", a)),
            ],
            None => Vec::new(),
        },
        Phase::Coding => match session.active_artifact.as_deref() {
            Some(a) => {
                vec![Hint::info("Score after coding").with_action(format!("forgeplan score {}", a))]
            }
            None => Vec::new(),
        },
        Phase::Evidence => match session.active_artifact.as_deref() {
            Some(a) => vec![
                Hint::info("Score after evidence").with_action(format!("forgeplan score {}", a)),
            ],
            None => Vec::new(),
        },
        Phase::Pr => match session.active_artifact.as_deref() {
            Some(a) => vec![
                Hint::info("Activate the artifact")
                    .with_action(format!("forgeplan activate {}", a)),
            ],
            None => Vec::new(),
        },
    };
    print!("{}", hints::render_next_action_line(&next_hints));
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

    // PRD-071: after reset, routing is the only sensible next step.
    let next_hints: Vec<Hint> = vec![
        Hint::info("Session reset").with_action("forgeplan route \"<your task>\"".to_string()),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));
}
