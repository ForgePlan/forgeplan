use forgeplan_core::artifact::frontmatter::Frontmatter;
use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::validation::rules::check_stub_detailed;

use crate::commands::common;

/// Build a minimal Frontmatter BTreeMap from record fields for stub checking.
///
/// Note: import_cmd reads raw JSON (not `ArtifactRecord`), so we cannot reuse
/// `ArtifactRecord::frontmatter_map()` here. For the canonical builder, see
/// `forgeplan_core::db::store::ArtifactRecord::frontmatter_map`.
fn build_frontmatter(id: &str, kind: &str, status: &str, title: &str) -> Frontmatter {
    let mut fm = Frontmatter::new();
    fm.insert("id".to_string(), serde_yaml::Value::String(id.to_string()));
    fm.insert(
        "kind".to_string(),
        serde_yaml::Value::String(kind.to_string()),
    );
    fm.insert(
        "status".to_string(),
        serde_yaml::Value::String(status.to_string()),
    );
    fm.insert(
        "title".to_string(),
        serde_yaml::Value::String(title.to_string()),
    );
    fm
}

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
        anyhow::bail!(
            "Import file too large ({} MB). Max 100 MB.",
            file_size / 1024 / 1024
        );
    }

    let json = std::fs::read_to_string(&full_path)
        .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", full_path.display(), e))?;
    let data: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| anyhow::anyhow!("Invalid export JSON: {}", e))?;

    let artifacts = data["artifacts"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing 'artifacts' array in export file"))?;

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut downgraded = 0usize;

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
        let raw_kind = art["kind"].as_str().unwrap_or("note");
        let kind_str = if raw_kind
            .parse::<forgeplan_core::artifact::types::ArtifactKind>()
            .is_err()
        {
            eprintln!(
                "  Warning: unknown kind '{}' for {}, defaulting to note",
                raw_kind, id
            );
            "note"
        } else {
            raw_kind
        };
        let raw_status = art["status"].as_str().unwrap_or("draft");
        let status_str = if !matches!(raw_status, "draft" | "active" | "superseded" | "deprecated")
        {
            eprintln!(
                "  Warning: unknown status '{}' for {}, defaulting to draft",
                raw_status, id
            );
            "draft"
        } else {
            raw_status
        };

        let title = art["title"].as_str().unwrap_or("").to_string();
        let body = art["body"].as_str().unwrap_or("").to_string();

        // F3: Stub gate — prevent importing active artifacts with stub content.
        // Without --force, downgrade to draft to force reviewers through the
        // activate() gate instead of silently bypassing it.
        let final_status = if status_str == "active" {
            let fm = build_frontmatter(id, kind_str, status_str, &title);
            if let Some(report) = check_stub_detailed(&body, &fm) {
                if force {
                    eprintln!(
                        "  Warning: {} is a stub ({} markers) but --force bypasses gate, importing as active",
                        id, report.count
                    );
                    status_str.to_string()
                } else {
                    eprintln!(
                        "⚠ Importing {}: stub detected ({} markers). Downgrading to draft.",
                        id, report.count
                    );
                    downgraded += 1;
                    "draft".to_string()
                }
            } else {
                status_str.to_string()
            }
        } else {
            status_str.to_string()
        };

        let new_artifact = NewArtifact {
            id: id.to_string(),
            kind: kind_str.to_string(),
            status: final_status,
            title,
            body,
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
            if !source.is_empty()
                && !target.is_empty()
                && store.add_relation(source, target, relation).await.is_ok()
            {
                relations_imported += 1;
            }
        }
    }

    println!(
        "Imported {} artifacts ({} skipped, {} stubs downgraded to draft), {} relations",
        imported, skipped, downgraded, relations_imported
    );

    Ok(())
}
