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
//! caller owns. FPF and change-log handles are opened independently, without
//! resurrecting artifact tables or initializing another auxiliary feature.
//!
//! The query bodies live here as free functions over a `&Table`, so
//! `LanceStore` and `AuxiliaryStore` share one implementation of each
//! operation instead of two copies that can drift.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::{Int32Array, RecordBatch, StringArray};
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::table::Table;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::changelog::ChangeLogEntry;
use crate::db::schema;
use crate::db::store::{
    FpfChunk, FpfChunkSummary, collect_batches, empty_change_log_batch, extract_change_log_entry,
    extract_fpf_chunk, extract_fpf_chunk_summary,
};

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
    change_log: Option<Table>,
}

impl AuxiliaryStore {
    /// Open the FPF knowledge base, creating it if missing.
    ///
    /// Never creates artifact, evidence, relation or change-log tables. The
    /// caller must already hold the workspace lock.
    pub async fn open_or_create(workspace_path: &Path) -> anyhow::Result<Self> {
        let lance_dir = workspace_path.join("lance");
        tokio::fs::create_dir_all(&lance_dir).await?;
        let db = connect(&lance_dir).await?;

        let fpf_spec = match open_optional_table(&db, &lance_dir, "fpf_spec").await? {
            Some(table) => table,
            None => {
                let batch = crate::db::store::empty_fpf_batch_for(schema::fpf_spec_schema())?;
                db.create_table("fpf_spec", vec![batch]).execute().await?
            }
        };
        validate_fpf_schema(&fpf_spec).await?;
        crate::db::migrate::migrate_fpf_spec(&fpf_spec).await?;
        Ok(Self {
            _db: Some(db),
            fpf_spec: Some(fpf_spec),
            change_log: None,
        })
    }

    /// Open only the FPF knowledge base without creating anything.
    ///
    /// A read path must not turn a missing table into a new one — `fpf status`
    /// on a workspace that never ingested has to report "not initialized", not
    /// quietly allocate a table as a side effect of looking.
    pub async fn open_existing(workspace_path: &Path) -> anyhow::Result<Self> {
        let lance_dir = workspace_path.join("lance");
        if !path_present(&lance_dir).await? {
            return Ok(detached());
        }
        let db = connect(&lance_dir).await?;
        let fpf_spec = open_optional_table(&db, &lance_dir, "fpf_spec").await?;
        if let Some(table) = fpf_spec.as_ref() {
            validate_fpf_schema(table).await?;
        }
        Ok(Self {
            _db: Some(db),
            fpf_spec,
            change_log: None,
        })
    }

    /// Open only existing history, without creating or migrating any table.
    ///
    /// A confirmed missing directory or table yields an empty history handle.
    /// Existing but unreadable or incompatible history is an error.
    pub async fn open_history_existing(workspace_path: &Path) -> anyhow::Result<Self> {
        let lance_dir = workspace_path.join("lance");
        if !path_present(&lance_dir).await? {
            return Ok(detached());
        }
        let db = connect(&lance_dir).await?;
        let change_log = open_optional_table(&db, &lance_dir, "change_log").await?;
        if let Some(table) = change_log.as_ref() {
            validate_change_log_schema(table).await?;
        }
        Ok(Self {
            _db: Some(db),
            fpf_spec: None,
            change_log,
        })
    }

    /// Open history, creating only a missing `change_log` table.
    ///
    /// The caller already holds the workspace lock. Existing rows and schema
    /// are never migrated, replaced or cleared by this constructor.
    pub async fn open_history_or_create(workspace_path: &Path) -> anyhow::Result<Self> {
        let lance_dir = workspace_path.join("lance");
        tokio::fs::create_dir_all(&lance_dir).await?;
        let db = connect(&lance_dir).await?;
        let change_log = match open_optional_table(&db, &lance_dir, "change_log").await? {
            Some(table) => {
                validate_change_log_schema(&table).await?;
                table
            }
            None => {
                let batch = empty_change_log_batch(schema::change_log_schema())?;
                db.create_table("change_log", vec![batch]).execute().await?
            }
        };
        Ok(Self {
            _db: Some(db),
            fpf_spec: None,
            change_log: Some(change_log),
        })
    }

