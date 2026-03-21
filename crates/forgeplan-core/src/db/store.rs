use std::path::Path;
use std::sync::Arc;

use arrow_array::{Array, Float64Array, Int32Array, RecordBatch, StringArray};
use arrow_schema::ArrowError;
use chrono::Utc;
use futures::StreamExt;
use lancedb::connection::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::Table;

use crate::artifact::store::ArtifactSummary;
use crate::db::{convert, schema};

/// Filter for listing artifacts.
#[derive(Debug, Default)]
pub struct ArtifactFilter {
    pub kind: Option<String>,
    pub status: Option<String>,
}

/// Minimal artifact data for creation.
#[derive(Debug)]
pub struct NewArtifact {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub title: String,
    pub body: String,
    pub depth: String,
    pub author: Option<String>,
    pub parent_epic: Option<String>,
    pub valid_until: Option<String>,
}

/// LanceDB-backed artifact store.
pub struct LanceStore {
    _db: Connection,
    artifacts: Table,
    #[allow(dead_code)]
    evidence: Table,
    relations: Table,
}

impl LanceStore {
    /// Connect to an existing LanceDB workspace (tables must already exist).
    pub async fn open(workspace_path: &Path) -> anyhow::Result<Self> {
        let lance_dir = workspace_path.join("lance");
        let db = lancedb::connect(lance_dir.to_str().ok_or_else(|| anyhow::anyhow!("LanceDB path contains non-UTF-8 characters: {:?}", lance_dir))?).execute().await?;

        let artifacts = db.open_table("artifacts").execute().await?;
        let evidence = db.open_table("evidence").execute().await?;
        let relations = db.open_table("relations").execute().await?;

        Ok(Self {
            _db: db,
            artifacts,
            evidence,
            relations,
        })
    }

    /// Create tables if they don't exist, then open the store.
    pub async fn init(workspace_path: &Path) -> anyhow::Result<Self> {
        let lance_dir = workspace_path.join("lance");
        tokio::fs::create_dir_all(&lance_dir).await?;

        let db = lancedb::connect(lance_dir.to_str().ok_or_else(|| anyhow::anyhow!("LanceDB path contains non-UTF-8 characters: {:?}", lance_dir))?).execute().await?;
        let existing_tables = db.table_names().execute().await?;

        // Create artifacts table if not present
        if !existing_tables.contains(&"artifacts".to_string()) {
            let schema = schema::artifacts_schema();
            let batch = empty_artifacts_batch(schema.clone())?;
            db.create_table("artifacts", vec![batch]).execute().await?;
        }

        // Create evidence table if not present
        if !existing_tables.contains(&"evidence".to_string()) {
            let schema = schema::evidence_schema();
            let batch = empty_evidence_batch(schema.clone())?;
            db.create_table("evidence", vec![batch]).execute().await?;
        }

        // Create relations table if not present
        if !existing_tables.contains(&"relations".to_string()) {
            let schema = schema::relations_schema();
            let batch = empty_relations_batch(schema.clone())?;
            db.create_table("relations", vec![batch]).execute().await?;
        }

        let artifacts = db.open_table("artifacts").execute().await?;
        let evidence = db.open_table("evidence").execute().await?;
        let relations = db.open_table("relations").execute().await?;

        Ok(Self {
            _db: db,
            artifacts,
            evidence,
            relations,
        })
    }

    /// Insert a new artifact, returning its ID.
    pub async fn create_artifact(&self, artifact: &NewArtifact) -> anyhow::Result<String> {
        // Guard: check for duplicate ID
        if self.get_artifact(&artifact.id).await?.is_some() {
            anyhow::bail!("Artifact '{}' already exists", artifact.id);
        }

        let now = Utc::now().to_rfc3339();
        let batch = convert::artifact_to_batch(artifact, &now)?;

        self.artifacts.add(vec![batch]).execute().await?;
        Ok(artifact.id.clone())
    }

    /// Get a single artifact by ID. Returns None if not found.
    pub async fn get_artifact(&self, id: &str) -> anyhow::Result<Option<ArtifactSummary>> {
        let filter = format!("id = '{}'", id.replace('\'', "''"));
        let batches = collect_batches(
            self.artifacts
                .query()
                .only_if(filter)
                .execute()
                .await?,
        )
        .await?;

        for batch in &batches {
            if batch.num_rows() == 0 {
                continue;
            }
            if let Some(summary) = extract_summary(batch, 0) {
                return Ok(Some(summary));
            }
        }
        Ok(None)
    }

