use crate::commands::common;

pub async fn run() -> anyhow::Result<()> {
    println!("  Running schema migrations...");
    let _store = common::store().await?; // open() runs migrations
    println!("  Migrations complete. Schema up to date.");

    Ok(())
}
