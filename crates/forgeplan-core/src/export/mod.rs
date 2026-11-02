use serde::{Deserialize, Serialize};

use crate::db::store::{ArtifactRecord, LanceStore, NewArtifact};

/// Full export of all artifacts + relations.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
    pub version: String,
    pub exported_at: String,
    pub artifacts: Vec<ArtifactRecord>,
    pub relations: Vec<(String, String, String)>,
}

/// Result of an import operation.
#[derive(Debug)]
pub struct ImportResult {
    pub created: usize,
    pub skipped: usize,
    pub relations_created: usize,
}

/// Export all data from LanceStore to ExportData.
pub async fn export_all(store: &LanceStore, permit: &dyn crate::workspace::lock::WorkspaceReadPermit) -> anyhow::Result<ExportData> {
    let artifacts = store.list_records(permit, None).await?;
    let relations = store.get_all_relations(permit).await?;

    Ok(ExportData {
        version: "1.0".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        artifacts,
        relations,
    })
}

/// Import data into LanceStore. Skip existing artifacts unless force=true.
pub async fn import_all(
    store: &LanceStore,
    guard: &crate::workspace::lock::WorkspaceLock,
    data: &ExportData,
    force: bool,
) -> anyhow::Result<ImportResult> {
    let mut created = 0usize;
    let mut skipped = 0usize;

    for record in &data.artifacts {
        // A bundle carries the document's identity. Two records can share a
        // display id (the alias is not unique), so the key — not the id —
        // decides which document exists, which one is replaced, and what the
        // projected file is called. A legacy bundle without a key falls back
        // to the id, which is what it always used.
        let canonical_key = if record.canonical_key.trim().is_empty() {
            record.id.clone()
        } else {
            record.canonical_key.clone()
        };
        let exists = store.get_record(guard, &canonical_key).await?.is_some();

        if exists && !force {
            skipped += 1;
            continue;
        }

        if exists {
            // force: delete existing, then re-create
            store.delete_artifact(guard, &canonical_key).await?;
        }

        let new = NewArtifact {
            // ADR-030: no minted key yet — the artifact declares no slug, so
            // its canonical key is its display id.
            canonical_key: canonical_key.clone(),
            id: record.id.clone(),
            kind: record.kind.clone(),
            status: record.status.clone(),
            title: record.title.clone(),
            body: record.body.clone(),
            depth: record.depth.clone(),
            author: record.author.clone(),
            parent_epic: record.parent_epic.clone(),
            valid_until: record.valid_until.clone(),
            tags: record.tags.clone(),
        };
        store.create_artifact(guard, &new).await?;
        created += 1;
    }

    let mut relations_created = 0usize;
    for (source, target, relation) in &data.relations {
        store.add_relation(guard, source, target, relation).await?;
        relations_created += 1;
    }

    Ok(ImportResult {
        created,
        skipped,
        relations_created,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::store::LanceStore;
    use tempfile::TempDir;

    async fn make_store() -> (LanceStore, TempDir) {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let store = { let guard = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap(); LanceStore::init_new(&ws, &guard).await.unwrap() };
        (store, tmp)
    }

    #[tokio::test]
    async fn export_rejects_a_permit_from_another_workspace() {
        let (store, _tmp) = make_store().await;
        let (other, _other_tmp) = make_store().await;
        let wrong = crate::workspace::lock::acquire_workspace_read_lock(other.workspace_path()).await.unwrap();
        assert!(export_all(&store, &wrong).await.is_err());
        let correct = crate::workspace::lock::acquire_workspace_read_lock(store.workspace_path()).await.unwrap();
        assert!(export_all(&store, &correct).await.is_ok());
    }

    #[tokio::test]
    async fn export_empty_store() {
        let (store, _tmp, guard) = { let (store, _tmp) = make_store().await; let guard = crate::workspace::lock::acquire_workspace_lock(store.workspace_path()).await.unwrap(); (store, _tmp, guard) };
        let data = export_all(&store, &guard).await.unwrap();
        assert_eq!(data.version, "1.0");
        assert!(data.artifacts.is_empty());
        assert!(data.relations.is_empty());
    }

    #[tokio::test]
    async fn export_with_data() {
        let (store, _tmp, guard) = { let (store, _tmp) = make_store().await; let guard = crate::workspace::lock::acquire_workspace_lock(store.workspace_path()).await.unwrap(); (store, _tmp, guard) };

        let artifact = NewArtifact {
            canonical_key: "prd-001".to_string(),
            id: "prd-001".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Test PRD".to_string(),
            body: "# Test\n\nBody".to_string(),
            depth: "standard".to_string(),
            author: Some("test".to_string()),
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        };
        store.create_artifact(&guard, &artifact).await.unwrap();

        let data = export_all(&store, &guard).await.unwrap();
        assert_eq!(data.artifacts.len(), 1);
        assert_eq!(data.artifacts[0].id, "prd-001");
        assert_eq!(data.artifacts[0].title, "Test PRD");
    }

    #[tokio::test]
    async fn import_skip_existing() {
        let (store, _tmp, guard) = { let (store, _tmp) = make_store().await; let guard = crate::workspace::lock::acquire_workspace_lock(store.workspace_path()).await.unwrap(); (store, _tmp, guard) };

        let artifact = NewArtifact {
            canonical_key: "prd-001".to_string(),
            id: "prd-001".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Original".to_string(),
            body: "body".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        };
        store.create_artifact(&guard, &artifact).await.unwrap();

        let data = ExportData {
            version: "1.0".to_string(),
            exported_at: "2026-01-01T00:00:00Z".to_string(),
            artifacts: vec![ArtifactRecord {
                canonical_key: "prd-001".to_string(),
                id: "prd-001".to_string(),
                kind: "prd".to_string(),
                status: "draft".to_string(),
                title: "Imported".to_string(),
                body: "new body".to_string(),
                depth: "standard".to_string(),
                author: None,
                parent_epic: None,
                r_eff_score: 0.0,
                valid_until: None,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
                tags: Vec::new(),
                body_hash: None,
                embedding: None,
            }],
            relations: vec![],
        };

        let result = import_all(&store, &guard, &data, false).await.unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.skipped, 1);

        // Title unchanged
        let rec = store.get_record(&guard, "prd-001").await.unwrap().unwrap();
        assert_eq!(rec.title, "Original");
    }

    #[tokio::test]
    async fn import_force_overwrites() {
        let (store, _tmp, guard) = { let (store, _tmp) = make_store().await; let guard = crate::workspace::lock::acquire_workspace_lock(store.workspace_path()).await.unwrap(); (store, _tmp, guard) };

        let artifact = NewArtifact {
            canonical_key: "prd-002".to_string(),
            id: "prd-002".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Original".to_string(),
            body: "body".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        };
        store.create_artifact(&guard, &artifact).await.unwrap();

        let data = ExportData {
            version: "1.0".to_string(),
            exported_at: "2026-01-01T00:00:00Z".to_string(),
            artifacts: vec![ArtifactRecord {
                canonical_key: "prd-002".to_string(),
                id: "prd-002".to_string(),
                kind: "prd".to_string(),
                status: "active".to_string(),
                title: "Overwritten".to_string(),
                body: "new body".to_string(),
                depth: "deep".to_string(),
                author: Some("importer".to_string()),
                parent_epic: None,
                r_eff_score: 0.5,
                valid_until: None,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
                tags: Vec::new(),
                body_hash: None,
                embedding: None,
            }],
            relations: vec![],
        };

        let result = import_all(&store, &guard, &data, true).await.unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.skipped, 0);

        let rec = store.get_record(&guard, "prd-002").await.unwrap().unwrap();
        assert_eq!(rec.title, "Overwritten");
    }
}
