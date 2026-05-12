use std::env;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::Result;
use console::style;

use forgeplan_core::config::Config;
use forgeplan_core::db::store::LanceStore;
use forgeplan_core::hints::{self, Hint};
use forgeplan_core::plugins::{
    KnownPlaybook, build_recommendations, detect_plugins, detect_signals, extended_registry,
    format_recommendations,
};
use forgeplan_core::scan::import::{ImportStatus, ScanImportOptions, scan_and_import_to_workspace};
use forgeplan_core::workspace::{ARTIFACT_DIRS, FORGEPLAN_DIR, find_workspace, init_workspace};

use crate::ui;

/// Default project name fallback (unified for both paths).
const DEFAULT_PROJECT_NAME: &str = "my-project";

pub async fn run(force: bool, non_interactive: bool, scan: bool, no_backup: bool) -> Result<()> {
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
            // Even if already initialized, run scan if requested
            if scan {
                run_scan_import(&cwd, &existing).await?;
            }
            emit_recommendation_hints(&cwd);
            // PRD-071 contract: hint to start shaping (or to force reinit).
            let hints_vec = vec![
                Hint::suggestion("Start shaping a PRD")
                    .with_action("forgeplan new prd \"<title>\"".to_string()),
            ];
            print!("{}", hints::render_next_action_line(&hints_vec));
            return Ok(());
        }

        // PROB-068: --force is now strictly additive.
        // - Existing artifact .md bodies are NEVER overwritten.
        // - config.yaml is regenerated (with backup) so stale defaults
        //   from older versions can be refreshed.
        // - LanceDB index is rebuilt from the existing markdown via the
        //   subsequent scan-import flow (when --scan is requested).
        //
        // An auto-backup of the artifact directories is taken unless
        // --no-backup is set. This protects against any future logic
        // bug that might mutate file bodies under --force.

        // [SECURITY] Guard against symlink attack and workspace outside cwd
        ensure_workspace_under_cwd(&existing, &cwd)?;

        if !no_backup {
            let summary = create_force_backup(&existing, &cwd).await?;
            if let Some(s) = summary.as_deref() {
                if non_interactive {
                    println!("  Auto-backup created at {} — use --no-backup to skip", s);
                } else {
                    cliclack::log::success(format!(
                        "Auto-backup created at {} — use --no-backup to skip",
                        s
                    ))?;
                }
            }
        } else if non_interactive {
            eprintln!(
                "  Skipping auto-backup (--no-backup). Existing artifact bodies are preserved."
            );
        }

        // Additive reinit: refresh config + dirs + LanceDB index without
        // touching existing artifact .md files.
        let project_name = cwd
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| DEFAULT_PROJECT_NAME.into());
        refresh_existing_workspace(&existing, &project_name).await?;

        if non_interactive {
            println!(
                "  Reinitialized {}/ in {} (config + indices refreshed, artifact bodies preserved)",
                FORGEPLAN_DIR,
                cwd.display()
            );
            if scan {
                run_scan_import(&cwd, &existing).await?;
            }
            emit_recommendation_hints(&cwd);
            let hints_vec = vec![
                Hint::suggestion("Verify integrity").with_action("forgeplan health".to_string()),
            ];
            print!("{}", hints::render_next_action_line(&hints_vec));
            return Ok(());
        }

        // Interactive: emit a success note via cliclack, run scan if asked,
        // then exit before falling through to the create-from-scratch path.
        ui::print_banner();
        cliclack::intro(style(" forgeplan init --force ").bold())?;
        cliclack::log::success(format!(
            "Reinitialized config + indices at {} (artifact bodies preserved)",
            existing.display()
        ))?;
        if scan {
            run_scan_import(&cwd, &existing).await?;
        }
        cliclack::outro(format!("Done! Next: {}", style("forgeplan health").cyan()))?;
        emit_recommendation_hints(&cwd);
        let hints_vec =
            vec![Hint::suggestion("Verify integrity").with_action("forgeplan health".to_string())];
        print!("{}", hints::render_next_action_line(&hints_vec));
        return Ok(());
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

        if scan {
            let workspace = cwd.join(FORGEPLAN_DIR);
            run_scan_import(&cwd, &workspace).await?;
        }

        emit_recommendation_hints(&cwd);
        // PRD-071 contract: hint at the next step in the workflow.
        let hints_vec = vec![
            Hint::suggestion("Shape your first PRD")
                .with_action("forgeplan new prd \"<title>\"".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&hints_vec));
        return Ok(());
    }

    // ─── Interactive wizard ──────────────────────────────────────

    ui::print_banner();
    cliclack::intro(style(" forgeplan init ").bold())?;

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
        .item(
            "gemini",
            "Gemini CLI",
            ".gemini/settings.json (coming soon)",
        )
        .item(
            "copilot",
            "GitHub Copilot",
            "copilot-instructions.md (coming soon)",
        )
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
            "codex" => {
                cliclack::log::info("Codex: AGENTS.md support coming soon")?;
            }
            "gemini" => {
                cliclack::log::info("Gemini CLI: config support coming soon")?;
            }
            "copilot" => {
                cliclack::log::info("Copilot: instructions support coming soon")?;
            }
            _ => {}
        }
    }

    // Configure hooks
    let configure_hooks = cliclack::confirm("Configure SessionStart hook? (auto health check)")
        .initial_value(true)
        .interact()?;

    let hooks_configured = if configure_hooks {
        let claude_settings_dir = cwd.join(".claude");
        fs::create_dir_all(&claude_settings_dir)?;
        let settings_path = claude_settings_dir.join("settings.json");

        // Read existing or create new
        let mut settings: serde_json::Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        // Add hook if not already present — safe handling of malformed JSON
        let settings_obj = match settings.as_object_mut() {
            Some(obj) => obj,
            None => {
                settings = serde_json::json!({});
                settings.as_object_mut().expect("just created")
            }
        };

        let hooks = settings_obj.entry("hooks").or_insert(serde_json::json!({}));

        // Ensure hooks is an object
        if !hooks.is_object() {
            *hooks = serde_json::json!({});
        }

        let user_prompt = hooks
            .as_object_mut()
            .expect("just ensured object")
            .entry("UserPromptSubmit")
            .or_insert(serde_json::json!([]));

        // Ensure UserPromptSubmit is an array
        if !user_prompt.is_array() {
            *user_prompt = serde_json::json!([]);
        }

        // Check if forgeplan hook already exists
        let already_has = user_prompt
            .as_array()
            .map(|arr| {
                arr.iter().any(|h| {
                    h.get("hooks")
                        .and_then(|h| h.as_array())
                        .map(|hooks| {
                            hooks.iter().any(|hook| {
                                hook.get("command")
                                    .and_then(|c| c.as_str())
                                    .map(|s| s.contains("forgeplan"))
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        if !already_has {
            let hook_entry = serde_json::json!({
                "hooks": [{
                    "type": "command",
                    "command": "forgeplan health --compact --json 2>/dev/null || true",
                    "timeout": 5
                }]
            });
            user_prompt
                .as_array_mut()
                .expect("just ensured array")
                .push(hook_entry);
            fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
            cliclack::log::success("SessionStart hook configured in .claude/settings.json")?;
            true
        } else {
            cliclack::log::info("Forgeplan hook already configured — skipped")?;
            false
        }
    } else {
        false
    };

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
    if hooks_configured {
        summary_lines.push("Hooks           configured".into());
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

    if scan {
        let workspace = cwd.join(FORGEPLAN_DIR);
        run_scan_import(&cwd, &workspace).await?;
    }

    emit_recommendation_hints(&cwd);
    // PRD-071 contract: deterministic Next: line for agents (CLI text contract).
    let hints_vec = vec![
        Hint::suggestion("Shape your first PRD")
            .with_action("forgeplan new prd \"<title>\"".to_string()),
    ];
    print!("{}", hints::render_next_action_line(&hints_vec));

    Ok(())
}

/// Initialize workspace + LanceDB with rollback on failure.
/// If LanceStore::init fails, removes the partially created .forgeplan/ directory.
async fn init_with_rollback(cwd: &std::path::Path, project_name: &str) -> Result<()> {
    let ws = init_workspace(cwd, project_name)?;
    if let Err(e) = LanceStore::init(&ws).await {
        // Rollback: remove partially created workspace
        let _ = tokio::fs::remove_dir_all(&ws).await;
        return Err(e);
    }
    Ok(())
}

/// PROB-068: assert workspace is safe to operate on under --force without
/// any destructive action. Mirrors the historical symlink/escape guard
/// from `safe_remove_workspace`, but never deletes — the additive
/// `--force` flow only refreshes config + indices.
fn ensure_workspace_under_cwd(workspace: &std::path::Path, cwd: &std::path::Path) -> Result<()> {
    let canonical_ws = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let canonical_cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    if !canonical_ws.starts_with(&canonical_cwd) {
        anyhow::bail!(
            "Workspace {} is outside current directory, refusing --force",
            workspace.display()
        );
    }

    let meta = fs::symlink_metadata(workspace)?;
    if meta.file_type().is_symlink() {
        anyhow::bail!(
            "Workspace {} is a symlink, refusing --force for safety",
            workspace.display()
        );
    }
    Ok(())
}

/// PROB-068 Option C: snapshot artifact directories into
/// `.forgeplan-backup-<UTC-timestamp>/` before any `--force` refresh.
///
/// Returns `Ok(Some(path))` with the backup directory name (relative to
/// `cwd`) when artifacts were found and copied. Returns `Ok(None)` when
/// the workspace had no artifact files worth backing up — there is
/// nothing to lose so we skip the noise.
///
/// Failures (disk full, permissions, etc.) are returned as `Err` so
/// `--force` aborts before doing anything else. The user can then free
/// space, fix permissions, or pass `--no-backup` if they've already
/// exported.
async fn create_force_backup(
    workspace: &std::path::Path,
    cwd: &std::path::Path,
) -> Result<Option<String>> {
    // Count existing markdown bodies — skip the backup if there's
    // nothing to protect.
    let mut artifact_count: usize = 0;
    for dir_name in ARTIFACT_DIRS {
        let dir = workspace.join(dir_name);
        if !dir.exists() {
            continue;
        }
        let mut rd = match tokio::fs::read_dir(&dir).await {
            Ok(r) => r,
            Err(_) => continue,
        };
        while let Some(entry) = rd.next_entry().await.transpose() {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".md") {
                artifact_count += 1;
            }
        }
    }
    if artifact_count == 0 {
        return Ok(None);
    }

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_name = format!(".forgeplan-backup-{}", timestamp);
    let backup_root = cwd.join(&backup_name);
    // Defensive: refuse to clobber an existing path with the same name.
    if backup_root.exists() {
        anyhow::bail!(
            "Backup target {} already exists — refusing to overwrite. Move or remove it.",
            backup_root.display()
        );
    }
    tokio::fs::create_dir_all(&backup_root).await?;

    for dir_name in ARTIFACT_DIRS {
        let src = workspace.join(dir_name);
        if !src.exists() {
            continue;
        }
        let dst = backup_root.join(dir_name);
        copy_dir_recursive(&src, &dst).await?;
    }

    Ok(Some(backup_name))
}

/// Recursively copy a directory tree — used by `create_force_backup`.
/// Only walks regular files / directories; symlinks are followed via
/// the standard `fs::copy` semantics (target file contents are copied).
async fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    tokio::fs::create_dir_all(dst).await?;
    let mut rd = tokio::fs::read_dir(src).await?;
    while let Some(entry) = rd.next_entry().await? {
        let entry_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);
        let ft = entry.file_type().await?;
        if ft.is_dir() {
            Box::pin(copy_dir_recursive(&entry_path, &dst_path)).await?;
        } else if ft.is_file() {
            tokio::fs::copy(&entry_path, &dst_path).await?;
        }
        // Skip symlinks / specials; backup is best-effort for those.
    }
    Ok(())
}

/// PROB-068 Option A: additive refresh of an existing workspace.
///
/// - Ensures every `ARTIFACT_DIRS` entry exists (idempotent).
/// - Rewrites `config.yaml` so users on older versions pick up new
///   commented defaults. The previous config (if any) is moved aside
///   as `config.yaml.bak-<timestamp>` so a manual diff is possible.
/// - Reinitializes the LanceDB index in place — LanceStore::init is
///   idempotent and will pick up existing rows on subsequent
///   scan-import calls.
///
/// CRITICALLY: artifact .md files are never touched. Bodies, links,
/// custom frontmatter, agent stamps — all preserved.
async fn refresh_existing_workspace(workspace: &std::path::Path, project_name: &str) -> Result<()> {
    // Ensure every artifact subdir exists.
    for dir_name in ARTIFACT_DIRS {
        let dir = workspace.join(dir_name);
        tokio::fs::create_dir_all(&dir).await?;
    }

    // Rewrite config.yaml; move the old one aside so the user can diff.
    let config_path = workspace.join("config.yaml");
    if config_path.exists() {
        let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let backup_path = workspace.join(format!("config.yaml.bak-{}", ts));
        let _ = tokio::fs::rename(&config_path, &backup_path).await;
    }
    let config = Config {
        project_name: project_name.to_string(),
        ..Config::default()
    };
    let yaml = serde_yaml::to_string(&config)?;
    tokio::fs::write(&config_path, yaml).await?;

    // Reinit LanceDB index. LanceStore::init is idempotent for an
    // existing dataset, so this is safe to run repeatedly.
    LanceStore::init(workspace).await?;
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

/// Bundled minimal playbook descriptors (PRD-072 FR-6, PRD-067 AC-3/AC-4/AC-5).
///
/// Wave 2 ships these descriptors so the recommendation engine can emit
/// `Next:` hints from the very first `forgeplan init` invocation; the full
/// canonical YAML files arrive in Wave 3 (`marketplace/playbooks/*.yaml`).
///
/// `KnownPlaybook` and `TriggeredBy` are `#[non_exhaustive]` — direct
/// struct-literal construction is forbidden outside `forgeplan-core`, so we
/// build them via `serde_json::from_value`, which is the supported escape
/// hatch for non-exhaustive types.
fn bundled_known_playbooks() -> Vec<KnownPlaybook> {
    let raw = serde_json::json!([
        {
            "name": "greenfield-kickoff",
            "source_pack": "forgeplan",
            "triggered_by": { "empty_repo": true, "has_git": true },
            "requires_plugins": ["forgeplan"]
        },
        {
            "name": "brownfield-docs",
            "source_pack": "brownfield-docs-pack",
            "triggered_by": { "has_obsidian": true },
            "requires_plugins": ["forgeplan"]
        },
        {
            "name": "brownfield-code",
            "source_pack": "forgeplan",
            "triggered_by": {
                "has_git": true,
                "commit_count_min": 100,
                "has_docs": false
            },
            "requires_plugins": ["c4-architecture", "forgeplan"]
        }
    ]);
    serde_json::from_value(raw).expect("bundled playbook descriptors are well-formed")
}

/// Discover known playbooks from disk (workspace-local + installed plugin packs)
/// and merge with bundled descriptors. Discovery failures are non-fatal —
/// the bundled list is always returned.
fn discover_known_playbooks(workspace_root: &Path) -> Vec<KnownPlaybook> {
    let mut found = bundled_known_playbooks();
    let mut seen: std::collections::HashSet<String> =
        found.iter().map(|p| p.name.clone()).collect();

    let mut search_dirs: Vec<PathBuf> = Vec::new();
    search_dirs.push(workspace_root.join("playbooks"));
    search_dirs.push(workspace_root.join(".forgeplan").join("playbooks"));
    if let Ok(home) = std::env::var("HOME") {
        let plugins_root = PathBuf::from(home).join(".claude").join("plugins");
        if let Ok(entries) = std::fs::read_dir(&plugins_root) {
            for entry in entries.flatten() {
                let pb_dir = entry.path().join("playbooks");
                if pb_dir.is_dir() {
                    search_dirs.push(pb_dir);
                }
            }
        }
    }

    for dir in search_dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                continue;
            }
            if let Some(pb) = parse_known_playbook(&path)
                && seen.insert(playbook_name(&pb).to_string())
            {
                found.push(pb);
            }
        }
    }

    found
}

/// Extract the `name` from a `KnownPlaybook` without requiring direct field
/// access (the struct is `#[non_exhaustive]`). Round-trips via `serde_json`.
fn playbook_name(pb: &KnownPlaybook) -> String {
    serde_json::to_value(pb)
        .ok()
        .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(str::to_string))
        .unwrap_or_default()
}

/// Best-effort YAML parse of a playbook descriptor. Reads the YAML, converts
/// to a JSON `Value`, then deserializes through `serde_json` (the supported
/// route for `#[non_exhaustive]` targets).
fn parse_known_playbook(path: &Path) -> Option<KnownPlaybook> {
    let raw_text = std::fs::read_to_string(path).ok()?;
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&raw_text).ok()?;
    let json_value = serde_json::to_value(yaml_value).ok()?;
    // Reject documents that don't even carry a `name` so we don't pollute the
    // recommendation list with empty-named entries.
    json_value.get("name")?.as_str()?;
    serde_json::from_value::<KnownPlaybook>(json_value).ok()
}

/// Emit playbook recommendation hints to stderr after workspace creation.
///
/// Honours `FORGEPLAN_HINTS=0` (PRD-067 AC-7) and stderr TTY status so
/// machine-readable consumers (CI, piped agents) are not polluted. Any
/// signal/plugin detection failure is logged but never propagated.
fn emit_recommendation_hints(workspace_root: &Path) {
    let env_flag = std::env::var("FORGEPLAN_HINTS").ok();
    if env_flag.as_deref() == Some("0") {
        return;
    }
    // When `FORGEPLAN_HINTS=1` is explicitly set, bypass the TTY guard so
    // CI / agentic pipelines that read stderr can opt in to hints. Default
    // (env unset) keeps the no-TTY suppression so piped consumers stay quiet.
    let force = env_flag.as_deref() == Some("1");
    if !force && !std::io::stderr().is_terminal() {
        return;
    }

    let signals = match detect_signals(workspace_root) {
        Ok(s) => s,
        Err(err) => {
            eprintln!(
                "warning: skipping playbook recommendations — signal detection failed: {err}"
            );
            return;
        }
    };
    let installed = detect_plugins(&extended_registry());
    let known = discover_known_playbooks(workspace_root);
    let recs = build_recommendations(&signals, &installed, &known);
    let formatted = format_recommendations(&recs);
    if !formatted.is_empty() {
        eprintln!("{}", formatted);
    }
}

/// Run scan-import after init to discover and import existing docs.
async fn run_scan_import(project_root: &Path, workspace: &Path) -> Result<()> {
    println!(
        "\n  {} Scanning for existing documents...",
        style("◉").cyan()
    );

    let store = LanceStore::init(workspace).await?;
    let options = ScanImportOptions::default();

    // PRD-058 FR-001: ADR-003-compliant variant writes markdown projections.
    let result = scan_and_import_to_workspace(project_root, workspace, &store, &options).await?;

    if result.total_found == 0 {
        println!("  No documents found to import.");
        return Ok(());
    }

    let mut by_kind: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for entry in &result.entries {
        if entry.status == ImportStatus::Imported {
            let kind = entry
                .detected_kind
                .as_ref()
                .map(|k| k.template_key().to_uppercase())
                .unwrap_or_else(|| "???".to_string());
            *by_kind.entry(kind).or_insert(0) += 1;
        }
    }

    let kind_summary: Vec<String> = by_kind
        .iter()
        .map(|(k, v)| format!("{} {}", v, k))
        .collect();

    println!(
        "  Imported {} artifact(s): {}",
        style(result.imported).green().bold(),
        kind_summary.join(", ")
    );

    if result.skipped > 0 {
        println!(
            "  Skipped {} (already exist)",
            style(result.skipped).yellow()
        );
    }
    if result.unknown > 0 {
        println!(
            "  {} unknown file(s) — run {} for details",
            style(result.unknown).dim(),
            style("forgeplan scan-import --dry-run").cyan()
        );
    }

    // R2 audit rust-pro HIGH: surface warnings from core (unknown
    // frontmatter status, projection write failure). PRD-058 R-2 fail-
    // loud.
    let warnings_total: usize = result.entries.iter().map(|e| e.warnings.len()).sum();
    if warnings_total > 0 {
        println!(
            "  {} {} warning(s) — run {} to inspect",
            style("⚠").yellow(),
            style(warnings_total).yellow().bold(),
            style("forgeplan scan-import").cyan()
        );
    }

    Ok(())
}