    /// Append one entry without acquiring a workspace lock.
    ///
    /// The caller holds the lock. A read handle for absent history cannot
    /// append; use `open_history_or_create` for a write.
    pub async fn log_change(&self, entry: &ChangeLogEntry) -> anyhow::Result<()> {
        let table = self
            .change_log
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("change_log table not initialized"))?;
        change_log_append(table, entry).await
    }

    /// Query history, including deleted artifacts, newest timestamps first.
    ///
    /// Both filters are literal, AND-combined strings. The limit is applied
    /// after sorting, exactly as on the legacy store. Absent history is empty.
    pub async fn get_change_log(
        &self,
        artifact_id: Option<&str>,
        source: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<ChangeLogEntry>> {
        match self.change_log.as_ref() {
            Some(table) => change_log_query(table, artifact_id, source, limit).await,
            None => Ok(Vec::new()),
        }
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

/// Filesystem errors are not evidence of absence. Inspect the link itself so
/// a dangling dataset symlink is opened (and fails), not silently recreated.
async fn path_present(path: &Path) -> anyhow::Result<bool> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// The directory inside a Lance table that holds one manifest per version.
const LANCE_VERSIONS_DIR: &str = "_versions";

/// The extension every Lance manifest file name ends with
/// (`lance-table-4.0.0/src/io/commit.rs:70`).
const LANCE_MANIFEST_EXTENSION: &str = "manifest";

/// The prefix that marks a detached manifest file
/// (`lance-table-4.0.0/src/io/commit.rs:71`).
const LANCE_DETACHED_PREFIX: &str = "d";

/// The bytes every Lance manifest ends with: the file offset of the manifest
/// message, the format version and the magic.
const LANCE_MANIFEST_FOOTER_LEN: u64 = 16;

/// The magic every Lance file ends with (`lance-file-4.0.0/src/format.rs`).
const LANCE_FILE_MAGIC: &[u8; 4] = b"LANC";

/// Refuse a table whose newest manifest does not describe itself.
///
/// LanceDB opens a table by reading the manifest of its newest version, and
/// `lance` derives the manifest length by subtracting the metadata offset
/// recorded in that manifest's footer from the file size
/// (`lance-4.0.0/src/dataset.rs:612`). Manifest bytes that were damaged —
/// truncated, or overwritten with unrelated text — make that subtraction
/// underflow and abort the process with a panic instead of returning an error
/// the caller can report. The recorded length drives a second unchecked
/// subtraction further down, so both are verified here.
///
/// A manifest is a valid Lance manifest when it ends with the Lance magic and
/// the message it announces — the length prefix at the recorded offset plus
/// the message bytes behind it — fills the file exactly. Only the manifest
/// Lance itself reads is checked, and only its 16-byte footer and 4-byte
/// length prefix are read; the manifest body is never loaded.
///
/// Every open of a Lance table has to pass through this guard, including the
/// legacy store, which opens the same tables itself.
pub(crate) async fn verify_table_manifests(table_dir: &Path, table: &str) -> anyhow::Result<()> {
    let versions_dir = table_dir.join(LANCE_VERSIONS_DIR);
    let Some(manifest) = current_manifest(&versions_dir, table).await? else {
        // The table has no manifest to read: LanceDB reports the missing
        // dataset itself.
        return Ok(());
    };
    verify_manifest_file(&manifest, table).await
}

/// The manifest Lance reads when it opens the table, or `None` when there is
/// none to read.
///
/// Lance resolves the newest version in the table's version directory. Its
/// local resolver compares naming schemes before it parses versions and
/// refuses a mixed directory (`lance-table-4.0.0/src/io/commit.rs:371`), but
/// that refusal is discarded and the object-store listing takes over
/// (`commit.rs:268-272`). The listing is the resolution that actually
/// applies: it keeps only the names whose version parses, compares the
/// schemes of those candidates, refuses a genuine mix, and picks the
/// greatest version (`commit.rs:274-355`).
///
/// This mirrors that listing: detached and unparseable names never decide
/// anything, a genuine mix of naming schemes is refused, and the greatest
/// version wins.
async fn current_manifest(versions_dir: &Path, table: &str) -> anyhow::Result<Option<PathBuf>> {
    let mut entries = match tokio::fs::read_dir(versions_dir).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            anyhow::bail!(
                "Corrupt Lance table {table}: {} cannot be read: {error}",
                versions_dir.display()
            );
        }
    };
    let mut latest: Option<(u64, PathBuf)> = None;
    let mut scheme: Option<ManifestScheme> = None;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(entry_scheme) = detect_scheme(name) else {
            continue;
        };
        // Only a name whose version parses is a candidate. Lance drops
        // detached and unparseable names before it compares schemes
        // (`lance-table-4.0.0/src/io/commit.rs:276-287`), so a sidecar such
        // as `notes.manifest` must not decide anything here.
        let Some(version) = parse_version(name, entry_scheme) else {
            continue;
        };
        match scheme {
            Some(seen) if seen != entry_scheme => {
                // Genuinely mixed naming schemes. Lance refuses such a
                // directory (`commit.rs:341-346`), and the legacy store opens
                // its optional tables with `.ok()`, which would turn that
                // refusal into a missing table. Refuse it here instead.
                anyhow::bail!(
                    "Incompatible Lance table {table}: {} mixes {seen:?} and {entry_scheme:?} manifest naming schemes",
                    versions_dir.display()
                );
            }
            Some(_) => {}
            None => scheme = Some(entry_scheme),
        }
        let newer = match &latest {
            Some((latest_version, _)) => version > *latest_version,
            None => true,
        };
        if newer {
            latest = Some((version, entry.path()));
        }
    }
    Ok(latest.map(|(_, path)| path))
}

/// The naming scheme of a manifest file name
/// (`lance-table-4.0.0/src/io/commit.rs:69`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ManifestScheme {
    /// The version number is the name itself (`1.manifest`).
    V1,
    /// The version number is inverted and padded to 20 digits
    /// (`18446744073709551612.manifest`).
    V2,
}

/// The naming scheme of a manifest name: V2 names carry the inverted version
/// number padded to 20 digits (`lance-table-4.0.0/src/io/commit.rs:131`).
fn detect_scheme(name: &str) -> Option<ManifestScheme> {
    // A detached version always implies V2.
    if name.starts_with(LANCE_DETACHED_PREFIX) {
        return Some(ManifestScheme::V2);
    }
    if !name.ends_with(LANCE_MANIFEST_EXTENSION) {
        return None;
    }
    const V2_LEN: usize = 20 + 1 + LANCE_MANIFEST_EXTENSION.len();
    Some(if name.len() == V2_LEN {
        ManifestScheme::V2
    } else {
        ManifestScheme::V1
    })
}

/// The version a manifest name carries (`lance-table-4.0.0/src/io/commit.rs:107`).
fn parse_version(name: &str, scheme: ManifestScheme) -> Option<u64> {
    // A detached name does not parse here, which is how Lance skips it.
    let number = name.split_once('.')?.0.parse::<u64>().ok()?;
    Some(match scheme {
        ManifestScheme::V1 => number,
        ManifestScheme::V2 => u64::MAX - number,
    })
}

