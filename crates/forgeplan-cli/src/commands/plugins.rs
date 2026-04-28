//! `forgeplan plugins` CLI surface (PRD-067).
//!
//! Wave 3 implementation by agent **w3b-cli-ingest-plugins**.
//!
//! Three sub-commands:
//!
//! - `forgeplan plugins list [--json]` — installed plugins
//! - `forgeplan plugins doctor [--json]` — health check (missing/outdated/ok)
//! - `forgeplan plugins info <name> [--json]` — single plugin details
//!
//! Each command emits a single PRD-071 hint marker (`Next:` / `Or:` /
//! `Done.` / `Fix:`) in text mode and a `_next_action` field in JSON.

use anyhow::Result;
use console::style;
use serde_json::json;

use forgeplan_core::plugins::{
    InstalledPlugin, PluginInfo, PluginRegistry, detect_plugins, extended_registry,
};

// ────────────────────────────────────────────────────────────────────────────
// list
// ────────────────────────────────────────────────────────────────────────────

/// `forgeplan plugins list [--json]`
pub async fn run_list(json: bool) -> Result<()> {
    let registry = extended_registry();
    let installed = detect_plugins(&registry);

    if json {
        // Same next action regardless of installed count: doctor surfaces
        // both "all good" and "missing → install" follow-ups.
        let next = Some("forgeplan plugins doctor");
        print_json(&json!({
            "installed": installed,
            "registry_size": registry.len(),
            "_next_action": next,
        }));
        return Ok(());
    }

    if installed.is_empty() {
        println!(
            "  {} no plugins detected (registry knows {} entries)",
            style("⊘").dim(),
            registry.len()
        );
        println!("\nOr: forgeplan plugins doctor");
        return Ok(());
    }

    println!(
        "  {} {} plugin(s) installed",
        style("✓").green(),
        installed.len()
    );
    println!();
    println!(
        "  {:<28}  {:<14}  {:<10}  Path",
        "Name", "Source", "Version"
    );
    println!("  {}", "-".repeat(80));
    for ip in &installed {
        let source_label = format!("{:?}", ip.info.source).to_lowercase();
        let version = ip.detected_version.as_deref().unwrap_or("-");
        println!(
            "  {:<28}  {:<14}  {:<10}  {}",
            truncate(&ip.info.name, 28),
            truncate(&source_label, 14),
            truncate(version, 10),
            ip.detected_path.display()
        );
    }
    println!("\nNext: forgeplan plugins doctor");
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// doctor
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct DoctorReport {
    ok: Vec<DoctorOk>,
    missing: Vec<DoctorMissing>,
    outdated: Vec<DoctorOutdated>,
}

#[derive(Debug, Clone)]
struct DoctorOk {
    name: String,
    detected_version: Option<String>,
    expected: String,
}

impl DoctorOk {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "name": self.name,
            "detected_version": self.detected_version,
            "expected": self.expected,
        })
    }
}

#[derive(Debug, Clone)]
struct DoctorMissing {
    name: String,
    install_command: String,
    description: String,
}

impl DoctorMissing {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "name": self.name,
            "install_command": self.install_command,
            "description": self.description,
        })
    }
}

#[derive(Debug, Clone)]
struct DoctorOutdated {
    name: String,
    expected: String,
    found: String,
}

impl DoctorOutdated {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "name": self.name,
            "expected": self.expected,
            "found": self.found,
        })
    }
}

/// `forgeplan plugins doctor [--json]`
///
/// Exit code: `0` if all known plugins are OK, `1` if any missing or
/// outdated. Per PRD-067 AC-2 + AC-6, every missing entry surfaces an
/// exact install command via `Fix:` (text) / `install_command` (JSON).
pub async fn run_doctor(json: bool) -> Result<()> {
    let registry = extended_registry();
    let installed = detect_plugins(&registry);
    let report = compute_doctor_report(&registry, &installed);

    let has_problems = !report.missing.is_empty() || !report.outdated.is_empty();
    let next_action = if has_problems {
        report
            .missing
            .first()
            .map(|m| m.install_command.clone())
            .or_else(|| Some("forgeplan plugins list".to_string()))
    } else {
        Some("forgeplan plugins list".to_string())
    };

    if json {
        print_json(&json!({
            "ok": report.ok.iter().map(DoctorOk::to_json).collect::<Vec<_>>(),
            "missing": report.missing.iter().map(DoctorMissing::to_json).collect::<Vec<_>>(),
            "outdated": report.outdated.iter().map(DoctorOutdated::to_json).collect::<Vec<_>>(),
            "_next_action": next_action.as_deref(),
        }));
    } else {
        print_doctor_text(&report);
        if let Some(first_missing) = report.missing.first() {
            println!("\nFix: {}", first_missing.install_command);
        } else if !report.outdated.is_empty() {
            println!("\nFix: re-install / upgrade flagged plugins");
        } else {
            println!("\nDone.");
        }
    }

    if has_problems {
        std::process::exit(1);
    }
    Ok(())
}

