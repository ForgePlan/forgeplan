use std::collections::HashMap;
use std::env;
use std::fs;

use anyhow::{Context, Result};

use forgeplan_core::artifact::store::{kind_dir, next_id, slugify};
use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::template::{get_embedded_template, render_template};
use forgeplan_core::workspace::{find_workspace, load_config};

pub fn run(kind_str: &str, title: &str) -> Result<()> {
    let kind = parse_kind(kind_str)?;

    let cwd = env::current_dir()?;
    let workspace = find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("Not in a forgeplan workspace. Run `forgeplan init` first."))?;

    let config = load_config(&workspace)?;
    let id = next_id(&workspace, &kind, config.id_digits)?;
    let slug = slugify(title);

    // The kind string used for template lookup (lowercase, CLI-style name)
    let template_key = kind_to_template_key(&kind);
    let template = get_embedded_template(template_key)
        .ok_or_else(|| anyhow::anyhow!("No template found for kind '{}'", template_key))?;

    // Build template variables
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    // Extract the numeric part from ID like "PRD-001" -> "001"
    let nnn = id
        .split('-')
        .last()
        .unwrap_or("001")
        .to_string();
    let prefix = kind.prefix().trim_end_matches('-').to_uppercase();

    let mut vars = HashMap::new();
    vars.insert("NNN".to_string(), nnn.clone());
    vars.insert("title".to_string(), title.to_string());
    vars.insert("Title".to_string(), title.to_string());

    // Render the template with variable substitution
    let mut rendered = render_template(template, &vars);

    // Replace date placeholders
    rendered = rendered.replace("YYYY-MM-DD", &today);

    // Replace full ID patterns like PRD-{NNN} that may remain after render
    // (render_template already handles {NNN}, but the prefix-qualified pattern
    // e.g. "PRD-001" is produced naturally from "PRD-{NNN}" -> "PRD-001")
    // So this should already work. But let's also replace the heading pattern.
    // The heading "# PRD-001: {Product Area / Feature Name}" needs title substitution.
    // The template renders {NNN} to the number, producing "# PRD-001: ..."
    // We replace the boilerplate heading hint with the actual title.
    let heading_pattern = format!("# {}-{}: ", prefix, nnn);
    if let Some(pos) = rendered.find(&heading_pattern) {
        // Find the end of that line
        let line_start = pos + heading_pattern.len();
        if let Some(nl) = rendered[line_start..].find('\n') {
            let old_heading_text = &rendered[line_start..line_start + nl];
            // Only replace if it looks like a placeholder (contains braces or slashes)
            if old_heading_text.contains('{') || old_heading_text.contains('/') {
                let before = &rendered[..line_start];
                let after = &rendered[line_start + nl..];
                rendered = format!("{}{}{}", before, title, after);
            }
        }
    }

    // Write file
    let dir = workspace.join(kind_dir(&kind));
    fs::create_dir_all(&dir)?;
    let filename = format!("{}-{}.md", id, slug);
    let filepath = dir.join(&filename);

    fs::write(&filepath, &rendered)
        .with_context(|| format!("Failed to write {}", filepath.display()))?;

    println!("  Created: {}", filepath.display());
    println!("  ID:      {}", id);
    println!("  Kind:    {}", template_key);
    println!("  Title:   {}", title);
    Ok(())
}

fn parse_kind(s: &str) -> Result<ArtifactKind> {
    match s.to_lowercase().as_str() {
        "prd" => Ok(ArtifactKind::Prd),
        "epic" => Ok(ArtifactKind::Epic),
        "spec" => Ok(ArtifactKind::Spec),
        "rfc" => Ok(ArtifactKind::Rfc),
        "adr" => Ok(ArtifactKind::Adr),
        "problem" => Ok(ArtifactKind::ProblemCard),
        "solution" => Ok(ArtifactKind::SolutionPortfolio),
        "evidence" => Ok(ArtifactKind::EvidencePack),
        "note" => Ok(ArtifactKind::Note),
        "refresh" => Ok(ArtifactKind::RefreshReport),
        _ => anyhow::bail!(
            "Unknown artifact kind: '{}'. Valid: prd, epic, spec, rfc, adr, problem, solution, evidence, note, refresh",
            s
        ),
    }
}

/// Map ArtifactKind to the template lookup key.
fn kind_to_template_key(kind: &ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Prd => "prd",
        ArtifactKind::Epic => "epic",
        ArtifactKind::Spec => "spec",
        ArtifactKind::Rfc => "rfc",
        ArtifactKind::Adr => "adr",
        ArtifactKind::ProblemCard => "problem",
        ArtifactKind::SolutionPortfolio => "solution",
        ArtifactKind::EvidencePack => "evidence",
        ArtifactKind::Note => "note",
        ArtifactKind::RefreshReport => "refresh",
    }
}
