use forgeplan_core::artifact::canonical_key::{ArtifactCandidate, resolve_artifact};
use forgeplan_core::changelog::ChangeLogEntry;
use forgeplan_core::db::auxiliary::AuxiliaryStore;
use forgeplan_core::hints::{self, Hint};

use crate::commands::common;

pub async fn run(
    artifact_id: Option<&str>,
    source: Option<&str>,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    let ws = common::find_workspace_path()?;
    let _config = forgeplan_core::workspace::load_config(&ws)?;
    let _permit = forgeplan_core::workspace::lock::acquire_workspace_read_lock(&ws).await?;
    let store = AuxiliaryStore::open_history_existing(&ws).await?;
    let entries = if let Some(selector) = artifact_id {
        let candidates = forgeplan_core::artifact::documents::workspace_candidates(&ws).await?;
        let entries = store.get_change_log(None, None, usize::MAX).await?;
        select_history(entries, &candidates, selector, source, limit)?
    } else {
        store.get_change_log(None, source, limit).await?
    };

    // PRD-071 contract: pick the most recently changed artifact and suggest
    // viewing it. log entries are reverse-chronological (most recent first).
    let mut hints_vec: Vec<Hint> = Vec::new();
    if let Some(first) = entries.first() {
        hints_vec.push(
            Hint::info(format!(
                "Inspect most recent change on {}",
                first.artifact_id
            ))
            .with_action(format!("forgeplan get {}", first.artifact_id)),
        );
    }

    if json {
        let payload = serde_json::json!({
            "entries": entries,
            "_next_action": hints::primary_action(&hints_vec),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if entries.is_empty() {
        println!("No change log entries found.");
        let bootstrap = vec![
            Hint::suggestion("No change history yet — create or edit an artifact")
                .with_action("forgeplan list".to_string()),
        ];
        print!("{}", hints::render_next_action_line(&bootstrap));
        return Ok(());
    }

    // Header
    println!(
        "{:<18} {:<10} {:<9} {:<8} {:<20} {:<10} COMMIT",
        "TIMESTAMP", "ARTIFACT", "ACTION", "FIELD", "CHANGE", "SOURCE"
    );
    println!("{}", "-".repeat(90));

    for entry in &entries {
        // Shorten timestamp: "2026-03-31T10:15:00+00:00" → "2026-03-31 10:15"
        let ts = entry
            .timestamp
            .replace('T', " ")
            .chars()
            .take(16)
            .collect::<String>();

        let field = entry.field.as_deref().unwrap_or("-");

        let change = match (&entry.old_value, &entry.new_value) {
            (Some(old), Some(new)) => {
                let old_short = truncate(old, 8);
                let new_short = truncate(new, 8);
                format!("{}→{}", old_short, new_short)
            }
            (None, Some(new)) => truncate(new, 18),
            (Some(old), None) => format!("(was {})", truncate(old, 14)),
            (None, None) => "-".to_string(),
        };

        let commit = entry.commit_hash.as_deref().unwrap_or("-");

        println!(
            "{:<18} {:<10} {:<9} {:<8} {:<20} {:<10} {}",
            ts, entry.artifact_id, entry.action, field, change, entry.source, commit
        );
    }

    println!();
    println!("{} entries shown", entries.len());
    print!("{}", hints::render_next_action_line(&hints_vec));
    Ok(())
}

/// Resolve against the complete document census, then select by stored identity.
/// A display id alone is not evidence of a historical event's owner.
fn select_history(
    entries: Vec<ChangeLogEntry>,
    candidates: &[ArtifactCandidate],
    selector: &str,
    source: Option<&str>,
    limit: usize,
) -> anyhow::Result<Vec<ChangeLogEntry>> {
    let resolved = resolve_artifact(selector, candidates)?;
    let key = resolved.as_ref()
        .map(|item| item.identity.canonical_key.as_str())
        .unwrap_or(selector.trim());
    // An unkeyed event may use an older alias no longer present in the
    // census or in keyed history. Neither can prove that it belongs elsewhere.
    if entries.iter().any(|entry| entry.canonical_key.is_none()) {
        anyhow::bail!("history identity is indeterminate: unkeyed events cannot be attributed to '{key}'");
    }
    Ok(entries.into_iter()
        .filter(|entry| entry.canonical_key.as_deref() == Some(key))
        .filter(|entry| source.is_none_or(|source| entry.source == source))
        .take(limit)
        .collect())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn candidate(key: &str, id: &str) -> ArtifactCandidate {
        ArtifactCandidate {
            canonical_key: key.into(),
            display_id: id.into(),
            title: key.into(),
            relative_path: PathBuf::from(format!("prds/{key}.md")),
        }
    }

    #[test]
    fn duplicate_alias_errors_and_keyed_histories_stay_separate() {
        let candidates = vec![candidate("prd-alpha", "PRD-001"), candidate("prd-beta", "PRD-001")];
        let entries = vec![
            ChangeLogEntry::new("PRD-001", "update", "cli").with_canonical_key("prd-beta"),
            ChangeLogEntry::new("PRD-001", "create", "cli").with_canonical_key("prd-alpha"),
        ];
        assert!(select_history(entries.clone(), &candidates, "PRD-001", None, 10).is_err());
        let selected = select_history(entries, &candidates, "prd-alpha", None, 10).unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].canonical_key.as_deref(), Some("prd-alpha"));
        assert_eq!(selected[0].action, "create");
    }

    #[test]
    fn deleted_key_remains_queryable_and_unkeyed_history_is_not_guessed() {
        let keyed = ChangeLogEntry::new("PRD-001", "delete", "cli").with_canonical_key("prd-alpha");
        let other = ChangeLogEntry::new("PRD-001", "update", "cli").with_canonical_key("prd-beta");
        let selected = select_history(vec![keyed.clone(), other], &[], "prd-alpha", Some("cli"), 1).unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].action, "delete");
        let unkeyed = ChangeLogEntry::new("PRD-001", "create", "legacy");
        let error = select_history(vec![keyed, unkeyed.clone()], &[], "prd-alpha", Some("cli"), 1).unwrap_err();
        assert!(error.to_string().contains("history identity is indeterminate"));
        let error = select_history(vec![unkeyed], &[], "prd-deleted", None, 10).unwrap_err();
        assert!(error.to_string().contains("history identity is indeterminate"));
        let renamed = vec![candidate("prd-alpha", "PRD-002")];
        let old_alias = ChangeLogEntry::new("PRD-001", "update", "legacy");
        let error = select_history(vec![old_alias], &renamed, "prd-alpha", None, 10).unwrap_err();
        assert!(error.to_string().contains("history identity is indeterminate"));
    }
}
