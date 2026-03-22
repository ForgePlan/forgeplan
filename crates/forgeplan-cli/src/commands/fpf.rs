use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::fpf;
use forgeplan_core::workspace;

pub async fn run() -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let dashboard = fpf::dashboard(&store).await?;
    print!("{dashboard}");

    Ok(())
}
