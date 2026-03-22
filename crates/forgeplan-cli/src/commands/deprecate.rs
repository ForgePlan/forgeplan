use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::lifecycle;
use forgeplan_core::workspace;

pub async fn run(id: &str, reason: &str) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let dependents = lifecycle::deprecate(&store, id, reason).await?;

    println!("  Deprecated {id}: {reason}");

    if !dependents.is_empty() {
        println!("\nDependents affected:");
        for dep in &dependents {
            println!("  ! {dep} depends on deprecated {id}");
        }
    }

    Ok(())
}
