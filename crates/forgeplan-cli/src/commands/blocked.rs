use std::collections::HashSet;
use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::graph::topological;
use forgeplan_core::workspace;

/// `forgeplan blocked [id] [--json]` — show blocked artifacts and their dependencies.
pub async fn run(id: Option<&str>, json: bool) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let all_relations = store.get_all_relations().await?;

    let all_records = store.list_records(None).await?;
    let active_ids: HashSet<String> = all_records
        .iter()
        .filter(|r| r.status == "active")
        .map(|r| r.id.clone())
        .collect();

    if let Some(artifact_id) = id {
        let blocked_by = topological::get_blocked_by(artifact_id, &all_relations, &active_ids);
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
            for dep in &blocked_by {
                let status = if active_ids.contains(dep) { "active" } else { "draft" };
                println!("    -> {} ({})", dep, status);
            }
        }
    } else {
        let result = topological::kahn_sort(&all_relations, &active_ids);

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
