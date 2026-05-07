use std::collections::HashSet;

use chrono::{NaiveDate, Utc};

use forgeplan_core::artifact::frontmatter::Frontmatter;
use forgeplan_core::artifact::types::{ArtifactKind, Mode};
use forgeplan_core::db::store::ArtifactFilter;
use forgeplan_core::hints;
use forgeplan_core::scoring::evidence::parse_evidence_from_record;
use forgeplan_core::scoring::fgr;
use forgeplan_core::scoring::reff::{self, EvidenceItem};

use crate::commands::common;
use crate::ui;

/// Score all active decision artifacts and update cached R_eff.
pub async fn run_all(json: bool) -> anyhow::Result<()> {
    use forgeplan_core::artifact::types::DECISION_KINDS_EVIDENCE;

    // PROB-058 AC-2: acquire workspace lock so concurrent CLI invocations
    // (operator running `score-all` while a multi-agent dispatch runs `link`
    // в parallel — PRD-057) cannot race on `update_r_eff_score` writes.
    let (_ws, _lock, store) = common::open_store_locked().await?;
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
    let mut errors: Vec<serde_json::Value> = Vec::new();
    for record in &decision_records {
        // PRD-075 FR-004: route batch reconciliation through the same shared
        // helper that mutators (link/unlink/activate) use.
        let r_eff = match forgeplan_core::scoring::sync_score_target(&store, &record.id).await {
            Ok(report) => report.r_eff,
            Err(e) => {
                // Round 8 audit MED-4: surface failed artifacts in JSON instead
                // of silently dropping them, so scripted consumers cannot
                // mistake partial success for full success.
                let msg = e.to_string();
                eprintln!("  Warning: could not score {}: {msg}", record.id);
                errors.push(serde_json::json!({"id": record.id, "error": msg}));
                continue;
            }
        };

        if !json {
            let symbol = if r_eff >= 0.5 {
                "+"
            } else if r_eff >= 0.1 {
                "~"
            } else {
                "!"
            };
            println!("  {} {} → R_eff={:.2}", symbol, record.id, r_eff);
        }
        results.push(serde_json::json!({"id": record.id, "r_eff": r_eff}));
    }

    if json {
        // Round 8 audit MED-4: include `errors` so scripted consumers can
        // distinguish partial vs full success. Empty array stays present so
        // the schema is stable.
        let payload = serde_json::json!({
            "results": results,
            "errors": errors,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        let high = results
            .iter()
            .filter(|r| r["r_eff"].as_f64().unwrap_or(0.0) >= 0.5)
            .count();
        let total = results.len();
        println!();
        println!("  {}/{} artifacts with R_eff >= 0.5", high, total);
        if !errors.is_empty() {
            println!("  ! {} artifacts skipped due to errors", errors.len());
        }
    }

    Ok(())
}

pub async fn run(id: Option<&str>, json: bool) -> anyhow::Result<()> {
    let target_id_input = id.ok_or_else(|| anyhow::anyhow!("Usage: forgeplan score <ID>"))?;

    // PROB-058 AC-2: acquire workspace lock — `score` writes `r_eff_score` and
    // must serialize against concurrent `link` / `activate` to avoid
    // latest-writer-wins data loss.
    let (_ws, _lock, store) = common::open_store_locked().await?;

    // PROB-060 / SPEC-005 Phase 1.5b — slug-aware lookup.
    let target_id_owned = store.resolve_id(target_id_input).await?.ok_or_else(|| {
        anyhow::anyhow!("Artifact '{target_id_input}' not found\nFix: forgeplan list")
    })?;
    let target_id = target_id_owned.as_str();

    // Get the target artifact
    let target = store.get_record(target_id).await?.ok_or_else(|| {
        anyhow::anyhow!("Artifact '{}' not found\nFix: forgeplan list", target_id)
    })?;

    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — pick the canonical
    // reference form for hint emission: slug pre-merge, display id
    // post-merge. Body comes from LanceStore; refs_form_from_body is
    // non-fatal on legacy / malformed frontmatter.
    let target_ref =
        forgeplan_core::artifact::frontmatter::refs_form_from_body(&target.body, &target.id);

    // --- Recursive R_eff via AssuranceReport ---
    // PRD-075 FR-004 + Round 8 audit MED-1: the shared helper persists the
    // score AND returns the report so we avoid a second recursive walk for
    // display. On persist failure we fall back to a fresh recursive walk so
    // the user still sees a breakdown — but we annotate it with a warning so
    // the displayed value cannot silently disagree with stored cache.
    let report = match forgeplan_core::scoring::sync_score_target(&store, target_id).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("  Warning: could not persist R_eff score: {e}");
            eprintln!("Fix: forgeplan score-all");
            let mut visited = HashSet::new();
            reff::r_eff_recursive(target_id, &store, &mut visited).await?
        }
    };

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

        // PRD-071 contract: surface deterministic primary next-action.
        // Reuse score_hints (already exposed in text mode). When all hints
        // are silent, fall back to "activate" (R_eff is healthy).
        let cl0_count_json = evidence_items
            .iter()
            .filter(|e| e.congruence_level == 0)
            .count();
        let score_hints_json = hints::score_hints(
            &target_ref,
            report.r_eff,
            !evidence_items.is_empty(),
            cl0_count_json,
        );
        let next_action_json = hints::primary_action(&score_hints_json).or_else(|| {
            if report.r_eff >= 0.5 && target.status == "draft" {
                Some(format!("forgeplan activate {}", target_ref))
            } else {
                None
            }
        });

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
            "_next_action": next_action_json,
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
                "forgeplan new evidence \"Benchmark for {}\" && forgeplan link EVID-NNN {} --relation informs",
                target_ref, target_ref
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
    let score_hints =
        forgeplan_core::hints::score_hints(&target_ref, report.r_eff, has_evidence, cl0_count);
    if !score_hints.is_empty() {
        print!("{}", forgeplan_core::hints::format_hints(&score_hints));
    }

    // PRD-071 contract: surface single primary next-action via Next: line.
    // Falls back to activate when score is healthy and artifact is still draft.
    let next_hints: Vec<forgeplan_core::hints::Hint> =
        if let Some(action) = hints::primary_action(&score_hints) {
            vec![forgeplan_core::hints::Hint::info("score advisory").with_action(action)]
        } else if report.r_eff >= 0.5 && target.status == "draft" {
            vec![
                forgeplan_core::hints::Hint::info("R_eff healthy — ready to activate")
                    .with_action(format!("forgeplan activate {}", target_ref)),
            ]
        } else {
            Vec::new()
        };
    print!(
        "{}",
        forgeplan_core::hints::render_next_action_line(&next_hints)
    );

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
