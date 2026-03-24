use forgeplan_core::lifecycle;

use crate::commands::common;

pub async fn run(id: &str) -> anyhow::Result<()> {
    let store = common::store().await?;
    lifecycle::activate(&store, id).await?;
    println!("  Activated {id} (draft → active)");

    Ok(())
}
