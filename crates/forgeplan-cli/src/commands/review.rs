use console::style;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::lifecycle;

use crate::commands::common;

pub async fn run(id: &str) -> anyhow::Result<()> {
    let store = common::store().await?;
    // Phase 2.5 (PROB-060) — accept slug or display id form. Without
    // resolver, slug input fails в lifecycle::review's get_record path.
    let canonical = store
        .resolve_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found"))?;
    let result = lifecycle::review(&store, &canonical).await?;

    // Styled output — distinguish MUST errors from gate warnings
    if result.can_activate {
        println!("  {}", style("Review PASSED").green().bold());
        println!("  {}", style("Ready to activate").green());
    } else if !result.must_findings.is_empty() {
        println!("  {}", style("Review FAILED").red().bold());
        println!("  {}", style("Fix MUST validation errors first").red());
    } else {
        // Gates failed but no MUST errors — methodology gates (evidence, body length)
        println!("  {}", style("Review BLOCKED").yellow().bold());
        println!(
            "  {}",
            style("Methodology gates not met (see warnings below)").yellow()
        );
    }

    if !result.must_findings.is_empty() {
        println!();
        for finding in &result.must_findings {
            println!(
                "  {} [{}] {}",
                style("x").red().bold(),
                style("MUST").red().bold(),
                finding
            );
        }
    }

    if !result.should_findings.is_empty() {
        println!();
        for finding in &result.should_findings {
            println!(
                "  {} [{}] {}",
                style("!").yellow(),
                style("SHOULD").yellow(),
                finding
            );
        }
    }

    if !result.warnings.is_empty() {
        println!();
        for warning in &result.warnings {
            println!(
                "  {} {}",
                style("!").yellow().bold(),
                style(warning).yellow()
            );
        }
    }

    // Contextual hints
    let has_evidence = !result
        .warnings
        .iter()
        .any(|w| w.contains("No evidence linked"));
    let is_stub = result.warnings.iter().any(|w| w.contains("Body too short"));
    let has_must_errors = !result.must_findings.is_empty();
    // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — load the record so we
    // can pick slug pre-merge or display id post-merge for hint emission,
    // matching the canonical reference form used in commit `Refs:` lines.
    let record_opt = store.get_record(id).await.ok().flatten();
    let kind = record_opt
        .as_ref()
        .and_then(|r| {
            r.kind
                .parse::<forgeplan_core::artifact::types::ArtifactKind>()
                .ok()
        })
        .unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
    let ref_form = record_opt
        .as_ref()
        .map(|r| forgeplan_core::artifact::frontmatter::refs_form_from_body(&r.body, &r.id))
        .unwrap_or_else(|| id.to_string());
    let review_hints = forgeplan_core::hints::review_hints(
        &ref_form,
        has_evidence,
        is_stub,
        has_must_errors,
        &kind,
    );
    if !review_hints.is_empty() {
        print!("{}", forgeplan_core::hints::format_hints(&review_hints));
    }

    // PRD-071 contract: emit single primary next-action.
    // - MUST errors → Fix path (validate to see specifics)
    // - clean review → Next path (activate)
    // - intermediate (warnings only) → Next path (advisory hints already covered above)
    let next_hints: Vec<Hint> = if has_must_errors {
        vec![
            Hint::warning("Validation has MUST errors")
                .with_action(format!("forgeplan validate {}", ref_form)),
        ]
    } else if result.can_activate {
        vec![Hint::info("Review passed").with_action(format!("forgeplan activate {}", ref_form))]
    } else {
        // Pull primary action from the advisory review_hints (e.g. add evidence).
        match hints::primary_action(&review_hints) {
            Some(action) => vec![Hint::info("Methodology gate not met").with_action(action)],
            None => Vec::new(),
        }
    };
    print!("{}", hints::render_next_action_line(&next_hints));

    Ok(())
}
