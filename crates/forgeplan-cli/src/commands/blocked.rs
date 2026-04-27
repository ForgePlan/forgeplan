use forgeplan_core::graph::topological;
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

/// `forgeplan blocked [id] [--json]` — show blocked artifacts and their dependencies.
pub async fn run(id: Option<&str>, json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let all_relations = store.get_all_relations().await?;

    let all_records = store.list_records(None).await?;
    let resolved_ids = common::resolved_ids(&all_records);

    if let Some(artifact_id) = id {
        let blocked_by = topological::get_blocked_by(artifact_id, &all_relations, &resolved_ids);

        // Compose hints: surface the activate-the-draft suggestion from
        // blocked_hints when there are blockers. When the artifact has no
        // blockers, the dependency-graph state is fully resolved — emit
        // `Done.` per PRD-071 CONDITIONALITY (terminal: nothing to unblock).
        let mut blocker_pairs = Vec::new();
        for dep in &blocked_by {
            let status = all_records
                .iter()
                .find(|r| r.id.eq_ignore_ascii_case(dep))
                .map(|r| r.status.as_str())
                .unwrap_or("unknown");
            blocker_pairs.push((dep.clone(), status.to_string()));
        }
        let hint_list: Vec<Hint> = forgeplan_core::hints::blocked_hints(&blocker_pairs);

        if json {
            let data = serde_json::json!({
                "id": artifact_id,
                "blocked": !blocked_by.is_empty(),
                "blocked_by": blocked_by,
                "_next_action": hints::primary_action(&hint_list),
                "hints": hint_list,
            });
            println!("{}", serde_json::to_string_pretty(&data)?);
            return Ok(());
        }
        if blocked_by.is_empty() {
            println!("  {} is NOT blocked (all dependencies met)", artifact_id);
            println!();
            println!("Done.");
            return Ok(());
        } else {
            println!("  {} is BLOCKED by:", artifact_id);
            for (dep, status) in &blocker_pairs {
                println!("    -> {} ({})", dep, status);
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
                "cycles": result.cycles,
                "blocked": result.blocked.iter().map(|(id, deps)| serde_json::json!({"id": id, "blocked_by": deps})).collect::<Vec<_>>(),
                "ready": result.ready,
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
                println!("    {}", cycle.join(" -> "));
            }
            println!();
        }

        if result.blocked.is_empty() {
            println!("  No blocked artifacts. All dependencies met.");
        } else {
            println!("  Blocked artifacts:");
            for (id, blockers) in &result.blocked {
                println!("    {} <- blocked by: {}", id, blockers.join(", "));
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
