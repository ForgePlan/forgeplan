//! LanceDB implementation of storage traits.
//! Thin wrapper delegating all calls to the existing LanceStore.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::artifact::store::ArtifactSummary;
#[cfg(feature = "semantic-search")]
use crate::db::store::VectorSearchHit;
use crate::db::store::{
    ArtifactFilter, ArtifactRecord, FpfChunk, FpfChunkSummary, LanceStore, NewArtifact,
};
use crate::driver::{ArtifactStorage, FpfStorage, RelationStorage, SearchStorage, VectorStorage};
use crate::workspace::lock::{WorkspaceLock, WorkspaceReadPermit};

/// LanceDB-backed storage driver.
pub struct LanceDriver {
    workspace: PathBuf,
    store: RwLock<Arc<LanceStore>>,
    has_fpf: bool,
}

#[derive(Clone, Copy)]
enum SelectorUse {
    Required,
    Optional,
    NewKey,
}

impl LanceDriver {
    pub async fn open(workspace_path: &Path) -> anyhow::Result<Self> {
        let store = LanceStore::open(workspace_path).await?;
        Ok(Self::from_store(workspace_path, store))
    }

    pub async fn init(workspace_path: &Path) -> anyhow::Result<Self> {
        #[cfg(windows)]
        anyhow::bail!("legacy identity initialization is unavailable on Windows");
        let lock = crate::workspace::acquire_workspace_lock(workspace_path).await?;
        let lance = workspace_path.join("lance");
        let marked = crate::db::migrate::read_identity_v5_marker(&lance)?.is_some();
        let legacy_exists = lance.join("artifacts.lance").try_exists()?;
        let store = if marked || legacy_exists {
            LanceStore::init(workspace_path, &lock).await?
        } else {
            LanceStore::init_new(workspace_path, &lock).await?
        };
        Ok(Self::from_store(workspace_path, store))
    }

    fn from_store(workspace_path: &Path, store: LanceStore) -> Self {
        Self {
            workspace: workspace_path.to_path_buf(),
            has_fpf: store.has_fpf(),
            store: RwLock::new(Arc::new(store)),
        }
    }

    async fn read_store(&self, permit: &dyn WorkspaceReadPermit) -> anyhow::Result<Arc<LanceStore>> {
        permit.matches_workspace(&self.workspace)?;
        let mut cached = self.store.write().await;
        let current = cached.clone();
        let marker = crate::db::migrate::read_identity_v5_marker(&self.workspace.join("lance"))?;
        if marker.is_none() && current.uses_identity_v5() {
            anyhow::bail!("identity-v5 readiness marker disappeared from an open store");
        }
        if current.identity_generation_changed(permit)? {
            let fresh = Arc::new(LanceStore::open(&self.workspace).await?);
            fresh.refresh(permit).await?;
            *cached = fresh.clone();
            return Ok(fresh);
        }
        current.refresh(permit).await?;
        Ok(current)
    }

    async fn write_store(&self, lock: &WorkspaceLock, selectors: &[(&str, SelectorUse)]) -> anyhow::Result<Arc<LanceStore>> {
        lock.matches_workspace(&self.workspace)?;
        let before = self.check_selectors(selectors).await?;
        let store = Arc::new(LanceStore::open_for_write(&self.workspace, lock).await?);
        if before != self.check_selectors(selectors).await? {
            anyhow::bail!("artifact identity changed during write preparation");
        }
        for ((selector, usage), expected) in selectors.iter().zip(&before) {
            if selector.contains('@') && matches!(usage, SelectorUse::Optional) {
                continue;
            }
            if &store.resolve_canonical_key(lock, selector).await? != expected {
                anyhow::bail!("reopened index does not match the selected artifact identity");
            }
        }
        *self.store.write().await = store.clone();
        Ok(store)
    }

    async fn check_selectors(&self, selectors: &[(&str, SelectorUse)]) -> anyhow::Result<Vec<Option<String>>> {
        let candidates = crate::artifact::documents::workspace_candidates(&self.workspace).await?;
        let mut keys = Vec::with_capacity(selectors.len());
        for (selector, usage) in selectors {
            if selector.contains('@') && matches!(usage, SelectorUse::Optional) {
                keys.push(Some((*selector).to_string()));
                continue;
            }
            let found = crate::artifact::canonical_key::resolve_artifact(selector, &candidates)?
                .map(|item| item.identity.canonical_key);
            match (usage, &found) {
                (SelectorUse::Required, None) => anyhow::bail!("Artifact '{selector}' not found"),
                (SelectorUse::NewKey, Some(_)) => anyhow::bail!("Artifact key '{selector}' already exists"),
                _ => keys.push(found),
            }
        }
        Ok(keys)
    }
}

