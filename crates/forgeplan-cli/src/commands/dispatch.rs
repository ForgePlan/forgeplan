//! `forgeplan dispatch` — compute a parallel-safe work plan for N
//! sub-agents (PRD-057 Inc 4 + PRD-070 CLI parity).
//!
//! Mirrors `forgeplan_dispatch` MCP tool: pure read — does not mutate the
//! workspace. Hydrates candidate frontmatter (`affected_files`, `domain`,
//! `parent_epic`) from disk, drops candidates blocked by structural
//! dependencies, skips already-claimed artifacts, and returns N buckets
//! plus a serial queue of leftover work.

use std::collections::HashSet;

use forgeplan_core::artifact::types::{ArtifactKind, slugify};
use forgeplan_core::claim::ClaimStore;
use forgeplan_core::db::store::ArtifactFilter;
use forgeplan_core::dispatch::{
    self, ArtifactCandidate, DEFAULT_OVERLAP_THRESHOLD, MAX_AFFECTED_FILE_LEN, MAX_AFFECTED_FILES,
    MAX_AGENTS, MAX_SKILLS_PER_AGENT,
};
use forgeplan_core::graph::topological;
use forgeplan_core::workspace;

use crate::commands::common;

pub async fn run(
    agents: u32,
    epic: Option<&str>,
    kind: Option<&str>,
    status: Option<&str>,
    overlap_threshold: Option<f64>,
    json: bool,
) -> anyhow::Result<()> {
    // Validate / clamp inputs at the boundary (matches the MCP tool —
    // CWE-770 defense even on the CLI path).
    if agents == 0 {
        anyhow::bail!("--agents must be >= 1");
    }
    let agents = agents as usize;
    if agents > MAX_AGENTS {
        anyhow::bail!(
            "--agents must be <= {MAX_AGENTS} — PRD-057 targets 2-5 concurrent sub-agents"
        );
    }

    let threshold = overlap_threshold.unwrap_or(DEFAULT_OVERLAP_THRESHOLD);
    if !(0.0..=1.0).contains(&threshold) {
        anyhow::bail!("--overlap-threshold must be in [0.0, 1.0]");
    }

    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    let store = common::store().await?;

    // Default status filter is `draft` — the dispatch-relevant set.
    // `--status any` opts into the full list.
    let status_filter = status.unwrap_or("draft");
    let filter = ArtifactFilter {
        kind: kind.map(|s| s.to_string()),
        status: if status_filter == "any" {
            None
        } else {
            Some(status_filter.to_string())
        },
    };
    let summaries = store.list_artifacts(Some(&filter)).await?;

    // Compute blocked set so dependency-blocked artifacts never land in a
    // parallel bucket (FR-003).
    let relations = store.get_all_relations().await?;
    let records = store.list_records(None).await?;
    let resolved_ids: HashSet<String> = records
        .iter()
        .filter(|r| r.status == "active" || r.status == "deprecated" || r.status == "superseded")
        .map(|r| r.id.clone())
        .collect();
    let topo = topological::kahn_sort(&relations, &resolved_ids);
    let blocked_ids: HashSet<String> = topo.blocked.iter().map(|(id, _)| id.clone()).collect();

    let mut candidates = Vec::with_capacity(summaries.len());
    let mut skipped_parse_errors = 0usize;
    let mut skipped_blocked = Vec::<String>::new();
    for summary in &summaries {
        let fields = read_dispatch_fm_fields(&ws, &summary.kind, &summary.id, &summary.title).await;
        if fields.parse_failed {
            skipped_parse_errors += 1;
            continue;
        }
        if let Some(wanted) = epic
            && fields.parent_epic.as_deref() != Some(wanted)
        {
            continue;
        }
        if blocked_ids.contains(&summary.id) {
            skipped_blocked.push(summary.id.clone());
            continue;
        }
        candidates.push(ArtifactCandidate {
            id: summary.id.clone(),
            affected_files: fields.files,
            domain: fields.domain,
        });
    }
    let candidate_count = candidates.len();

    let claim_store = ClaimStore::new(&ws);
    let claimed_map = claim_store
        .list_active_map()
        .await
        .map_err(|e| anyhow::anyhow!("list_active_map: {e}"))?;
    let claimed_count = claimed_map.len();
    let claimed_set: HashSet<String> = claimed_map.into_keys().collect();

    // CLI path doesn't expose `--agent-skills` for now (the MCP tool keeps
    // it for orchestrator-driven dispatch). Empty skill list = match
    // anything, matching the MCP default.
    let agent_skills: Vec<Vec<String>> = Vec::new();

    let mut plan = dispatch::compute_dispatch_plan(
        &candidates,
        agents,
        &agent_skills,
        &claimed_set,
        threshold,
    );
    for id in &skipped_blocked {
        plan.reasoning.insert(
            0,
            format!("{id}: skipped (blocked by unresolved structural dependency)"),
        );
    }

    if json {
        let body = serde_json::json!({
            "buckets": plan.buckets,
            "serial_queue": plan.serial_queue,
            "reasoning": plan.reasoning,
            "generated_at": plan.generated_at,
            "agent_count": plan.agent_count,
            "overlap_threshold": plan.overlap_threshold,
            "candidate_count": candidate_count,
            "claimed_count": claimed_count,
            "skipped_parse_errors": skipped_parse_errors,
            "blocked_count": skipped_blocked.len(),
        });
        println!("{}", serde_json::to_string_pretty(&body)?);
        return Ok(());
    }

    println!(
        "Dispatch plan ({} agent(s), threshold {:.2})",
        agents, threshold
    );
    println!("  Candidates:        {candidate_count}");
    println!("  Already claimed:   {claimed_count} skipped");
    println!("  Blocked by deps:   {} skipped", skipped_blocked.len());
    if skipped_parse_errors > 0 {
        println!("  Parse errors:      {skipped_parse_errors} skipped (see logs)");
    }
    println!();

    println!("Buckets (parallel — hand bucket[i] to agent i):");
    for (i, bucket) in plan.buckets.iter().enumerate() {
        if bucket.is_empty() {
            println!("  agent {i}: (idle)");
        } else {
            println!("  agent {i}: {}", bucket.join(", "));
        }
    }

    if !plan.serial_queue.is_empty() {
        println!();
        println!(
            "Serial queue ({} item(s) — re-dispatch when an agent frees):",
            plan.serial_queue.len()
        );
        for id in &plan.serial_queue {
            println!("  {id}");
        }
    }

    if !plan.reasoning.is_empty() {
        println!();
        println!("Reasoning (top {}):", plan.reasoning.len().min(10));
        for line in plan.reasoning.iter().take(10) {
            println!("  - {line}");
        }
    }

    Ok(())
}

