use forgeplan_core::graph::topological;
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

/// `forgeplan blocked [id] [--json]` — show blocked artifacts and their dependencies.
pub async fn run(id: Option<&str>, json: bool) -> anyhow::Result<()> {
    let (workspace, source, _read_guard) = common::open_read_source().await?;
    let all_relations = match &source {
        common::ReadSource::Legacy(store) => store.get_all_relations(&_read_guard).await?,
        common::ReadSource::Captured(read) => read.get_all_relations(),
    };
    let legacy_records;
    let all_records = match &source {
        common::ReadSource::Legacy(store) => {
            legacy_records = store.list_records(&_read_guard, None).await?;
            legacy_records.as_slice()
        }
        common::ReadSource::Captured(read) => read.records(),
    };
    let resolved_ids = all_records.iter()
        .filter(|r| matches!(r.status.as_str(), "active" | "deprecated" | "superseded"))
        .map(|r| r.canonical_key.clone()).collect();
    let display_by_key: std::collections::HashMap<&str, &str> = all_records.iter()
        .map(|r| (r.canonical_key.as_str(), r.id.as_str())).collect();
    let display = |key: &str| display_by_key.get(key).copied().unwrap_or(key).to_string();
    let display_list = |keys: &[String]| keys.iter().map(|key| display(key)).collect::<Vec<_>>();

    if let Some(selector) = id {
        let (artifact_id, _) = common::resolve_required_identity(&source, &workspace, selector).await?;
        let blocked_by = topological::get_blocked_by(&artifact_id, &all_relations, &resolved_ids);

        // Compose hints: surface the activate-the-draft suggestion from
        // blocked_hints when there are blockers. When the artifact has no
        // blockers, the dependency-graph state is fully resolved — emit
        // `Done.` per PRD-071 CONDITIONALITY (terminal: nothing to unblock).
        let mut blocker_pairs = Vec::new();
        for dep in &blocked_by {
            let status = all_records
                .iter()
                .find(|r| r.canonical_key == *dep)
                .map(|r| r.status.as_str())
                .unwrap_or("unknown");
            blocker_pairs.push((dep.clone(), status.to_string()));
        }
        let hint_list: Vec<Hint> = forgeplan_core::hints::blocked_hints(&blocker_pairs);

        if json {
            let data = serde_json::json!({
                "id": display(&artifact_id),
                "canonical_key": artifact_id,
                "blocked": !blocked_by.is_empty(),
                "blocked_by": display_list(&blocked_by),
                "canonical_blocked_by": blocked_by,
                "_next_action": hints::primary_action(&hint_list),
                "hints": hint_list,
            });
            println!("{}", serde_json::to_string_pretty(&data)?);
            return Ok(());
        }
        if blocked_by.is_empty() {
            println!("  {} is NOT blocked (all dependencies met)", display(&artifact_id));
            println!();
            println!("Done.");
            return Ok(());
        } else {
            println!("  {} is BLOCKED by:", display(&artifact_id));
            for (dep, status) in &blocker_pairs {
                println!("    -> {} ({})", display(dep), status);
            }
            if !hint_list.is_empty() {
                print!("{}", forgeplan_core::hints::format_hints(&hint_list));
            }
        }
        print!("{}", hints::render_next_action_line(&hint_list));
    } else {
        let result = topological::kahn_sort(&all_relations, &resolved_ids);

        // Pick a hint: surface the first blocked artifact's first blocker,
        // or push the user toward `forgeplan order` when nothing is blocked.
        let mut hint_list: Vec<Hint> = Vec::new();
        if let Some((blocked_id, blockers)) = result.blocked.first() {
            if let Some(first_blocker) = blockers.first() {
                hint_list.push(
                    Hint::warning(format!("{} is blocked by {}", blocked_id, first_blocker))
                        .with_action(format!("forgeplan blocked {}", blocked_id)),
                );
            }
        } else if let Some(first_ready) = result.ready.first() {
            hint_list.push(
                Hint::info("Pick the next ready artifact")
                    .with_action(format!("forgeplan score {}", first_ready)),
            );
        }

        if json {
            let data = serde_json::json!({
                "cycles": result.cycles.iter().map(|cycle| display_list(cycle)).collect::<Vec<_>>(),
                "canonical_cycles": result.cycles,
                "blocked": result.blocked.iter().map(|(id, deps)| serde_json::json!({"id": display(id), "canonical_key": id, "blocked_by": display_list(deps), "canonical_blocked_by": deps})).collect::<Vec<_>>(),
                "ready": display_list(&result.ready),
                "canonical_ready": result.ready,
                "ready_count": result.ready.len(),
                "blocked_count": result.blocked.len(),
                "_next_action": hints::primary_action(&hint_list),
                "hints": hint_list,
            });
            println!("{}", serde_json::to_string_pretty(&data)?);
            return Ok(());
        }

        if !result.cycles.is_empty() {
            println!("  Warning: Cycles detected:");
            for cycle in &result.cycles {
                println!("    {}", display_list(cycle).join(" -> "));
            }
            println!();
        }

        if result.blocked.is_empty() {
            println!("  No blocked artifacts. All dependencies met.");
        } else {
            println!("  Blocked artifacts:");
            for (id, blockers) in &result.blocked {
                println!("    {} <- blocked by: {}", display(id), display_list(blockers).join(", "));
            }
        }

        println!();
        println!("  Ready to work: {}", result.ready.len());
        println!("  Blocked: {}", result.blocked.len());

        // PRD-071 CONDITIONALITY: when there are no blockers and no actionable
        // hint, the workspace is in a terminal "all clear" state. Emit `Done.`
        // rather than fall silent. The `Or:` rendering keeps `forgeplan order`
        // discoverable as a follow-up if there are still ready artifacts.
        if hint_list.is_empty() {
            println!();
            println!("Done.");
        } else {
            print!("{}", hints::render_next_action_line(&hint_list));
        }
    }

    Ok(())
}