// ── ArtifactStorage ─────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl ArtifactStorage for LanceDriver {
    async fn create_artifact(&self, guard: &WorkspaceLock, artifact: &NewArtifact) -> anyhow::Result<String> {
        let key = crate::db::store::canonical_key_of(&artifact.canonical_key, &artifact.id, &artifact.body);
        let store = self.write_store(guard, &[(&key, SelectorUse::NewKey)]).await?;
        store.create_artifact(guard, artifact).await
    }

    async fn get_artifact(&self, permit: &dyn WorkspaceReadPermit, id: &str) -> anyhow::Result<Option<ArtifactSummary>> {
        self.read_store(permit).await?.get_artifact(permit, id).await
    }

    async fn list_artifacts(
        &self,
        permit: &dyn WorkspaceReadPermit,
        filter: Option<&ArtifactFilter>,
    ) -> anyhow::Result<Vec<ArtifactSummary>> {
        self.read_store(permit).await?.list_artifacts(permit, filter).await
    }

    async fn update_artifact(
        &self,
        guard: &WorkspaceLock,
        id: &str,
        status: Option<&str>,
        title: Option<&str>,
    ) -> anyhow::Result<()> {
        let store = self.write_store(guard, &[(id, SelectorUse::Required)]).await?;
        store.update_artifact(guard, id, status, title).await
    }

    async fn update_r_eff_score(&self, guard: &WorkspaceLock, id: &str, score: f64) -> anyhow::Result<()> {
        let store = self.write_store(guard, &[(id, SelectorUse::Required)]).await?;
        store.update_r_eff_score(guard, id, score).await
    }

    async fn delete_artifact(&self, guard: &WorkspaceLock, id: &str) -> anyhow::Result<()> {
        let store = self.write_store(guard, &[(id, SelectorUse::Required)]).await?;
        store.delete_artifact(guard, id).await
    }

    async fn get_record(&self, permit: &dyn WorkspaceReadPermit, id: &str) -> anyhow::Result<Option<ArtifactRecord>> {
        self.read_store(permit).await?.get_record(permit, id).await
    }

    async fn list_records(
        &self,
        permit: &dyn WorkspaceReadPermit,
        filter: Option<&ArtifactFilter>,
    ) -> anyhow::Result<Vec<ArtifactRecord>> {
        self.read_store(permit).await?.list_records(permit, filter).await
    }

    async fn update_body(&self, guard: &WorkspaceLock, id: &str, body: &str) -> anyhow::Result<()> {
        let store = self.write_store(guard, &[(id, SelectorUse::Required)]).await?;
        store.update_body(guard, id, body).await
    }
}

// ── RelationStorage ─────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl RelationStorage for LanceDriver {
    async fn add_relation(&self, guard: &WorkspaceLock, source: &str, target: &str, relation: &str) -> anyhow::Result<()> {
        let store = self.write_store(guard, &[(source, SelectorUse::Required), (target, SelectorUse::Optional)]).await?;
        store.add_relation(guard, source, target, relation).await
    }

    async fn delete_relation(
        &self,
        guard: &WorkspaceLock,
        source: &str,
        target: &str,
        relation: &str,
    ) -> anyhow::Result<()> {
        let store = self.write_store(guard, &[(source, SelectorUse::Required), (target, SelectorUse::Optional)]).await?;
        store.delete_relation(guard, source, target, relation).await
    }

    async fn get_relations(&self, permit: &dyn WorkspaceReadPermit, id: &str) -> anyhow::Result<Vec<(String, String)>> {
        self.read_store(permit).await?.get_relations(permit, id).await
    }

    async fn get_incoming_relations(&self, permit: &dyn WorkspaceReadPermit, id: &str) -> anyhow::Result<Vec<(String, String)>> {
        self.read_store(permit).await?.get_incoming_relations(permit, id).await
    }

    async fn get_all_relations(&self, permit: &dyn WorkspaceReadPermit) -> anyhow::Result<Vec<(String, String, String)>> {
        self.read_store(permit).await?.get_all_relations(permit).await
    }

    async fn delete_relations_for_artifact(&self, guard: &WorkspaceLock, id: &str) -> anyhow::Result<()> {
        let store = self.write_store(guard, &[(id, SelectorUse::Required)]).await?;
        store.delete_relations_for_artifact(guard, id).await
    }
}

