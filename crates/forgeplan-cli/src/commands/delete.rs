use std::env;

use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::LanceStore;
use forgeplan_core::workspace;

pub async fn run(id: &str, yes: bool) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&ws).await?;

    let record = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", id))?;

    if !yes {
        eprintln!(
            "  About to delete {} \"{}\" (kind: {}, status: {})",
            record.id, record.title, record.kind, record.status
        );
        eprintln!("  This cannot be undone. Use --yes to confirm.");
        return Ok(());
    }

    // Delete from LanceDB
    store.delete_artifact(id).await?;

    // Remove markdown projection file
    if let Ok(kind) = record.kind.parse::<ArtifactKind>() {
        let slug = forgeplan_core::artifact::types::slugify(&record.title);
        let filename = format!("{}-{}.md", record.id, slug);
        let filepath = ws.join(kind.dir_name()).join(&filename);
        if filepath.exists() {
            tokio::fs::remove_file(&filepath).await.ok();
        }
    }

    println!("  Deleted: {} \"{}\"", record.id, record.title);

    Ok(())
}
