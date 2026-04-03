use console::style;
use forgeplan_core::health;
use forgeplan_core::workspace;

use crate::commands::common;
use crate::ui;

pub async fn run(compact: bool, json: bool) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    let config = workspace::load_config(&ws)?;
    let report = health::health_report(&store).await?;

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
        if let Some(action) = report.next_actions.first() {
            println!("Next: {}", action);
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

    // Overall health summary
    let has_issues = !report.at_risk.is_empty()
        || !report.blind_spots.is_empty()
        || !report.orphans.is_empty()
        || report.stale_count > 0;
    if !has_issues && report.total > 0 {
        println!();
        println!("  {}", style("Project looks healthy!").green().bold());
    }

    println!();
    Ok(())
}
