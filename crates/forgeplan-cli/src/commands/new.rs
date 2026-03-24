use std::collections::HashMap;

use anyhow::{Context, Result};

use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::projection;
use forgeplan_core::template::{get_embedded_template, render_template};

use crate::commands::common;

pub async fn run(kind_str: &str, title: &str) -> Result<()> {
    let kind: ArtifactKind = kind_str.parse().map_err(|e| anyhow::anyhow!("{}", e))?;

    let (workspace, store) = common::open_store().await?;

    // Get next sequential ID from LanceDB
    let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
    let id = store.next_id(&prefix).await?;

    // The kind string used for template lookup
    let template_key = kind.template_key();
    let template = get_embedded_template(template_key)
        .ok_or_else(|| anyhow::anyhow!("No template found for kind '{}'", template_key))?;

    // Build template variables
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let nnn = id.split('-').last().unwrap_or("001").to_string();

    let mut vars = HashMap::new();
    vars.insert("NNN".to_string(), nnn.clone());
    vars.insert("title".to_string(), title.to_string());
    vars.insert("Title".to_string(), title.to_string());

    // Render the template with variable substitution
    let mut rendered = render_template(template, &vars);

    // Replace date placeholders
    rendered = rendered.replace("YYYY-MM-DD", &today);

    // Replace full ID patterns like PRD-{NNN} that may remain after render
    let heading_pattern = format!("# {}-{}: ", prefix, nnn);
    if let Some(pos) = rendered.find(&heading_pattern) {
        let line_start = pos + heading_pattern.len();
        if let Some(nl) = rendered[line_start..].find('\n') {
            let old_heading_text = &rendered[line_start..line_start + nl];
            if old_heading_text.contains('{') || old_heading_text.contains('/') {
                let before = &rendered[..line_start];
                let after = &rendered[line_start + nl..];
                rendered = format!("{}{}{}", before, title, after);
            }
        }
    }

    // Write to LanceDB (source of truth)
    let artifact = NewArtifact {
        id: id.clone(),
        kind: template_key.to_string(),
        status: "draft".to_string(),
        title: title.to_string(),
        body: rendered.clone(),
        depth: "standard".to_string(),
        author: None,
        parent_epic: None,
        valid_until: None,
    };
    store
        .create_artifact(&artifact)
        .await
        .with_context(|| format!("Failed to create artifact {} in LanceDB", id))?;

    // Render markdown projection (git-tracked)
    let filepath = projection::render_projection(
        &workspace,
        &id,
        template_key,
        title,
        "draft",
        "standard",
        None,
        None,
        None,
        &rendered,
        &[],
    )
    .await
    .with_context(|| format!("Failed to write projection for {}", id))?;

    println!("  Created: {}", filepath.display());
    println!("  ID:      {}", id);
    println!("  Kind:    {}", template_key);
    println!("  Title:   {}", title);
    Ok(())
}