fn compute_doctor_report(registry: &PluginRegistry, installed: &[InstalledPlugin]) -> DoctorReport {
    let mut ok = Vec::new();
    let mut missing = Vec::new();
    let mut outdated = Vec::new();

    let installed_names: std::collections::HashMap<&str, &InstalledPlugin> = installed
        .iter()
        .map(|p| (p.info.name.as_str(), p))
        .collect();

    for info in registry.iter() {
        match installed_names.get(info.name.as_str()) {
            None => {
                missing.push(DoctorMissing {
                    name: info.name.clone(),
                    install_command: info.install_command.clone(),
                    description: info.description.clone(),
                });
            }
            Some(ip) => match ip.is_version_compatible() {
                Ok(true) => {
                    ok.push(DoctorOk {
                        name: info.name.clone(),
                        detected_version: ip.detected_version.clone(),
                        expected: info.version_req.clone(),
                    });
                }
                Ok(false) => {
                    // No version detected, OR version mismatch.
                    if ip.detected_version.is_none() {
                        // Manifest version absent — treat as OK since we
                        // cannot prove incompatibility (avoids false alarms
                        // for plugins shipping without manifest.json).
                        ok.push(DoctorOk {
                            name: info.name.clone(),
                            detected_version: None,
                            expected: info.version_req.clone(),
                        });
                    } else {
                        outdated.push(DoctorOutdated {
                            name: info.name.clone(),
                            expected: info.version_req.clone(),
                            found: ip.detected_version.clone().unwrap_or_default(),
                        });
                    }
                }
                Err(_) => {
                    // Bad semver in registry / manifest — log as outdated so
                    // it surfaces but doesn't crash the doctor.
                    outdated.push(DoctorOutdated {
                        name: info.name.clone(),
                        expected: info.version_req.clone(),
                        found: ip
                            .detected_version
                            .clone()
                            .unwrap_or_else(|| "?".to_string()),
                    });
                }
            },
        }
    }

    DoctorReport {
        ok,
        missing,
        outdated,
    }
}

