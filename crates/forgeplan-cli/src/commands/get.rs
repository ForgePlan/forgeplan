use std::env;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::workspace;

pub async fn run(id: &str) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;
    let record = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", id))?;

    println!();
    println!("ID:           {}", record.id);
    println!("Kind:         {}", record.kind);
    println!("Status:       {}", record.status);
    println!("Title:        {}", record.title);
    println!("Depth:        {}", record.depth);
    if let Some(ref author) = record.author {
        println!("Author:       {}", author);
    }
    if let Some(ref epic) = record.parent_epic {
        if !epic.is_empty() {
            println!("Parent Epic:  {}", epic);
        }
    }
    if let Some(ref vu) = record.valid_until {
        println!("Valid Until:   {}", vu);
    }
    println!("R_eff:        {:.2}", record.r_eff_score);
    println!("Created:      {}", record.created_at);
    println!("Updated:      {}", record.updated_at);
    println!();
    println!("{}", record.body);

    Ok(())
}
