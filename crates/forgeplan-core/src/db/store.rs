use std::collections::BTreeMap;
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
use crate::changelog::ChangeLogEntry;
use crate::db::{convert, migrate, schema};

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

/// Full artifact record from LanceDB — includes body and all metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactRecord {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub title: String,
    pub body: String,
    pub depth: String,
    pub author: Option<String>,
    pub parent_epic: Option<String>,
    pub r_eff_score: f64,
    pub valid_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Vector search result: an artifact record paired with its distance from the query.
///
/// `distance` is the raw value from LanceDB (L2 or cosine distance).
/// For cosine distance: 0.0 = identical, 2.0 = opposite.
/// Convert to similarity via `similarity()` method.
#[derive(Debug, Clone)]
pub struct VectorSearchHit {
    pub record: ArtifactRecord,
    pub distance: f64,
}

impl VectorSearchHit {
    /// Convert cosine distance to similarity score in [0.0, 1.0].
    ///
    /// cosine_distance ∈ [0, 2], similarity = 1.0 - distance/2.0
    pub fn similarity(&self) -> f64 {
        (1.0 - self.distance / 2.0).clamp(0.0, 1.0)
    }
}

impl ArtifactRecord {
    /// Build the text used for embedding: title + first `chunk_size` chars of body.
    ///
    /// ID is excluded — it has no semantic meaning and pollutes the vector space.
    /// Default chunk_size=2000 captures Problem/Goals/FR sections.
    /// Configurable via `embedding.chunk_size` in config.yaml.
    pub fn embedding_text(&self, chunk_size: usize) -> String {
        let body_preview: String = self.body.chars().take(chunk_size).collect();
        format!("{} {}", self.title, body_preview)
    }

    /// Convert to a lightweight ArtifactSummary (drops body and most metadata).
    pub fn to_summary(&self) -> ArtifactSummary {
        ArtifactSummary {
            id: self.id.clone(),
            title: self.title.clone(),
            kind: self.kind.clone(),
            status: self.status.clone(),
        }
    }

    /// Reconstruct YAML frontmatter fields as a BTreeMap for serialization.
    pub fn frontmatter_map(&self) -> BTreeMap<String, serde_yml::Value> {
        use serde_yml::Value;

        let mut map = BTreeMap::new();
        map.insert("id".to_string(), Value::String(self.id.clone()));
        map.insert("kind".to_string(), Value::String(self.kind.clone()));
        map.insert("status".to_string(), Value::String(self.status.clone()));
        map.insert("title".to_string(), Value::String(self.title.clone()));
        map.insert("depth".to_string(), Value::String(self.depth.clone()));
        if let Some(ref author) = self.author {
            map.insert("author".to_string(), Value::String(author.clone()));
        }
        if let Some(ref parent) = self.parent_epic {
            map.insert("parent_epic".to_string(), Value::String(parent.clone()));
        }
        map.insert(
            "r_eff_score".to_string(),
            Value::Number(serde_yml::Number::from(self.r_eff_score)),
        );
        if let Some(ref vu) = self.valid_until {
            map.insert("valid_until".to_string(), Value::String(vu.clone()));
        }
        map.insert("created_at".to_string(), Value::String(self.created_at.clone()));
        map.insert("updated_at".to_string(), Value::String(self.updated_at.clone()));
        map
    }
}

/// Compute a simple fingerprint hash for artifact body content.
///
/// Uses a lightweight approach (length + byte sum) to avoid adding sha2 dependency.
/// Sufficient for change detection across artifact versions.
pub fn compute_body_hash(body: &str) -> String {
    let bytes = body.as_bytes();
    let len = bytes.len();
    // Simple hash: sum of bytes with position weighting
    let hash: u64 = bytes
        .iter()
        .enumerate()
        .fold(0u64, |acc, (i, &b)| {
            acc.wrapping_add((b as u64).wrapping_mul(i as u64 + 1))
        });
    format!("{:016x}-{:08x}", hash, len)
}

/// FPF knowledge base chunk.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FpfChunk {
    pub id: String,
    pub section_id: String,
    pub parent_section: Option<String>,
    pub title: String,
    pub body: String,
    pub line_count: i32,
    pub file_path: String,
    pub created_at: String,
}

/// FPF chunk summary (without body for listing).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FpfChunkSummary {
    pub section_id: String,
    pub title: String,
    pub line_count: i32,
}

/// LanceDB-backed artifact store.
pub struct LanceStore {
    _db: Connection,
    artifacts: Table,
    #[allow(dead_code)]
    evidence: Table,
    relations: Table,
    fpf_spec: Option<Table>,
    change_log: Option<Table>,
}

