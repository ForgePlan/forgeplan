use forgeplan_core::lifecycle;

use crate::commands::common;

pub async fn run(id: &str, by: &str) -> anyhow::Result<()> {
    let store = common::store().await?;
    let result = lifecycle::supersede(&store, id, by).await?;

    println!("  Superseded {id} → {by}");

    for w in &result.warnings {
        println!("  {w}");
    }

    if !result.dependents.is_empty() {
        println!("\nDependents to update:");
        for dep in &result.dependents {
            println!("  ! {dep} depends on superseded {id} → consider updating to {by}");
        }
    }

    Ok(())
}
