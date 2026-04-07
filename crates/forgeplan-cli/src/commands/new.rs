use std::collections::HashMap;

use anyhow::{Context, Result};

use forgeplan_core::artifact::store::ArtifactSummary;
use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::{ArtifactFilter, NewArtifact};
use forgeplan_core::duplicate::{DUPLICATE_SIMILARITY_THRESHOLD, title_similarity};
use forgeplan_core::projection;
use forgeplan_core::template::{get_embedded_template, render_template};

use crate::commands::common;

pub async fn run(kind_str: &str, title: &str, allow_duplicate: bool) -> Result<()> {
    let kind: ArtifactKind = kind_str.parse().map_err(|e| anyhow::anyhow!("{}", e))?;

    let (workspace, store) = common::open_store().await?;

    // Duplicate guard (FR-001 of PRD-043): warn before creating an artifact
    // whose title closely matches an existing one of the same kind.
    let template_key_for_filter = kind.template_key().to_string();
    let filter = ArtifactFilter {
        kind: Some(template_key_for_filter.clone()),
        status: None,
    };
    let existing = store.list_artifacts(Some(&filter)).await?;
    if let Some((dup_id, dup_title, dup_score)) = find_duplicate(&existing, title) {
        if allow_duplicate {
            eprintln!(
                "warning: similar artifact exists: {} \"{}\" (similarity {:.0}%) — continuing due to --allow-duplicate",
                dup_id,
                dup_title,
                dup_score * 100.0
            );
        } else {
            let proceed = cliclack::confirm(format!(
                "Found similar artifact: {} \"{}\" (similarity {:.0}%)\nContinue creating new artifact?",
                dup_id,
                dup_title,
                dup_score * 100.0
            ))
            .initial_value(false)
            .interact()
            .unwrap_or(false);
            if !proceed {
                println!("Cancelled");
                return Ok(());
            }
        }
    }

    // Get next sequential ID from LanceDB
    let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
    let id = store.next_id(&prefix).await?;

    // The kind string used for template lookup
    let template_key = kind.template_key();
    let template = get_embedded_template(template_key)
        .ok_or_else(|| anyhow::anyhow!("No template found for kind '{}'", template_key))?;

    // Build template variables
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let nnn = id.split('-').next_back().unwrap_or("001").to_string();

    let mut vars = HashMap::new();
    vars.insert("NNN".to_string(), nnn.clone());
    vars.insert("title".to_string(), title.to_string());
    vars.insert("Title".to_string(), title.to_string());

    // Render the template with variable substitution
    let mut rendered = render_template(template, &vars);

    // Replace date placeholders
    rendered = rendered.replace("YYYY-MM-DD", &today);

    // Replace full ID patterns like PRD-{NNN} that may remain after render
    let heading_pattern = format!("# {}-{}: ", prefix, nnn);
    if let Some(pos) = rendered.find(&heading_pattern) {
        let line_start = pos + heading_pattern.len();
        if let Some(nl) = rendered[line_start..].find('\n') {
            let old_heading_text = &rendered[line_start..line_start + nl];
            if old_heading_text.contains('{') || old_heading_text.contains('/') {
                let before = &rendered[..line_start];
                let after = &rendered[line_start + nl..];
                rendered = format!("{}{}{}", before, title, after);
            }
        }
    }

    // Lightweight kinds default to tactical depth; structured kinds default to standard
    let depth = match kind {
        ArtifactKind::Note
        | ArtifactKind::EvidencePack
        | ArtifactKind::ProblemCard
        | ArtifactKind::SolutionPortfolio
        | ArtifactKind::RefreshReport => "tactical",
        _ => "standard",
    };

    // Write to LanceDB (source of truth)
    let artifact = NewArtifact {
        id: id.clone(),
        kind: template_key.to_string(),
        status: "draft".to_string(),
        title: title.to_string(),
        body: rendered.clone(),
        depth: depth.to_string(),
        author: None,
        parent_epic: None,
        valid_until: None,
    };
    store
        .create_artifact(&artifact)
        .await
        .with_context(|| format!("Failed to create artifact {} in LanceDB", id))?;

    // Render markdown projection (git-tracked)
    let filepath = projection::render_projection(
        &workspace,
        &id,
        template_key,
        title,
        "draft",
        depth,
        None,
        None,
        None,
        &rendered,
        &[],
    )
    .await
    .with_context(|| format!("Failed to write projection for {}", id))?;

    // Log creation in change_log
    common::log_change(&store, &id, "create", "cli").await;

    println!("  Created: {}", filepath.display());
    println!("  ID:      {}", id);
    println!("  Kind:    {}", template_key);
    println!("  Title:   {}", title);

    // Next-step hint based on kind
    let hint = match template_key {
        "prd" => Some(
            "Next: fill Problem, Goals, Non-Goals, Target Users, FR sections → forgeplan validate",
        ),
        "rfc" => Some(
            "Next: fill Summary, Motivation, Goals, Options, Implementation Phases → forgeplan validate",
        ),
        "adr" => Some("Next: fill Context, Decision, Consequences → forgeplan validate"),
        "evidence" => Some(
            "Next: fill Structured Fields (verdict, congruence_level, evidence_type) → forgeplan activate",
        ),
        "epic" => Some("Next: fill Vision, Children PRDs, Progress → forgeplan validate"),
        _ => None,
    };
    if let Some(h) = hint {
        println!("\n  * {}", h);
    }

    // Session: advance to Shaping (for decision artifacts) or Evidence (for evidence kind)
    let phase = if template_key == "evidence" {
        forgeplan_core::session::Phase::Evidence
    } else if matches!(template_key, "prd" | "rfc" | "adr" | "epic" | "spec") {
        forgeplan_core::session::Phase::Shaping
    } else {
        // Notes, problems, etc. — don't change phase
        return Ok(());
    };
    common::advance_session(phase, Some(&id));

    Ok(())
}

