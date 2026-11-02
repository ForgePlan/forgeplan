//! LanceDB schema migration — add missing columns without data loss.
//!
//! Each migration is idempotent: checks if column exists before adding.
//! Version tracked in config.yaml as `schema_version`.
//!
//! Schema v5 (ADR-030) adds the `canonical_key` column to the artifacts
//! table — the row's authoritative identity — and the **staged index
//! migration** below, which rebuilds the index from the Markdown and
//! publishes it behind a readiness marker.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::{RecordBatch, StringArray};
use lancedb::Table;
use lancedb::query::{ExecutableQuery, QueryBase, Select};
use lancedb::table::NewColumnTransform;

/// Current schema version. Increment when adding migrations.
///
/// v5: add the `canonical_key` column to artifacts (ADR-030) — the row's
/// authoritative identity. Uniqueness, upsert and every mutation use it.
pub const CURRENT_SCHEMA_VERSION: u32 = 5;

/// Run all pending migrations on the artifacts table.
/// Idempotent — safe to run multiple times.
pub async fn migrate_artifacts(table: &Table) -> anyhow::Result<()> {
    let schema = table.schema().await?;
    let field_names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

    // Migration 1→2: add body_hash column (nullable Utf8)
    if !field_names.contains(&"body_hash".to_string()) {
        eprintln!("[migrate] Adding body_hash column to artifacts");
        table
            .add_columns(
                NewColumnTransform::SqlExpressions(vec![(
                    "body_hash".to_string(),
                    "CAST(NULL AS STRING)".to_string(),
                )]),
                None,
            )
            .await?;
    }

    // Migration 3→4: add tags List(Utf8) column (PRD-035 FR-001).
    // Existing rows get NULL (interpreted as empty tag list on read).
    if !field_names.contains(&"tags".to_string()) {
        eprintln!("[migrate] Adding tags column to artifacts");
        // The lance-datafusion SQL parser does not understand `LIST<...>`
        // syntax, so `CAST(NULL AS LIST<STRING>)` fails at runtime on real
        // v3 tables (release blocker). Use `AllNulls` instead — it accepts an
        // Arrow schema directly and fills existing rows with typed nulls.
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;
        let tags_schema = Arc::new(Schema::new(vec![Field::new(
            "tags",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        )]));
        table
            .add_columns(NewColumnTransform::AllNulls(tags_schema), None)
            .await?;
    }


    Ok(())
}

/// Run all pending migrations on the relations table.
/// Idempotent — safe to run multiple times.
pub async fn migrate_relations(table: &Table) -> anyhow::Result<()> {
    let schema = table.schema().await?;
    let field_names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

    // Migration 1→2: add congruence_level column (nullable Int32, default 3)
    if !field_names.contains(&"congruence_level".to_string()) {
        eprintln!("[migrate] Adding congruence_level column to relations");
        table
            .add_columns(
                NewColumnTransform::SqlExpressions(vec![(
                    "congruence_level".to_string(),
                    "CAST(3 AS INT)".to_string(),
                )]),
                None,
            )
            .await?;
    }

    Ok(())
}

/// Create a missing history table during locked write preparation. Opening a
/// v4 workspace for reading does not create or migrate this table.
pub async fn ensure_change_log(db: &lancedb::connection::Connection) -> anyhow::Result<()> {
    let tables = db.table_names().execute().await?;
    if !tables.contains(&"change_log".to_string()) {
        use arrow_array::{RecordBatch, StringArray};
        use arrow_schema::ArrowError;
        use std::sync::Arc;

        let schema = super::schema::change_log_schema();
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(Vec::<&str>::new())),
                Arc::new(StringArray::from(Vec::<&str>::new())),
                Arc::new(StringArray::from(Vec::<&str>::new())),
                Arc::new(StringArray::from(Vec::<Option<&str>>::new())),
                Arc::new(StringArray::from(Vec::<Option<&str>>::new())),
                Arc::new(StringArray::from(Vec::<Option<&str>>::new())),
                Arc::new(StringArray::from(Vec::<&str>::new())),
                Arc::new(StringArray::from(Vec::<Option<&str>>::new())), // commit_hash
                Arc::new(StringArray::from(Vec::<Option<&str>>::new())), // canonical_key
            ],
        )
        .map_err(|e: ArrowError| anyhow::anyhow!("Failed to create change_log batch: {}", e))?;
        db.create_table("change_log", vec![batch]).execute().await?;
        eprintln!("[migrate] Created change_log table");
    }
    Ok(())
}

/// Migrate existing change_log table to add commit_hash column (v2→v3)
/// and the canonical_key column (ADR-030).
///
/// The `canonical_key` column carries the identity of the document an entry
/// is about: the display id in `artifact_id` is an alias and may name two
/// documents, the key names exactly one. Existing rows keep their
/// description and get a NULL key — a historical event is not rewritten.
pub async fn migrate_change_log(table: &Table) -> anyhow::Result<()> {
    let schema = table.schema().await?;
    let field_names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

    if !field_names.contains(&"commit_hash".to_string()) {
        eprintln!("[migrate] Adding commit_hash column to change_log");
        table
            .add_columns(
                NewColumnTransform::SqlExpressions(vec![(
                    "commit_hash".to_string(),
                    "CAST(NULL AS STRING)".to_string(),
                )]),
                None,
            )
            .await?;
    }

    if !field_names.contains(&"canonical_key".to_string()) {
        eprintln!("[migrate] Adding canonical_key column to change_log");
        table
            .add_columns(
                NewColumnTransform::SqlExpressions(vec![(
                    "canonical_key".to_string(),
                    "CAST(NULL AS STRING)".to_string(),
                )]),
                None,
            )
            .await?;
    }

    Ok(())
}

/// Run idempotent migrations on the `fpf_spec` table — currently adds the
/// `embedding` FixedSizeList<Float32, 1024> column for semantic search
/// (PRD-042). Pre-existing rows are filled with NULL via
/// `NewColumnTransform::AllNulls`.
pub async fn migrate_fpf_spec(table: &Table) -> anyhow::Result<()> {
    let schema = table.schema().await?;
    let field_names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

    if !field_names.contains(&"embedding".to_string()) {
        eprintln!("[migrate] Adding embedding column to fpf_spec");
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;
        let embedding_schema = Arc::new(Schema::new(vec![Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                super::schema::EMBEDDING_DIM,
            ),
            true,
        )]));
        table
            .add_columns(NewColumnTransform::AllNulls(embedding_schema), None)
            .await?;
    }

    Ok(())
}

/// Run all migrations on all tables. Call from LanceStore::open().
pub async fn run_migrations(
    artifacts: &Table,
    relations: &Table,
    change_log: Option<&Table>,
    fpf_spec: Option<&Table>,
) -> anyhow::Result<()> {
    migrate_artifacts(artifacts).await?;
    migrate_relations(relations).await?;
    if let Some(cl) = change_log {
        migrate_change_log(cl).await?;
    }
    if let Some(fs) = fpf_spec {
        migrate_fpf_spec(fs).await?;
    }
    Ok(())
}

// ── Identity-v5 staged migration ─────────────────────────────────────────────
//
// SPEC-009 "Staged index migration". The legacy Lance root gains three new
// tables — `artifacts_identity_v5`, `relations_identity_v5` and
// `evidence_identity_v5` — built FROM THE MARKDOWN, never by copying
// `canonical_key = id` in SQL: only the Markdown knows whether a document
// declares a slug.
//
// The old tables stay untouched during preparation, `fpf_spec` and the
// historical `change_log` are not duplicated, and publication happens only
// after the staged set has been verified. The readiness marker
// `lance/IDENTITY_V5.json` is the single source of truth about which set is
// live: after it exists only the v5 set is used, and a damaged marker is an
// explicit error — never a silent fallback to the pre-migration index.

/// The artifacts table of the identity-v5 index.
pub const IDENTITY_V5_ARTIFACTS_TABLE: &str = "artifacts_identity_v5";
/// The relations table of the identity-v5 index.
pub const IDENTITY_V5_RELATIONS_TABLE: &str = "relations_identity_v5";
/// The evidence table of the identity-v5 index.
pub const IDENTITY_V5_EVIDENCE_TABLE: &str = "evidence_identity_v5";
/// The readiness marker, inside the Lance root.
pub const IDENTITY_V5_MARKER_FILE: &str = "IDENTITY_V5.json";
/// The schema version the identity-v5 index is built for.
pub const IDENTITY_V5_VERSION: u32 = 5;

/// The workspace directories the staged migration reads the Markdown from.
pub const IDENTITY_V5_SOURCE_DIRS: &[&str] = crate::workspace::ARTIFACT_DIRS;

/// The readiness marker of the identity-v5 index.
///
/// Its presence — not the presence of a column — is what says the staged set
/// is live. The marker records the version, the three table names, the digest
/// of the Markdown the set was built from, and that the build completed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdentityV5Marker {
    /// The schema version the staged set was built for.
    pub version: u32,
    /// The artifacts table of the staged set.
    pub artifacts_table: String,
    /// The relations table of the staged set.
    pub relations_table: String,
    /// The evidence table of the staged set.
    pub evidence_table: String,
    /// The digest of the Markdown the set was built from.
    pub source_digest: String,
    /// How many documents the staged set holds.
    pub document_count: usize,
    /// Whether the build finished. A marker without it is damaged.
    pub completed: bool,
    /// Provenance of vectors recomputed during this migration, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embeddings: Option<IdentityV5EmbeddingProvenance>,
    /// Row-local semantic authority is enabled only after full backfill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic: Option<IdentityV5SemanticProtocol>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdentityV5SemanticProtocol {
    pub protocol: u32,
    pub active_profile_digest: Option<String>,
}


/// Pinned model and exact per-identity inputs for migrated embeddings.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdentityV5EmbeddingProvenance {
    pub model: String,
    pub repository: String,
    pub snapshot_id: String,
    pub snapshot_digest: String,
    pub implementation: String,
    pub profile_digest: String,
    pub chunk_size: usize,
    pub dimension: usize,
    pub input_digests: BTreeMap<String, String>,
    pub vector_digests: BTreeMap<String, String>,
}

/// The table names this workspace's legacy index lives in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyTableNames {
    /// The artifacts table.
    pub artifacts: String,
    /// The relations table.
    pub relations: String,
    /// The evidence table.
    pub evidence: String,
}

impl Default for LegacyTableNames {
    fn default() -> Self {
        Self {
            artifacts: "artifacts".to_string(),
            relations: "relations".to_string(),
            evidence: "evidence".to_string(),
        }
    }
}

/// One document the staged migration read from the Markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityV5Document {
    /// The canonical key: the frontmatter slug when the document declares a
    /// valid one, the declared id otherwise.
    pub canonical_key: String,
    /// The declared display id — an alias, not the identity.
    pub display_id: String,
    /// The artifact kind.
    pub kind: String,
    /// The lifecycle status.
    pub status: String,
    /// The document title.
    pub title: String,
    /// The body, without the frontmatter block.
    pub body: String,
    /// The exact Markdown bytes represented as UTF-8, including frontmatter.
    pub full_text: String,
    /// The depth (`tactical`, `standard`, `deep`).
    pub depth: String,
    /// The author.
    pub author: Option<String>,
    /// The parent epic.
    pub parent_epic: Option<String>,
    /// The evidence decay date.
    pub valid_until: Option<String>,
    /// The creation timestamp.
    pub created_at: String,
    /// The last-update timestamp.
    pub updated_at: String,
    /// The content hash of title + body.
    pub body_hash: Option<String>,
    /// The tags.
    pub tags: Vec<String>,
    /// The declared links: `(target, relation)`.
    pub links: Vec<(String, String)>,
    /// The workspace-relative path the document was read from.
    pub relative_path: String,
}

/// Everything the staged migration read from the Markdown.
#[derive(Debug, Clone)]
pub struct IdentityV5Source {
    /// The documents, ordered by canonical key.
    pub documents: Vec<IdentityV5Document>,
    /// A digest over every source file — its path and its bytes.
    pub source_digest: String,
}

/// What the staged migration found and did.
///
/// The two blocking findings are `duplicate_keys` (two documents claiming one
/// canonical key) and `unmappable` (a document that cannot be turned into a
/// row). Both stop publication and leave the original index exactly as it
/// was. `ambiguous_aliases` and `unresolved_references` are reported: an
/// alias may repeat (ADR-030) and a link may point at a document that does
/// not exist yet (a forward reference) — neither loses a document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdentityV5Report {
    /// How many documents the staged set holds.
    pub document_count: usize,
    /// How many relations the staged set holds.
    pub relation_count: usize,
    /// How many evidence rows the staged set holds.
    pub evidence_count: usize,
    /// The digest of the Markdown the set was built from.
    pub source_digest: String,
    /// Canonical keys claimed by more than one document — blocks publication.
    pub duplicate_keys: Vec<(String, Vec<String>)>,
    /// Documents that cannot be mapped into a row — blocks publication.
    pub unmappable: Vec<String>,
    /// Display aliases shared by more than one document — reported.
    pub ambiguous_aliases: Vec<(String, Vec<String>)>,
    /// Link targets that name no document in this workspace — reported.
    pub unresolved_references: Vec<String>,
    /// Whether the staged set was published.
    pub published: bool,
    /// Whether the staged set was already published before this run.
    pub already_published: bool,
}

impl IdentityV5Report {
    /// Whether a finding blocks publication.
    pub fn is_blocked(&self) -> bool {
        !self.duplicate_keys.is_empty() || !self.unmappable.is_empty()
    }
}

/// Lowercase hex of a byte string.
pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
    }
    out
}

/// The path of the readiness marker inside a Lance root.
pub fn identity_v5_marker_path(lance_dir: &Path) -> PathBuf {
    lance_dir.join(IDENTITY_V5_MARKER_FILE)
}

/// Read the readiness marker of the identity-v5 index.
///
/// `Ok(None)` — no marker: this workspace was never migrated.
/// `Err` — the marker exists and is damaged. A damaged marker is an explicit
/// error, never a silent fallback to the pre-migration index: a half-written
/// marker must not make the store read the wrong tables.
pub fn read_identity_v5_marker(lance_dir: &Path) -> anyhow::Result<Option<IdentityV5Marker>> {
    let path = identity_v5_marker_path(lance_dir);
    use std::io::Read;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let mut file = match options.open(&path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => anyhow::bail!("cannot read the identity-v5 readiness marker {}: {err}", path.display()),
    };
    let metadata = file.metadata()?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            anyhow::bail!("the identity-v5 readiness marker {} is a reparse point", path.display());
        }
    }
    if !metadata.is_file() {
        anyhow::bail!("the identity-v5 readiness marker {} is not a regular file", path.display());
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let marker: IdentityV5Marker = serde_json::from_slice(&bytes).map_err(|err| {
        anyhow::anyhow!(
            "the identity-v5 readiness marker {} is damaged and cannot be parsed: {err}. \
             Repair or remove it — the store will not fall back to the pre-migration index.",
            path.display()
        )
    })?;
    if marker.version != IDENTITY_V5_VERSION {
        anyhow::bail!(
            "the identity-v5 readiness marker {} names version {} but this build knows version {}. \
             Repair or remove it — the store will not fall back to the pre-migration index.",
            path.display(),
            marker.version,
            IDENTITY_V5_VERSION
        );
    }
    if !marker.completed {
        anyhow::bail!(
            "the identity-v5 readiness marker {} records an incomplete build. \
             Repair or remove it — the store will not fall back to the pre-migration index.",
            path.display()
        );
    }
    if marker.artifacts_table != IDENTITY_V5_ARTIFACTS_TABLE
        || marker.relations_table != IDENTITY_V5_RELATIONS_TABLE
        || marker.evidence_table != IDENTITY_V5_EVIDENCE_TABLE
        || marker.source_digest.len() != 64
        || !marker.source_digest.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        anyhow::bail!("the identity-v5 readiness marker {} is damaged: invalid table names or source digest", path.display());
    }
    if let Some(profile) = &marker.embeddings {
        let valid_digest = |value: &str| value.len() == 64
            && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
        if profile.model.is_empty() || profile.repository.is_empty() || profile.snapshot_id.is_empty()
            || !valid_digest(&profile.snapshot_digest) || profile.dimension != super::schema::EMBEDDING_DIM as usize
            || profile.implementation.is_empty() || profile.profile_digest != identity_profile_digest(profile)
            || (marker.semantic.is_none() && profile.input_digests.is_empty())
            || profile.input_digests.iter().any(|(key, digest)| key.is_empty() || !valid_digest(digest))
            || profile.vector_digests.len() != profile.input_digests.len()
            || profile.vector_digests.iter().any(|(key, digest)| !profile.input_digests.contains_key(key) || !valid_digest(digest))
        {
            anyhow::bail!("the identity-v5 readiness marker {} has damaged embedding provenance", path.display());
        }
    }
    if let Some(semantic) = &marker.semantic {
        if semantic.protocol != 1 || semantic.active_profile_digest.as_deref()
            != marker.embeddings.as_ref().map(|profile| profile.profile_digest.as_str())
        {
            anyhow::bail!("the identity-v5 readiness marker {} has an invalid semantic protocol", path.display());
        }
    }
    Ok(Some(marker))
}

/// Update an existing readiness marker while holding its publication lock.
pub(crate) fn publish_identity_v5_marker(lance_dir: &Path, marker: &IdentityV5Marker) -> anyhow::Result<()> {
    write_identity_v5_marker(lance_dir, marker, false)
}

/// Publish the first readiness marker without replacing an existing path.
pub(crate) fn publish_identity_v5_marker_new(lance_dir: &Path, marker: &IdentityV5Marker) -> anyhow::Result<()> {
    write_identity_v5_marker(lance_dir, marker, true)
}

#[cfg(windows)]
fn write_identity_v5_marker(lance_dir: &Path, marker: &IdentityV5Marker, create_only: bool) -> anyhow::Result<()> {
    use rand::TryRngCore;
    use std::io::Write;
    use std::os::windows::fs::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

    let (_, barrier, diagnostic) = crate::constellation::durability::windows_ntfs_primitive_barrier(lance_dir);
    let barrier = barrier.ok_or_else(|| anyhow::anyhow!("identity marker durability unavailable: {diagnostic:?}"))?;
    let bytes = serde_json::to_vec_pretty(marker)?;
    if create_only {
        crate::constellation::durability::install_immutable_file(
            lance_dir, IDENTITY_V5_MARKER_FILE, &bytes, barrier.as_ref(),
        )?;
        return Ok(());
    }
    let path = identity_v5_marker_path(lance_dir);
    let metadata = std::fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        anyhow::bail!("identity-v5 readiness marker is not a real file");
    }
    let mut nonce = [0u8; 16];
    rand::rngs::OsRng.try_fill_bytes(&mut nonce)
        .map_err(|error| anyhow::anyhow!("cannot create identity marker transaction: {error}"))?;
    let tmp = lance_dir.join(format!("{IDENTITY_V5_MARKER_FILE}.{}.tmp", hex_encode(&nonce)));
    struct MarkerTemp(PathBuf);
    impl Drop for MarkerTemp {
        fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
    }
    let _temp_guard = MarkerTemp(tmp.clone());
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    barrier.sync_file(&tmp)?;
    barrier.replace_atomically(&tmp, &path)?;
    barrier.sync_dir(lance_dir)?;
    Ok(())
}

#[cfg(not(windows))]
fn write_identity_v5_marker(lance_dir: &Path, marker: &IdentityV5Marker, create_only: bool) -> anyhow::Result<()> {
    use rand::TryRngCore;
    use std::io::Write;
    let metadata = std::fs::symlink_metadata(lance_dir)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        anyhow::bail!("identity-v5 Lance directory is not a real directory");
    }
    let path = identity_v5_marker_path(lance_dir);
    if !create_only {
        let metadata = std::fs::symlink_metadata(&path)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            anyhow::bail!("identity-v5 readiness marker is not a real file");
        }
    }
    let mut nonce = [0u8; 16];
    rand::rngs::OsRng.try_fill_bytes(&mut nonce)
        .map_err(|error| anyhow::anyhow!("cannot create identity marker transaction: {error}"))?;
    let tmp = lance_dir.join(format!("{IDENTITY_V5_MARKER_FILE}.{}.tmp", hex_encode(&nonce)));
    let bytes = serde_json::to_vec_pretty(marker)?;
    struct MarkerTemp(PathBuf);
    impl Drop for MarkerTemp {
        fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
    }
    let _temp_guard = MarkerTemp(tmp.clone());
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW).mode(0o600);
    }
    let mut file = options.open(&tmp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    let dir = std::fs::File::open(lance_dir)?;
    if create_only {
        // Both paths are in the same directory; link fails atomically if
        // anything already occupies the final name, including a symlink.
        std::fs::hard_link(&tmp, &path)?;
        dir.sync_all()?;
        std::fs::remove_file(&tmp)?;
        dir.sync_all()?;
    } else {
        std::fs::rename(&tmp, &path)?;
        dir.sync_all()?;
    }
    Ok(())
}
pub(crate) const SEMANTIC_COLUMNS: [&str; 6] = [
    "semantic_input_digest", "semantic_profile_digest", "semantic_vector_digest",
    "semantic_job_id", "semantic_attempt_token", "semantic_state",
];

/// Complete or resume row-local semantic activation under the workspace lock.
/// Until the marker is switched, existing marker maps remain authoritative.
pub(crate) async fn prepare_identity_semantic_protocol(
    workspace: &Path,
    db: &lancedb::connection::Connection,
    lock: &crate::workspace::lock::WorkspaceLock,
) -> anyhow::Result<()> {
    #[cfg(windows)]
    anyhow::bail!("legacy semantic activation is unavailable on Windows");
    use crate::workspace::lock::WorkspaceReadPermit;
    lock.matches_workspace(workspace)?;
    let lance = workspace.join("lance");
    let mut marker = read_identity_v5_marker(&lance)?
        .ok_or_else(|| anyhow::anyhow!("identity-v5 readiness marker is missing"))?;
    let table = db.open_table(&marker.artifacts_table).execute().await?;
    let schema = table.schema().await?;
    for name in SEMANTIC_COLUMNS {
        if let Ok(field) = schema.field_with_name(name) {
            if field.data_type() != &arrow_schema::DataType::Utf8 || !field.is_nullable() {
                anyhow::bail!("identity-v5 semantic column '{name}' has an incompatible type");
            }
        } else if marker.semantic.is_some() {
            anyhow::bail!("published identity-v5 semantic column '{name}' is missing");
        } else {
            table.add_columns(
                NewColumnTransform::SqlExpressions(vec![(name.to_string(), "CAST(NULL AS STRING)".to_string())]),
                None,
            ).await?;
        }
    }
    if marker.semantic.is_some() {
        return Ok(());
    }
    let rows = table_rows(db, &marker.artifacts_table).await?;
    let mut seen = std::collections::BTreeSet::new();
    let mut expected = BTreeMap::new();
    for row in &rows {
        let key = required_string(row, "canonical_key")?;
        if !seen.insert(key.clone()) {
            anyhow::bail!("duplicate semantic identity '{key}'");
        }
        let embedding = row.column_by_name("embedding")
            .ok_or_else(|| anyhow::anyhow!("artifact '{key}' has no embedding column"))?;
        let (input, profile, vector, state) = if embedding.is_null(0) {
            (None, None, None, "pending")
        } else {
            let profile = marker.embeddings.as_ref()
                .ok_or_else(|| anyhow::anyhow!("artifact '{key}' has an unproven vector"))?;
            let title = required_string(row, "title")?;
            let body = super::store::record_large_string(row, "body", 0)
                .ok_or_else(|| anyhow::anyhow!("artifact '{key}' has no body"))?;
            let input = identity_input_digest(&format!("{} {}", title, body.chars().take(profile.chunk_size).collect::<String>()));
            let values = super::convert::extract_embedding(row, 0)
                .ok_or_else(|| anyhow::anyhow!("artifact '{key}' has an invalid vector"))?;
            let vector = identity_vector_digest(&values);
            if values.len() != profile.dimension || values.iter().any(|value| !value.is_finite())
                || !values.iter().any(|value| *value != 0.0)
                || profile.input_digests.get(&key) != Some(&input)
                || profile.vector_digests.get(&key) != Some(&vector)
            {
                anyhow::bail!("artifact '{key}' does not match its migration embedding provenance");
            }
            (Some(input), Some(profile.profile_digest.clone()), Some(vector), "ready")
        };
        let sql = |value: Option<&str>| value.map(|value| format!("'{}'", value.replace('\'', "''")))
            .unwrap_or_else(|| "NULL".to_string());
        let result = table.update()
            .only_if(format!("canonical_key = '{}'", key.replace('\'', "''")))
            .column("semantic_input_digest", sql(input.as_deref()))
            .column("semantic_profile_digest", sql(profile.as_deref()))
            .column("semantic_vector_digest", sql(vector.as_deref()))
            .column("semantic_job_id", "NULL")
            .column("semantic_attempt_token", "NULL")
            .column("semantic_state", format!("'{state}'"))
            .execute().await?;
        if result.rows_updated != 1 {
            anyhow::bail!("semantic backfill changed {} rows for '{key}'", result.rows_updated);
        }
        expected.insert(key, (input, profile, vector, state, row.clone()));
    }
    let backfilled = table_rows(db, &marker.artifacts_table).await?;
    if backfilled.len() != rows.len() {
        anyhow::bail!("semantic backfill changed the artifact count");
    }
    for row in &backfilled {
        let key = required_string(row, "canonical_key")?;
        let (input, profile, vector, state, original) = expected.remove(&key)
            .ok_or_else(|| anyhow::anyhow!("unexpected semantic row '{key}'"))?;
        if !seen.remove(&key)
            || super::store::record_string(row, "semantic_input_digest", 0) != input
            || super::store::record_string(row, "semantic_profile_digest", 0) != profile
            || super::store::record_string(row, "semantic_vector_digest", 0) != vector
            || required_string(row, "semantic_state")? != state
            || super::store::record_string(row, "semantic_job_id", 0).is_some()
            || super::store::record_string(row, "semantic_attempt_token", 0).is_some()
        {
            anyhow::bail!("semantic backfill failed to verify '{key}'");
        }
        for field in original.schema().fields() {
            if !field.name().starts_with("semantic_")
                && original.column_by_name(field.name()).map(|column| column.to_data())
                    != row.column_by_name(field.name()).map(|column| column.to_data())
            {
                anyhow::bail!("semantic backfill changed artifact payload '{key}'");
            }
        }
    }
    if !seen.is_empty() || read_identity_v5_marker(&lance)?.as_ref() != Some(&marker) {
        anyhow::bail!("identity-v5 state changed during semantic activation");
    }
    marker.semantic = Some(IdentityV5SemanticProtocol {
        protocol: 1,
        active_profile_digest: marker.embeddings.as_ref().map(|profile| profile.profile_digest.clone()),
    });
    publish_identity_v5_marker(&lance, &marker)
}

