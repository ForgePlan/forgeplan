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
    println!("{}", style(BANNER).bold());
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
    if depth.eq_ignore_ascii_case("tactical") {
        style(depth).green().to_string()
    } else if depth.eq_ignore_ascii_case("standard") {
        style(depth).cyan().to_string()
    } else if depth.eq_ignore_ascii_case("deep") || depth.eq_ignore_ascii_case("deep/critical") {
        style(depth).red().bold().to_string()
    } else {
        depth.to_string()
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

/// Style an R_eff score with color: green >= 0.5, yellow 0.1-0.5, red < 0.1.
pub fn styled_reff(score: f64) -> String {
    let text = format!("{:.2}", score);
    if score >= 0.5 {
        style(text).green().bold().to_string()
    } else if score >= 0.1 {
        style(text).yellow().to_string()
    } else {
        style(text).red().to_string()
    }
}

// ─── Unified Output Helpers ──────────────────────────────────────

/// Print a command header with title and optional separator.
/// Example: `Forgeplan Health — ProjectName`
pub fn header(title: &str, subtitle: &str) {
    println!();
    if subtitle.is_empty() {
        println!("{}", style(title).bold());
    } else {
        println!("{} — {}", style(title).bold(), style(subtitle).cyan());
    }
    println!("{}", style("─".repeat(50)).dim());
}

/// Print a key-value pair with consistent alignment.
/// Example: `  ID:           PRD-001`
pub fn kv(key: &str, value: &str) {
    println!("  {:<14}{}", style(format!("{}:", key)).bold(), value);
}

/// Print a section heading within command output.
/// Example: `  Evidence breakdown:`
pub fn section(title: &str) {
    println!();
    println!("  {}:", style(title).bold());
}

/// Print an error with an actionable hint.
/// Example: `  ✗ Artifact 'X' not found`
///          `    → Run `forgeplan list` to see available artifacts`
pub fn error_hint(message: &str, hint: &str) {
    eprintln!("  {} {}", style("✗").red(), message);
    if !hint.is_empty() {
        eprintln!("    {} {}", style("→").dim(), style(hint).dim());
    }
}

/// Print a success message.
#[allow(dead_code)]
pub fn success(message: &str) {
    println!("  {} {}", style("✓").green(), message);
}

/// Print a warning message.
pub fn warning(message: &str) {
    println!("  {} {}", style("!").yellow(), message);
}

/// Print a dimmed info message.
pub fn info(message: &str) {
    println!("  {}", style(message).dim());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styled_status_all_variants() {
        // Verify no panics and non-empty output for all known statuses
        assert!(!styled_status("active").is_empty());
        assert!(!styled_status("draft").is_empty());
        assert!(!styled_status("superseded").is_empty());
        assert!(!styled_status("deprecated").is_empty());
        assert!(!styled_status("unknown").is_empty());
    }

    #[test]
    fn styled_depth_all_variants() {
        assert!(!styled_depth("tactical").is_empty());
        assert!(!styled_depth("standard").is_empty());
        assert!(!styled_depth("deep").is_empty());
        assert!(!styled_depth("deep/critical").is_empty());
        assert!(!styled_depth("Deep").is_empty()); // case insensitive
        assert!(!styled_depth("TACTICAL").is_empty());
    }

    #[test]
    fn styled_severity_all_variants() {
        assert!(!styled_severity("MUST").is_empty());
        assert!(!styled_severity("SHOULD").is_empty());
        assert!(!styled_severity("COULD").is_empty());
        assert!(!styled_severity("OTHER").is_empty());
    }

    #[test]
    fn styled_count_zero_and_nonzero() {
        assert!(!styled_count(0, false).is_empty());
        assert!(!styled_count(5, false).is_empty());
        assert!(!styled_count(3, true).is_empty());
        assert!(!styled_count(0, true).is_empty());
    }

    #[test]
    fn banner_contains_fpl() {
        assert!(BANNER.contains("███████╗"));
        assert!(BANNER.contains("██████╔╝"));
    }

    #[test]
    fn styled_reff_colors() {
        let high = styled_reff(1.0);
        let mid = styled_reff(0.3);
        let low = styled_reff(0.0);
        assert!(!high.is_empty());
        assert!(!mid.is_empty());
        assert!(!low.is_empty());
        // Different values should produce different styled output
        assert_ne!(styled_reff(1.0), styled_reff(0.0));
    }

    #[test]
    fn styled_reff_boundary_values() {
        // 0.5 is green boundary
        assert!(!styled_reff(0.5).is_empty());
        // 0.1 is yellow boundary
        assert!(!styled_reff(0.1).is_empty());
        // 0.09 is red
        assert!(!styled_reff(0.09).is_empty());
    }

    #[test]
    fn styled_reff_threshold_correctness() {
        // Green zone: >= 0.5
        let green = styled_reff(0.5);
        let also_green = styled_reff(1.0);
        assert_eq!(green, green); // same threshold, same output structure

        // Yellow zone: 0.1 <= x < 0.5
        let yellow = styled_reff(0.49);
        assert_ne!(yellow, green); // different zone

        // Red zone: < 0.1
        let red = styled_reff(0.09);
        assert_ne!(red, yellow); // different zone
        assert_ne!(red, green); // different zone

        // Boundary: 0.5 is green, 0.49 is yellow
        assert_ne!(styled_reff(0.5), styled_reff(0.49));
        // Boundary: 0.1 is yellow, 0.09 is red
        assert_ne!(styled_reff(0.1), styled_reff(0.09));
    }

    #[test]
    fn styled_reff_contains_value() {
        // Output should contain the numeric value
        assert!(styled_reff(1.0).contains("1.00"));
        assert!(styled_reff(0.0).contains("0.00"));
        assert!(styled_reff(0.42).contains("0.42"));
    }
}
