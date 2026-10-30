use forgeplan_core::hints::{self, Hint};
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(id: &str, reason: &str) -> anyhow::Result<()> {
    let (ws, lock, store) = common::open_store_locked_tolerating_constellation().await?;
    run_file_first(ws, lock, id, reason, store.as_ref()).await
}

/// Reopen through the canonical document set for either index layout.
///
/// The successor key is minted before its display number is reserved, and
/// `reopen_captured` keeps the audit-C3 order: create → link → deprecate.
/// A failure before the last step leaves the old artifact active and the new
/// draft orphaned rather than retiring something with no successor.
async fn run_file_first(
    ws: std::path::PathBuf,
    lock: forgeplan_core::workspace::lock::WorkspaceLock,
    id: &str,
    reason: &str,
    store: Option<&forgeplan_core::db::store::LanceStore>,
) -> anyhow::Result<()> {
    let read = common::workspace_read(&ws).await?;
    let canonical = read
        .resolve_id(id)
        .ok_or_else(|| anyhow::anyhow!("Artifact '{id}' not found\nFix: forgeplan list"))?;
    let record = read
        .get_record(&canonical)
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {canonical}\nFix: forgeplan list"))?;
    if let Some(store) = store {
        if let Some(stored) = store.get_record(&record.canonical_key).await? {
            projection::sync_file_to_store(store, &ws, &stored).await?;
        }
    }

    // The context borrow ends before reconcile, which reacquires the same
    // workspace lock on constellation-managed workspaces.
    let result = {
        let ctx = projection::MutationContext::new(&ws, &lock, store);
        forgeplan_core::lifecycle::canonical::reopen_captured(&ctx, &read, &canonical, reason)
            .await
            .map_err(|e| anyhow::anyhow!("{}\nFix: forgeplan validate {}", e, canonical))?
    };

    common::log_change_field(
        &ws,
        store,
        &canonical,
        "update",
        ("status", Some(&record.status), Some("deprecated")),
        "cli",
    )
    .await;
    common::log_change_field(
        &ws,
        store,
        &result.new_id,
        "create",
        ("status", None, Some("draft")),
        "cli",
    )
    .await;

    let published = if store.is_none() {
        common::note_constellation_owns_index();
        // Lock released before reconcile — see `common::reconcile_after_file_mutation`.
        drop(lock);
        Some(common::reconcile_after_file_mutation(&ws).await)
    } else {
        None
    };

    println!("  Reopened {canonical} → deprecated");
    println!(
        "  Created {} ({}, draft) with lineage from {canonical}",
        result.new_id, result.new_kind
    );
    println!("  Link: {} --based_on--> {canonical}", result.new_id);
    println!("  Key: {}", result.new_key);

    let next_hints: Vec<Hint> = vec![
        Hint::info(format!(
            "New draft {} created — fill MUST sections",
            result.new_id
        ))
        .with_action(format!("forgeplan validate {}", result.new_id)),
    ];
    print!("{}", hints::render_next_action_line(&next_hints));

    if published == Some(false) {
        println!("  ! derived index not yet updated");
    }
    Ok(())
}
