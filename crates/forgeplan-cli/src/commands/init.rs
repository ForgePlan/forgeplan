use std::env;
use std::fs;

use anyhow::Result;
use console::style;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::workspace::{find_workspace, init_workspace, FORGEPLAN_DIR};

use crate::ui;

/// Default project name fallback (unified for both paths).
const DEFAULT_PROJECT_NAME: &str = "my-project";

pub async fn run(force: bool, non_interactive: bool) -> Result<()> {
    let cwd = env::current_dir()?;

    // Check if already initialized
    if let Some(existing) = find_workspace(&cwd) {
        if !force {
            if non_interactive {
                println!("  Already initialized at {}", existing.display());
            } else {
                cliclack::log::warning(format!(
                    "Already initialized at {}. Use --force to reinitialize.",
                    existing.display()
                ))?;
            }
            return Ok(());
        }

        // [SECURITY] Guard against symlink attack and workspace outside cwd
        safe_remove_workspace(&existing, &cwd).await?;
    }

    // Non-interactive mode (for CI, tests, scripts)
    if non_interactive {
        let project_name = cwd
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| DEFAULT_PROJECT_NAME.into());

        init_with_rollback(&cwd, &project_name).await?;

        println!("  Initialized {}/ in {}", FORGEPLAN_DIR, cwd.display());
        println!("  Project: {}", project_name);
        return Ok(());
    }

    // ─── Interactive wizard ──────────────────────────────────────

    ui::print_banner();
    cliclack::intro(style(" forgeplan init ").on_cyan().black())?;

    // Project name
    let default_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| DEFAULT_PROJECT_NAME.into());

    let project_name: String = cliclack::input("Project name?")
        .placeholder(&default_name)
        .default_input(&default_name)
        .validate(|input: &String| {
            if input.len() > 128 {
                Err("Project name too long (max 128 characters)")
            } else if input.contains('\n') || input.contains('\r') {
                Err("Project name cannot contain newlines")
            } else {
                Ok(())
            }
        })
        .interact()?;

    // Agent selection
    let agents: Vec<&str> = cliclack::multiselect("Which AI agents to configure?")
        .initial_values(vec!["claude"])
        .item("claude", "Claude Code", ".mcp.json + CLAUDE.md section")
        .item("cursor", "Cursor", ".mcp.json + .cursorrules")
        .item("codex", "Codex", "AGENTS.md (coming soon)")
        .item("gemini", "Gemini CLI", ".gemini/settings.json (coming soon)")
        .item("copilot", "GitHub Copilot", "copilot-instructions.md (coming soon)")
        .interact()?;

    // Spinner — create workspace (with rollback on failure)
    let spinner = cliclack::spinner();
    spinner.start("Creating workspace...");

    init_with_rollback(&cwd, &project_name).await?;

    spinner.stop("Workspace created");

    // Generate agent configs
    let generate_mcp = agents.iter().any(|a| *a == "claude" || *a == "cursor");
    if generate_mcp {
        generate_mcp_json(&cwd)?;
    }

    if agents.contains(&"cursor") {
        generate_cursorrules(&cwd)?;
    }

    // Log "coming soon" for unimplemented agents
    for agent in &agents {
        match *agent {
            "codex" => { cliclack::log::info("Codex: AGENTS.md support coming soon")?; }
            "gemini" => { cliclack::log::info("Gemini CLI: config support coming soon")?; }
            "copilot" => { cliclack::log::info("Copilot: instructions support coming soon")?; }
            _ => {}
        }
    }

    // Summary
    let mut summary_lines = vec![
        ".forgeplan/     created".to_string(),
        "LanceDB         initialized".to_string(),
    ];
    if generate_mcp {
        summary_lines.push(".mcp.json       ready".into());
    }
    if agents.contains(&"cursor") {
        summary_lines.push(".cursorrules    ready".into());
    }
    summary_lines.push(format!(
        "Agents:         {}",
        agents
            .iter()
            .map(|a| agent_display_name(a))
            .collect::<Vec<_>>()
            .join(", ")
    ));

    cliclack::note("Installation Summary", summary_lines.join("\n"))?;

    cliclack::outro(format!(
        "Done! Next: {} or {}",
        style("forgeplan health").cyan(),
        style("/forge \"your task\"").cyan()
    ))?;

    Ok(())
}

/// Initialize workspace + LanceDB with rollback on failure.
/// If LanceStore::init fails, removes the partially created .forgeplan/ directory.
async fn init_with_rollback(cwd: &std::path::Path, project_name: &str) -> Result<()> {
    let ws = init_workspace(cwd, project_name)?;
    if let Err(e) = LanceStore::init(&ws).await {
        // Rollback: remove partially created workspace
        let _ = tokio::fs::remove_dir_all(&ws).await;
        return Err(e.into());
    }
    Ok(())
}

/// Safely remove a workspace directory, guarding against symlinks and paths outside cwd.
async fn safe_remove_workspace(
    workspace: &std::path::Path,
    cwd: &std::path::Path,
) -> Result<()> {
    // Guard: workspace must be inside cwd
    let canonical_ws = workspace.canonicalize().unwrap_or_else(|_| workspace.to_path_buf());
    let canonical_cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    if !canonical_ws.starts_with(&canonical_cwd) {
        anyhow::bail!(
            "Workspace {} is outside current directory, refusing --force",
            workspace.display()
        );
    }

    // Guard: workspace must not be a symlink
    let meta = fs::symlink_metadata(workspace)?;
    if meta.file_type().is_symlink() {
        anyhow::bail!(
            "Workspace {} is a symlink, refusing --force for safety",
            workspace.display()
        );
    }

    tokio::fs::remove_dir_all(workspace).await?;
    Ok(())
}

/// Generate .mcp.json for Claude Code / Cursor.
fn generate_mcp_json(cwd: &std::path::Path) -> Result<()> {
    let mcp_path = cwd.join(".mcp.json");
    if mcp_path.exists() {
        cliclack::log::info(".mcp.json already exists — skipped")?;
        return Ok(());
    }
    let mcp_content = serde_json::json!({
        "mcpServers": {
            "forgeplan": {
                "command": "forgeplan",
                "args": ["serve"]
            }
        }
    });
    fs::write(&mcp_path, serde_json::to_string_pretty(&mcp_content)?)?;
    cliclack::log::success(".mcp.json created")?;
    Ok(())
}

/// Generate .cursorrules for Cursor.
fn generate_cursorrules(cwd: &std::path::Path) -> Result<()> {
    let rules_path = cwd.join(".cursorrules");
    if rules_path.exists() {
        cliclack::log::info(".cursorrules already exists — skipped")?;
        return Ok(());
    }
    let rules = include_str!("../../templates/cursorrules.md");
    fs::write(&rules_path, rules)?;
    cliclack::log::success(".cursorrules created")?;
    Ok(())
}

/// Human-readable display name for agent ID.
fn agent_display_name(agent: &str) -> &str {
    match agent {
        "claude" => "Claude Code",
        "cursor" => "Cursor",
        "codex" => "Codex",
        "gemini" => "Gemini CLI",
        "copilot" => "Copilot",
        _ => agent,
    }
}