impl LanceStore {
    /// Connect to an existing LanceDB workspace (tables must already exist).
    /// Runs idempotent schema migrations on every open.
    pub async fn open(workspace_path: &Path) -> anyhow::Result<Self> {
        let lance_dir = workspace_path.join("lance");
        let db = lancedb::connect(lance_dir.to_str().ok_or_else(|| anyhow::anyhow!("LanceDB path contains non-UTF-8 characters: {:?}", lance_dir))?).execute().await?;

        let artifacts = db.open_table("artifacts").execute().await?;
        let evidence = db.open_table("evidence").execute().await?;
        let relations = db.open_table("relations").execute().await?;
        let fpf_spec = db.open_table("fpf_spec").execute().await.ok();

        // Run migrations (idempotent — safe on every open)
        migrate::run_migrations(&artifacts, &relations).await?;

        // Ensure change_log table exists (migration for older workspaces)
        migrate::ensure_change_log(&db).await?;
        let change_log = db.open_table("change_log").execute().await.ok();

        Ok(Self {
            _db: db,
            artifacts,
            evidence,
            relations,
            fpf_spec,
            change_log,
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

        // Create fpf_spec table if not present
        if !existing_tables.contains(&"fpf_spec".to_string()) {
            let schema = schema::fpf_spec_schema();
            let batch = empty_fpf_batch(schema.clone())?;
            db.create_table("fpf_spec", vec![batch]).execute().await?;
        }

        // Create change_log table if not present
        if !existing_tables.contains(&"change_log".to_string()) {
            let schema = schema::change_log_schema();
            let batch = empty_change_log_batch(schema.clone())?;
            db.create_table("change_log", vec![batch]).execute().await?;
        }

        let artifacts = db.open_table("artifacts").execute().await?;
        let evidence = db.open_table("evidence").execute().await?;
        let relations = db.open_table("relations").execute().await?;
        let fpf_spec = db.open_table("fpf_spec").execute().await.ok();
        let change_log = db.open_table("change_log").execute().await.ok();

        Ok(Self {
            _db: db,
            artifacts,
            evidence,
            relations,
            fpf_spec,
            change_log,
        })
    }

    /// Insert a new artifact, returning its ID.
    pub async fn create_artifact(&self, artifact: &NewArtifact) -> anyhow::Result<String> {
        // Validate ID format — prevent path traversal and SQL injection
        if !artifact.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            anyhow::bail!("Invalid artifact ID '{}': must contain only alphanumeric characters and hyphens", artifact.id);
        }

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

    /// Update the depth column of an artifact.
    pub async fn update_depth(&self, id: &str, depth: &str) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        let predicate = format!("id = '{}'", id.replace('\'', "''"));
        self.artifacts
            .update()
            .only_if(predicate)
            .column("updated_at", &format!("'{}'", now))
            .column("depth", &format!("'{}'", depth.replace('\'', "''")))
            .execute()
            .await?;
        Ok(())
    }

    /// Update r_eff_score for an artifact. Verifies the artifact exists first.
    pub async fn update_r_eff_score(&self, id: &str, score: f64) -> anyhow::Result<()> {
        let score = if score.is_nan() { 0.0 } else { score.clamp(0.0, 1.0) };
        // Verify artifact exists before update (LanceDB update is silent no-op on missing ID)
        if self.get_record(id).await?.is_none() {
            anyhow::bail!("Cannot update R_eff: artifact '{}' not found", id);
        }
        let now = Utc::now().to_rfc3339();
        let predicate = format!("id = '{}'", id.replace('\'', "''"));
        self.artifacts
            .update()
            .only_if(predicate)
            .column("updated_at", &format!("'{}'", now))
            .column("r_eff_score", &format!("{score}"))
            .execute()
            .await?;
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

    /// Remove a specific relation between two artifacts.
    pub async fn delete_relation(
        &self,
        source: &str,
        target: &str,
        relation: &str,
    ) -> anyhow::Result<()> {
        let filter = format!(
            "source_id = '{}' AND target_id = '{}' AND relation_type = '{}'",
            source.replace('\'', "''"),
            target.replace('\'', "''"),
            relation.replace('\'', "''"),
        );
        self.relations.delete(&filter).await?;
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

    /// Get incoming relations where this artifact is the TARGET.
    /// Returns Vec<(source_id, relation_type)>.
    pub async fn get_incoming_relations(&self, id: &str) -> anyhow::Result<Vec<(String, String)>> {
        let filter = format!("target_id = '{}'", id.replace('\'', "''"));
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
            let source_col = batch
                .column_by_name("source_id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let rel_col = batch
                .column_by_name("relation_type")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            if let (Some(sources), Some(rels)) = (source_col, rel_col) {
                for i in 0..batch.num_rows() {
                    if !sources.is_null(i) && !rels.is_null(i) {
                        results.push((
                            sources.value(i).to_string(),
                            rels.value(i).to_string(),
                        ));
                    }
                }
            }
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Full-record methods (ArtifactRecord)
    // -----------------------------------------------------------------------

    /// Get a single artifact by ID as a full record. Returns None if not found.
    pub async fn get_record(&self, id: &str) -> anyhow::Result<Option<ArtifactRecord>> {
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
            if let Some(record) = extract_record(batch, 0) {
                return Ok(Some(record));
            }
        }
        Ok(None)
    }

    /// List artifacts as full records with optional kind/status filter.
    pub async fn list_records(
        &self,
        filter: Option<&ArtifactFilter>,
    ) -> anyhow::Result<Vec<ArtifactRecord>> {
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
                if let Some(record) = extract_record(batch, row) {
                    results.push(record);
                }
            }
        }
        results.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(results)
    }

    /// Search artifacts by body/title content using case-insensitive substring match.
    ///
    /// LanceDB SQL may not support LIKE on LargeUtf8, so this does a full scan
    /// and filters in Rust for maximum compatibility.
    pub async fn search_body(
        &self,
        query: &str,
        kind_filter: Option<&str>,
    ) -> anyhow::Result<Vec<ArtifactRecord>> {
        let filter = kind_filter.map(|k| ArtifactFilter {
            kind: Some(k.to_string()),
            status: None,
        });
        let all = self.list_records(filter.as_ref()).await?;

        let query_lower = query.to_lowercase();
        let results = all
            .into_iter()
            .filter(|r| {
                r.title.to_lowercase().contains(&query_lower)
                    || r.body.to_lowercase().contains(&query_lower)
            })
            .collect();
        Ok(results)
    }

    /// Vector similarity search using pre-computed embedding.
    /// Returns artifacts with distance scores, sorted by cosine distance (closest first).
    #[cfg(feature = "semantic-search")]
    pub async fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> anyhow::Result<Vec<VectorSearchHit>> {
        use lancedb::query::QueryBase;

        let results = self
            .artifacts
            .vector_search(query_embedding)
            .map_err(|e| anyhow::anyhow!("Vector search failed: {e}"))?
            .distance_type(lancedb::DistanceType::Cosine)
            .limit(limit)
            .execute()
            .await?;

        let batches: Vec<_> = futures::StreamExt::collect::<Vec<_>>(results).await;
        let mut hits = Vec::new();
        for batch_result in batches {
            let batch = batch_result?;
            for row in 0..batch.num_rows() {
                if let Some(record) = extract_record(&batch, row) {
                    let distance = get_f32(&batch, "_distance", row)
                        .map(|d| d as f64)
                        .unwrap_or(1.0); // fallback: mid-range distance
                    hits.push(VectorSearchHit { record, distance });
                }
            }
        }
        Ok(hits)
    }

    /// Update the embedding column for an artifact.
    pub async fn update_embedding(&self, id: &str, embedding: &[f32]) -> anyhow::Result<()> {
        let embedding_str = format!(
            "[{}]",
            embedding
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let predicate = format!("id = '{}'", id.replace('\'', "''"));
        self.artifacts
            .update()
            .only_if(predicate)
            .column("embedding", &embedding_str)
            .execute()
            .await?;
        Ok(())
    }

    /// Find artifacts where `valid_until` is set and is earlier than the current date.
    pub async fn find_stale(&self) -> anyhow::Result<Vec<ArtifactRecord>> {
        let today = chrono::Utc::now().date_naive();
        let all = self.list_records(None).await?;
        let mut stale: Vec<ArtifactRecord> = all.into_iter().filter(|r| {
            r.valid_until.as_ref().map_or(false, |vu| {
                chrono::NaiveDate::parse_from_str(vu, "%Y-%m-%d")
                    .or_else(|_| {
                        // Try parsing as full ISO datetime and extract date
                        chrono::DateTime::parse_from_rfc3339(vu)
                            .map(|dt| dt.date_naive())
                    })
                    .map(|d| d < today)
                    .unwrap_or(false)
            })
        }).collect();
        stale.sort_by(|a, b| a.valid_until.cmp(&b.valid_until));
        Ok(stale)
    }

    /// Compute the next sequential ID for a given kind prefix.
    ///
    /// Scans all existing artifact IDs matching the prefix, finds the maximum
    /// numeric suffix, and returns the next one (e.g., "PRD-003" if max is "PRD-002").
    /// If no artifacts exist with that prefix, returns "{PREFIX}-001".
    pub async fn next_id(&self, kind_prefix: &str) -> anyhow::Result<String> {
        let prefix_upper = kind_prefix.to_uppercase();
        let all = self.list_artifacts(None).await?;

        let mut max_num: u32 = 0;
        let search_prefix = format!("{}-", prefix_upper);
        for summary in &all {
            let id_upper = summary.id.to_uppercase();
            if let Some(rest) = id_upper.strip_prefix(&search_prefix) {
                if let Some(num_str) = rest.split('-').next() {
                    if let Ok(num) = num_str.parse::<u32>() {
                        max_num = max_num.max(num);
                    }
                }
            }
        }

        let next = max_num + 1;
        Ok(format!("{}-{:03}", prefix_upper, next))
    }

    /// Update the body column of an artifact.
    pub async fn update_body(&self, id: &str, body: &str) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        let predicate = format!("id = '{}'", id.replace('\'', "''"));
        let escaped_body = body.replace('\'', "''");
        // LanceDB coerces string literals to the column's schema type (LargeUtf8).
        self.artifacts
            .update()
            .only_if(predicate)
            .column("updated_at", &format!("'{}'", now))
            .column("body", &format!("'{}'", escaped_body))
            .execute()
            .await?;
        Ok(())
    }

    /// Get all relations across all artifacts.
    /// Returns Vec<(source_id, target_id, relation_type)>.
    pub async fn get_all_relations(&self) -> anyhow::Result<Vec<(String, String, String)>> {
        let batches = collect_batches(
            self.relations.query().execute().await?,
        )
        .await?;

        let mut results = Vec::new();
        for batch in &batches {
            let source_col = batch
                .column_by_name("source_id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let target_col = batch
                .column_by_name("target_id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let rel_col = batch
                .column_by_name("relation_type")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            if let (Some(sources), Some(targets), Some(rels)) = (source_col, target_col, rel_col) {
                for i in 0..batch.num_rows() {
                    if !sources.is_null(i) && !targets.is_null(i) && !rels.is_null(i) {
                        results.push((
                            sources.value(i).to_string(),
                            targets.value(i).to_string(),
                            rels.value(i).to_string(),
                        ));
                    }
                }
            }
        }
        Ok(results)
    }

    // ── Change Log ───────────────────────────────────────────────────

    /// Insert a change log entry.
    pub async fn log_change(&self, entry: &ChangeLogEntry) -> anyhow::Result<()> {
        let table = self
            .change_log
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("change_log table not initialized. Run `forgeplan init -y` to recreate workspace."))?;

        let batch = RecordBatch::try_new(
            schema::change_log_schema(),
            vec![
                Arc::new(StringArray::from(vec![entry.timestamp.as_str()])),
                Arc::new(StringArray::from(vec![entry.artifact_id.as_str()])),
                Arc::new(StringArray::from(vec![entry.action.as_str()])),
                Arc::new(StringArray::from(vec![entry.field.as_deref()])),
                Arc::new(StringArray::from(vec![entry.old_value.as_deref()])),
                Arc::new(StringArray::from(vec![entry.new_value.as_deref()])),
                Arc::new(StringArray::from(vec![entry.source.as_str()])),
                Arc::new(StringArray::from(vec![entry.commit_hash.as_deref()])),
            ],
        )?;

        table.add(vec![batch]).execute().await?;
        Ok(())
    }

    /// Query change log entries with optional artifact filter and source filter.
    pub async fn get_change_log(
        &self,
        artifact_id: Option<&str>,
        source: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<ChangeLogEntry>> {
        let table = self
            .change_log
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("change_log table not initialized"))?;

        let mut conditions: Vec<String> = Vec::new();
        if let Some(aid) = artifact_id {
            conditions.push(format!("artifact_id = '{}'", aid.replace('\'', "''")));
        }
        if let Some(src) = source {
            conditions.push(format!("source = '{}'", src.replace('\'', "''")));
        }

        let mut query = table.query();
        if !conditions.is_empty() {
            query = query.only_if(conditions.join(" AND "));
        }

        let batches = collect_batches(query.execute().await?).await?;
        let mut results: Vec<ChangeLogEntry> = Vec::new();

        for batch in &batches {
            for row in 0..batch.num_rows() {
                if let Some(entry) = extract_change_log_entry(batch, row) {
                    results.push(entry);
                }
            }
        }

        // Sort by timestamp descending (newest first)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        results.truncate(limit);
        Ok(results)
    }

    // ── FPF Knowledge Base ───────────────────────────────────────────

    /// Check if FPF knowledge base is loaded.
    pub fn has_fpf(&self) -> bool {
        self.fpf_spec.is_some()
    }

    /// Insert FPF chunks in batch.
    pub async fn insert_fpf_chunks(&self, chunks: &[FpfChunk]) -> anyhow::Result<usize> {
        let table = self
            .fpf_spec
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("FPF knowledge base not initialized. Run `forgeplan fpf ingest`"))?;

        let ids: Vec<&str> = chunks.iter().map(|c| c.id.as_str()).collect();
        let section_ids: Vec<&str> = chunks.iter().map(|c| c.section_id.as_str()).collect();
        let parent_sections: Vec<Option<&str>> =
            chunks.iter().map(|c| c.parent_section.as_deref()).collect();
        let titles: Vec<&str> = chunks.iter().map(|c| c.title.as_str()).collect();
        let bodies: Vec<&str> = chunks.iter().map(|c| c.body.as_str()).collect();
        let line_counts: Vec<i32> = chunks.iter().map(|c| c.line_count).collect();
        let file_paths: Vec<&str> = chunks.iter().map(|c| c.file_path.as_str()).collect();
        let created_ats: Vec<&str> = chunks.iter().map(|c| c.created_at.as_str()).collect();

        let batch = RecordBatch::try_new(
            schema::fpf_spec_schema(),
            vec![
                Arc::new(StringArray::from(ids)),
                Arc::new(StringArray::from(section_ids)),
                Arc::new(StringArray::from(parent_sections)),
                Arc::new(StringArray::from(titles)),
                Arc::new(arrow_array::LargeStringArray::from(bodies)),
                Arc::new(Int32Array::from(line_counts)),
                Arc::new(StringArray::from(file_paths)),
                Arc::new(StringArray::from(created_ats)),
            ],
        )?;

        table.add(vec![batch]).execute().await?;
        Ok(chunks.len())
    }

    /// Search FPF spec by keyword (case-insensitive substring match on title + body).
    pub async fn search_fpf(&self, query: &str, limit: usize) -> anyhow::Result<Vec<FpfChunk>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            anyhow::bail!("Search query cannot be empty");
        }

        let table = self
            .fpf_spec
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("FPF knowledge base not initialized"))?;

