use std::collections::HashSet;

use chrono::{NaiveDate, Utc};
use console::style;

use forgeplan_core::artifact::frontmatter::Frontmatter;
use forgeplan_core::artifact::types::{ArtifactKind, Mode};
use forgeplan_core::db::store::ArtifactFilter;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::scoring::fgr;
use forgeplan_core::scoring::reff;
use forgeplan_core::status::derived::derive_status;
use forgeplan_core::validation;

use crate::commands::common;
use crate::ui;

pub async fn run(id: &str, json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;

    // 1. Fetch artifact
    let record = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", id))?;

    // 2. Parse kind and depth
    let kind: ArtifactKind = record.kind.parse().unwrap_or_else(|_| {
        eprintln!(
            "  Warning: unknown kind '{}', defaulting to Note",
            record.kind
        );
        ArtifactKind::Note
    });
    let depth: Mode = record.depth.parse().unwrap_or_else(|_| {
        eprintln!(
            "  Warning: unknown depth '{}', defaulting to Standard",
            record.depth
        );
        Mode::Standard
    });

    // 3. Relations — outgoing and incoming
    let outgoing = store.get_relations(id).await?;
    let incoming = store.get_incoming_relations(id).await?;

    // Categorize relations into graph structure
    let parent = record.parent_epic.clone().filter(|s| !s.is_empty());

    let mut evidence_ids: Vec<String> = Vec::new();
    let mut depends_on: Vec<String> = Vec::new();
    let mut dependents: Vec<String> = Vec::new();
    let mut related: Vec<String> = Vec::new();

    // Check which outgoing targets are evidence
    let evidence_filter = ArtifactFilter {
        kind: Some("evidence".to_string()),
        status: None,
    };
    let all_evidence = store.list_records(Some(&evidence_filter)).await?;
    let evidence_id_set: HashSet<String> =
        all_evidence.iter().map(|r| r.id.to_uppercase()).collect();

    for (target, rel) in &outgoing {
        if evidence_id_set.contains(&target.to_uppercase()) {
            evidence_ids.push(target.clone());
        } else {
            match rel.as_str() {
                "based_on" | "refines" => depends_on.push(target.clone()),
                "supersedes" | "contradicts" => related.push(target.clone()),
                _ => related.push(target.clone()),
            }
        }
    }

    // Incoming evidence links (evidence → this artifact)
    for (source, _rel) in &incoming {
        if evidence_id_set.contains(&source.to_uppercase())
            && !evidence_ids.iter().any(|e| e.eq_ignore_ascii_case(source))
        {
            evidence_ids.push(source.clone());
        } else if !evidence_id_set.contains(&source.to_uppercase()) {
            dependents.push(source.clone());
        }
    }

    // 4. R_eff via recursive scoring
    let mut visited = HashSet::new();
    let report = reff::r_eff_recursive(id, &store, &mut visited).await?;

    // 5. Validation (catch errors, don't propagate)
    let fm = record.frontmatter_map();
    let val_result = validation::validate(&record.id, &record.body, &fm, &kind, &depth);
    let must_errors = val_result.error_count();
    let should_warnings = val_result.warning_count();
    let validation_passed = val_result.passed();

    // 6. F-G-R computation
    let frontmatter = Frontmatter::new();
    let is_stale = record
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

    let all_relations = store.get_all_relations().await?;
    let link_count = all_relations
        .iter()
        .filter(|(src, tgt, _)| src.eq_ignore_ascii_case(id) || tgt.eq_ignore_ascii_case(id))
        .count();

    let fpf_weights = common::config().ok().and_then(|c| c.fpf.map(|f| f.weights));
    let fgr_score = fgr::compute(
        id,
        &record.body,
        &frontmatter,
        &kind,
        &depth,
        report.r_eff,
        link_count,
        is_stale,
        fpf_weights.as_ref(),
    );

    // 7. Derived status
    let has_evidence = !evidence_ids.is_empty();
    let derived = derive_status(
        &record.status,
        &record.body,
        &record.kind,
        has_evidence,
        report.r_eff,
        validation_passed,
    );

    // 8. Suggestions
    let suggestions = build_suggestions(
        &record.id,
        &record.status,
        has_evidence,
        report.r_eff,
        validation_passed,
        must_errors,
        &fgr_score,
    );

    // Build a single primary next-action with a real command, prioritised
    // by severity: validation errors > missing evidence > ready-to-activate
    // > otherwise score.
    let mut hint_list: Vec<Hint> = Vec::new();
    if must_errors > 0 {
        hint_list.push(
            Hint::warning(format!("Fix {} MUST error(s)", must_errors))
                .with_action(format!("forgeplan validate {}", record.id)),
        );
    } else if !has_evidence {
        hint_list.push(
            Hint::warning("No evidence linked")
                .with_action(format!(
                    "forgeplan new evidence \"Evidence for {}\" && forgeplan link EVID-XXX {} --relation informs",
                    record.id, record.id
                )),
        );
    } else if validation_passed && report.r_eff > 0.0 && record.status == "draft" {
        hint_list.push(
            Hint::info(format!("Ready to activate {}", record.id))
                .with_action(format!("forgeplan activate {}", record.id)),
        );
    } else {
        hint_list
            .push(Hint::info("Verify R_eff").with_action(format!("forgeplan score {}", record.id)));
    }

    // --- Output ---
    if json {
        let json_data = serde_json::json!({
            "artifact": {
                "id": record.id,
                "kind": record.kind,
                "status": record.status,
                "title": record.title,
                "depth": record.depth,
                "r_eff": report.r_eff,
                "created_at": record.created_at,
                "updated_at": record.updated_at,
            },
            "derived_status": derived.label(),
            "graph": {
                "parent": parent,
                "evidence": evidence_ids,
                "depends_on": depends_on,
                "dependents": dependents,
                "related": related,
            },
            "validation": {
                "passed": validation_passed,
                "must_errors": must_errors,
                "should_warnings": should_warnings,
            },
            "fgr": {
                "formality": fgr_score.formality,
                "granularity": fgr_score.granularity,
                "reliability": fgr_score.reliability,
                "grade": fgr_score.grade(),
            },
            "suggestions": suggestions,
            "_next_action": hints::primary_action(&hint_list),
            "hints": hint_list,
        });
        println!("{}", serde_json::to_string_pretty(&json_data)?);
        return Ok(());
    }

    // --- Human-readable output ---
    ui::header(&record.id, &record.title);

    ui::kv(
        "Status",
        &format!(
            "{} ({})",
            ui::styled_status(&record.status),
            derived.label(),
        ),
    );
    ui::kv("Depth", &ui::styled_depth(&record.depth));
    ui::kv("R_eff", &ui::styled_reff(report.r_eff));
    ui::kv(
        "F-G-R",
        &format!(
            "{} (F={:.1} G={:.1} R={:.1})",
            fgr_score.grade(),
            fgr_score.formality,
            fgr_score.granularity,
            fgr_score.reliability,
        ),
    );

    // Graph section
    ui::section("Graph");
    if let Some(ref p) = parent {
        println!("    {:<14}{}", style("Parent:").dim(), p);
    }
    if !evidence_ids.is_empty() {
        println!(
            "    {:<14}{}",
            style("Evidence:").dim(),
            evidence_ids.join(", ")
        );
    }
    if !depends_on.is_empty() {
        println!(
            "    {:<14}{}",
            style("Depends on:").dim(),
            depends_on.join(", ")
        );
    }
    if !dependents.is_empty() {
        println!(
            "    {:<14}{}",
            style("Dependents:").dim(),
            dependents.join(", ")
        );
    }
    if !related.is_empty() {
        println!("    {:<14}{}", style("Related:").dim(), related.join(", "));
    }
    if parent.is_none()
        && evidence_ids.is_empty()
        && depends_on.is_empty()
        && dependents.is_empty()
        && related.is_empty()
    {
        println!("    {}", style("(no links)").dim());
    }

    // Validation section
    let val_status = if validation_passed && must_errors == 0 && should_warnings == 0 {
        style("PASS").green().bold().to_string()
    } else if validation_passed {
        format!("{}", style("PASS").green())
    } else {
        style("FAIL").red().bold().to_string()
    };
    println!();
    ui::kv(
        "Validation",
        &format!(
            "{} ({} error(s), {} warning(s))",
            val_status, must_errors, should_warnings
        ),
    );

    // Suggestions
    if !suggestions.is_empty() {
        ui::section("Suggestions");
        for s in &suggestions {
            println!("    {} {}", style("->").yellow(), s);
        }
    }

    println!();
    print!("{}", hints::render_next_action_line(&hint_list));
    Ok(())
}

