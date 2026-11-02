//! In-memory implementation of StorageDriver for testing.
//! Uses HashMap + tokio::sync::RwLock for thread-safe async access. No disk I/O.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use tokio::sync::RwLock;

use crate::artifact::canonical_key::ArtifactCandidate;
use crate::artifact::store::ArtifactSummary;
use crate::db::store::{ArtifactFilter, ArtifactRecord, FpfChunk, FpfChunkSummary, NewArtifact};
use crate::driver::{ArtifactStorage, FpfStorage, RelationStorage, SearchStorage, VectorStorage};
use crate::workspace::lock::{WorkspaceLock, WorkspaceReadPermit};

/// Internal state behind a single RwLock to avoid TOCTOU races.
struct InMemoryState {
    /// Canonical key → record. The key is the identity (ADR-030); the display
    /// id in the record is a repeatable alias.
    artifacts: HashMap<String, ArtifactRecord>,
    /// (source key, target key, relation_type)
    relations: Vec<(String, String, String)>,
    fpf_chunks: Vec<FpfChunk>,
    id_counters: HashMap<String, u32>,
}

/// Resolve a caller-supplied selector to the canonical key of exactly one
/// document, using the **shared** algorithm — the same one the Lance store and
/// driver must propagate ambiguity rather than treating it as absence.
fn resolve_key(state: &InMemoryState, selector: &str) -> anyhow::Result<Option<String>> {
    let candidates: Vec<ArtifactCandidate> = state
        .artifacts
        .values()
        .map(|record| ArtifactCandidate {
            canonical_key: record.canonical_key.clone(),
            display_id: record.id.clone(),
            title: record.title.clone(),
            // The in-memory driver has no files; the path is never read.
            relative_path: PathBuf::new(),
        })
        .collect();
    Ok(crate::artifact::canonical_key::resolve_artifact(selector, &candidates)?
        .map(|resolved| resolved.identity.canonical_key))
}

/// Thread-safe in-memory store for testing.
pub struct InMemoryStore {
    state: RwLock<InMemoryState>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(InMemoryState {
                artifacts: HashMap::new(),
                relations: Vec::new(),
                fpf_chunks: Vec::new(),
                id_counters: HashMap::new(),
            }),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Try RFC3339 first, then NaiveDate (%Y-%m-%d).
fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
        .or_else(|| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
        })
}

impl InMemoryStore {
    /// Open an existing workspace (no-op for in-memory).
    pub async fn open(_workspace_path: &Path) -> anyhow::Result<Self> {
        Ok(Self::new())
    }

    /// Create tables / schema if needed (no-op for in-memory).
    pub async fn init(_workspace_path: &Path) -> anyhow::Result<Self> {
        Ok(Self::new())
    }
}

