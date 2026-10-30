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

use arrow_array::{Int32Array, RecordBatch, StringArray};
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

    // Migration 4→5: add the canonical_key column (ADR-030).
    //
    // The column is the row's identity, so the in-place upgrade fills it
    // with NULLs rather than guessing: only the Markdown knows whether a
    // document declares a slug, and the staged identity-v5 migration rebuilds
    // the index from the Markdown for exactly that reason. Until that
    // migration publishes, the pre-migration rows keep their own bytes — the
    // index that is preserved.
    if !field_names.contains(&"canonical_key".to_string()) {
        eprintln!("[migrate] Adding canonical_key column to artifacts");
        use arrow_schema::{DataType, Field, Schema};
        let key_schema = Arc::new(Schema::new(vec![Field::new(
            "canonical_key",
            DataType::Utf8,
            true,
        )]));
        table
            .add_columns(NewColumnTransform::AllNulls(key_schema), None)
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

/// Ensure the change_log table exists. Called from LanceStore::open().
/// Unlike other migrations, this creates the table if missing (since older workspaces
/// don't have it). The table handle is passed as Option — None means "not found".
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
            ],
        )
        .map_err(|e: ArrowError| anyhow::anyhow!("Failed to create change_log batch: {}", e))?;
        db.create_table("change_log", vec![batch]).execute().await?;
        eprintln!("[migrate] Created change_log table");
    }
    Ok(())
}

/// Migrate existing change_log table to add commit_hash column (v2→v3).
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
pub const IDENTITY_V5_SOURCE_DIRS: &[&str] = &[
    "prds",
    "epics",
    "specs",
    "rfcs",
    "adrs",
    "problems",
    "solutions",
    "evidence",
    "notes",
    "refresh",
    "memory",
    "use_cases",
    "glossary",
    "invariants",
    "scenarios",
    "hypotheses",
    "domain_models",
];

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
fn hex_encode(bytes: &[u8]) -> String {
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
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            anyhow::bail!(
                "cannot read the identity-v5 readiness marker {}: {err}",
                path.display()
            );
        }
    };
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
    Ok(Some(marker))
}

