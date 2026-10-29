use forgeplan_core::constellation::{
    capture::CaptureLimits,
    facade::{MaintainCapability, maintain_index},
    storage,
};
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run() -> anyhow::Result<()> {
    println!("  Running schema migrations...");
    // Opening a legacy store runs migrations, so choose the owner while the
    // workspace is locked and before touching any tables. Invalid protocol
    // metadata must never fall through to a legacy open.
    let (ws, lock) = common::open_workspace_locked().await?;
    let index_root = ws.join("lance");
    storage::read_marker(&index_root)?;
    storage::read_current(&index_root)?;
    if common::index_migrated(&ws) {
        // Maintenance owns its owner -> workspace lock ordering. Do not hold
        // this routing guard across the pass or open a legacy store first.
        //
        // A constellation layout opens and creates NO legacy index: the
        // identity-v5 tables belong to the legacy plane only.
        drop(lock);
        let capability = MaintainCapability::for_strict_maintenance(ws.canonicalize()?);
        maintain_index(
            &capability,
            CaptureLimits::default(),
            std::time::Instant::now() + std::time::Duration::from_secs(120),
        )
        .await?;
    } else {
        // Legacy layout: the staged identity-v5 migration runs here, under
        // the workspace lock this command already holds. It rebuilds the
        // index from the Markdown, verifies it, and publishes it behind the
        // readiness marker. A conflict leaves the original index untouched.
        let report = forgeplan_core::db::migrate::run_identity_v5_migration(&ws).await?;
        if report.already_published {
            println!(
                "  Identity-v5 index already published ({} documents).",
                report.document_count
            );
        } else {
            println!(
                "  Identity-v5 index published: {} documents, {} relations, {} evidence rows.",
                report.document_count, report.relation_count, report.evidence_count
            );
        }
        for (alias, paths) in &report.ambiguous_aliases {
            println!(
                "  Warning: display alias '{}' is shared by {} documents ({}) — a reference to it is ambiguous.",
                alias,
                paths.len(),
                paths.join(", ")
            );
        }
        for reference in &report.unresolved_references {
            println!(
                "  Warning: reference '{}' names no document of this workspace — it is kept as a forward reference.",
                reference
            );
        }
        let _store = forgeplan_core::db::store::LanceStore::open(&ws).await?;
    }
    println!("  Migrations complete. Schema up to date.");

    // PRD-071 contract: after migrate, surface health so any drift triggered
    // by the schema change is visible.
    let hints_vec = vec![
        Hint::suggestion("Audit workspace after migration")
            .with_action("forgeplan health".to_string()),
    ];
    print!("{}", hints::render_next_action_line(&hints_vec));

    Ok(())
}
