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
