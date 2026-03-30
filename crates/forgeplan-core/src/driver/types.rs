use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Entry in the agent memory log (decisions, context, insights).
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    /// When the entry was created.
    pub timestamp: DateTime<Utc>,
    /// Category of the memory entry.
    pub kind: MemoryKind,
    /// Free-form content of the entry.
    pub content: String,
    /// Origin of this entry (e.g. "cli", "mcp", "llm").
    pub source: String,
    /// Optional artifact ID this entry relates to.
    pub artifact_id: Option<String>,
    /// Arbitrary key-value metadata.
    pub metadata: HashMap<String, String>,
}

/// Category of a memory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryKind {
    /// A decision that was made.
    Decision,
    /// Contextual information captured for future reference.
    Context,
    /// An insight derived from analysis.
    Insight,
    /// An action that was taken.
    Action,
}
