use console::style;
use forgeplan_core::health;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::workspace;

use crate::commands::common;
use crate::ui;

/// Parse `--fail-on` thresholds like "orphans=5,blind_spots=3,stale=2"
fn parse_fail_on(fail_on: &str) -> std::collections::HashMap<String, usize> {
    let mut thresholds = std::collections::HashMap::new();
    for part in fail_on.split(',') {
        let part = part.trim();
        if let Some((key, val)) = part.split_once('=')
            && let Ok(n) = val.trim().parse::<usize>()
        {
            thresholds.insert(key.trim().to_string(), n);
        }
    }
    thresholds
}

pub async fn run(
    compact: bool,
    json: bool,
    ci: bool,
    fail_on: Option<String>,
) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    let config = workspace::load_config(&ws)?;
    let report = health::health_report(&store).await?;

    // PRD-071 contract: derive a single deterministic Next: action from the
    // report. Priority order — blind spots > stubs > orphans > stale > at risk
    // > healthy. Use the first id in each bucket so the hint is real and
    // copy-pasteable.
    let mut hints_vec: Vec<Hint> = Vec::new();
    if let Some(spot) = report.blind_spots.first() {
        hints_vec.push(
            Hint::warning(format!("Validate blind spot {}", spot.id))
                .with_action(format!("forgeplan validate {}", spot.id)),
        );
    } else if let Some(stub) = report.active_stubs.first() {
        hints_vec.push(
            Hint::warning(format!("Active stub detected: {}", stub.id))
                .with_action(format!("forgeplan review {}", stub.id)),
        );
    } else if let Some(orphan) = report.orphans.first() {
        hints_vec.push(
            Hint::warning(format!("Orphan artifact {}", orphan))
                .with_action(format!("forgeplan get {}", orphan)),
        );
    } else if report.stale_count > 0 {
        hints_vec.push(
            Hint::warning(format!("{} stale evidence", report.stale_count))
                .with_action("forgeplan stale".to_string()),
        );
    } else if let Some(risk) = report.at_risk.first() {
        hints_vec.push(
            Hint::warning(format!("At-risk artifact {}", risk.id))
                .with_action(format!("forgeplan score {}", risk.id)),
        );
    }
    // No hints → workspace healthy → render `Done.` terminal indicator.

    if json {
        let json_data = serde_json::json!({
            "project": config.project_name,
            "total": report.total,
            "by_kind": report.by_kind.iter().map(|(k, v)| serde_json::json!({"kind": k, "count": v})).collect::<Vec<_>>(),
            "by_status": report.by_status.iter().map(|(s, v)| serde_json::json!({"status": s, "count": v})).collect::<Vec<_>>(),
            "at_risk": report.at_risk.iter().map(|a| serde_json::json!({"id": a.id, "title": a.title, "reason": a.reason})).collect::<Vec<_>>(),
            "blind_spots": report.blind_spots.iter().map(|b| serde_json::json!({"id": b.id, "title": b.title, "issue": b.issue})).collect::<Vec<_>>(),
            "stale_count": report.stale_count,
            "orphans": report.orphans,
            "by_derived_status": report.by_derived_status.iter().map(|(ds, v)| serde_json::json!({"status": ds.label(), "count": v})).collect::<Vec<_>>(),
            "next_actions": report.next_actions,
            "possible_duplicates": report.possible_duplicates.iter().map(|d| serde_json::json!({
                "id_a": d.id_a,
                "id_b": d.id_b,
                "similarity": d.similarity,
                "title_a": d.title_a,
                "title_b": d.title_b,
                "kind": d.kind,
            })).collect::<Vec<_>>(),
            "active_stubs": report.active_stubs.iter().map(|s| serde_json::json!({
                "id": s.id,
                "kind": s.kind,
                "title": s.title,
                "markers_found": s.markers_found,
                "message": s.message,
            })).collect::<Vec<_>>(),
            "_next_action": hints::primary_action(&hints_vec),
        });
        println!("{}", serde_json::to_string_pretty(&json_data)?);
        return Ok(());
    }

    if compact {
        // Compact mode for hooks/scripts
        println!(
            "Project: {} | Artifacts: {} | Blind spots: {} | Stale: {} | At risk: {}",
            config.project_name,
            report.total,
            report.blind_spots.len(),
            report.stale_count,
            report.at_risk.len(),
        );
        match hints::primary_action(&hints_vec) {
            Some(cmd) => println!("Next: {}", cmd),
            None => println!("Done."),
        }
        return Ok(());
    }

    // Full dashboard
    println!();
    println!(
        "{} — {}",
        style("Forgeplan Health").bold(),
        style(&config.project_name).cyan()
    );
    println!("{}", style("═".repeat(50)).dim());

    println!();
    println!(
        "  {}  {} total",
        style("Artifacts:").bold(),
        ui::styled_count(report.total, false)
    );

    if !report.by_kind.is_empty() {
        println!();
        println!("  {}:", style("By kind").bold());
        for (kind, count) in &report.by_kind {
            println!("    {:<16} {}", style(kind).cyan(), count);
        }
    }

    if !report.by_status.is_empty() {
        println!();
        println!("  {}:", style("By status").bold());
        for (status, count) in &report.by_status {
            let warning = if status == "draft" && *count == report.total && report.total > 0 {
                format!(" {}", style("ALL DRAFT").red().bold())
            } else {
                String::new()
            };
            println!("    {}  {}{}", ui::styled_status(status), count, warning);
        }
    }

    if !report.by_derived_status.is_empty() {
        println!();
        println!("  {}:", style("By derived status").bold());
        for (ds, count) in &report.by_derived_status {
            let label = ds.label();
            let styled_label = match ds {
                forgeplan_core::status::DerivedStatus::Stub => style(label).red(),
                forgeplan_core::status::DerivedStatus::Shaped => style(label).yellow(),
                forgeplan_core::status::DerivedStatus::Validated => style(label).blue(),
                forgeplan_core::status::DerivedStatus::Evidenced => style(label).cyan(),
                forgeplan_core::status::DerivedStatus::Activated => style(label).green(),
            };
            println!("    {:<16} {}", styled_label, count);
        }
    }

    // At Risk
    if !report.at_risk.is_empty() {
        println!();
        println!(
            "  {} At Risk ({}):",
            style("!").yellow().bold(),
            ui::styled_count(report.at_risk.len(), true)
        );
        for item in &report.at_risk {
            println!(
                "    {} \"{}\" — {}",
                style(&item.id).yellow(),
                item.title,
                style(&item.reason).red()
            );
        }
    }

    // Blind Spots
    if !report.blind_spots.is_empty() {
        println!();
        println!(
            "  {} Blind Spots ({}):",
            style("●").red().bold(),
            ui::styled_count(report.blind_spots.len(), true)
        );
        for spot in &report.blind_spots {
            println!(
                "    {} \"{}\" — {}",
                style(&spot.id).yellow(),
                spot.title,
                style(&spot.issue).red()
            );
        }
    }

    // Stale
    if report.stale_count > 0 {
        println!();
        println!(
            "  {} Stale: {} evidence expired",
            style("⏰").yellow(),
            ui::styled_count(report.stale_count, true)
        );
    }

    // Orphans
    if !report.orphans.is_empty() {
        println!();
        println!(
            "  {} Orphans ({}):",
            style("○").red(),
            ui::styled_count(report.orphans.len(), true)
        );
        for id in &report.orphans {
            println!("    {} — {}", style(id).yellow(), style("no links").red());
        }
    }

    // Possible duplicates
    if !report.possible_duplicates.is_empty() {
        println!();
        println!(
            "  {} Possible duplicates ({}):",
            style("⧗").yellow().bold(),
            ui::styled_count(report.possible_duplicates.len(), true)
        );
        for d in &report.possible_duplicates {
            let pct = (d.similarity * 100.0).round() as u32;
            println!(
                "    {} ↔ {} ({}%) — \"{}\"",
                style(&d.id_a).yellow(),
                style(&d.id_b).yellow(),
                pct,
                d.title_a
            );
        }
    }

    // Active stubs (direct-edit bypasses of activate gate)
    if !report.active_stubs.is_empty() {
        println!();
        println!(
            "  {} Active stubs ({}):",
            style("⚠").yellow().bold(),
            ui::styled_count(report.active_stubs.len(), true)
        );
        for s in &report.active_stubs {
            println!(
                "    {} ({}) \"{}\" — {} markers",
                style(&s.id).yellow(),
                s.kind,
                s.title,
                s.markers_found
            );
        }
    }

    // Next Actions
    if !report.next_actions.is_empty() {
        println!();
        println!(
            "  {} {}:",
            style("→").green().bold(),
            style("Next actions").bold()
        );
        for (i, action) in report.next_actions.iter().enumerate() {
            println!("    {}. {}", style(i + 1).green(), action);
        }
    }

    // Overall health summary — drive off the verdict aggregator so the CLI
    // banner cannot disagree with `next_actions` (PROB-029 closure: previously
    // `has_issues` here missed `active_stubs` + `possible_duplicates` and
    // could print "Project looks healthy!" right after a list of warnings).
    if report.verdict == health::Verdict::Healthy && report.total > 0 {
        println!();
        println!("  {}", style("Project looks healthy!").green().bold());
    }

    // PRD-071 contract: terminal Next:/Done line.
    match hints::primary_action(&hints_vec) {
        Some(cmd) => println!("\nNext: {}", cmd),
        None if report.total > 0 => println!("\nDone."),
        None => {}
    }

    println!();

    // CI mode: check thresholds and exit with code 1 if exceeded
    if ci {
        let thresholds = fail_on.as_deref().map(parse_fail_on).unwrap_or_default();

        let mut failures = Vec::new();

        // Default thresholds: any blind spots or MUST orphans fail
        let max_orphans = thresholds.get("orphans").copied().unwrap_or(0);
        let max_blind_spots = thresholds.get("blind_spots").copied().unwrap_or(0);
        let max_stale = thresholds.get("stale").copied().unwrap_or(usize::MAX);
        let max_at_risk = thresholds.get("at_risk").copied().unwrap_or(usize::MAX);

        if report.orphans.len() > max_orphans {
            failures.push(format!(
                "orphans: {} (threshold: {})",
                report.orphans.len(),
                max_orphans
            ));
        }
        if report.blind_spots.len() > max_blind_spots {
            failures.push(format!(
                "blind_spots: {} (threshold: {})",
                report.blind_spots.len(),
                max_blind_spots
            ));
        }
        if report.stale_count > max_stale {
            failures.push(format!(
                "stale: {} (threshold: {})",
                report.stale_count, max_stale
            ));
        }
        if report.at_risk.len() > max_at_risk {
            failures.push(format!(
                "at_risk: {} (threshold: {})",
                report.at_risk.len(),
                max_at_risk
            ));
        }

        if !failures.is_empty() {
            eprintln!("CI FAILED — health thresholds exceeded:");
            for f in &failures {
                eprintln!("  - {f}");
            }
            std::process::exit(1);
        } else {
            println!("CI PASSED — health within thresholds");
        }
    }

    Ok(())
}
