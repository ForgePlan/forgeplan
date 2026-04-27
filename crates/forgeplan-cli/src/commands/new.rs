use std::collections::HashMap;

use anyhow::{Context, Result};

use forgeplan_core::artifact::store::ArtifactSummary;
use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::{ArtifactFilter, NewArtifact};
use forgeplan_core::duplicate::{DUPLICATE_SIMILARITY_THRESHOLD, title_similarity};
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::projection;
use forgeplan_core::template::{get_embedded_template, render_template};

use crate::commands::common;

/// Maximum allowed title length in characters.
///
/// Chosen as a safe upper bound for filesystem path limits across platforms
/// (macOS/Linux filenames cap at 255 bytes; we leave headroom for slug
/// prefix/suffix, extension, and multi-byte characters).
pub const MAX_TITLE_LEN: usize = 128;

/// Validate an artifact title before any DB or filesystem writes.
///
/// Rejects empty titles (including whitespace-only) and titles longer than
/// [`MAX_TITLE_LEN`] characters. Called at the very start of `run` so that
/// invalid input never produces orphan DB rows.
pub fn validate_title(title: &str) -> Result<()> {
    if title.trim().is_empty() {
        anyhow::bail!("Title cannot be empty. Provide a non-empty title.");
    }
    let len = title.chars().count();
    if len > MAX_TITLE_LEN {
        anyhow::bail!(
            "Title too long (got {} chars, max {}). Shorten the title.",
            len,
            MAX_TITLE_LEN
        );
    }
    Ok(())
}

pub async fn run(kind_str: &str, title: &str, allow_duplicate: bool) -> Result<()> {
    // Validate title BEFORE any DB insert or filesystem write to prevent
    // orphan rows on invalid input (see final-e2e audit for release v0.17.0).
    validate_title(title)?;

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
        // C1: new artifacts start untagged; users add tags later via `forgeplan tag`.
        // TODO: add `--tag key=value` flag in future sprint.
        tags: Vec::new(),
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

    // PRD-071 contract: emit Next: line with full command pointing at the new id.
    // Rationale advisory printed first; Next line always validate.
    let rationale = match template_key {
        "prd" => Some("fill Problem, Goals, Non-Goals, Target Users, FR sections, then validate"),
        "rfc" => {
            Some("fill Summary, Motivation, Goals, Options, Implementation Phases, then validate")
        }
        "adr" => Some("fill Context, Decision, Consequences, then validate"),
        "evidence" => {
            Some("fill Structured Fields (verdict, congruence_level, evidence_type), then validate")
        }
        "epic" => Some("fill Vision, Children PRDs, Progress, then validate"),
        _ => None,
    };
    if let Some(r) = rationale {
        println!("\n  {}", r);
    }
    let hints_vec = vec![
        Hint::suggestion(format!("Validate {} after filling MUST sections", id))
            .with_action(format!("forgeplan validate {}", id)),
    ];
    print!("{}", hints::render_next_action_line(&hints_vec));

    // Session: advance to Shaping for decision artifacts only.
    //
    // PROB-033 fix: `new evidence` is phase-agnostic — creating evidence is a
    // legitimate operation in ANY session state (backfill from shipped code,
    // audit findings, brownfield import, external benchmark). The state machine
    // guardrail still applies at `activate` time (stub detection + validation),
    // so loosening `new` is safe.
    //
    // Notes/problems/evidence never drive the state machine; they're orthogonal
    // to the decision pipeline.
    let phase = if matches!(template_key, "prd" | "rfc" | "adr" | "epic" | "spec") {
        forgeplan_core::session::Phase::Shaping
    } else {
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
    fn validate_title_accepts_normal() {
        assert!(validate_title("Auth System").is_ok());
    }

    #[test]
    fn validate_title_rejects_empty() {
        let err = validate_title("").unwrap_err().to_string();
        assert!(err.contains("cannot be empty"), "got: {}", err);
    }

    #[test]
    fn validate_title_rejects_whitespace_only() {
        let err = validate_title("   \t\n  ").unwrap_err().to_string();
        assert!(err.contains("cannot be empty"), "got: {}", err);
    }

    #[test]
    fn validate_title_accepts_exactly_max() {
        let t: String = "x".repeat(MAX_TITLE_LEN);
        assert!(validate_title(&t).is_ok());
    }

    #[test]
    fn validate_title_rejects_over_max_by_one() {
        let t: String = "x".repeat(MAX_TITLE_LEN + 1);
        let err = validate_title(&t).unwrap_err().to_string();
        assert!(err.contains("too long"), "got: {}", err);
    }

    #[test]
    fn validate_title_rejects_very_long() {
        let t: String = "X".repeat(500);
        let err = validate_title(&t).unwrap_err().to_string();
        assert!(err.contains("too long"), "got: {}", err);
        assert!(err.contains("500"), "got: {}", err);
    }

    #[test]
    fn validate_title_counts_chars_not_bytes() {
        // 128 multi-byte chars is valid (char count, not byte count)
        let t: String = "й".repeat(MAX_TITLE_LEN);
        assert!(validate_title(&t).is_ok());
        let t2: String = "й".repeat(MAX_TITLE_LEN + 1);
        assert!(validate_title(&t2).is_err());
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
