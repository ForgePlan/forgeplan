use std::collections::HashSet;

use chrono::{NaiveDate, Utc};

use forgeplan_core::artifact::frontmatter::Frontmatter;
use forgeplan_core::artifact::types::{ArtifactKind, Mode};
use forgeplan_core::db::store::ArtifactFilter;
use forgeplan_core::scoring::evidence::parse_evidence_from_record;
use forgeplan_core::scoring::fgr;
use forgeplan_core::scoring::reff::{self, EvidenceItem};

use crate::commands::common;
use crate::ui;

/// Score all active decision artifacts and update cached R_eff.
pub async fn run_all(json: bool) -> anyhow::Result<()> {
    use forgeplan_core::artifact::types::DECISION_KINDS_EVIDENCE;

    let store = common::store().await?;
    let records = store.list_records(None).await?;

    let decision_records: Vec<_> = records
        .iter()
        .filter(|r| r.status == "active" && DECISION_KINDS_EVIDENCE.contains(&r.kind.as_str()))
        .collect();

    if decision_records.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("  No active decision artifacts found.");
        }
        return Ok(());
    }

    if !json {
        println!(
            "  Scoring {} active decision artifacts...",
            decision_records.len()
        );
        println!();
    }

    let mut results = Vec::new();
    for record in &decision_records {
        let mut visited = HashSet::new();
        let report = reff::r_eff_recursive(&record.id, &store, &mut visited).await?;

        if let Err(e) = store.update_r_eff_score(&record.id, report.r_eff).await {
            eprintln!("  Warning: could not persist R_eff for {}: {e}", record.id);
        }

        if !json {
            let symbol = if report.r_eff >= 0.5 {
                "+"
            } else if report.r_eff >= 0.1 {
                "~"
            } else {
                "!"
            };
            println!("  {} {} → R_eff={:.2}", symbol, record.id, report.r_eff);
        }
        results.push(serde_json::json!({"id": record.id, "r_eff": report.r_eff}));
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        let high = results
            .iter()
            .filter(|r| r["r_eff"].as_f64().unwrap_or(0.0) >= 0.5)
            .count();
        let total = results.len();
        println!();
        println!("  {}/{} artifacts with R_eff >= 0.5", high, total);
    }

    Ok(())
}

