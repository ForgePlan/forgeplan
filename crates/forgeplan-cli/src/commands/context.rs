use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use chrono::{NaiveDate, Utc};
use console::style;

use forgeplan_core::artifact::frontmatter::Frontmatter;
use forgeplan_core::artifact::types::{ArtifactKind, Mode};
use forgeplan_core::db::store::{ArtifactRecord, frontmatter_map_with_links_from};
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::scoring::fgr;
use forgeplan_core::scoring::reff;
use forgeplan_core::status::derived::derive_status;
use forgeplan_core::validation;

use crate::commands::common;
use crate::ui;

pub async fn run(id: &str, json: bool) -> anyhow::Result<()> {
    let (workspace, source, _read_guard) = common::open_read_source().await?;

    let (canonical, _) = common::resolve_required_identity(&source, &workspace, id).await?;
    let id = canonical.as_str();
    let record: Cow<'_, ArtifactRecord> = match &source {
        common::ReadSource::Legacy(store) => store.get_record(&_read_guard, id).await?.map(Cow::Owned),
        common::ReadSource::Captured(read) => read.get_record(id).map(Cow::Borrowed),
    }
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
    let (outgoing, incoming) = match &source {
        common::ReadSource::Legacy(store) => (
            store.get_relations(&_read_guard, id).await?,
            store.get_incoming_relations(&_read_guard, id).await?,
        ),
        common::ReadSource::Captured(read) => {
            (read.get_relations(id), read.get_incoming_relations(id))
        }
    };

    // Categorize relations into graph structure
    let parent = record.parent_epic.clone().filter(|s| !s.is_empty());

    let mut evidence_ids: Vec<String> = Vec::new();
    let mut depends_on: Vec<String> = Vec::new();
    let mut dependents: Vec<String> = Vec::new();
    let mut related: Vec<String> = Vec::new();

    // Every record, in both layouts: the display map below names the endpoint
    // of every relation, not only of the evidence ones.
    let all_records: Cow<'_, [ArtifactRecord]> = match &source {
        common::ReadSource::Legacy(store) => Cow::Owned(store.list_records(&_read_guard, None).await?),
        common::ReadSource::Captured(read) => Cow::Borrowed(read.records()),
    };
    let all_evidence = || {
        all_records
            .iter()
            .filter(|record| record.kind.eq_ignore_ascii_case("evidence"))
    };
    // The relation endpoints are canonical keys (ADR-030) and the display id is
    // a repeatable alias, so an endpoint is matched on the key. Every list a
    // person reads still carries the display id.
    let display_by_key: HashMap<String, String> = all_records
        .iter()
        .map(|r| (r.canonical_key.to_uppercase(), r.id.clone()))
        .collect();
    let display_of = |endpoint: &str| -> String {
        display_by_key
            .get(&endpoint.to_uppercase())
            .cloned()
            .unwrap_or_else(|| endpoint.to_string())
    };
    let evidence_id_set: HashSet<String> = all_evidence()
        .map(|r| r.canonical_key.to_uppercase())
        .collect();
    // ADR-020: displayed links keep ALL packs (context is a history view),
    // but `has_evidence` below reasons over non-terminal packs only — an
    // all-displaced artifact needs "link a live successor", not
    // "review evidence quality".
    let eligible_evidence_id_set: HashSet<String> = all_evidence()
        .filter(|r| !forgeplan_core::lifecycle::transitions::is_terminal(&r.status))
        .map(|r| r.canonical_key.to_uppercase())
        .collect();

    for (target, rel) in &outgoing {
        if evidence_id_set.contains(&target.to_uppercase()) {
            evidence_ids.push(display_of(target));
        } else {
            match rel.as_str() {
                "based_on" | "refines" => depends_on.push(display_of(target)),
                "supersedes" | "contradicts" => related.push(display_of(target)),
                _ => related.push(display_of(target)),
            }
        }
    }

    // Incoming evidence links (evidence → this artifact)
    for (source, _rel) in &incoming {
        if evidence_id_set.contains(&source.to_uppercase())
            && !evidence_ids.iter().any(|e| e.eq_ignore_ascii_case(&display_of(source)))
        {
            evidence_ids.push(display_of(source));
        } else if !evidence_id_set.contains(&source.to_uppercase()) {
            dependents.push(display_of(source));
        }
    }

    // 4. R_eff via recursive scoring
    let mut visited = HashSet::new();
    let report = match &source {
        common::ReadSource::Legacy(store) => reff::r_eff_recursive(id, store, &_read_guard, &mut visited).await?,
        common::ReadSource::Captured(read) => {
            reff::r_eff_recursive_captured(id, read, &mut visited).await?
        }
    };

    // 5. Validation (catch errors, don't propagate)
    let fm = frontmatter_map_with_links_from(&record, &outgoing);
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

    let all_relations = match &source {
        common::ReadSource::Legacy(store) => store.get_all_relations(&_read_guard).await?,
        common::ReadSource::Captured(read) => read.get_all_relations(),
    };
    let link_count = all_relations
        .iter()
        // The relation endpoints are canonical keys (ADR-030); the display id
        // is a repeatable alias, so the comparison is made on the key.
        .filter(|(src, tgt, _)| {
            src.eq_ignore_ascii_case(&record.canonical_key)
                || tgt.eq_ignore_ascii_case(&record.canonical_key)
        })
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
    let has_evidence = evidence_ids
        .iter()
        .any(|e| eligible_evidence_id_set.contains(&e.to_uppercase()));
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
    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — emit slug pre-merge and
    // display id post-merge so commit `Refs:` lines stay canonical.
    let ref_form =
        forgeplan_core::artifact::frontmatter::refs_form_from_body(&record.body, &record.id);
    let mut hint_list: Vec<Hint> = Vec::new();
    if must_errors > 0 {
        hint_list.push(
            Hint::warning(format!("Fix {} MUST error(s)", must_errors))
                .with_action(format!("forgeplan validate {}", ref_form)),
        );
    // PRD-086 FR-001. `has_evidence` counts LINKED child packs, which a leaf
    // EvidencePack has none of by definition — it is the evidence. Since leaf
    // packs began scoring on their own structured fields, this branch fired
    // under an `R_eff: 1.00` printed a few lines above it, and its `Next:` told
    // the agent to create an EvidencePack for an EvidencePack. That is a
    // PRD-071 contract line an agent is obliged to run, so the cost is phantom
    // artifacts in the graph, not just a confusing sentence.
    //
    // Guarding on the score keeps the advice true without teaching this
    // function which kinds self-score.
    } else if !has_evidence && report.r_eff <= 0.0 {
        hint_list.push(
            Hint::warning("No evidence linked")
                .with_action(format!(
                    "forgeplan new evidence \"Evidence for {}\" && forgeplan link EVID-XXX {} --relation informs",
                    ref_form, ref_form
                )),
        );
    } else if validation_passed && report.r_eff > 0.0 && record.status == "draft" {
        hint_list.push(
            Hint::info(format!("Ready to activate {}", ref_form))
                .with_action(format!("forgeplan activate {}", ref_form)),
        );
    } else {
        hint_list
            .push(Hint::info("Verify R_eff").with_action(format!("forgeplan score {}", ref_form)));
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

    // PRD-086 FR-001 — see the hint above; same condition, same reason.
    if !has_evidence && r_eff <= 0.0 {
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
