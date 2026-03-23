//! CLI UI helpers — cliclack wrappers, banner, styled output.

use console::style;

pub const BANNER: &str = r#"
    ███████╗ ██████╗  ██╗
    ██╔════╝ ██╔══██╗ ██║
    █████╗   ██████╔╝ ██║
    ██╔══╝   ██╔═══╝  ██║
    ██║      ██║      ███████╗
    ╚═╝      ╚═╝      ╚══════╝"#;

/// Print the FPL banner with version.
pub fn print_banner() {
    println!("{}", style(BANNER).cyan());
    println!(
        "     {} — v{}",
        style("forge your plan").dim(),
        env!("CARGO_PKG_VERSION")
    );
    println!();
}

/// Style a status string with appropriate color.
pub fn styled_status(status: &str) -> String {
    match status {
        "active" => style(status).green().bold().to_string(),
        "draft" => style(status).dim().to_string(),
        "superseded" => style(status).yellow().to_string(),
        "deprecated" => style(status).red().strikethrough().to_string(),
        _ => status.to_string(),
    }
}

/// Style a depth level with appropriate color.
pub fn styled_depth(depth: &str) -> String {
    match depth.to_lowercase().as_str() {
        "tactical" => style(depth).green().to_string(),
        "standard" => style(depth).cyan().to_string(),
        "deep" | "deep/critical" => style(depth).red().bold().to_string(),
        _ => depth.to_string(),
    }
}

/// Style a severity level.
pub fn styled_severity(severity: &str) -> String {
    match severity {
        "MUST" => style(severity).red().bold().to_string(),
        "SHOULD" => style(severity).yellow().to_string(),
        "COULD" => style(severity).dim().to_string(),
        _ => severity.to_string(),
    }
}

/// Format a count with color based on whether it's good or bad.
pub fn styled_count(count: usize, is_problem: bool) -> String {
    if is_problem && count > 0 {
        style(count.to_string()).red().bold().to_string()
    } else if count > 0 {
        style(count.to_string()).green().to_string()
    } else {
        style(count.to_string()).dim().to_string()
    }
}
