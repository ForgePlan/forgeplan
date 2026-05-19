//! `forgeplan anomalies` — issue #289 CLI surface for the pipeline anomaly
//! detector. Mirrors the `forgeplan_anomalies` MCP tool: same inputs, same
//! semantics, identical structured output with `--json`.

use crate::commands::common;
use forgeplan_core::anomalies::{self, AnomalyFilter, AnomalyKind, AnomalyReport, Severity, Tier};

pub async fn run(
    kind: Option<String>,
    severity: Option<String>,
    since: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let (ws, _lock, store) = common::open_store_locked().await?;

    let kind_filter = match kind.as_deref() {
        Some(s) => Some(parse_kind(s)?),
        None => None,
    };
    let severity_filter = match severity.as_deref() {
        Some(s) => Some(parse_severity(s)?),
        None => None,
    };
    let since_filter = match since.as_deref() {
        Some(s) => Some(
            chrono::DateTime::parse_from_rfc3339(s)
                .map_err(|e| {
                    anyhow::anyhow!(
                        "--since is not a valid RFC 3339 timestamp ({s}): {e}\n\
                         Example: 2026-05-19T00:00:00Z"
                    )
                })?
                .with_timezone(&chrono::Utc),
        ),
        None => None,
    };

    let filter = AnomalyFilter {
        severity: severity_filter,
        since: since_filter,
        kind: kind_filter,
    };
    let report = anomalies::detect_anomalies(&store, &ws, &filter).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    render_text(&report);
    Ok(())
}

fn parse_kind(s: &str) -> anyhow::Result<AnomalyKind> {
    serde_json::from_value::<AnomalyKind>(serde_json::Value::String(s.to_string())).map_err(|_| {
        anyhow::anyhow!(
            "Unknown anomaly kind: {s}\n\
             Valid kinds: stuck_draft, orphan_link, mistyped_based_on, missing_must_section, \
             expired_evidence, weakest_link_unresolvable, phase_mismatch, circular_dependency, \
             duplicate_artifact"
        )
    })
}

fn parse_severity(s: &str) -> anyhow::Result<Severity> {
    serde_json::from_value::<Severity>(serde_json::Value::String(s.to_string()))
        .map_err(|_| anyhow::anyhow!("Unknown severity: {s}\nValid severities: low, medium, high"))
}

fn render_text(report: &AnomalyReport) {
    if report.total == 0 {
        println!("No pipeline anomalies detected. ✓");
        println!("Done.");
        return;
    }

    println!("forgeplan anomalies — {} detected", report.total);
    println!(
        "  by severity: {} high, {} medium, {} low",
        report.by_severity.high, report.by_severity.medium, report.by_severity.low
    );
    println!(
        "  by tier:     {} auto, {} adi, {} user",
        report.by_tier.auto, report.by_tier.adi, report.by_tier.user
    );
    println!();

    for a in &report.anomalies {
        let sev = severity_glyph(a.severity);
        let tier = tier_glyph(a.tier);
        println!(
            "  {sev} [{tier}] {} — {}",
            kind_label(a.kind),
            a.description
        );
        if let Some(res) = &a.suggested_resolution {
            let action = res.action.as_deref().unwrap_or("(manual)");
            let target = res.target.as_deref().unwrap_or("");
            println!("      → {action} {target}");
        }
    }

    println!();
    if report.by_tier.auto > 0 {
        println!("Next: forgeplan anomalies --kind stuck_draft  # auto-tier candidates ready");
    } else if report.by_tier.user > 0 {
        println!("Next: forgeplan get <affected-id>  # review user-tier anomalies");
    } else {
        println!("Next: forgeplan score <id>  # investigate adi-tier candidates");
    }
}

fn severity_glyph(s: Severity) -> &'static str {
    match s {
        Severity::Low => "ℹ ",
        Severity::Medium => "⚠ ",
        Severity::High => "✗ ",
    }
}

fn tier_glyph(t: Tier) -> &'static str {
    match t {
        Tier::Auto => "auto",
        Tier::Adi => "adi ",
        Tier::User => "user",
    }
}

fn kind_label(k: AnomalyKind) -> &'static str {
    match k {
        AnomalyKind::StuckDraft => "stuck_draft",
        AnomalyKind::OrphanLink => "orphan_link",
        AnomalyKind::MistypedBasedOn => "mistyped_based_on",
        AnomalyKind::MissingMustSection => "missing_must_section",
        AnomalyKind::ExpiredEvidence => "expired_evidence",
        AnomalyKind::WeakestLinkUnresolvable => "weakest_link_unresolvable",
        AnomalyKind::PhaseMismatch => "phase_mismatch",
        AnomalyKind::CircularDependency => "circular_dependency",
        AnomalyKind::DuplicateArtifact => "duplicate_artifact",
    }
}