/// Generate actionable suggestions based on current artifact state.
fn build_suggestions(
    id: &str,
    status: &str,
    has_evidence: bool,
    r_eff: f64,
    validation_passed: bool,
    must_errors: usize,
    fgr: &fgr::FgrScore,
) -> Vec<String> {
    let mut suggestions = Vec::new();

    if must_errors > 0 {
        suggestions.push(format!(
            "Fix {} MUST error(s) — run `forgeplan validate {}`",
            must_errors, id
        ));
    }

    if !has_evidence {
        suggestions.push(format!(
            "Add evidence — `forgeplan new evidence \"Evidence for {}\"` + `forgeplan link EVID-XXX {} --relation informs`",
            id, id
        ));
    } else if r_eff < 0.3 {
        suggestions.push(format!(
            "Low R_eff ({:.2}) — review evidence quality (congruence_level, verdict)",
            r_eff
        ));
    }

    if validation_passed && has_evidence && r_eff > 0.0 && status == "draft" {
        suggestions.push(format!("Ready to activate — `forgeplan activate {}`", id));
    }

    if fgr.formality < 0.4 {
        suggestions.push("Low formality — fill in required sections".to_string());
    }

    if fgr.granularity < 0.4 {
        suggestions.push("Low granularity — add more detail and FR checkboxes".to_string());
    }

    suggestions
}
