//! Storage driver traits for pluggable backends (LanceDB, SQLite, InMemory).
//!
//! These traits abstract the storage, embedding, memory, and LLM layers
//! so that implementations can be swapped without changing business logic.

pub mod factory;
pub mod in_memory;
pub mod lance;
pub mod types;

// Re-export key types for convenience
pub use types::*;

use crate::artifact::store::ArtifactSummary;
use crate::db::store::{ArtifactFilter, ArtifactRecord, FpfChunk, FpfChunkSummary, NewArtifact};

use std::path::Path;

/// Core storage abstraction over artifact CRUD, relations, search, scoring, and FPF.
///
/// Every pub async method on `LanceStore` is mirrored here so that any backend
/// (LanceDB, SQLite, in-memory) can fulfil the contract.
#[async_trait::async_trait]
pub trait StorageDriver: Send + Sync {
    // ── Lifecycle ────────────────────────────────────────────────────

    /// Open an existing workspace at `workspace_path`.
    async fn open(workspace_path: &Path) -> anyhow::Result<Self>
    where
        Self: Sized;

    /// Create tables / schema if needed, then open the store.
    async fn init(workspace_path: &Path) -> anyhow::Result<Self>
    where
        Self: Sized;

    // ── Artifact CRUD ────────────────────────────────────────────────

    /// Insert a new artifact, returning its ID.
    async fn create_artifact(&self, artifact: &NewArtifact) -> anyhow::Result<String>;

    /// Get a single artifact by ID as a summary. Returns `None` if not found.
    async fn get_artifact(&self, id: &str) -> anyhow::Result<Option<ArtifactSummary>>;

    /// List artifacts with optional kind/status filter.
    async fn list_artifacts(
        &self,
        filter: Option<&ArtifactFilter>,
    ) -> anyhow::Result<Vec<ArtifactSummary>>;

    /// Update artifact metadata (status, title). Always bumps `updated_at`.
    async fn update_artifact(
        &self,
        id: &str,
        status: Option<&str>,
        title: Option<&str>,
    ) -> anyhow::Result<()>;

    /// Update `r_eff_score` for an artifact.
    async fn update_r_eff_score(&self, id: &str, score: f64) -> anyhow::Result<()>;

    /// Delete an artifact by ID.
    async fn delete_artifact(&self, id: &str) -> anyhow::Result<()>;

    // ── Full records ─────────────────────────────────────────────────

    /// Get a single artifact by ID as a full record. Returns `None` if not found.
    async fn get_record(&self, id: &str) -> anyhow::Result<Option<ArtifactRecord>>;

    /// List artifacts as full records with optional kind/status filter.
    async fn list_records(
        &self,
        filter: Option<&ArtifactFilter>,
    ) -> anyhow::Result<Vec<ArtifactRecord>>;

    /// Update the body column of an artifact.
    async fn update_body(&self, id: &str, body: &str) -> anyhow::Result<()>;

    // ── Relations ────────────────────────────────────────────────────

    /// Add a typed relation between two artifacts. Rejects duplicates.
    async fn add_relation(
        &self,
        source: &str,
        target: &str,
        relation: &str,
    ) -> anyhow::Result<()>;

    /// Get outgoing relations for an artifact (source -> targets).
    /// Returns `Vec<(target_id, relation_type)>`.
    async fn get_relations(&self, id: &str) -> anyhow::Result<Vec<(String, String)>>;

    /// Get incoming relations where this artifact is the target.
    /// Returns `Vec<(source_id, relation_type)>`.
    async fn get_incoming_relations(&self, id: &str) -> anyhow::Result<Vec<(String, String)>>;

    /// Get all relations across all artifacts.
    /// Returns `Vec<(source_id, target_id, relation_type)>`.
    async fn get_all_relations(&self) -> anyhow::Result<Vec<(String, String, String)>>;

    // ── Search ───────────────────────────────────────────────────────

    /// Search artifacts by body/title content (case-insensitive substring).
    async fn search_body(
        &self,
        query: &str,
        kind_filter: Option<&str>,
    ) -> anyhow::Result<Vec<ArtifactRecord>>;

    /// Find artifacts whose `valid_until` has expired.
    async fn find_stale(&self) -> anyhow::Result<Vec<ArtifactRecord>>;

