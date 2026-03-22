use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::lifecycle;
use forgeplan_core::workspace;

pub async fn run(id: &str, by: &str) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let dependents = lifecycle::supersede(&store, id, by).await?;

    println!("  Superseded {id} → {by}");

    if !dependents.is_empty() {
        println!("\nDependents to update:");
        for dep in &dependents {
            println!("  ! {dep} depends on superseded {id} → consider updating to {by}");
        }
    }

    Ok(())
}
