//! Change log — audit trail of artifact mutations.

/// A single change log entry recording an artifact mutation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangeLogEntry {
    /// RFC 3339 timestamp
    pub timestamp: String,
    /// Which artifact changed — the **display id** (`PRD-002`).
    ///
    /// The audit trail keeps the original description of the event: this is
    /// what a person reads and what the row said before the key existed.
    pub artifact_id: String,
    /// The document's **canonical key** — the identity the entry is about.
    ///
    /// The display id is an alias, and an alias may name two documents: one
    /// in this project, one in another. The key names exactly one, so a
    /// receipt recorded under it says which document it describes. `None`
    /// for an entry written before the key was recorded, and for an entry
    /// about a document that is no longer in the index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_key: Option<String>,
    /// create, update, delete, link, unlink
    pub action: String,
    /// Which field changed (status, body, title, etc.)
    pub field: Option<String>,
    /// Previous value (hash for body)
    pub old_value: Option<String>,
    /// New value (hash for body)
    pub new_value: Option<String>,
    /// cli, file_edit, git_sync, reindex
    pub source: String,
    /// Git commit hash (short, 7 chars) — set when source is git_sync
    pub commit_hash: Option<String>,
}

impl ChangeLogEntry {
    /// Create a new entry with current timestamp.
    pub fn new(artifact_id: &str, action: &str, source: &str) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            artifact_id: artifact_id.to_string(),
            canonical_key: None,
            action: action.to_string(),
            field: None,
            old_value: None,
            new_value: None,
            source: source.to_string(),
            commit_hash: None,
        }
    }

    /// Set the canonical key of the document the entry is about.
    ///
    /// The key travels beside the display id: the id is what a person
    /// reads, the key is what names exactly one document.
    pub fn with_canonical_key(mut self, canonical_key: &str) -> Self {
        self.canonical_key = Some(canonical_key.to_string());
        self
    }

    /// Set the field that changed.
    pub fn with_field(mut self, field: &str) -> Self {
        self.field = Some(field.to_string());
        self
    }

    /// Set old and new values.
    pub fn with_values(mut self, old: Option<&str>, new: Option<&str>) -> Self {
        self.old_value = old.map(|s| s.to_string());
        self.new_value = new.map(|s| s.to_string());
        self
    }

    /// Set git commit hash (short form, 7 chars).
    pub fn with_commit(mut self, hash: &str) -> Self {
        self.commit_hash = Some(hash.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_basic() {
        let entry = ChangeLogEntry::new("PRD-001", "create", "cli");
        assert_eq!(entry.artifact_id, "PRD-001");
        assert_eq!(entry.action, "create");
        assert_eq!(entry.source, "cli");
        assert!(entry.field.is_none());
        assert!(entry.old_value.is_none());
        assert!(!entry.timestamp.is_empty());
    }

    #[test]
    fn builder_chain() {
        let entry = ChangeLogEntry::new("PRD-001", "update", "cli")
            .with_field("status")
            .with_values(Some("draft"), Some("active"));
        assert_eq!(entry.field, Some("status".to_string()));
        assert_eq!(entry.old_value, Some("draft".to_string()));
        assert_eq!(entry.new_value, Some("active".to_string()));
    }

    #[test]
    fn builder_partial_values() {
        let entry = ChangeLogEntry::new("EVID-001", "link", "reindex")
            .with_field("relation")
            .with_values(None, Some("PRD-001:informs"));
        assert!(entry.old_value.is_none());
        assert_eq!(entry.new_value, Some("PRD-001:informs".to_string()));
    }
}
