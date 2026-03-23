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
pub async fn export_all(store: &LanceStore) -> anyhow::Result<ExportData> {
    let artifacts = store.list_records(None).await?;
    let relations = store.get_all_relations().await?;

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
    data: &ExportData,
    force: bool,
) -> anyhow::Result<ImportResult> {
    let mut created = 0usize;
    let mut skipped = 0usize;

    for record in &data.artifacts {
        let exists = store.get_record(&record.id).await?.is_some();

        if exists && !force {
            skipped += 1;
            continue;
        }

        if exists {
            // force: delete existing, then re-create
            store.delete_artifact(&record.id).await?;
        }

        let new = NewArtifact {
            id: record.id.clone(),
            kind: record.kind.clone(),
            status: record.status.clone(),
            title: record.title.clone(),
            body: record.body.clone(),
            depth: record.depth.clone(),
            author: record.author.clone(),
            parent_epic: record.parent_epic.clone(),
            valid_until: record.valid_until.clone(),
        };
        store.create_artifact(&new).await?;
        created += 1;
    }

    let mut relations_created = 0usize;
    for (source, target, relation) in &data.relations {
        store.add_relation(source, target, relation).await?;
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
        let store = LanceStore::init(&ws).await.unwrap();
        (store, tmp)
    }

    #[tokio::test]
    async fn export_empty_store() {
        let (store, _tmp) = make_store().await;
        let data = export_all(&store).await.unwrap();
        assert_eq!(data.version, "1.0");
        assert!(data.artifacts.is_empty());
        assert!(data.relations.is_empty());
    }

    #[tokio::test]
    async fn export_with_data() {
        let (store, _tmp) = make_store().await;

        let artifact = NewArtifact {
            id: "prd-001".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Test PRD".to_string(),
            body: "# Test\n\nBody".to_string(),
            depth: "standard".to_string(),
            author: Some("test".to_string()),
            parent_epic: None,
            valid_until: None,
        };
        store.create_artifact(&artifact).await.unwrap();

        let data = export_all(&store).await.unwrap();
        assert_eq!(data.artifacts.len(), 1);
        assert_eq!(data.artifacts[0].id, "prd-001");
        assert_eq!(data.artifacts[0].title, "Test PRD");
    }

    #[tokio::test]
    async fn import_skip_existing() {
        let (store, _tmp) = make_store().await;

        let artifact = NewArtifact {
            id: "prd-001".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Original".to_string(),
            body: "body".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
        };
        store.create_artifact(&artifact).await.unwrap();

        let data = ExportData {
            version: "1.0".to_string(),
            exported_at: "2026-01-01T00:00:00Z".to_string(),
            artifacts: vec![ArtifactRecord {
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
            }],
            relations: vec![],
        };

        let result = import_all(&store, &data, false).await.unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.skipped, 1);

        // Title unchanged
        let rec = store.get_record("prd-001").await.unwrap().unwrap();
        assert_eq!(rec.title, "Original");
    }

    #[tokio::test]
    async fn import_force_overwrites() {
        let (store, _tmp) = make_store().await;

        let artifact = NewArtifact {
            id: "prd-002".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Original".to_string(),
            body: "body".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
        };
        store.create_artifact(&artifact).await.unwrap();

        let data = ExportData {
            version: "1.0".to_string(),
            exported_at: "2026-01-01T00:00:00Z".to_string(),
            artifacts: vec![ArtifactRecord {
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
            }],
            relations: vec![],
        };

        let result = import_all(&store, &data, true).await.unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.skipped, 0);

        let rec = store.get_record("prd-002").await.unwrap().unwrap();
        assert_eq!(rec.title, "Overwritten");
    }
}
