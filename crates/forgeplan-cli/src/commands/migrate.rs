use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run() -> anyhow::Result<()> {
    println!("  Running schema migrations...");
    let _store = common::store().await?; // open() runs migrations
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
