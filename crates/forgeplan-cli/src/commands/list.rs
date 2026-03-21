use std::env;

use anyhow::Result;

use forgeplan_core::db::store::{ArtifactFilter, LanceStore};
use forgeplan_core::workspace::find_workspace;

pub async fn run(kind_filter: Option<&str>, status_filter: Option<&str>) -> Result<()> {
    let cwd = env::current_dir()?;
    let workspace = find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("Not in a forgeplan workspace. Run `forgeplan init` first."))?;

    let store = LanceStore::open(&workspace).await?;

    let filter = if kind_filter.is_some() || status_filter.is_some() {
        Some(ArtifactFilter {
            kind: kind_filter.map(|s| s.to_lowercase()),
            status: status_filter.map(|s| s.to_lowercase()),
        })
    } else {
        None
    };

    let artifacts = store.list_artifacts(filter.as_ref()).await?;

    if artifacts.is_empty() {
        println!("  No artifacts found.");
        return Ok(());
    }

    // Calculate column widths for alignment
    let id_width = artifacts.iter().map(|a| a.id.len()).max().unwrap_or(6).max(2);
    let kind_width = artifacts
        .iter()
        .map(|a| a.kind.len())
        .max()
        .unwrap_or(6)
        .max(4);
    let status_width = artifacts
        .iter()
        .map(|a| a.status.len())
        .max()
        .unwrap_or(6)
        .max(6);

    // Print header
    println!(
        "{:<id_w$}  {:<kind_w$}  {:<status_w$}  Title",
        "ID",
        "Kind",
        "Status",
        id_w = id_width,
        kind_w = kind_width,
        status_w = status_width,
    );

    // Print rows
    for a in &artifacts {
        println!(
            "{:<id_w$}  {:<kind_w$}  {:<status_w$}  {}",
            a.id,
            a.kind,
            a.status,
            a.title,
            id_w = id_width,
            kind_w = kind_width,
            status_w = status_width,
        );
    }

    println!("\n  {} artifact(s) total", artifacts.len());
    Ok(())
}