#[derive(Debug, Default)]
struct DispatchFmFields {
    files: Vec<String>,
    domain: Option<String>,
    parent_epic: Option<String>,
    parse_failed: bool,
}

/// Read dispatcher-relevant frontmatter fields from an artifact's markdown
/// projection. Mirrors the MCP-side helper of the same name (kept private
/// there) so CLI dispatch produces identical decisions.
async fn read_dispatch_fm_fields(
    ws: &std::path::Path,
    kind: &str,
    id: &str,
    title: &str,
) -> DispatchFmFields {
    let artifact_kind = match kind.parse::<ArtifactKind>() {
        Ok(k) => k,
        Err(_) => return DispatchFmFields::default(),
    };
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return DispatchFmFields {
            parse_failed: true,
            ..Default::default()
        };
    }
    let dir = ws.join(artifact_kind.dir_name());
    let filename = format!("{}-{}.md", id, slugify(title));
    let path = dir.join(filename);
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(s) => s,
        Err(_) => {
            return DispatchFmFields {
                parse_failed: true,
                ..Default::default()
            };
        }
    };
    let (fm, body) = match forgeplan_core::artifact::frontmatter::parse_frontmatter(&content) {
        Ok((fm, body)) => (fm, body),
        Err(_) => {
            return DispatchFmFields {
                parse_failed: true,
                ..Default::default()
            };
        }
    };

    let mut files = fm
        .get("affected_files")
        .map(dispatch::parse_affected_files_from_fm)
        .unwrap_or_default();
    if files.is_empty() {
        let from_section = forgeplan_core::validation::checks::extract_affected_files(&body);
        files = from_section
            .into_iter()
            .filter(|s| s.len() <= MAX_AFFECTED_FILE_LEN)
            .take(MAX_AFFECTED_FILES)
            .collect();
    }

    let domain = fm
        .get("domain")
        .and_then(|v| v.as_str())
        .and_then(dispatch::normalize_dispatch_domain);
    let parent_epic = fm
        .get("parent_epic")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    DispatchFmFields {
        files,
        domain,
        parent_epic,
        parse_failed: false,
    }
}

// Silence unused-const warning when `MAX_SKILLS_PER_AGENT` becomes only
// referenced through future `--agent-skills` flags. Re-export of the
// invariant from the core crate keeps dead-code lint quiet.
const _ASSERT_MAX_SKILLS: usize = MAX_SKILLS_PER_AGENT;
