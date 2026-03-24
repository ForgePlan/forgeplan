use forgeplan_core::db::store::NewArtifact;

use crate::commands::common;

pub async fn run(path: &str, force: bool) -> anyhow::Result<()> {
    let (_ws, store) = common::open_store().await?;
    let cwd = std::env::current_dir()?;

    let full_path = if std::path::Path::new(path).is_absolute() {
        std::path::PathBuf::from(path)
    } else {
        cwd.join(path)
    };

    // Check file size before reading (max 100 MB)
    let file_size = std::fs::metadata(&full_path)
        .map_err(|e| anyhow::anyhow!("Failed to stat '{}': {}", full_path.display(), e))?
        .len();
    if file_size > 100 * 1024 * 1024 {
        anyhow::bail!("Import file too large ({} MB). Max 100 MB.", file_size / 1024 / 1024);
    }

    let json = std::fs::read_to_string(&full_path)
        .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", full_path.display(), e))?;
    let data: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| anyhow::anyhow!("Invalid export JSON: {}", e))?;

    let artifacts = data["artifacts"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing 'artifacts' array in export file"))?;

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

        // Validate kind against known types
        let kind_str = art["kind"].as_str().unwrap_or("note");
        if kind_str.parse::<forgeplan_core::artifact::types::ArtifactKind>().is_err() {
            eprintln!("  Warning: unknown kind '{}' for {}, defaulting to note", kind_str, id);
        }
        // Validate status
        let status_str = art["status"].as_str().unwrap_or("draft");
        if !matches!(status_str, "draft" | "active" | "superseded" | "deprecated") {
            eprintln!("  Warning: unknown status '{}' for {}, defaulting to draft", status_str, id);
        }

        let new_artifact = NewArtifact {
            id: id.to_string(),
            kind: kind_str.to_string(),
            status: status_str.to_string(),
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