/// The table names this workspace's legacy index lives in.
///
/// A workspace whose identity-v5 migration has been published keeps its rows
/// in the three tables the marker names; a workspace without a marker uses
/// the default names.
pub fn resolve_legacy_table_names(lance_dir: &Path) -> anyhow::Result<LegacyTableNames> {
    match read_identity_v5_marker(lance_dir)? {
        Some(marker) => Ok(LegacyTableNames {
            artifacts: marker.artifacts_table,
            relations: marker.relations_table,
            evidence: marker.evidence_table,
        }),
        None => Ok(LegacyTableNames::default()),
    }
}

/// The kind a workspace directory holds.
fn kind_for_dir(dir: &str) -> &'static str {
    match dir {
        "prds" => "prd",
        "epics" => "epic",
        "specs" => "spec",
        "rfcs" => "rfc",
        "adrs" => "adr",
        "problems" => "problem",
        "solutions" => "solution",
        "evidence" => "evidence",
        "notes" => "note",
        "refresh" => "refresh",
        "memory" => "memory",
        "use_cases" => "use_case",
        "glossary" => "glossary",
        "invariants" => "invariant",
        "scenarios" => "scenario",
        "hypotheses" => "hypothesis",
        "domain_models" => "domain_model",
        _ => "note",
    }
}

/// A frontmatter value as a trimmed, non-empty string.
fn frontmatter_string(
    frontmatter: &BTreeMap<String, serde_yaml::Value>,
    key: &str,
) -> Option<String> {
    frontmatter
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Parse one Markdown document into the row the identity-v5 index holds.
///
/// The key is the ADR-030 rule: the frontmatter slug when the document
/// declares a valid one, the declared id otherwise. A document that declares
/// neither cannot be mapped — the caller reports it and publication stops.
pub(crate) fn parse_identity_v5_document(
    relative_path: &str,
    text: &str,
    fallback_timestamp: &str,
) -> anyhow::Result<IdentityV5Document> {
    chrono::DateTime::parse_from_rfc3339(fallback_timestamp)
        .map_err(|error| anyhow::anyhow!("invalid identity capture timestamp: {error}"))?;
    let (frontmatter, body) = crate::artifact::frontmatter::parse_frontmatter(text)
        .map_err(|err| anyhow::anyhow!("the frontmatter does not parse: {err}"))?;

    let slug = crate::artifact::frontmatter::slug_from_frontmatter(&frontmatter)
        .filter(|slug| crate::artifact::types::is_canonical_key(slug))
        .map(str::to_string);
    let display_id = frontmatter_string(&frontmatter, "id").ok_or_else(|| {
        anyhow::anyhow!(
            "the document declares neither a valid `slug:` nor an `id:`, so it has no identity"
        )
    })?;
    let canonical_key = slug.clone().unwrap_or_else(|| display_id.clone());

    let dir = relative_path
        .split('/')
        .next()
        .unwrap_or(relative_path)
        .to_string();
    let kind = frontmatter_string(&frontmatter, "kind")
        .unwrap_or_else(|| kind_for_dir(&dir).to_string());
    let status = frontmatter_string(&frontmatter, "status").unwrap_or_else(|| "draft".to_string());
    let title = frontmatter_string(&frontmatter, "title").unwrap_or_else(|| {
        relative_path
            .rsplit('/')
            .next()
            .unwrap_or(relative_path)
            .trim_end_matches(".md")
            .to_string()
    });
    let depth =
        frontmatter_string(&frontmatter, "depth").unwrap_or_else(|| "standard".to_string());
    let now = fallback_timestamp;
    let created_at = frontmatter_string(&frontmatter, "created_at")
        .or_else(|| frontmatter_string(&frontmatter, "created"))
        .unwrap_or_else(|| now.to_string());
    let updated_at = frontmatter_string(&frontmatter, "updated_at")
        .or_else(|| frontmatter_string(&frontmatter, "updated"))
        .unwrap_or_else(|| now.to_string());

    Ok(IdentityV5Document {
        canonical_key,
        display_id,
        kind,
        status,
        title,
        body_hash: Some(crate::db::store::compute_content_hash(
            &frontmatter_string(&frontmatter, "title").unwrap_or_default(),
            &body,
        )),
        body,
        full_text: text.to_string(),
        depth,
        author: crate::artifact::frontmatter::author_from_frontmatter(&frontmatter),
        parent_epic: frontmatter_string(&frontmatter, "parent_epic"),
        valid_until: frontmatter_string(&frontmatter, "valid_until"),
        created_at,
        updated_at,
        tags: crate::artifact::frontmatter::tags_from_frontmatter(&frontmatter),
        links: crate::artifact::frontmatter::links_from_frontmatter(&frontmatter),
        relative_path: relative_path.to_string(),
    })
}

/// Read the complete artifact source set without following symlinks.
pub(crate) async fn read_identity_source_files(workspace: &Path) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for dir in IDENTITY_V5_SOURCE_DIRS {
        let root = workspace.join(dir);
        match tokio::fs::symlink_metadata(&root).await {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(anyhow::anyhow!("cannot inspect {}: {err}", root.display())),
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                anyhow::bail!("artifact source {} is not a real directory", root.display());
            }
            Ok(_) => {}
        }
        let mut pending = vec![root];
        while let Some(path) = pending.pop() {
            let mut entries = tokio::fs::read_dir(&path).await
                .map_err(|err| anyhow::anyhow!("cannot read {}: {err}", path.display()))?;
            while let Some(entry) = entries.next_entry().await? {
                let entry_path = entry.path();
                let kind = entry.file_type().await?;
                if kind.is_symlink() {
                    anyhow::bail!("artifact source contains a symlink: {}", entry_path.display());
                }
                if kind.is_dir() {
                    pending.push(entry_path);
                    continue;
                }
                if entry_path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                    continue;
                }
                if !kind.is_file() {
                    anyhow::bail!("artifact source is not a regular file: {}", entry_path.display());
                }
                let relative = entry_path.strip_prefix(workspace)?
                    .to_str().ok_or_else(|| anyhow::anyhow!("artifact path is not UTF-8: {}", entry_path.display()))?
                    .replace(std::path::MAIN_SEPARATOR, "/");
                let bytes = tokio::fs::read(&entry_path).await?;
                files.push((relative, bytes));
            }
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

/// Read every document of the workspace from the Markdown.
///
/// The staged index is built from source bytes, never from legacy rows alone.
pub async fn read_identity_v5_source(workspace: &Path) -> anyhow::Result<IdentityV5Source> {
    read_identity_v5_source_at(workspace, &chrono::Utc::now().to_rfc3339()).await
}

/// Capture using one caller-supplied, durably frozen insertion timestamp.
pub(crate) async fn read_identity_v5_source_at(workspace: &Path, fallback_timestamp: &str) -> anyhow::Result<IdentityV5Source> {
    chrono::DateTime::parse_from_rfc3339(fallback_timestamp)
        .map_err(|error| anyhow::anyhow!("invalid identity capture timestamp: {error}"))?;
    use sha2::{Digest, Sha256};
    let files = read_identity_source_files(workspace).await?;

    // The digest covers every source file: its path and its bytes.
    let mut hasher = Sha256::new();
    for (relative, bytes) in &files {
        hasher.update(relative.as_bytes());
        hasher.update([0u8]);
        hasher.update(Sha256::digest(bytes));
    }
    let source_digest = hex_encode(&hasher.finalize());

    let mut documents = Vec::with_capacity(files.len());
    for (relative, bytes) in &files {
        let text = std::str::from_utf8(bytes)
            .map_err(|error| anyhow::anyhow!("{relative} is not valid UTF-8: {error}"))?;
        match parse_identity_v5_document(relative, text, fallback_timestamp) {
            Ok(document) => documents.push(document),
            Err(reason) => {
                anyhow::bail!(
                    "{} cannot be mapped into the identity-v5 index: {reason}",
                    relative
                );
            }
        }
    }
    documents.sort_by(|left, right| left.canonical_key.cmp(&right.canonical_key));
    Ok(IdentityV5Source {
        documents,
        source_digest,
    })
}

/// Verify the staged set before it may be published.
///
/// SPEC-009 asks for key uniqueness, alias distinguishability and the
/// references to be checked. A canonical duplicate blocks publication; a
/// repeated display alias is reported (ADR-030 lets an alias repeat) and a
/// link target that names no document is reported as a forward reference.
pub fn verify_identity_v5_source(source: &IdentityV5Source) -> IdentityV5Report {
    let mut by_key: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut by_alias: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for document in &source.documents {
        by_key
            .entry(document.canonical_key.clone())
            .or_default()
            .push(document.relative_path.clone());
        by_alias
            .entry(document.display_id.clone())
            .or_default()
            .push(document.relative_path.clone());
    }

    let mut report = IdentityV5Report {
        document_count: source.documents.len(),
        source_digest: source.source_digest.clone(),
        ..Default::default()
    };
    for (key, paths) in by_key {
        if paths.len() > 1 {
            report.duplicate_keys.push((key, paths));
        }
    }
    for (alias, paths) in by_alias {
        if paths.len() > 1 {
            report.ambiguous_aliases.push((alias, paths));
        }
    }

    let candidates = identity_v5_candidates(source);
    for document in &source.documents {
        for (target, _relation) in &document.links {
            if target.contains('@') {
                continue;
            }
            match crate::artifact::canonical_key::resolve_structural_target(target, &candidates) {
                Ok(Some(_)) => {}
                Ok(None) => report.unresolved_references.push(format!(
                    "{} -> {}", document.relative_path, target
                )),
                Err(error) => report.unmappable.push(format!(
                    "{} -> {}: {}", document.relative_path, target, error
                )),
            }
        }
    }
    report
}


/// Write the three staged tables of the identity-v5 index.
///
/// The old tables are not touched, and `fpf_spec` and `change_log` are not
/// duplicated. Re-running before the marker exists rebuilds only these three
/// tables.
pub(crate) async fn write_identity_v5_tables(
    lance_dir: &Path,
    guard: &crate::workspace::lock::WorkspaceLock,
    source: &IdentityV5Source,
    report: &mut IdentityV5Report,
) -> anyhow::Result<()> {
    use crate::workspace::lock::WorkspaceReadPermit;
    let workspace = lance_dir.parent().ok_or_else(|| anyhow::anyhow!("Lance path has no workspace parent"))?;
    guard.matches_workspace(workspace)?;
    if verify_identity_v5_source(source).is_blocked() {
        anyhow::bail!("identity-v5 source contains conflicting or unmappable identities");
    }
    if read_identity_v5_marker(lance_dir)?.is_some() {
        anyhow::bail!("cannot rebuild a published identity-v5 index");
    }
    let db = lancedb::connect(lance_dir.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 LanceDB path"))?)
        .execute().await?;
    let rows = prepare_identity_v5_rows(&db, source, false).await?;
    report.relation_count = rows[1].len();
    report.evidence_count = rows[2].len();
    drop(db);
    write_prepared_identity_v5_rows(lance_dir, &rows).await
}

#[cfg(windows)]
pub(crate) async fn write_prepared_identity_v5_rows(
    root: &Path,
    rows: &[Vec<RecordBatch>; 3],
) -> anyhow::Result<()> {
    use rand::TryRngCore;
    use crate::constellation::durability;

    let (_, barrier, diagnostic) = durability::windows_ntfs_primitive_barrier(root);
    let barrier = barrier.ok_or_else(|| anyhow::anyhow!("legacy stage durability unavailable: {diagnostic:?}"))?;
    barrier.ensure_dir(root)?;
    let canonical_root = std::fs::canonicalize(root)?;
    let root = canonical_root.as_path();
    let tracker = Arc::new(durability::StageTracker::default());
    tracker.begin_stage(1);
    let wrapper = durability::tracked_legacy_store(root, Arc::clone(&tracker), Arc::clone(&barrier));
    let wrapper: Arc<dyn lance_io::object_store::WrappingObjectStore> = wrapper;
    let names = [IDENTITY_V5_ARTIFACTS_TABLE, IDENTITY_V5_RELATIONS_TABLE, IDENTITY_V5_EVIDENCE_TABLE];
    {
    let uri = durability::windows_wrapped_uri(root)?;
    let session = durability::windows_wrapped_session();
    let connection = lancedb::connect(&uri).session(Arc::clone(&session)).execute().await?;
    let database = Arc::clone(connection.database());
    let schemas = [super::schema::artifacts_schema(), super::schema::relations_schema(), super::schema::evidence_schema()];
    for ((name, schema), batches) in names.into_iter().zip(schemas).zip(rows) {
        let live = root.join(format!("{name}.lance"));
        match std::fs::symlink_metadata(&live) {
            Ok(meta) => {
                use std::os::windows::fs::MetadataExt;
                use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
                if !meta.is_dir() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                    anyhow::bail!("legacy stage table is not a real directory: {}", live.display());
                }
                let mut nonce = [0u8; 16];
                rand::rngs::OsRng.try_fill_bytes(&mut nonce)
                    .map_err(|error| anyhow::anyhow!("cannot retire stage table: {error}"))?;
                let retired = root.join(format!(".retired-{name}-{}", hex_encode(&nonce)));
                rename_identity_noreplace(&live, &retired)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let uri = durability::windows_wrapped_uri(&live)?;
        let params = lance::dataset::WriteParams {
            session: Some(Arc::clone(&session)),
            ..Default::default()
        };
        let native = lancedb::table::NativeTable::create_empty(
            &uri, name, Vec::new(), schema, Some(Arc::clone(&wrapper)), Some(params), None, None, false,
        ).await?;
        let table = lancedb::table::Table::new(Arc::new(native), Arc::clone(&database));
        if !batches.is_empty() {
            table.add(batches.clone()).execute().await?;
        }
    }
    }
    drop(wrapper);
    let writes = tracker.finish_writes()?;
    tracker.sync_written_set(&writes, root, barrier.as_ref())?;
    for name in names {
        durability::seal_unpublished_tree(&root.join(format!("{name}.lance")))?;
    }
    Ok(())
}


#[cfg(not(windows))]
pub(crate) async fn write_prepared_identity_v5_rows(
    root: &Path,
    rows: &[Vec<RecordBatch>; 3],
) -> anyhow::Result<()> {
    let db = lancedb::connect(root.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 LanceDB path"))?)
        .execute().await?;
    let schemas = [super::schema::artifacts_schema(), super::schema::relations_schema(), super::schema::evidence_schema()];
    let names = [IDENTITY_V5_ARTIFACTS_TABLE, IDENTITY_V5_RELATIONS_TABLE, IDENTITY_V5_EVIDENCE_TABLE];
    let existing = db.table_names().execute().await?;
    for ((name, schema), batches) in names.into_iter().zip(schemas).zip(rows.iter()) {
        if existing.iter().any(|entry| entry == name) {
            db.drop_table(name, &[]).await?;
        }
        let table = db.create_table(name, vec![RecordBatch::new_empty(schema)]).execute().await?;

        if !batches.is_empty() {
            table.add(batches.clone()).execute().await?;
        }
    }
    Ok(())
}

pub(crate) fn identity_input_digest(input: &str) -> String {
    use sha2::{Digest, Sha256};
    hex_encode(&Sha256::digest(input.as_bytes()))
}

pub(crate) fn identity_vector_digest(vector: &[f32]) -> String {
    use sha2::{Digest, Sha256};
    let mut digest = Sha256::new();
    for value in vector {
        digest.update(value.to_bits().to_le_bytes());
    }
    hex_encode(&digest.finalize())
}

pub(crate) fn identity_profile_digest(profile: &IdentityV5EmbeddingProvenance) -> String {
    use sha2::{Digest, Sha256};
    let mut digest = Sha256::new();
    digest.update(b"forgeplan-identity-v5-embedding-profile\0");
    for part in [&profile.model, &profile.repository, &profile.snapshot_id, &profile.snapshot_digest, &profile.implementation] {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part.as_bytes());
    }
    digest.update((profile.chunk_size as u64).to_le_bytes());
    digest.update((profile.dimension as u64).to_le_bytes());
    hex_encode(&digest.finalize())
}

#[cfg(any(test, feature = "tract-engine"))]
/// Replace only non-null legacy vectors using the pinned backend. Nothing is
/// written to LanceDB until every input has been recomputed successfully.
fn recompute_identity_vectors<F>(
    rows: &mut [RecordBatch],
    mut profile: IdentityV5EmbeddingProvenance,
    mut embed: F,
) -> anyhow::Result<IdentityV5EmbeddingProvenance>
where
    F: FnMut(&str) -> anyhow::Result<Vec<f32>>,
{
    if profile.dimension != super::schema::EMBEDDING_DIM as usize {
        anyhow::bail!("embedding profile dimension is incompatible with the v5 table");
    }
    let profile_digest = identity_profile_digest(&profile);
    if !profile.profile_digest.is_empty() && profile.profile_digest != profile_digest {
        anyhow::bail!("embedding profile fingerprint is inconsistent");
    }
    profile.profile_digest = profile_digest;
    for row in rows {
        let embedding = row.column_by_name("embedding")
            .ok_or_else(|| anyhow::anyhow!("artifact row has no embedding column"))?;
        if embedding.is_null(0) {
            continue;
        }
        let key = required_string(row, "canonical_key")?;
        let title = required_string(row, "title")?;
        let body = crate::db::store::record_large_string(row, "body", 0)
            .ok_or_else(|| anyhow::anyhow!("artifact '{key}' has no body"))?;
        let input = format!("{} {}", title, body.chars().take(profile.chunk_size).collect::<String>());
        let input_digest = identity_input_digest(&input);
        let vector = embed(&input)?;
        if vector.len() != profile.dimension || vector.iter().any(|value| !value.is_finite()) || !vector.iter().any(|value| *value != 0.0) {
            anyhow::bail!("embedding backend returned an invalid vector for '{key}'");
        }
        let vector_digest = identity_vector_digest(&vector);
        if profile.input_digests.insert(key.clone(), input_digest.clone()).is_some() {
            anyhow::bail!("duplicate embedding identity '{key}'");
        }
        profile.vector_digests.insert(key, vector_digest.clone());
        let mut columns = row.columns().to_vec();
        columns[row.schema().index_of("embedding")?] = super::convert::make_embedding_col_from_option(Some(&vector));
        for (name, value) in [
            ("semantic_input_digest", input_digest.as_str()),
            ("semantic_profile_digest", profile.profile_digest.as_str()),
            ("semantic_vector_digest", vector_digest.as_str()),
            ("semantic_state", "ready"),
        ] {
            columns[row.schema().index_of(name)?] = Arc::new(StringArray::from(vec![value]));
        }
        *row = RecordBatch::try_new(row.schema(), columns)?;
    }
    Ok(profile)
}

#[cfg(feature = "tract-engine")]
fn embedding_snapshot_digest(snapshot: &Path) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut hash = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    for relative in crate::embed::tract_engine::REQUIRED_FILES {
        hash.update((relative.len() as u64).to_le_bytes());
        hash.update(relative.as_bytes());
        let mut file = std::fs::File::open(snapshot.join(relative))?;
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 { break; }
            hash.update(&buffer[..count]);
        }
    }
    Ok(hex_encode(&hash.finalize()))
}

pub(crate) async fn recompute_configured_vectors(
    workspace: &Path,
    rows: &mut Vec<RecordBatch>,
) -> anyhow::Result<Option<IdentityV5EmbeddingProvenance>> {
    if !rows.iter().any(|row| row.column_by_name("embedding").is_some_and(|column| !column.is_null(0))) {
        return Ok(None);
    }
    #[cfg(all(feature = "semantic-search", any(test, all(feature = "test-helpers", debug_assertions))))]
    {
        if crate::workspace::load_config(workspace)?.embedding.as_ref()
            .is_some_and(|config| config.model == "forgeplan-test")
        {
            let profile = test_identity_profile(workspace)?;
            let backend = super::store::semantic_backend_for_test(workspace)?
                .ok_or_else(|| anyhow::anyhow!("configured test embedding backend is not installed"))?;
            let mut candidate = rows.clone();
            let profile = recompute_identity_vectors(&mut candidate, profile, |input| backend(input))?;
            validate_current_embedding_profile(workspace, &profile)?;
            *rows = candidate;
            return Ok(Some(profile));
        }
    }
    #[cfg(not(feature = "tract-engine"))]
    {
        let _ = workspace;
        anyhow::bail!("legacy embeddings require the configured embedding backend, unavailable in this build");
    }
    #[cfg(feature = "tract-engine")]
    {
        let config = crate::workspace::load_config(workspace)?.embedding
            .ok_or_else(|| anyhow::anyhow!("legacy vectors require an explicitly configured embedding model"))?;
        let model = config.model.clone();
        let chunk_size = config.chunk_size;
        let mut inputs = rows.clone();
        let (recomputed, profile) = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
            let repo = crate::embed::resolve_repo(&model)
                .ok_or_else(|| anyhow::anyhow!("unsupported embedding model '{model}'"))?;
            let snapshot = crate::embed::tract_engine::find_snapshot(&crate::embed::resolve_cache_dir(), repo)
                .ok_or_else(|| anyhow::anyhow!("configured embedding model '{model}' has no cached snapshot"))?;
            let snapshot_id = snapshot.file_name().and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .ok_or_else(|| anyhow::anyhow!("embedding snapshot has no valid revision"))?.to_string();
            let snapshot_digest = embedding_snapshot_digest(&snapshot)?;
            let mut embedder = crate::embed::Embedder::from_snapshot(&snapshot, &model)?;
            let dimension = embedder.dim();
            let profile = IdentityV5EmbeddingProvenance {
                model, repository: repo.to_string(), snapshot_id, snapshot_digest: snapshot_digest.clone(),
                implementation: "forgeplan-tract-bge-m3-cls-l2-512-v1".into(), profile_digest: String::new(),
                chunk_size, dimension, input_digests: BTreeMap::new(), vector_digests: BTreeMap::new(),
            };
            let profile = recompute_identity_vectors(&mut inputs, profile, |text| embedder.embed(text))?;
            if embedding_snapshot_digest(&snapshot)? != snapshot_digest {
                anyhow::bail!("embedding snapshot changed during recomputation");
            }
            Ok((inputs, profile))
        }).await??;
        let current = crate::workspace::load_config(workspace)?.embedding
            .ok_or_else(|| anyhow::anyhow!("embedding configuration disappeared during recomputation"))?;
        if current.model != profile.model || current.chunk_size != profile.chunk_size {
            anyhow::bail!("embedding profile changed during recomputation");
        }
        *rows = recomputed;
        Ok(Some(profile))
    }
}

pub(crate) fn validate_current_embedding_profile(
    workspace: &Path,
    profile: &IdentityV5EmbeddingProvenance,
) -> anyhow::Result<()> {
    if profile.implementation == "forgeplan-test-only" {
        #[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
        {
            let config = crate::workspace::load_config(workspace)?.embedding
                .ok_or_else(|| anyhow::anyhow!("embedding model configuration is missing"))?;
            if config.model == "forgeplan-test" && config.chunk_size == profile.chunk_size
                && profile.model == "forgeplan-test" && profile.repository == "test-only"
                && profile.snapshot_id == "in-process" && profile.snapshot_digest == identity_input_digest("forgeplan-test")
                && profile.dimension == super::schema::EMBEDDING_DIM as usize
                && profile.profile_digest == identity_profile_digest(profile)
                && super::store::semantic_backend_for_test(workspace)?.is_some()
            {
                return Ok(());
            }
        }
        anyhow::bail!("test embedding profile has no matching live test backend");
    }
    #[cfg(not(feature = "tract-engine"))]
    {
        let _ = (workspace, profile);
        anyhow::bail!("embedding backend is unavailable in this build");
    }
    #[cfg(feature = "tract-engine")]
    {
        if profile.implementation != "forgeplan-tract-bge-m3-cls-l2-512-v1"
            || profile.profile_digest != identity_profile_digest(profile)
        {
            anyhow::bail!("embedding implementation or profile fingerprint does not match");
        }
        let config = crate::workspace::load_config(workspace)?.embedding
            .ok_or_else(|| anyhow::anyhow!("embedding model configuration is missing"))?;
        if config.model != profile.model || config.chunk_size != profile.chunk_size {
            anyhow::bail!("embedding profile changed before publication");
        }
        let repo = crate::embed::resolve_repo(&profile.model)
            .ok_or_else(|| anyhow::anyhow!("unsupported embedding model"))?;
        if repo != profile.repository {
            anyhow::bail!("embedding model repository changed before publication");
        }
        let snapshot = crate::embed::tract_engine::find_snapshot(&crate::embed::resolve_cache_dir(), repo)
            .ok_or_else(|| anyhow::anyhow!("embedding snapshot disappeared before publication"))?;
        if snapshot.file_name().and_then(|name| name.to_str()) != Some(profile.snapshot_id.as_str())
            || embedding_snapshot_digest(&snapshot)? != profile.snapshot_digest
        {
            anyhow::bail!("embedding snapshot changed before publication");
        }
        Ok(())
    }
}

