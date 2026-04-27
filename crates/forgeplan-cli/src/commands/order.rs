use std::collections::HashSet;

use forgeplan_core::graph::topological;
use forgeplan_core::hints::{self, Hint};

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

    // PRD-071 contract: priority — fix cycles first, then activate ready
    // artifacts, else inspect order.
    let mut hints_vec: Vec<Hint> = Vec::new();
    if let Some(cycle) = result.cycles.first() {
        if let Some(first_id) = cycle.first() {
            hints_vec.push(
                Hint::warning(format!("Cycle includes {} — break it", first_id))
                    .with_action(format!("forgeplan get {}", first_id)),
            );
        }
    } else if let Some(ready_id) = result
        .ready
        .iter()
        .find(|id| !active_ids.contains(*id))
        .cloned()
    {
        hints_vec.push(
            Hint::suggestion(format!("Activate ready artifact {}", ready_id))
                .with_action(format!("forgeplan activate {}", ready_id)),
        );
    } else if let Some(id) = result.order.first() {
        hints_vec.push(
            Hint::info(format!("Inspect first in topological order ({})", id))
                .with_action(format!("forgeplan get {}", id)),
        );
    }

    if json {
        let data = serde_json::json!({
            "order": result.order,
            "cycles": result.cycles,
            "blocked": result.blocked.iter().map(|(id, deps)| serde_json::json!({"id": id, "blocked_by": deps})).collect::<Vec<_>>(),
            "ready": result.ready,
            "_next_action": hints::primary_action(&hints_vec),
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
        let bootstrap = vec![
            Hint::suggestion("Link two artifacts to build a dependency graph").with_action(
                "forgeplan link <source-id> <target-id> --relation refines".to_string(),
            ),
        ];
        print!("{}", hints::render_next_action_line(&bootstrap));
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
    let active_count = result
        .order
        .iter()
        .filter(|id| active_ids.contains(*id))
        .count();
    let ready_count = result.ready.len().saturating_sub(active_count);
    let blocked_count = result.blocked.len();
    println!(
        "  Total: {}  Active: {}  Ready: {}  Blocked: {}",
        total, active_count, ready_count, blocked_count
    );

    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}