pub async fn run(id: Option<&str>, json: bool) -> anyhow::Result<()> {
    let target_id = id.ok_or_else(|| anyhow::anyhow!("Usage: forgeplan score <ID>"))?;

    let store = common::store().await?;

    // Get the target artifact
    let target = store
        .get_record(target_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", target_id))?;

    // --- Recursive R_eff via AssuranceReport ---
    let mut visited = HashSet::new();
    let report = reff::r_eff_recursive(target_id, &store, &mut visited).await?;

    // Write R_eff back to LanceDB (soft error — don't block display)
    if let Err(e) = store.update_r_eff_score(target_id, report.r_eff).await {
        eprintln!("  Warning: could not persist R_eff score: {e}");
    }

    // --- Flat evidence list for display (backward-compat) ---
    let outgoing = store.get_relations(target_id).await?;
    let evidence_targets: Vec<String> = outgoing
        .iter()
        .filter(|(_, rel)| rel == "informs" || rel == "based_on" || rel == "refines")
        .map(|(t, _)| t.clone())
        .collect();

    let filter = ArtifactFilter {
        kind: Some("evidence".to_string()),
        status: None,
    };
    let evidence_records = store.list_records(Some(&filter)).await?;

    let mut evidence_items: Vec<EvidenceItem> = Vec::new();

    for ev_record in &evidence_records {
        let is_linked = evidence_targets
            .iter()
            .any(|eid| eid.eq_ignore_ascii_case(&ev_record.id));

        if !is_linked {
            let ev_relations = store.get_relations(&ev_record.id).await?;
            let links_to_target = ev_relations
                .iter()
                .any(|(t, _)| t.eq_ignore_ascii_case(target_id));
            if !links_to_target {
                continue;
            }
        }

        let item = parse_evidence_from_record(ev_record);
        evidence_items.push(item);
    }

    // --- F-G-R computation ---
    let kind: ArtifactKind = target.kind.parse().unwrap_or_else(|_| {
        eprintln!(
            "  Warning: unknown kind '{}', defaulting to Note",
            target.kind
        );
        ArtifactKind::Note
    });
    let depth: Mode = target.depth.parse().unwrap_or_else(|_| {
        eprintln!(
            "  Warning: unknown depth '{}', defaulting to Standard",
            target.depth
        );
        Mode::Standard
    });
    let frontmatter: Frontmatter = Frontmatter::new();

    // Determine staleness from valid_until
    let is_stale = target
        .valid_until
        .as_deref()
        .and_then(|s| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().or_else(|| {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                    .ok()
                    .map(|dt| dt.date())
            })
        })
        .map(|d| Utc::now().date_naive() > d)
        .unwrap_or(false);

    // Link count for reliability
    let all_relations = store.get_all_relations().await?;
    let link_count = all_relations
        .iter()
        .filter(|(src, tgt, _)| src == target_id || tgt == target_id)
        .count();

    let fpf_weights = common::config().ok().and_then(|c| c.fpf.map(|f| f.weights));
    let fgr_score = fgr::compute(
        target_id,
        &target.body,
        &frontmatter,
        &kind,
        &depth,
        report.r_eff,
        link_count,
        is_stale,
        fpf_weights.as_ref(),
    );

    // --- R_eff Confidence Interval (PRD-040 FR-002) ---
    let ci = reff::r_eff_with_ci(&evidence_items);

    // --- JSON output ---
    if json {
        let evidence_json: Vec<_> = evidence_items
            .iter()
            .map(|item| {
                let item_score = reff::r_eff(std::slice::from_ref(item));
                serde_json::json!({
                    "id": item.id,
                    "verdict": format!("{:?}", item.verdict),
                    "congruence_level": item.congruence_level,
                    "score": item_score,
                    "expired": item.valid_until.map(|dt| Utc::now().naive_utc() > dt).unwrap_or(false),
                })
            })
            .collect();

        let json_data = serde_json::json!({
            "id": target.id,
            "title": target.title,
            "r_eff": report.r_eff,
            "r_eff_ci": {
                "point": ci.point,
                "low": ci.low,
                "high": ci.high,
                "evidence_count": ci.evidence_count,
                "stale_count": ci.stale_count,
                "insufficient": ci.is_insufficient(),
                "width": ci.width(),
            },
            "weakest_link": report.weakest_link,
            "factors": report.factors,
            "evidence": evidence_json,
            "fgr": {
                "formality": fgr_score.formality,
                "granularity": fgr_score.granularity,
                "reliability": fgr_score.reliability,
                "overall": fgr_score.overall(),
                "grade": fgr_score.grade(),
            },
        });
        println!("{}", serde_json::to_string_pretty(&json_data)?);
        return Ok(());
    }

    // --- Styled display ---
    ui::header(&target.id, &target.title);

    if evidence_items.is_empty() {
        ui::info("No evidence linked. R_eff = 0.0");
        println!();
        ui::error_hint(
            "No evidence found",
            &format!(
                "forgeplan new evidence \"Benchmark for {}\" && forgeplan link EVID-XXX {} --relation informs",
                target.id, target.id
            ),
        );
    } else {
        ui::section("Evidence breakdown");
        for item in &evidence_items {
            let expired = item
                .valid_until
                .map(|dt| Utc::now().naive_utc() > dt)
                .unwrap_or(false);
            let item_score = reff::r_eff(std::slice::from_ref(item));
            println!(
                "    {} [{:?}] CL{} = {:.1}{}",
                item.id,
                item.verdict,
                item.congruence_level,
                item_score,
                if expired { " (EXPIRED)" } else { "" }
            );
        }
        println!();

        let status = if report.r_eff >= 0.5 {
            "Adequate"
        } else if report.r_eff >= 0.3 {
            "Needs Review"
        } else {
            "AT RISK"
        };

        ui::kv(
            "R_eff",
            &format!("{} -- {}", ui::styled_reff(report.r_eff), status),
        );

        // Confidence interval (PRD-040 FR-002)
        if ci.evidence_count > 0 {
            let ci_label = if ci.is_insufficient() {
                format!("insufficient ({} evidence)", ci.evidence_count)
            } else if ci.stale_count > 0 {
                format!(
                    "[{:.2} — {:.2}] ({} fresh, {} stale)",
                    ci.low,
                    ci.high,
                    ci.evidence_count - ci.stale_count,
                    ci.stale_count
                )
            } else {
                format!(
                    "[{:.2} — {:.2}] ({} evidence)",
                    ci.low, ci.high, ci.evidence_count
                )
            };
            ui::kv("Confidence", &ci_label);
        }
    }

    if let Some(ref wl) = report.weakest_link {
        ui::kv("Weakest link", wl);
    }

    if !report.factors.is_empty() {
        println!();
        for factor in &report.factors {
            println!("  \u{2022} {}", factor);
        }
    }

    ui::section("Quality (F-G-R)");
    println!(
        "    Formality:    {:.2} ({})",
        fgr_score.formality,
        score_grade(fgr_score.formality)
    );
    println!(
        "    Granularity:  {:.2} ({})",
        fgr_score.granularity,
        score_grade(fgr_score.granularity)
    );
    println!(
        "    Reliability:  {:.2} ({})",
        fgr_score.reliability,
        score_grade(fgr_score.reliability)
    );
    println!(
        "    Overall:      {:.2} ({})",
        fgr_score.overall(),
        fgr_score.grade()
    );

    // Hints for low scores
    if fgr_score.formality < 0.4 {
        ui::warning("Fill in required sections to improve formality");
    }
    if fgr_score.granularity < 0.4 {
        ui::warning("Add more FR checkboxes to improve granularity");
    }
    if fgr_score.reliability < 0.3 {
        ui::warning("Add evidence with `forgeplan new evidence`");
    }

    // Contextual hints
    let has_evidence = !evidence_items.is_empty();
    let cl0_count = evidence_items
        .iter()
        .filter(|e| e.congruence_level == 0)
        .count();
    let score_hints = forgeplan_core::hints::score_hints(report.r_eff, has_evidence, cl0_count);
    if !score_hints.is_empty() {
        print!("{}", forgeplan_core::hints::format_hints(&score_hints));
    }

    println!();

    Ok(())
}

/// Per-dimension grade (same thresholds as FgrScore::grade but for a single value).
fn score_grade(v: f64) -> &'static str {
    if v > 0.8 {
        "A"
    } else if v > 0.6 {
        "B"
    } else if v > 0.4 {
        "C"
    } else if v > 0.2 {
        "D"
    } else {
        "F"
    }
}

// PROB-031 fix: removed local `parse_evidence_from_record` and `extract_field`
// — they duplicated forgeplan_core::scoring::evidence::parse_evidence_from_record
// but with a DIFFERENT default: CL0 (penalty 0.9) vs core's CL3 (no penalty,
// trust-local default).
//
// This caused a visible contradiction: the per-item "breakdown" line used CLI
// parser and showed "EVID-001 [Supports] CL0 = 0.1" while the R_eff rollup
// used core's parser via r_eff_recursive and computed 1.00 (CL3 default).
//
// Also: the core parser implements the PRD-035 Sprint 13.3 H2 security
// precedence (`min(tier_cl, explicit_cl)`) that prevents trust amplification
// via self-signed T1 evidence. The CLI local parser did not implement this,
// opening the same attack surface on the display path.
//
// Fix: import the core parser and delete the local duplicate. Both paths
// now agree on CL and on formula.
