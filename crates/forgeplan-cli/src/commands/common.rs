use std::collections::HashSet;
use std::path::PathBuf;

use forgeplan_core::config::types::Config;
use forgeplan_core::db::store::{ArtifactRecord, LanceStore};
use forgeplan_core::session::{Phase, SessionState};
use forgeplan_core::workspace;

/// Open workspace store — shared boilerplate for all commands.
/// Returns (workspace_path, store).
pub async fn open_store() -> anyhow::Result<(PathBuf, LanceStore)> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    // Load + validate config on every command that opens the store.
    // This ensures IntegrityConfig::validate() runs for all code paths
    // (new, score, list, etc. — not just commands that explicitly load config).
    let _config = workspace::load_config(&ws)?;
    let store = LanceStore::open(&ws).await?;
    Ok((ws, store))
}

/// Load workspace config. Validates FpfConfig if present.
pub fn config() -> anyhow::Result<Config> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ found. Run `forgeplan init` first."))?;
    let config = workspace::load_config(&ws)?;
    if let Some(ref fpf) = config.fpf {
        fpf.validate()
            .map_err(|e| anyhow::anyhow!("Invalid fpf config: {e}"))?;
    }
    Ok(config)
}

/// Open workspace store, returning only the store (most common case).
pub async fn store() -> anyhow::Result<LanceStore> {
    let (_, store) = open_store().await?;
    Ok(store)
}

/// Load session state from workspace.
pub fn load_session() -> SessionState {
    let cwd = std::env::current_dir().unwrap_or_default();
    let ws = workspace::find_workspace(&cwd).unwrap_or_default();
    SessionState::load(&ws)
}

/// Save session state to workspace.
pub fn save_session(session: &SessionState) {
    let cwd = std::env::current_dir().unwrap_or_default();
    if let Some(ws) = workspace::find_workspace(&cwd) {
        let _ = session.save(&ws);
    }
}

/// Advance session phase. Prints transition info. Silently skips if not enforced.
pub fn advance_session(to: Phase, artifact: Option<&str>) {
    let mut session = load_session();
    if !session.is_enforced() && session.route_depth.is_none() {
        // No enforcement — still track for visibility but don't block
        session.phase = to;
        if let Some(id) = artifact {
            session.active_artifact = Some(id.to_string());
        }
        save_session(&session);
        return;
    }

    match session.transition(to) {
        Ok(()) => {
            if let Some(id) = artifact {
                session.active_artifact = Some(id.to_string());
            }
            save_session(&session);
        }
        Err(e) => {
            eprintln!("  Session: {e}");
            eprintln!("  Hint: {}", session.next_action_hint());
        }
    }
}

/// Build set of "resolved" artifact IDs: active + deprecated + superseded.
/// Only "draft" (and "stale") artifacts are considered unresolved and can block.
pub fn resolved_ids(records: &[ArtifactRecord]) -> HashSet<String> {
    records
        .iter()
        .filter(|r| r.status == "active" || r.status == "deprecated" || r.status == "superseded")
        .map(|r| r.id.clone())
        .collect()
}

/// Extract a field value from YAML frontmatter in a markdown body.
pub fn extract_frontmatter_field(body: &str, field: &str) -> Option<String> {
    let prefix = format!("{}:", field);
    for line in body.lines() {
        if line == "---" {
            continue;
        }
        if line.starts_with(&prefix) {
            let value = line[prefix.len()..].trim();
            let value = value.trim_matches('"');
            return Some(value.to_string());
        }
    }
    None
}

/// Extract plain text from a markdown body (skip YAML frontmatter).
pub fn extract_plain_text(body: &str) -> String {
    let mut in_frontmatter = false;
    let mut lines = Vec::new();
    for line in body.lines() {
        if line.trim() == "---" {
            in_frontmatter = !in_frontmatter;
            continue;
        }
        if !in_frontmatter {
            lines.push(line);
        }
    }
    lines.join(" ").trim().to_string()
}

/// Load and validate LLM config — fails early with actionable message if not configured.
pub fn require_llm_config() -> anyhow::Result<forgeplan_core::config::types::LlmConfig> {
    let cfg = config()?;
    let llm = cfg
        .llm
        .ok_or_else(|| {
            anyhow::anyhow!(
                "LLM not configured.\n\
                 Add to .forgeplan/config.yaml:\n\
                 llm:\n\
                   provider: gemini\n\
                   api_key_env: GEMINI_API_KEY"
            )
        })?
        .with_env_overrides();
    if llm.resolve_api_key().is_none() {
        anyhow::bail!(
            "API key not found for provider '{}'.\n\
             Set environment variable: {}",
            llm.provider,
            llm.api_key_env.as_deref().unwrap_or("(none configured)")
        );
    }
    Ok(llm)
}

/// Log a change to the change_log table (best-effort, never fails the command).
/// May reference deleted artifacts by design (audit trail).
pub async fn log_change(store: &LanceStore, artifact_id: &str, action: &str, source: &str) {
    let entry = forgeplan_core::changelog::ChangeLogEntry::new(artifact_id, action, source);
    if let Err(e) = store.log_change(&entry).await {
        eprintln!(
            "  Warning: changelog write failed for {}: {}",
            artifact_id, e
        );
    }
}

/// Log a change with field + values (best-effort).
pub async fn log_change_field(
    store: &LanceStore,
    artifact_id: &str,
    action: &str,
    field: &str,
    old_value: Option<&str>,
    new_value: Option<&str>,
    source: &str,
) {
    let entry = forgeplan_core::changelog::ChangeLogEntry::new(artifact_id, action, source)
        .with_field(field)
        .with_values(old_value, new_value);
    if let Err(e) = store.log_change(&entry).await {
        eprintln!(
            "  Warning: changelog write failed for {}: {}",
            artifact_id, e
        );
    }
}

/// Open storage using driver trait (new API — will replace open_store over time).
#[allow(dead_code)]
pub async fn open_driver()
-> anyhow::Result<std::sync::Arc<dyn forgeplan_core::driver::StorageDriver>> {
    let cwd = std::env::current_dir()?;
    let ws = workspace::find_workspace(&cwd)
        .ok_or_else(|| anyhow::anyhow!("No .forgeplan/ workspace found"))?;
    let config = workspace::load_config(&ws)?;
    let storage_config = config.storage.unwrap_or_default();
    forgeplan_core::driver::factory::create_storage(&storage_config, &ws).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_frontmatter_field_basic() {
        let body = "---\nid: \"mem-test\"\ncategory: fact\nstatus: active\n---\n\nHello world";
        assert_eq!(
            extract_frontmatter_field(body, "category"),
            Some("fact".to_string())
        );
        assert_eq!(
            extract_frontmatter_field(body, "id"),
            Some("mem-test".to_string())
        );
        assert_eq!(extract_frontmatter_field(body, "missing"), None);
    }

    #[test]
    fn extract_plain_text_skips_frontmatter() {
        let body = "---\nid: test\nkind: memory\n---\n\nThis is the content.";
        assert_eq!(extract_plain_text(body), "This is the content.");
    }

    #[test]
    fn extract_plain_text_no_frontmatter() {
        let body = "Just plain text here.";
        assert_eq!(extract_plain_text(body), "Just plain text here.");
    }
}