    /// Compute the next sequential ID for a given kind prefix (e.g. "PRD" -> "PRD-003").
    async fn next_id(&self, kind_prefix: &str) -> anyhow::Result<String>;

    // ── Vectors (optional) ───────────────────────────────────────────

    /// Whether this backend supports vector similarity search.
    fn supports_vectors(&self) -> bool {
        false
    }

    /// Vector similarity search using a pre-computed embedding.
    async fn vector_search(
        &self,
        _query_embedding: &[f32],
        _limit: usize,
    ) -> anyhow::Result<Vec<ArtifactRecord>> {
        Ok(Vec::new())
    }

    /// Update the embedding column for an artifact.
    async fn update_embedding(&self, _id: &str, _embedding: &[f32]) -> anyhow::Result<()> {
        Ok(())
    }

    // ── FPF Knowledge Base (optional) ────────────────────────────────

    /// Whether FPF knowledge base is available.
    fn has_fpf(&self) -> bool {
        false
    }

    /// Insert FPF chunks in batch. Returns number inserted.
    async fn insert_fpf_chunks(&self, _chunks: &[FpfChunk]) -> anyhow::Result<usize> {
        Ok(0)
    }

    /// Search FPF spec by keyword.
    async fn search_fpf(&self, _query: &str, _limit: usize) -> anyhow::Result<Vec<FpfChunk>> {
        Ok(Vec::new())
    }

    /// Get a specific FPF section by section_id.
    async fn get_fpf_section(
        &self,
        _section_id: &str,
    ) -> anyhow::Result<Option<FpfChunk>> {
        Ok(None)
    }

    /// List all FPF sections (without body content).
    async fn list_fpf_sections(&self) -> anyhow::Result<Vec<FpfChunkSummary>> {
        Ok(Vec::new())
    }

    /// Delete all FPF chunks (for re-ingestion).
    async fn clear_fpf(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Embedding driver — wraps a text embedding model.
pub trait EmbedDriver: Send {
    /// Embed a single text string into a vector.
    fn embed(&mut self, text: &str) -> anyhow::Result<Vec<f32>>;

    /// Embed multiple texts in a single batch.
    fn embed_batch(&mut self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>>;

    /// Dimensionality of the output vectors.
    fn dim(&self) -> usize;

    /// Name of the underlying model.
    fn model_name(&self) -> &str;
}

/// Agent memory driver — log and recall contextual entries across sessions.
#[async_trait::async_trait]
pub trait MemoryDriver: Send + Sync {
    /// Append a memory entry.
    async fn log(&self, entry: MemoryEntry) -> anyhow::Result<()>;

    /// Recall entries matching a query, up to `limit`.
    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>>;

    /// Get the most recent entries.
    async fn recent(&self, count: usize) -> anyhow::Result<Vec<MemoryEntry>>;
}

/// LLM driver — unified text generation interface.
#[async_trait::async_trait]
pub trait LlmDriver: Send + Sync {
    /// Generate text from a prompt with an optional system message.
    async fn generate(&self, prompt: &str, system: Option<&str>) -> anyhow::Result<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that StorageDriver is object-safe (can be used as `dyn`).
    #[allow(dead_code)]
    fn assert_storage_driver_object_safe(_: &dyn StorageDriver) {}

    /// Verify that EmbedDriver is object-safe.
    #[allow(dead_code)]
    fn assert_embed_driver_object_safe(_: &dyn EmbedDriver) {}

    /// Verify that MemoryDriver is object-safe.
    #[allow(dead_code)]
    fn assert_memory_driver_object_safe(_: &dyn MemoryDriver) {}

    /// Verify that LlmDriver is object-safe.
    #[allow(dead_code)]
    fn assert_llm_driver_object_safe(_: &dyn LlmDriver) {}

    #[test]
    fn memory_kind_equality() {
        assert_eq!(MemoryKind::Decision, MemoryKind::Decision);
        assert_ne!(MemoryKind::Decision, MemoryKind::Context);
    }

    #[test]
    fn memory_entry_debug() {
        let entry = MemoryEntry {
            timestamp: chrono::Utc::now(),
            kind: MemoryKind::Insight,
            content: "test".to_string(),
            source: "cli".to_string(),
            artifact_id: None,
            metadata: std::collections::HashMap::new(),
        };
        let debug = format!("{:?}", entry);
        assert!(debug.contains("Insight"));
    }
}
