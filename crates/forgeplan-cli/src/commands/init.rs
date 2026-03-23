use std::env;
use std::fs;

use anyhow::Result;
use console::style;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::workspace::{find_workspace, init_workspace, FORGEPLAN_DIR};

use crate::ui;

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
        tokio::fs::remove_dir_all(&existing).await?;
    }

    // Non-interactive mode (for CI, tests, scripts)
    if non_interactive {
        let project_name = cwd
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".into());

        let ws = init_workspace(&cwd, &project_name)?;
        LanceStore::init(&ws).await?;

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
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "my-project".into());

    let project_name: String = cliclack::input("Project name?")
        .placeholder(&default_name)
        .default_input(&default_name)
        .interact()?;

    // Agent selection
    let agents: Vec<&str> = cliclack::multiselect("Which AI agents to configure?")
        .initial_values(vec!["claude"])
        .item("claude", "Claude Code", ".mcp.json + CLAUDE.md section")
        .item("cursor", "Cursor", ".mcp.json + .cursorrules")
        .item("codex", "Codex", "AGENTS.md")
        .item("gemini", "Gemini CLI", ".gemini/settings.json")
        .item("copilot", "GitHub Copilot", ".github/copilot-instructions.md")
        .interact()?;

    // Spinner — create workspace
    let spinner = cliclack::spinner();
    spinner.start("Creating workspace...");

    let ws = init_workspace(&cwd, &project_name)?;
    LanceStore::init(&ws).await?;

    spinner.stop("Workspace created");

    // Generate .mcp.json if needed
    let generate_mcp = agents.iter().any(|a| *a == "claude" || *a == "cursor");
    if generate_mcp {
        let mcp_path = cwd.join(".mcp.json");
        if !mcp_path.exists() {
            let mcp_content = serde_json::json!({
                "mcpServers": {
                    "forgeplan": {
                        "command": "forgeplan",
                        "args": ["serve"]
                    }
                }
            });
            fs::write(&mcp_path, serde_json::to_string_pretty(&mcp_content)?)?;
            cliclack::log::success(format!(".mcp.json created"))?;
        } else {
            cliclack::log::info(".mcp.json already exists — skipped")?;
        }
    }

    // Generate .cursorrules if Cursor selected
    if agents.contains(&"cursor") {
        let rules_path = cwd.join(".cursorrules");
        if !rules_path.exists() {
            let rules = include_str!("../../templates/cursorrules.md");
            fs::write(&rules_path, rules)?;
            cliclack::log::success(".cursorrules created")?;
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
    summary_lines.push(format!(
        "Agents:         {}",
        agents
            .iter()
            .map(|a| match *a {
                "claude" => "Claude Code",
                "cursor" => "Cursor",
                "codex" => "Codex",
                "gemini" => "Gemini CLI",
                "copilot" => "Copilot",
                _ => a,
            })
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
