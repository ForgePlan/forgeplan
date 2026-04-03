use forgeplan_core::graph::topological;

use crate::commands::common;

/// `forgeplan blocked [id] [--json]` — show blocked artifacts and their dependencies.
pub async fn run(id: Option<&str>, json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let all_relations = store.get_all_relations().await?;

    let all_records = store.list_records(None).await?;
    let resolved_ids = common::resolved_ids(&all_records);

    if let Some(artifact_id) = id {
        let blocked_by = topological::get_blocked_by(artifact_id, &all_relations, &resolved_ids);
        if json {
            let data = serde_json::json!({
                "id": artifact_id,
                "blocked": !blocked_by.is_empty(),
                "blocked_by": blocked_by,
            });
            println!("{}", serde_json::to_string_pretty(&data)?);
            return Ok(());
        }
        if blocked_by.is_empty() {
            println!("  {} is NOT blocked (all dependencies met)", artifact_id);
        } else {
            println!("  {} is BLOCKED by:", artifact_id);
            let mut blocker_pairs = Vec::new();
            for dep in &blocked_by {
                let status = all_records.iter()
                    .find(|r| r.id.eq_ignore_ascii_case(dep))
                    .map(|r| r.status.as_str())
                    .unwrap_or("unknown");
                println!("    -> {} ({})", dep, status);
                blocker_pairs.push((dep.clone(), status.to_string()));
            }
            let hints = forgeplan_core::hints::blocked_hints(&blocker_pairs);
            if !hints.is_empty() {
                print!("{}", forgeplan_core::hints::format_hints(&hints));
            }
        }
    } else {
        let result = topological::kahn_sort(&all_relations, &resolved_ids);

        if json {
            let data = serde_json::json!({
                "cycles": result.cycles,
                "blocked": result.blocked.iter().map(|(id, deps)| serde_json::json!({"id": id, "blocked_by": deps})).collect::<Vec<_>>(),
                "ready": result.ready,
                "ready_count": result.ready.len(),
                "blocked_count": result.blocked.len(),
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
    }

    Ok(())
}
