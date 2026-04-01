//! Change log — audit trail of artifact mutations.

/// A single change log entry recording an artifact mutation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangeLogEntry {
    /// RFC 3339 timestamp
    pub timestamp: String,
    /// Which artifact changed
    pub artifact_id: String,
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
}

impl ChangeLogEntry {
    /// Create a new entry with current timestamp.
    pub fn new(artifact_id: &str, action: &str, source: &str) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            artifact_id: artifact_id.to_string(),
            action: action.to_string(),
            field: None,
            old_value: None,
            new_value: None,
            source: source.to_string(),
        }
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
}