fn print_doctor_text(report: &DoctorReport) {
    println!(
        "  {} {} ok, {} missing, {} outdated",
        style("✓").green(),
        report.ok.len(),
        report.missing.len(),
        report.outdated.len()
    );

    if !report.ok.is_empty() {
        println!();
        for o in &report.ok {
            let v = o.detected_version.as_deref().unwrap_or("(no manifest)");
            println!(
                "    {} {:<32} {} (req {})",
                style("✓").green(),
                o.name,
                style(v).dim(),
                o.expected
            );
        }
    }

    if !report.outdated.is_empty() {
        println!();
        for o in &report.outdated {
            println!(
                "    {} {:<32} found {} expected {}",
                style("!").yellow(),
                o.name,
                o.found,
                o.expected
            );
        }
    }

    if !report.missing.is_empty() {
        println!();
        for m in &report.missing {
            println!(
                "    {} {:<32} {}",
                style("✗").red(),
                m.name,
                style(&m.description).dim()
            );
            println!("        install: {}", m.install_command);
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// info
// ────────────────────────────────────────────────────────────────────────────

/// `forgeplan plugins info <name> [--json]`
pub async fn run_info(name: &str, json: bool) -> Result<()> {
    let registry = extended_registry();
    let info = match registry.get(name) {
        Some(i) => i,
        None => {
            if json {
                print_json(&json!({
                    "error": format!("plugin '{name}' not in registry"),
                    "_next_action": "forgeplan plugins list",
                }));
            } else {
                eprintln!("Error: plugin '{name}' not in registry");
                eprintln!("Or: forgeplan plugins list");
            }
            std::process::exit(2);
        }
    };

    let installed = detect_plugins(&registry);
    let installed_match = installed.iter().find(|p| p.info.name == name).cloned();

    if json {
        let payload = json!({
            "info": info,
            "installed": installed_match,
            "_next_action": next_action_for_info(&installed_match, info),
        });
        print_json(&payload);
        return Ok(());
    }

    print_info_text(info, installed_match.as_ref());

    let next = next_action_for_info(&installed_match, info);
    match next {
        Some(cmd) if cmd == "Done." => println!("\nDone."),
        Some(cmd) => println!("\nNext: {cmd}"),
        None => println!("\nDone."),
    }
    Ok(())
}

fn print_info_text(info: &PluginInfo, installed: Option<&InstalledPlugin>) {
    println!("  Name:        {}", info.name);
    println!("  Source:      {:?}", info.source);
    println!("  Version req: {}", info.version_req);
    println!("  Description: {}", info.description);
    println!("  Install:     {}", info.install_command);
    if !info.expected_paths.is_empty() {
        println!("  Expected paths:");
        for p in &info.expected_paths {
            println!("    - {}", p.display());
        }
    }
    match installed {
        Some(ip) => {
            println!("\n  Installed:   {}", style("yes").green());
            println!("  Path:        {}", ip.detected_path.display());
            if let Some(v) = &ip.detected_version {
                println!("  Version:     {v}");
            }
        }
        None => {
            println!("\n  Installed:   {}", style("no").red());
        }
    }
}

fn next_action_for_info(installed: &Option<InstalledPlugin>, info: &PluginInfo) -> Option<String> {
    match installed {
        Some(_) => Some("Done.".to_string()),
        None => Some(info.install_command.clone()),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────────

fn print_json(value: &serde_json::Value) {
    match serde_json::to_string_pretty(value) {
        Ok(s) => println!("{s}"),
        Err(_) => println!("{value}"),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use forgeplan_core::plugins::types::{PluginInfo, PluginSource};
    use std::path::PathBuf;

    fn info(name: &str) -> PluginInfo {
        PluginInfo {
            name: name.to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=1.0".to_string(),
            expected_paths: vec![PathBuf::from(name)],
            install_command: format!("install {name}"),
            description: "test plugin".to_string(),
        }
    }

    #[test]
    fn truncate_under_limit_unchanged() {
        assert_eq!(truncate("abc", 5), "abc");
    }

    #[test]
    fn truncate_over_limit_appends_ellipsis() {
        let out = truncate("abcdefghij", 5);
        assert_eq!(out.chars().count(), 5);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn doctor_report_classifies_missing_when_not_installed() {
        let mut reg = PluginRegistry::new();
        reg.insert(info("ghost"));
        let report = compute_doctor_report(&reg, &[]);
        assert_eq!(report.missing.len(), 1);
        assert_eq!(report.missing[0].name, "ghost");
        assert!(report.missing[0].install_command.contains("install ghost"));
    }

    #[test]
    fn doctor_report_classifies_ok_when_compatible() {
        let mut reg = PluginRegistry::new();
        let i = info("good");
        reg.insert(i.clone());
        let installed = vec![InstalledPlugin {
            info: i,
            detected_path: PathBuf::from("/tmp/good"),
            detected_version: Some("2.0.0".to_string()),
        }];
        let report = compute_doctor_report(&reg, &installed);
        assert_eq!(report.ok.len(), 1);
        assert_eq!(report.missing.len(), 0);
    }

    #[test]
    fn doctor_report_classifies_outdated_when_below_req() {
        let mut reg = PluginRegistry::new();
        let i = info("stale");
        reg.insert(i.clone());
        let installed = vec![InstalledPlugin {
            info: i,
            detected_path: PathBuf::from("/tmp/stale"),
            detected_version: Some("0.5.0".to_string()),
        }];
        let report = compute_doctor_report(&reg, &installed);
        assert_eq!(report.outdated.len(), 1);
        assert_eq!(report.outdated[0].found, "0.5.0");
    }

    #[test]
    fn doctor_report_treats_missing_manifest_version_as_ok() {
        // No detected_version ⇒ we cannot prove incompat, so report OK
        // (PRD-067 AC-1: don't false-alarm on manifest-less plugins).
        let mut reg = PluginRegistry::new();
        let i = info("no-manifest");
        reg.insert(i.clone());
        let installed = vec![InstalledPlugin {
            info: i,
            detected_path: PathBuf::from("/tmp/x"),
            detected_version: None,
        }];
        let report = compute_doctor_report(&reg, &installed);
        assert_eq!(report.ok.len(), 1);
        assert_eq!(report.outdated.len(), 0);
    }

    #[test]
    fn next_action_for_info_returns_install_when_missing() {
        let i = info("missing-thing");
        let action = next_action_for_info(&None, &i);
        assert_eq!(action.as_deref(), Some("install missing-thing"));
    }

    #[test]
    fn next_action_for_info_done_when_installed() {
        let i = info("present");
        let installed = Some(InstalledPlugin {
            info: i.clone(),
            detected_path: PathBuf::from("/tmp/present"),
            detected_version: Some("1.0.0".to_string()),
        });
        assert_eq!(
            next_action_for_info(&installed, &i).as_deref(),
            Some("Done.")
        );
    }
}