/// Publish the identity-v5 index by writing its readiness marker.
///
/// The marker is written to a temporary file, fsynced, renamed over the final
/// name, and the directory is fsynced: a reader sees either the old marker
/// or the new one, never a partial file.
pub fn publish_identity_v5_marker(
    lance_dir: &Path,
    marker: &IdentityV5Marker,
) -> anyhow::Result<()> {
    let path = identity_v5_marker_path(lance_dir);
    let tmp = lance_dir.join(format!("{IDENTITY_V5_MARKER_FILE}.tmp"));
    let bytes = serde_json::to_vec_pretty(marker)?;
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(&bytes)?;
        file.flush()?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, &path)?;
    // The rename is durable only once the directory entry is on disk.
    let dir = std::fs::File::open(lance_dir)?;
    dir.sync_all()?;
    Ok(())
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
fn parse_identity_v5_document(
    relative_path: &str,
    text: &str,
) -> anyhow::Result<IdentityV5Document> {
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
    let now = chrono::Utc::now().to_rfc3339();
    let created_at = frontmatter_string(&frontmatter, "created_at")
        .or_else(|| frontmatter_string(&frontmatter, "created"))
        .unwrap_or_else(|| now.clone());
    let updated_at = frontmatter_string(&frontmatter, "updated_at")
        .or_else(|| frontmatter_string(&frontmatter, "updated"))
        .unwrap_or(now);

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

/// Read every document of the workspace from the Markdown.
///
/// The Markdown is the only place that knows whether a document declares a
/// slug, so the staged index is built from here and never from the rows of
/// the pre-migration index.
pub async fn read_identity_v5_source(workspace: &Path) -> anyhow::Result<IdentityV5Source> {
    use sha2::{Digest, Sha256};

    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for dir in IDENTITY_V5_SOURCE_DIRS {
        let path = workspace.join(dir);
        let mut entries = match tokio::fs::read_dir(&path).await {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(anyhow::anyhow!("cannot read {}: {err}", path.display())),
        };
        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            if entry_path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            let bytes = tokio::fs::read(&entry_path).await?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            files.push((format!("{dir}/{file_name}"), bytes));
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));

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
        let text = String::from_utf8_lossy(bytes);
        match parse_identity_v5_document(relative, &text) {
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

    // The references: every target that names a document of this workspace
    // must name exactly one.
    let keys: std::collections::HashSet<&str> = source
        .documents
        .iter()
        .map(|document| document.canonical_key.as_str())
        .collect();
    let aliases: std::collections::HashSet<&str> = source
        .documents
        .iter()
        .map(|document| document.display_id.as_str())
        .collect();
    for document in &source.documents {
        for (target, _relation) in &document.links {
            if target.contains('@') {
                // A qualified cross-source reference is not a local selector.
                continue;
            }
            let known = keys.contains(target.as_str())
                || aliases.contains(target.as_str())
                || keys.contains(target.to_ascii_lowercase().as_str());
            if !known {
                report.unresolved_references.push(format!(
                    "{} -> {}",
                    document.relative_path, target
                ));
            }
        }
    }
    report
}

/// An empty evidence batch, shaped like the evidence schema.
fn empty_evidence_batch() -> anyhow::Result<RecordBatch> {
    let schema = super::schema::evidence_schema();
    Ok(RecordBatch::try_new(
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
    )?)
}

/// Write the three staged tables of the identity-v5 index.
///
/// The old tables are not touched, and `fpf_spec` and `change_log` are not
/// duplicated. Re-running before the marker exists rebuilds only these three
/// tables.
pub async fn write_identity_v5_tables(
    lance_dir: &Path,
    source: &IdentityV5Source,
    report: &mut IdentityV5Report,
) -> anyhow::Result<()> {
    let db = lancedb::connect(
        lance_dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("LanceDB path contains non-UTF-8 characters"))?,
    )
    .execute()
    .await?;

    let existing = db.table_names().execute().await?;
    for table in [
        IDENTITY_V5_ARTIFACTS_TABLE,
        IDENTITY_V5_RELATIONS_TABLE,
        IDENTITY_V5_EVIDENCE_TABLE,
    ] {
        if existing.iter().any(|name| name == table) {
            db.drop_table(table, &[]).await?;
        }
    }

    // The artifacts table: one row per document, keyed by its canonical key.
    let schema = super::schema::artifacts_schema();
    let mut rows: Vec<RecordBatch> = Vec::new();
    for document in &source.documents {
        let artifact = crate::db::store::NewArtifact {
            canonical_key: document.canonical_key.clone(),
            id: document.display_id.clone(),
            kind: document.kind.clone(),
            status: document.status.clone(),
            title: document.title.clone(),
            body: document.body.clone(),
            depth: document.depth.clone(),
            author: document.author.clone(),
            parent_epic: document.parent_epic.clone(),
            valid_until: document.valid_until.clone(),
            tags: document.tags.clone(),
        };
        rows.push(crate::db::convert::artifact_to_batch(&artifact, &document.created_at)?);
    }
    let artifacts_table = match rows.split_first() {
        Some((first, rest)) => {
            let table = db
                .create_table(IDENTITY_V5_ARTIFACTS_TABLE, vec![first.clone()])
                .execute()
                .await?;
            for row in rest {
                table.add(vec![row.clone()]).execute().await?;
            }
            table
        }
        None => {
            db.create_table(
                IDENTITY_V5_ARTIFACTS_TABLE,
                vec![RecordBatch::new_empty(schema.clone())],
            )
            .execute()
            .await?
        }
    };
    drop(artifacts_table);

    // The relations table: the endpoints hold canonical keys.
    let relation_schema = super::schema::relations_schema();
    let mut relation_rows: Vec<RecordBatch> = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();
    for document in &source.documents {
        for (target, relation) in &document.links {
            let target_key = resolve_reference(source, target);
            relation_rows.push(crate::db::convert::relation_to_batch(
                &document.canonical_key,
                &target_key,
                relation,
                &now,
            )?);
        }
    }
    match relation_rows.split_first() {
        Some((first, rest)) => {
            let table = db
                .create_table(IDENTITY_V5_RELATIONS_TABLE, vec![first.clone()])
                .execute()
                .await?;
            for row in rest {
                table.add(vec![row.clone()]).execute().await?;
            }
        }
        None => {
            db.create_table(
                IDENTITY_V5_RELATIONS_TABLE,
                vec![RecordBatch::new_empty(relation_schema.clone())],
            )
            .execute()
            .await?;
        }
    }

    // The evidence table: the legacy evidence table holds no rows (evidence
    // lives in ordinary artifacts), so the staged table is created empty
    // rather than silently dropping rows that never existed.
    let evidence_batch = empty_evidence_batch()?;
    db.create_table(IDENTITY_V5_EVIDENCE_TABLE, vec![evidence_batch])
        .execute()
        .await?;

    report.relation_count = source
        .documents
        .iter()
        .map(|document| document.links.len())
        .sum();
    report.evidence_count = 0;
    Ok(())
}