#[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
pub(crate) fn test_identity_profile(workspace: &Path) -> anyhow::Result<IdentityV5EmbeddingProvenance> {
    let config = crate::workspace::load_config(workspace)?.embedding.unwrap_or_default();
    if config.model != "forgeplan-test" || super::store::semantic_backend_for_test(workspace)?.is_none() {
        anyhow::bail!("configured test embedding backend is not installed");
    }
    let mut profile = IdentityV5EmbeddingProvenance {
        model: config.model, repository: "test-only".into(), snapshot_id: "in-process".into(),
        snapshot_digest: identity_input_digest("forgeplan-test"), implementation: "forgeplan-test-only".into(),
        profile_digest: String::new(), chunk_size: config.chunk_size,
        dimension: super::schema::EMBEDDING_DIM as usize,
        input_digests: BTreeMap::new(), vector_digests: BTreeMap::new(),
    };
    profile.profile_digest = identity_profile_digest(&profile);
    Ok(profile)
}

/// Resolve the configured model into a pinned profile without downloading it.
#[cfg(feature = "semantic-search")]
pub(crate) fn configured_identity_profile(
    workspace: &Path,
    existing_profile: bool,
) -> anyhow::Result<Option<IdentityV5EmbeddingProvenance>> {
    let config = crate::workspace::load_config(workspace)?;
    let Some(config) = config.embedding else {
        if existing_profile {
            anyhow::bail!("embedding model configuration is missing for the existing profile");
        }
        return Ok(None);
    };
    #[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
    if config.model == "forgeplan-test" {
        return Ok(Some(test_identity_profile(workspace)?));
    }
    let repo = crate::embed::resolve_repo(&config.model)
        .ok_or_else(|| anyhow::anyhow!("unsupported embedding model '{}'", config.model))?;
    let snapshot = crate::embed::tract_engine::find_snapshot(&crate::embed::resolve_cache_dir(), repo)
        .ok_or_else(|| anyhow::anyhow!("configured embedding model '{}' has no cached snapshot", config.model))?;
    let revision = snapshot.file_name().and_then(|name| name.to_str())
        .filter(|name| !name.is_empty()).ok_or_else(|| anyhow::anyhow!("embedding snapshot has no revision"))?;
    let dimension = crate::embed::embedding_dim(&config.model);
    if dimension != super::schema::EMBEDDING_DIM as usize {
        anyhow::bail!("configured embedding dimension {dimension} is incompatible with identity-v5");
    }
    let mut profile = IdentityV5EmbeddingProvenance {
        model: config.model, repository: repo.to_string(), snapshot_id: revision.to_string(),
        snapshot_digest: embedding_snapshot_digest(&snapshot)?, implementation: "forgeplan-tract-bge-m3-cls-l2-512-v1".into(),
        profile_digest: String::new(), chunk_size: config.chunk_size, dimension,
        input_digests: BTreeMap::new(), vector_digests: BTreeMap::new(),
    };
    profile.profile_digest = identity_profile_digest(&profile);
    Ok(Some(profile))
}

/// Persist a profile before any row may reference it. Existing row bindings
/// remain stale rather than becoming authoritative for a different model.
#[cfg(any(test, feature = "semantic-search"))]
pub(crate) fn publish_identity_semantic_profile(
    workspace: &Path,
    profile: &IdentityV5EmbeddingProvenance,
    lock: &crate::workspace::lock::WorkspaceLock,
) -> anyhow::Result<()> {
    #[cfg(windows)]
    anyhow::bail!("legacy semantic publication is unavailable on Windows");
    use crate::workspace::lock::WorkspaceReadPermit;
    lock.matches_workspace(workspace)?;
    validate_current_embedding_profile(workspace, profile)?;
    let lance = workspace.join("lance");
    let mut marker = read_identity_v5_marker(&lance)?
        .ok_or_else(|| anyhow::anyhow!("identity-v5 readiness is missing"))?;
    let semantic = marker.semantic.as_mut()
        .ok_or_else(|| anyhow::anyhow!("identity-v5 semantic protocol is not activated"))?;
    if semantic.active_profile_digest.as_deref() == Some(profile.profile_digest.as_str()) {
        return Ok(());
    }
    semantic.active_profile_digest = Some(profile.profile_digest.clone());
    marker.embeddings = Some(profile.clone());
    publish_identity_v5_marker(&lance, &marker)
}

#[cfg(feature = "semantic-search")]
pub(crate) fn embed_identity_batch(
    workspace: &Path,
    profile: &IdentityV5EmbeddingProvenance,
    inputs: &[String],
) -> anyhow::Result<Vec<Vec<f32>>> {
    validate_current_embedding_profile(workspace, profile)?;
    #[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
    let test = if profile.implementation == "forgeplan-test-only" {
        super::store::semantic_backend_for_test(workspace)?
    } else { None };
    #[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
    let vectors = if let Some(test) = test {
        inputs.iter().map(|input| test(input)).collect::<anyhow::Result<Vec<_>>>()?
    } else {
        embed_identity_batch_real(profile, inputs)?
    };
    #[cfg(not(any(test, all(feature = "test-helpers", debug_assertions))))]
    let vectors = embed_identity_batch_real(profile, inputs)?;
    for vector in &vectors {
        if vector.len() != profile.dimension || vector.iter().any(|value| !value.is_finite())
            || !vector.iter().any(|value| *value != 0.0)
        {
            anyhow::bail!("embedding backend returned an invalid vector");
        }
    }
    validate_current_embedding_profile(workspace, profile)?;
    Ok(vectors)
}

#[cfg(feature = "semantic-search")]
fn embed_identity_batch_real(
    profile: &IdentityV5EmbeddingProvenance,
    inputs: &[String],
) -> anyhow::Result<Vec<Vec<f32>>> {
    let snapshot = crate::embed::tract_engine::find_snapshot(&crate::embed::resolve_cache_dir(), &profile.repository)
        .ok_or_else(|| anyhow::anyhow!("embedding snapshot is unavailable"))?;
    let mut embedder = crate::embed::Embedder::from_snapshot(&snapshot, &profile.model)?;
    if embedder.dim() != profile.dimension {
        anyhow::bail!("embedding backend dimension differs from the pinned profile");
    }
    inputs.iter().map(|input| embedder.embed(input)).collect()
}

/// Embed a query only with the exact pinned migration profile.
#[cfg(feature = "semantic-search")]
pub(crate) fn embed_identity_query(
    workspace: &Path,
    profile: &IdentityV5EmbeddingProvenance,
    query: &str,
) -> anyhow::Result<Vec<f32>> {
    embed_identity_batch(workspace, profile, &[query.to_string()])?
        .pop().ok_or_else(|| anyhow::anyhow!("embedding backend returned no query vector"))
}

/// Complete repair embeddings in memory before any staged-table plan is
/// committed. A crash can replay the stored stage without re-running the model.
pub(crate) async fn prepare_semantic_rows_for_repair(
    workspace: &Path,
    proof: &crate::artifact::identity_repair::ValidatedLegacyRepair<'_>,
    had_legacy_vectors: bool,
    rows: &mut [Vec<RecordBatch>; 3],
    marker: &mut IdentityV5Marker,
) -> anyhow::Result<()> {
    proof.revalidate(workspace)?;
    let config = crate::workspace::load_config(workspace)?;
    if config.embedding.is_none() && marker.embeddings.is_none() {
        if had_legacy_vectors {
            anyhow::bail!("previous repair vectors require a configured model snapshot for recomputation");
        }
        for row in &rows[0] {
            let embedding = row.column_by_name("embedding")
                .ok_or_else(|| anyhow::anyhow!("repair artifact lacks embedding column"))?;
            if !embedding.is_null(0) {
                anyhow::bail!("repair row has an embedding but no configured model profile");
            }
        }
        return Ok(());
    }
    #[cfg(not(feature = "semantic-search"))]
    {
        let _ = (rows, marker);
        anyhow::bail!("configured semantic embedding backend is unavailable in this build");
    }
    #[cfg(feature = "semantic-search")]
    {
        let workspace_owned = workspace.to_path_buf();
        let had_profile = marker.embeddings.is_some();
        let profile = tokio::task::spawn_blocking(move || configured_identity_profile(&workspace_owned, had_profile))
            .await??.ok_or_else(|| anyhow::anyhow!("repair semantic profile is not configured"))?;
        let mut work = Vec::new();
        let mut inputs = Vec::new();
        for (index, row) in rows[0].iter().enumerate() {
            if row.num_rows() != 1 || row.schema() != super::schema::artifacts_schema() {
                anyhow::bail!("repair semantic artifact row has an invalid schema");
            }
            let record = super::store::extract_record(row, 0)
                .ok_or_else(|| anyhow::anyhow!("repair semantic artifact row is invalid"))?;
            let state = super::store::record_string(row, "semantic_state", 0);
            if state.as_deref().is_some_and(|value| !["ready", "pending", "running", "failed"].contains(&value)) {
                anyhow::bail!("repair artifact '{}' has an invalid semantic state", record.canonical_key);
            }
            let row_profile = super::store::record_string(row, "semantic_profile_digest", 0);
            if state.as_deref() == Some("ready") && row_profile.as_deref() == Some(profile.profile_digest.as_str()) {
                super::store::verify_row_local_semantic_provenance(row, 0, &record, &profile)?;
                continue;
            }
            work.push(index);
            inputs.push(record.embedding_text(profile.chunk_size));
        }
        proof.revalidate(workspace)?;
        let workspace_owned = workspace.to_path_buf();
        let pinned = profile.clone();
        let vectors = if inputs.is_empty() {
            Vec::new()
        } else {
            tokio::task::spawn_blocking(move || embed_identity_batch(&workspace_owned, &pinned, &inputs)).await??
        };
        if vectors.len() != work.len() {
            anyhow::bail!("repair semantic backend returned an unexpected vector count");
        }
        proof.revalidate(workspace)?;
        validate_current_embedding_profile(workspace, &profile)?;
        for (index, vector) in work.into_iter().zip(vectors) {
            let row = &mut rows[0][index];
            let record = super::store::extract_record(row, 0)
                .ok_or_else(|| anyhow::anyhow!("repair semantic artifact row disappeared"))?;
            let input_digest = identity_input_digest(&record.embedding_text(profile.chunk_size));
            let vector_digest = identity_vector_digest(&vector);
            let mut token = [0u8; 16];
            use rand::TryRngCore;
            rand::rngs::OsRng.try_fill_bytes(&mut token)
                .map_err(|error| anyhow::anyhow!("cannot create repair semantic job: {error}"))?;
            let job = hex_encode(&token);
            let schema = row.schema();
            let mut columns = row.columns().to_vec();
            columns[schema.index_of("embedding")?] = super::convert::make_embedding_col_from_option(Some(&vector));
            for (name, value) in [
                ("semantic_input_digest", input_digest.as_str()),
                ("semantic_profile_digest", profile.profile_digest.as_str()),
                ("semantic_vector_digest", vector_digest.as_str()),
                ("semantic_job_id", job.as_str()),
                ("semantic_state", "ready"),
            ] {
                columns[schema.index_of(name)?] = Arc::new(StringArray::from(vec![value]));
            }
            columns[schema.index_of("semantic_attempt_token")?] = Arc::new(StringArray::from(vec![None::<&str>]));
            *row = RecordBatch::try_new(schema, columns)?;
        }
        marker.embeddings = Some(profile.clone());
        marker.semantic = Some(IdentityV5SemanticProtocol {
            protocol: 1, active_profile_digest: Some(profile.profile_digest),
        });
        proof.revalidate(workspace)?;
        Ok(())
    }
}

/// Re-check a saved repair stage without rerunning the embedding model.
pub(crate) fn verify_semantic_rows_for_repair(
    workspace: &Path,
    proof: &crate::artifact::identity_repair::ValidatedLegacyRepair<'_>,
    had_legacy_vectors: bool,
    expected: &[Vec<RecordBatch>; 3],
    actual: &[Vec<RecordBatch>; 3],
    marker: &IdentityV5Marker,
) -> anyhow::Result<()> {
    use std::collections::BTreeMap;
    proof.revalidate(workspace)?;
    if marker.version != IDENTITY_V5_VERSION || !marker.completed
        || marker.artifacts_table != IDENTITY_V5_ARTIFACTS_TABLE
        || marker.relations_table != IDENTITY_V5_RELATIONS_TABLE
        || marker.evidence_table != IDENTITY_V5_EVIDENCE_TABLE
        || marker.semantic.as_ref().is_none_or(|semantic| semantic.protocol != 1
            || semantic.active_profile_digest.as_deref() != marker.embeddings.as_ref().map(|p| p.profile_digest.as_str()))
    {
        anyhow::bail!("repair marker has an invalid semantic protocol");
    }
    for table_index in 0..3 {
        let mut remaining = actual[table_index].clone();
        if remaining.len() != expected[table_index].len() {
            anyhow::bail!("repair staged table {table_index} has an unexpected row count");
        }
        for wanted in &expected[table_index] {
            let wanted_schema = wanted.schema();
            let position = remaining.iter().position(|candidate| {
                candidate.schema() == wanted_schema
                    && candidate.columns().iter().zip(wanted.columns()).zip(wanted_schema.fields())
                        .all(|((left, right), field)| {
                            (table_index == 0 && (field.name() == "embedding" || field.name().starts_with("semantic_")))
                                || left.to_data() == right.to_data()
                        })
            }).ok_or_else(|| anyhow::anyhow!("repair staged table {table_index} differs in non-semantic payload"))?;
            remaining.swap_remove(position);
        }
    }
    let mut keys = BTreeMap::new();
    for row in &actual[0] {
        if row.num_rows() != 1 || row.schema() != super::schema::artifacts_schema() {
            anyhow::bail!("repair staged artifact has an invalid schema");
        }
        let key = required_string(row, "canonical_key")?;
        if keys.insert(key.clone(), ()).is_some() {
            anyhow::bail!("repair staged artifacts repeat canonical key '{key}'");
        }
    }
    let config = crate::workspace::load_config(workspace)?;
    if config.embedding.is_none() && marker.embeddings.is_none() {
        if had_legacy_vectors {
            anyhow::bail!("previous repair vectors have no configured replacement profile");
        }
        for row in &actual[0] {
            let vector = row.column_by_name("embedding")
                .ok_or_else(|| anyhow::anyhow!("repair artifact lacks an embedding column"))?;
            if !vector.is_null(0) || super::store::record_string(row, "semantic_state", 0).as_deref() == Some("ready") {
                anyhow::bail!("repair row claims semantic readiness without a profile");
            }
        }
    } else {
        #[cfg(not(feature = "semantic-search"))]
        anyhow::bail!("configured semantic embedding backend is unavailable in this build");
        #[cfg(feature = "semantic-search")]
        {
            let profile = marker.embeddings.as_ref()
                .ok_or_else(|| anyhow::anyhow!("repair semantic profile is missing"))?;
            validate_current_embedding_profile(workspace, profile)?;
            let configured = configured_identity_profile(workspace, true)?
                .ok_or_else(|| anyhow::anyhow!("repair semantic profile is not configured"))?;
            if configured.profile_digest != profile.profile_digest {
                anyhow::bail!("repair semantic profile no longer matches configuration");
            }
            for row in &actual[0] {
                let record = super::store::extract_record(row, 0)
                    .ok_or_else(|| anyhow::anyhow!("repair staged artifact is invalid"))?;
                super::store::verify_row_local_semantic_provenance(row, 0, &record, profile)?;
            }
        }
    }
    proof.revalidate(workspace)?;
    Ok(())
}

pub(crate) async fn table_rows(db: &lancedb::connection::Connection, name: &str) -> anyhow::Result<Vec<RecordBatch>> {
    let table = db.open_table(name).execute().await?;
    let batches = crate::db::store::collect_batches(table.query().execute().await?).await?;
    Ok(batches.into_iter().flat_map(|batch| {
        (0..batch.num_rows()).map(move |row| batch.slice(row, 1))
    }).collect())
}

pub(crate) fn required_string(batch: &RecordBatch, column: &str) -> anyhow::Result<String> {
    crate::db::store::record_string(batch, column, 0)
        .ok_or_else(|| anyhow::anyhow!("legacy row has missing or invalid {column}"))
}

pub(crate) fn replace_string(batch: &RecordBatch, column: &str, value: &str) -> anyhow::Result<RecordBatch> {
    let index = batch.schema().index_of(column)?;
    let mut columns = batch.columns().to_vec();
    columns[index] = Arc::new(StringArray::from(vec![value]));
    Ok(RecordBatch::try_new(batch.schema(), columns)?)
}

pub(crate) fn strict_local_reference(candidates: &[crate::artifact::canonical_key::ArtifactCandidate], value: &str) -> anyhow::Result<String> {
    if value.contains('@') {
        return Ok(value.to_string());
    }
    crate::artifact::canonical_key::resolve_artifact(value, candidates)?
        .map(|resolved| resolved.identity.canonical_key)
        .ok_or_else(|| anyhow::anyhow!("legacy reference '{value}' has no matching document"))
}

/// Reconcile every old row before rebuilding a stage. Preserve its payload;
/// refuse non-null vectors whose input and model provenance cannot be proven.
async fn prepare_identity_v5_rows(
    db: &lancedb::connection::Connection,
    source: &IdentityV5Source,
    allow_vectors_for_recompute: bool,
) -> anyhow::Result<[Vec<RecordBatch>; 3]> {
    let candidates = identity_v5_candidates(source);
    let names = db.table_names().execute().await?;
    let legacy = ["artifacts", "relations", "evidence"];
    let present = legacy.iter().filter(|name| names.iter().any(|item| item.as_str() == **name)).count();
    if present != 0 && present != legacy.len() {
        anyhow::bail!("incomplete legacy index: all three v4 tables are required");
    }
    if present == 0 && !source.documents.is_empty() {
        anyhow::bail!("legacy index is missing; non-empty Markdown cannot be reconciled with its evidence and embeddings");
    }
    let mut artifacts = Vec::new();
    let mut mapped = std::collections::BTreeSet::new();
    if present != 0 {
        for row in table_rows(db, "artifacts").await? {
            let id = required_string(&row, "id")?;
            let document = source.documents.iter().filter(|doc| doc.display_id == id).collect::<Vec<_>>();
            if document.len() != 1 || !mapped.insert(id.clone()) {
                anyhow::bail!("legacy artifact '{id}' cannot be mapped uniquely to Markdown");
            }
            let document = document[0];
            for (name, expected) in [("kind", document.kind.as_str()), ("status", document.status.as_str()), ("title", document.title.as_str()), ("depth", document.depth.as_str())] {
                if required_string(&row, name)? != expected {
                    anyhow::bail!("legacy artifact '{id}' differs from Markdown in {name}");
                }
            }
            let old_body = crate::db::store::record_large_string(&row, "body", 0)
                .ok_or_else(|| anyhow::anyhow!("legacy artifact '{id}' has no valid body"))?;
            if old_body != document.body && old_body != document.full_text {
                anyhow::bail!("legacy artifact '{id}' differs from Markdown in body");
            }
            for (name, expected) in [("author", document.author.as_deref()), ("valid_until", document.valid_until.as_deref())] {
                if crate::db::store::record_string(&row, name, 0).as_deref() != expected {
                    anyhow::bail!("legacy artifact '{id}' differs from Markdown in {name}");
                }
            }
            if crate::db::convert::extract_tags(&row, 0) != document.tags {
                anyhow::bail!("legacy artifact '{id}' differs from Markdown in tags");
            }
            let embedding = row.column_by_name("embedding")
                .ok_or_else(|| anyhow::anyhow!("legacy artifact '{id}' has no embedding column"))?;
            if !embedding.is_null(0) && !allow_vectors_for_recompute {
                anyhow::bail!("legacy artifact '{id}' has an embedding without verifiable input, model profile, and dimension provenance; migration cannot publish until it is reconciled");
            }
            let target = super::schema::artifacts_schema();
            for field in row.schema().fields() {
                if target.field_with_name(field.name()).is_err() {
                    anyhow::bail!("legacy artifact '{id}' has unsupported column {}", field.name());
                }
            }
            if let Some(existing_key) = crate::db::store::record_string(&row, "canonical_key", 0) {
                if existing_key != document.canonical_key {
                    anyhow::bail!("legacy artifact '{id}' has conflicting canonical key '{existing_key}'");
                }
            }
            let mut columns = Vec::with_capacity(target.fields().len());
            for field in target.fields() {
                if field.name() == "canonical_key" {
                    columns.push(Arc::new(StringArray::from(vec![document.canonical_key.as_str()])) as Arc<dyn arrow_array::Array>);
                } else if field.name().starts_with("semantic_") {
                    columns.push(Arc::new(StringArray::from(vec![None::<&str>])) as Arc<dyn arrow_array::Array>);
                } else {
                    let column = row.column_by_name(field.name()).ok_or_else(|| anyhow::anyhow!("legacy artifacts missing {}", field.name()))?;
                    if column.data_type() != field.data_type() {
                        anyhow::bail!("legacy artifacts has incompatible {}", field.name());
                    }
                    columns.push(column.clone());
                }
            }
            let old_parent = crate::db::store::record_string(&row, "parent_epic", 0)
                .as_deref().map(|parent| resolve_reference(&candidates, parent)).transpose()?;
            let source_parent = document.parent_epic.as_deref()
                .map(|parent| resolve_reference(&candidates, parent)).transpose()?;
            if old_parent != source_parent {
                anyhow::bail!("legacy artifact '{id}' differs from Markdown in parent_epic");
            }
            let mut migrated = RecordBatch::try_new(target, columns)?;
            if let Some(parent) = old_parent {
                migrated = replace_string(&migrated, "parent_epic", &parent)?;
            }
            artifacts.push(migrated);
        }
    }
    for document in &source.documents {
        if !mapped.contains(&document.display_id) {
            let artifact = crate::db::store::NewArtifact {
                canonical_key: document.canonical_key.clone(), id: document.display_id.clone(),
                kind: document.kind.clone(), status: document.status.clone(), title: document.title.clone(),
                body: document.full_text.clone(), depth: document.depth.clone(), author: document.author.clone(),
                parent_epic: document.parent_epic.as_deref()
                    .map(|parent| resolve_reference(&candidates, parent)).transpose()?,
                valid_until: document.valid_until.clone(),
                tags: document.tags.clone(),
            };
            let row = crate::db::convert::artifact_to_batch(&artifact, &document.created_at)?;
            artifacts.push(replace_string(&row, "updated_at", &document.updated_at)?);
        }
    }
    let mut expected_edges = BTreeMap::<(String, String, String), usize>::new();
    for document in &source.documents {
        for (target, relation) in &document.links {
            let target = resolve_reference(&candidates, target)?;
            *expected_edges.entry((document.canonical_key.clone(), target, relation.clone())).or_default() += 1;
        }
    }
    let mut relations = Vec::new();
    if present != 0 {
        for row in table_rows(db, "relations").await? {
            let source_key = strict_local_reference(&candidates, &required_string(&row, "source_id")?)?;
            let target_key = resolve_reference(&candidates, &required_string(&row, "target_id")?)?;
            let edge = (source_key.clone(), target_key.clone(), required_string(&row, "relation_type")?);
            match expected_edges.get_mut(&edge) {
                Some(count) if *count > 0 => *count -= 1,
                _ => anyhow::bail!("legacy relation {edge:?} is absent from Markdown"),
            }
            let row = replace_string(&row, "source_id", &source_key)?;
            relations.push(replace_string(&row, "target_id", &target_key)?);
        }
    }
    for ((source_key, target, relation), count) in expected_edges {
        for _ in 0..count {
            let document = source.documents.iter().find(|doc| doc.canonical_key == source_key)
                .ok_or_else(|| anyhow::anyhow!("relation source disappeared"))?;
            relations.push(crate::db::convert::relation_to_batch(&source_key, &target, &relation, &document.created_at)?);
        }
    }
    let mut evidence = Vec::new();
    let mut evidence_ids = std::collections::BTreeSet::new();
    if present != 0 {
        for row in table_rows(db, "evidence").await? {
            let id = required_string(&row, "id")?;
            if !evidence_ids.insert(id.clone()) {
                anyhow::bail!("duplicate legacy evidence id '{id}'");
            }
            for (index, field) in row.schema().fields().iter().enumerate() {
                if !field.is_nullable() && row.column(index).is_null(0) {
                    anyhow::bail!("legacy evidence '{id}' has null {}", field.name());
                }
            }
            let key = strict_local_reference(&candidates, &required_string(&row, "artifact_id")?)?;
            evidence.push(replace_string(&row, "artifact_id", &key)?);
        }
    }
    Ok([artifacts, relations, evidence])
}

pub(crate) fn identity_v5_candidates(source: &IdentityV5Source) -> Vec<crate::artifact::canonical_key::ArtifactCandidate> {
    source.documents.iter().map(|document| crate::artifact::canonical_key::ArtifactCandidate {
        canonical_key: document.canonical_key.clone(),
        display_id: document.display_id.clone(),
        title: document.title.clone(),
        relative_path: document.relative_path.clone().into(),
    }).collect()
}

/// Preserve forward and cross-source references, but refuse ambiguous local ones.
pub(crate) fn resolve_reference(
    candidates: &[crate::artifact::canonical_key::ArtifactCandidate],
    target: &str,
) -> anyhow::Result<String> {
    if target.contains('@') {
        return Ok(target.to_string());
    }
    Ok(crate::artifact::canonical_key::resolve_structural_target(target, candidates)?
        .map(|resolved| resolved.identity.canonical_key)
        .unwrap_or_else(|| target.to_string()))
}

