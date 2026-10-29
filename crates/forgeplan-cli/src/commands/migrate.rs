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
        drop(lock);
        let capability = MaintainCapability::for_strict_maintenance(ws.canonicalize()?);
        maintain_index(
            &capability,
            CaptureLimits::default(),
            std::time::Instant::now() + std::time::Duration::from_secs(120),
        )
        .await?;
    } else {
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
