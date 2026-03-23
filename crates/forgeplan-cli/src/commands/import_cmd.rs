use std::env;

use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::workspace;

pub async fn run(path: &str, force: bool) -> anyhow::Result<()> {
    let cwd = env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;

    let full_path = if std::path::Path::new(path).is_absolute() {
        std::path::PathBuf::from(path)
    } else {
        cwd.join(path)
    };

    let json = std::fs::read_to_string(&full_path)
        .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", full_path.display(), e))?;
    let data: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| anyhow::anyhow!("Invalid export JSON: {}", e))?;

    let artifacts = data["artifacts"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing 'artifacts' array in export file"))?;

    let store = LanceStore::open(&ws).await?;

    let mut imported = 0usize;
    let mut skipped = 0usize;

    for art in artifacts {
        let id = art["id"].as_str().unwrap_or_default();
        if id.is_empty() {
            continue;
        }

        let existing = store.get_record(id).await?;
        if existing.is_some() && !force {
            skipped += 1;
            continue;
        }

        if existing.is_some() {
            store.delete_artifact(id).await?;
        }

        let new_artifact = NewArtifact {
            id: id.to_string(),
            kind: art["kind"].as_str().unwrap_or("note").to_string(),
            status: art["status"].as_str().unwrap_or("draft").to_string(),
            title: art["title"].as_str().unwrap_or("").to_string(),
            body: art["body"].as_str().unwrap_or("").to_string(),
            depth: art["depth"].as_str().unwrap_or("standard").to_string(),
            author: art["author"].as_str().map(String::from),
            parent_epic: art["parent_epic"].as_str().map(String::from),
            valid_until: art["valid_until"].as_str().map(String::from),
        };

        store.create_artifact(&new_artifact).await?;
        imported += 1;
    }

    let mut relations_imported = 0usize;
    if let Some(relations) = data["relations"].as_array() {
        for rel in relations {
            let source = rel["source"].as_str().unwrap_or_default();
            let target = rel["target"].as_str().unwrap_or_default();
            let relation = rel["relation"].as_str().unwrap_or("informs");
            if !source.is_empty() && !target.is_empty() {
                if store.add_relation(source, target, relation).await.is_ok() {
                    relations_imported += 1;
                }
            }
        }
    }

    println!(
        "Imported {} artifacts ({} skipped), {} relations",
        imported, skipped, relations_imported
    );

    Ok(())
}
