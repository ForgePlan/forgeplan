use forgeplan_core::lifecycle;

use crate::commands::common;

pub async fn run(id: &str, force: bool) -> anyhow::Result<()> {
    let store = common::store().await?;
    let result = lifecycle::activate(&store, id, force).await?;

    if result.forced {
        println!("  Activated {id} (draft → active)");
        println!(
            "  Warning: Activated with {} validation error{}",
            result.must_errors.len(),
            if result.must_errors.len() == 1 { "" } else { "s" }
        );
    } else {
        println!("  Activated {id} (draft → active)");
    }

    Ok(())
}