        let query_lower = trimmed.to_lowercase();
        // Split query into words for per-word OR matching
        let words: Vec<String> = query_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .map(|w| w.replace('\'', "''"))
            .collect();

        let filter = if words.is_empty() {
            let escaped = query_lower.replace('\'', "''");
            format!("LOWER(title) LIKE '%{}%' OR LOWER(body) LIKE '%{}%'", escaped, escaped)
        } else {
            words.iter()
                .map(|w| format!("(LOWER(title) LIKE '%{}%' OR LOWER(body) LIKE '%{}%')", w, w))
                .collect::<Vec<_>>()
                .join(" OR ")
        };

        // Fetch ALL matching then rank (small dataset ~204 sections)
        let batches = collect_batches(
            table.query().only_if(filter).execute().await?,
        )
        .await?;

        let mut scored: Vec<(FpfChunk, usize)> = Vec::new();
        for batch in &batches {
            for i in 0..batch.num_rows() {
                if let Some(chunk) = extract_fpf_chunk(batch, i) {
                    let mut score = 0usize;
                    let title_lower = chunk.title.to_lowercase();
                    let body_lower = chunk.body.to_lowercase();

                    for word in &words {
                        if title_lower.contains(word.as_str()) {
                            score += 50;
                        }
                        score += body_lower.matches(word.as_str()).count().min(20);
                    }
                    let all_in_title = words.len() > 1 && words.iter().all(|w| title_lower.contains(w.as_str()));
                    if all_in_title {
                        score += 100;
                    }
                    if title_lower.contains(&query_lower) {
                        score += 200;
                    }

                    scored.push((chunk, score));
                }
            }
        }

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        let results: Vec<FpfChunk> = scored.into_iter().take(limit).map(|(c, _)| c).collect();
        Ok(results)
    }

    /// Get a specific FPF section by section_id.
    pub async fn get_fpf_section(&self, section_id: &str) -> anyhow::Result<Option<FpfChunk>> {
        let table = self
            .fpf_spec
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("FPF knowledge base not initialized"))?;

        let filter = format!("section_id = '{}'", section_id.replace('\'', "''"));
        let batches = collect_batches(
            table.query().only_if(filter).limit(1).execute().await?,
        )
        .await?;

        for batch in &batches {
            if batch.num_rows() > 0 {
                return Ok(extract_fpf_chunk(batch, 0));
            }
        }
        Ok(None)
    }

    /// List all FPF sections (without body content for performance).
    pub async fn list_fpf_sections(&self) -> anyhow::Result<Vec<FpfChunkSummary>> {
        let table = self
            .fpf_spec
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("FPF knowledge base not initialized"))?;

        let batches = collect_batches(table.query().execute().await?).await?;

        let mut results = Vec::new();
        for batch in &batches {
            let id_col = batch
                .column_by_name("section_id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let title_col = batch
                .column_by_name("title")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let lines_col = batch
                .column_by_name("line_count")
                .and_then(|c| c.as_any().downcast_ref::<Int32Array>());

            if let (Some(ids), Some(titles), Some(lines)) = (id_col, title_col, lines_col) {
                for i in 0..batch.num_rows() {
                    if !ids.is_null(i) {
                        results.push(FpfChunkSummary {
                            section_id: ids.value(i).to_string(),
                            title: titles.value(i).to_string(),
                            line_count: lines.value(i),
                        });
                    }
                }
            }
        }
        Ok(results)
    }

    /// Delete all FPF chunks (for re-ingestion).
    pub async fn clear_fpf(&self) -> anyhow::Result<()> {
        let table = self
            .fpf_spec
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("FPF knowledge base not initialized"))?;
        table.delete("id IS NOT NULL").await?;
        Ok(())
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
    })
}

