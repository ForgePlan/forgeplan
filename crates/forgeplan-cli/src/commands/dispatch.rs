//! `forgeplan dispatch` — compute a parallel-safe work plan for N
//! sub-agents (PRD-057 Inc 4 + PRD-070 CLI parity).
//!
//! Mirrors `forgeplan_dispatch` MCP tool: pure read, without workspace mutation.
//! Hydrates candidate frontmatter (`affected_files`, `domain`,
//! `parent_epic`) from the legacy projection or the confirmed source capture,
//! drops candidates blocked by structural dependencies, skips already-claimed
//! artifacts, and returns N buckets plus a serial queue of leftover work.

use std::collections::{HashMap, HashSet};

use forgeplan_core::artifact::types::{ArtifactKind, slugify};
use forgeplan_core::claim::ClaimStore;
use forgeplan_core::dispatch::{
    self, ArtifactCandidate, DEFAULT_OVERLAP_THRESHOLD, MAX_AFFECTED_FILE_LEN, MAX_AFFECTED_FILES,
    MAX_AGENTS, MAX_SKILLS_PER_AGENT,
};
use forgeplan_core::graph::topological;
use forgeplan_core::hints::{self, Hint};

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

    let (ws, source, _read_guard) = common::open_read_source().await?;
    let legacy_records;
    let records = match &source {
        common::ReadSource::Legacy(store) => {
            legacy_records = store.list_records(&_read_guard, None).await?;
            legacy_records.as_slice()
        }
        common::ReadSource::Captured(read) => read.records(),
    };
    let epic_key = match epic {
        Some(selector) => Some(common::resolve_required_identity(&source, &ws, selector).await?.0),
        None => None,
    };

    // Default status filter is `draft` — the dispatch-relevant set.
    // `--status any` opts into the full list.
    let status_filter = status.unwrap_or("draft");
    let summaries = records.iter().filter(|record| {
        kind.is_none_or(|kind| record.kind == kind)
            && (status_filter == "any" || record.status == status_filter)
    });

    // Compute blocked set so dependency-blocked artifacts never land in a
    // parallel bucket (FR-003).
    let relations = match &source {
        common::ReadSource::Legacy(store) => store.get_all_relations(&_read_guard).await?,
        common::ReadSource::Captured(read) => read.get_all_relations(),
    };
    let resolved_ids: HashSet<String> = records
        .iter()
        .filter(|record| {
            record.status == "active" || record.status == "deprecated" || record.status == "superseded"
        })
        .map(|record| record.canonical_key.clone())
        .collect();
    let topo = topological::kahn_sort(&relations, &resolved_ids);
    let blocked_ids: HashSet<String> = topo.blocked.iter().map(|(id, _)| id.clone()).collect();

    let mut candidates = Vec::new();
    let mut skipped_parse_errors = 0usize;
    let mut skipped_blocked = Vec::<String>::new();
    for summary in summaries {
        let fields = match &source {
            common::ReadSource::Legacy(_) => {
                read_dispatch_fm_fields(
                    &ws,
                    &summary.kind,
                    &summary.id,
                    &summary.title,
                    &summary.canonical_key,
                )
                .await
            }
            common::ReadSource::Captured(_) => parse_dispatch_fm_fields(&summary.body),
        };
        if fields.parse_failed {
            skipped_parse_errors += 1;
            continue;
        }
        if let Some(wanted) = epic_key.as_deref() {
            let Some(parent) = fields.parent_epic.as_deref() else {
                continue;
            };
            let (parent_key, _) = common::resolve_required_identity(&source, &ws, parent).await?;
            if parent_key != wanted {
                continue;
            }
        }
        if blocked_ids.contains(&summary.canonical_key) {
            skipped_blocked.push(summary.canonical_key.clone());
            continue;
        }
        candidates.push(ArtifactCandidate {
            id: summary.canonical_key.clone(),
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

    // PRD-077 FR-010 + CR-H4: enrich serial entries with
    // `blocked by dependencies: <PARENT>[, <PARENT>...]` when applicable.
    // The bare structural-dep skip ran before `compute_dispatch_plan`, so
    // those entries are already in `skipped_blocked` (and the `reasoning`
    // log) — push them into the structured serial queue with a precise
    // dependency-source reason so downstream agents can fix the root
    // cause without re-running `forgeplan_blocked`.
    //
    // Helper extracted to `forgeplan_core::dispatch::build_blocker_reasons_from_slice`
    // so CLI and MCP surfaces can't drift (CR-H4 closure mandate).
    // Pre-CR-H4 bug: the inline HashMap collapsed multi-parent edges to
    // a single arbitrary parent AND counted non-`depends_on`/`blocks`
    // relations (e.g. `informs`) as structural blockers. Both fixed by
    // the helper.
    plan.serial_queue
        .extend(forgeplan_core::dispatch::build_blocker_reasons_from_slice(
            &relations,
            &skipped_blocked,
        ));
    let display_by_key: HashMap<&str, &str> = records
        .iter()
        .map(|record| (record.canonical_key.as_str(), record.id.as_str()))
        .collect();
    let mut alias_counts: HashMap<&str, usize> = HashMap::new();
    for record in records {
        *alias_counts.entry(record.id.as_str()).or_default() += 1;
    }
    // Both closures own their strings: `display_by_key` and the caller's key
    // have different lifetimes, so a `&str` borrowed from either cannot be
    // returned by one closure.
    let display_id = |key: &str| -> String {
        display_by_key
            .get(key)
            .map(|display| (*display).to_string())
            .unwrap_or_else(|| key.to_string())
    };
    let display_label = |key: &str| {
        let display = display_id(key);
        if alias_counts.get(display.as_str()).copied().unwrap_or(0) > 1 {
            format!("{display} [{key}]")
        } else {
            display
        }
    };

    // Hint: pick the first non-empty bucket's first artifact to show
    // exactly what the agent should claim next; if every bucket is empty
    // (no candidates), nudge toward listing draft artifacts to seed work.
    let first_bucket_id = plan.buckets.iter().find_map(|b| b.first().cloned());
    let mut hint_list: Vec<Hint> = Vec::new();
    if let Some(first_id) = first_bucket_id.as_ref() {
        // Concrete claim command — agent identity uses the CLI default,
        // matching the run-time fallback in `claim::run`.
        hint_list.push(
            Hint::info(format!(
                "Claim first parallel-safe artifact {}",
                display_label(first_id)
            ))
            .with_action(format!("forgeplan claim {} --agent agent-0", first_id)),
        );
    } else {
        hint_list.push(
            Hint::info("No parallel-safe candidates — list drafts")
                .with_action("forgeplan list --status draft".to_string()),
        );
    }

    if json {
        let presentation_buckets: Vec<Vec<String>> = plan
            .buckets
            .iter()
            .map(|bucket| bucket.iter().map(|key| display_id(key).to_string()).collect())
            .collect();
        let presentation_serial: Vec<_> = plan
            .serial_queue
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "id": display_id(&entry.id),
                    "canonical_key": entry.id,
                    "reason": entry.reason,
                })
            })
            .collect();
        let body = serde_json::json!({
            "buckets": presentation_buckets,
            "canonical_buckets": plan.buckets,
            "serial_queue": presentation_serial,
            "reasoning": plan.reasoning,
            "generated_at": plan.generated_at,
            "agent_count": plan.agent_count,
            "overlap_threshold": plan.overlap_threshold,
            "candidate_count": candidate_count,
            "claimed_count": claimed_count,
            "skipped_parse_errors": skipped_parse_errors,
            "blocked_count": skipped_blocked.len(),
            "_next_action": hints::primary_action(&hint_list),
            "hints": hint_list,
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
            let labels: Vec<_> = bucket.iter().map(|key| display_label(key)).collect();
            println!("  agent {i}: {}", labels.join(", "));
        }
    }

    if !plan.serial_queue.is_empty() {
        println!();
        println!(
            "Serial queue ({} item(s) — re-dispatch when an agent frees):",
            plan.serial_queue.len()
        );
        // PRD-077 FR-010: print the per-entry reason indented under the ID
        // so agents see the cause of deferral inline — no need to cross-
        // reference `reasoning[]`.
        for entry in &plan.serial_queue {
            println!("  {}", display_label(&entry.id));
            println!("    reason: {}", entry.reason);
        }
    }

    if !plan.reasoning.is_empty() {
        println!();
        println!("Reasoning (top {}):", plan.reasoning.len().min(10));
        for line in plan.reasoning.iter().take(10) {
            println!("  - {line}");
        }
    }

    print!("{}", hints::render_next_action_line(&hint_list));

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
    canonical_key: &str,
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
    // A document minted under a key is filed as `<canonical_key>.md`; one
    // that predates the key keeps its historical `{id}-{slug}.md`. The file
    // on disk decides which one it is.
    let keyed = dir.join(format!("{canonical_key}.md"));
    let historical = dir.join(format!("{}-{}.md", id, slugify(title)));
    let content = match tokio::fs::read_to_string(&keyed).await {
        Ok(s) => s,
        Err(_) => match tokio::fs::read_to_string(&historical).await {
            Ok(s) => s,
            Err(_) => {
                return DispatchFmFields {
                    parse_failed: true,
                    ..Default::default()
                };
            }
        },
    };
    parse_dispatch_fm_fields(&content)
}

