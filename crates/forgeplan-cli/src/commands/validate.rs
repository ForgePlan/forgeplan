use console::style;
use forgeplan_core::artifact::types::{ArtifactKind, Mode};
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::validation::{self, Severity, ValidationResult, adversarial};

use crate::commands::common;
use crate::ui;

pub async fn run(id: Option<&str>, json: bool, adversarial: bool, ci: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let all_records = store.list_records(None).await?;

    if all_records.is_empty() {
        println!("No artifacts found.");
        return Ok(());
    }

    let to_validate: Vec<_> = if let Some(target_id) = id {
        let upper = target_id.to_uppercase();
        all_records
            .into_iter()
            .filter(|r| r.id.to_uppercase() == upper)
            .collect()
    } else if ci {
        // CI mode: validate active + stale (stale = expired but still live decisions)
        all_records
            .into_iter()
            .filter(|r| r.status == "active" || r.status == "stale")
            .collect()
    } else {
        all_records
    };

    if to_validate.is_empty()
        && let Some(target_id) = id
    {
        anyhow::bail!(
            "Artifact '{}' not found
Fix: forgeplan list",
            target_id
        );
    }

    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut total_passed = 0;
    let mut json_results = Vec::new();

    for record in &to_validate {
        let fm = record.frontmatter_map();

        let kind = record.kind.parse::<ArtifactKind>().unwrap_or_else(|_| {
            if !json {
                eprintln!(
                    "  Warning: unknown artifact kind '{}', applying base rules only",
                    record.kind
                );
            }
            ArtifactKind::Note
        });
        let depth = record.depth.parse::<Mode>().unwrap_or(Mode::Standard);

        let mut result = validation::validate(&record.id, &record.body, &fm, &kind, &depth);

        if adversarial {
            let adv_findings = adversarial::adversarial_checks(&record.body, &record.kind);
            let adv_count = adv_findings.len();
            result.findings.extend(adv_findings);
            result.total_rules_checked += adv_count;
        }

        if json {
            json_results.push(serde_json::json!({
                "artifact_id": result.artifact_id,
                "kind": result.kind,
                "depth": result.depth,
                "passed": result.passed(),
                "errors": result.error_count(),
                "warnings": result.warning_count(),
                "findings": result.findings.iter().map(|f| serde_json::json!({
                    "rule_id": f.rule_id,
                    "severity": format!("{:?}", f.severity),
                    "message": f.message,
                    "section": f.section,
                })).collect::<Vec<_>>(),
            }));
        } else {
            print_result(&result, &record.title, &depth);
        }

        total_errors += result.error_count();
        total_warnings += result.warning_count();
        if result.passed() {
            total_passed += 1;
        }
    }

    // PRD-071 contract: emit deterministic primary next-action.
    // - single ID + 0 errors → activate
    // - single ID + errors → fix-style hint (re-validate after editing)
    // - multi/CI mode → run health to get blind-spot view
    let next_hints: Vec<Hint> = if let Some(target_id) = id {
        if total_errors == 0 {
            vec![
                Hint::info("Validation passed")
                    .with_action(format!("forgeplan activate {}", target_id)),
            ]
        } else {
            vec![
                Hint::warning(format!(
                    "{} MUST error(s) — fix and revalidate",
                    total_errors
                ))
                .with_action(format!("forgeplan validate {}", target_id)),
            ]
        }
    } else if total_errors > 0 {
        vec![
            Hint::warning(format!("{} MUST error(s) across artifacts", total_errors))
                .with_action("forgeplan health".to_string()),
        ]
    } else {
        vec![Hint::info("All validations passed").with_action("forgeplan health".to_string())]
    };

    if json {
        // Embed _next_action alongside the per-artifact results array.
        let payload = serde_json::json!({
            "results": json_results,
            "_next_action": hints::primary_action(&next_hints),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if to_validate.len() > 1 {
        println!();
        println!(
            "Summary: {} artifact(s), {} passed, {} error(s), {} warning(s)",
            to_validate.len(),
            ui::styled_count(total_passed, false),
            ui::styled_count(total_errors, true),
            ui::styled_count(total_warnings, total_warnings > 0),
        );
    }

    // Session: advance to Coding when single artifact validates PASS
    if id.is_some() && total_errors == 0 && total_passed > 0 {
        common::advance_session(forgeplan_core::session::Phase::Coding, None);
    }

    // Render primary next-action for non-JSON paths. Errors render as Fix:
    // (warnings level) so the agent picks them up via the same Next: stream.
    if !json {
        if total_errors > 0 {
            if let Some(fix) = hints::primary_action(&next_hints) {
                eprintln!("Fix: {}", fix);
            }
        } else {
            print!("{}", hints::render_next_action_line(&next_hints));
        }
    }

    if ci && total_errors > 0 {
        eprintln!(
            "CI FAILED — {} MUST error(s) in {} artifact(s)",
            total_errors,
            to_validate.len()
        );
        std::process::exit(1);
    } else if ci {
        println!(
            "CI PASSED — {} artifact(s) validated, 0 MUST errors",
            to_validate.len()
        );
    } else if total_errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn print_result(result: &ValidationResult, title: &str, depth: &Mode) {
    let depth_str = format!("{:?}", depth);
    println!();
    println!(
        "{} \"{}\" (depth: {})",
        style(&result.artifact_id).bold(),
        title,
        ui::styled_depth(&depth_str),
    );
    println!("{}", style("─".repeat(50)).dim());

    if result.findings.is_empty() {
        println!("  {}", style("All checks passed!").green().bold());
    } else {
        for f in &result.findings {
            let icon = match f.severity {
                Severity::Must => style("x").red().bold().to_string(),
                Severity::Should => style("!").yellow().to_string(),
                Severity::Could => style("~").dim().to_string(),
            };
            let severity_str = match f.severity {
                Severity::Must => "MUST",
                Severity::Should => "SHOULD",
                Severity::Could => "COULD",
            };
            println!(
                "  {} [{}] {}: {}",
                icon,
                ui::styled_severity(severity_str),
                style(&f.rule_id).dim(),
                f.message
            );
        }
    }

    let no_findings = result.findings.is_empty();
    let status_styled = if no_findings {
        style("PASS").green().bold()
    } else if result.passed() {
        style("PASS (with warnings)").green()
    } else {
        style("FAIL").red().bold()
    };
    println!();
    println!(
        "  Result: {} -- {} error(s), {} warning(s)",
        status_styled,
        ui::styled_count(result.error_count(), true),
        ui::styled_count(result.warning_count(), result.warning_count() > 0),
    );
}
