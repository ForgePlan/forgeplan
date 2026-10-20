//! Auxiliary-only store handle: the tables that belong to one feature.
//!
//! `LanceStore::init` creates `artifacts`, `evidence` and `relations` before
//! `fpf_spec`. That is correct for a legacy workspace and wrong for a
//! constellation-owned one: the protocol retired those three tables, and
//! recreating them puts a shadow index beside the real one that no later step
//! can remove (`backup_legacy_tables` refuses a destination that already
//! exists) while every reader keeps misdetecting the owner.
//!
//! This type opens the LanceDB connection and touches **only** the tables its
//! caller owns. `fpf ingest` on a migrated workspace therefore keeps working
//! without resurrecting a single artifact table.
//!
//! The query bodies live here as free functions over a `&Table`, so
//! `LanceStore` and `AuxiliaryStore` share one implementation of each
//! operation instead of two copies that can drift.

use std::path::Path;
use std::sync::Arc;

use arrow_array::{Array, Int32Array, StringArray};
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::table::Table;

use crate::db::schema;
use crate::db::store::{FpfChunk, FpfChunkSummary, collect_batches, extract_fpf_chunk};

/// A handle to the auxiliary tables a single feature owns.
///
/// Deliberately not a second artifact store: it exposes no artifact, evidence
/// or relation method at all, so a caller cannot reach a retired table through
/// it even by accident.
pub struct AuxiliaryStore {
    /// `None` only for the detached "this workspace has no auxiliary tables"
    /// case, where nothing was opened and nothing needs to be.
    _db: Option<lancedb::Connection>,
    fpf_spec: Option<Table>,
}

impl AuxiliaryStore {
    /// Open the auxiliary tables, creating the ones this caller owns.
    ///
    /// Never creates `artifacts`, `evidence` or `relations`. Safe on both
    /// layouts: on a legacy workspace those tables already exist and are left
    /// untouched; on a migrated one they stay retired.
    pub async fn open_or_create(workspace_path: &Path) -> anyhow::Result<Self> {
        let lance_dir = workspace_path.join("lance");
        tokio::fs::create_dir_all(&lance_dir).await?;
        let db = connect(&lance_dir).await?;

        let existing = db.table_names().execute().await?;
        if !existing.contains(&"fpf_spec".to_string()) {
            let batch = crate::db::store::empty_fpf_batch_for(schema::fpf_spec_schema())?;
            db.create_table("fpf_spec", vec![batch]).execute().await?;
        }
        let fpf_spec = db.open_table("fpf_spec").execute().await.ok();
        if let Some(table) = fpf_spec.as_ref() {
            crate::db::migrate::migrate_fpf_spec(table).await?;
        }
        Ok(Self {
            _db: Some(db),
            fpf_spec,
        })
    }

    /// Open the auxiliary tables without creating anything.
    ///
    /// A read path must not turn a missing table into a new one — `fpf status`
    /// on a workspace that never ingested has to report "not initialized", not
    /// quietly allocate a table as a side effect of looking.
    pub async fn open_existing(workspace_path: &Path) -> anyhow::Result<Self> {
        let lance_dir = workspace_path.join("lance");
        if !lance_dir.exists() {
            return Ok(detached());
        }
        let db = connect(&lance_dir).await?;
        let fpf_spec = db.open_table("fpf_spec").execute().await.ok();
        Ok(Self {
            _db: Some(db),
            fpf_spec,
        })
    }

    /// Whether this workspace has an ingested FPF knowledge base.
    pub fn has_fpf(&self) -> bool {
        self.fpf_spec.is_some()
    }

