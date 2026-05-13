use console::style;
use forgeplan_core::artifact::sanitize::sanitize_for_hint;
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

/// Compute the `--strict` exit code for a built health report.
///
/// Returns `Some(1)` when the workspace surfaces any critical signal
/// (verdict ∈ {NeedsAttention, Unhealthy} OR any of orphans / blind_spots
/// / active_stubs / at_risk > 0). Returns `None` otherwise (caller emits
/// exit 0). Empty workspaces and advisory-only signals (phase mismatches
/// alone — consistent with PROB-063: advisory ≠ critical) are NOT counted
/// as failures.
///
/// Pure function on the report so unit tests can pin the contract without
/// invoking the CLI shell.
fn strict_exit_code(report: &health::HealthReport) -> Option<i32> {
    if !report.orphans.is_empty()
        || !report.blind_spots.is_empty()
        || !report.active_stubs.is_empty()
        || !report.at_risk.is_empty()
    {
        return Some(1);
    }
    match report.verdict {
        health::Verdict::NeedsAttention | health::Verdict::Unhealthy => Some(1),
        // `Empty` and `Healthy` (plus any forward-compat variants) → no signal
        _ => None,
    }
}

pub async fn run(
    compact: bool,
    json: bool,
    ci: bool,
    fail_on: Option<String>,
    strict: bool,
) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    let config = workspace::load_config(&ws)?;
    // PROB-051 L-H3 closure: route through `health_report_with_phase` so
    // CLI returns the SAME verdict as MCP `forgeplan_health` for the same
    // workspace (folds phase mismatches into the verdict aggregator). Pre-
    // PROB-051 CLI used `health_report` (no phase folding) → operator could
    // see different verdict via CLI vs MCP. Phase tracking is opt-in per
    // workspace config; when disabled `phase_mismatches` is empty and the
    // verdict equals the legacy `health_report` value.
    let (report, phase_mismatches) = health::health_report_with_phase(&store, &ws).await?;

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
        // Wave 9 ARCH-C1 closure: route the JSON shape through the
        // unified `forgeplan_core::health::health_report_to_json` helper
        // so CLI and MCP surfaces emit IDENTICAL key sets and shapes by
        // construction. Pre-fix the two surfaces hand-rolled `json!(...)`
        // literals with subtly different shapes (`by_kind` object vs
        // tuple; missing `possible_duplicates` on MCP — see PROB-064 /
        // CR-001). The helper also bundles SEC-M1 closure — every
        // title / message / reason / issue / advisory string is
        // sanitised before serialisation (CWE-117 / CWE-1007 defence).
        let mut json_data =
            forgeplan_core::health::health_report_to_json(&report, &phase_mismatches);
        // Layer in CLI-specific keys AFTER the helper builds the
        // shared shape: `project` (workspace identity, not part of the
        // core health domain) and `_next_action` (CLI hint protocol).
        // `--strict` adds a parseable `exit_code` field so CI scripts
        // can branch on a single integer.
        let strict_exit = if strict {
            strict_exit_code(&report).unwrap_or(0)
        } else {
            0
        };
        if let serde_json::Value::Object(ref mut map) = json_data {
            map.insert(
                "project".to_string(),
                serde_json::Value::String(config.project_name.clone()),
            );
            map.insert(
                "_next_action".to_string(),
                serde_json::to_value(hints::primary_action(&hints_vec))
                    .unwrap_or(serde_json::Value::Null),
            );
            if strict {
                map.insert("exit_code".to_string(), serde_json::json!(strict_exit));
            }
        }
        println!("{}", serde_json::to_string_pretty(&json_data)?);
        if strict && strict_exit != 0 {
            std::process::exit(strict_exit);
        }
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
        // LOG-001 (audit Wave 9): sanitize title + reason so an
        // attacker-controlled artifact title (ANSI escapes, bidi
        // overrides, zero-width chars) cannot hijack operator's
        // terminal output (CWE-117 / CWE-150).
        for item in &report.at_risk {
            println!(
                "    {} \"{}\" — {}",
                style(&item.id).yellow(),
                sanitize_for_hint(&item.title),
                style(sanitize_for_hint(&item.reason)).red()
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
                sanitize_for_hint(&spot.title),
                style(sanitize_for_hint(&spot.issue)).red()
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
    // PROB-051 L-M2: report carries the FULL list (verdict aggregator
    // needs the unclipped count). Display-side cap is the CLI's call —
    // print top-N by similarity and surface the overflow as a one-line
    // summary so operators know more pairs exist.
    if !report.possible_duplicates.is_empty() {
        let total_dups = report.possible_duplicates.len();
        let display_limit = health::DUPLICATE_PAIRS_DISPLAY_LIMIT;
        println!();
        println!(
            "  {} Possible duplicates ({}):",
            style("⧗").yellow().bold(),
            ui::styled_count(total_dups, true)
        );
        for d in report.possible_duplicates.iter().take(display_limit) {
            let pct = (d.similarity * 100.0).round() as u32;
            println!(
                "    {} ↔ {} ({}%) — \"{}\"",
                style(&d.id_a).yellow(),
                style(&d.id_b).yellow(),
                pct,
                sanitize_for_hint(&d.title_a)
            );
        }
        if total_dups > display_limit {
            println!(
                "    {} {} more pair(s) — see `forgeplan health --json` for the full list",
                style("…").dim(),
                total_dups - display_limit
            );
        }
    }

    // PROB-051 L-H3: phase mismatches advisory (active artifacts whose
    // recorded phase is still early-cycle — Code/Evidence likely skipped).
    if !phase_mismatches.is_empty() {
        println!();
        println!(
            "  {} Phase mismatches ({}):",
            style("⏳").yellow().bold(),
            ui::styled_count(phase_mismatches.len(), true)
        );
        for m in &phase_mismatches {
            println!(
                "    {} \"{}\" — phase: {}",
                style(&m.id).yellow(),
                sanitize_for_hint(&m.title),
                style(&m.current_phase).yellow()
            );
        }
    }

    // PROB-062: gitignore drift advisory — tracked files matching
    // canonical forgeplan ignore patterns (derived state, per-machine
    // runtime). Advisory like phase mismatches — printed for visibility
    // but never promoted into the verdict aggregator.
    if !report.gitignore_drift.is_empty() {
        println!();
        println!(
            "  {} Gitignore drift ({}):",
            style("◈").yellow().bold(),
            ui::styled_count(report.gitignore_drift.len(), true)
        );
        for d in &report.gitignore_drift {
            println!(
                "    {} — {}",
                style(sanitize_for_hint(&d.path)).yellow(),
                style(sanitize_for_hint(&d.reason)).dim()
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
                sanitize_for_hint(&s.title),
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

    // Overall health summary — drive the literal off `Verdict::human_summary()`
    // (single source of truth) and render for ALL three verdict levels so the
    // banner is always present, with colour signalling the severity gradient
    // (green/yellow/red + dim-cyan for future `#[non_exhaustive]` variants).
    // PROB-029 anti-contradiction guarantee: banner cannot disagree with
    // `next_actions` because both fold off the same verdict aggregator.
    if report.total > 0 {
        let summary = report.verdict.human_summary();
        let styled = match report.verdict {
            health::Verdict::Healthy => style(summary).green().bold(),
            health::Verdict::NeedsAttention => style(summary).yellow().bold(),
            health::Verdict::Unhealthy => style(summary).red().bold(),
            // `#[non_exhaustive]` future-proofing: render new verdicts as
            // dim cyan placeholder — better than crashing or hiding them.
            _ => style(summary).cyan().dim(),
        };
        println!();
        println!("  {}", styled);
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

    // `--strict` gate: exit non-zero when any critical signal trips. Runs
    // AFTER human rendering so operators still see the dashboard before
    // the CI failure. JSON path handles its own exit above to keep the
    // `exit_code` field in the payload.
    if strict && let Some(code) = strict_exit_code(&report) {
        std::process::exit(code);
    }

    Ok(())
}
