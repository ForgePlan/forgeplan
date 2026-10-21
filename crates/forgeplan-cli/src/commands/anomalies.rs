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
    // The report itself is a pure read now: `detect_anomalies*` computes the
    // summary and returns the journal alongside it, without persisting. The
    // WRITE is the explicit `persist_anomaly_journal` call below, and it is
    // serialized by the workspace lock.
    //
    // The lock spans scan AND persist on purpose: the scan reads the journal
    // and applies `--since` against it, so releasing between them would let a
    // concurrent process change the journal in the gap and leave this run
    // persisting state the report was not computed from. Persisting first
    // would be worse still — the run would observe its own detection and
    // `--since` would return everything, defeating diff-style polling.
    //
    // `_lock` is bound (not `_`) on purpose: the guard must stay ALIVE for the
    // whole of run() — dropping it early would silently un-serialize the
    // persist below. Its leading underscore says "held, never read".
    let (ws, _lock, store) = common::open_store_locked_tolerating_constellation().await?;

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
    // The detection is the same computation over records + relations on both
    // layouts — so a migrated workspace reports the same anomalies instead of
    // refusing — and it returns the journal it would have written without
    // writing it.
    let scan = match &store {
        Some(store) => {
            anomalies::detect_anomalies_with_journal(store, &ws, &filter).await?
        }
        None => {
            let read = common::workspace_read(&ws).await?;
            anomalies::detect_anomalies_captured_with_journal(&read, &ws, &filter).await?
        }
    };
    // Exactly one persist, under the guard acquired above. A failure degrades
    // `--since` to per-scan timestamps — the same best-effort contract the
    // implicit write had — and must not fail the report.
    //
    // `eprintln!`, not `tracing::warn!`: this crate has no `tracing` dependency
    // and its warnings all go to stderr this way. Adding a logging framework for
    // one line would be a manifest change well outside this surface.
    if let Err(e) = anomalies::persist_anomaly_journal(&ws, &scan.journal).await {
        eprintln!(
            "  warning: anomaly journal write failed ({e}) — `--since` filter will lose \
             pre-existing-anomaly memory"
        );
    }
    let report = scan.report;

    if json {
        // Audit-r3 HIGH-1 / F3: emit `_next_action` so CLI JSON matches
        // the MCP wire shape (PRD-071 parity). Shared helper computes
        // the canonical hint string for both surfaces.
        let next_action = anomalies::next_action_for_report(&report);
        let mut payload = serde_json::to_value(&report)?;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "_next_action".to_string(),
                serde_json::Value::String(next_action),
            );
        }
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    render_text(&report);
    Ok(())
}

fn parse_kind(s: &str) -> anyhow::Result<AnomalyKind> {
    serde_json::from_value::<AnomalyKind>(serde_json::Value::String(s.to_string())).map_err(|_| {
        // Audit-r7 F4: must list ALL 14 kinds (9 pipeline + 5 brownfield
        // from Epic #287). The error UX previously hid brownfield from
        // the operator even though serde accepts them.
        anyhow::anyhow!(
            "Unknown anomaly kind: {s}\n\
             Valid kinds (pipeline): stuck_draft, orphan_link, mistyped_based_on, \
             missing_must_section, expired_evidence, weakest_link_unresolvable, \
             phase_mismatch, circular_dependency, duplicate_artifact\n\
             Valid kinds (brownfield, Epic #287): hypothesis_duplicate, uncovered_use_case, \
             unverified_invariant, orphan_glossary_term, untriangulated_hypothesis"
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
        // Audit-r6 ARCH-H1: brownfield catalog (Epic #287).
        AnomalyKind::HypothesisDuplicate => "hypothesis_duplicate",
        AnomalyKind::UncoveredUseCase => "uncovered_use_case",
        AnomalyKind::UnverifiedInvariant => "unverified_invariant",
        AnomalyKind::OrphanGlossaryTerm => "orphan_glossary_term",
        AnomalyKind::UntriangulatedHypothesis => "untriangulated_hypothesis",
        // ADR-020 laundering guard.
        AnomalyKind::UnbackedDisplacement => "unbacked_displacement",
    }
}
