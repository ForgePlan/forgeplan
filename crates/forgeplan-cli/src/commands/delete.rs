use forgeplan_core::artifact::types::ArtifactKind;

use crate::commands::common;

pub async fn run(id: &str, yes: bool) -> anyhow::Result<()> {
    let (ws, store) = common::open_store().await?;

    let record = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact '{}' not found", id))?;

    // Check for dependents (other artifacts linking TO this one)
    let all_relations = store.get_all_relations().await?;
    let dependents: Vec<_> = all_relations
        .iter()
        .filter(|(_, target, _)| target.eq_ignore_ascii_case(id))
        .collect();

    if !dependents.is_empty() {
        eprintln!("  WARNING: {} has {} dependent(s):", id, dependents.len());
        for (source, _, rel) in &dependents {
            eprintln!("    {} --{}--> {}", source, rel, id);
        }
        if !yes {
            anyhow::bail!(
                "{} has {} dependent(s). Use --yes to confirm deletion despite dependents.",
                id,
                dependents.len()
            );
        }
        eprintln!("  Proceeding with --yes despite dependents.");
    }

    if !yes {
        anyhow::bail!(
            "About to delete {} \"{}\". This cannot be undone. Use --yes to confirm.",
            record.id,
            record.title
        );
    }

    // Cascade: delete all relations involving this artifact.
    // Count from already-fetched data to avoid double table scan.
    let relation_count = all_relations
        .iter()
        .filter(|(s, t, _)| s.eq_ignore_ascii_case(id) || t.eq_ignore_ascii_case(id))
        .count();
    if relation_count > 0 {
        store.delete_relations_for_artifact(id).await?;
        eprintln!("  Removed {} relation(s) involving {}", relation_count, id);
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