    /// List artifacts with optional kind/status filter.
    pub async fn list_artifacts(
        &self,
        filter: Option<&ArtifactFilter>,
    ) -> anyhow::Result<Vec<ArtifactSummary>> {
        let mut query = self.artifacts.query();

        if let Some(f) = filter {
            let mut conditions = Vec::new();
            if let Some(kind) = &f.kind {
                conditions.push(format!("kind = '{}'", kind.replace('\'', "''")));
            }
            if let Some(status) = &f.status {
                conditions.push(format!("status = '{}'", status.replace('\'', "''")));
            }
            if !conditions.is_empty() {
                query = query.only_if(conditions.join(" AND "));
            }
        }

        let batches = collect_batches(query.execute().await?).await?;

        let mut results = Vec::new();
        for batch in &batches {
            for row in 0..batch.num_rows() {
                if let Some(summary) = extract_summary(batch, row) {
                    results.push(summary);
                }
            }
        }
        results.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(results)
    }

    /// Update artifact updated_at (and optionally status/title).
    pub async fn update_artifact(
        &self,
        id: &str,
        status: Option<&str>,
        title: Option<&str>,
    ) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        let predicate = format!("id = '{}'", id.replace('\'', "''"));
        let mut builder = self
            .artifacts
            .update()
            .only_if(predicate)
            .column("updated_at", &format!("'{}'", now));
        if let Some(s) = status {
            builder = builder.column("status", &format!("'{}'", s.replace('\'', "''")));
        }
        if let Some(t) = title {
            builder = builder.column("title", &format!("'{}'", t.replace('\'', "''")));
        }
        builder.execute().await?;
        Ok(())
    }

    /// Delete an artifact by ID.
    pub async fn delete_artifact(&self, id: &str) -> anyhow::Result<()> {
        let predicate = format!("id = '{}'", id.replace('\'', "''"));
        self.artifacts.delete(&predicate).await?;
        Ok(())
    }

    /// Add a relation between two artifacts. Rejects duplicates.
    pub async fn add_relation(
        &self,
        source: &str,
        target: &str,
        relation: &str,
    ) -> anyhow::Result<()> {
        // Dedup: check if relation already exists
        let existing = self.get_relations(source).await?;
        let duplicate = existing.iter().any(|(t, r)| {
            t.eq_ignore_ascii_case(target) && r == relation
        });
        if duplicate {
            anyhow::bail!(
                "Relation already exists: {} --{}--> {}",
                source, relation, target
            );
        }

        let now = Utc::now().to_rfc3339();
        let batch = convert::relation_to_batch(source, target, relation, &now)?;

        self.relations.add(vec![batch]).execute().await?;
        Ok(())
    }

    /// Get all relations for an artifact (as source).
    /// Returns Vec<(target_id, relation_type)>.
    pub async fn get_relations(&self, id: &str) -> anyhow::Result<Vec<(String, String)>> {
        let filter = format!("source_id = '{}'", id.replace('\'', "''"));
        let batches = collect_batches(
            self.relations
                .query()
                .only_if(filter)
                .execute()
                .await?,
        )
        .await?;

        let mut results = Vec::new();
        for batch in &batches {
            let target_col = batch
                .column_by_name("target_id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let rel_col = batch
                .column_by_name("relation_type")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            if let (Some(targets), Some(rels)) = (target_col, rel_col) {
                for i in 0..batch.num_rows() {
                    if !targets.is_null(i) && !rels.is_null(i) {
                        results.push((
                            targets.value(i).to_string(),
                            rels.value(i).to_string(),
                        ));
                    }
                }
            }
        }
        Ok(results)
    }
}

/// Collect a stream of RecordBatches into a Vec.
async fn collect_batches(
    stream: lancedb::arrow::SendableRecordBatchStream,
) -> anyhow::Result<Vec<RecordBatch>> {
    let mut batches = Vec::new();
    let mut stream = std::pin::pin!(stream);
    while let Some(result) = stream.next().await {
        batches.push(result?);
    }
    Ok(batches)
}

/// Extract an ArtifactSummary from a RecordBatch row.
fn extract_summary(batch: &RecordBatch, row: usize) -> Option<ArtifactSummary> {
    let id = get_string(batch, "id", row)?;
    let title = get_string(batch, "title", row).unwrap_or_default();
    let kind = get_string(batch, "kind", row).unwrap_or_default();
    let status = get_string(batch, "status", row).unwrap_or_default();

    Some(ArtifactSummary {
        id,
        title,
        kind,
        status,
        path: std::path::PathBuf::new(), // LanceDB store doesn't use file paths
    })
}

