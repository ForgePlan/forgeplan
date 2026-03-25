use forgeplan_core::lifecycle;

use crate::commands::common;

pub async fn run(id: &str, reason: &str) -> anyhow::Result<()> {
    let store = common::store().await?;
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