/// Verify the manifest Lance reads against the layout it reads it with.
async fn verify_manifest_file(path: &Path, table: &str) -> anyhow::Result<()> {
    let mut file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        // The version was cleaned up while the table was being read. A
        // manifest that is gone is not a damaged manifest.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let size = file.metadata().await?.len();
    if size < LANCE_MANIFEST_FOOTER_LEN {
        anyhow::bail!(
            "Corrupt Lance manifest for table {table}: {} is {size} bytes and cannot hold a Lance footer",
            path.display()
        );
    }
    let mut footer = [0u8; 16];
    file.seek(std::io::SeekFrom::Start(size - LANCE_MANIFEST_FOOTER_LEN))
        .await?;
    file.read_exact(&mut footer).await?;
    if footer[12..16] != LANCE_FILE_MAGIC[..] {
        anyhow::bail!(
            "Corrupt Lance manifest for table {table}: {} does not end with the Lance magic",
            path.display()
        );
    }
    let mut offset_bytes = [0u8; 8];
    offset_bytes.copy_from_slice(&footer[0..8]);
    let offset = u64::from_le_bytes(offset_bytes);
    // The offset has to leave room for the length prefix it points at,
    // otherwise `lance` subtracts a length larger than the file. The offset
    // comes from damaged bytes, so it is compared without adding to it.
    if offset > size - 4 {
        anyhow::bail!(
            "Corrupt Lance manifest for table {table}: {} records the metadata offset {offset} outside the {size}-byte file",
            path.display()
        );
    }
    let mut prefix = [0u8; 4];
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    file.read_exact(&mut prefix).await?;
    let message_len = u32::from_le_bytes(prefix) as u64;
    if offset + 4 + message_len + LANCE_MANIFEST_FOOTER_LEN != size {
        anyhow::bail!(
            "Corrupt Lance manifest for table {table}: {} records a {message_len}-byte message that does not fill the {size}-byte file",
            path.display()
        );
    }
    Ok(())
}

async fn open_optional_table(
    db: &lancedb::Connection,
    lance_dir: &Path,
    name: &str,
) -> anyhow::Result<Option<Table>> {
    let tables = db.table_names().execute().await?;
    let table_dir = lance_dir.join(format!("{name}.lance"));
    if !tables.iter().any(|table| table == name) && !path_present(&table_dir).await? {
        return Ok(None);
    }
    // The table exists, so its manifests have to be readable before LanceDB
    // reads one of them: a damaged manifest aborts the process inside
    // `lance` instead of failing the open.
    verify_table_manifests(&table_dir, name).await?;
    Ok(Some(db.open_table(name).execute().await?))
}

fn validate_read_column(
    actual: &arrow_schema::Schema,
    expected: &arrow_schema::Field,
    table_name: &str,
) -> anyhow::Result<()> {
    let field = actual
        .field_with_name(expected.name())
        .map_err(|error| anyhow::anyhow!("Incompatible {table_name} schema: {error}"))?;
    if field.data_type() != expected.data_type() {
        anyhow::bail!(
            "Incompatible {table_name} schema: {} must be {:?}, found {:?}",
            expected.name(),
            expected.data_type(),
            field.data_type()
        );
    }
    if !expected.is_nullable() && field.is_nullable() {
        anyhow::bail!(
            "Incompatible {table_name} schema: required column '{}' must be non-nullable",
            expected.name()
        );
    }
    Ok(())
}

async fn validate_change_log_schema(table: &Table) -> anyhow::Result<()> {
    let actual = table.schema().await?;
    for field in schema::change_log_schema().fields() {
        validate_read_column(&actual, field, "change_log")?;
    }
    Ok(())
}

async fn validate_fpf_schema(table: &Table) -> anyhow::Result<Arc<arrow_schema::Schema>> {
    let actual = table.schema().await?;
    for field in schema::fpf_spec_schema().fields() {
        // The real pre-semantic layout has these eight text/metadata columns
        // and no embedding column. Text reads do not constrain vector layout.
        if field.name() != "embedding" {
            validate_read_column(&actual, field, "fpf_spec")?;
        }
    }
    Ok(actual)
}

/// A handle with no connection at all, for "this workspace has no auxiliary
/// tables" — the read result must not depend on allocating a database.
fn detached() -> AuxiliaryStore {
    AuxiliaryStore {
        _db: None,
        fpf_spec: None,
        change_log: None,
    }
}