/// The canonical key a declared reference names, or its own bytes when it
/// names no document of this workspace.
fn resolve_reference(source: &IdentityV5Source, target: &str) -> String {
    if target.contains('@') {
        return target.to_string();
    }
    for document in &source.documents {
        if document.canonical_key == target || document.display_id == target {
            return document.canonical_key.clone();
        }
    }
    target.to_string()
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
    let db = lancedb::connect(
        lance_dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("LanceDB path contains non-UTF-8 characters"))?,
    )
    .execute()
    .await?;
    let table = db
        .open_table(IDENTITY_V5_ARTIFACTS_TABLE)
        .execute()
        .await?;
    let batches = crate::db::store::collect_batches(table.query().execute().await?).await?;

    let mut rows = 0usize;
    for batch in &batches {
        for row in 0..batch.num_rows() {
            rows += 1;
            let key = crate::db::store::record_string(batch, "canonical_key", row)
                .ok_or_else(|| anyhow::anyhow!("a staged row carries no canonical key"))?;
            let source_document = source
                .documents
                .iter()
                .find(|document| document.canonical_key == key)
                .ok_or_else(|| {
                    anyhow::anyhow!("the staged index holds the unknown key '{key}'")
                })?;
            let id = crate::db::store::record_string(batch, "id", row).unwrap_or_default();
            if id != source_document.display_id {
                anyhow::bail!(
                    "the staged row for '{key}' carries the display id '{id}' but the source declares '{}'",
                    source_document.display_id
                );
            }
            let body = crate::db::store::record_large_string(batch, "body", row)
                .unwrap_or_default();
            if body != source_document.body {
                anyhow::bail!(
                    "the staged row for '{key}' does not carry the body the Markdown holds"
                );
            }
        }
    }
    if rows != source.documents.len() {
        anyhow::bail!(
            "the staged index holds {rows} rows but the Markdown holds {} documents",
            source.documents.len()
        );
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

/// Build the identity-v5 index from the Markdown, verify it, and publish it.
///
/// The caller holds the workspace lock. Before the marker exists a re-run
/// rebuilds only the three staged tables; after it, the staged set is already
/// live and this call is a no-op that reports the published state.
pub async fn run_identity_v5_migration(workspace: &Path) -> anyhow::Result<IdentityV5Report> {
    let lance_dir = workspace.join("lance");
    tokio::fs::create_dir_all(&lance_dir).await?;

    if let Some(marker) = read_identity_v5_marker(&lance_dir)? {
        // Already published: only the v5 set is used from here on.
        return Ok(IdentityV5Report {
            document_count: marker.document_count,
            source_digest: marker.source_digest,
            published: true,
            already_published: true,
            ..Default::default()
        });
    }

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

    write_identity_v5_tables(&lance_dir, &source, &mut report).await?;
    verify_identity_v5_round_trip(&lance_dir, &source).await?;

    let marker = IdentityV5Marker {
        version: IDENTITY_V5_VERSION,
        artifacts_table: IDENTITY_V5_ARTIFACTS_TABLE.to_string(),
        relations_table: IDENTITY_V5_RELATIONS_TABLE.to_string(),
        evidence_table: IDENTITY_V5_EVIDENCE_TABLE.to_string(),
        source_digest: report.source_digest.clone(),
        document_count: report.document_count,
        completed: true,
    };
    publish_identity_v5_marker(&lance_dir, &marker)?;
    report.published = true;
    Ok(report)
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
        publish_identity_v5_marker(
            &lance_dir,
            &IdentityV5Marker {
                version: IDENTITY_V5_VERSION,
                artifacts_table: IDENTITY_V5_ARTIFACTS_TABLE.to_string(),
                relations_table: IDENTITY_V5_RELATIONS_TABLE.to_string(),
                evidence_table: IDENTITY_V5_EVIDENCE_TABLE.to_string(),
                source_digest: "abc".to_string(),
                document_count: 1,
                completed: false,
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
            source_digest: "digest".to_string(),
            document_count: 2,
            completed: true,
        };
        publish_identity_v5_marker(&lance_dir, &marker).unwrap();

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
    async fn the_staged_index_is_built_from_the_markdown_and_published() {
        let tmp = TempDir::new().unwrap();
        let ws = identity_v5_workspace(tmp.path()).await;

        let report = run_identity_v5_migration(&ws).await.unwrap();
        assert!(report.published, "the verified set must be published");
        assert!(!report.already_published);
        assert_eq!(report.document_count, 2, "both documents must be indexed");
        assert_eq!(report.relation_count, 1, "the declared link must be indexed");

        let lance_dir = ws.join("lance");
        let marker = read_identity_v5_marker(&lance_dir).unwrap().unwrap();
        assert!(marker.completed);
        assert_eq!(marker.document_count, 2);
        assert_eq!(marker.source_digest, report.source_digest);

        // The three staged tables exist, and the pre-migration ones are not
        // duplicated into the staged set: `fpf_spec` and `change_log` are not
        // duplicated at all.
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
            1,
            "only the staged artifacts table may exist after the migration"
        );

        // The store now reads the staged set: the row is addressed by the
        // key the Markdown declares.
        let store = crate::db::store::LanceStore::open(&ws).await.unwrap();
        let record = store
            .get_record("prd-auth-system")
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
        let ws = identity_v5_workspace(tmp.path()).await;
        run_identity_v5_migration(&ws).await.unwrap();

        let store = crate::db::store::LanceStore::open(&ws).await.unwrap();
        let record = store
            .get_record("RFC-001")
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
        let ws = identity_v5_workspace(tmp.path()).await;
        tokio::fs::write(
            ws.join("prds/PRD-002-alpha.md"),
            "---\nid: PRD-002\nkind: prd\nstatus: draft\ntitle: Alpha\nslug: prd-auth-system\n---\n\nAlpha body.\n",
        )
        .await
        .unwrap();

        let error = run_identity_v5_migration(&ws)
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
        let ws = identity_v5_workspace(tmp.path()).await;
        let first = run_identity_v5_migration(&ws).await.unwrap();
        assert!(first.published);

        let second = run_identity_v5_migration(&ws).await.unwrap();
        assert!(second.already_published, "the second run must be a no-op");
        assert!(second.published, "the set stays published");
        assert_eq!(second.document_count, first.document_count);
    }
}