fn get_string(batch: &RecordBatch, col: &str, row: usize) -> Option<String> {
    let array = batch.column_by_name(col)?;
    if let Some(arr) = array.as_any().downcast_ref::<StringArray>() {
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    } else {
        None
    }
}

/// Create an empty RecordBatch for artifacts (used when initializing tables).
fn empty_artifacts_batch(
    schema: Arc<arrow_schema::Schema>,
) -> Result<RecordBatch, ArrowError> {
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(arrow_array::LargeStringArray::from(Vec::<&str>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(StringArray::from(Vec::<Option<&str>>::new())),
            Arc::new(StringArray::from(Vec::<Option<&str>>::new())),
            Arc::new(Float64Array::from(Vec::<f64>::new())),
            Arc::new(StringArray::from(Vec::<Option<&str>>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
            convert::make_null_embedding_col(0),
        ],
    )
}

/// Create an empty RecordBatch for evidence.
fn empty_evidence_batch(
    schema: Arc<arrow_schema::Schema>,
) -> Result<RecordBatch, ArrowError> {
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(Int32Array::from(Vec::<i32>::new())),
            Arc::new(StringArray::from(Vec::<Option<&str>>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
        ],
    )
}

/// Create an empty RecordBatch for relations.
fn empty_relations_batch(
    schema: Arc<arrow_schema::Schema>,
) -> Result<RecordBatch, ArrowError> {
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
            Arc::new(StringArray::from(Vec::<&str>::new())),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn make_store(tmp: &TempDir) -> LanceStore {
        let ws = tmp.path().join(".forgeplan");
        LanceStore::init(&ws).await.unwrap()
    }

    fn sample_artifact(id: &str) -> NewArtifact {
        NewArtifact {
            id: id.to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: format!("Test PRD {}", id),
            body: "## Summary\n\nTest body content.".to_string(),
            depth: "standard".to_string(),
            author: Some("test-author".to_string()),
            parent_epic: None,
            valid_until: None,
        }
    }

    #[tokio::test]
    async fn init_creates_tables() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let store = LanceStore::init(&ws).await.unwrap();

        // Should be able to list (empty) artifacts without error
        let artifacts = store.list_artifacts(None).await.unwrap();
        assert!(artifacts.is_empty());
    }

    #[tokio::test]
    async fn create_and_get_artifact() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        let artifact = sample_artifact("PRD-001");
        let id = store.create_artifact(&artifact).await.unwrap();
        assert_eq!(id, "PRD-001");

        let retrieved = store.get_artifact("PRD-001").await.unwrap();
        assert!(retrieved.is_some());
        let summary = retrieved.unwrap();
        assert_eq!(summary.id, "PRD-001");
        assert_eq!(summary.kind, "prd");
        assert_eq!(summary.status, "draft");
    }

    #[tokio::test]
    async fn get_nonexistent_artifact_returns_none() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        let result = store.get_artifact("MISSING-999").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn list_artifacts_returns_all() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.create_artifact(&sample_artifact("PRD-001")).await.unwrap();
        store.create_artifact(&sample_artifact("PRD-002")).await.unwrap();

        let list = store.list_artifacts(None).await.unwrap();
        assert_eq!(list.len(), 2);
        // sorted by id
        assert_eq!(list[0].id, "PRD-001");
        assert_eq!(list[1].id, "PRD-002");
    }

    #[tokio::test]
    async fn list_artifacts_with_kind_filter() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.create_artifact(&sample_artifact("PRD-001")).await.unwrap();

        let mut rfc = sample_artifact("RFC-001");
        rfc.kind = "rfc".to_string();
        store.create_artifact(&rfc).await.unwrap();

        let filter = ArtifactFilter {
            kind: Some("prd".to_string()),
            status: None,
        };
        let list = store.list_artifacts(Some(&filter)).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "PRD-001");
    }

    #[tokio::test]
    async fn delete_artifact() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.create_artifact(&sample_artifact("PRD-001")).await.unwrap();
        store.delete_artifact("PRD-001").await.unwrap();

        let result = store.get_artifact("PRD-001").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn add_and_get_relations() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.add_relation("PRD-001", "RFC-001", "informs").await.unwrap();
        store.add_relation("PRD-001", "ADR-001", "based_on").await.unwrap();

        let relations = store.get_relations("PRD-001").await.unwrap();
        assert_eq!(relations.len(), 2);

        let targets: Vec<&str> = relations.iter().map(|(t, _)| t.as_str()).collect();
        assert!(targets.contains(&"RFC-001"));
        assert!(targets.contains(&"ADR-001"));
    }
}
