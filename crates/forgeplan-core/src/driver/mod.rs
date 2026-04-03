//! Storage driver traits for pluggable backends (LanceDB, SQLite, InMemory).
//!
//! Traits follow ISP (Interface Segregation Principle) — each trait has a single
//! bounded context. Backends implement only the traits they support.
//!
//! ## Trait hierarchy
//!
//! - `ArtifactStorage` — CRUD operations on artifacts (REQUIRED)
//! - `RelationStorage` — typed relations between artifacts (REQUIRED)
//! - `SearchStorage`   — keyword search, stale detection, ID generation (REQUIRED)
//! - `VectorStorage`   — embedding storage and vector similarity (OPTIONAL, has defaults)
//! - `FpfStorage`      — FPF knowledge base chunks (OPTIONAL, has defaults)
//!
//! `StorageDriver` = supertrait combining all 5. Blanket-implemented for any type
//! that implements all 5 traits. Existing `dyn StorageDriver` usage is unchanged.

pub mod factory;
pub mod in_memory;
pub mod lance;
pub mod types;

// Re-export key types for convenience
pub use types::*;

use crate::artifact::store::ArtifactSummary;
use crate::db::store::{
    ArtifactFilter, ArtifactRecord, FpfChunk, FpfChunkSummary, NewArtifact, VectorSearchHit,
};

// ── Core traits (REQUIRED for any backend) ──────────────────────────────────

/// Artifact CRUD — create, read, update, delete artifacts and their records.
#[async_trait::async_trait]
pub trait ArtifactStorage: Send + Sync {
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

    /// Get a single artifact by ID as a full record. Returns `None` if not found.
    async fn get_record(&self, id: &str) -> anyhow::Result<Option<ArtifactRecord>>;

    /// List artifacts as full records with optional kind/status filter.
    async fn list_records(
        &self,
        filter: Option<&ArtifactFilter>,
    ) -> anyhow::Result<Vec<ArtifactRecord>>;

    /// Update the body column of an artifact.
    async fn update_body(&self, id: &str, body: &str) -> anyhow::Result<()>;
}

/// Typed relations between artifacts — dependency graph.
#[async_trait::async_trait]
pub trait RelationStorage: Send + Sync {
    /// Add a typed relation between two artifacts. Rejects duplicates.
    async fn add_relation(&self, source: &str, target: &str, relation: &str) -> anyhow::Result<()>;

    /// Remove a specific relation between two artifacts.
    async fn delete_relation(
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

    /// Remove ALL relations where artifact is source or target (cascade on delete).
    async fn delete_relations_for_artifact(&self, id: &str) -> anyhow::Result<()>;
}

/// Keyword search, stale detection, and sequential ID generation.
#[async_trait::async_trait]
pub trait SearchStorage: Send + Sync {
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
}

// ── Optional traits (have default implementations) ──────────────────────────

/// Vector embedding storage and similarity search.
///
/// Backends that don't support vectors get no-op defaults automatically.
#[async_trait::async_trait]
pub trait VectorStorage: Send + Sync {
    /// Whether this backend supports vector similarity search.
    fn supports_vectors(&self) -> bool {
        false
    }

    /// Vector similarity search using a pre-computed embedding.
    async fn vector_search(
        &self,
        _query_embedding: &[f32],
        _limit: usize,
    ) -> anyhow::Result<Vec<VectorSearchHit>> {
        Ok(Vec::new())
    }

    /// Update the embedding column for an artifact.
    async fn update_embedding(&self, _id: &str, _embedding: &[f32]) -> anyhow::Result<()> {
        Ok(())
    }
}

/// FPF (First Principles Framework) knowledge base storage.
///
/// Backends that don't support FPF get no-op defaults automatically.
#[async_trait::async_trait]
pub trait FpfStorage: Send + Sync {
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
    async fn get_fpf_section(&self, _section_id: &str) -> anyhow::Result<Option<FpfChunk>> {
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

// ── Supertrait — backward compatible ────────────────────────────────────────

/// Combined storage driver — any type implementing all 5 traits is a StorageDriver.
///
/// This supertrait exists for backward compatibility with existing code that uses
/// `dyn StorageDriver`. New code should prefer specific trait bounds
/// (e.g., `impl ArtifactStorage`) when only a subset of operations is needed.
pub trait StorageDriver:
    ArtifactStorage + RelationStorage + SearchStorage + VectorStorage + FpfStorage
{
}

/// Blanket implementation — any type that implements all 5 sub-traits IS a StorageDriver.
impl<T> StorageDriver for T where
    T: ArtifactStorage + RelationStorage + SearchStorage + VectorStorage + FpfStorage
{
}

// ── Embedding driver ────────────────────────────────────────────────────────

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

/// No-op embedding driver — returns empty vectors. Used as fallback when
/// semantic-search feature is disabled or no model is configured.
pub struct NoOpEmbedDriver;

impl EmbedDriver for NoOpEmbedDriver {
    fn embed(&mut self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Ok(Vec::new())
    }

    fn embed_batch(&mut self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        Ok(vec![Vec::new(); texts.len()])
    }

    fn dim(&self) -> usize {
        0
    }

    fn model_name(&self) -> &str {
        "noop"
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that StorageDriver is object-safe (can be used as `dyn`).
    #[allow(dead_code)]
    fn assert_storage_driver_object_safe(_: &dyn StorageDriver) {}

    /// Verify that each sub-trait is object-safe.
    #[allow(dead_code)]
    fn assert_artifact_storage_object_safe(_: &dyn ArtifactStorage) {}

    #[allow(dead_code)]
    fn assert_relation_storage_object_safe(_: &dyn RelationStorage) {}

    #[allow(dead_code)]
    fn assert_search_storage_object_safe(_: &dyn SearchStorage) {}

    #[allow(dead_code)]
    fn assert_vector_storage_object_safe(_: &dyn VectorStorage) {}

    #[allow(dead_code)]
    fn assert_fpf_storage_object_safe(_: &dyn FpfStorage) {}

    /// Verify that EmbedDriver is object-safe.
    #[allow(dead_code)]
    fn assert_embed_driver_object_safe(_: &dyn EmbedDriver) {}

    #[test]
    fn noop_embed_driver_works() {
        let mut driver = NoOpEmbedDriver;
        let vec = driver.embed("test").unwrap();
        assert!(vec.is_empty());
        assert_eq!(driver.dim(), 0);
        assert_eq!(driver.model_name(), "noop");

        let batch = driver.embed_batch(&["a", "b"]).unwrap();
        assert_eq!(batch.len(), 2);
    }

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