pub(crate) async fn change_log_append(table: &Table, entry: &ChangeLogEntry) -> anyhow::Result<()> {
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

pub(crate) async fn change_log_query(
    table: &Table,
    artifact_id: Option<&str>,
    source: Option<&str>,
    limit: usize,
) -> anyhow::Result<Vec<ChangeLogEntry>> {
    validate_change_log_schema(table).await?;
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
            results.push(extract_change_log_entry(batch, row)?);
        }
    }

    // Sort by timestamp descending (newest first).
    results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    results.truncate(limit);
    Ok(results)
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
    let read_schema = validate_fpf_schema(table).await?;
    // Missing/pre-semantic and confirmed all-null vectors retain their empty
    // result. Other query failures are errors, not an empty healthy KB.
    if read_schema.field_with_name("embedding").is_err() || limit == 0 {
        return Ok(Vec::new());
    }
    let embedded_rows = table
        .count_rows(Some("embedding IS NOT NULL".to_string()))
        .await?;
    let limit = limit.min(embedded_rows);
    if limit == 0 {
        return Ok(Vec::new());
    }
    let builder = table
        .vector_search(query_vec)?
        .distance_type(lancedb::DistanceType::Cosine)
        .limit(limit);
    let stream = builder.execute().await?;

    let batches = collect_batches(stream).await?;
    let mut results = Vec::new();
    for batch in &batches {
        for row in 0..batch.num_rows() {
            results.push(extract_fpf_chunk(batch, row)?);
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
    validate_fpf_schema(table).await?;
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
            let chunk = extract_fpf_chunk(batch, i)?;
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
    scored.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    Ok(scored.into_iter().take(limit).map(|(c, _)| c).collect())
}

pub(crate) async fn fpf_get_section(
    table: &Table,
    section_id: &str,
) -> anyhow::Result<Option<FpfChunk>> {
    validate_fpf_schema(table).await?;
    let filter = format!("section_id = '{}'", section_id.replace('\'', "''"));
    let batches = collect_batches(table.query().only_if(filter).limit(1).execute().await?).await?;
    for batch in &batches {
        if batch.num_rows() > 0 {
            return extract_fpf_chunk(batch, 0).map(Some);
        }
    }
    Ok(None)
}

pub(crate) async fn fpf_list_sections(table: &Table) -> anyhow::Result<Vec<FpfChunkSummary>> {
    validate_fpf_schema(table).await?;
    let batches = collect_batches(table.query().execute().await?).await?;
    let mut results = Vec::new();
    for batch in &batches {
        for row in 0..batch.num_rows() {
            results.push(extract_fpf_chunk_summary(batch, row)?);
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

    #[test]
    fn history_row_decoder_rejects_required_nulls_but_preserves_optional_nulls() {
        use arrow_schema::{Field, Schema};

        for null_column in [
            None,
            Some("timestamp"),
            Some("artifact_id"),
            Some("action"),
            Some("source"),
        ] {
            let fields = schema::change_log_schema()
                .fields()
                .iter()
                .map(|field| {
                    Field::new(
                        field.name().as_str(),
                        field.data_type().clone(),
                        field.is_nullable() || null_column == Some(field.name().as_str()),
                    )
                })
                .collect::<Vec<_>>();
            let columns = fields
                .iter()
                .map(|field| -> arrow_array::ArrayRef {
                    let value = if field.is_nullable() {
                        None
                    } else {
                        Some(match field.name().as_str() {
                            "timestamp" => "2026-10-24T00:00:00Z",
                            "artifact_id" => "PRD-001",
                            "action" => "update",
                            "source" => "cli",
                            _ => unreachable!(
                                "only the four required history fields are non-nullable"
                            ),
                        })
                    };
                    Arc::new(StringArray::from(vec![value]))
                })
                .collect();
            let batch = RecordBatch::try_new(Arc::new(Schema::new(fields)), columns).unwrap();
            match null_column {
                Some(column) => {
                    let message = extract_change_log_entry(&batch, 0).unwrap_err().to_string();
                    assert!(message.contains("Invalid change_log row 0"), "{message}");
                    assert!(message.contains(&format!("'{column}'")), "{message}");
                }
                None => {
                    let row = extract_change_log_entry(&batch, 0).unwrap();
                    assert_eq!(row.artifact_id, "PRD-001");
                    assert_eq!(row.field, None);
                    assert_eq!(row.old_value, None);
                    assert_eq!(row.new_value, None);
                    assert_eq!(row.commit_hash, None);
                }
            }
        }
    }

    fn fpf_read_batch(
        null_required: Option<&str>,
        embedding_dimension: Option<i32>,
        empty: bool,
    ) -> RecordBatch {
        use arrow_schema::{DataType, Field, Schema};

        let mut fields = schema::fpf_spec_schema()
            .fields()
            .iter()
            .filter(|field| field.name() != "embedding")
            .map(|field| {
                Field::new(
                    field.name().as_str(),
                    field.data_type().clone(),
                    field.is_nullable() || null_required == Some(field.name().as_str()),
                )
            })
            .collect::<Vec<_>>();
        if let Some(dimension) = embedding_dimension {
            fields.push(Field::new(
                "embedding",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dimension,
                ),
                true,
            ));
        }
        let columns = fields
            .iter()
            .map(|field| -> arrow_array::ArrayRef {
                if empty {
                    return arrow_array::new_empty_array(field.data_type());
                }
                if null_required == Some(field.name().as_str())
                    || matches!(field.name().as_str(), "parent_section" | "embedding")
                {
                    return arrow_array::new_null_array(field.data_type(), 1);
                }
                match field.name().as_str() {
                    "body" => Arc::new(arrow_array::LargeStringArray::from(vec![
                        "Reliability belongs in the trust calculus.",
                    ])),
                    "line_count" => Arc::new(Int32Array::from(vec![12])),
                    column => {
                        let value = match column {
                            "id" => "fpf-B.3-001",
                            "section_id" => "B.3",
                            "title" => "Trust calculus",
                            "file_path" => "B.3.md",
                            "created_at" => "2026-10-24T00:00:00Z",
                            _ => unreachable!("the fixture declares only the FPF read columns"),
                        };
                        Arc::new(StringArray::from(vec![value]))
                    }
                }
            })
            .collect();
        RecordBatch::try_new(Arc::new(Schema::new(fields)), columns).unwrap()
    }

    async fn seed_fpf_read_table(ws: &Path, batch: RecordBatch) -> Table {
        let lance = ws.join("lance");
        std::fs::create_dir_all(&lance).unwrap();
        connect(&lance)
            .await
            .unwrap()
            .create_table("fpf_spec", vec![batch])
            .execute()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn wrong_schema_fpf_is_not_an_empty_kb_for_either_owner() {
        use arrow_schema::{DataType, Field, Schema};

        for empty in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let ws = temp.path().join(".forgeplan");
            let lock = crate::workspace::lock::acquire_workspace_lock(&ws)
                .await
                .unwrap();
            let wrong_schema = Arc::new(Schema::new(vec![
                Field::new("title", DataType::Utf8, false),
                Field::new("body", DataType::LargeUtf8, false),
            ]));
            let batch = if empty {
                RecordBatch::new_empty(wrong_schema.clone())
            } else {
                RecordBatch::try_new(
                    wrong_schema.clone(),
                    vec![
                        Arc::new(StringArray::from(vec!["Trust calculus"])),
                        Arc::new(arrow_array::LargeStringArray::from(vec!["calculus body"])),
                    ],
                )
                .unwrap()
            };
            let table = seed_fpf_read_table(&ws, batch).await;
            for result in [
                AuxiliaryStore::open_existing(&ws).await,
                AuxiliaryStore::open_or_create(&ws).await,
            ] {
                let error = result
                    .err()
                    .expect("an existing incompatible KB is an error");
                assert!(error.to_string().contains("Incompatible fpf_spec schema"));
                assert!(error.to_string().contains("id"));
            }
            assert_eq!(
                table.schema().await.unwrap().fields(),
                wrong_schema.fields()
            );
            for name in ["artifacts", "evidence", "relations", "change_log"] {
                assert!(!ws.join("lance").join(format!("{name}.lance")).exists());
            }

            // init intentionally seeds a legacy fixture, not the auxiliary
            // access under test. Its shared query paths must reject it too.
            let legacy = LanceStore::init(&ws).await.unwrap();
            assert!(
                legacy
                    .list_fpf_sections()
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("fpf_spec")
            );
            assert!(
                legacy
                    .search_fpf("calculus", 5)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("fpf_spec")
            );
            assert!(
                legacy
                    .get_fpf_section("B.3")
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("fpf_spec")
            );
            assert!(
                legacy
                    .search_fpf_by_vector(&vec![0.25; schema::EMBEDDING_DIM as usize], 5)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("fpf_spec")
            );
            drop(legacy);
            drop(table);
            drop(lock);
        }
    }

    #[tokio::test]
    async fn required_null_fpf_rows_error_in_shared_decoders_and_both_read_paths() {
        for column in [
            "id",
            "section_id",
            "title",
            "body",
            "line_count",
            "file_path",
            "created_at",
        ] {
            let batch = fpf_read_batch(Some(column), None, false);
            for error in [
                extract_fpf_chunk(&batch, 0).err().unwrap(),
                extract_fpf_chunk_summary(&batch, 0).err().unwrap(),
            ] {
                let message = error.to_string();
                assert!(message.contains("Invalid fpf_spec row 0"), "{message}");
                assert!(message.contains(&format!("'{column}'")), "{message}");
            }
            let temp = tempfile::tempdir().unwrap();
            let ws = temp.path().join(".forgeplan");
            let lock = crate::workspace::lock::acquire_workspace_lock(&ws)
                .await
                .unwrap();
            let table = seed_fpf_read_table(&ws, batch).await;
            for result in [
                AuxiliaryStore::open_existing(&ws).await,
                AuxiliaryStore::open_or_create(&ws).await,
            ] {
                let message = result.err().unwrap().to_string();
                assert!(
                    message.contains("Incompatible fpf_spec schema"),
                    "{message}"
                );
                assert!(message.contains(column), "{message}");
                assert!(message.contains("non-nullable"), "{message}");
            }
            let legacy = LanceStore::init(&ws).await.unwrap();
            for message in [
                legacy.list_fpf_sections().await.unwrap_err().to_string(),
                legacy
                    .search_fpf("calculus", 5)
                    .await
                    .unwrap_err()
                    .to_string(),
                legacy.get_fpf_section("B.3").await.unwrap_err().to_string(),
            ] {
                assert!(message.contains("fpf_spec"), "{message}");
                assert!(message.contains(column), "{message}");
            }
            drop(legacy);
            drop(table);
            drop(lock);
        }
    }

    #[tokio::test]
    async fn empty_fpf_schema_cannot_make_a_required_column_nullable() {
        for column in [
            "id",
            "section_id",
            "title",
            "body",
            "line_count",
            "file_path",
            "created_at",
        ] {
            let temp = tempfile::tempdir().unwrap();
            let ws = temp.path().join(".forgeplan");
            let lock = crate::workspace::lock::acquire_workspace_lock(&ws)
                .await
                .unwrap();
            let table = seed_fpf_read_table(&ws, fpf_read_batch(Some(column), None, true)).await;
            let message = AuxiliaryStore::open_existing(&ws)
                .await
                .err()
                .unwrap()
                .to_string();
            assert!(
                message.contains("Incompatible fpf_spec schema"),
                "{message}"
            );
            assert!(message.contains(column), "{message}");
            drop(table);
            drop(lock);
        }
    }

    #[tokio::test]
    async fn real_pre_embedding_layout_and_existing_vector_dimensions_remain_readable() {
        for dimension in [None, Some(8)] {
            let temp = tempfile::tempdir().unwrap();
            let ws = temp.path().join(".forgeplan");
            let lock = crate::workspace::lock::acquire_workspace_lock(&ws)
                .await
                .unwrap();
            let expected = fpf_read_batch(None, dimension, false);
            let expected_schema = expected.schema();
            let table = seed_fpf_read_table(&ws, expected).await;
            let auxiliary = AuxiliaryStore::open_existing(&ws).await.unwrap();
            assert!(auxiliary.has_fpf());
            let chunks = auxiliary.search_fpf("calculus", 5).await.unwrap();
            assert_eq!(chunks.len(), 1);
            assert_eq!(chunks[0].id, "fpf-B.3-001");
            assert_eq!(chunks[0].parent_section, None);
            assert_eq!(chunks[0].body, "Reliability belongs in the trust calculus.");
            let summary = auxiliary.list_fpf_sections().await.unwrap();
            assert_eq!(summary.len(), 1);
            assert_eq!(summary[0].section_id, "B.3");
            assert_eq!(summary[0].title, "Trust calculus");
            assert_eq!(summary[0].line_count, 12);
            assert_eq!(
                auxiliary
                    .get_fpf_section("B.3")
                    .await
                    .unwrap()
                    .unwrap()
                    .file_path,
                "B.3.md"
            );
            assert!(
                auxiliary
                    .search_fpf_by_vector(&vec![0.25; schema::EMBEDDING_DIM as usize], 5)
                    .await
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(
                table.schema().await.unwrap().fields(),
                expected_schema.fields()
            );
            drop(auxiliary);

            let legacy = LanceStore::init(&ws).await.unwrap();
            assert_eq!(legacy.search_fpf("calculus", 5).await.unwrap().len(), 1);
            assert_eq!(legacy.list_fpf_sections().await.unwrap().len(), 1);
            assert!(legacy.get_fpf_section("B.3").await.unwrap().is_some());
            drop(legacy);

            let writable = AuxiliaryStore::open_or_create(&ws).await.unwrap();
            assert_eq!(writable.search_fpf("calculus", 5).await.unwrap().len(), 1);
            let reopened = connect(&ws.join("lance"))
                .await
                .unwrap()
                .open_table("fpf_spec")
                .execute()
                .await
                .unwrap();
            match dimension {
                None => assert!(
                    reopened
                        .schema()
                        .await
                        .unwrap()
                        .field_with_name("embedding")
                        .is_ok()
                ),
                Some(_) => assert_eq!(
                    reopened.schema().await.unwrap().fields(),
                    expected_schema.fields()
                ),
            }
            drop(reopened);
            drop(writable);
            drop(table);
            drop(lock);
        }
    }

    #[tokio::test]
    async fn existing_corrupt_fpf_is_an_error_for_both_constructors() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws)
            .await
            .unwrap();
        std::fs::create_dir_all(ws.join("lance/fpf_spec.lance")).unwrap();
        let sentinel = ws.join("lance/fpf_spec.lance/sentinel");
        std::fs::write(&sentinel, b"not a Lance dataset").unwrap();

        assert!(AuxiliaryStore::open_existing(&ws).await.is_err());
        assert!(AuxiliaryStore::open_or_create(&ws).await.is_err());
        assert_eq!(std::fs::read(sentinel).unwrap(), b"not a Lance dataset");
        for name in ["artifacts", "evidence", "relations", "change_log"] {
            assert!(!ws.join("lance").join(format!("{name}.lance")).exists());
        }
        drop(lock);
    }

    #[tokio::test]
    async fn fpf_constructors_do_not_open_or_create_history() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws)
            .await
            .unwrap();
        let fpf = AuxiliaryStore::open_or_create(&ws).await.unwrap();
        assert!(!ws.join("lance/change_log.lance").exists());
        assert!(
            fpf.log_change(&ChangeLogEntry::new("PRD-001", "create", "cli"))
                .await
                .is_err(),
            "an FPF-only handle must not silently create history"
        );
        drop(fpf);

        let broken_history = ws.join("lance/change_log.lance");
        std::fs::write(&broken_history, b"unrelated broken history").unwrap();
        let fpf = AuxiliaryStore::open_existing(&ws).await.unwrap();
        assert!(fpf.has_fpf());
        assert!(fpf.list_fpf_sections().await.unwrap().is_empty());
        drop(fpf);
        let fpf = AuxiliaryStore::open_or_create(&ws).await.unwrap();
        assert!(fpf.has_fpf());
        assert_eq!(
            std::fs::read(broken_history).unwrap(),
            b"unrelated broken history"
        );
        drop(fpf);
        drop(lock);
    }

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

    /// The manifest files a Lance table keeps for its versions.
    fn manifest_files(workspace: &std::path::Path) -> Vec<std::path::PathBuf> {
        let versions = workspace.join("lance/fpf_spec.lance/_versions");
        let mut manifests: Vec<std::path::PathBuf> = std::fs::read_dir(&versions)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().and_then(|extension| extension.to_str()) == Some("manifest")
            })
            .collect();
        manifests.sort();
        manifests
    }

    /// A Lance manifest envelope with a chosen offset and announced length.
    ///
    /// The layout is the one Lance reads: the body, the u32 length of the
    /// message, the message, and the footer that records the offset of that
    /// length prefix followed by the format version and the Lance magic.
    /// `offset` and `message_len` are the two values the guard has to read
    /// back.
    fn envelope(offset: u64, message_len: u32) -> Vec<u8> {
        let mut bytes = vec![0u8; 8];
        bytes.extend_from_slice(&message_len.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 16]);
        bytes.extend_from_slice(&offset.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(b"LANC");
        bytes
    }

    /// A table directory holding the given manifest files.
    fn table_with(root: &std::path::Path, files: &[(&str, Vec<u8>)]) -> PathBuf {
        let versions = root.join("fpf_spec.lance/_versions");
        std::fs::create_dir_all(&versions).unwrap();
        for (name, bytes) in files {
            std::fs::write(versions.join(name), bytes).unwrap();
        }
        root.join("fpf_spec.lance")
    }

    /// The guard must judge the envelope Lance reads, not only its magic.
    ///
    /// The offset and the announced length are the two unchecked values in
    /// `lance`'s reader (`lance-4.0.0/src/dataset.rs:608-617`), so each of
    /// them has to be refused on its own.
    #[tokio::test]
    async fn the_guard_judges_the_offset_and_the_announced_length() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        // A valid envelope is accepted under both naming schemes.
        for name in ["1.manifest", "18446744073709551615.manifest"] {
            let table = table_with(&root.join(name), &[(name, envelope(8, 16))]);
            verify_table_manifests(&table, "fpf_spec")
                .await
                .unwrap_or_else(|error| panic!("{name} must be accepted: {error}"));
        }

        // An offset outside the file is refused, including the maximal one.
        for offset in [44u64, u64::MAX, u64::MAX - 1] {
            let table = table_with(
                &root.join(format!("offset-{offset}")),
                &[("1.manifest", envelope(offset, 16))],
            );
            let error = verify_table_manifests(&table, "fpf_spec")
                .await
                .err()
                .expect("an offset outside the file must be refused");
            assert!(
                error.to_string().contains("metadata offset"),
                "{offset}: {error}"
            );
        }

        // An announced length that does not fill the file is refused.
        for message_len in [0u32, 15, 17, 4096] {
            let table = table_with(
                &root.join(format!("length-{message_len}")),
                &[("1.manifest", envelope(8, message_len))],
            );
            let error = verify_table_manifests(&table, "fpf_spec")
                .await
                .err()
                .expect("an inconsistent message length must be refused");
            assert!(
                error.to_string().contains("message"),
                "{message_len}: {error}"
            );
        }
    }

    /// The guard must decide on the manifest Lance actually reads.
    ///
    /// Lance keeps only the names whose version parses and compares the
    /// schemes of those candidates (`lance-table-4.0.0/src/io/commit.rs:274-355`).
    /// An unparseable or detached sidecar must not hide a damaged current
    /// manifest, and a genuine mix of naming schemes must be refused instead
    /// of passing unnoticed behind an optional-table open.
    #[tokio::test]
    async fn the_guard_follows_the_candidates_lance_actually_reads() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let damaged = b"deliberately invalid owned table version".to_vec();

        // An unparseable sidecar does not hide the damaged current manifest.
        for sidecar in ["notes.manifest", "d5.manifest"] {
            let table = table_with(
                &root.join(sidecar),
                &[
                    ("18446744073709551612.manifest", damaged.clone()),
                    (sidecar, damaged.clone()),
                ],
            );
            let error = verify_table_manifests(&table, "fpf_spec")
                .await
                .err()
                .expect("the current manifest must still be checked");
            assert!(
                error.to_string().contains("Corrupt Lance manifest"),
                "{sidecar}: {error}"
            );
        }

        // A detached manifest beside an intact current one is ignored.
        let table = table_with(
            &root.join("detached"),
            &[
                ("18446744073709551612.manifest", envelope(8, 16)),
                ("d5.manifest", damaged.clone()),
            ],
        );
        verify_table_manifests(&table, "fpf_spec")
            .await
            .unwrap_or_else(|error| panic!("a detached manifest is not a candidate: {error}"));

        // Genuinely mixed naming schemes are refused, not ignored.
        let table = table_with(
            &root.join("mixed"),
            &[
                ("1.manifest", envelope(8, 16)),
                ("18446744073709551612.manifest", envelope(8, 16)),
            ],
        );
        let error = verify_table_manifests(&table, "fpf_spec")
            .await
            .err()
            .expect("mixed naming schemes must be refused");
        assert!(error.to_string().contains("naming schemes"), "{error}");
    }

    /// A damaged manifest must be refused, not handed to `lance`.
    ///
    /// `lance` derives the manifest length by subtracting the metadata offset
    /// recorded in the manifest footer from the file size
    /// (`lance-4.0.0/src/dataset.rs:612`). Manifest bytes that were
    /// overwritten make that subtraction underflow, which aborts the whole
    /// process instead of failing the open.
    #[tokio::test]
    async fn a_damaged_manifest_is_refused_instead_of_aborting_the_process() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let auxiliary = AuxiliaryStore::open_or_create(&ws).await.unwrap();
        auxiliary
            .insert_fpf_chunks(
                &[FpfChunk {
                    id: "fpf-B.3".into(),
                    section_id: "B.3".into(),
                    parent_section: None,
                    title: "Trust calculus".into(),
                    body: "Trust requires explicit evidence.".into(),
                    line_count: 12,
                    file_path: "B.3.md".into(),
                    created_at: "2026-10-19T00:00:00Z".into(),
                }],
                None,
            )
            .await
            .unwrap();
        drop(auxiliary);

        // The intact knowledge base is readable and searchable.
        let opened = AuxiliaryStore::open_existing(&ws).await.unwrap();
        assert!(opened.has_fpf());
        assert_eq!(opened.search_fpf("trust", 5).await.unwrap().len(), 1);
        drop(opened);

        let manifests = manifest_files(&ws);
        assert!(
            !manifests.is_empty(),
            "the fixture must produce real Lance manifests"
        );
        for manifest in &manifests {
            std::fs::write(manifest, b"deliberately invalid owned table version").unwrap();
        }

        let error = AuxiliaryStore::open_existing(&ws)
            .await
            .err()
            .expect("a damaged manifest must be an error, not a panic");
        assert!(
            error.to_string().contains("Corrupt Lance manifest"),
            "{error}"
        );

        // Creating must not repair a damaged table behind the caller's back.
        let error = AuxiliaryStore::open_or_create(&ws)
            .await
            .err()
            .expect("a damaged manifest must be an error, not a panic");
        assert!(
            error.to_string().contains("Corrupt Lance manifest"),
            "{error}"
        );
    }

    /// A short manifest must be refused with the same typed error.
    #[tokio::test]
    async fn a_short_manifest_is_refused_with_a_typed_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let auxiliary = AuxiliaryStore::open_or_create(&ws).await.unwrap();
        auxiliary
            .insert_fpf_chunks(
                &[FpfChunk {
                    id: "fpf-C.2.2".into(),
                    section_id: "C.2.2".into(),
                    parent_section: None,
                    title: "Bounded contexts".into(),
                    body: "Contexts follow actual graph links.".into(),
                    line_count: 22,
                    file_path: "C.2.2.md".into(),
                    created_at: "2026-10-19T00:00:00Z".into(),
                }],
                None,
            )
            .await
            .unwrap();
        drop(auxiliary);

        let manifests = manifest_files(&ws);
        assert!(
            !manifests.is_empty(),
            "the fixture must produce real Lance manifests"
        );
        for manifest in &manifests {
            let bytes = std::fs::read(manifest).unwrap();
            std::fs::write(manifest, &bytes[..12]).unwrap();
        }

        let error = AuxiliaryStore::open_existing(&ws)
            .await
            .err()
            .expect("a short manifest must be an error, not a panic");
        assert!(
            error.to_string().contains("Corrupt Lance manifest"),
            "{error}"
        );
    }

    /// A damaged obsolete manifest must not break the latest read.
    ///
    /// Lance reads the manifest of the newest version only. An older
    /// manifest it never opens is not a reason to refuse the table.
    #[tokio::test]
    async fn an_obsolete_corrupt_manifest_does_not_break_the_latest_read() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let auxiliary = AuxiliaryStore::open_or_create(&ws).await.unwrap();
        auxiliary
            .insert_fpf_chunks(
                &[FpfChunk {
                    id: "fpf-B.3".into(),
                    section_id: "B.3".into(),
                    parent_section: None,
                    title: "Trust calculus".into(),
                    body: "Trust requires explicit evidence.".into(),
                    line_count: 12,
                    file_path: "B.3.md".into(),
                    created_at: "2026-10-19T00:00:00Z".into(),
                }],
                None,
            )
            .await
            .unwrap();
        drop(auxiliary);
        let obsolete = manifest_files(&ws);
        assert!(
            !obsolete.is_empty(),
            "the fixture must produce real Lance manifests"
        );

        // A second write makes the manifests above obsolete.
        let auxiliary = AuxiliaryStore::open_or_create(&ws).await.unwrap();
        auxiliary
            .insert_fpf_chunks(
                &[FpfChunk {
                    id: "fpf-C.2.2".into(),
                    section_id: "C.2.2".into(),
                    parent_section: None,
                    title: "Bounded contexts".into(),
                    body: "Contexts follow actual graph links.".into(),
                    line_count: 22,
                    file_path: "C.2.2.md".into(),
                    created_at: "2026-10-19T00:00:00Z".into(),
                }],
                None,
            )
            .await
            .unwrap();
        drop(auxiliary);
        assert!(
            manifest_files(&ws).len() > obsolete.len(),
            "the second write must leave the first manifest behind"
        );
        for manifest in &obsolete {
            assert!(
                manifest.exists(),
                "the first version's manifest must still be on disk"
            );
        }

        for manifest in &obsolete {
            std::fs::write(manifest, b"deliberately invalid owned table version").unwrap();
        }

        // The manifest Lance reads is intact, so the read still works.
        let opened = AuxiliaryStore::open_existing(&ws).await.unwrap();
        assert!(opened.has_fpf());
        assert_eq!(opened.search_fpf("contexts", 5).await.unwrap().len(), 1);
        assert_eq!(opened.list_fpf_sections().await.unwrap().len(), 2);
    }

    /// The legacy store must refuse a damaged manifest as well.
    ///
    /// It opens the same tables itself, and a panic there kills every
    /// command that reads a legacy workspace.
    #[tokio::test]
    async fn the_legacy_store_refuses_a_damaged_manifest_instead_of_aborting_the_process() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let legacy = LanceStore::init(&ws).await.unwrap();
        legacy
            .insert_fpf_chunks(
                &[FpfChunk {
                    id: "fpf-B.3".into(),
                    section_id: "B.3".into(),
                    parent_section: None,
                    title: "Trust calculus".into(),
                    body: "Trust requires explicit evidence.".into(),
                    line_count: 12,
                    file_path: "B.3.md".into(),
                    created_at: "2026-10-19T00:00:00Z".into(),
                }],
                None,
            )
            .await
            .unwrap();
        drop(legacy);

        // The intact legacy workspace opens.
        assert!(LanceStore::open(&ws).await.is_ok());

        let manifests = manifest_files(&ws);
        assert!(
            !manifests.is_empty(),
            "the fixture must produce real Lance manifests"
        );
        for manifest in &manifests {
            std::fs::write(manifest, b"deliberately invalid owned table version").unwrap();
        }

        let error = LanceStore::open(&ws)
            .await
            .err()
            .expect("a damaged manifest must be an error, not a panic");
        assert!(
            error.to_string().contains("Corrupt Lance manifest"),
            "{error}"
        );
    }
}