// ── ArtifactStorage ─────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl ArtifactStorage for InMemoryStore {
    async fn create_artifact(&self, _guard: &WorkspaceLock, artifact: &NewArtifact) -> anyhow::Result<String> {
        let now = Utc::now().to_rfc3339();
        // The row is keyed by the canonical key (ADR-030). The prepared key
        // wins; a caller that supplied none gets the ADR-030 rule applied to
        // the document. The row's own field must carry the SAME key, or the
        // shared resolver names a key the map does not hold and a lookup by
        // display id finds nothing.
        let key = crate::db::store::canonical_key_of(
            &artifact.canonical_key,
            &artifact.id,
            &artifact.body,
        );
        let record = ArtifactRecord {
            canonical_key: key.clone(),
            id: artifact.id.clone(),
            kind: artifact.kind.clone(),
            status: artifact.status.clone(),
            title: artifact.title.clone(),
            body: artifact.body.clone(),
            depth: artifact.depth.clone(),
            author: artifact.author.clone(),
            parent_epic: artifact.parent_epic.clone(),
            r_eff_score: 0.0,
            valid_until: artifact.valid_until.clone(),
            created_at: now.clone(),
            updated_at: now,
            tags: artifact.tags.clone(),
            body_hash: None,
            embedding: None,
        };
        let id = record.id.clone();
        self.state.write().await.artifacts.insert(key, record);
        Ok(id)
    }

    async fn get_artifact(&self, _permit: &dyn WorkspaceReadPermit, id: &str) -> anyhow::Result<Option<ArtifactSummary>> {
        let state = self.state.read().await;
        let key = resolve_key(&state, id)?;
        Ok(key
            .and_then(|key| state.artifacts.get(&key))
            .map(|r| r.to_summary()))
    }

    async fn list_artifacts(
        &self,
        _permit: &dyn WorkspaceReadPermit,
        filter: Option<&ArtifactFilter>,
    ) -> anyhow::Result<Vec<ArtifactSummary>> {
        let state = self.state.read().await;
        let iter = state.artifacts.values().filter(|r| {
            if let Some(f) = filter {
                if let Some(ref k) = f.kind
                    && !r.kind.eq_ignore_ascii_case(k)
                {
                    return false;
                }
                if let Some(ref s) = f.status
                    && !r.status.eq_ignore_ascii_case(s)
                {
                    return false;
                }
            }
            true
        });
        let mut summaries: Vec<ArtifactSummary> = iter.map(|r| r.to_summary()).collect();
        summaries.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(summaries)
    }

    async fn update_artifact(
        &self,
        _guard: &WorkspaceLock,
        id: &str,
        status: Option<&str>,
        title: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut state = self.state.write().await;
        let key = resolve_key(&state, id)?
            .ok_or_else(|| anyhow::anyhow!("artifact not found: {id}"))?;
        let record = state
            .artifacts
            .get_mut(&key)
            .ok_or_else(|| anyhow::anyhow!("artifact not found: {id}"))?;
        if let Some(s) = status {
            record.status = s.to_string();
        }
        if let Some(t) = title {
            record.title = t.to_string();
        }
        record.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    async fn update_r_eff_score(&self, _guard: &WorkspaceLock, id: &str, score: f64) -> anyhow::Result<()> {
        let mut state = self.state.write().await;
        let key = resolve_key(&state, id)?
            .ok_or_else(|| anyhow::anyhow!("artifact not found: {id}"))?;
        let record = state
            .artifacts
            .get_mut(&key)
            .ok_or_else(|| anyhow::anyhow!("artifact not found: {id}"))?;
        record.r_eff_score = score;
        record.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    async fn delete_artifact(&self, _guard: &WorkspaceLock, id: &str) -> anyhow::Result<()> {
        let mut state = self.state.write().await;
        let key = resolve_key(&state, id)?
            .ok_or_else(|| anyhow::anyhow!("artifact not found: {id}"))?;
        state
            .artifacts
            .remove(&key)
            .ok_or_else(|| anyhow::anyhow!("artifact not found: {id}"))?;
        // Atomically clean up relations involving this artifact (same lock)
        state.relations.retain(|(s, t, _)| s != &key && t != &key);
        Ok(())
    }

    // -- Full records ---------------------------------------------------------

    async fn get_record(&self, _permit: &dyn WorkspaceReadPermit, id: &str) -> anyhow::Result<Option<ArtifactRecord>> {
        let state = self.state.read().await;
        Ok(resolve_key(&state, id)?
            .and_then(|key| state.artifacts.get(&key))
            .cloned())
    }

    async fn list_records(
        &self,
        _permit: &dyn WorkspaceReadPermit,
        filter: Option<&ArtifactFilter>,
    ) -> anyhow::Result<Vec<ArtifactRecord>> {
        let state = self.state.read().await;
        let iter = state.artifacts.values().filter(|r| {
            if let Some(f) = filter {
                if let Some(ref k) = f.kind
                    && !r.kind.eq_ignore_ascii_case(k)
                {
                    return false;
                }
                if let Some(ref s) = f.status
                    && !r.status.eq_ignore_ascii_case(s)
                {
                    return false;
                }
            }
            true
        });
        let mut records: Vec<ArtifactRecord> = iter.cloned().collect();
        records.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(records)
    }

    async fn update_body(&self, _guard: &WorkspaceLock, id: &str, body: &str) -> anyhow::Result<()> {
        let mut state = self.state.write().await;
        let key = resolve_key(&state, id)?
            .ok_or_else(|| anyhow::anyhow!("artifact not found: {id}"))?;
        let record = state
            .artifacts
            .get_mut(&key)
            .ok_or_else(|| anyhow::anyhow!("artifact not found: {id}"))?;
        record.body = body.to_string();
        record.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }
}

// ── RelationStorage ─────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl RelationStorage for InMemoryStore {
    async fn add_relation(&self, _guard: &WorkspaceLock, source: &str, target: &str, relation: &str) -> anyhow::Result<()> {
        // Self-link guard (PROB-019)
        if source.eq_ignore_ascii_case(target) {
            anyhow::bail!("Self-link not allowed: {} cannot link to itself", source);
        }
        let mut state = self.state.write().await;
        // Relation endpoints are canonical keys (ADR-030); a caller may still
        // pass a display alias, so resolve both before storing.
        let source_key = resolve_key(&state, source)?.unwrap_or_else(|| source.to_string());
        let target_key = resolve_key(&state, target)?.unwrap_or_else(|| target.to_string());
        // Reject duplicates
        let exists = state.relations.iter().any(|(s, t, r)| {
            s.eq_ignore_ascii_case(&source_key)
                && t.eq_ignore_ascii_case(&target_key)
                && r == relation
        });
        if exists {
            anyhow::bail!("relation already exists: {source} -> {target} ({relation})");
        }
        state
            .relations
            .push((source_key, target_key, relation.to_string()));
        Ok(())
    }

    async fn delete_relation(
        &self,
        _guard: &WorkspaceLock,
        source: &str,
        target: &str,
        relation: &str,
    ) -> anyhow::Result<()> {
        let mut state = self.state.write().await;
        let source_key = resolve_key(&state, source)?.unwrap_or_else(|| source.to_string());
        let target_key = resolve_key(&state, target)?.unwrap_or_else(|| target.to_string());
        let before = state.relations.len();
        state
            .relations
            .retain(|(s, t, r)| !(s == &source_key && t == &target_key && r == relation));
        if state.relations.len() == before {
            anyhow::bail!("relation not found: {source} -> {target} ({relation})");
        }
        Ok(())
    }

    async fn get_relations(&self, _permit: &dyn WorkspaceReadPermit, id: &str) -> anyhow::Result<Vec<(String, String)>> {
        let state = self.state.read().await;
        let key = resolve_key(&state, id)?;
        Ok(state
            .relations
            .iter()
            .filter(|(s, _, _)| Some(s) == key.as_ref())
            .map(|(_, t, r)| (t.clone(), r.clone()))
            .collect())
    }

    async fn get_incoming_relations(&self, _permit: &dyn WorkspaceReadPermit, id: &str) -> anyhow::Result<Vec<(String, String)>> {
        let state = self.state.read().await;
        let key = resolve_key(&state, id)?;
        Ok(state
            .relations
            .iter()
            .filter(|(_, t, _)| Some(t) == key.as_ref())
            .map(|(s, _, r)| (s.clone(), r.clone()))
            .collect())
    }

    async fn get_all_relations(&self, _permit: &dyn WorkspaceReadPermit) -> anyhow::Result<Vec<(String, String, String)>> {
        let state = self.state.read().await;
        Ok(state.relations.clone())
    }

    async fn delete_relations_for_artifact(&self, _guard: &WorkspaceLock, id: &str) -> anyhow::Result<()> {
        let mut state = self.state.write().await;
        let key = resolve_key(&state, id)?.unwrap_or_else(|| id.to_string());
        state
            .relations
            .retain(|(s, t, _)| !s.eq_ignore_ascii_case(&key) && !t.eq_ignore_ascii_case(&key));
        Ok(())
    }
}