/// Find the closest duplicate among `existing` for the given title.
///
/// Returns `Some((id, title, similarity))` when the best match has similarity
/// at or above [`DUPLICATE_SIMILARITY_THRESHOLD`] (canonical Jaccard).
fn find_duplicate(existing: &[ArtifactSummary], title: &str) -> Option<(String, String, f64)> {
    let mut best: Option<(String, String, f64)> = None;
    for s in existing {
        let score = title_similarity(&s.title, title);
        if score >= DUPLICATE_SIMILARITY_THRESHOLD
            && best.as_ref().is_none_or(|(_, _, b)| score > *b)
        {
            best = Some((s.id.clone(), s.title.clone(), score));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str, title: &str) -> ArtifactSummary {
        ArtifactSummary {
            id: id.to_string(),
            title: title.to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
        }
    }

    #[test]
    fn find_duplicate_exact_title_match() {
        let existing = vec![rec("PRD-001", "Auth System")];
        let dup = find_duplicate(&existing, "Auth System");
        assert!(dup.is_some());
        let (id, _, score) = dup.unwrap();
        assert_eq!(id, "PRD-001");
        assert!(score >= 1.0);
    }

    #[test]
    fn find_duplicate_no_match_returns_none() {
        let existing = vec![rec("PRD-001", "Billing Pipeline")];
        assert!(find_duplicate(&existing, "Auth System").is_none());
    }

    #[test]
    fn find_duplicate_substring_below_threshold() {
        // 0.8 is NOT strictly > 0.8 → no dup reported
        let existing = vec![rec("PRD-001", "Auth System Design")];
        assert!(find_duplicate(&existing, "auth system").is_none());
    }

    #[test]
    fn find_duplicate_picks_exact_over_substring() {
        let existing = vec![
            rec("PRD-001", "Unrelated Topic"),
            rec("PRD-002", "Auth System Design"),
            rec("PRD-003", "Auth System"),
        ];
        let (id, _, score) = find_duplicate(&existing, "Auth System").unwrap();
        assert_eq!(id, "PRD-003");
        assert!((score - 1.0).abs() < 1e-9);
    }
}
