use forgeplan_core::lifecycle;

use crate::commands::common;

pub async fn run(id: &str, by: &str) -> anyhow::Result<()> {
    let store = common::store().await?;
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