// ── SearchStorage ───────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl SearchStorage for InMemoryStore {
    async fn search_body(
        &self,
        _permit: &dyn WorkspaceReadPermit,
        query: &str,
        kind_filter: Option<&str>,
    ) -> anyhow::Result<Vec<ArtifactRecord>> {
        let state = self.state.read().await;
        let query_lower = query.to_lowercase();
        let mut results: Vec<ArtifactRecord> = state
            .artifacts
            .values()
            .filter(|r| {
                if let Some(k) = kind_filter
                    && !r.kind.eq_ignore_ascii_case(k)
                {
                    return false;
                }
                r.title.to_lowercase().contains(&query_lower)
                    || r.body.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect();
        results.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(results)
    }

    async fn find_stale(&self, _permit: &dyn WorkspaceReadPermit) -> anyhow::Result<Vec<ArtifactRecord>> {
        let now = Utc::now();
        let state = self.state.read().await;
        let mut stale: Vec<ArtifactRecord> = state
            .artifacts
            .values()
            .filter(|r| {
                r.valid_until
                    .as_ref()
                    .is_some_and(|vu| parse_date(vu).is_some_and(|d| d < now))
            })
            .cloned()
            .collect();
        stale.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(stale)
    }

    async fn next_id(&self, _permit: &dyn WorkspaceReadPermit, kind_prefix: &str) -> anyhow::Result<String> {
        let mut state = self.state.write().await;
        let counter = state
            .id_counters
            .entry(kind_prefix.to_uppercase())
            .or_insert(0);
        *counter += 1;
        Ok(format!("{}-{:03}", kind_prefix.to_uppercase(), *counter))
    }
}

// ── VectorStorage (defaults — InMemory doesn't support vectors) ─────────────

#[async_trait::async_trait]
impl VectorStorage for InMemoryStore {}

// ── FpfStorage ──────────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl FpfStorage for InMemoryStore {
    fn has_fpf(&self) -> bool {
        self.state
            .try_read()
            .is_ok_and(|s| !s.fpf_chunks.is_empty())
    }

    async fn insert_fpf_chunks(
        &self,
        chunks: &[FpfChunk],
        embeddings: Option<&[Vec<f32>]>,
    ) -> anyhow::Result<usize> {
        if let Some(vecs) = embeddings
            && vecs.len() != chunks.len()
        {
            anyhow::bail!(
                "embeddings length ({}) must match chunks length ({})",
                vecs.len(),
                chunks.len()
            );
        }
        let mut state = self.state.write().await;
        let count = chunks.len();
        state.fpf_chunks.extend(chunks.iter().cloned());
        Ok(count)
    }

    async fn search_fpf(&self, query: &str, limit: usize) -> anyhow::Result<Vec<FpfChunk>> {
        let state = self.state.read().await;
        let query_lower = query.to_lowercase();
        let results: Vec<FpfChunk> = state
            .fpf_chunks
            .iter()
            .filter(|c| {
                c.title.to_lowercase().contains(&query_lower)
                    || c.body.to_lowercase().contains(&query_lower)
                    || c.section_id.to_lowercase().contains(&query_lower)
            })
            .take(limit)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn get_fpf_section(&self, section_id: &str) -> anyhow::Result<Option<FpfChunk>> {
        let state = self.state.read().await;
        Ok(state
            .fpf_chunks
            .iter()
            .find(|c| c.section_id == section_id)
            .cloned())
    }

    async fn list_fpf_sections(&self) -> anyhow::Result<Vec<FpfChunkSummary>> {
        let state = self.state.read().await;
        Ok(state
            .fpf_chunks
            .iter()
            .map(|c| FpfChunkSummary {
                section_id: c.section_id.clone(),
                title: c.title.clone(),
                line_count: c.line_count,
            })
            .collect())
    }

    async fn clear_fpf(&self) -> anyhow::Result<()> {
        let mut state = self.state.write().await;
        state.fpf_chunks.clear();
        Ok(())
    }

    async fn search_fpf_by_vector(
        &self,
        _query_vec: &[f32],
        _limit: usize,
    ) -> anyhow::Result<Vec<FpfChunk>> {
        // InMemoryStore does not support vector similarity search — it has
        // no embedding storage or cosine computation. Return an explicit Err
        // rather than silently returning Ok(empty), because silent success is
        // indistinguishable from "no matches" and creates a landmine for
        // tests that accidentally route through this backend.
        // See NOTE-045 H1 (Sprint 13.7 post-closeout re-audit).
        anyhow::bail!(
            "search_fpf_by_vector is not supported by InMemoryStore; \
             use LanceDriver for semantic search or keyword search via search_fpf"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_new_artifact(kind: &str, title: &str, body: &str) -> NewArtifact {
        NewArtifact {
            // Placeholder like the id below — the id is replaced by
            // `next_id` in tests, and the canonical key follows it.
            canonical_key: String::new(),
            id: String::new(), // will be replaced by next_id in tests
            kind: kind.to_string(),
            status: "draft".to_string(),
            title: title.to_string(),
            body: body.to_string(),
            depth: "standard".to_string(),
            author: Some("test".to_string()),
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        }
    }

    async fn create_with_id(guard: &crate::workspace::lock::WorkspaceLock, store: &InMemoryStore, kind: &str, title: &str, body: &str) -> String {
        let id = store.next_id(guard, kind).await.unwrap();
        let mut art = make_new_artifact(kind, title, body);
        art.id = id.clone();
        store.create_artifact(guard, &art).await.unwrap();
        id
    }

    #[tokio::test]
    async fn test_create_and_get_artifact() {
        let store = InMemoryStore::new();
        let tmp = tempfile::tempdir().unwrap();
        let guard = crate::workspace::acquire_workspace_lock(tmp.path()).await.unwrap();
        let id = create_with_id(&guard, &store, "PRD", "Auth System", "Login flow").await;

        assert_eq!(id, "PRD-001");

        let summary = store.get_artifact(&guard, &id).await.unwrap().unwrap();
        assert_eq!(summary.id, "PRD-001");
        assert_eq!(summary.title, "Auth System");
        assert_eq!(summary.kind, "PRD");
        assert_eq!(summary.status, "draft");

        let record = store.get_record(&guard, &id).await.unwrap().unwrap();
        assert_eq!(record.body, "Login flow");
        assert_eq!(record.r_eff_score, 0.0);
        assert!(record.author.as_deref() == Some("test"));

        // Not found
        assert!(store.get_artifact(&guard, "PRD-999").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_list_artifacts_with_filter() {
        let store = InMemoryStore::new();
        let tmp = tempfile::tempdir().unwrap();
        let guard = crate::workspace::acquire_workspace_lock(tmp.path()).await.unwrap();
        create_with_id(&guard, &store, "PRD", "PRD One", "body1").await;
        create_with_id(&guard, &store, "RFC", "RFC One", "body2").await;
        create_with_id(&guard, &store, "PRD", "PRD Two", "body3").await;

        // No filter -- all 3
        let all = store.list_artifacts(&guard, None).await.unwrap();
        assert_eq!(all.len(), 3);

        // Filter by kind
        let filter = ArtifactFilter {
            kind: Some("PRD".to_string()),
            status: None,
        };
        let prds = store.list_artifacts(&guard, Some(&filter)).await.unwrap();
        assert_eq!(prds.len(), 2);
        assert!(prds.iter().all(|s| s.kind == "PRD"));

        // Filter by status
        let filter = ArtifactFilter {
            kind: None,
            status: Some("active".to_string()),
        };
        let active = store.list_artifacts(&guard, Some(&filter)).await.unwrap();
        assert_eq!(active.len(), 0);
    }

    #[tokio::test]
    async fn test_update_artifact_status() {
        let store = InMemoryStore::new();
        let tmp = tempfile::tempdir().unwrap();
        let guard = crate::workspace::acquire_workspace_lock(tmp.path()).await.unwrap();
        let id = create_with_id(&guard, &store, "PRD", "Title", "body").await;

        store
            .update_artifact(&guard, &id, Some("active"), Some("New Title"))
            .await
            .unwrap();

        let summary = store.get_artifact(&guard, &id).await.unwrap().unwrap();
        assert_eq!(summary.status, "active");
        assert_eq!(summary.title, "New Title");

        // Update r_eff
        store.update_r_eff_score(&guard, &id, 0.85).await.unwrap();
        let record = store.get_record(&guard, &id).await.unwrap().unwrap();
        assert!((record.r_eff_score - 0.85).abs() < f64::EPSILON);

        // Update body
        store.update_body(&guard, &id, "updated body").await.unwrap();
        let record = store.get_record(&guard, &id).await.unwrap().unwrap();
        assert_eq!(record.body, "updated body");

        // Non-existent artifact
        assert!(
            store
                .update_artifact(&guard, "NOPE-001", Some("x"), None)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_delete_artifact() {
        let store = InMemoryStore::new();
        let tmp = tempfile::tempdir().unwrap();
        let guard = crate::workspace::acquire_workspace_lock(tmp.path()).await.unwrap();
        let id = create_with_id(&guard, &store, "PRD", "To Delete", "body").await;
        let id2 = create_with_id(&guard, &store, "RFC", "Keep", "body").await;

        // Add a relation involving the deleted artifact
        store.add_relation(&guard, &id, &id2, "informs").await.unwrap();

        store.delete_artifact(&guard, &id).await.unwrap();

        assert!(store.get_artifact(&guard, &id).await.unwrap().is_none());
        assert!(store.get_artifact(&guard, &id2).await.unwrap().is_some());

        // Relations cleaned up
        let rels = store.get_all_relations(&guard).await.unwrap();
        assert!(rels.is_empty());

        // Double delete errors
        assert!(store.delete_artifact(&guard, &id).await.is_err());
    }

    #[tokio::test]
    async fn test_relations_crud() {
        let store = InMemoryStore::new();
        let tmp = tempfile::tempdir().unwrap();
        let guard = crate::workspace::acquire_workspace_lock(tmp.path()).await.unwrap();
        let prd = create_with_id(&guard, &store, "PRD", "PRD", "body").await;
        let rfc = create_with_id(&guard, &store, "RFC", "RFC", "body").await;
        let adr = create_with_id(&guard, &store, "ADR", "ADR", "body").await;

        store.add_relation(&guard, &prd, &rfc, "informs").await.unwrap();
        store.add_relation(&guard, &rfc, &adr, "implements").await.unwrap();

        // Outgoing from prd
        let out = store.get_relations(&guard, &prd).await.unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], ("RFC-001".to_string(), "informs".to_string()));

        // Incoming to rfc
        let inc = store.get_incoming_relations(&guard, &rfc).await.unwrap();
        assert_eq!(inc.len(), 1);
        assert_eq!(inc[0], ("PRD-001".to_string(), "informs".to_string()));

        // All relations
        let all = store.get_all_relations(&guard).await.unwrap();
        assert_eq!(all.len(), 2);

        // Duplicate rejection
        assert!(store.add_relation(&guard, &prd, &rfc, "informs").await.is_err());

        // Self-link rejection (PROB-019)
        assert!(store.add_relation(&guard, &prd, &prd, "informs").await.is_err());
        // Case-insensitive self-link
        assert!(
            store
                .add_relation(&guard, &prd, &prd.to_uppercase(), "informs")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_search_body() {
        let store = InMemoryStore::new();
        let tmp = tempfile::tempdir().unwrap();
        let guard = crate::workspace::acquire_workspace_lock(tmp.path()).await.unwrap();
        create_with_id(&guard, &store, "PRD", "Auth System", "OAuth2 login flow").await;
        create_with_id(&guard, &store, "RFC", "DB Migration", "PostgreSQL schema changes").await;
        create_with_id(&guard, 
            &store,
            "PRD",
            "Search Feature",
            "Full-text search with PostgreSQL",
        )
        .await;

        // Search by body content (case-insensitive)
        let results = store.search_body(&guard, "postgresql", None).await.unwrap();
        assert_eq!(results.len(), 2);

        // Search with kind filter
        let results = store.search_body(&guard, "postgresql", Some("PRD")).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Search Feature");

        // Search by title
        let results = store.search_body(&guard, "auth", None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Auth System");

        // No match
        let results = store.search_body(&guard, "nonexistent", None).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_next_id_increments() {
        let store = InMemoryStore::new();
        let tmp = tempfile::tempdir().unwrap();
        let guard = crate::workspace::acquire_workspace_lock(tmp.path()).await.unwrap();

        assert_eq!(store.next_id(&guard, "prd").await.unwrap(), "PRD-001");
        assert_eq!(store.next_id(&guard, "prd").await.unwrap(), "PRD-002");
        assert_eq!(store.next_id(&guard, "prd").await.unwrap(), "PRD-003");

        // Different prefix has its own counter
        assert_eq!(store.next_id(&guard, "rfc").await.unwrap(), "RFC-001");
        assert_eq!(store.next_id(&guard, "rfc").await.unwrap(), "RFC-002");

        // Case-insensitive prefix
        assert_eq!(store.next_id(&guard, "PRD").await.unwrap(), "PRD-004");
    }

    #[tokio::test]
    async fn test_thread_safety() {
        use std::sync::Arc;

        let store = Arc::new(InMemoryStore::new());
        let tmp = tempfile::tempdir().unwrap();
        let guard = Arc::new(crate::workspace::acquire_workspace_lock(tmp.path()).await.unwrap());

        let store1 = Arc::clone(&store);
        let guard1 = Arc::clone(&guard);
        let t1 = tokio::spawn(async move {
            for i in 0..50 {
                let id = store1.next_id(&*guard1, "TST").await.unwrap();
                let art = NewArtifact {
                    canonical_key: id.clone(),
                    id,
                    kind: "TST".to_string(),
                    status: "draft".to_string(),
                    title: format!("Task A-{i}"),
                    body: "body".to_string(),
                    depth: "tactical".to_string(),
                    author: None,
                    parent_epic: None,
                    valid_until: None,
                    tags: Vec::new(),
                };
                store1.create_artifact(&*guard1, &art).await.unwrap();
            }
        });

        let store2 = Arc::clone(&store);
        let guard2 = Arc::clone(&guard);
        let t2 = tokio::spawn(async move {
            for i in 0..50 {
                let id = store2.next_id(&*guard2, "TST").await.unwrap();
                let art = NewArtifact {
                    canonical_key: id.clone(),
                    id,
                    kind: "TST".to_string(),
                    status: "draft".to_string(),
                    title: format!("Task B-{i}"),
                    body: "body".to_string(),
                    depth: "tactical".to_string(),
                    author: None,
                    parent_epic: None,
                    valid_until: None,
                    tags: Vec::new(),
                };
                store2.create_artifact(&*guard2, &art).await.unwrap();
            }
        });

        t1.await.unwrap();
        t2.await.unwrap();

        // All 100 artifacts created with unique IDs
        let all = store.list_artifacts(&*guard, None).await.unwrap();
        assert_eq!(all.len(), 100);

        // All IDs unique
        let mut ids: Vec<String> = all.iter().map(|s| s.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 100);
    }

    #[tokio::test]
    async fn test_find_stale() {
        let store = InMemoryStore::new();
        let tmp = tempfile::tempdir().unwrap();
        let guard = crate::workspace::acquire_workspace_lock(tmp.path()).await.unwrap();

        // Create artifact with expired valid_until (RFC3339 format)
        let id = store.next_id(&guard, "PRD").await.unwrap();
        let art = NewArtifact {
            canonical_key: id.clone(),
            id: id.clone(),
            kind: "PRD".to_string(),
            status: "active".to_string(),
            title: "Stale".to_string(),
            body: "old".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: Some("2020-01-01T00:00:00Z".to_string()),
            tags: Vec::new(),
        };
        store.create_artifact(&guard, &art).await.unwrap();

        // Create artifact with expired valid_until (NaiveDate format)
        let id_naive = store.next_id(&guard, "PRD").await.unwrap();
        let art_naive = NewArtifact {
            canonical_key: id_naive.clone(),
            id: id_naive.clone(),
            kind: "PRD".to_string(),
            status: "active".to_string(),
            title: "Stale Naive".to_string(),
            body: "old naive".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: Some("2020-06-15".to_string()),
            tags: Vec::new(),
        };
        store.create_artifact(&guard, &art_naive).await.unwrap();

        // Create artifact without valid_until
        let id3 = store.next_id(&guard, "PRD").await.unwrap();
        let art3 = NewArtifact {
            canonical_key: id3.clone(),
            id: id3,
            kind: "PRD".to_string(),
            status: "active".to_string(),
            title: "Fresh".to_string(),
            body: "new".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        };
        store.create_artifact(&guard, &art3).await.unwrap();

        // Create artifact with future valid_until
        let id4 = store.next_id(&guard, "PRD").await.unwrap();
        let art4 = NewArtifact {
            canonical_key: id4.clone(),
            id: id4,
            kind: "PRD".to_string(),
            status: "active".to_string(),
            title: "Future".to_string(),
            body: "still valid".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: Some("2099-12-31T23:59:59Z".to_string()),
            tags: Vec::new(),
        };
        store.create_artifact(&guard, &art4).await.unwrap();

        let stale = store.find_stale(&guard).await.unwrap();
        assert_eq!(stale.len(), 2);
        assert_eq!(stale[0].id, id);
        assert_eq!(stale[1].id, id_naive);
    }

    #[tokio::test]
    async fn test_fpf_operations() {
        let store = InMemoryStore::new();
        assert!(!store.has_fpf());

        let chunks = vec![
            FpfChunk {
                id: "1".to_string(),
                section_id: "B.3".to_string(),
                parent_section: Some("B".to_string()),
                title: "Trust Calculus".to_string(),
                body: "Trust is earned through evidence".to_string(),
                line_count: 5,
                file_path: "fpf.md".to_string(),
                created_at: Utc::now().to_rfc3339(),
            },
            FpfChunk {
                id: "2".to_string(),
                section_id: "A.1".to_string(),
                parent_section: Some("A".to_string()),
                title: "ADI Cycle".to_string(),
                body: "Abduction, Deduction, Induction".to_string(),
                line_count: 3,
                file_path: "fpf.md".to_string(),
                created_at: Utc::now().to_rfc3339(),
            },
        ];

        let inserted = store.insert_fpf_chunks(&chunks, None).await.unwrap();
        assert_eq!(inserted, 2);
        assert!(store.has_fpf());

        // Search
        let found = store.search_fpf("trust", 10).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].section_id, "B.3");

        // Get section
        let section = store.get_fpf_section("A.1").await.unwrap().unwrap();
        assert_eq!(section.title, "ADI Cycle");
        assert!(store.get_fpf_section("Z.9").await.unwrap().is_none());

        // List sections
        let list = store.list_fpf_sections().await.unwrap();
        assert_eq!(list.len(), 2);

        // Clear
        store.clear_fpf().await.unwrap();
        assert!(!store.has_fpf());
    }

    #[tokio::test]
    async fn test_list_records_with_filter() {
        let store = InMemoryStore::new();
        let tmp = tempfile::tempdir().unwrap();
        let guard = crate::workspace::acquire_workspace_lock(tmp.path()).await.unwrap();
        create_with_id(&guard, &store, "PRD", "P1", "b1").await;
        create_with_id(&guard, &store, "RFC", "R1", "b2").await;

        let filter = ArtifactFilter {
            kind: Some("RFC".to_string()),
            status: None,
        };
        let records = store.list_records(&guard, Some(&filter)).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, "RFC");
        assert_eq!(records[0].body, "b2");
    }

    #[tokio::test]
    async fn test_parse_date_helper() {
        // RFC3339
        let d1 = parse_date("2020-01-01T00:00:00Z");
        assert!(d1.is_some());

        // NaiveDate
        let d2 = parse_date("2020-06-15");
        assert!(d2.is_some());

        // Invalid
        let d3 = parse_date("not-a-date");
        assert!(d3.is_none());

        // RFC3339 with offset
        let d4 = parse_date("2020-01-01T00:00:00+03:00");
        assert!(d4.is_some());
    }
}
