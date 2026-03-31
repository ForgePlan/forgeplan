use anyhow::Result;
use console::style;

use forgeplan_core::artifact::types::{slugify, ArtifactKind};
use forgeplan_core::db::store::{ArtifactFilter, NewArtifact};
use forgeplan_core::projection;

use crate::commands::common;

pub async fn run(
    text: Option<&str>,
    category: Option<&str>,
    list: bool,
    forget: Option<&str>,
) -> Result<()> {
    if list {
        return run_list().await;
    }

    if let Some(id) = forget {
        return run_forget(id).await;
    }

    let text = text.ok_or_else(|| {
        anyhow::anyhow!("Provide text to remember, or use --list / --forget <id>")
    })?;

    run_remember(text, category.unwrap_or("fact")).await
}

async fn run_remember(text: &str, category: &str) -> Result<()> {
    let (workspace, store) = common::open_store().await?;

    // Generate slug from first 50 chars
    let slug_source: String = text.chars().take(50).collect();
    let slug = slugify(&slug_source);
    let id = format!("mem-{}", slug);

    // Title = first 80 chars
    let title: String = text.chars().take(80).collect();

    // Build markdown body with frontmatter
    let now = chrono::Utc::now().to_rfc3339();
    let body = format!(
        "---\nid: \"{}\"\nkind: memory\ncategory: {}\nstatus: active\ndepth: tactical\ntitle: \"{}\"\ncreated: {}\nauthor: cli\n---\n\n{}",
        id, category, title.replace('"', "\\\""), now, text
    );

    // Create in LanceDB
    let artifact = NewArtifact {
        id: id.clone(),
        kind: "memory".to_string(),
        status: "active".to_string(),
        title: title.clone(),
        body: body.clone(),
        depth: "tactical".to_string(),
        author: Some("cli".to_string()),
        parent_epic: None,
        valid_until: None,
    };
    store.create_artifact(&artifact).await?;

    // Write markdown projection
    projection::render_projection(
        &workspace,
        &id,
        "memory",
        &title,
        "active",
        "tactical",
        Some("cli"),
        None,
        None,
        &body,
        &[],
    )
    .await?;

    println!(
        "  Remembered: {} — \"{}\"",
        style(&id).bold(),
        title
    );
    Ok(())
}

async fn run_list() -> Result<()> {
    let store = common::store().await?;

    let filter = ArtifactFilter {
        kind: Some("memory".to_string()),
        status: None,
    };
    let records = store.list_records(Some(&filter)).await?;

    if records.is_empty() {
        println!("  No memories found.");
        return Ok(());
    }

    // Print header
    let id_width = records.iter().map(|r| r.id.len()).max().unwrap_or(6).max(2);
    println!(
        "{:<id_w$}  {:<12}  {:<12}  {}",
        style("ID").bold().underlined(),
        style("Category").bold().underlined(),
        style("Created").bold().underlined(),
        style("Text").bold().underlined(),
        id_w = id_width,
    );

    for r in &records {
        let category = common::extract_frontmatter_field(&r.body, "category")
            .unwrap_or_else(|| "fact".to_string());
        let date: String = r.created_at.chars().take(10).collect();
        let plain_text = common::extract_plain_text(&r.body);
        let truncated: String = plain_text.chars().take(60).collect();

        println!(
            "{:<id_w$}  {:<12}  {:<12}  {}",
            style(&r.id).bold(),
            category,
            date,
            truncated,
            id_w = id_width,
        );
    }

    println!("\n  {} memory(ies) total", records.len());
    Ok(())
}

async fn run_forget(id: &str) -> Result<()> {
    let (ws, store) = common::open_store().await?;

    // Verify it exists and is a memory
    let record = store
        .get_record(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Memory '{}' not found", id))?;

    if record.kind != "memory" {
        anyhow::bail!("'{}' is not a memory (kind: {})", id, record.kind);
    }

    // Delete from LanceDB
    store.delete_artifact(id).await?;

    // Remove markdown file
    let slug = slugify(&record.title);
    let filename = format!("{}-{}.md", record.id, slug);
    let filepath = ws.join(ArtifactKind::Memory.dir_name()).join(&filename);
    if filepath.exists() {
        tokio::fs::remove_file(&filepath).await.ok();
    }

    println!("  Forgotten: {}", style(id).bold());
    Ok(())
}

