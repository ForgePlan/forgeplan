use std::collections::HashSet;

use forgeplan_core::graph::topological;

use crate::commands::common;

/// `forgeplan order` — show artifacts in topological order (dependency order).
pub async fn run(json: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let all_relations = store.get_all_relations().await?;

    let all_records = store.list_records(None).await?;
    // For display: which are active
    let active_ids: HashSet<String> = all_records
        .iter()
        .filter(|r| r.status == "active")
        .map(|r| r.id.clone())
        .collect();
    // For blocking logic: resolved = active + deprecated + superseded
    let resolved_ids = common::resolved_ids(&all_records);

    let result = topological::kahn_sort(&all_relations, &resolved_ids);

    if json {
        let data = serde_json::json!({
            "order": result.order,
            "cycles": result.cycles,
            "blocked": result.blocked.iter().map(|(id, deps)| serde_json::json!({"id": id, "blocked_by": deps})).collect::<Vec<_>>(),
            "ready": result.ready,
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

    if result.order.is_empty() {
        println!("  No linked artifacts found. Use `forgeplan link` to create dependencies.");
        return Ok(());
    }

    // Build lookup maps for O(1) display (avoids O(n²) scan)
    let blocked_map: std::collections::HashMap<&str, &Vec<String>> = result
        .blocked
        .iter()
        .map(|(id, deps)| (id.as_str(), deps))
        .collect();

    println!("  Artifacts in dependency order:");
    println!();
    for id in &result.order {
        let marker = if active_ids.contains(id) {
            "v"
        } else if blocked_map.contains_key(id.as_str()) {
            "x"
        } else {
            "o"
        };

        let status: String = if active_ids.contains(id) {
            "active".to_string()
        } else if let Some(blockers) = blocked_map.get(id.as_str()) {
            let names: Vec<&str> = blockers.iter().map(|s| s.as_str()).collect();
            format!("draft, blocked by {}", names.join(", "))
        } else {
            "draft, ready".to_string()
        };

        println!("    {} {} ({})", marker, id, status);
    }

    println!();
    let total = result.order.len();
    let active_count = result.order.iter().filter(|id| active_ids.contains(*id)).count();
    let ready_count = result.ready.len().saturating_sub(active_count);
    let blocked_count = result.blocked.len();
    println!(
        "  Total: {}  Active: {}  Ready: {}  Blocked: {}",
        total, active_count, ready_count, blocked_count
    );

    Ok(())
}
