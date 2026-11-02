use console::style;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::lifecycle;

use crate::commands::common;

pub async fn run(id: &str) -> anyhow::Result<()> {
    // `review` is read-only: it runs the validation rules and the activation
    // gates, both of which need the artifact's relation graph. On a migrated
    // workspace that graph is the canonical Markdown's `links:` blocks, so the
    // read comes from one capture instead of the retired `relations.lance`.
    // Hold one shared permit through selector resolution, review and rendering.
    let ws = common::find_workspace_path()?;
    let read_guard = forgeplan_core::workspace::lock::acquire_workspace_read_lock(&ws).await?;
    let (_, store) = common::open_store_tolerating_constellation().await?;

    let (result, kind, ref_form) = match &store {
        Some(store) => {
            // Phase 2.5 (PROB-060) — accept slug or display id form. Without
            // resolver, slug input fails в lifecycle::review's get_record path.
            let resolution = common::resolve_against_workspace(&ws, id).await?;
            let (canonical, _) = common::require_found_identity(id, &resolution)?;
            let result = lifecycle::review(&read_guard, store, &canonical).await?;
            // PROB-060 / SPEC-005 / ADR-012 (W1.B, CD-5) — load the record so we
            // can pick slug pre-merge or display id post-merge for hint emission,
            // matching the canonical reference form used in commit `Refs:` lines.
            let record = store
                .get_record(&read_guard, &canonical)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Artifact '{canonical}' not found"))?;
            let kind = record
                .kind
                .parse::<forgeplan_core::artifact::types::ArtifactKind>()
                .unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
            let ref_form = forgeplan_core::artifact::frontmatter::refs_form_from_body(
                &record.body,
                &record.id,
            );
            (result, kind, ref_form)
        }
        None => {
            let read = forgeplan_core::constellation::facade::open_workspace_read_with_permit(&ws, &read_guard).await?;
            let resolution = common::resolve_against_capture(&read, id);
            let (canonical, _) = common::require_found_identity(id, &resolution)?;
            let result = forgeplan_core::lifecycle::canonical::review_captured(&read, &canonical)?;
            let kind = read
                .get_record(&canonical)
                .and_then(|r| {
                    r.kind
                        .parse::<forgeplan_core::artifact::types::ArtifactKind>()
                        .ok()
                })
                .unwrap_or(forgeplan_core::artifact::types::ArtifactKind::Note);
            let ref_form = forgeplan_core::lifecycle::canonical::refs_form(&read, &canonical);
            (result, kind, ref_form)
        }
    };

    render(&result, kind, &ref_form)
}

/// Render a review verdict.
///
/// One renderer for both read sources: the finding list, the gate warnings and
/// the hint chain are identical for a store-backed review and a captured one,
/// so a divergence here would be a printing bug the operator reads as a
/// methodology difference.
fn render(
    result: &lifecycle::ReviewResult,
    kind: forgeplan_core::artifact::types::ArtifactKind,
    ref_form: &str,
) -> anyhow::Result<()> {
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
    // The stub is recognised by the validation's stub marker. The lifecycle
    // review carries its findings as strings, so the marker is matched rather
    // than the whole sentence around it.
    let is_stub = result
        .warnings
        .iter()
        .any(|warning| warning.contains("unfilled template") || warning.contains("Body too short"));
    let has_must_errors = !result.must_findings.is_empty();
    let review_hints = forgeplan_core::hints::review_hints(
        ref_form,
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
                .with_action(format!("forgeplan validate {ref_form}")),
        ]
    } else if result.can_activate {
        vec![Hint::info("Review passed").with_action(format!("forgeplan activate {ref_form}"))]
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