/// Extract a full ArtifactRecord from a RecordBatch row.
fn extract_record(batch: &RecordBatch, row: usize) -> Option<ArtifactRecord> {
    let id = get_string(batch, "id", row)?;
    let kind = get_string(batch, "kind", row).unwrap_or_default();
    let status = get_string(batch, "status", row).unwrap_or_default();
    let title = get_string(batch, "title", row).unwrap_or_default();
    let body = get_large_string(batch, "body", row).unwrap_or_default();
    let depth = get_string(batch, "depth", row).unwrap_or_default();
    let author = get_string(batch, "author", row);
    let parent_epic = get_string(batch, "parent_epic", row);
    let r_eff_score = get_f64(batch, "r_eff_score", row).unwrap_or(0.0);
    let valid_until = get_string(batch, "valid_until", row);
    let created_at = get_string(batch, "created_at", row).unwrap_or_default();
    let updated_at = get_string(batch, "updated_at", row).unwrap_or_default();
    Some(ArtifactRecord {
        id,
        kind,
        status,
        title,
        body,
        depth,
        author,
        parent_epic,
        r_eff_score,
        valid_until,
        created_at,
        updated_at,
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

/// Read a LargeUtf8 column value from a RecordBatch row.
fn get_large_string(batch: &RecordBatch, col: &str, row: usize) -> Option<String> {
    let array = batch.column_by_name(col)?;
    if let Some(arr) = array
        .as_any()
        .downcast_ref::<arrow_array::LargeStringArray>()
    {
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    } else {
        None
    }
}

/// Read a Float32 column value from a RecordBatch row.
#[cfg(feature = "semantic-search")]
fn get_f32(batch: &RecordBatch, col: &str, row: usize) -> Option<f32> {
    let array = batch.column_by_name(col)?;
    if let Some(arr) = array
        .as_any()
        .downcast_ref::<arrow_array::Float32Array>()
    {
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    } else {
        None
    }
}

/// Read a Float64 column value from a RecordBatch row.
fn get_f64(batch: &RecordBatch, col: &str, row: usize) -> Option<f64> {
    let array = batch.column_by_name(col)?;
    if let Some(arr) = array.as_any().downcast_ref::<Float64Array>() {
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    } else {
        None
    }
}

/// Extract an FpfChunk from a RecordBatch row.
fn extract_fpf_chunk(batch: &RecordBatch, row: usize) -> Option<FpfChunk> {
    let id = get_string(batch, "id", row)?;
    let section_id = get_string(batch, "section_id", row)?;
    let parent_section = get_string(batch, "parent_section", row);
    let title = get_string(batch, "title", row)?;
    let body = get_large_string(batch, "body", row)?;
    let line_count = {
        let array = batch.column_by_name("line_count")?;
        let arr = array.as_any().downcast_ref::<Int32Array>()?;
        if arr.is_null(row) {
            return None;
        }
        arr.value(row)
    };
    let file_path = get_string(batch, "file_path", row)?;
    let created_at = get_string(batch, "created_at", row)?;

    Some(FpfChunk {
        id,
        section_id,
        parent_section,
        title,
        body,
        line_count,
        file_path,
        created_at,
    })
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
            Arc::new(StringArray::from(Vec::<Option<&str>>::new())),
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
            Arc::new(Int32Array::from(Vec::<Option<i32>>::new())), // congruence_level
        ],
    )
}

/// Extract a ChangeLogEntry from a RecordBatch row.
fn extract_change_log_entry(batch: &RecordBatch, row: usize) -> Option<ChangeLogEntry> {
    let timestamp = get_string(batch, "timestamp", row)?;
    let artifact_id = get_string(batch, "artifact_id", row)?;
    let action = get_string(batch, "action", row)?;
    let field = get_string(batch, "field", row);
    let old_value = get_string(batch, "old_value", row);
    let new_value = get_string(batch, "new_value", row);
    let source = get_string(batch, "source", row)?;
    let commit_hash = get_string(batch, "commit_hash", row);

    Some(ChangeLogEntry {
        timestamp,
        artifact_id,
        action,
        field,
        old_value,
        new_value,
        source,
        commit_hash,
    })
}

/// Create an empty RecordBatch for change_log.
fn empty_change_log_batch(
    schema: Arc<arrow_schema::Schema>,
) -> Result<RecordBatch, ArrowError> {
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(Vec::<&str>::new())),       // timestamp
            Arc::new(StringArray::from(Vec::<&str>::new())),       // artifact_id
            Arc::new(StringArray::from(Vec::<&str>::new())),       // action
            Arc::new(StringArray::from(Vec::<Option<&str>>::new())), // field (nullable)
            Arc::new(StringArray::from(Vec::<Option<&str>>::new())), // old_value (nullable)
            Arc::new(StringArray::from(Vec::<Option<&str>>::new())), // new_value (nullable)
            Arc::new(StringArray::from(Vec::<&str>::new())),       // source
            Arc::new(StringArray::from(Vec::<Option<&str>>::new())), // commit_hash (nullable)
        ],
    )
}