/// Read the staged artifacts table back and compare it with the Markdown it
/// was built from.
///
/// This is the payload round-trip: the published rows must be the rows the
/// source describes, field for field.
pub async fn verify_identity_v5_round_trip(
    lance_dir: &Path,
    source: &IdentityV5Source,
) -> anyhow::Result<()> {
    let db = lancedb::connect(lance_dir.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 LanceDB path"))?)
        .execute().await?;
    let expected = prepare_identity_v5_rows(&db, source, false).await?;
    verify_prepared_identity_v5_rows(&db, &expected).await
}

pub(crate) async fn verify_prepared_identity_v5_rows(
    db: &lancedb::connection::Connection,
    expected: &[Vec<RecordBatch>; 3],
) -> anyhow::Result<()> {
    for (name, expected_rows) in [IDENTITY_V5_ARTIFACTS_TABLE, IDENTITY_V5_RELATIONS_TABLE, IDENTITY_V5_EVIDENCE_TABLE]
        .into_iter().zip(expected.iter())
    {
        let mut actual = table_rows(&db, name).await?;
        if actual.len() != expected_rows.len() {
            anyhow::bail!("{name} row count does not match the reconciled source");
        }
        for expected_row in expected_rows {
            let position = actual.iter().position(|row| {
                row.schema() == expected_row.schema()
                    && row.columns().iter().zip(expected_row.columns()).all(|(left, right)| left.to_data() == right.to_data())
            }).ok_or_else(|| anyhow::anyhow!("{name} has a missing or changed staged row"))?;
            actual.swap_remove(position);
        }
    }
    Ok(())
}

/// How many rows the pre-migration artifacts table holds.
///
/// Only the `id` column is read: the count is what matters, not the bodies.
pub async fn legacy_artifact_row_count(lance_dir: &Path) -> anyhow::Result<usize> {
    let db = lancedb::connect(
        lance_dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("LanceDB path contains non-UTF-8 characters"))?,
    )
    .execute()
    .await?;
    let names = db.table_names().execute().await?;
    if !names.iter().any(|name| name == "artifacts") {
        return Ok(0);
    }
    let table = db.open_table("artifacts").execute().await?;
    let batches = crate::db::store::collect_batches(
        table
            .query()
            .select(Select::columns(&["id"]))
            .execute()
            .await?,
    )
    .await?;
    Ok(batches.iter().map(|batch| batch.num_rows()).sum())
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityDeleteIntent {
    version: u32,
    transaction_id: String,
    workspace: String,
    canonical_key: String,
    relative_path: String,
    recorded_at: String,
    source_digest: String,
    preimage: String,
    preimage_digest: String,
    digest: String,
}

fn identity_delete_intent_root(workspace: &Path) -> PathBuf {
    workspace.join("lance/identity-delete-intents")
}

fn identity_delete_intent_digest(intent: &IdentityDeleteIntent) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    let mut copy = intent.clone();
    copy.digest.clear();
    Ok(hex_encode(&Sha256::digest(serde_json::to_vec(&copy)?)))
}

fn read_identity_delete_intents(workspace: &Path) -> anyhow::Result<Vec<(PathBuf, IdentityDeleteIntent)>> {
    use sha2::{Digest, Sha256};
    let root = identity_delete_intent_root(workspace);
    let metadata = match std::fs::symlink_metadata(&root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        anyhow::bail!("identity delete intent root is not a real directory");
    }
    let canonical_workspace = std::fs::canonicalize(workspace)?;
    let mut entries = std::fs::read_dir(root)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut intents = Vec::with_capacity(entries.len());
    let mut keys = std::collections::BTreeSet::new();
    for entry in entries {
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            anyhow::bail!("identity delete intent is not a regular file: {}", path.display());
        }
        let intent: IdentityDeleteIntent = serde_json::from_slice(&std::fs::read(&path)?)?;
        let expected_name = format!("{}.json", intent.transaction_id);
        let document = parse_identity_v5_document(&intent.relative_path, &intent.preimage, &intent.recorded_at)?;
        let relative = Path::new(&intent.relative_path);
        if intent.version != 1 || !rebuild_transaction_id(&intent.transaction_id)
            || entry.file_name() != std::ffi::OsStr::new(&expected_name) || Path::new(&intent.workspace) != canonical_workspace
            || relative.is_absolute() || relative.components().any(|part| !matches!(part, std::path::Component::Normal(_)))
            || !lowercase_digest(&intent.source_digest) || !lowercase_digest(&intent.preimage_digest)
            || !lowercase_digest(&intent.digest) || identity_delete_intent_digest(&intent)? != intent.digest
            || hex_encode(&Sha256::digest(intent.preimage.as_bytes())) != intent.preimage_digest
            || document.canonical_key != intent.canonical_key || !keys.insert(intent.canonical_key.clone())
        {
            anyhow::bail!("identity delete intent is damaged or ambiguous: {}", path.display());
        }
        intents.push((path, intent));
    }
    Ok(intents)
}

pub(crate) fn has_identity_delete_intents(workspace: &Path) -> anyhow::Result<bool> {
    Ok(!read_identity_delete_intents(workspace)?.is_empty())
}

/// Durably bind a canonical preimage to one file-first legacy deletion. Call
/// only after selector preflight and before moving or unlinking that file.
pub async fn begin_identity_delete_intent(
    workspace: &Path,
    guard: &crate::workspace::lock::WorkspaceLock,
    canonical_key: &str,
    canonical_path: &Path,
) -> anyhow::Result<()> {
    #[cfg(windows)]
    anyhow::bail!("legacy identity deletion is unavailable on Windows");
    use crate::workspace::lock::WorkspaceReadPermit;
    use rand::TryRngCore;
    use sha2::{Digest, Sha256};
    use std::io::Write;
    guard.matches_workspace(workspace)?;
    let source = read_identity_v5_source(workspace).await?;
    if verify_identity_v5_source(&source).is_blocked() {
        anyhow::bail!("cannot record deletion from an ambiguous canonical capture");
    }
    let document = source.documents.iter().find(|document| document.canonical_key == canonical_key)
        .ok_or_else(|| anyhow::anyhow!("delete key '{canonical_key}' is absent from canonical capture"))?;
    let expected_path = workspace.join(&document.relative_path);
    if std::fs::canonicalize(canonical_path)? != std::fs::canonicalize(&expected_path)? {
        anyhow::bail!("delete path does not match canonical key '{canonical_key}'");
    }
    let bytes = std::fs::read(&expected_path)?;
    if bytes != document.full_text.as_bytes() {
        anyhow::bail!("delete preimage changed after canonical capture");
    }
    let store = crate::db::store::LanceStore::open(workspace).await?;
    store.ensure_identity_v5_writable(guard)?;
    let record = store.get_record(guard, canonical_key).await?
        .ok_or_else(|| anyhow::anyhow!("delete key '{canonical_key}' has no indexed row"))?;
    if record.canonical_key != canonical_key || record.id != document.display_id {
        anyhow::bail!("delete preimage does not match its indexed identity");
    }
    let drift = crate::projection::compare_projection(guard, &store, workspace, &record).await?;
    if !drift.unwritable.is_empty() {
        anyhow::bail!("delete preimage has unwritable projection drift: {:?}", drift.unwritable);
    }
    if read_identity_v5_source(workspace).await?.source_digest != source.source_digest {
        anyhow::bail!("delete source changed while recording its preimage");
    }
    let existing = read_identity_delete_intents(workspace)?;
    if let Some((_, intent)) = existing.iter().find(|(_, intent)| intent.canonical_key == canonical_key) {
        if intent.preimage.as_bytes() == bytes && intent.relative_path == document.relative_path {
            return Ok(());
        }
        anyhow::bail!("an unresolved delete intent already exists for '{canonical_key}'");
    }
    let mut token = [0u8; 16];
    rand::rngs::OsRng.try_fill_bytes(&mut token)
        .map_err(|error| anyhow::anyhow!("cannot mint delete intent transaction: {error}"))?;
    let transaction_id = hex_encode(&token);
    let mut intent = IdentityDeleteIntent {
        version: 1, transaction_id: transaction_id.clone(),
        workspace: std::fs::canonicalize(workspace)?.to_str()
            .ok_or_else(|| anyhow::anyhow!("non-UTF-8 workspace path"))?.to_string(),
        canonical_key: canonical_key.to_string(), relative_path: document.relative_path.clone(),
        recorded_at: chrono::Utc::now().to_rfc3339(),
        source_digest: source.source_digest, preimage: document.full_text.clone(),
        preimage_digest: hex_encode(&Sha256::digest(&bytes)), digest: String::new(),
    };
    intent.digest = identity_delete_intent_digest(&intent)?;
    let root = identity_delete_intent_root(workspace);
    std::fs::create_dir_all(&root)?;
    let root_meta = std::fs::symlink_metadata(&root)?;
    if !root_meta.is_dir() || root_meta.file_type().is_symlink() {
        anyhow::bail!("identity delete intent root is not a real directory");
    }
    let path = root.join(format!("{transaction_id}.json"));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(&serde_json::to_vec(&intent)?)?;
    file.sync_all()?;
    std::fs::File::open(&root)?.sync_all()?;
    std::fs::File::open(workspace.join("lance"))?.sync_all()?;
    Ok(())
}

/// Refuse deletion of an orphan unless an earlier operation durably recorded
/// its exact canonical preimage before moving the source file.
pub async fn require_identity_delete_intent(
    workspace: &Path,
    guard: &crate::workspace::lock::WorkspaceLock,
    canonical_key: &str,
) -> anyhow::Result<()> {
    #[cfg(windows)]
    anyhow::bail!("legacy identity deletion is unavailable on Windows");
    use crate::workspace::lock::WorkspaceReadPermit;
    guard.matches_workspace(workspace)?;
    let source = read_identity_v5_source(workspace).await?;
    if verify_identity_v5_source(&source).is_blocked()
        || source.documents.iter().any(|document| document.canonical_key == canonical_key)
    {
        anyhow::bail!("delete intent requires a complete capture with the key absent");
    }
    let intents = read_identity_delete_intents(workspace)?;
    let intent = intents.iter().find(|(_, intent)| intent.canonical_key == canonical_key)
        .map(|(_, intent)| intent)
        .ok_or_else(|| anyhow::anyhow!("canonical key '{canonical_key}' has no durable delete intent"))?;
    let before = parse_identity_v5_document(&intent.relative_path, &intent.preimage, &intent.recorded_at)?;
    let store = crate::db::store::LanceStore::open(workspace).await?;
    store.ensure_identity_v5_writable(guard)?;
    let record = store.get_record(guard, canonical_key).await?
        .ok_or_else(|| anyhow::anyhow!("delete intent has no matching indexed row"))?;
    fn normalize(text: &str) -> &str { text.strip_suffix('\n').unwrap_or(text) }
    if record.canonical_key != before.canonical_key || record.id != before.display_id
        || record.kind != before.kind || record.title != before.title || record.status != before.status
        || record.depth != before.depth || record.author != before.author
        || record.valid_until != before.valid_until || record.tags != before.tags
        || (normalize(&record.body) != normalize(&before.body)
            && normalize(&record.body) != normalize(&before.full_text))
    {
        anyhow::bail!("delete intent preimage differs from indexed artifact '{canonical_key}'");
    }
    Ok(())
}

/// Build the exact replacement rows without modifying any published table.
/// Existing evidence is retained byte-for-byte and an unresolvable attachment
/// blocks the rebuild rather than being silently discarded.
async fn prepare_live_identity_rebuild_rows(
    workspace: &Path,
    db: &lancedb::connection::Connection,
    source: &IdentityV5Source,
) -> anyhow::Result<[Vec<RecordBatch>; 3]> {
    use std::collections::{BTreeMap, BTreeSet};
    fn normalize(text: &str) -> &str { text.strip_suffix('\n').unwrap_or(text) }
    let marker = read_identity_v5_marker(&workspace.join("lance"))?
        .ok_or_else(|| anyhow::anyhow!("semantic rebuild requires a published identity index"))?;
    if verify_identity_v5_source(source).is_blocked() {
        anyhow::bail!("semantic rebuild source has conflicting identities");
    }
    let candidates = identity_v5_candidates(source);
    let intents = read_identity_delete_intents(workspace)?;
    let mut documents: BTreeMap<_, _> = source.documents.iter()
        .map(|document| (document.canonical_key.as_str(), document)).collect();
    if documents.len() != source.documents.len() {
        anyhow::bail!("semantic rebuild source repeats a canonical key");
    }
    let mut artifacts = Vec::new();
    let mut removed = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for row in table_rows(db, &marker.artifacts_table).await? {
        if row.schema() != super::schema::artifacts_schema() {
            anyhow::bail!("semantic rebuild artifact schema differs from the published schema");
        }
        let key = required_string(&row, "canonical_key")?;
        if !seen.insert(key.clone()) {
            anyhow::bail!("semantic rebuild has duplicate artifact key '{key}'");
        }
        let record = super::store::extract_record(&row, 0)
            .ok_or_else(|| anyhow::anyhow!("semantic rebuild has a malformed artifact row"))?;
        if let Some(document) = documents.remove(key.as_str()) {
            let expected_parent = document.parent_epic.as_deref()
                .map(|parent| resolve_reference(&candidates, parent)).transpose()?;
            if record.id != document.display_id || record.kind != document.kind
                || record.title != document.title || record.status != document.status
                || record.depth != document.depth || record.author != document.author
                || record.valid_until != document.valid_until || record.tags != document.tags
                || record.parent_epic != expected_parent
                || (normalize(&record.body) != normalize(&document.body)
                    && normalize(&record.body) != normalize(&document.full_text))
            {
                anyhow::bail!("semantic rebuild row '{key}' differs from canonical source");
            }
            artifacts.push(row);
        } else {
            let intent = intents.iter().find(|(_, intent)| intent.canonical_key == key)
                .map(|(_, intent)| intent)
                .ok_or_else(|| anyhow::anyhow!("semantic rebuild refuses orphan '{key}' without a durable delete intent"))?;
            let before = parse_identity_v5_document(&intent.relative_path, &intent.preimage, &intent.recorded_at)?;
            if record.id != before.display_id || record.kind != before.kind
                || record.title != before.title || record.status != before.status
                || record.depth != before.depth || record.author != before.author
                || record.valid_until != before.valid_until || record.tags != before.tags
                || (normalize(&record.body) != normalize(&before.body)
                    && normalize(&record.body) != normalize(&before.full_text))
            {
                anyhow::bail!("semantic rebuild delete intent for '{key}' does not match its indexed preimage");
            }
            let kind: crate::artifact::types::ArtifactKind = record.kind.parse()?;
            let dir = workspace.join(kind.dir_name());
            let metadata = std::fs::symlink_metadata(&dir)
                .map_err(|error| anyhow::anyhow!("cannot prove complete capture of {}: {error}", dir.display()))?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                anyhow::bail!("cannot prove complete capture of {}", dir.display());
            }
            removed.insert(key);
        }
    }
    for document in documents.into_values() {
        let artifact = crate::db::store::NewArtifact {
            canonical_key: document.canonical_key.clone(), id: document.display_id.clone(),
            kind: document.kind.clone(), status: document.status.clone(), title: document.title.clone(),
            body: document.full_text.clone(), depth: document.depth.clone(), author: document.author.clone(),
            parent_epic: document.parent_epic.as_deref()
                .map(|parent| resolve_reference(&candidates, parent)).transpose()?,
            valid_until: document.valid_until.clone(), tags: document.tags.clone(),
        };
        let row = super::convert::artifact_to_batch(&artifact, &document.created_at)?;
        artifacts.push(replace_string(&row, "updated_at", &document.updated_at)?);
    }
    let mut expected = BTreeMap::new();
    for document in &source.documents {
        for (target, relation) in &document.links {
            let edge = (document.canonical_key.clone(), resolve_reference(&candidates, target)?, relation.clone());
            if expected.insert(edge, document.created_at.clone()).is_some() {
                anyhow::bail!("semantic rebuild source has a duplicate relation");
            }
        }
    }
    let mut relations = Vec::new();
    let mut relation_keys = BTreeSet::new();
    for row in table_rows(db, &marker.relations_table).await? {
        if row.schema() != super::schema::relations_schema() {
            anyhow::bail!("semantic rebuild relation schema differs from the published schema");
        }
        let edge = (required_string(&row, "source_id")?, required_string(&row, "target_id")?,
            required_string(&row, "relation_type")?);
        let _ = required_string(&row, "created_at")?;
        if !relation_keys.insert(edge.clone()) {
            anyhow::bail!("semantic rebuild has duplicate relation {edge:?}");
        }
        if expected.remove(&edge).is_some() {
            relations.push(row);
        } else if !removed.contains(&edge.0) && !removed.contains(&edge.1) {
            anyhow::bail!("semantic rebuild relation {edge:?} differs from canonical source");
        }
    }
    for ((source, target, relation), created_at) in expected {
        relations.push(super::convert::relation_to_batch(&source, &target, &relation, &created_at)?);
    }
    let mut evidence_ids = BTreeSet::new();
    let mut evidence = Vec::new();
    for row in table_rows(db, &marker.evidence_table).await? {
        if row.schema() != super::schema::evidence_schema() {
            anyhow::bail!("semantic rebuild evidence schema differs from the published schema");
        }
        let id = required_string(&row, "id")?;
        let key = required_string(&row, "artifact_id")?;
        if !evidence_ids.insert(id.clone()) {
            anyhow::bail!("semantic rebuild has duplicate evidence id '{id}'");
        }
        for (index, field) in row.schema().fields().iter().enumerate() {
            if !field.is_nullable() && row.column(index).is_null(0) {
                anyhow::bail!("semantic rebuild evidence '{id}' has a missing {}", field.name());
            }
        }
        if !source.documents.iter().any(|document| document.canonical_key == key) {
            anyhow::bail!("semantic rebuild cannot map evidence '{id}' to a current canonical key");
        }
        evidence.push(row);
    }
    Ok([artifacts, relations, evidence])
}

/// Durable owner record for a fixed-name structural rebuild. Its digest binds
/// the complete plan and is immutable across recovery attempts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySemanticRebuildManifest {
    version: u32,
    transaction_id: String,
    workspace: String,
    source_digest: String,
    old_marker_digest: String,
    new_marker: IdentityV5Marker,
    tables: Vec<LegacySemanticRebuildTable>,
    intent_digests: Vec<String>,
    digest: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySemanticRebuildTable {
    name: String,
    before_digest: String,
    after_digest: String,
}

fn legacy_semantic_rebuild_root(workspace: &Path) -> PathBuf {
    workspace.join("lance/legacy-semantic-rebuild")
}

fn lowercase_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn rebuild_transaction_id(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn rebuild_manifest_digest(manifest: &LegacySemanticRebuildManifest) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    let mut copy = manifest.clone();
    copy.digest.clear();
    Ok(hex_encode(&Sha256::digest(serde_json::to_vec(&copy)?)))
}

/// Hash all table contents, including relative names and lengths. Symlinks and
/// non-regular files are never followed or silently skipped.
pub(crate) fn semantic_rebuild_tree_digest(root: &Path) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    fn visit(path: &Path, relative: &str, hash: &mut Sha256) -> anyhow::Result<()> {
        let metadata = std::fs::symlink_metadata(path)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
            if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                anyhow::bail!("semantic rebuild table contains a reparse point");
            }
        }
        if metadata.file_type().is_symlink() {
            anyhow::bail!("semantic rebuild table contains a symlink");
        }
        hash.update((relative.len() as u64).to_le_bytes());
        hash.update(relative.as_bytes());
        if metadata.is_dir() {
            hash.update(b"D");
            let mut entries = std::fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let name = entry.file_name().into_string()
                    .map_err(|_| anyhow::anyhow!("semantic rebuild table has a non-UTF-8 name"))?;
                let child = if relative.is_empty() { name } else { format!("{relative}/{name}") };
                visit(&entry.path(), &child, hash)?;
            }
        } else if metadata.is_file() {
            hash.update(b"F");
            hash.update(metadata.len().to_le_bytes());
            let mut file = std::fs::File::open(path)?;
            let mut buf = [0u8; 65536];
            loop {
                let count = file.read(&mut buf)?;
                if count == 0 { break; }
                hash.update(&buf[..count]);
            }
        } else {
            anyhow::bail!("semantic rebuild table contains a non-regular entry");
        }
        Ok(())
    }
    let mut hash = Sha256::new();
    visit(root, "", &mut hash)?;
    Ok(hex_encode(&hash.finalize()))
}

fn optional_rebuild_tree_digest(path: &Path) -> anyhow::Result<Option<String>> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(Some(semantic_rebuild_tree_digest(path)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Atomically rename without replacing an existing target. Platforms without a
/// kernel no-replace primitive fail before any rename.
pub(crate) fn rename_identity_noreplace(source: &Path, destination: &Path) -> anyhow::Result<()> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::ffi::OsStrExt;
        let source = std::ffi::CString::new(source.as_os_str().as_bytes())?;
        let destination = std::ffi::CString::new(destination.as_os_str().as_bytes())?;
        #[cfg(target_os = "macos")]
        // SAFETY: both C strings remain alive and NUL-terminated for the call.
        let result = unsafe { libc::renameatx_np(libc::AT_FDCWD, source.as_ptr(), libc::AT_FDCWD, destination.as_ptr(), libc::RENAME_EXCL) };
        #[cfg(target_os = "linux")]
        // SAFETY: the syscall receives the documented file descriptors, C strings and flag.
        let result = unsafe { libc::syscall(libc::SYS_renameat2, libc::AT_FDCWD, source.as_ptr(), libc::AT_FDCWD, destination.as_ptr(), libc::RENAME_NOREPLACE) };
        if result != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        crate::constellation::durability::move_noreplace_write_through(source, destination)?;
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        let _ = (source, destination);
        anyhow::bail!("atomic no-replace identity rename is unavailable on this platform")
    }
}

/// Hash-bound fixed-name swap. The caller owns the durable journal and supplies
/// an owner-specific admission check; this helper never creates capabilities
/// or publishes a readiness marker.
pub(crate) struct FixedTableSwapEntry {
    pub(crate) name: String,
    pub(crate) before_digest: Option<String>,
    pub(crate) after_digest: String,
}

pub(crate) fn complete_fixed_table_swap<F>(
    workspace: &Path,
    staged: &Path,
    backup: &Path,
    entries: &[FixedTableSwapEntry],
    revalidate: F,
) -> anyhow::Result<()>
where
    F: Fn() -> anyhow::Result<()>,
{
    #[cfg(windows)]
    anyhow::bail!("legacy fixed-table swap on Windows requires durable replay acknowledgement");
    revalidate()?;
    let names = [IDENTITY_V5_ARTIFACTS_TABLE, IDENTITY_V5_RELATIONS_TABLE, IDENTITY_V5_EVIDENCE_TABLE];
    if entries.len() != names.len() || entries.iter().zip(names).any(|(entry, name)| {
        entry.name != name || !lowercase_digest(&entry.after_digest)
            || entry.before_digest.as_ref().is_some_and(|digest| !lowercase_digest(digest))
    }) {
        anyhow::bail!("fixed-table swap plan has invalid table names or hashes");
    }
    let lance = workspace.join("lance");
    #[cfg(windows)]
    for path in [&lance, staged, backup] {
        let (_, barrier, diagnostic) = crate::constellation::durability::windows_ntfs_primitive_barrier(path);
        if barrier.is_none() {
            anyhow::bail!("fixed-table swap durability unavailable: {diagnostic:?}");
        }
    }
    for path in [&lance, staged, backup] {
        let metadata = std::fs::symlink_metadata(path)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
            if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                anyhow::bail!("fixed-table swap directory is a reparse point: {}", path.display());
            }
        }
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            anyhow::bail!("fixed-table swap directory is invalid: {}", path.display());
        }
    }
    for entry in entries {
        revalidate()?;
        let filename = format!("{}.lance", entry.name);
        let live = lance.join(&filename);
        let stage = staged.join(&filename);
        let saved = backup.join(&filename);
        let current = optional_rebuild_tree_digest(&live)?;
        let incoming = optional_rebuild_tree_digest(&stage)?;
        let previous = optional_rebuild_tree_digest(&saved)?;
        let before = current.as_deref() == entry.before_digest.as_deref()
            && incoming.as_deref() == Some(entry.after_digest.as_str()) && previous.is_none();
        let between = entry.before_digest.is_some() && current.is_none()
            && incoming.as_deref() == Some(entry.after_digest.as_str())
            && previous.as_deref() == entry.before_digest.as_deref();
        let after = current.as_deref() == Some(entry.after_digest.as_str()) && incoming.is_none()
            && previous.as_deref() == entry.before_digest.as_deref();
        if !(before || between || after) {
            anyhow::bail!("fixed-table swap '{}' is in an unrecognized state", entry.name);
        }
        if before && entry.before_digest.is_some() {
            revalidate()?;
            rename_identity_noreplace(&live, &saved)?;
            #[cfg(not(windows))]
            {
            std::fs::File::open(&lance)?.sync_all()?;
            std::fs::File::open(backup)?.sync_all()?;
            }
            revalidate()?;
        }
        if before || between {
            revalidate()?;
            rename_identity_noreplace(&stage, &live)?;
            #[cfg(not(windows))]
            {
            std::fs::File::open(staged)?.sync_all()?;
            std::fs::File::open(&lance)?.sync_all()?;
            }
            revalidate()?;
        }
    }
    revalidate()
}

/// A pending, malformed or unreadable rebuild blocks ordinary locks. The
/// owner path is the only way to recover an interrupted transaction.
pub(crate) fn check_legacy_semantic_rebuild(workspace: &Path) -> anyhow::Result<()> {
    let root = legacy_semantic_rebuild_root(workspace);
    let metadata = match std::fs::symlink_metadata(&root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        anyhow::bail!("semantic rebuild journal root is not a real directory");
    }
    let mut entries = std::fs::read_dir(&root)?;
    if let Some(entry) = entries.next() {
        let entry = entry?;
        anyhow::bail!("semantic rebuild requires recovery: {}", entry.path().display());
    }
    Ok(())
}

/// Read-only routing hint; owner acquisition validates the transaction again
/// after taking the exclusive lock.
pub(crate) fn pending_legacy_semantic_rebuild(workspace: &Path) -> anyhow::Result<Option<String>> {
    let root = legacy_semantic_rebuild_root(workspace);
    let metadata = match std::fs::symlink_metadata(&root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        anyhow::bail!("semantic rebuild journal root is not a real directory");
    }
    let mut entries = std::fs::read_dir(&root)?;
    let Some(entry) = entries.next() else { return Ok(None) };
    let transaction = entry?.file_name().into_string()
        .map_err(|_| anyhow::anyhow!("semantic rebuild transaction name is not UTF-8"))?;
    if entries.next().transpose()?.is_some() || !rebuild_transaction_id(&transaction) {
        anyhow::bail!("semantic rebuild journal is ambiguous or damaged");
    }
    Ok(Some(transaction))
}

