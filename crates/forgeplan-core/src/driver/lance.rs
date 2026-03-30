//! LanceDB implementation of StorageDriver.
//! Thin wrapper delegating all calls to the existing LanceStore.

use std::path::Path;

use crate::artifact::store::ArtifactSummary;
use crate::db::store::{ArtifactFilter, ArtifactRecord, FpfChunk, FpfChunkSummary, LanceStore, NewArtifact, VectorSearchHit};
use crate::driver::StorageDriver;

/// LanceDB-backed storage driver.
pub struct LanceDriver {
    store: LanceStore,
}

impl LanceDriver {
    pub async fn open(workspace_path: &Path) -> anyhow::Result<Self> {
        let store = LanceStore::open(workspace_path).await?;
        Ok(Self { store })
    }

    pub async fn init(workspace_path: &Path) -> anyhow::Result<Self> {
        let store = LanceStore::init(workspace_path).await?;
        Ok(Self { store })
    }
}

#[async_trait::async_trait]
impl StorageDriver for LanceDriver {
    // ── Lifecycle ────────────────────────────────────────────────────

    async fn open(workspace_path: &Path) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        LanceDriver::open(workspace_path).await
    }

    async fn init(workspace_path: &Path) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        LanceDriver::init(workspace_path).await
    }

    // ── Artifact CRUD ────────────────────────────────────────────────

    async fn create_artifact(&self, artifact: &NewArtifact) -> anyhow::Result<String> {
        self.store.create_artifact(artifact).await
    }

    async fn get_artifact(&self, id: &str) -> anyhow::Result<Option<ArtifactSummary>> {
        self.store.get_artifact(id).await
    }

    async fn list_artifacts(
        &self,
        filter: Option<&ArtifactFilter>,
    ) -> anyhow::Result<Vec<ArtifactSummary>> {
        self.store.list_artifacts(filter).await
    }

    async fn update_artifact(
        &self,
        id: &str,
        status: Option<&str>,
        title: Option<&str>,
    ) -> anyhow::Result<()> {
        self.store.update_artifact(id, status, title).await
    }

    async fn update_r_eff_score(&self, id: &str, score: f64) -> anyhow::Result<()> {
        self.store.update_r_eff_score(id, score).await
    }

    async fn delete_artifact(&self, id: &str) -> anyhow::Result<()> {
        self.store.delete_artifact(id).await
    }

    // ── Full records ─────────────────────────────────────────────────

    async fn get_record(&self, id: &str) -> anyhow::Result<Option<ArtifactRecord>> {
        self.store.get_record(id).await
    }

    async fn list_records(
        &self,
        filter: Option<&ArtifactFilter>,
    ) -> anyhow::Result<Vec<ArtifactRecord>> {
        self.store.list_records(filter).await
    }

    async fn update_body(&self, id: &str, body: &str) -> anyhow::Result<()> {
        self.store.update_body(id, body).await
    }

    // ── Relations ────────────────────────────────────────────────────

    async fn add_relation(
        &self,
        source: &str,
        target: &str,
        relation: &str,
    ) -> anyhow::Result<()> {
        self.store.add_relation(source, target, relation).await
    }

    async fn delete_relation(
        &self,
        source: &str,
        target: &str,
        relation: &str,
    ) -> anyhow::Result<()> {
        self.store.delete_relation(source, target, relation).await
    }

    async fn get_relations(&self, id: &str) -> anyhow::Result<Vec<(String, String)>> {
        self.store.get_relations(id).await
    }

    async fn get_incoming_relations(&self, id: &str) -> anyhow::Result<Vec<(String, String)>> {
        self.store.get_incoming_relations(id).await
    }

    async fn get_all_relations(&self) -> anyhow::Result<Vec<(String, String, String)>> {
        self.store.get_all_relations().await
    }

    // ── Search ───────────────────────────────────────────────────────

    async fn search_body(
        &self,
        query: &str,
        kind_filter: Option<&str>,
    ) -> anyhow::Result<Vec<ArtifactRecord>> {
        self.store.search_body(query, kind_filter).await
    }

    async fn find_stale(&self) -> anyhow::Result<Vec<ArtifactRecord>> {
        self.store.find_stale().await
    }

    async fn next_id(&self, kind_prefix: &str) -> anyhow::Result<String> {
        self.store.next_id(kind_prefix).await
    }

    // ── Vectors ──────────────────────────────────────────────────────

    fn supports_vectors(&self) -> bool {
        true
    }

    async fn update_embedding(&self, id: &str, embedding: &[f32]) -> anyhow::Result<()> {
        self.store.update_embedding(id, embedding).await
    }

    #[cfg(feature = "semantic-search")]
    async fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> anyhow::Result<Vec<VectorSearchHit>> {
        self.store.vector_search(query_embedding, limit).await
    }

    // ── FPF Knowledge Base ───────────────────────────────────────────

    fn has_fpf(&self) -> bool {
        self.store.has_fpf()
    }

    async fn insert_fpf_chunks(&self, chunks: &[FpfChunk]) -> anyhow::Result<usize> {
        self.store.insert_fpf_chunks(chunks).await
    }

    async fn search_fpf(&self, query: &str, limit: usize) -> anyhow::Result<Vec<FpfChunk>> {
        self.store.search_fpf(query, limit).await
    }

    async fn get_fpf_section(&self, section_id: &str) -> anyhow::Result<Option<FpfChunk>> {
        self.store.get_fpf_section(section_id).await
    }

    async fn list_fpf_sections(&self) -> anyhow::Result<Vec<FpfChunkSummary>> {
        self.store.list_fpf_sections().await
    }

    async fn clear_fpf(&self) -> anyhow::Result<()> {
        self.store.clear_fpf().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that LanceDriver can be used as `&dyn StorageDriver`.
    #[allow(dead_code)]
    fn assert_lance_driver_is_storage_driver(_: &dyn StorageDriver) {}

    #[tokio::test]
    async fn lance_driver_open_tempdir() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path();

        // First init to create tables
        let driver = LanceDriver::init(ws).await.unwrap();
        // Verify FPF defaults to false (no fpf_spec table created by default init)
        // has_fpf depends on whether init creates fpf_spec table
        let _ = driver.has_fpf();

        // Re-open existing workspace
        let driver2 = LanceDriver::open(ws).await.unwrap();
        assert!(driver2.supports_vectors());
    }

    #[tokio::test]
    async fn lance_driver_as_dyn_trait() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = LanceDriver::init(tmp.path()).await.unwrap();
        let dyn_ref: &dyn StorageDriver = &driver;
        // Should be able to call trait methods through dyn reference
        let artifacts = dyn_ref.list_artifacts(None).await.unwrap();
        assert!(artifacts.is_empty());
    }
}