    fn fpf_table(&self) -> anyhow::Result<&Table> {
        self.fpf_spec
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("FPF knowledge base not initialized"))
    }

    /// Replace the knowledge base with `chunks`.
    ///
    /// `embeddings`, when present, must match `chunks` in length and every
    /// vector must be `EMBEDDING_DIM` wide and finite — enforced here so a
    /// swapped embedder cannot persist a corrupt table.
    pub async fn insert_fpf_chunks(
        &self,
        chunks: &[FpfChunk],
        embeddings: Option<&[Vec<f32>]>,
    ) -> anyhow::Result<usize> {
        fpf_insert_chunks(self.fpf_table()?, chunks, embeddings).await
    }

    /// Keyword search over the knowledge base (case-insensitive).
    pub async fn search_fpf(&self, query: &str, limit: usize) -> anyhow::Result<Vec<FpfChunk>> {
        fpf_search_keyword(self.fpf_table()?, query, limit).await
    }

    /// Cosine vector search over the knowledge base.
    pub async fn search_fpf_by_vector(
        &self,
        query_vec: &[f32],
        limit: usize,
    ) -> anyhow::Result<Vec<FpfChunk>> {
        fpf_search_vector(self.fpf_table()?, query_vec, limit).await
    }

    /// One section by its `section_id`, or `None`.
    pub async fn get_fpf_section(&self, section_id: &str) -> anyhow::Result<Option<FpfChunk>> {
        fpf_get_section(self.fpf_table()?, section_id).await
    }

    /// Section summaries without bodies.
    pub async fn list_fpf_sections(&self) -> anyhow::Result<Vec<FpfChunkSummary>> {
        fpf_list_sections(self.fpf_table()?).await
    }

    /// Drop every ingested section, keeping the table itself.
    pub async fn clear_fpf(&self) -> anyhow::Result<()> {
        fpf_clear(self.fpf_table()?).await
    }
}

async fn connect(lance_dir: &Path) -> anyhow::Result<lancedb::Connection> {
    lancedb::connect(lance_dir.to_str().ok_or_else(|| {
        anyhow::anyhow!(
            "LanceDB path contains non-UTF-8 characters: {:?}",
            lance_dir
        )
    })?)
    .execute()
    .await
    .map_err(Into::into)
}

/// A handle with no connection at all, for "this workspace has no auxiliary
/// tables" — the read result must not depend on allocating a database.
fn detached() -> AuxiliaryStore {
    AuxiliaryStore {
        _db: None,
        fpf_spec: None,
    }
}

pub(crate) async fn fpf_insert_chunks(
    table: &Table,
    chunks: &[FpfChunk],
    embeddings: Option<&[Vec<f32>]>,
) -> anyhow::Result<usize> {
    let ids: Vec<&str> = chunks.iter().map(|c| c.id.as_str()).collect();
    let section_ids: Vec<&str> = chunks.iter().map(|c| c.section_id.as_str()).collect();
    let parent_sections: Vec<Option<&str>> =
        chunks.iter().map(|c| c.parent_section.as_deref()).collect();
    let titles: Vec<&str> = chunks.iter().map(|c| c.title.as_str()).collect();
    let bodies: Vec<&str> = chunks.iter().map(|c| c.body.as_str()).collect();
    let line_counts: Vec<i32> = chunks.iter().map(|c| c.line_count).collect();
    let file_paths: Vec<&str> = chunks.iter().map(|c| c.file_path.as_str()).collect();
    let created_ats: Vec<&str> = chunks.iter().map(|c| c.created_at.as_str()).collect();

    let dim = schema::EMBEDDING_DIM as usize;
    let embedding_col: Arc<dyn arrow_array::Array> = match embeddings {
        None => crate::db::convert::make_null_embedding_col(chunks.len()),
        Some(vecs) => {
            if vecs.len() != chunks.len() {
                anyhow::bail!(
                    "embeddings length ({}) must match chunks length ({})",
                    vecs.len(),
                    chunks.len()
                );
            }
            for (i, v) in vecs.iter().enumerate() {
                if v.len() != dim {
                    anyhow::bail!(
                        "embedding[{}] has wrong dim: expected {}, got {}",
                        i,
                        dim,
                        v.len()
                    );
                }
                if let Some(pos) = v.iter().position(|f| !f.is_finite()) {
                    anyhow::bail!(
                        "embedding[{}] contains non-finite value at index {}",
                        i,
                        pos
                    );
                }
            }
            use arrow_array::{FixedSizeListArray, Float32Array};
            use arrow_schema::Field as ArrowField;
            let total: Vec<f32> = vecs.iter().flatten().copied().collect();
            let values = Arc::new(Float32Array::from(total));
            let item_field = Arc::new(ArrowField::new(
                "item",
                arrow_schema::DataType::Float32,
                true,
            ));
            Arc::new(FixedSizeListArray::new(
                item_field,
                schema::EMBEDDING_DIM,
                values,
                None,
            ))
        }
    };

    let batch = arrow_array::RecordBatch::try_new(
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
            embedding_col,
        ],
    )?;

    table.add(vec![batch]).execute().await?;
    Ok(chunks.len())
}