/// Validate the immutable plan before issuing the transaction-bound owner
/// capability. Table contents and the source are checked again by recovery.
pub(crate) fn validate_legacy_semantic_rebuild(workspace: &Path, transaction: &str) -> anyhow::Result<String> {
    if !rebuild_transaction_id(transaction) {
        anyhow::bail!("invalid semantic rebuild transaction id");
    }
    let root = legacy_semantic_rebuild_root(workspace);
    let root_meta = std::fs::symlink_metadata(&root)?;
    if !root_meta.is_dir() || root_meta.file_type().is_symlink() {
        anyhow::bail!("semantic rebuild journal root is not a real directory");
    }
    let mut entries = std::fs::read_dir(&root)?;
    let entry = entries.next().ok_or_else(|| anyhow::anyhow!("semantic rebuild journal is missing"))??;
    if entries.next().transpose()?.is_some() || entry.file_name() != transaction {
        anyhow::bail!("semantic rebuild journal is ambiguous");
    }
    let dir = entry.path();
    let metadata = std::fs::symlink_metadata(&dir)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        anyhow::bail!("semantic rebuild transaction is not a real directory");
    }
    let path = dir.join("manifest.json");
    let metadata = std::fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 1_048_576 {
        anyhow::bail!("semantic rebuild manifest is not a valid regular file");
    }
    let manifest: LegacySemanticRebuildManifest = serde_json::from_slice(&std::fs::read(path)?)?;
    let canonical = std::fs::canonicalize(workspace)?;
    let expected_names = [IDENTITY_V5_ARTIFACTS_TABLE, IDENTITY_V5_RELATIONS_TABLE, IDENTITY_V5_EVIDENCE_TABLE];
    if manifest.version != 1 || manifest.transaction_id != transaction
        || Path::new(&manifest.workspace) != canonical
        || !lowercase_digest(&manifest.source_digest) || !lowercase_digest(&manifest.old_marker_digest)
        || manifest.new_marker.source_digest != manifest.source_digest
        || manifest.new_marker.version != IDENTITY_V5_VERSION || !manifest.new_marker.completed
        || manifest.new_marker.artifacts_table != expected_names[0]
        || manifest.new_marker.relations_table != expected_names[1]
        || manifest.new_marker.evidence_table != expected_names[2]
        || manifest.tables.len() != expected_names.len()
        || manifest.tables.iter().zip(expected_names).any(|(table, name)| {
            table.name != name || !lowercase_digest(&table.before_digest) || !lowercase_digest(&table.after_digest)
        })
        || manifest.intent_digests.iter().any(|digest| !lowercase_digest(digest))
        || !lowercase_digest(&manifest.digest)
        || rebuild_manifest_digest(&manifest)? != manifest.digest
    {
        anyhow::bail!("semantic rebuild manifest is damaged or belongs to a different transaction");
    }
    use sha2::{Digest, Sha256};
    let lance = workspace.join("lance");
    let marker_path = identity_v5_marker_path(&lance);
    let marker_metadata = std::fs::symlink_metadata(&marker_path)?;
    if !marker_metadata.is_file() || marker_metadata.file_type().is_symlink() {
        anyhow::bail!("semantic rebuild readiness marker is not a regular file");
    }
    let marker_bytes = std::fs::read(&marker_path)?;
    let marker_digest = hex_encode(&Sha256::digest(&marker_bytes));
    let new_marker_digest = hex_encode(&Sha256::digest(serde_json::to_vec_pretty(&manifest.new_marker)?));
    if marker_digest != manifest.old_marker_digest && marker_digest != new_marker_digest {
        anyhow::bail!("semantic rebuild readiness marker does not match its transaction");
    }
    for part in ["staged", "backup"] {
        let metadata = std::fs::symlink_metadata(dir.join(part))?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            anyhow::bail!("semantic rebuild {part} is not a real directory");
        }
    }
    let mut all_published = true;
    for table in &manifest.tables {
        let filename = format!("{}.lance", table.name);
        let live = optional_rebuild_tree_digest(&lance.join(&filename))?;
        let staged = optional_rebuild_tree_digest(&dir.join("staged").join(&filename))?;
        let backup = optional_rebuild_tree_digest(&dir.join("backup").join(&filename))?;
        let before = live.as_deref() == Some(table.before_digest.as_str())
            && staged.as_deref() == Some(table.after_digest.as_str()) && backup.is_none();
        let between = live.is_none() && staged.as_deref() == Some(table.after_digest.as_str())
            && backup.as_deref() == Some(table.before_digest.as_str());
        let after = live.as_deref() == Some(table.after_digest.as_str()) && staged.is_none()
            && backup.as_deref() == Some(table.before_digest.as_str());
        if !(before || between || after) {
            anyhow::bail!("semantic rebuild table '{}' is in an unrecognized state", table.name);
        }
        all_published &= after;
    }
    if marker_digest == new_marker_digest && new_marker_digest != manifest.old_marker_digest && !all_published {
        anyhow::bail!("semantic rebuild readiness marker is ahead of its tables");
    }
    Ok(manifest.digest)
}

pub(crate) fn sync_rebuild_tree(path: &Path) -> anyhow::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            anyhow::bail!("semantic rebuild stage contains a reparse point");
        }
    }
    if metadata.file_type().is_symlink() {
        anyhow::bail!("semantic rebuild stage contains a symlink");
    }
    if metadata.is_dir() {
        for entry in std::fs::read_dir(path)? {
            sync_rebuild_tree(&entry?.path())?;
        }
    } else if !metadata.is_file() {
        anyhow::bail!("semantic rebuild stage contains a non-regular entry");
    }
    #[cfg(not(windows))]
    std::fs::File::open(path)?.sync_all()?;
    #[cfg(windows)]
    if metadata.is_file() {
        let parent = path.parent().ok_or_else(|| anyhow::anyhow!("stage file has no parent"))?;
        let (_, barrier, diagnostic) = crate::constellation::durability::windows_ntfs_primitive_barrier(parent);
        let barrier = barrier.ok_or_else(|| anyhow::anyhow!("stage durability unavailable: {diagnostic:?}"))?;
        barrier.sync_file(path)?;
    }
    Ok(())
}

/// Build and prove a complete replacement set, then publish only an immutable
/// pending transaction. No currently published table is changed by this step.
pub(crate) async fn prepare_legacy_semantic_rebuild(
    workspace: &Path,
    guard: &crate::workspace::lock::WorkspaceLock,
    db: &lancedb::connection::Connection,
    source: &IdentityV5Source,
) -> anyhow::Result<String> {
    use crate::workspace::lock::WorkspaceReadPermit;
    use rand::TryRngCore;
    use sha2::{Digest, Sha256};
    use std::io::Write;
    guard.matches_workspace(workspace)?;
    if !cfg!(any(target_os = "macos", target_os = "linux")) {
        anyhow::bail!("fixed-name semantic rebuild requires atomic no-replace rename");
    }
    check_legacy_semantic_rebuild(workspace)?;
    let lance = workspace.join("lance");
    let marker_path = identity_v5_marker_path(&lance);
    let marker_bytes = std::fs::read(&marker_path)?;
    let mut marker = read_identity_v5_marker(&lance)?
        .ok_or_else(|| anyhow::anyhow!("semantic rebuild readiness marker is missing"))?;
    let rows = prepare_live_identity_rebuild_rows(workspace, db, source).await?;
    let mut token = [0u8; 16];
    rand::rngs::OsRng.try_fill_bytes(&mut token)
        .map_err(|error| anyhow::anyhow!("cannot create semantic rebuild transaction: {error}"))?;
    let transaction = hex_encode(&token);
    let temporary = lance.join(format!(".semantic-rebuild-{transaction}"));
    std::fs::create_dir(&temporary)?;
    struct UnpublishedStage(PathBuf);
    impl Drop for UnpublishedStage {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }
    let temporary_guard = UnpublishedStage(temporary.clone());
    std::fs::create_dir(temporary.join("staged"))?;
    std::fs::create_dir(temporary.join("backup"))?;
    let stage_path = temporary.join("staged");
    write_prepared_identity_v5_rows(&stage_path, &rows).await?;
    let stage_db = lancedb::connect(stage_path.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 stage path"))?)
        .execute().await?;
    verify_prepared_identity_v5_rows(&stage_db, &rows).await?;
    drop(stage_db);
    sync_rebuild_tree(&stage_path)?;
    if read_identity_v5_source(workspace).await?.source_digest != source.source_digest
        || std::fs::read(&marker_path)? != marker_bytes
    {
        anyhow::bail!("canonical source or marker changed during semantic staging");
    }
    let mut tables = Vec::with_capacity(3);
    for name in [IDENTITY_V5_ARTIFACTS_TABLE, IDENTITY_V5_RELATIONS_TABLE, IDENTITY_V5_EVIDENCE_TABLE] {
        let filename = format!("{name}.lance");
        tables.push(LegacySemanticRebuildTable {
            name: name.to_string(),
            before_digest: semantic_rebuild_tree_digest(&lance.join(&filename))?,
            after_digest: semantic_rebuild_tree_digest(&stage_path.join(&filename))?,
        });
    }
    marker.source_digest = source.source_digest.clone();
    marker.document_count = source.documents.len();
    let mut manifest = LegacySemanticRebuildManifest {
        version: 1, transaction_id: transaction.clone(),
        workspace: std::fs::canonicalize(workspace)?.to_str()
            .ok_or_else(|| anyhow::anyhow!("non-UTF-8 workspace path"))?.to_string(),
        source_digest: source.source_digest.clone(), old_marker_digest: hex_encode(&Sha256::digest(&marker_bytes)),
        new_marker: marker, tables,
        intent_digests: read_identity_delete_intents(workspace)?.into_iter().map(|(_, intent)| intent.digest).collect(),
        digest: String::new(),
    };
    manifest.digest = rebuild_manifest_digest(&manifest)?;
    let mut manifest_file = std::fs::OpenOptions::new().write(true).create_new(true)
        .open(temporary.join("manifest.json"))?;
    manifest_file.write_all(&serde_json::to_vec(&manifest)?)?;
    manifest_file.sync_all()?;
    std::fs::File::open(&temporary)?.sync_all()?;
    let root = legacy_semantic_rebuild_root(workspace);
    match std::fs::create_dir(&root) {
        Ok(()) => std::fs::File::open(&lance)?.sync_all()?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => check_legacy_semantic_rebuild(workspace)?,
        Err(error) => return Err(error.into()),
    }
    rename_identity_noreplace(&temporary, &root.join(&transaction))?;
    std::fs::File::open(&root)?.sync_all()?;
    drop(temporary_guard);
    Ok(transaction)
}

/// Finish an interrupted fixed-name swap under the transaction-bound owner
/// capability. Every intermediate filesystem state is checked against the
/// immutable pre/post hashes before making the next durable transition.
pub(crate) async fn resume_legacy_semantic_rebuild(
    workspace: &Path,
    guard: &crate::workspace::lock::WorkspaceLock,
    transaction: &str,
) -> anyhow::Result<()> {
    #[cfg(windows)]
    anyhow::bail!("legacy identity rebuild is unavailable on Windows");
    let digest = validate_legacy_semantic_rebuild(workspace, transaction)?;
    if !guard.is_legacy_semantic_rebuild_guard_for(workspace, transaction, &digest) {
        anyhow::bail!("semantic rebuild requires its transaction-bound owner lock");
    }
    let root = legacy_semantic_rebuild_root(workspace);
    let dir = root.join(transaction);
    let manifest: LegacySemanticRebuildManifest =
        serde_json::from_slice(&std::fs::read(dir.join("manifest.json"))?)?;
    if read_identity_v5_source(workspace).await?.source_digest != manifest.source_digest {
        anyhow::bail!("canonical source changed while semantic rebuild was pending");
    }
    let lance = workspace.join("lance");
    let entries = manifest.tables.iter().map(|table| FixedTableSwapEntry {
        name: table.name.clone(),
        before_digest: Some(table.before_digest.clone()),
        after_digest: table.after_digest.clone(),
    }).collect::<Vec<_>>();
    complete_fixed_table_swap(workspace, &dir.join("staged"), &dir.join("backup"), &entries, || {
        if !guard.is_legacy_semantic_rebuild_guard_for(workspace, transaction, &digest)
            || validate_legacy_semantic_rebuild(workspace, transaction)? != digest
        {
            anyhow::bail!("semantic rebuild owner or journal changed during recovery");
        }
        Ok(())
    })?;
    let source = read_identity_v5_source(workspace).await?;
    if source.source_digest != manifest.source_digest {
        anyhow::bail!("canonical source changed before semantic rebuild verification");
    }
    let db = lancedb::connect(lance.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 Lance path"))?)
        .execute().await?;
    let expected = prepare_live_identity_rebuild_rows(workspace, &db, &source).await?;
    verify_prepared_identity_v5_rows(&db, &expected).await?;
    drop(db);
    let intents = read_identity_delete_intents(workspace)?;
    if intents.iter().any(|(_, intent)| !manifest.intent_digests.contains(&intent.digest)) {
        anyhow::bail!("delete intent set changed during semantic rebuild");
    }
    if validate_legacy_semantic_rebuild(workspace, transaction)? != digest {
        anyhow::bail!("semantic rebuild transaction changed before marker publication");
    }
    publish_identity_v5_marker(&lance, &manifest.new_marker)?;
    if validate_legacy_semantic_rebuild(workspace, transaction)? != digest {
        anyhow::bail!("semantic rebuild marker failed verification");
    }
    for (path, _) in intents {
        std::fs::remove_file(path)?;
        std::fs::File::open(identity_delete_intent_root(workspace))?.sync_all()?;
    }
    let archive = lance.join("legacy-semantic-rebuild-archive");
    match std::fs::create_dir(&archive) {
        Ok(()) => std::fs::File::open(&lance)?.sync_all()?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = std::fs::symlink_metadata(&archive)?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                anyhow::bail!("semantic rebuild archive is not a real directory");
            }
        }
        Err(error) => return Err(error.into()),
    }
    if archive.join(transaction).try_exists()? {
        anyhow::bail!("semantic rebuild archive transaction already exists");
    }
    rename_identity_noreplace(&dir, &archive.join(transaction))?;
    std::fs::File::open(&root)?.sync_all()?;
    std::fs::File::open(&archive)?.sync_all()?;
    Ok(())
}

/// Construct a nonempty schema-4 workspace for integration tests. This helper
/// is unavailable in release builds, including with the test-helpers feature.
#[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
pub async fn populated_identity_v4_workspace_for_test(root: &Path) -> anyhow::Result<PathBuf> {
    populated_identity_v4_with_vector_for_test(root, false).await
}

/// Construct a nonempty schema-4 workspace whose old rows carry vectors.
#[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
pub async fn populated_identity_v4_workspace_with_vector_for_test(root: &Path) -> anyhow::Result<PathBuf> {
    populated_identity_v4_with_vector_for_test(root, true).await
}

