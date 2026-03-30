//! In-memory implementation of StorageDriver for testing.
//! Uses HashMap + RwLock for thread-safe access. No disk I/O.

use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use chrono::Utc;

use crate::artifact::store::ArtifactSummary;
use crate::db::store::{ArtifactFilter, ArtifactRecord, FpfChunk, FpfChunkSummary, NewArtifact};
use crate::driver::StorageDriver;

/// Thread-safe in-memory store for testing.
pub struct InMemoryStore {
    artifacts: RwLock<HashMap<String, ArtifactRecord>>,
    relations: RwLock<Vec<(String, String, String)>>, // source, target, relation_type
    fpf_chunks: RwLock<Vec<FpfChunk>>,
    id_counters: RwLock<HashMap<String, u32>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            artifacts: RwLock::new(HashMap::new()),
            relations: RwLock::new(Vec::new()),
            fpf_chunks: RwLock::new(Vec::new()),
            id_counters: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl StorageDriver for InMemoryStore {
    // ── Lifecycle ────────────────────────────────────────────────────

    async fn open(_workspace_path: &Path) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self::new())
    }

    async fn init(_workspace_path: &Path) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self::new())
    }

    // ── Artifact CRUD ────────────────────────────────────────────────

    async fn create_artifact(&self, artifact: &NewArtifact) -> anyhow::Result<String> {
        let now = Utc::now().to_rfc3339();
        let record = ArtifactRecord {
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
        };
        let id = record.id.clone();
        self.artifacts
            .write()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?
            .insert(id.clone(), record);
        Ok(id)
    }

    async fn get_artifact(&self, id: &str) -> anyhow::Result<Option<ArtifactSummary>> {
        let arts = self
            .artifacts
            .read()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        Ok(arts.get(id).map(|r| r.to_summary()))
    }

    async fn list_artifacts(
        &self,
        filter: Option<&ArtifactFilter>,
    ) -> anyhow::Result<Vec<ArtifactSummary>> {
        let arts = self
            .artifacts
            .read()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        let iter = arts.values().filter(|r| {
            if let Some(f) = filter {
                if let Some(ref k) = f.kind {
                    if !r.kind.eq_ignore_ascii_case(k) {
                        return false;
                    }
                }
                if let Some(ref s) = f.status {
                    if !r.status.eq_ignore_ascii_case(s) {
                        return false;
                    }
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
        id: &str,
        status: Option<&str>,
        title: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut arts = self
            .artifacts
            .write()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        let record = arts
            .get_mut(id)
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

    async fn update_r_eff_score(&self, id: &str, score: f64) -> anyhow::Result<()> {
        let mut arts = self
            .artifacts
            .write()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        let record = arts
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("artifact not found: {id}"))?;
        record.r_eff_score = score;
        record.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    async fn delete_artifact(&self, id: &str) -> anyhow::Result<()> {
        let mut arts = self
            .artifacts
            .write()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        arts.remove(id)
            .ok_or_else(|| anyhow::anyhow!("artifact not found: {id}"))?;
        // Also clean up relations involving this artifact
        drop(arts);
        let mut rels = self
            .relations
            .write()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        rels.retain(|(s, t, _)| s != id && t != id);
        Ok(())
    }

    // ── Full records ─────────────────────────────────────────────────

    async fn get_record(&self, id: &str) -> anyhow::Result<Option<ArtifactRecord>> {
        let arts = self
            .artifacts
            .read()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        Ok(arts.get(id).cloned())
    }

    async fn list_records(
        &self,
        filter: Option<&ArtifactFilter>,
    ) -> anyhow::Result<Vec<ArtifactRecord>> {
        let arts = self
            .artifacts
            .read()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        let iter = arts.values().filter(|r| {
            if let Some(f) = filter {
                if let Some(ref k) = f.kind {
                    if !r.kind.eq_ignore_ascii_case(k) {
                        return false;
                    }
                }
                if let Some(ref s) = f.status {
                    if !r.status.eq_ignore_ascii_case(s) {
                        return false;
                    }
                }
            }
            true
        });
        let mut records: Vec<ArtifactRecord> = iter.cloned().collect();
        records.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(records)
    }

    async fn update_body(&self, id: &str, body: &str) -> anyhow::Result<()> {
        let mut arts = self
            .artifacts
            .write()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        let record = arts
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("artifact not found: {id}"))?;
        record.body = body.to_string();
        record.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    // ── Relations ────────────────────────────────────────────────────

    async fn add_relation(
        &self,
        source: &str,
        target: &str,
        relation: &str,
    ) -> anyhow::Result<()> {
        let mut rels = self
            .relations
            .write()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        // Reject duplicates
        let exists = rels
            .iter()
            .any(|(s, t, r)| s == source && t == target && r == relation);
        if exists {
            anyhow::bail!("relation already exists: {source} -> {target} ({relation})");
        }
        rels.push((source.to_string(), target.to_string(), relation.to_string()));
        Ok(())
    }

    async fn get_relations(&self, id: &str) -> anyhow::Result<Vec<(String, String)>> {
        let rels = self
            .relations
            .read()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        Ok(rels
            .iter()
            .filter(|(s, _, _)| s == id)
            .map(|(_, t, r)| (t.clone(), r.clone()))
            .collect())
    }

    async fn get_incoming_relations(&self, id: &str) -> anyhow::Result<Vec<(String, String)>> {
        let rels = self
            .relations
            .read()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        Ok(rels
            .iter()
            .filter(|(_, t, _)| t == id)
            .map(|(s, _, r)| (s.clone(), r.clone()))
            .collect())
    }

    async fn get_all_relations(&self) -> anyhow::Result<Vec<(String, String, String)>> {
        let rels = self
            .relations
            .read()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        Ok(rels.clone())
    }

    // ── Search ───────────────────────────────────────────────────────

    async fn search_body(
        &self,
        query: &str,
        kind_filter: Option<&str>,
    ) -> anyhow::Result<Vec<ArtifactRecord>> {
        let arts = self
            .artifacts
            .read()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        let query_lower = query.to_lowercase();
        let mut results: Vec<ArtifactRecord> = arts
            .values()
            .filter(|r| {
                if let Some(k) = kind_filter {
                    if !r.kind.eq_ignore_ascii_case(k) {
                        return false;
                    }
                }
                r.title.to_lowercase().contains(&query_lower)
                    || r.body.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect();
        results.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(results)
    }

    async fn find_stale(&self) -> anyhow::Result<Vec<ArtifactRecord>> {
        let now = Utc::now().to_rfc3339();
        let arts = self
            .artifacts
            .read()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        let mut stale: Vec<ArtifactRecord> = arts
            .values()
            .filter(|r| {
                if let Some(ref vu) = r.valid_until {
                    vu.as_str() < now.as_str()
                } else {
                    false
                }
            })
            .cloned()
            .collect();
        stale.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(stale)
    }

    async fn next_id(&self, kind_prefix: &str) -> anyhow::Result<String> {
        let mut counters = self
            .id_counters
            .write()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        let counter = counters.entry(kind_prefix.to_uppercase()).or_insert(0);
        *counter += 1;
        Ok(format!("{}-{:03}", kind_prefix.to_uppercase(), *counter))
    }

    // ── FPF Knowledge Base ───────────────────────────────────────────

    fn has_fpf(&self) -> bool {
        let chunks = self.fpf_chunks.read().unwrap_or_else(|e| e.into_inner());
        !chunks.is_empty()
    }

    async fn insert_fpf_chunks(&self, chunks: &[FpfChunk]) -> anyhow::Result<usize> {
        let mut store = self
            .fpf_chunks
            .write()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        let count = chunks.len();
        store.extend(chunks.iter().cloned());
        Ok(count)
    }

    async fn search_fpf(&self, query: &str, limit: usize) -> anyhow::Result<Vec<FpfChunk>> {
        let store = self
            .fpf_chunks
            .read()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        let query_lower = query.to_lowercase();
        let results: Vec<FpfChunk> = store
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
        let store = self
            .fpf_chunks
            .read()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        Ok(store.iter().find(|c| c.section_id == section_id).cloned())
    }

    async fn list_fpf_sections(&self) -> anyhow::Result<Vec<FpfChunkSummary>> {
        let store = self
            .fpf_chunks
            .read()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        Ok(store
            .iter()
            .map(|c| FpfChunkSummary {
                section_id: c.section_id.clone(),
                title: c.title.clone(),
                line_count: c.line_count,
            })
            .collect())
    }

    async fn clear_fpf(&self) -> anyhow::Result<()> {
        let mut store = self
            .fpf_chunks
            .write()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        store.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_new_artifact(kind: &str, title: &str, body: &str) -> NewArtifact {
        NewArtifact {
            id: String::new(), // will be replaced by next_id in tests
            kind: kind.to_string(),
            status: "draft".to_string(),
            title: title.to_string(),
            body: body.to_string(),
            depth: "standard".to_string(),
            author: Some("test".to_string()),
            parent_epic: None,
            valid_until: None,
        }
    }

    async fn create_with_id(store: &InMemoryStore, kind: &str, title: &str, body: &str) -> String {
        let id = store.next_id(kind).await.unwrap();
        let mut art = make_new_artifact(kind, title, body);
        art.id = id.clone();
        store.create_artifact(&art).await.unwrap();
        id
    }

    #[tokio::test]
    async fn test_create_and_get_artifact() {
        let store = InMemoryStore::new();
        let id = create_with_id(&store, "PRD", "Auth System", "Login flow").await;

        assert_eq!(id, "PRD-001");

        let summary = store.get_artifact(&id).await.unwrap().unwrap();
        assert_eq!(summary.id, "PRD-001");
        assert_eq!(summary.title, "Auth System");
        assert_eq!(summary.kind, "PRD");
        assert_eq!(summary.status, "draft");

        let record = store.get_record(&id).await.unwrap().unwrap();
        assert_eq!(record.body, "Login flow");
        assert_eq!(record.r_eff_score, 0.0);
        assert!(record.author.as_deref() == Some("test"));

        // Not found
        assert!(store.get_artifact("PRD-999").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_list_artifacts_with_filter() {
        let store = InMemoryStore::new();
        create_with_id(&store, "PRD", "PRD One", "body1").await;
        create_with_id(&store, "RFC", "RFC One", "body2").await;
        create_with_id(&store, "PRD", "PRD Two", "body3").await;

        // No filter — all 3
        let all = store.list_artifacts(None).await.unwrap();
        assert_eq!(all.len(), 3);

        // Filter by kind
        let filter = ArtifactFilter {
            kind: Some("PRD".to_string()),
            status: None,
        };
        let prds = store.list_artifacts(Some(&filter)).await.unwrap();
        assert_eq!(prds.len(), 2);
        assert!(prds.iter().all(|s| s.kind == "PRD"));

        // Filter by status
        let filter = ArtifactFilter {
            kind: None,
            status: Some("active".to_string()),
        };
        let active = store.list_artifacts(Some(&filter)).await.unwrap();
        assert_eq!(active.len(), 0);
    }

    #[tokio::test]
    async fn test_update_artifact_status() {
        let store = InMemoryStore::new();
        let id = create_with_id(&store, "PRD", "Title", "body").await;

        store
            .update_artifact(&id, Some("active"), Some("New Title"))
            .await
            .unwrap();

        let summary = store.get_artifact(&id).await.unwrap().unwrap();
        assert_eq!(summary.status, "active");
        assert_eq!(summary.title, "New Title");

        // Update r_eff
        store.update_r_eff_score(&id, 0.85).await.unwrap();
        let record = store.get_record(&id).await.unwrap().unwrap();
        assert!((record.r_eff_score - 0.85).abs() < f64::EPSILON);

        // Update body
        store.update_body(&id, "updated body").await.unwrap();
        let record = store.get_record(&id).await.unwrap().unwrap();
        assert_eq!(record.body, "updated body");

        // Non-existent artifact
        assert!(store
            .update_artifact("NOPE-001", Some("x"), None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_delete_artifact() {
        let store = InMemoryStore::new();
        let id = create_with_id(&store, "PRD", "To Delete", "body").await;
        let id2 = create_with_id(&store, "RFC", "Keep", "body").await;

        // Add a relation involving the deleted artifact
        store.add_relation(&id, &id2, "informs").await.unwrap();

        store.delete_artifact(&id).await.unwrap();

        assert!(store.get_artifact(&id).await.unwrap().is_none());
        assert!(store.get_artifact(&id2).await.unwrap().is_some());

        // Relations cleaned up
        let rels = store.get_all_relations().await.unwrap();
        assert!(rels.is_empty());

        // Double delete errors
        assert!(store.delete_artifact(&id).await.is_err());
    }

    #[tokio::test]
    async fn test_relations_crud() {
        let store = InMemoryStore::new();
        let prd = create_with_id(&store, "PRD", "PRD", "body").await;
        let rfc = create_with_id(&store, "RFC", "RFC", "body").await;
        let adr = create_with_id(&store, "ADR", "ADR", "body").await;

        store.add_relation(&prd, &rfc, "informs").await.unwrap();
        store
            .add_relation(&rfc, &adr, "implements")
            .await
            .unwrap();

        // Outgoing from prd
        let out = store.get_relations(&prd).await.unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], ("RFC-001".to_string(), "informs".to_string()));

        // Incoming to rfc
        let inc = store.get_incoming_relations(&rfc).await.unwrap();
        assert_eq!(inc.len(), 1);
        assert_eq!(inc[0], ("PRD-001".to_string(), "informs".to_string()));

        // All relations
        let all = store.get_all_relations().await.unwrap();
        assert_eq!(all.len(), 2);

        // Duplicate rejection
        assert!(store.add_relation(&prd, &rfc, "informs").await.is_err());
    }

    #[tokio::test]
    async fn test_search_body() {
        let store = InMemoryStore::new();
        create_with_id(&store, "PRD", "Auth System", "OAuth2 login flow").await;
        create_with_id(&store, "RFC", "DB Migration", "PostgreSQL schema changes").await;
        create_with_id(&store, "PRD", "Search Feature", "Full-text search with PostgreSQL").await;

        // Search by body content (case-insensitive)
        let results = store.search_body("postgresql", None).await.unwrap();
        assert_eq!(results.len(), 2);

        // Search with kind filter
        let results = store.search_body("postgresql", Some("PRD")).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Search Feature");

        // Search by title
        let results = store.search_body("auth", None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Auth System");

        // No match
        let results = store.search_body("nonexistent", None).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_next_id_increments() {
        let store = InMemoryStore::new();

        assert_eq!(store.next_id("prd").await.unwrap(), "PRD-001");
        assert_eq!(store.next_id("prd").await.unwrap(), "PRD-002");
        assert_eq!(store.next_id("prd").await.unwrap(), "PRD-003");

        // Different prefix has its own counter
        assert_eq!(store.next_id("rfc").await.unwrap(), "RFC-001");
        assert_eq!(store.next_id("rfc").await.unwrap(), "RFC-002");

        // Case-insensitive prefix
        assert_eq!(store.next_id("PRD").await.unwrap(), "PRD-004");
    }

    #[tokio::test]
    async fn test_thread_safety() {
        use std::sync::Arc;

        let store = Arc::new(InMemoryStore::new());

        let store1 = Arc::clone(&store);
        let t1 = tokio::spawn(async move {
            for i in 0..50 {
                let id = store1.next_id("TST").await.unwrap();
                let art = NewArtifact {
                    id,
                    kind: "TST".to_string(),
                    status: "draft".to_string(),
                    title: format!("Task A-{i}"),
                    body: "body".to_string(),
                    depth: "tactical".to_string(),
                    author: None,
                    parent_epic: None,
                    valid_until: None,
                };
                store1.create_artifact(&art).await.unwrap();
            }
        });

        let store2 = Arc::clone(&store);
        let t2 = tokio::spawn(async move {
            for i in 0..50 {
                let id = store2.next_id("TST").await.unwrap();
                let art = NewArtifact {
                    id,
                    kind: "TST".to_string(),
                    status: "draft".to_string(),
                    title: format!("Task B-{i}"),
                    body: "body".to_string(),
                    depth: "tactical".to_string(),
                    author: None,
                    parent_epic: None,
                    valid_until: None,
                };
                store2.create_artifact(&art).await.unwrap();
            }
        });

        t1.await.unwrap();
        t2.await.unwrap();

        // All 100 artifacts created with unique IDs
        let all = store.list_artifacts(None).await.unwrap();
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

        // Create artifact with expired valid_until
        let id = store.next_id("PRD").await.unwrap();
        let art = NewArtifact {
            id: id.clone(),
            kind: "PRD".to_string(),
            status: "active".to_string(),
            title: "Stale".to_string(),
            body: "old".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: Some("2020-01-01T00:00:00Z".to_string()),
        };
        store.create_artifact(&art).await.unwrap();

        // Create artifact without valid_until
        let id2 = store.next_id("PRD").await.unwrap();
        let art2 = NewArtifact {
            id: id2,
            kind: "PRD".to_string(),
            status: "active".to_string(),
            title: "Fresh".to_string(),
            body: "new".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
        };
        store.create_artifact(&art2).await.unwrap();

        let stale = store.find_stale().await.unwrap();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].id, id);
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

        let inserted = store.insert_fpf_chunks(&chunks).await.unwrap();
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
        create_with_id(&store, "PRD", "P1", "b1").await;
        create_with_id(&store, "RFC", "R1", "b2").await;

        let filter = ArtifactFilter {
            kind: Some("RFC".to_string()),
            status: None,
        };
        let records = store.list_records(Some(&filter)).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, "RFC");
        assert_eq!(records[0].body, "b2");
    }
}
