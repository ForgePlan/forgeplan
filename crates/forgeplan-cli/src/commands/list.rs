use anyhow::Result;
use console::style;

use forgeplan_core::hints::{self, Hint};
use forgeplan_core::search::filter::ArtifactFilter as QueryFilter;

use crate::commands::common;
use crate::ui;

pub async fn run(
    kind_filter: Option<&str>,
    status_filter: Option<&str>,
    tag_filter: Option<&str>,
    json: bool,
) -> Result<()> {
    let store = common::store().await?;

    // Sprint 13.3 H1: compose all predicates through the search filter DSL
    // so kind+status+tag work together (previously tag forced bespoke
    // list_by_tag + retain() workaround that duplicated DSL logic).
    let mut clauses: Vec<QueryFilter> = Vec::new();
    if let Some(k) = kind_filter {
        clauses.push(QueryFilter::Kind(k.to_string()));
    }
    if let Some(s) = status_filter {
        clauses.push(QueryFilter::Status(s.to_string()));
    }
    if let Some(t) = tag_filter {
        clauses.push(QueryFilter::HasTag(t.to_string()));
    }

    let composed = if clauses.is_empty() {
        None
    } else if clauses.len() == 1 {
        Some(clauses.into_iter().next().unwrap())
    } else {
        Some(QueryFilter::And(clauses))
    };

    let all = store.list_records(None).await?;
    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — capture full records so
    // hint emission can pick slug vs display id based on each artifact's
    // pre/post-merge state. The summary projection is built afterwards.
    let full_records: Vec<_> = all
        .into_iter()
        .filter(|r| composed.as_ref().map(|f| f.matches(r)).unwrap_or(true))
        .collect();
    let artifacts: Vec<_> = full_records.iter().map(|r| r.to_summary()).collect();

    let mut hints_vec: Vec<Hint> = Vec::new();

    if artifacts.is_empty() {
        // Empty workspace — guide user to create the first PRD.
        let action = match kind_filter {
            Some("prd") | None => "forgeplan new prd \"<title>\"".to_string(),
            Some(k) => format!("forgeplan new {} \"<title>\"", k),
        };
        hints_vec.push(Hint::suggestion("No artifacts match — create one").with_action(action));

        if json {
            // PRD-071: bw-compat — stdout is bare array so existing
            // `forgeplan list --json | jq '.[]'` consumers still work.
            // Hint goes to stderr per the additive `Next:` marker rule.
            println!("[]");
            if let Some(next) = hints::primary_action(&hints_vec) {
                eprintln!("Next: {}", next);
            }
        } else {
            println!("  No artifacts found.");
            print!("{}", hints::render_next_action_line(&hints_vec));
        }
        return Ok(());
    }

    // Pick first artifact for default Next: action.
    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — emit slug for pre-merge
    // artifacts, display id otherwise, so the agent's next `forgeplan get`
    // call uses the canonical reference form (matters for commit `Refs:`).
    let first_record = &full_records[0];
    let first_ref = forgeplan_core::artifact::frontmatter::refs_form_from_body(
        &first_record.body,
        &first_record.id,
    );
    hints_vec.push(
        Hint::info(format!("Inspect {}", first_ref))
            .with_action(format!("forgeplan get {}", first_ref)),
    );

    if json {
        let json_data: Vec<_> = artifacts
            .iter()
            .map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "kind": a.kind,
                    "status": a.status,
                    "title": a.title,
                })
            })
            .collect();
        // PRD-071: bw-compat — stdout is bare array so existing
        // `forgeplan list --json | jq '.[]'` consumers still work.
        // Hint goes to stderr per the additive `Next:` marker rule.
        println!("{}", serde_json::to_string_pretty(&json_data)?);
        if let Some(next) = hints::primary_action(&hints_vec) {
            eprintln!("Next: {}", next);
        }
        return Ok(());
    }

    // Calculate column widths for alignment
    let id_width = artifacts
        .iter()
        .map(|a| a.id.len())
        .max()
        .unwrap_or(6)
        .max(2);
    let kind_width = artifacts
        .iter()
        .map(|a| a.kind.len())
        .max()
        .unwrap_or(6)
        .max(4);
    let status_width = artifacts
        .iter()
        .map(|a| a.status.len())
        .max()
        .unwrap_or(6)
        .max(6);

    // Print header — bold underlined
    println!(
        "{:<id_w$}  {:<kind_w$}  {:<status_w$}  {}",
        style("ID").bold().underlined(),
        style("Kind").bold().underlined(),
        style("Status").bold().underlined(),
        style("Title").bold().underlined(),
        id_w = id_width,
        kind_w = kind_width,
        status_w = status_width,
    );

    // Print rows
    for a in &artifacts {
        // Pad status manually so ANSI codes don't break alignment
        let status_plain_len = a.status.len();
        let status_styled = ui::styled_status(&a.status);
        let status_padding = if status_width > status_plain_len {
            " ".repeat(status_width - status_plain_len)
        } else {
            String::new()
        };

        println!(
            "{:<id_w$}  {:<kind_w$}  {}{}  {}",
            style(&a.id).bold(),
            a.kind,
            status_styled,
            status_padding,
            a.title,
            id_w = id_width,
            kind_w = kind_width,
        );
    }

    println!("\n  {} artifact(s) total", artifacts.len());
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}