/// Extract dispatch metadata from the same canonical bytes the read source saw.
fn parse_dispatch_fm_fields(content: &str) -> DispatchFmFields {
    let (fm, body) = match forgeplan_core::artifact::frontmatter::parse_frontmatter(content) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_metadata_preserves_frontmatter_files_domain_and_epic() {
        let fields = parse_dispatch_fm_fields(
            "---\naffected_files: src/alpha.rs, src/beta.rs\n\
             domain: BackEnd\nparent_epic: EPIC-001\n---\n\n\
             ## Affected Files\n- `src/not-selected.rs`\n",
        );
        assert!(!fields.parse_failed);
        assert_eq!(fields.files, ["src/alpha.rs", "src/beta.rs"]);
        assert_eq!(fields.domain.as_deref(), Some("backend"));
        assert_eq!(fields.parent_epic.as_deref(), Some("EPIC-001"));
    }

    #[test]
    fn dispatch_metadata_uses_affected_files_section_when_frontmatter_is_empty() {
        let fields = parse_dispatch_fm_fields(
            "---\naffected_files: []\ndomain: invalid/domain\n---\n\n\
             ## Affected Files\n- `src/fallback.rs`\n",
        );
        assert!(!fields.parse_failed);
        assert_eq!(fields.files, ["src/fallback.rs"]);
        assert_eq!(fields.domain, None);
        assert_eq!(fields.parent_epic, None);
    }

    #[test]
    fn unreadable_dispatch_frontmatter_is_not_an_empty_candidate() {
        for content in ["no frontmatter", "---\naffected_files: [\n---\n"] {
            assert!(parse_dispatch_fm_fields(content).parse_failed);
        }
    }
}