fn empty_fpf_batch(
    schema: Arc<arrow_schema::Schema>,
) -> Result<RecordBatch, ArrowError> {
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(Vec::<&str>::new())),       // id
            Arc::new(StringArray::from(Vec::<&str>::new())),       // section_id
            Arc::new(StringArray::from(Vec::<Option<&str>>::new())), // parent_section (nullable)
            Arc::new(StringArray::from(Vec::<&str>::new())),       // title
            Arc::new(arrow_array::LargeStringArray::from(Vec::<&str>::new())), // body
            Arc::new(Int32Array::from(Vec::<i32>::new())),         // line_count
            Arc::new(StringArray::from(Vec::<&str>::new())),       // file_path
            Arc::new(StringArray::from(Vec::<&str>::new())),       // created_at
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

    // -----------------------------------------------------------------------
    // Tests for new full-record methods
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn get_record_returns_full_data() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        let artifact = sample_artifact("PRD-001");
        store.create_artifact(&artifact).await.unwrap();

        let record = store.get_record("PRD-001").await.unwrap();
        assert!(record.is_some());
        let r = record.unwrap();
        assert_eq!(r.id, "PRD-001");
        assert_eq!(r.kind, "prd");
        assert_eq!(r.status, "draft");
        assert_eq!(r.title, "Test PRD PRD-001");
        assert_eq!(r.body, "## Summary\n\nTest body content.");
        assert_eq!(r.depth, "standard");
        assert_eq!(r.author.as_deref(), Some("test-author"));
        assert!(r.parent_epic.is_none());
        assert!((r.r_eff_score - 0.0).abs() < f64::EPSILON);
        assert!(r.valid_until.is_none());
        assert!(!r.created_at.is_empty());
        assert!(!r.updated_at.is_empty());
    }

    #[tokio::test]
    async fn get_record_nonexistent_returns_none() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        let result = store.get_record("MISSING-999").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn list_records_returns_full_data() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.create_artifact(&sample_artifact("PRD-001")).await.unwrap();
        store.create_artifact(&sample_artifact("PRD-002")).await.unwrap();

        let records = store.list_records(None).await.unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id, "PRD-001");
        assert_eq!(records[1].id, "PRD-002");
        // Verify body is present
        assert!(records[0].body.contains("Test body content"));
    }

    #[tokio::test]
    async fn list_records_with_filter() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.create_artifact(&sample_artifact("PRD-001")).await.unwrap();
        let mut rfc = sample_artifact("RFC-001");
        rfc.kind = "rfc".to_string();
        store.create_artifact(&rfc).await.unwrap();

        let filter = ArtifactFilter {
            kind: Some("rfc".to_string()),
            status: None,
        };
        let records = store.list_records(Some(&filter)).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "RFC-001");
    }

    #[tokio::test]
    async fn search_body_finds_matching_content() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        let mut a1 = sample_artifact("PRD-001");
        a1.body = "This artifact is about authentication flow.".to_string();
        store.create_artifact(&a1).await.unwrap();

        let mut a2 = sample_artifact("PRD-002");
        a2.body = "This artifact covers database schema design.".to_string();
        store.create_artifact(&a2).await.unwrap();

        // Search for "authentication" — should match PRD-001 only
        let results = store.search_body("authentication", None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "PRD-001");
    }

    #[tokio::test]
    async fn search_body_case_insensitive() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        let mut a1 = sample_artifact("PRD-001");
        a1.body = "IMPORTANT: Security review needed.".to_string();
        store.create_artifact(&a1).await.unwrap();

        let results = store.search_body("security", None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "PRD-001");
    }

    #[tokio::test]
    async fn search_body_matches_title_too() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        let mut a1 = sample_artifact("PRD-001");
        a1.title = "Authentication Module Design".to_string();
        a1.body = "No relevant keywords here.".to_string();
        store.create_artifact(&a1).await.unwrap();

        let results = store.search_body("authentication", None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "PRD-001");
    }

    #[tokio::test]
    async fn search_body_with_kind_filter() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        let mut a1 = sample_artifact("PRD-001");
        a1.body = "Search target text here.".to_string();
        store.create_artifact(&a1).await.unwrap();

        let mut a2 = sample_artifact("RFC-001");
        a2.kind = "rfc".to_string();
        a2.body = "Search target text here too.".to_string();
        store.create_artifact(&a2).await.unwrap();

        // Filter to rfc only
        let results = store.search_body("target", Some("rfc")).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "RFC-001");
    }

    #[tokio::test]
    async fn find_stale_returns_expired_artifacts() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        // Create an artifact with valid_until in the past
        let mut stale = sample_artifact("PRD-001");
        stale.valid_until = Some("2020-01-01T00:00:00+00:00".to_string());
        store.create_artifact(&stale).await.unwrap();

        // Create an artifact with valid_until in the future
        let mut fresh = sample_artifact("PRD-002");
        fresh.valid_until = Some("2099-12-31T23:59:59+00:00".to_string());
        store.create_artifact(&fresh).await.unwrap();

        // Create an artifact with no valid_until
        store.create_artifact(&sample_artifact("PRD-003")).await.unwrap();

        let stale_results = store.find_stale().await.unwrap();
        assert_eq!(stale_results.len(), 1);
        assert_eq!(stale_results[0].id, "PRD-001");
    }

    #[tokio::test]
    async fn find_stale_empty_when_none_expired() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.create_artifact(&sample_artifact("PRD-001")).await.unwrap();

        let stale_results = store.find_stale().await.unwrap();
        assert!(stale_results.is_empty());
    }

    #[tokio::test]
    async fn next_id_starts_at_001() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        let id = store.next_id("PRD").await.unwrap();
        assert_eq!(id, "PRD-001");
    }

    #[tokio::test]
    async fn next_id_increments_correctly() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.create_artifact(&sample_artifact("PRD-001")).await.unwrap();
        store.create_artifact(&sample_artifact("PRD-002")).await.unwrap();

        let id = store.next_id("PRD").await.unwrap();
        assert_eq!(id, "PRD-003");
    }

    #[tokio::test]
    async fn next_id_ignores_other_kinds() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.create_artifact(&sample_artifact("PRD-001")).await.unwrap();
        let mut rfc = sample_artifact("RFC-001");
        rfc.kind = "rfc".to_string();
        store.create_artifact(&rfc).await.unwrap();

        let prd_next = store.next_id("PRD").await.unwrap();
        assert_eq!(prd_next, "PRD-002");

        let rfc_next = store.next_id("RFC").await.unwrap();
        assert_eq!(rfc_next, "RFC-002");
    }

    #[tokio::test]
    async fn next_id_case_insensitive_prefix() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.create_artifact(&sample_artifact("PRD-001")).await.unwrap();

        // Lowercase prefix should still find PRD-001
        let id = store.next_id("prd").await.unwrap();
        assert_eq!(id, "PRD-002");
    }

    #[tokio::test]
    async fn update_body_changes_content() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.create_artifact(&sample_artifact("PRD-001")).await.unwrap();

        store
            .update_body("PRD-001", "## Updated\n\nNew body content.")
            .await
            .unwrap();

        let record = store.get_record("PRD-001").await.unwrap().unwrap();
        assert_eq!(record.body, "## Updated\n\nNew body content.");
    }

    #[tokio::test]
    async fn get_all_relations_returns_everything() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.add_relation("PRD-001", "RFC-001", "informs").await.unwrap();
        store.add_relation("PRD-002", "ADR-001", "based_on").await.unwrap();

        let all = store.get_all_relations().await.unwrap();
        assert_eq!(all.len(), 2);

        let sources: Vec<&str> = all.iter().map(|(s, _, _)| s.as_str()).collect();
        assert!(sources.contains(&"PRD-001"));
        assert!(sources.contains(&"PRD-002"));
    }

    #[tokio::test]
    async fn get_all_relations_empty() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        let all = store.get_all_relations().await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn artifact_record_to_summary() {
        let record = ArtifactRecord {
            id: "PRD-001".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Test".to_string(),
            body: "body content".to_string(),
            depth: "standard".to_string(),
            author: Some("author".to_string()),
            parent_epic: None,
            r_eff_score: 0.5,
            valid_until: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let summary = record.to_summary();
        assert_eq!(summary.id, "PRD-001");
        assert_eq!(summary.kind, "prd");
        assert_eq!(summary.status, "draft");
        assert_eq!(summary.title, "Test");
    }

    #[tokio::test]
    async fn artifact_record_frontmatter_map() {
        let record = ArtifactRecord {
            id: "PRD-001".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Test Title".to_string(),
            body: "body".to_string(),
            depth: "standard".to_string(),
            author: Some("alice".to_string()),
            parent_epic: None,
            r_eff_score: 0.75,
            valid_until: Some("2025-06-01".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-06-01T00:00:00Z".to_string(),
        };

        let map = record.frontmatter_map();
        assert_eq!(map.get("id").unwrap(), &serde_yml::Value::String("PRD-001".to_string()));
        assert_eq!(map.get("kind").unwrap(), &serde_yml::Value::String("prd".to_string()));
        assert_eq!(map.get("author").unwrap(), &serde_yml::Value::String("alice".to_string()));
        assert_eq!(map.get("valid_until").unwrap(), &serde_yml::Value::String("2025-06-01".to_string()));
        // parent_epic should not be present (None)
        assert!(map.get("parent_epic").is_none());
        // Should have all expected keys
        assert!(map.contains_key("r_eff_score"));
        assert!(map.contains_key("created_at"));
        assert!(map.contains_key("updated_at"));
    }

    #[tokio::test]
    async fn update_r_eff_score_persists() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.create_artifact(&sample_artifact("PRD-001")).await.unwrap();

        // Default r_eff_score is 0.0
        let before = store.get_record("PRD-001").await.unwrap().unwrap();
        assert!((before.r_eff_score - 0.0).abs() < f64::EPSILON);

        store.update_r_eff_score("PRD-001", 0.85).await.unwrap();

        let after = store.get_record("PRD-001").await.unwrap().unwrap();
        assert!((after.r_eff_score - 0.85).abs() < f64::EPSILON);
        // updated_at should have changed
        assert_ne!(before.updated_at, after.updated_at);
    }

    #[tokio::test]
    async fn update_r_eff_score_missing_id_returns_error() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        let result = store.update_r_eff_score("NONEXISTENT-999", 0.5).await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("not found"),
            "Error should mention 'not found'"
        );
    }

    #[tokio::test]
    async fn update_r_eff_score_nan_becomes_zero() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;

        store.create_artifact(&sample_artifact("PRD-002")).await.unwrap();
        store.update_r_eff_score("PRD-002", f64::NAN).await.unwrap();

        let record = store.get_record("PRD-002").await.unwrap().unwrap();
        assert!((record.r_eff_score - 0.0).abs() < f64::EPSILON, "NaN should become 0.0");
    }

    #[tokio::test]
    async fn update_r_eff_score_clamps_out_of_range() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;
        store.create_artifact(&sample_artifact("PRD-003")).await.unwrap();

        // Above range → clamped to 1.0
        store.update_r_eff_score("PRD-003", 2.5).await.unwrap();
        let r = store.get_record("PRD-003").await.unwrap().unwrap();
        assert!((r.r_eff_score - 1.0).abs() < f64::EPSILON, "2.5 should clamp to 1.0");

        // Below range → clamped to 0.0
        store.update_r_eff_score("PRD-003", -0.5).await.unwrap();
        let r = store.get_record("PRD-003").await.unwrap().unwrap();
        assert!((r.r_eff_score - 0.0).abs() < f64::EPSILON, "-0.5 should clamp to 0.0");

        // Infinity → clamped to 1.0
        store.update_r_eff_score("PRD-003", f64::INFINITY).await.unwrap();
        let r = store.get_record("PRD-003").await.unwrap().unwrap();
        assert!((r.r_eff_score - 1.0).abs() < f64::EPSILON, "Infinity should clamp to 1.0");

        // Boundary: exact 0.0 and 1.0
        store.update_r_eff_score("PRD-003", 0.0).await.unwrap();
        let r = store.get_record("PRD-003").await.unwrap().unwrap();
        assert!((r.r_eff_score - 0.0).abs() < f64::EPSILON, "0.0 should stay 0.0");

        store.update_r_eff_score("PRD-003", 1.0).await.unwrap();
        let r = store.get_record("PRD-003").await.unwrap().unwrap();
        assert!((r.r_eff_score - 1.0).abs() < f64::EPSILON, "1.0 should stay 1.0");
    }

    #[test]
    fn embedding_text_includes_id_title_and_body_preview() {
        let record = ArtifactRecord {
            id: "PRD-042".to_string(),
            kind: "prd".to_string(),
            status: "Draft".to_string(),
            title: "Authentication Module".to_string(),
            body: "## Problem\nUsers cannot log in with OAuth2.\n## Goals\nSupport Google and GitHub OAuth.".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            r_eff_score: 0.0,
            valid_until: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };

        let text = record.embedding_text(2000);
        assert!(!text.contains("PRD-042"), "should NOT contain artifact id (no semantic value)");
        assert!(text.contains("Authentication Module"), "should contain title");
        assert!(text.contains("OAuth2"), "should contain body content");
        assert!(text.contains("Google and GitHub"), "should contain body goals");
    }

    #[test]
    fn embedding_text_truncates_body_at_chunk_size() {
        let long_body = "x".repeat(5000);
        let record = ArtifactRecord {
            id: "PRD-001".to_string(),
            kind: "prd".to_string(),
            status: "Draft".to_string(),
            title: "Title".to_string(),
            body: long_body,
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            r_eff_score: 0.0,
            valid_until: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };

        // Default chunk_size=2000
        let text = record.embedding_text(2000);
        let body_part = text.strip_prefix("Title ").unwrap();
        assert_eq!(body_part.len(), 2000, "body should be truncated to chunk_size");

        // Custom chunk_size=500
        let text_small = record.embedding_text(500);
        let body_small = text_small.strip_prefix("Title ").unwrap();
        assert_eq!(body_small.len(), 500, "body should respect custom chunk_size");
    }

    // ── VectorSearchHit similarity ──────────────────────────────────

    fn make_record(id: &str) -> ArtifactRecord {
        ArtifactRecord {
            id: id.to_string(),
            kind: "prd".to_string(),
            status: "Draft".to_string(),
            title: "Test".to_string(),
            body: String::new(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            r_eff_score: 0.0,
            valid_until: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn similarity_identical_vectors() {
        let hit = VectorSearchHit {
            record: make_record("PRD-001"),
            distance: 0.0, // cosine distance 0 = identical
        };
        assert_eq!(hit.similarity(), 1.0);
    }

    #[test]
    fn similarity_opposite_vectors() {
        let hit = VectorSearchHit {
            record: make_record("PRD-001"),
            distance: 2.0, // cosine distance 2 = opposite
        };
        assert_eq!(hit.similarity(), 0.0);
    }

    #[test]
    fn similarity_mid_distance() {
        let hit = VectorSearchHit {
            record: make_record("PRD-001"),
            distance: 1.0, // cosine distance 1 = orthogonal
        };
        assert!((hit.similarity() - 0.5).abs() < 0.001);
    }

    #[test]
    fn similarity_clamps_out_of_range() {
        // Distance > 2.0 (shouldn't happen, but defensive)
        let hit = VectorSearchHit {
            record: make_record("PRD-001"),
            distance: 3.0,
        };
        assert_eq!(hit.similarity(), 0.0);

        // Negative distance (shouldn't happen)
        let hit2 = VectorSearchHit {
            record: make_record("PRD-001"),
            distance: -0.5,
        };
        assert_eq!(hit2.similarity(), 1.0);
    }
}