pub(crate) async fn fpf_search_vector(
    table: &Table,
    query_vec: &[f32],
    limit: usize,
) -> anyhow::Result<Vec<FpfChunk>> {
    let dim = schema::EMBEDDING_DIM as usize;
    if query_vec.len() != dim {
        anyhow::bail!(
            "query vector has wrong dim: expected {}, got {}",
            dim,
            query_vec.len()
        );
    }
    // On an empty/all-null embedding column LanceDB may return zero rows or
    // error. Ok([]) is the documented contract; the reason is logged.
    let builder = match table.vector_search(query_vec) {
        Ok(b) => b.distance_type(lancedb::DistanceType::Cosine).limit(limit),
        Err(e) => {
            eprintln!("  [fpf_search_vector] vector search builder failed: {e}");
            return Ok(Vec::new());
        }
    };
    let stream = match builder.execute().await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("  [fpf_search_vector] vector search failed: {e}");
            return Ok(Vec::new());
        }
    };

    let batches = collect_batches(stream).await?;
    let mut results = Vec::new();
    for batch in &batches {
        for row in 0..batch.num_rows() {
            if let Some(chunk) = extract_fpf_chunk(batch, row) {
                results.push(chunk);
            }
        }
    }
    Ok(results)
}

/// Keyword scoring shared by both store handles. Ranking is deterministic:
/// title hits outweigh body frequency, and ties fall back to the table's own
/// order, so the same corpus always produces the same list.
pub(crate) async fn fpf_search_keyword(
    table: &Table,
    query: &str,
    limit: usize,
) -> anyhow::Result<Vec<FpfChunk>> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Search query cannot be empty");
    }
    let query_lower = trimmed.to_lowercase();
    let words: Vec<String> = query_lower
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .map(|w| w.replace('\'', "''"))
        .collect();

    let filter = if words.is_empty() {
        let escaped = query_lower.replace('\'', "''");
        format!(
            "LOWER(title) LIKE '%{}%' OR LOWER(body) LIKE '%{}%'",
            escaped, escaped
        )
    } else {
        words
            .iter()
            .map(|w| {
                format!(
                    "(LOWER(title) LIKE '%{}%' OR LOWER(body) LIKE '%{}%')",
                    w, w
                )
            })
            .collect::<Vec<_>>()
            .join(" OR ")
    };

    let batches = collect_batches(table.query().only_if(filter).execute().await?).await?;

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
                let all_in_title =
                    words.len() > 1 && words.iter().all(|w| title_lower.contains(w.as_str()));
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
    scored.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    Ok(scored.into_iter().take(limit).map(|(c, _)| c).collect())
}

pub(crate) async fn fpf_get_section(
    table: &Table,
    section_id: &str,
) -> anyhow::Result<Option<FpfChunk>> {
    let filter = format!("section_id = '{}'", section_id.replace('\'', "''"));
    let batches = collect_batches(table.query().only_if(filter).limit(1).execute().await?).await?;
    for batch in &batches {
        if batch.num_rows() > 0 {
            return Ok(extract_fpf_chunk(batch, 0));
        }
    }
    Ok(None)
}

