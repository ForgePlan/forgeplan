use std::collections::HashSet;
use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::graph::topological;
use forgeplan_core::workspace;

/// `forgeplan blocked [id]` — show blocked artifacts and their dependencies.
pub async fn run(id: Option<&str>) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let all_relations = store.get_all_relations().await?;

    // Get active artifact IDs
    let all_records = store.list_records(None).await?;
    let active_ids: HashSet<String> = all_records
        .iter()
        .filter(|r| r.status == "active")
        .map(|r| r.id.clone())
        .collect();

    if let Some(artifact_id) = id {
        // Show blocked status for specific artifact
        let blocked_by = topological::get_blocked_by(artifact_id, &all_relations, &active_ids);
        if blocked_by.is_empty() {
            println!("  {} is NOT blocked (all dependencies met)", artifact_id);
        } else {
            println!("  {} is BLOCKED by:", artifact_id);
            for dep in &blocked_by {
                let status = if active_ids.contains(dep) {
                    "active"
                } else {
                    "draft"
                };
                println!("    -> {} ({})", dep, status);
            }
        }
    } else {
        // Show all blocked artifacts
        let result = topological::kahn_sort(&all_relations, &active_ids);

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
