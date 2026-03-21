use std::env;

use anyhow::Result;

use forgeplan_core::artifact::store::list_artifacts;
use forgeplan_core::workspace::find_workspace;

pub fn run(kind_filter: Option<&str>, status_filter: Option<&str>) -> Result<()> {
    let cwd = env::current_dir()?;
    let workspace = find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("Not in a forgeplan workspace. Run `forgeplan init` first."))?;

    let mut artifacts = list_artifacts(&workspace)?;

    // Apply filters
    if let Some(kind) = kind_filter {
        let kind_lower = kind.to_lowercase();
        artifacts.retain(|a| a.kind.to_lowercase() == kind_lower);
    }
    if let Some(status) = status_filter {
        let status_lower = status.to_lowercase();
        artifacts.retain(|a| a.status.to_lowercase() == status_lower);
    }

    if artifacts.is_empty() {
        println!("  No artifacts found.");
        return Ok(());
    }

    // Calculate column widths for alignment
    let id_width = artifacts.iter().map(|a| a.id.len()).max().unwrap_or(6).max(2);
    let kind_width = artifacts.iter().map(|a| a.kind.len()).max().unwrap_or(6).max(4);
    let status_width = artifacts.iter().map(|a| a.status.len()).max().unwrap_or(6).max(6);

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