// ── SearchStorage ───────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl SearchStorage for LanceDriver {
    async fn search_body(
        &self,
        permit: &dyn WorkspaceReadPermit,
        query: &str,
        kind_filter: Option<&str>,
    ) -> anyhow::Result<Vec<ArtifactRecord>> {
        self.read_store(permit).await?.search_body(permit, query, kind_filter).await
    }

    async fn find_stale(&self, permit: &dyn WorkspaceReadPermit) -> anyhow::Result<Vec<ArtifactRecord>> {
        self.read_store(permit).await?.find_stale(permit).await
    }

    async fn next_id(&self, permit: &dyn WorkspaceReadPermit, kind_prefix: &str) -> anyhow::Result<String> {
        self.read_store(permit).await?.next_id(permit, kind_prefix).await
    }
}

// ── VectorStorage ───────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl VectorStorage for LanceDriver {
    fn supports_vectors(&self) -> bool {
        cfg!(feature = "semantic-search")
    }

    // `update_embedding` was removed: it forwarded to
    // `LanceStore::update_embedding` with no binding, incarnation or profile
    // check, and no production caller existed. Publishing a vector goes
    // through the conditional path in the constellation facade.

    #[cfg(feature = "semantic-search")]
    async fn vector_search(
        &self,
        permit: &dyn WorkspaceReadPermit,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<VectorSearchHit>> {
        self.read_store(permit).await?.vector_search(permit, query, limit).await
    }
}

// ── FpfStorage ──────────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl FpfStorage for LanceDriver {
    fn has_fpf(&self) -> bool {
        self.has_fpf
    }

    async fn insert_fpf_chunks(
        &self,
        chunks: &[FpfChunk],
        embeddings: Option<&[Vec<f32>]>,
    ) -> anyhow::Result<usize> {
        self.store.read().await.insert_fpf_chunks(chunks, embeddings).await
    }

    async fn search_fpf(&self, query: &str, limit: usize) -> anyhow::Result<Vec<FpfChunk>> {
        self.store.read().await.search_fpf(query, limit).await
    }

    async fn search_fpf_by_vector(
        &self,
        query_vec: &[f32],
        limit: usize,
    ) -> anyhow::Result<Vec<FpfChunk>> {
        self.store.read().await.search_fpf_by_vector(query_vec, limit).await
    }

    async fn get_fpf_section(&self, section_id: &str) -> anyhow::Result<Option<FpfChunk>> {
        self.store.read().await.get_fpf_section(section_id).await
    }

    async fn list_fpf_sections(&self) -> anyhow::Result<Vec<FpfChunkSummary>> {
        self.store.read().await.list_fpf_sections().await
    }

    async fn clear_fpf(&self) -> anyhow::Result<()> {
        self.store.read().await.clear_fpf().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::StorageDriver;

    /// Verify that LanceDriver can be used as `&dyn StorageDriver`.
    #[allow(dead_code)]
    fn assert_lance_driver_is_storage_driver(_: &dyn StorageDriver) {}

    /// Verify sub-traits are object-safe.
    #[allow(dead_code)]
    fn assert_sub_traits_object_safe(
        _a: &dyn ArtifactStorage,
        _r: &dyn RelationStorage,
        _s: &dyn SearchStorage,
        _v: &dyn VectorStorage,
        _f: &dyn FpfStorage,
    ) {
    }

    #[tokio::test]
    async fn lance_driver_open_tempdir() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path();

        // First init to create tables
        let driver = LanceDriver::init(ws).await.unwrap();
        let _ = driver.has_fpf();

        // Re-open existing workspace
        let driver2 = LanceDriver::open(ws).await.unwrap();
        // supports_vectors() is true only when semantic-search feature is enabled
        assert_eq!(
            driver2.supports_vectors(),
            cfg!(feature = "semantic-search")
        );
    }

    #[tokio::test]
    async fn lance_driver_as_dyn_trait() {
        let tmp = tempfile::tempdir().unwrap();
        let driver = LanceDriver::init(tmp.path()).await.unwrap();
        let permit = crate::workspace::lock::acquire_workspace_read_lock(tmp.path()).await.unwrap();
        let dyn_ref: &dyn StorageDriver = &driver;
        let artifacts = dyn_ref.list_artifacts(&permit, None).await.unwrap();
        assert!(artifacts.is_empty());
    }
}