pub(crate) async fn fpf_list_sections(table: &Table) -> anyhow::Result<Vec<FpfChunkSummary>> {
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

pub(crate) async fn fpf_clear(table: &Table) -> anyhow::Result<()> {
    table.delete("id IS NOT NULL").await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::store::LanceStore;

    /// A constellation-owned workspace must stay free of retired tables even
    /// when the auxiliary feature initialises itself there.
    #[tokio::test]
    async fn auxiliary_open_never_creates_retired_artifact_tables() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        // Publish a real generation so the workspace is genuinely protocol-owned.
        let root = tmp.path().canonicalize().unwrap();
        let capability =
            crate::constellation::facade::MaintainCapability::for_strict_maintenance(root);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        crate::constellation::facade::maintain_index(
            &capability,
            crate::constellation::capture::CaptureLimits::default(),
            deadline,
        )
        .await
        .expect("publish a generation");

        let auxiliary = AuxiliaryStore::open_or_create(&ws).await.unwrap();
        assert!(
            auxiliary.has_fpf(),
            "the auxiliary table it owns must exist"
        );

        let lance = ws.join("lance");
        for retired in ["artifacts", "evidence", "relations"] {
            assert!(
                !lance.join(format!("{retired}.lance")).exists(),
                "{retired}.lance must stay retired: auxiliary access created it"
            );
        }
    }

    /// The read path must not allocate a table just because it looked.
    #[tokio::test]
    async fn auxiliary_read_does_not_create_the_table_it_looks_for() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let auxiliary = AuxiliaryStore::open_existing(&ws).await.unwrap();
        assert!(!auxiliary.has_fpf());
        assert!(!ws.join("lance/fpf_spec.lance").exists());
    }

    /// The knowledge base must survive a round trip on a migrated workspace,
    /// where no legacy store can be opened at all.
    #[tokio::test]
    async fn migrated_workspace_ingests_and_searches_without_a_legacy_store() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(ws.join("prds")).await.unwrap();
        tokio::fs::write(
            ws.join("prds/PRD-001-lagoon.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: active\ntitle: Lagoon\ndepth: standard\n---\n\n## Summary\n\nlagoonbody\n",
        )
        .await
        .unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let capability =
            crate::constellation::facade::MaintainCapability::for_strict_maintenance(root);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        crate::constellation::facade::maintain_index(
            &capability,
            crate::constellation::capture::CaptureLimits::default(),
            deadline,
        )
        .await
        .expect("publish a generation");

        let auxiliary = AuxiliaryStore::open_or_create(&ws).await.unwrap();
        auxiliary
            .insert_fpf_chunks(
                &[FpfChunk {
                    id: "fpf-A.1-001".into(),
                    section_id: "A.1".into(),
                    parent_section: None,
                    title: "Trust calculus".into(),
                    body: "Reliability of each source is scored.".into(),
                    line_count: 12,
                    file_path: "A.1.md".into(),
                    created_at: "2026-10-19T00:00:00Z".into(),
                }],
                None,
            )
            .await
            .unwrap();

        let hits = auxiliary.search_fpf("calculus", 5).await.unwrap();
        assert_eq!(hits.len(), 1, "the ingested section must be findable");
        assert_eq!(hits[0].section_id, "A.1");
        assert_eq!(
            auxiliary
                .get_fpf_section("A.1")
                .await
                .unwrap()
                .unwrap()
                .title,
            "Trust calculus"
        );
        assert_eq!(auxiliary.list_fpf_sections().await.unwrap().len(), 1);

        // The retired tables are still absent after real writes and reads.
        for retired in ["artifacts", "evidence", "relations"] {
            assert!(
                !ws.join("lance").join(format!("{retired}.lance")).exists(),
                "{retired}.lance must stay retired after an FPF round trip"
            );
        }
    }

    /// Clearing removes the rows, not the table: a later ingest must not have
    /// to recreate it.
    #[tokio::test]
    async fn clearing_removes_rows_and_keeps_the_table() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let auxiliary = AuxiliaryStore::open_or_create(&ws).await.unwrap();
        auxiliary
            .insert_fpf_chunks(
                &[FpfChunk {
                    id: "fpf-B.1".into(),
                    section_id: "B.1".into(),
                    parent_section: None,
                    title: "Abduction".into(),
                    body: "body".into(),
                    line_count: 1,
                    file_path: "B.1.md".into(),
                    created_at: "2026-10-19T00:00:00Z".into(),
                }],
                None,
            )
            .await
            .unwrap();
        auxiliary.clear_fpf().await.unwrap();
        assert!(auxiliary.list_fpf_sections().await.unwrap().is_empty());
        assert!(ws.join("lance/fpf_spec.lance").exists());
    }

    /// A dimension mismatch is refused at the boundary rather than persisted.
    #[tokio::test]
    async fn auxiliary_ingest_rejects_a_wrong_dimension_vector() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let auxiliary = AuxiliaryStore::open_or_create(&ws).await.unwrap();
        let chunks = vec![FpfChunk {
            id: "fpf-C.1".into(),
            section_id: "C.1".into(),
            parent_section: None,
            title: "Deduction".into(),
            body: "body".into(),
            line_count: 1,
            file_path: "C.1.md".into(),
            created_at: "2026-10-19T00:00:00Z".into(),
        }];
        let err = auxiliary
            .insert_fpf_chunks(&chunks, Some(&[vec![0.5f32; 8]]))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("wrong dim"));
        assert!(auxiliary.list_fpf_sections().await.unwrap().is_empty());
    }

    /// The auxiliary path must not disturb a legacy workspace: its artifact
    /// tables survive and the legacy store still reads its rows.
    #[tokio::test]
    async fn auxiliary_open_preserves_an_existing_legacy_workspace() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let legacy = LanceStore::init(&ws).await.unwrap();
        legacy
            .create_artifact(&crate::db::store::NewArtifact {
                id: "PRD-001".into(),
                kind: "prd".into(),
                status: "draft".into(),
                title: "Legacy artifact".into(),
                body: "# Body\n\nkept\n".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: vec![],
            })
            .await
            .unwrap();

        AuxiliaryStore::open_or_create(&ws).await.unwrap();

        for table in ["artifacts", "evidence", "relations"] {
            assert!(
                ws.join("lance").join(format!("{table}.lance")).exists(),
                "{table}.lance must survive auxiliary access"
            );
        }
        let reopened = LanceStore::open(&ws).await.unwrap();
        assert!(
            reopened.get_record("PRD-001").await.unwrap().is_some(),
            "the legacy row must still be readable after auxiliary access"
        );
    }

    /// The two handles must agree on results for the same corpus: a KB read
    /// through `LanceStore` and through `AuxiliaryStore` cannot diverge.
    #[tokio::test]
    async fn both_handles_agree_on_the_same_knowledge_base() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let legacy = LanceStore::init(&ws).await.unwrap();
        legacy
            .insert_fpf_chunks(
                &[FpfChunk {
                    id: "fpf-D.1".into(),
                    section_id: "D.1".into(),
                    parent_section: None,
                    title: "Induction".into(),
                    body: "Synthesise the recommendation".into(),
                    line_count: 3,
                    file_path: "D.1.md".into(),
                    created_at: "2026-10-19T00:00:00Z".into(),
                }],
                None,
            )
            .await
            .unwrap();

        let auxiliary = AuxiliaryStore::open_existing(&ws).await.unwrap();
        let via_store = legacy.search_fpf("induction", 5).await.unwrap();
        let via_auxiliary = auxiliary.search_fpf("induction", 5).await.unwrap();
        assert_eq!(
            via_store.iter().map(|c| &c.id).collect::<Vec<_>>(),
            via_auxiliary.iter().map(|c| &c.id).collect::<Vec<_>>(),
            "the shared keyword path must give both handles the same ranking"
        );
    }
}