#[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]
async fn populated_identity_v4_with_vector_for_test(root: &Path, with_vector: bool) -> anyhow::Result<PathBuf> {
    let workspace = root.join(".forgeplan");
    tokio::fs::create_dir_all(workspace.join("prds")).await?;
    tokio::fs::create_dir_all(workspace.join("rfcs")).await?;
    tokio::fs::create_dir_all(workspace.join("lance")).await?;
    tokio::fs::write(workspace.join("prds/PRD-001-auth.md"),
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\nslug: prd-auth-system\nlinks:\n  - target: RFC-001\n    relation: informs\n---\n\nAuth body.\n").await?;
    tokio::fs::write(workspace.join("rfcs/RFC-001-design.md"),
        "---\nid: RFC-001\nkind: rfc\nstatus: draft\ntitle: Design\n---\n\nDesign body.\n").await?;
    let source = read_identity_v5_source(&workspace).await?;
    let path = workspace.join("lance");
    let db = lancedb::connect(path.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 test path"))?)
        .execute().await?;
    let modern_schema = super::schema::artifacts_schema();
    let v4_schema = Arc::new(arrow_schema::Schema::new(modern_schema.fields().iter().take(15).cloned().collect::<Vec<_>>()));
    let mut rows = Vec::new();
    for doc in &source.documents {
        let artifact = crate::db::store::NewArtifact {
            canonical_key: doc.canonical_key.clone(), id: doc.display_id.clone(), kind: doc.kind.clone(),
            status: doc.status.clone(), title: doc.title.clone(), body: doc.body.clone(), depth: doc.depth.clone(),
            author: doc.author.clone(), parent_epic: doc.parent_epic.clone(), valid_until: doc.valid_until.clone(),
            tags: doc.tags.clone(),
        };
        let modern = super::convert::artifact_to_batch(&artifact, &doc.created_at)?;
        let mut columns = modern.columns()[..15].to_vec();
        if with_vector {
            columns[13] = super::convert::make_embedding_col_from_option(Some(&vec![0.25; super::schema::EMBEDDING_DIM as usize]));
        }
        rows.push(RecordBatch::try_new(v4_schema.clone(), columns)?);
    }
    db.create_table("artifacts", rows).execute().await?;
    let relation = super::convert::relation_to_batch("PRD-001", "RFC-001", "informs", "2020-01-01T00:00:00Z")?;
    db.create_table("relations", vec![relation]).execute().await?;
    let evidence = RecordBatch::try_new(super::schema::evidence_schema(), vec![
        Arc::new(StringArray::from(vec!["EVID-001"])),
        Arc::new(StringArray::from(vec!["PRD-001"])),
        Arc::new(StringArray::from(vec!["test"])),
        Arc::new(StringArray::from(vec!["supports"])),
        Arc::new(arrow_array::Int32Array::from(vec![3])),
        Arc::new(StringArray::from(vec![Some("2030-01-01")])),
        Arc::new(StringArray::from(vec!["original evidence"])),
        Arc::new(StringArray::from(vec!["2020-01-02T00:00:00Z"])),
    ])?;
    db.create_table("evidence", vec![evidence]).execute().await?;
    Ok(workspace)
}

pub(crate) fn ensure_legacy_layout(lance_dir: &Path) -> anyhow::Result<()> {
    use crate::constellation::storage::{self, IndexProtocolState};
    match std::fs::symlink_metadata(lance_dir) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => anyhow::bail!("legacy Lance path {} is not a real directory", lance_dir.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let state = storage::read_marker(lance_dir)?;
    let current = storage::read_current(lance_dir)?;
    if !matches!(state, IndexProtocolState::Uninitialized) || current.is_some() {
        anyhow::bail!("constellation index does not permit a legacy identity-v5 index");
    }
    Ok(())
}


/// Build the identity-v5 index from the Markdown, verify it, and publish it.
///
/// The caller holds the workspace lock. Before the marker exists a re-run
/// rebuilds only the three staged tables; after it, the staged set is already
/// live and this call is a no-op that reports the published state.
pub(crate) async fn run_identity_v5_migration(workspace: &Path, guard: &crate::workspace::lock::WorkspaceLock) -> anyhow::Result<IdentityV5Report> {
    use crate::workspace::lock::WorkspaceReadPermit;
    guard.matches_workspace(workspace)?;
    let lance_dir = workspace.join("lance");
    ensure_legacy_layout(&lance_dir)?;
    tokio::fs::create_dir_all(&lance_dir).await?;

    if let Some(marker) = read_identity_v5_marker(&lance_dir)? {
        // Use the guarded opener, which checks all table manifests before
        // LanceDB interprets them and rejects absent published tables.
        let _verified = crate::db::store::LanceStore::open(workspace).await?;
        // Already published: only the v5 set is used from here on.
        return Ok(IdentityV5Report {
            document_count: marker.document_count,
            source_digest: marker.source_digest,
            published: true,
            already_published: true,
            ..Default::default()
        });
    }
    crate::db::store::verify_manifests(
        &lance_dir, &["artifacts", "relations", "evidence", "change_log", "fpf_spec"],
    ).await?;
    let db = lancedb::connect(lance_dir.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 LanceDB path"))?)
        .execute().await?;
    super::auxiliary::check_identity_change_log(&db).await?;
    drop(db);


    let source = read_identity_v5_source(workspace).await?;
    let mut report = verify_identity_v5_source(&source);

    // Rows the pre-migration index holds but the Markdown does not explain
    // must not disappear silently. The Markdown is the source of truth, so a
    // row without a document is a conflict the operator has to resolve —
    // publication waits.
    let legacy_rows = legacy_artifact_row_count(&lance_dir).await?;
    if legacy_rows > source.documents.len() {
        report.unmappable.push(format!(
            "the pre-migration index holds {legacy_rows} rows but the Markdown holds {} documents",
            source.documents.len()
        ));
    }

    if report.is_blocked() {
        anyhow::bail!(
            "the identity-v5 migration found {} canonical duplicate(s) and {} document(s) that \
             cannot be mapped, so the staged index was NOT published and the original index is \
             preserved. Duplicates: {:?}. Unmappable: {:?}.",
            report.duplicate_keys.len(),
            report.unmappable.len(),
            report.duplicate_keys,
            report.unmappable
        );
    }

    let db = lancedb::connect(lance_dir.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 LanceDB path"))?)
        .execute().await?;
    let mut rows = prepare_identity_v5_rows(&db, &source, true).await?;
    drop(db);
    let embeddings = recompute_configured_vectors(workspace, &mut rows[0]).await?;
    publish_prepared_identity_v5(
        workspace, &source, &rows, embeddings, &mut report, validate_current_embedding_profile,
    ).await?;
    Ok(report)
}

fn verify_prepared_embeddings(
    rows: &[RecordBatch],
    profile: Option<&IdentityV5EmbeddingProvenance>,
) -> anyhow::Result<()> {
    let mut count = 0;
    for row in rows {
        let column = row.column_by_name("embedding")
            .ok_or_else(|| anyhow::anyhow!("artifact row has no embedding column"))?;
        if column.is_null(0) {
            continue;
        }
        let profile = profile.ok_or_else(|| anyhow::anyhow!("staged vector has no embedding provenance"))?;
        let key = required_string(row, "canonical_key")?;
        let title = required_string(row, "title")?;
        let body = crate::db::store::record_large_string(row, "body", 0)
            .ok_or_else(|| anyhow::anyhow!("artifact '{key}' has no body"))?;
        let input = format!("{} {}", title, body.chars().take(profile.chunk_size).collect::<String>());
        let vector = super::convert::extract_embedding(row, 0)
            .ok_or_else(|| anyhow::anyhow!("staged vector '{key}' is invalid"))?;
        if vector.len() != profile.dimension
            || vector.iter().any(|value| !value.is_finite())
            || !vector.iter().any(|value| *value != 0.0)
            || profile.input_digests.get(&key) != Some(&identity_input_digest(&input))
            || profile.vector_digests.get(&key) != Some(&identity_vector_digest(&vector))
            || crate::db::store::record_string(row, "semantic_input_digest", 0).as_deref() != Some(identity_input_digest(&input).as_str())
            || crate::db::store::record_string(row, "semantic_profile_digest", 0).as_deref() != Some(profile.profile_digest.as_str())
            || crate::db::store::record_string(row, "semantic_vector_digest", 0).as_deref() != Some(identity_vector_digest(&vector).as_str())
            || crate::db::store::record_string(row, "semantic_state", 0).as_deref() != Some("ready")
        {
            anyhow::bail!("staged vector '{key}' does not match its profile and input");
        }
        count += 1;
    }
    if let Some(profile) = profile {
        if profile.dimension != super::schema::EMBEDDING_DIM as usize
            || profile.profile_digest != identity_profile_digest(profile)
            || profile.input_digests.len() != count || profile.vector_digests.len() != count
        {
            anyhow::bail!("staged embedding provenance does not match the vector set");
        }
    }
    Ok(())
}

async fn publish_prepared_identity_v5<F>(
    workspace: &Path,
    source: &IdentityV5Source,
    rows: &[Vec<RecordBatch>; 3],
    embeddings: Option<IdentityV5EmbeddingProvenance>,
    report: &mut IdentityV5Report,
    validate_profile: F,
) -> anyhow::Result<()>
where
    F: FnOnce(&Path, &IdentityV5EmbeddingProvenance) -> anyhow::Result<()>,
{
    let lance_dir = workspace.join("lance");
    verify_prepared_embeddings(&rows[0], embeddings.as_ref())?;
    report.relation_count = rows[1].len();
    report.evidence_count = rows[2].len();
    write_prepared_identity_v5_rows(&lance_dir, rows).await?;
    let db = lancedb::connect(lance_dir.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 LanceDB path"))?)
        .execute().await?;
    verify_prepared_identity_v5_rows(&db, rows).await?;
    drop(db);
    let current_source = read_identity_v5_source(workspace).await?;
    if current_source.source_digest != source.source_digest {
        anyhow::bail!("Markdown changed during identity-v5 migration; stage was not published");
    }
    if let Some(profile) = &embeddings {
        validate_profile(workspace, profile)?;
    }

    let marker = IdentityV5Marker {
        version: IDENTITY_V5_VERSION,
        artifacts_table: IDENTITY_V5_ARTIFACTS_TABLE.to_string(),
        relations_table: IDENTITY_V5_RELATIONS_TABLE.to_string(),
        evidence_table: IDENTITY_V5_EVIDENCE_TABLE.to_string(),
        source_digest: report.source_digest.clone(),
        document_count: report.document_count,
        completed: true,
        semantic: Some(IdentityV5SemanticProtocol {
            protocol: 1,
            active_profile_digest: embeddings.as_ref().map(|profile| profile.profile_digest.clone()),
        }),
        embeddings,
    };
    publish_identity_v5_marker_new(&lance_dir, &marker)?;
    report.published = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::convert::{extract_tags, make_null_embedding_col};
    use crate::db::schema::EMBEDDING_DIM;
    use arrow_array::{
        Array, Float64Array, LargeStringArray, ListArray, RecordBatch, StringArray,
        builder::{ListBuilder, StringBuilder},
    };
    use arrow_schema::{DataType, Field, Schema};
    use futures::TryStreamExt;
    use lancedb::query::ExecutableQuery;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[cfg(windows)]
    #[test]
    fn windows_no_replace_move_preserves_existing_directory() {
        let root = TempDir::new().unwrap();
        let src = root.path().join("source");
        let dst = root.path().join("destination");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("source"), b"source").unwrap();
        std::fs::create_dir(&dst).unwrap();
        std::fs::write(dst.join("original"), b"original").unwrap();
        assert!(rename_identity_noreplace(&src, &dst).is_err());
        assert_eq!(std::fs::read(src.join("source")).unwrap(), b"source");
        assert_eq!(std::fs::read(dst.join("original")).unwrap(), b"original");
        let moved = root.path().join("moved");
        rename_identity_noreplace(&src, &moved).unwrap();
        assert!(!src.exists());
        assert_eq!(std::fs::read(moved.join("source")).unwrap(), b"source");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn windows_tracked_stage_rebuilds_without_publishing_marker() {
        let root = TempDir::new().unwrap();
        let stage = root.path().join("space Ю % #");
        std::fs::create_dir(&stage).unwrap();
        let canonical = stage.canonicalize().unwrap();
        assert!(canonical.to_string_lossy().starts_with(r"\\?\"));
        let relation = RecordBatch::try_new(
            super::super::schema::relations_schema(),
            vec![
                Arc::new(StringArray::from(vec!["prd-first"])),
                Arc::new(StringArray::from(vec!["rfc-second"])),
                Arc::new(StringArray::from(vec!["based_on"])),
                Arc::new(StringArray::from(vec!["2026-01-01T00:00:00Z"])),
                Arc::new(arrow_array::Int32Array::from(vec![Some(1)])),
            ],
        ).unwrap();
        let rows: [Vec<RecordBatch>; 3] = [vec![], vec![relation], vec![]];
        write_prepared_identity_v5_rows(&canonical, &rows).await.unwrap();
        {
            let db = lancedb::connect(stage.to_str().unwrap()).execute().await.unwrap();
            verify_prepared_identity_v5_rows(&db, &rows).await.unwrap();
        }
        write_prepared_identity_v5_rows(&canonical, &rows).await.unwrap();
        {
            let db = lancedb::connect(stage.to_str().unwrap()).execute().await.unwrap();
            verify_prepared_identity_v5_rows(&db, &rows).await.unwrap();
        }
        for name in [IDENTITY_V5_ARTIFACTS_TABLE, IDENTITY_V5_RELATIONS_TABLE, IDENTITY_V5_EVIDENCE_TABLE] {
            assert!(canonical.join(format!("{name}.lance")).is_dir());
        }
        assert!(read_identity_v5_marker(&stage).unwrap().is_none());
    }

    #[cfg(windows)]
    #[test]
    fn windows_seal_refuses_an_exclusively_open_stage_file() {
        use std::os::windows::fs::OpenOptionsExt;
        let root = TempDir::new().unwrap();
        let stage = root.path().join("stage");
        std::fs::create_dir(&stage).unwrap();
        let file = stage.join("data");
        std::fs::write(&file, b"stage").unwrap();
        let locked = std::fs::OpenOptions::new().read(true).share_mode(0).open(&file).unwrap();
        assert!(crate::constellation::durability::seal_unpublished_tree(&stage).is_err());
        drop(locked);
        assert_eq!(std::fs::read(file).unwrap(), b"stage");
    }

    /// Schema used by v3 workspaces (no `tags` column).
    fn v3_artifacts_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("kind", DataType::Utf8, false),
            Field::new("status", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("body", DataType::LargeUtf8, false),
            Field::new("depth", DataType::Utf8, false),
            Field::new("author", DataType::Utf8, true),
            Field::new("parent_epic", DataType::Utf8, true),
            Field::new("r_eff_score", DataType::Float64, false),
            Field::new("valid_until", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
            Field::new("updated_at", DataType::Utf8, false),
            Field::new("body_hash", DataType::Utf8, true),
            Field::new(
                "embedding",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    EMBEDDING_DIM,
                ),
                true,
            ),
        ]))
    }

    /// Build a one-row v3 batch (no tags column) for a legacy artifact.
    fn v3_row(id: &str, kind: &str, status: &str, title: &str, body: &str) -> RecordBatch {
        let schema = v3_artifacts_schema();
        let now = "2026-01-01T00:00:00Z";
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec![id])),
                Arc::new(StringArray::from(vec![kind])),
                Arc::new(StringArray::from(vec![status])),
                Arc::new(StringArray::from(vec![title])),
                Arc::new(LargeStringArray::from(vec![body])),
                Arc::new(StringArray::from(vec!["standard"])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                Arc::new(Float64Array::from(vec![0.0f64])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                Arc::new(StringArray::from(vec![now])),
                Arc::new(StringArray::from(vec![now])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                make_null_embedding_col(1),
            ],
        )
        .unwrap()
    }

    /// Create a fresh LanceDB at `dir` with an `artifacts` table matching v3 schema
    /// (no tags column) and return the opened table handle.
    async fn create_v3_table(dir: &std::path::Path) -> Table {
        let conn = lancedb::connect(dir.to_str().unwrap())
            .execute()
            .await
            .unwrap();
        let schema = v3_artifacts_schema();
        let empty = RecordBatch::new_empty(schema);
        conn.create_table("artifacts", vec![empty])
            .execute()
            .await
            .unwrap()
    }

    async fn collect_rows(table: &Table) -> Vec<RecordBatch> {
        table
            .query()
            .execute()
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap()
    }

    /// H4 regression (Sprint 13.3 W5): proves the migration now succeeds on a
    /// real v3 workspace. Previously `CAST(NULL AS LIST<STRING>)` was rejected
    /// by the lance-datafusion SQL parser; the fix switches to
    /// `NewColumnTransform::AllNulls` with a typed Arrow schema.
    #[tokio::test]
    async fn migrate_v3_to_v4_succeeds_on_real_v3_workspace() {
        let tmp = TempDir::new().unwrap();
        let table = create_v3_table(tmp.path()).await;
        let row = v3_row("PRD-001", "prd", "draft", "Legacy", "body");
        table.add(vec![row]).execute().await.unwrap();

        migrate_artifacts(&table)
            .await
            .expect("v3→v4 migration must succeed on a real v3 row");

        let schema = table.schema().await.unwrap();
        assert!(schema.fields().iter().any(|f| f.name() == "tags"));
    }

    #[tokio::test]
    async fn migrate_v3_to_v4_adds_tags_column() {
        let tmp = TempDir::new().unwrap();
        let table = create_v3_table(tmp.path()).await;

        // Precondition: no tags column yet
        let schema = table.schema().await.unwrap();
        assert!(!schema.fields().iter().any(|f| f.name() == "tags"));

        migrate_artifacts(&table).await.unwrap();

        let schema = table.schema().await.unwrap();
        assert!(
            schema.fields().iter().any(|f| f.name() == "tags"),
            "tags column should exist after migration"
        );
    }

    #[tokio::test]
    async fn migrate_v3_to_v4_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let table = create_v3_table(tmp.path()).await;

        migrate_artifacts(&table).await.unwrap();
        // Second run must not error — tags column already present.
        migrate_artifacts(&table).await.unwrap();
        migrate_artifacts(&table).await.unwrap();

        let schema = table.schema().await.unwrap();
        let tags_fields: Vec<_> = schema
            .fields()
            .iter()
            .filter(|f| f.name() == "tags")
            .collect();
        assert_eq!(tags_fields.len(), 1, "exactly one tags column");
    }

    #[tokio::test]
    async fn migrate_v3_to_v4_preserves_existing_rows() {
        let tmp = TempDir::new().unwrap();
        let table = create_v3_table(tmp.path()).await;

        // Insert v3-style row (no tags column)
        let _v3_schema = v3_artifacts_schema();
        let row = v3_row("PRD-001", "prd", "draft", "Legacy Test", "# legacy body");
        table.add(vec![row]).execute().await.unwrap();

        // Sanity: row is present pre-migration
        let pre = collect_rows(&table).await;
        let pre_rows: usize = pre.iter().map(|b| b.num_rows()).sum();
        assert_eq!(pre_rows, 1);

        // Run migration
        migrate_artifacts(&table).await.unwrap();

        // Verify row is still there post-migration
        let post = collect_rows(&table).await;
        let post_rows: usize = post.iter().map(|b| b.num_rows()).sum();
        assert_eq!(post_rows, 1, "existing row must survive migration");

        // Verify tags column exists and reads as empty (null → Vec::new via extract_tags)
        let batch = &post[0];
        let tags_col = batch
            .column_by_name("tags")
            .expect("tags column must exist after migration");
        // Column should be a ListArray; the one existing row should have null tags
        // (because the CAST(NULL AS LIST<STRING>) fills old rows with null).
        let list = tags_col.as_any().downcast_ref::<ListArray>().unwrap();
        assert!(
            list.is_null(0) || extract_tags(batch, 0).is_empty(),
            "pre-existing row must have null or empty tags"
        );

        // extract_tags must return an empty Vec for null rows (not panic)
        let tags = extract_tags(batch, 0);
        assert_eq!(tags, Vec::<String>::new());

        // Verify id/title preserved
        let id_col = batch
            .column_by_name("id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .unwrap();
        assert_eq!(id_col.value(0), "PRD-001");
        let title_col = batch
            .column_by_name("title")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .unwrap();
        assert_eq!(title_col.value(0), "Legacy Test");
    }

    #[tokio::test]
    async fn migrate_v3_to_v4_allows_new_rows_with_tags() {
        let tmp = TempDir::new().unwrap();
        let table = create_v3_table(tmp.path()).await;

        // Insert a v3 row first
        let _v3_schema = v3_artifacts_schema();
        let legacy = v3_row("PRD-001", "prd", "draft", "Legacy", "old body");
        table.add(vec![legacy]).execute().await.unwrap();

        // Migrate
        migrate_artifacts(&table).await.unwrap();

        // Now insert a v4 row WITH tags using the current (post-migration) schema
        let v4_schema = table.schema().await.unwrap();
        let mut tag_builder = ListBuilder::new(StringBuilder::new());
        tag_builder.values().append_value("source=code");
        tag_builder.values().append_value("layer=domain");
        tag_builder.append(true);
        let tags_col: ListArray = tag_builder.finish();

        let now = "2026-01-02T00:00:00Z";
        let v4_row = RecordBatch::try_new(
            v4_schema.clone(),
            vec![
                Arc::new(StringArray::from(vec!["PRD-002"])),
                Arc::new(StringArray::from(vec!["prd"])),
                Arc::new(StringArray::from(vec!["draft"])),
                Arc::new(StringArray::from(vec!["New With Tags"])),
                Arc::new(LargeStringArray::from(vec!["# new body"])),
                Arc::new(StringArray::from(vec!["standard"])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                Arc::new(Float64Array::from(vec![0.0f64])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                Arc::new(StringArray::from(vec![now])),
                Arc::new(StringArray::from(vec![now])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                make_null_embedding_col(1),
                Arc::new(tags_col),
            ],
        )
        .unwrap();
        table.add(vec![v4_row]).execute().await.unwrap();

        // Read back: both rows present, new row has tags populated
        let batches = collect_rows(&table).await;
        let total: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total, 2);

        // Find PRD-002 row across batches and verify its tags
        let mut found_tags = None;
        for batch in &batches {
            let ids = batch
                .column_by_name("id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .unwrap();
            for row in 0..batch.num_rows() {
                if ids.value(row) == "PRD-002" {
                    found_tags = Some(extract_tags(batch, row));
                }
            }
        }
        let tags = found_tags.expect("PRD-002 must be present");
        assert_eq!(tags, vec!["source=code", "layer=domain"]);
    }

    /// Pre-PRD-042 fpf_spec schema (8 columns, no embedding).
    fn legacy_fpf_spec_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("section_id", DataType::Utf8, false),
            Field::new("parent_section", DataType::Utf8, true),
            Field::new("title", DataType::Utf8, false),
            Field::new("body", DataType::LargeUtf8, false),
            Field::new("line_count", DataType::Int32, false),
            Field::new("file_path", DataType::Utf8, false),
            Field::new("created_at", DataType::Utf8, false),
        ]))
    }

    async fn create_legacy_fpf_spec_table(dir: &std::path::Path) -> Table {
        let conn = lancedb::connect(dir.to_str().unwrap())
            .execute()
            .await
            .unwrap();
        let schema = legacy_fpf_spec_schema();
        let empty = RecordBatch::new_empty(schema);
        conn.create_table("fpf_spec", vec![empty])
            .execute()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn migrate_fpf_spec_adds_embedding_column() {
        let tmp = TempDir::new().unwrap();
        let table = create_legacy_fpf_spec_table(tmp.path()).await;

        let schema = table.schema().await.unwrap();
        assert!(!schema.fields().iter().any(|f| f.name() == "embedding"));

        migrate_fpf_spec(&table).await.unwrap();

        let schema = table.schema().await.unwrap();
        let emb = schema
            .fields()
            .iter()
            .find(|f| f.name() == "embedding")
            .expect("embedding column should exist after migration");
        assert!(emb.is_nullable());
        match emb.data_type() {
            DataType::FixedSizeList(inner, size) => {
                assert_eq!(*size, EMBEDDING_DIM);
                assert_eq!(*inner.data_type(), DataType::Float32);
            }
            _ => panic!("embedding should be FixedSizeList<Float32, 1024>"),
        }
    }

    /// Build a one-row legacy fpf_spec batch (no embedding column).
    fn legacy_fpf_row(id: &str, section_id: &str, title: &str, body: &str) -> RecordBatch {
        let schema = legacy_fpf_spec_schema();
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec![id])),
                Arc::new(StringArray::from(vec![section_id])),
                Arc::new(StringArray::from(vec![Option::<&str>::None])),
                Arc::new(StringArray::from(vec![title])),
                Arc::new(LargeStringArray::from(vec![body])),
                Arc::new(arrow_array::Int32Array::from(vec![1])),
                Arc::new(StringArray::from(vec!["fpf.md"])),
                Arc::new(StringArray::from(vec!["2026-01-01T00:00:00Z"])),
            ],
        )
        .unwrap()
    }

    #[tokio::test]
    async fn migrate_fpf_spec_preserves_pre_existing_rows() {
        // Legacy v3-style fpf_spec workspace: 2 rows, no embedding column.
        // After migrate_fpf_spec, both rows must still be present and the
        // embedding column must exist with null values for the legacy rows.
        let tmp = TempDir::new().unwrap();
        let table = create_legacy_fpf_spec_table(tmp.path()).await;
        table
            .add(vec![legacy_fpf_row(
                "fpf-B.3-001",
                "B.3",
                "Trust",
                "body 1",
            )])
            .execute()
            .await
            .unwrap();
        table
            .add(vec![legacy_fpf_row("fpf-A.1-001", "A.1", "ADI", "body 2")])
            .execute()
            .await
            .unwrap();

        let pre = collect_rows(&table).await;
        let pre_rows: usize = pre.iter().map(|b| b.num_rows()).sum();
        assert_eq!(pre_rows, 2, "precondition: 2 legacy rows inserted");

        migrate_fpf_spec(&table).await.unwrap();

        // Schema now has embedding
        let schema = table.schema().await.unwrap();
        assert!(
            schema.fields().iter().any(|f| f.name() == "embedding"),
            "embedding column should exist after migration"
        );

        // Rows preserved
        let post = collect_rows(&table).await;
        let post_rows: usize = post.iter().map(|b| b.num_rows()).sum();
        assert_eq!(post_rows, 2, "existing rows must survive migration");

        // Content preserved + embedding is null for legacy rows
        let mut seen_ids: Vec<String> = Vec::new();
        for batch in &post {
            let ids = batch
                .column_by_name("id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .unwrap();
            let emb_col = batch.column_by_name("embedding").unwrap();
            for row in 0..batch.num_rows() {
                seen_ids.push(ids.value(row).to_string());
                assert!(
                    emb_col.is_null(row),
                    "legacy row {} should have null embedding",
                    ids.value(row)
                );
            }
        }
        seen_ids.sort();
        assert_eq!(seen_ids, vec!["fpf-A.1-001", "fpf-B.3-001"]);
    }

    #[tokio::test]
    async fn migrate_fpf_spec_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let table = create_legacy_fpf_spec_table(tmp.path()).await;

        migrate_fpf_spec(&table).await.unwrap();
        migrate_fpf_spec(&table).await.unwrap();
        migrate_fpf_spec(&table).await.unwrap();

        let schema = table.schema().await.unwrap();
        let emb_count = schema
            .fields()
            .iter()
            .filter(|f| f.name() == "embedding")
            .count();
        assert_eq!(emb_count, 1, "exactly one embedding column");
    }

    #[tokio::test]
    async fn migrate_v3_to_v4_null_tags_read_as_empty_vec() {
        // Multiple legacy rows → migrate → extract_tags must never panic and
        // must return empty Vec<String> for every pre-existing row.
        let tmp = TempDir::new().unwrap();
        let table = create_v3_table(tmp.path()).await;

        let _v3_schema = v3_artifacts_schema();
        for i in 1..=3 {
            let row = v3_row(
                &format!("PRD-{:03}", i),
                "prd",
                "draft",
                &format!("Legacy {}", i),
                "body",
            );
            table.add(vec![row]).execute().await.unwrap();
        }

        migrate_artifacts(&table).await.unwrap();

        let batches = collect_rows(&table).await;
        let mut total = 0;
        for batch in &batches {
            for row in 0..batch.num_rows() {
                let tags = extract_tags(batch, row);
                assert_eq!(
                    tags,
                    Vec::<String>::new(),
                    "legacy row {} must read as empty tags",
                    row
                );
                total += 1;
            }
        }
        assert_eq!(total, 3);
    }

    // ─────────────────────────────────────────────────────────────────
    // Identity-v5 staged migration.
    // ─────────────────────────────────────────────────────────────────

    /// A workspace with two documents: one declares a slug, one does not.
    async fn identity_v5_workspace(tmp: &std::path::Path) -> std::path::PathBuf {
        let ws = tmp.join(".forgeplan");
        tokio::fs::create_dir_all(ws.join("prds")).await.unwrap();
        tokio::fs::create_dir_all(ws.join("rfcs")).await.unwrap();
        tokio::fs::write(
            ws.join("prds/PRD-001-auth.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\nslug: prd-auth-system\nlinks:\n  - target: RFC-001\n    relation: informs\n---\n\nAuth body.\n",
        )
        .await
        .unwrap();
        tokio::fs::write(
            ws.join("rfcs/RFC-001-wire.md"),
            "---\nid: RFC-001\nkind: rfc\nstatus: draft\ntitle: Wire\n---\n\nWire body.\n",
        )
        .await
        .unwrap();
        ws
    }

    #[tokio::test]
    async fn ambiguous_link_blocks_identity_migration_without_selecting_a_document() {
        let tmp = TempDir::new().unwrap();
        let ws = identity_v5_workspace(tmp.path()).await;
        tokio::fs::write(
            ws.join("rfcs/RFC-001-second.md"),
            "---\nid: RFC-001\nkind: rfc\nstatus: draft\ntitle: Second\nslug: rfc-second\n---\n\nOther body.\n",
        )
        .await
        .unwrap();
        let source = read_identity_v5_source(&ws).await.unwrap();
        let report = verify_identity_v5_source(&source);
        assert!(report.is_blocked());
        assert!(report.unmappable.iter().any(|item| item.contains("RFC-001")));
        let candidates = identity_v5_candidates(&source);
        assert!(resolve_reference(&candidates, "RFC-001").is_err());
        assert_eq!(resolve_reference(&candidates, "rfc-second").unwrap(), "rfc-second");
    }

    #[tokio::test]
    async fn verification_preserves_historical_forward_targets_and_rejects_malformed() {
        let tmp = TempDir::new().unwrap();
        let ws = identity_v5_workspace(tmp.path()).await;
        let mut source = read_identity_v5_source(&ws).await.unwrap();
        let doc = source.documents.iter_mut().find(|doc| doc.display_id == "PRD-001").unwrap();
        doc.links = vec![("RFC-A01".into(), "informs".into())];
        let report = verify_identity_v5_source(&source);
        assert!(report.unmappable.is_empty(), "{:?}", report.unmappable);
        assert!(report.unresolved_references.iter().any(|item| item.contains("RFC-A01")));
        let candidates = identity_v5_candidates(&source);
        assert_eq!(resolve_reference(&candidates, "RFC-A01").unwrap(), "RFC-A01");
        for bad in ["---", "PRD-4294967296"] {
            assert!(resolve_reference(&candidates, bad).is_err());
            let doc = source.documents.iter_mut().find(|doc| doc.display_id == "PRD-001").unwrap();
            doc.links = vec![(bad.into(), "informs".into())];
            let report = verify_identity_v5_source(&source);
            assert!(report.unmappable.iter().any(|item| item.contains(bad)));
        }
    }

    /// The table names of a Lance root.
    async fn db_table_names(lance_dir: &std::path::Path) -> Vec<String> {
        let db = lancedb::connect(lance_dir.to_str().unwrap())
            .execute()
            .await
            .unwrap();
        db.table_names().execute().await.unwrap()
    }

    /// No marker means the workspace was never migrated — not a failure.
    #[test]
    fn a_missing_marker_is_not_a_migration() {
        let tmp = TempDir::new().unwrap();
        let lance_dir = tmp.path().join("lance");
        std::fs::create_dir_all(&lance_dir).unwrap();
        assert!(
            read_identity_v5_marker(&lance_dir).unwrap().is_none(),
            "a workspace without a marker was never migrated"
        );
        assert_eq!(
            resolve_legacy_table_names(&lance_dir).unwrap(),
            LegacyTableNames::default(),
            "without a marker the default table names apply"
        );
    }

    /// A damaged marker is an explicit error. The store must not fall back
    /// to the pre-migration index on its own.
    #[test]
    fn a_damaged_marker_is_an_explicit_error() {
        let tmp = TempDir::new().unwrap();
        let lance_dir = tmp.path().join("lance");
        std::fs::create_dir_all(&lance_dir).unwrap();
        std::fs::write(identity_v5_marker_path(&lance_dir), b"{not json").unwrap();

        let error = read_identity_v5_marker(&lance_dir)
            .expect_err("a damaged marker must not be read as a migration");
        assert!(
            error.to_string().contains("damaged"),
            "the error must say the marker is damaged, got {error}"
        );
        assert!(
            resolve_legacy_table_names(&lance_dir).is_err(),
            "a damaged marker must not resolve to the pre-migration tables"
        );
    }

    /// A marker without completion is damaged too — the set it names may be
    /// half built.
    #[test]
    fn an_incomplete_marker_is_an_explicit_error() {
        let tmp = TempDir::new().unwrap();
        let lance_dir = tmp.path().join("lance");
        std::fs::create_dir_all(&lance_dir).unwrap();
        publish_identity_v5_marker_new(
            &lance_dir,
            &IdentityV5Marker {
                version: IDENTITY_V5_VERSION,
                artifacts_table: IDENTITY_V5_ARTIFACTS_TABLE.to_string(),
                relations_table: IDENTITY_V5_RELATIONS_TABLE.to_string(),
                evidence_table: IDENTITY_V5_EVIDENCE_TABLE.to_string(),
                source_digest: "abc".to_string(),
                document_count: 1,
                completed: false,
                embeddings: None,
                semantic: None,
            },
        )
        .unwrap();

        let error = read_identity_v5_marker(&lance_dir)
            .expect_err("an incomplete marker must not be read as a migration");
        assert!(
            error.to_string().contains("incomplete"),
            "the error must say the build is incomplete, got {error}"
        );
    }

    /// The published marker names the three tables of the staged set, and
    /// the store opens exactly those.
    #[test]
    fn a_published_marker_names_the_staged_tables() {
        let tmp = TempDir::new().unwrap();
        let lance_dir = tmp.path().join("lance");
        std::fs::create_dir_all(&lance_dir).unwrap();
        let marker = IdentityV5Marker {
            version: IDENTITY_V5_VERSION,
            artifacts_table: IDENTITY_V5_ARTIFACTS_TABLE.to_string(),
            relations_table: IDENTITY_V5_RELATIONS_TABLE.to_string(),
            evidence_table: IDENTITY_V5_EVIDENCE_TABLE.to_string(),
            source_digest: "a".repeat(64),
            document_count: 2,
            completed: true,
            embeddings: None,
            semantic: None,
        };
        publish_identity_v5_marker_new(&lance_dir, &marker).unwrap();

        assert_eq!(read_identity_v5_marker(&lance_dir).unwrap(), Some(marker));
        let names = resolve_legacy_table_names(&lance_dir).unwrap();
        assert_eq!(names.artifacts, IDENTITY_V5_ARTIFACTS_TABLE);
        assert_eq!(names.relations, IDENTITY_V5_RELATIONS_TABLE);
        assert_eq!(names.evidence, IDENTITY_V5_EVIDENCE_TABLE);
    }

    /// The staged set is built FROM THE MARKDOWN: a document that declares a
    /// slug is keyed by that slug, and the marker is published only after the
    /// set was verified.
    #[tokio::test]
    async fn nested_markdown_is_captured_and_indexed_without_changing_v4() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        tokio::fs::create_dir_all(ws.join("prds/nested/deeper")).await.unwrap();
        let path = ws.join("prds/nested/deeper/PRD-002-nested.md");
        tokio::fs::write(&path, "---\nid: PRD-002\nkind: prd\nstatus: draft\ntitle: Nested\nslug: prd-nested\n---\n\nNested body.\n").await.unwrap();
        let before = tokio::fs::read(&path).await.unwrap();
        let candidates = crate::artifact::documents::workspace_candidates(&ws).await.unwrap();
        assert!(candidates.iter().any(|candidate| candidate.canonical_key == "prd-nested"
            && candidate.relative_path == std::path::Path::new("prds/nested/deeper/PRD-002-nested.md")));
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let old = table_rows(&db, "artifacts").await.unwrap();
        let report = run_identity_v5_migration(&ws, &lock).await.unwrap();
        assert_eq!(report.document_count, 3);
        let staged = table_rows(&db, IDENTITY_V5_ARTIFACTS_TABLE).await.unwrap();
        assert!(staged.iter().any(|row| required_string(row, "canonical_key").unwrap() == "prd-nested"));
        assert_eq!(tokio::fs::read(&path).await.unwrap(), before);
        let after = table_rows(&db, "artifacts").await.unwrap();
        assert_eq!(old.len(), after.len());
        for (left, right) in old.iter().zip(&after) {
            assert!(left.columns().iter().zip(right.columns()).all(|(a, b)| a.to_data() == b.to_data()));
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlink_in_source_cannot_publish_a_partial_index() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        std::os::unix::fs::symlink("PRD-001-auth.md", ws.join("prds/alias.md")).unwrap();
        assert!(run_identity_v5_migration(&ws, &lock).await.unwrap_err().to_string().contains("symlink"));
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
    }

    #[tokio::test]
    async fn the_staged_index_is_built_from_the_markdown_and_published() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();

        let report = run_identity_v5_migration(&ws, &lock).await.unwrap();
        assert!(report.published, "the verified set must be published");
        assert!(!report.already_published);
        assert_eq!(report.document_count, 2, "both documents must be indexed");
        assert_eq!(report.relation_count, 1, "the declared link must be indexed");

        let lance_dir = ws.join("lance");
        let marker = read_identity_v5_marker(&lance_dir).unwrap().unwrap();
        assert!(marker.completed);
        assert_eq!(marker.document_count, 2);
        assert_eq!(marker.source_digest, report.source_digest);

        // The staged tables coexist with the untouched v4 tables.
        let names = db_table_names(&lance_dir).await;
        assert!(
            names.iter().any(|name| name.as_str() == IDENTITY_V5_ARTIFACTS_TABLE),
            "the staged artifacts table must exist, got {names:?}"
        );
        assert!(
            names.iter().any(|name| name.as_str() == IDENTITY_V5_RELATIONS_TABLE),
            "the staged relations table must exist, got {names:?}"
        );
        assert!(
            names.iter().any(|name| name.as_str() == IDENTITY_V5_EVIDENCE_TABLE),
            "the staged evidence table must exist, got {names:?}"
        );
        assert_eq!(
            names
                .iter()
                .filter(|name| name.starts_with("artifacts"))
                .count(),
            2,
            "the staged and original artifact tables must both exist"
        );

        // The store now reads the staged set: the row is addressed by the
        // key the Markdown declares.
        let store = crate::db::store::LanceStore::open(&ws).await.unwrap();
        let record = store
            .get_record(&lock, "prd-auth-system")
            .await
            .unwrap()
            .expect("the slug-declaring document must be found by its key");
        assert_eq!(record.id, "PRD-001", "the public id stays the display id");
        assert_eq!(record.canonical_key, "prd-auth-system");
    }

    /// A document without a slug is keyed by its display id.
    #[tokio::test]
    async fn a_document_without_a_slug_is_keyed_by_its_display_id() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        run_identity_v5_migration(&ws, &lock).await.unwrap();

        let store = crate::db::store::LanceStore::open(&ws).await.unwrap();
        let record = store
            .get_record(&lock, "RFC-001")
            .await
            .unwrap()
            .expect("a document without a slug keeps its display id as the key");
        assert_eq!(record.canonical_key, "RFC-001");
    }

    /// Two documents claiming one canonical key are reported and block the
    /// publication: the original index stays exactly as it was.
    #[tokio::test]
    async fn a_canonical_duplicate_blocks_the_publication() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        tokio::fs::write(
            ws.join("prds/PRD-002-alpha.md"),
            "---\nid: PRD-002\nkind: prd\nstatus: draft\ntitle: Alpha\nslug: prd-auth-system\n---\n\nAlpha body.\n",
        )
        .await
        .unwrap();

        let error = run_identity_v5_migration(&ws, &lock)
            .await
            .expect_err("a canonical duplicate must block the publication");
        assert!(
            error.to_string().contains("NOT published"),
            "the error must say the set was not published, got {error}"
        );

        let lance_dir = ws.join("lance");
        assert!(
            read_identity_v5_marker(&lance_dir).unwrap().is_none(),
            "a blocked migration must leave no marker"
        );
        let names = db_table_names(&lance_dir).await;
        assert!(
            !names.iter().any(|name| name.as_str() == IDENTITY_V5_ARTIFACTS_TABLE),
            "a blocked migration must not publish the staged table, got {names:?}"
        );
    }

    /// The migration is idempotent: a second run finds the published marker
    /// and changes nothing.
    #[tokio::test]
    async fn a_published_migration_is_not_run_twice() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let first = run_identity_v5_migration(&ws, &lock).await.unwrap();
        assert!(first.published);

        let second = run_identity_v5_migration(&ws, &lock).await.unwrap();
        assert!(second.already_published, "the second run must be a no-op");
        assert!(second.published, "the set stays published");
        assert_eq!(second.document_count, first.document_count);
    }
    async fn populated_v4_workspace(root: &Path, orphan: bool) -> PathBuf {
        populated_v4_workspace_with_vector(root, orphan, false).await
    }

    async fn populated_v4_workspace_with_vector(root: &Path, orphan: bool, with_vector: bool) -> PathBuf {
        populated_v4_workspace_with_body(root, orphan, with_vector, false).await
    }

    async fn populated_v4_workspace_with_body(root: &Path, orphan: bool, with_vector: bool, full_body: bool) -> PathBuf {
        let ws = identity_v5_workspace(root).await;
        let source = read_identity_v5_source(&ws).await.unwrap();
        let lance = ws.join("lance");
        tokio::fs::create_dir_all(&lance).await.unwrap();
        let db = lancedb::connect(lance.to_str().unwrap()).execute().await.unwrap();
        let modern_schema = super::super::schema::artifacts_schema();
        let v4_schema = Arc::new(Schema::new(modern_schema.fields().iter().take(15).cloned().collect::<Vec<_>>()));
        let mut rows = Vec::new();
        for (index, doc) in source.documents.iter().enumerate() {
            let artifact = crate::db::store::NewArtifact {
                canonical_key: doc.canonical_key.clone(),
                id: if orphan && index == 0 { "PRD-999".into() } else { doc.display_id.clone() },
                kind: doc.kind.clone(), status: doc.status.clone(), title: doc.title.clone(),
                body: if full_body { doc.full_text.clone() } else { doc.body.clone() },
                depth: doc.depth.clone(), author: doc.author.clone(),
                parent_epic: doc.parent_epic.clone(), valid_until: doc.valid_until.clone(), tags: doc.tags.clone(),
            };
            let modern = crate::db::convert::artifact_to_batch(&artifact, &doc.created_at).unwrap();
            let mut columns = modern.columns()[..15].to_vec();
            if with_vector {
                columns[13] = crate::db::convert::make_embedding_col_from_option(Some(&vec![0.25; EMBEDDING_DIM as usize]));
            }
            rows.push(RecordBatch::try_new(v4_schema.clone(), columns).unwrap());
        }
        db.create_table("artifacts", rows).execute().await.unwrap();
        let relation = crate::db::convert::relation_to_batch("PRD-001", "RFC-001", "informs", "2020-01-01T00:00:00Z").unwrap();
        db.create_table("relations", vec![relation]).execute().await.unwrap();
        let evidence = RecordBatch::try_new(super::super::schema::evidence_schema(), vec![
            Arc::new(StringArray::from(vec!["EVID-001"])),
            Arc::new(StringArray::from(vec!["PRD-001"])),
            Arc::new(StringArray::from(vec!["test"])),
            Arc::new(StringArray::from(vec!["supports"])),
            Arc::new(arrow_array::Int32Array::from(vec![3])),
            Arc::new(StringArray::from(vec![Some("2030-01-01")])),
            Arc::new(StringArray::from(vec!["original evidence"])),
            Arc::new(StringArray::from(vec!["2020-01-02T00:00:00Z"])),
        ]).unwrap();
        db.create_table("evidence", vec![evidence]).execute().await.unwrap();
        ws
    }

    #[tokio::test]
    async fn v4_history_survives_upgrade_and_accepts_canonical_append() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let modern = super::super::schema::change_log_schema();
        let v4 = Arc::new(Schema::new(modern.fields().iter().take(8).cloned().collect::<Vec<_>>()));
        let old_event = RecordBatch::try_new(v4, vec![
            Arc::new(StringArray::from(vec!["2020-01-01T00:00:00Z"])),
            Arc::new(StringArray::from(vec!["PRD-001"])),
            Arc::new(StringArray::from(vec!["update"])),
            Arc::new(StringArray::from(vec![Some("status")])),
            Arc::new(StringArray::from(vec![Some("draft")])),
            Arc::new(StringArray::from(vec![Some("active")])),
            Arc::new(StringArray::from(vec!["cli"])),
            Arc::new(StringArray::from(vec![Some("abcdef0")])),
        ]).unwrap();
        db.create_table("change_log", vec![old_event]).execute().await.unwrap();
        let original = table_rows(&db, "change_log").await.unwrap().remove(0);
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let legacy = crate::db::store::LanceStore::open(&ws).await.unwrap();
        let historical = legacy.get_change_log(&lock, None, None, 10).await.unwrap();
        assert_eq!(historical.len(), 1);
        assert_eq!(historical[0].artifact_id, "PRD-001");
        assert_eq!(historical[0].canonical_key, None);
        drop(legacy);

        let store = crate::db::store::LanceStore::open_for_write(&ws, &lock).await.unwrap();
        let upgraded = table_rows(&db, "change_log").await.unwrap().remove(0);
        for index in 0..8 {
            assert_eq!(original.column(index).to_data(), upgraded.column(index).to_data());
        }
        assert!(upgraded.column_by_name("canonical_key").unwrap().is_null(0));

        let event = crate::changelog::ChangeLogEntry::new("PRD-001", "link", "test")
            .with_canonical_key("prd-auth-system");
        store.log_change(&lock, &event).await.unwrap();
        let rows = table_rows(&db, "change_log").await.unwrap();
        assert_eq!(rows.len(), 2);
        let appended = rows.iter().find(|row| required_string(row, "action").unwrap() == "link").unwrap();
        assert_eq!(required_string(appended, "artifact_id").unwrap(), "PRD-001");
        assert_eq!(required_string(appended, "canonical_key").unwrap(), "prd-auth-system");
    }

    #[tokio::test]
    async fn invalid_v4_history_schema_blocks_marker() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let expected = super::super::schema::change_log_schema();
        let invalid = Arc::new(Schema::new(expected.fields().iter()
            .filter(|field| field.name() != "source" && field.name() != "canonical_key")
            .cloned().collect::<Vec<_>>()));
        db.create_table("change_log", vec![RecordBatch::new_empty(invalid.clone())])
            .execute().await.unwrap();
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        assert!(crate::db::store::LanceStore::open_for_write(&ws, &lock).await.is_err());
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
        let schema = db.open_table("change_log").execute().await.unwrap().schema().await.unwrap();
        assert_eq!(schema.fields(), invalid.fields());
    }

    #[tokio::test]
    async fn v4_nonempty_migration_preserves_source_rows_and_evidence() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let lance = ws.join("lance");
        let db = lancedb::connect(lance.to_str().unwrap()).execute().await.unwrap();
        let old_artifacts = table_rows(&db, "artifacts").await.unwrap();
        let old_relations = table_rows(&db, "relations").await.unwrap();
        let old_evidence = table_rows(&db, "evidence").await.unwrap();
        let store = crate::db::store::LanceStore::open(&ws).await.unwrap();
        assert!(!store.uses_identity_v5());
        assert_eq!(store.get_record(&lock, "prd-auth-system").await.unwrap().unwrap().id, "PRD-001");
        assert_eq!(store.get_relations(&lock, "prd-auth-system").await.unwrap(), vec![("RFC-001".to_string(), "informs".to_string())]);
        assert!(store.ensure_identity_v5_writable(&lock).is_err());
        assert!(crate::db::store::LanceStore::init_new(&ws, &lock).await.is_err());
        assert!(db.open_table("artifacts").execute().await.unwrap().schema().await.unwrap().field_with_name("canonical_key").is_err());
        drop(store);

        let report = crate::db::store::LanceStore::migrate_identity_v5(&ws, &lock).await.unwrap();
        assert!(report.published);
        assert_eq!(report.evidence_count, 1);
        assert_eq!(report.relation_count, 1);
        for (name, before) in [("artifacts", old_artifacts), ("relations", old_relations), ("evidence", old_evidence)] {
            let after = table_rows(&db, name).await.unwrap();
            assert_eq!(before.len(), after.len());
            for (left, right) in before.iter().zip(after.iter()) {
                assert_eq!(left.schema(), right.schema());
                assert!(left.columns().iter().zip(right.columns()).all(|(a, b)| a.to_data() == b.to_data()));
            }
        }
        let evidence = table_rows(&db, IDENTITY_V5_EVIDENCE_TABLE).await.unwrap();
        assert_eq!(required_string(&evidence[0], "artifact_id").unwrap(), "prd-auth-system");
        assert_eq!(required_string(&evidence[0], "content").unwrap(), "original evidence");
        let relations = table_rows(&db, IDENTITY_V5_RELATIONS_TABLE).await.unwrap();
        assert_eq!(required_string(&relations[0], "source_id").unwrap(), "prd-auth-system");
        assert_eq!(required_string(&relations[0], "created_at").unwrap(), "2020-01-01T00:00:00Z");
        let store = crate::db::store::LanceStore::open(&ws).await.unwrap();
        let record = store.get_record(&lock, "prd-auth-system").await.unwrap().unwrap();
        assert!(record.embedding.is_none());
    }

    #[tokio::test]
    async fn v4_full_markdown_body_is_preserved_in_stage() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace_with_body(tmp.path(), false, false, true).await;
        let expected = tokio::fs::read_to_string(ws.join("prds/PRD-001-auth.md")).await.unwrap();
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::open_for_write(&ws, &lock).await.unwrap();
        assert_eq!(store.get_record(&lock, "prd-auth-system").await.unwrap().unwrap().body, expected);
    }

    #[tokio::test]
    async fn write_preflight_migrates_v4_and_returns_fresh_v5_handle() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let old = crate::db::store::LanceStore::open(&ws).await.unwrap();
        assert!(!old.uses_identity_v5());
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let fresh = crate::db::store::LanceStore::open_for_write(&ws, &lock).await.unwrap();
        assert!(fresh.uses_identity_v5());
        assert!(old.identity_generation_changed(&lock).unwrap());
        assert!(!fresh.identity_generation_changed(&lock).unwrap());
        fresh.ensure_identity_v5_writable(&lock).unwrap();
        assert_eq!(fresh.get_record(&lock, "prd-auth-system").await.unwrap().unwrap().id, "PRD-001");
        assert!(old.ensure_identity_v5_writable(&lock).is_err());
    }

    #[tokio::test]
    async fn allocation_guard_holds_kind_lock_until_dropped() {
        use crate::artifact::{id_alloc, types::ArtifactKind};
        use fs2::FileExt;
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let allocation = id_alloc::acquire_allocation_guard(&ws, &lock, &[ArtifactKind::Prd]).await.unwrap();
        let competing = std::fs::OpenOptions::new().read(true).write(true)
            .open(ws.join(".id-alloc-prd.lock")).unwrap();
        assert!(competing.try_lock_exclusive().is_err());
        drop(allocation);
        competing.try_lock_exclusive().unwrap();
        FileExt::unlock(&competing).unwrap();
    }

    #[tokio::test]
    async fn prepared_key_uses_preview_and_rejects_stale_number_without_consuming_it() {
        use crate::artifact::{id_alloc, types::ArtifactKind};
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::open_for_write(&ws, &lock).await.unwrap();
        let kind = ArtifactKind::Prd;
        let allocation = id_alloc::acquire_allocation_guard(&ws, &lock, &[kind.clone()]).await.unwrap();
        let (next, _) = id_alloc::preview_display_number(&ws, &lock, &allocation, Some(&store), &kind, 3, None).await.unwrap();
        let build = |id: &str, _: u32, key: &str| Ok(crate::db::store::NewArtifact {
            canonical_key: key.to_string(), id: id.to_string(), kind: "prd".into(),
            status: "draft".into(), title: "Prepared".into(), body: "Body".into(),
            depth: "standard".into(), author: None, parent_epic: None, valid_until: None, tags: vec![],
        });
        let key = crate::artifact::canonical_key::mint_artifact_key(&kind, "Prepared").unwrap();
        assert!(id_alloc::create_artifact_with_prepared_key(
            &ws, &lock, &allocation, Some(&store), &kind, 3, key.clone(), "PRD-999", &build,
        ).await.is_err());
        let (still_next, _) = id_alloc::preview_display_number(&ws, &lock, &allocation, Some(&store), &kind, 3, None).await.unwrap();
        assert_eq!(next, still_next);
        let created = id_alloc::create_artifact_with_prepared_key(
            &ws, &lock, &allocation, Some(&store), &kind, 3, key.clone(), &next, &build,
        ).await.unwrap();
        assert_eq!(created.id, next);
        assert_eq!(created.canonical_key, key);
        let (expected, _) = id_alloc::preview_display_number(&ws, &lock, &allocation, Some(&store), &kind, 3, None).await.unwrap();
        assert!(id_alloc::reserve_prepared_display_number(
            &ws, &lock, &allocation, Some(&store), &kind, 3, "PRD-999",
        ).await.is_err());
        let (unchanged, _) = id_alloc::preview_display_number(&ws, &lock, &allocation, Some(&store), &kind, 3, None).await.unwrap();
        assert_eq!(expected, unchanged);
        id_alloc::reserve_prepared_display_number(&ws, &lock, &allocation, Some(&store), &kind, 3, &expected).await.unwrap();
        let (later, _) = id_alloc::preview_display_number(&ws, &lock, &allocation, Some(&store), &kind, 3, None).await.unwrap();
        assert_ne!(later, expected);
    }

    #[tokio::test]
    async fn scan_plan_survives_v4_reopen_and_executes_exact_identities() {
        use crate::scan::import::{ScanImportOptions, execute_scan_import_plan, plan_scan_import};
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let docs = tmp.path().join("docs");
        tokio::fs::create_dir_all(&docs).await.unwrap();
        tokio::fs::write(docs.join("prd-extra.md"),
            "---\nkind: prd\nslug: prd-extra\ntitle: Extra\nlinks:\n  - target: RFC-001\n    relation: informs\n---\n\nBody.\n").await.unwrap();
        let options = ScanImportOptions::default();
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let legacy = crate::db::store::LanceStore::open(&ws).await.unwrap();
        let kinds = crate::scan::import::scan_import_kinds(tmp.path(), &options).unwrap();
        let allocation = crate::artifact::id_alloc::acquire_allocation_guard(&ws, &lock, &kinds).await.unwrap();
        let before = plan_scan_import(tmp.path(), &ws, &lock, &allocation, Some(&legacy), &options).await.unwrap();
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
        drop(legacy);
        let store = crate::db::store::LanceStore::open_for_write(&ws, &lock).await.unwrap();
        let after = plan_scan_import(tmp.path(), &ws, &lock, &allocation, Some(&store), &options).await.unwrap();
        assert_eq!(before, after);
        let result = execute_scan_import_plan(tmp.path(), &ws, &lock, &allocation, Some(&store), &options, &after).await.unwrap();
        assert_eq!(result.imported, 1);
        assert_eq!(store.get_record(&lock, "prd-extra").await.unwrap().unwrap().id, "PRD-002");
    }

    #[tokio::test]
    async fn scan_plan_refuses_changed_or_unreadable_source() {
        use crate::scan::import::{ScanImportOptions, execute_scan_import_plan, plan_scan_import};
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let docs = tmp.path().join("docs");
        tokio::fs::create_dir_all(&docs).await.unwrap();
        let source = docs.join("PRD-010.md");
        tokio::fs::write(&source, "---\nid: PRD-010\nkind: prd\ntitle: New\n---\nBody").await.unwrap();
        let options = ScanImportOptions::default();
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let legacy = crate::db::store::LanceStore::open(&ws).await.unwrap();
        let kinds = crate::scan::import::scan_import_kinds(tmp.path(), &options).unwrap();
        let allocation = crate::artifact::id_alloc::acquire_allocation_guard(&ws, &lock, &kinds).await.unwrap();
        let plan = plan_scan_import(tmp.path(), &ws, &lock, &allocation, Some(&legacy), &options).await.unwrap();
        tokio::fs::write(&source, [0xff]).await.unwrap();
        assert!(plan_scan_import(tmp.path(), &ws, &lock, &allocation, Some(&legacy), &options).await.is_err());
        assert!(execute_scan_import_plan(tmp.path(), &ws, &lock, &allocation, Some(&legacy), &options, &plan).await.is_err());
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
        assert!(legacy.get_record(&lock, "PRD-010").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn driver_reads_v4_then_migrates_before_first_write() {
        use crate::driver::ArtifactStorage;
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let driver = crate::driver::lance::LanceDriver::open(&ws).await.unwrap();
        assert_eq!(driver.get_record(&lock, "prd-auth-system").await.unwrap().unwrap().id, "PRD-001");
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
        driver.update_r_eff_score(&lock, "prd-auth-system", 0.75).await.unwrap();
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_some());
        assert_eq!(driver.get_record(&lock, "prd-auth-system").await.unwrap().unwrap().r_eff_score, 0.75);
    }

    #[tokio::test]
    async fn ambiguous_driver_selector_does_not_migrate_v4() {
        use crate::driver::ArtifactStorage;
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        tokio::fs::write(
            ws.join("prds/PRD-001-second.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Second\nslug: prd-second\n---\n\nOther body.\n",
        ).await.unwrap();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let before = table_rows(&db, "artifacts").await.unwrap();
        let driver = crate::driver::lance::LanceDriver::open(&ws).await.unwrap();
        assert!(driver.update_r_eff_score(&lock, "PRD-001", 0.75).await.is_err());
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
        let after = table_rows(&db, "artifacts").await.unwrap();
        assert_eq!(before.len(), after.len());
        for (left, right) in before.iter().zip(after.iter()) {
            assert!(left.columns().iter().zip(right.columns()).all(|(a, b)| a.to_data() == b.to_data()));
        }
    }

    #[tokio::test]
    async fn raw_v4_display_lookup_finds_orphans_and_rejects_duplicates() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), true).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::open(&ws).await.unwrap();
        assert!(store.get_record(&lock, "PRD-999").await.unwrap().is_none());
        assert_eq!(store.legacy_record_by_display_id(&lock, "prd-999").await.unwrap().unwrap().id, "PRD-999");
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let old = table_rows(&db, "artifacts").await.unwrap();
        let orphan = old.into_iter().find(|row| required_string(row, "id").unwrap() == "PRD-999").unwrap();
        db.open_table("artifacts").execute().await.unwrap().add(vec![orphan]).execute().await.unwrap();
        let reopened = crate::db::store::LanceStore::open(&ws).await.unwrap();
        assert!(reopened.legacy_record_by_display_id(&lock, "PRD-999").await.is_err());
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
    }

    #[cfg(feature = "semantic-search")]
    #[tokio::test]
    async fn legacy_semantic_backlog_recomputes_after_canonical_body_edit() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        tokio::fs::write(ws.join("config.yaml"), "embedding:\n  model: forgeplan-test\n  chunk_size: 2000\n").await.unwrap();
        let backend = Arc::new(|text: &str| -> anyhow::Result<Vec<f32>> {
            let mut vector = vec![0.0; EMBEDDING_DIM as usize];
            vector[if text.to_lowercase().contains("dolphin") { 1 } else { 0 }] = 1.0;
            Ok(vector)
        });
        let backend_guard = crate::db::store::install_semantic_backend_for_test(&ws, backend).unwrap();
        let guard = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::open_for_write(&ws, &guard).await.unwrap();
        drop(store);
        drop(guard);
        crate::db::store::LanceStore::drain_identity_semantic_backlog(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::open(&ws).await.unwrap();
        let permit = crate::workspace::lock::acquire_workspace_read_lock(&ws).await.unwrap();
        let old = store.get_record(&permit, "prd-auth-system").await.unwrap().unwrap().embedding.unwrap();
        assert_eq!(old[0], 1.0);
        drop(permit);
        let guard = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let ctx = crate::projection::MutationContext::new(&ws, &guard, Some(&store));
        crate::projection::update_body_with_projection(&ctx, "prd-auth-system", "dolphin signal").await.unwrap();
        drop(ctx);
        drop(guard);
        crate::db::store::LanceStore::drain_identity_semantic_backlog(&ws).await.unwrap();
        let fresh = crate::db::store::LanceStore::open(&ws).await.unwrap();
        let permit = crate::workspace::lock::acquire_workspace_read_lock(&ws).await.unwrap();
        let record = fresh.get_record(&permit, "prd-auth-system").await.unwrap().unwrap();
        assert!(record.body.contains("dolphin signal"));
        assert_eq!(record.embedding.as_ref().unwrap()[1], 1.0);
        assert_ne!(record.embedding.as_ref().unwrap(), &old);
        let hits = fresh.vector_search(&permit, "dolphin", 1).await.unwrap();
        assert_eq!(hits[0].record.canonical_key, "prd-auth-system");
        assert!(hits[0].record.body.contains("dolphin signal"));
        drop(backend_guard);
        assert!(fresh.vector_search(&permit, "dolphin", 1).await.is_err());
    }

    #[cfg(feature = "semantic-search")]
    #[tokio::test]
    async fn v4_real_migration_recomputes_vectors_with_scoped_backend() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace_with_vector(tmp.path(), false, true).await;
        tokio::fs::write(ws.join("config.yaml"), "embedding:\n  model: forgeplan-test\n  chunk_size: 2000\n").await.unwrap();
        let backend = Arc::new(|text: &str| -> anyhow::Result<Vec<f32>> {
            let mut vector = vec![0.0; EMBEDDING_DIM as usize];
            vector[if text.contains("Auth") { 0 } else { 1 }] = 1.0;
            Ok(vector)
        });
        let backend_guard = crate::db::store::install_semantic_backend_for_test(&ws, backend).unwrap();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let before = table_rows(&db, "artifacts").await.unwrap();
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::open_for_write(&ws, &lock).await.unwrap();
        let marker = read_identity_v5_marker(&ws.join("lance")).unwrap().unwrap();
        let profile = marker.embeddings.as_ref().unwrap();
        assert_eq!(profile.implementation, "forgeplan-test-only");
        assert_eq!(profile.input_digests.len(), 2);
        let record = store.get_record(&lock, "prd-auth-system").await.unwrap().unwrap();
        assert_eq!(record.embedding.as_ref().unwrap()[0], 1.0);
        let after = table_rows(&db, "artifacts").await.unwrap();
        assert_eq!(before.len(), after.len());
        assert!(before.iter().zip(&after).all(|(left, right)|
            left.columns().iter().zip(right.columns()).all(|(a, b)| a.to_data() == b.to_data())));
        drop(backend_guard);
        assert!(validate_current_embedding_profile(&ws, profile).is_err());
    }

    #[tokio::test]
    async fn v4_vectors_are_recomputed_and_proven_before_publication() {
        use sha2::{Digest, Sha256};
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace_with_vector(tmp.path(), false, true).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let source = read_identity_v5_source(&ws).await.unwrap();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let old = table_rows(&db, "artifacts").await.unwrap();
        let mut rows = prepare_identity_v5_rows(&db, &source, true).await.unwrap();
        let profile = IdentityV5EmbeddingProvenance {
            model: "deterministic-test".into(), repository: "test/repo".into(),
            snapshot_id: "revision".into(), snapshot_digest: "a".repeat(64),
            implementation: "deterministic-test-v1".into(), profile_digest: String::new(),
            chunk_size: 2000, dimension: EMBEDDING_DIM as usize, input_digests: BTreeMap::new(), vector_digests: BTreeMap::new(),
        };
        let mut observed = Vec::new();
        let profile = recompute_identity_vectors(&mut rows[0], profile, |input| {
            observed.push(input.to_string());
            Ok(vec![input.len() as f32; EMBEDDING_DIM as usize])
        }).unwrap();
        assert_eq!(profile.input_digests.len(), 2);
        assert_eq!(observed.len(), 2);
        for row in &rows[0] {
            let key = required_string(row, "canonical_key").unwrap();
            let title = required_string(row, "title").unwrap();
            let body = crate::db::store::record_large_string(row, "body", 0).unwrap();
            let text = format!("{} {}", title, body.chars().take(2000).collect::<String>());
            assert_eq!(profile.input_digests[&key], hex_encode(&Sha256::digest(text.as_bytes())));
            let vector = crate::db::convert::extract_embedding(row, 0).unwrap();
            assert_eq!(vector, vec![text.len() as f32; EMBEDDING_DIM as usize]);
            assert_eq!(profile.vector_digests[&key], identity_vector_digest(&vector));
        }
        let mut report = verify_identity_v5_source(&source);
        drop(db);
        publish_prepared_identity_v5(&ws, &source, &rows, Some(profile.clone()), &mut report,
            |_, checked| { assert_eq!(checked, &profile); Ok(()) }).await.unwrap();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        assert_eq!(read_identity_v5_marker(&ws.join("lance")).unwrap().unwrap().embeddings, Some(profile.clone()));
        let after = table_rows(&db, "artifacts").await.unwrap();
        assert_eq!(old.len(), after.len());
        for (before, after) in old.iter().zip(after.iter()) {
            assert!(before.columns().iter().zip(after.columns()).all(|(a, b)| a.to_data() == b.to_data()));
        }
        let store = crate::db::store::LanceStore::open(&ws).await.unwrap();
        #[cfg(feature = "semantic-search")]
        {
            let rows = table_rows(&db, IDENTITY_V5_ARTIFACTS_TABLE).await.unwrap();
            let row = rows.iter().find(|row| required_string(row, "canonical_key").unwrap() == "prd-auth-system").unwrap();
            let mut record = store.get_record(&lock, "prd-auth-system").await.unwrap().unwrap();
            crate::db::store::verify_row_local_semantic_provenance(row, 0, &record, &profile).unwrap();
            record.title.push_str(" changed without recomputing");
            assert!(crate::db::store::verify_row_local_semantic_provenance(row, 0, &record, &profile).is_err());
        }
        store.update_artifact(&lock, "prd-auth-system", None, Some("New title")).await.unwrap();
        assert!(store.get_record(&lock, "prd-auth-system").await.unwrap().unwrap().embedding.is_none());
        let changed = table_rows(&db, IDENTITY_V5_ARTIFACTS_TABLE).await.unwrap();
        let changed = changed.iter().find(|row| required_string(row, "canonical_key").unwrap() == "prd-auth-system").unwrap();
        assert_eq!(crate::db::store::record_string(changed, "semantic_state", 0).as_deref(), Some("pending"));
        store.update_body(&lock, "RFC-001", "Changed body").await.unwrap();
        assert!(store.get_record(&lock, "RFC-001").await.unwrap().unwrap().embedding.is_none());
    }

    #[tokio::test]
    async fn mismatched_vector_fingerprint_blocks_staging_and_marker() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace_with_vector(tmp.path(), false, true).await;
        let source = read_identity_v5_source(&ws).await.unwrap();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let mut rows = prepare_identity_v5_rows(&db, &source, true).await.unwrap();
        let profile = IdentityV5EmbeddingProvenance {
            model: "test".into(), repository: "test/repo".into(), snapshot_id: "revision".into(),
            snapshot_digest: "a".repeat(64), chunk_size: 2000, dimension: EMBEDDING_DIM as usize,
            implementation: "deterministic-test-v1".into(), profile_digest: String::new(),
            input_digests: BTreeMap::new(), vector_digests: BTreeMap::new(),
        };
        let mut profile = recompute_identity_vectors(&mut rows[0], profile, |_| {
            Ok(vec![0.5; EMBEDDING_DIM as usize])
        }).unwrap();
        *profile.vector_digests.values_mut().next().unwrap() = "b".repeat(64);
        let mut report = verify_identity_v5_source(&source);
        drop(db);
        assert!(publish_prepared_identity_v5(&ws, &source, &rows, Some(profile), &mut report,
            |_, _| Ok(())).await.is_err());
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
        assert!(!db.table_names().execute().await.unwrap().contains(&IDENTITY_V5_ARTIFACTS_TABLE.to_string()));
    }

    #[tokio::test]
    async fn changed_model_after_staging_keeps_v4_and_retry_can_publish() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace_with_vector(tmp.path(), false, true).await;
        let source = read_identity_v5_source(&ws).await.unwrap();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let old = table_rows(&db, "artifacts").await.unwrap();
        let mut rows = prepare_identity_v5_rows(&db, &source, true).await.unwrap();
        let profile = IdentityV5EmbeddingProvenance {
            model: "test".into(), repository: "test/repo".into(), snapshot_id: "revision".into(),
            snapshot_digest: "a".repeat(64), chunk_size: 2000, dimension: EMBEDDING_DIM as usize,
            implementation: "deterministic-test-v1".into(), profile_digest: String::new(),
            input_digests: BTreeMap::new(), vector_digests: BTreeMap::new(),
        };
        let profile = recompute_identity_vectors(&mut rows[0], profile, |_| {
            Ok(vec![0.5; EMBEDDING_DIM as usize])
        }).unwrap();
        let mut report = verify_identity_v5_source(&source);
        drop(db);
        let error = publish_prepared_identity_v5(
            &ws, &source, &rows, Some(profile.clone()), &mut report,
            |_, _| anyhow::bail!("embedding snapshot changed"),
        ).await.unwrap_err();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        assert!(error.to_string().contains("embedding snapshot changed"));
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
        let after = table_rows(&db, "artifacts").await.unwrap();
        assert_eq!(old.len(), after.len());
        for (before, after) in old.iter().zip(&after) {
            assert!(before.columns().iter().zip(after.columns()).all(|(a, b)| a.to_data() == b.to_data()));
        }
        drop(db);
        publish_prepared_identity_v5(
            &ws, &source, &rows, Some(profile), &mut report, |_, _| Ok(()),
        ).await.unwrap();
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().unwrap().completed);
    }

    #[tokio::test]
    async fn invalid_recomputed_vector_never_publishes_or_changes_v4() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace_with_vector(tmp.path(), false, true).await;
        let source = read_identity_v5_source(&ws).await.unwrap();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let old = table_rows(&db, "artifacts").await.unwrap();
        let mut rows = prepare_identity_v5_rows(&db, &source, true).await.unwrap();
        let profile = IdentityV5EmbeddingProvenance {
            model: "test".into(), repository: "test/repo".into(), snapshot_id: "revision".into(),
            snapshot_digest: "a".repeat(64), chunk_size: 2000, dimension: EMBEDDING_DIM as usize,
            implementation: "deterministic-test-v1".into(), profile_digest: String::new(),
            input_digests: BTreeMap::new(), vector_digests: BTreeMap::new(),
        };
        let mut count = 0;
        assert!(recompute_identity_vectors(&mut rows[0], profile, |_| {
            count += 1;
            Ok(vec![if count == 1 { 0.5 } else { f32::NAN }; EMBEDDING_DIM as usize])
        }).is_err());
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
        let after = table_rows(&db, "artifacts").await.unwrap();
        assert!(old.iter().zip(after.iter()).all(|(a, b)| a.column_by_name("embedding").unwrap().to_data() == b.column_by_name("embedding").unwrap().to_data()));
    }

    #[tokio::test]
    async fn v4_vector_unavailable_backend_blocks_publication_and_is_preserved() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace_with_vector(tmp.path(), false, true).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        tokio::fs::write(ws.join("config.yaml"), "embedding:\n  model: unsupported-test-model\n").await.unwrap();
        let error = run_identity_v5_migration(&ws, &lock).await.unwrap_err();
        #[cfg(feature = "tract-engine")]
        assert!(error.to_string().contains("unsupported embedding model"));
        #[cfg(not(feature = "tract-engine"))]
        assert!(error.to_string().contains("embedding backend"));
        let refusal = crate::db::store::LanceStore::open_for_write(&ws, &lock).await.err().unwrap();
        assert_eq!(error.to_string(), refusal.to_string());
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
        let store = crate::db::store::LanceStore::open(&ws).await.unwrap();
        assert_eq!(store.get_record(&lock, "prd-auth-system").await.unwrap().unwrap().embedding.unwrap(),
            vec![0.25; EMBEDDING_DIM as usize]);
    }

    #[tokio::test]
    async fn v4_unmapped_row_blocks_even_when_document_counts_match() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), true).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let error = run_identity_v5_migration(&ws, &lock).await.unwrap_err();
        assert!(error.to_string().contains("cannot be mapped uniquely"));
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        assert_eq!(table_rows(&db, "artifacts").await.unwrap().len(), 2);
        assert_eq!(table_rows(&db, "evidence").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn v4_unmapped_evidence_reference_blocks_publication() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let table = db.open_table("evidence").execute().await.unwrap();
        table.update().only_if("id = 'EVID-001'").column("artifact_id", "'PRD-999'")
            .execute().await.unwrap();
        let error = run_identity_v5_migration(&ws, &lock).await.unwrap_err();
        assert!(error.to_string().contains("no matching document"));
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
        let old = table_rows(&db, "evidence").await.unwrap();
        assert_eq!(required_string(&old[0], "artifact_id").unwrap(), "PRD-999");
        assert_eq!(required_string(&old[0], "content").unwrap(), "original evidence");
    }

    #[tokio::test]
    async fn published_marker_with_missing_table_is_an_error_even_during_init() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        run_identity_v5_migration(&ws, &lock).await.unwrap();
        let opened = crate::db::store::LanceStore::open(&ws).await.unwrap();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        db.drop_table(IDENTITY_V5_EVIDENCE_TABLE, &[]).await.unwrap();
        assert!(opened.ensure_identity_v5_writable(&lock).is_err());
        assert!(run_identity_v5_migration(&ws, &lock).await.is_err());
        assert!(crate::db::store::LanceStore::open(&ws).await.is_err());
        assert!(crate::db::store::LanceStore::init(&ws, &lock).await.is_err());
    }
    #[tokio::test]
    async fn stage_verifier_rejects_missing_evidence_row() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let source = read_identity_v5_source(&ws).await.unwrap();
        let mut report = verify_identity_v5_source(&source);
        write_identity_v5_tables(&ws.join("lance"), &lock, &source, &mut report).await.unwrap();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        db.open_table(IDENTITY_V5_EVIDENCE_TABLE).execute().await.unwrap()
            .delete("artifact_id = 'prd-auth-system'").await.unwrap();
        assert!(verify_identity_v5_round_trip(&ws.join("lance"), &source).await.is_err());
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
    }

    #[tokio::test]
    async fn invalid_source_encoding_never_publishes() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        tokio::fs::write(ws.join("prds/bad.md"), [0xff]).await.unwrap();
        assert!(run_identity_v5_migration(&ws, &lock).await.unwrap_err().to_string().contains("UTF-8"));
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn delete_intent_recovers_file_first_gap_without_deleting_evidence() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::open_for_write(&ws, &lock).await.unwrap();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let old_v4 = table_rows(&db, "artifacts").await.unwrap();
        let old_evidence = table_rows(&db, IDENTITY_V5_EVIDENCE_TABLE).await.unwrap();
        let path = ws.join("rfcs/RFC-001-wire.md");
        begin_identity_delete_intent(&ws, &lock, "RFC-001", &path).await.unwrap();
        begin_identity_delete_intent(&ws, &lock, "RFC-001", &path).await.unwrap();
        assert!(has_identity_delete_intents(&ws).unwrap());
        assert!(require_identity_delete_intent(&ws, &lock, "RFC-001").await.is_err());
        tokio::fs::remove_file(&path).await.unwrap();
        require_identity_delete_intent(&ws, &lock, "RFC-001").await.unwrap();
        assert!(require_identity_delete_intent(&ws, &lock, "prd-auth-system").await.is_err());
        drop(store);
        drop(lock);
        crate::db::store::LanceStore::drain_identity_semantic_backlog(&ws).await.unwrap();
        let lock = crate::workspace::lock::acquire_workspace_read_lock(&ws).await.unwrap();
        let recovered = crate::db::store::LanceStore::open(&ws).await.unwrap();
        assert!(recovered.get_record(&lock, "RFC-001").await.unwrap().is_none());
        assert_eq!(recovered.get_record(&lock, "prd-auth-system").await.unwrap().unwrap().id, "PRD-001");
        assert!(!has_identity_delete_intents(&ws).unwrap());
        let new_evidence = table_rows(&db, IDENTITY_V5_EVIDENCE_TABLE).await.unwrap();
        assert_eq!(old_evidence.len(), new_evidence.len());
        assert!(old_evidence[0].columns().iter().zip(new_evidence[0].columns()).all(|(a, b)| a.to_data() == b.to_data()));
        let new_v4 = table_rows(&db, "artifacts").await.unwrap();
        assert_eq!(old_v4.len(), new_v4.len());
        assert!(old_v4.iter().zip(&new_v4).all(|(left, right)|
            left.columns().iter().zip(right.columns()).all(|(a, b)| a.to_data() == b.to_data())));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unrecorded_orphan_does_not_get_deleted_by_reconciliation() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::open_for_write(&ws, &lock).await.unwrap();
        tokio::fs::remove_file(ws.join("rfcs/RFC-001-wire.md")).await.unwrap();
        assert!(require_identity_delete_intent(&ws, &lock, "RFC-001").await.is_err());
        drop(store);
        drop(lock);
        assert!(crate::db::store::LanceStore::drain_identity_semantic_backlog(&ws).await.is_err());
        let lock = crate::workspace::lock::acquire_workspace_read_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::open(&ws).await.unwrap();
        assert!(store.get_record(&lock, "RFC-001").await.unwrap().is_some());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn every_fixed_name_swap_window_recovers_a_file_first_create() {
        for cut in 0..=6 {
            let tmp = TempDir::new().unwrap();
            let ws = populated_v4_workspace(tmp.path(), false).await;
            let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
            let old = crate::db::store::LanceStore::open_for_write(&ws, &lock).await.unwrap();
            let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
            let before = table_rows(&db, IDENTITY_V5_EVIDENCE_TABLE).await.unwrap();
            tokio::fs::write(ws.join("prds/PRD-002-new.md"),
                "---\nid: PRD-002\nkind: prd\nstatus: draft\ntitle: New\nslug: prd-new\n---\n\nNew content.\n").await.unwrap();
            let source = read_identity_v5_source(&ws).await.unwrap();
            let transaction = prepare_legacy_semantic_rebuild(&ws, &lock, &db, &source).await.unwrap();
            drop(lock);
            assert!(crate::workspace::lock::acquire_workspace_lock(&ws).await.is_err());
            let owner = crate::workspace::lock::acquire_legacy_semantic_rebuild_lock(&ws, &transaction).await.unwrap();
            let dir = legacy_semantic_rebuild_root(&ws).join(&transaction);
            let names = [IDENTITY_V5_ARTIFACTS_TABLE, IDENTITY_V5_RELATIONS_TABLE, IDENTITY_V5_EVIDENCE_TABLE];
            for (step, name) in names.iter().enumerate() {
                let filename = format!("{name}.lance");
                if step * 2 < cut {
                    std::fs::rename(ws.join("lance").join(&filename), dir.join("backup").join(&filename)).unwrap();
                }
                if step * 2 + 1 < cut {
                    std::fs::rename(dir.join("staged").join(&filename), ws.join("lance").join(&filename)).unwrap();
                }
            }
            drop(owner);
            crate::db::store::LanceStore::drain_identity_semantic_backlog(&ws).await.unwrap();
            let read = crate::workspace::lock::acquire_workspace_read_lock(&ws).await.unwrap();
            assert!(old.identity_generation_changed(&read).unwrap());
            let fresh = crate::db::store::LanceStore::open(&ws).await.unwrap();
            assert_eq!(fresh.get_record(&read, "prd-new").await.unwrap().unwrap().id, "PRD-002");
            let after = table_rows(&db, IDENTITY_V5_EVIDENCE_TABLE).await.unwrap();
            assert_eq!(before.len(), after.len());
            assert!(before[0].columns().iter().zip(after[0].columns()).all(|(a, b)| a.to_data() == b.to_data()));
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn fixed_table_swap_revalidates_after_each_transition() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let tmp = TempDir::new().unwrap();
        let lance = tmp.path().join("lance");
        let staged = tmp.path().join("staged");
        let backup = tmp.path().join("backup");
        for dir in [&lance, &staged, &backup] { std::fs::create_dir(dir).unwrap(); }
        assert!(rename_identity_noreplace(&lance, &staged).is_err());
        assert!(lance.is_dir() && staged.is_dir());
        let names = [IDENTITY_V5_ARTIFACTS_TABLE, IDENTITY_V5_RELATIONS_TABLE, IDENTITY_V5_EVIDENCE_TABLE];
        let mut entries = Vec::new();
        for name in names {
            let filename = format!("{name}.lance");
            let old = lance.join(&filename);
            let new = staged.join(&filename);
            std::fs::create_dir(&old).unwrap();
            std::fs::create_dir(&new).unwrap();
            std::fs::write(old.join("content"), format!("old {name}")).unwrap();
            std::fs::write(new.join("content"), format!("new {name}")).unwrap();
            entries.push(FixedTableSwapEntry {
                name: name.into(),
                before_digest: Some(semantic_rebuild_tree_digest(&old).unwrap()),
                after_digest: semantic_rebuild_tree_digest(&new).unwrap(),
            });
        }
        let calls = AtomicUsize::new(0);
        let error = complete_fixed_table_swap(tmp.path(), &staged, &backup, &entries, || {
            if calls.fetch_add(1, Ordering::SeqCst) == 3 { anyhow::bail!("owner proof changed") }
            Ok(())
        }).unwrap_err();
        assert!(error.to_string().contains("owner proof changed"));
        assert!(!lance.join(format!("{}.lance", names[0])).exists());
        assert!(backup.join(format!("{}.lance", names[0])).exists());
        complete_fixed_table_swap(tmp.path(), &staged, &backup, &entries, || Ok(())).unwrap();
        for name in names {
            assert_eq!(std::fs::read_to_string(lance.join(format!("{name}.lance/content"))).unwrap(), format!("new {name}"));
            assert_eq!(std::fs::read_to_string(backup.join(format!("{name}.lance/content"))).unwrap(), format!("old {name}"));
        }
    }

    #[tokio::test]
    async fn exported_test_fixture_is_populated_schema_four() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_identity_v4_workspace_for_test(tmp.path()).await.unwrap();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let table = db.open_table("artifacts").execute().await.unwrap();
        assert_eq!(table.schema().await.unwrap().fields().len(), 15);
        assert_eq!(table_rows(&db, "artifacts").await.unwrap().len(), 2);
        assert_eq!(table_rows(&db, "relations").await.unwrap().len(), 1);
        assert_eq!(table_rows(&db, "evidence").await.unwrap().len(), 1);
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
    }

    #[tokio::test]
    async fn exported_vector_fixture_preserves_real_schema_four_vectors() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_identity_v4_workspace_with_vector_for_test(tmp.path()).await.unwrap();
        let db = lancedb::connect(ws.join("lance").to_str().unwrap()).execute().await.unwrap();
        let rows = table_rows(&db, "artifacts").await.unwrap();
        assert_eq!(rows.len(), 2);
        for row in &rows {
            assert_eq!(row.schema().fields().len(), 15);
            assert_eq!(crate::db::convert::extract_embedding(row, 0).unwrap(), vec![0.25; EMBEDDING_DIM as usize]);
        }
        assert!(read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
    }

    #[tokio::test]
    async fn frozen_capture_timestamp_is_reused_without_changing_source_digest() {
        let tmp = TempDir::new().unwrap();
        let ws = populated_v4_workspace(tmp.path(), false).await;
        let frozen = "2024-01-02T03:04:05+00:00";
        let first = read_identity_v5_source_at(&ws, frozen).await.unwrap();
        let replay = read_identity_v5_source_at(&ws, frozen).await.unwrap();
        assert_eq!(first.documents, replay.documents);
        assert_eq!(first.source_digest, replay.source_digest);
        assert!(first.documents.iter().all(|document| document.created_at == frozen && document.updated_at == frozen));
        assert!(read_identity_v5_source_at(&ws, "not-a-time").await.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn init_rejects_symlinked_lance_without_touching_target() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join("workspace");
        let external = tmp.path().join("external");
        std::fs::create_dir(&ws).unwrap();
        std::fs::create_dir(&external).unwrap();
        std::os::unix::fs::symlink(&external, ws.join("lance")).unwrap();
        let lock = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
        assert!(crate::db::store::LanceStore::init_new(&ws, &lock).await.is_err());
        assert!(std::fs::read_dir(external).unwrap().next().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn marker_symlink_is_neither_followed_nor_overwritten() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("must-not-change");
        std::fs::write(&target, "untouched").unwrap();
        std::os::unix::fs::symlink(&target, identity_v5_marker_path(tmp.path())).unwrap();
        let marker = IdentityV5Marker {
            version: IDENTITY_V5_VERSION, artifacts_table: IDENTITY_V5_ARTIFACTS_TABLE.into(),
            relations_table: IDENTITY_V5_RELATIONS_TABLE.into(), evidence_table: IDENTITY_V5_EVIDENCE_TABLE.into(),
            source_digest: "a".repeat(64), document_count: 0, completed: true, embeddings: None,
            semantic: Some(IdentityV5SemanticProtocol { protocol: 1, active_profile_digest: None }),
        };
        assert!(read_identity_v5_marker(tmp.path()).is_err());
        assert!(publish_identity_v5_marker_new(tmp.path(), &marker).is_err());
        assert!(publish_identity_v5_marker(tmp.path(), &marker).is_err());
        assert_eq!(std::fs::read_to_string(target).unwrap(), "untouched");
    }

    #[cfg(unix)]
    #[test]
    fn first_marker_publication_never_replaces_existing_file() {
        let tmp = TempDir::new().unwrap();
        let marker = IdentityV5Marker {
            version: IDENTITY_V5_VERSION, artifacts_table: IDENTITY_V5_ARTIFACTS_TABLE.into(),
            relations_table: IDENTITY_V5_RELATIONS_TABLE.into(), evidence_table: IDENTITY_V5_EVIDENCE_TABLE.into(),
            source_digest: "a".repeat(64), document_count: 0, completed: true, embeddings: None,
            semantic: Some(IdentityV5SemanticProtocol { protocol: 1, active_profile_digest: None }),
        };
        publish_identity_v5_marker_new(tmp.path(), &marker).unwrap();
        let original = std::fs::read(identity_v5_marker_path(tmp.path())).unwrap();
        let mut competing = marker;
        competing.source_digest = "b".repeat(64);
        assert!(publish_identity_v5_marker_new(tmp.path(), &competing).is_err());
        assert_eq!(std::fs::read(identity_v5_marker_path(tmp.path())).unwrap(), original);
    }

    #[test]
    fn marker_with_wrong_table_name_or_digest_is_damaged() {
        let tmp = TempDir::new().unwrap();
        let marker = IdentityV5Marker {
            version: IDENTITY_V5_VERSION,
            artifacts_table: "artifacts".into(),
            relations_table: IDENTITY_V5_RELATIONS_TABLE.into(),
            evidence_table: IDENTITY_V5_EVIDENCE_TABLE.into(),
            source_digest: "a".repeat(64),
            document_count: 0,
            completed: true,
            embeddings: None,
            semantic: None,
        };
        publish_identity_v5_marker_new(tmp.path(), &marker).unwrap();
        assert!(read_identity_v5_marker(tmp.path()).is_err());
        let mut marker = marker;
        marker.artifacts_table = IDENTITY_V5_ARTIFACTS_TABLE.into();
        marker.source_digest = "bad".into();
        publish_identity_v5_marker(tmp.path(), &marker).unwrap();
        assert!(read_identity_v5_marker(tmp.path()).is_err());
    }
}
