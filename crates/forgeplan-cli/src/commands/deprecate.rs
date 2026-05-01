use forgeplan_core::hints::{self, Hint};
use forgeplan_core::lifecycle;
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(id: &str, reason: &str) -> anyhow::Result<()> {
    let (ws, _lock, store) = common::open_store_locked().await?;

    let old_status = store
        .get_record(id)
        .await?
        .map(|r| r.status)
        .unwrap_or_else(|| "active".to_string());

    // Sync file→LanceDB before lifecycle call (preserve user edits)
    if let Some(record) = store.get_record(id).await? {
        projection::sync_file_to_store(&store, &ws, &record).await?;
    }

    // PRD-071 contract: error path emits a `Fix:` marker line so agents have
    // a deterministic next action — typically `validate` to surface why the
    // transition is rejected (e.g. draft→deprecated requires active first).
    let dependents = lifecycle::deprecate(&store, id, reason)
        .await
        .map_err(|e| anyhow::anyhow!("{}\nFix: forgeplan validate {}", e, id))?;

    // Re-render projection with updated status
    if let Some(record) = store.get_record(id).await? {
        let links = store.get_relations(id).await.unwrap_or_default();
        projection::render_projection(
            &ws,
            &record.id,
            &record.kind,
            &record.title,
            &record.status,
            &record.depth,
            record.author.as_deref(),
            record.parent_epic.as_deref(),
            record.valid_until.as_deref(),
            &record.body,
            &links,
        )
        .await?;
    }

    common::log_change_field(
        &store,
        id,
        "update",
        "status",
        Some(&old_status),
        Some("deprecated"),
        "cli",
    )
    .await;

    println!("  Deprecated {id}: {reason}");

    if !dependents.is_empty() {
        println!("\nDependents affected:");
        for dep in &dependents {
            println!("  ! {dep} depends on deprecated {id}");
        }
    }

    // Hint: if there are dependents, point at the first one (real ID) so
    // the operator can supersede or refactor; otherwise the action is
    // terminal — surface health.
    let hint_list = if let Some(first_dep) = dependents.first() {
        vec![
            Hint::warning(format!("{} depends on deprecated {}", first_dep, id))
                .with_action(format!("forgeplan get {}", first_dep)),
        ]
    } else {
        vec![Hint::info("Verify workspace integrity").with_action("forgeplan health".to_string())]
    };
    print!("{}", hints::render_next_action_line(&hint_list));

    Ok(())
}
