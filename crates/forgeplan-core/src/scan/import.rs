use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context as _;

use crate::artifact::types::ArtifactKind;
use crate::db::store::{LanceStore, NewArtifact};
use crate::scan::detect::{DetectionResult, DetectionTier, detect_kind_with_path};
use crate::scan::discovery::{DiscoveredFile, discover_markdown_files};
use crate::scan::status_map::map_external_status;

/// Author recorded for a document whose frontmatter declares none. Keeps the
/// import auditable ("this row came from a scan") without overwriting a
/// human-supplied `author:` (PROB-068).
const SCAN_IMPORT_AUTHOR: &str = "scan-import";

/// Options for scan-import operation.
#[derive(Debug, Clone, Default)]
pub struct ScanImportOptions {
    /// If true, only show what would be imported without making changes.
    pub dry_run: bool,
    /// Custom path to scan (overrides default doc directories).
    pub custom_path: Option<String>,
}

/// Status of a single file during import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportStatus {
    /// Written as canonical Markdown (and, where one exists, into the
    /// optional legacy projection).
    Imported,
    /// The canonical file already exists — an earlier run imported it, or the
    /// artifact was created by hand. The existing document is never rewritten.
    Skipped,
    /// Could not determine artifact type.
    Unknown,
    /// Failed to import (with error message).
    Failed(String),
}

/// Entry in the scan-import result.
#[derive(Debug, Clone)]
pub struct ScanImportEntry {
    /// Relative path of the source file.
    pub relative_path: String,
    /// Detected artifact kind (if any).
    pub detected_kind: Option<ArtifactKind>,
    /// Detection tier used.
    pub detection_tier: Option<DetectionTier>,
    /// Assigned artifact ID.
    pub artifact_id: Option<String>,
    /// Import status.
    pub status: ImportStatus,
    /// Non-fatal warnings (PRD-058 FR-004): unknown frontmatter status,
    /// canonical write failure, a derived relation that could not be restored.
    /// Fail-loud — the import report surfaces these so the user knows
    /// something needs attention.
    #[doc(alias = "advisory")]
    pub warnings: Vec<String>,
}

/// A set of source files on disk that all carry the SAME artifact id — a
/// file-level id collision. Distinct from the title-similarity
/// `AnomalyKind::DuplicateArtifact`, which works on the DB where two same-id
/// files have already been collapsed into one row. Two `.md` with one id
/// desync the resolver (status read from one file, body from another), so this
/// surfaces them for cleanup. Origin is usually a legacy artifact (pre-Phase-1.5
/// id-assignment) plus a hand-authored file that bypassed `create_artifact`'s
/// dup-id guard (a RED-LINE #11 violation predating enforcement).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateIdGroup {
    /// The artifact id shared by all `paths`.
    pub id: String,
    /// Relative paths of every file resolving to `id` (sorted, ≥2).
    pub paths: Vec<String>,
}

/// Aggregate result of scan-import operation.
#[derive(Debug, Clone)]
pub struct ScanImportResult {
    pub entries: Vec<ScanImportEntry>,
    pub total_found: usize,
    pub imported: usize,
    pub skipped: usize,
    pub unknown: usize,
    pub failed: usize,
    /// File-level id collisions found during the scan (two+ files → one id).
    /// Empty in the healthy case. Reported so the user can delete the stale
    /// duplicate before the resolver returns desynced status/body.
    pub duplicate_ids: Vec<DuplicateIdGroup>,
}

/// Group scan entries by their resolved artifact id and return every id that
/// maps to more than one source file. Deterministic order (BTreeMap by id,
/// paths sorted) so the report and tests are stable.
pub fn find_duplicate_ids(entries: &[ScanImportEntry]) -> Vec<DuplicateIdGroup> {
    let mut by_id: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for e in entries {
        if let Some(id) = &e.artifact_id {
            by_id
                .entry(id.clone())
                .or_default()
                .push(e.relative_path.clone());
        }
    }
    by_id
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(id, mut paths)| {
            paths.sort();
            DuplicateIdGroup { id, paths }
        })
        .collect()
}

/// Canonical-first scan-import: discover external documents, detect their
/// kind, and write each accepted one into `.forgeplan/` as canonical Markdown.
///
/// `legacy` is the OPTIONAL derived projection:
///
/// - `Some(store)` — a workspace that still owns a legacy `artifacts.lance`.
///   The Markdown is written first and the row second; a row failure is
///   reported, never repaired by rolling the file back.
/// - `None` — a constellation-managed workspace. The Markdown write is the
///   whole mutation and the caller runs one reconcile afterwards
///   (`maintain_index` / `forgeplan index`). No `LanceStore` is opened and no
///   retired table is recreated.
///
/// Ordering per file: resolve id → check existence → canonical write →
/// optional projection. The file is never derived from the store, and the
/// store never becomes a precondition for the file existing.
///
/// `workspace` is the `.forgeplan/` directory, not the project root.
///
/// `guard` is the caller's already-held
/// [`WorkspaceLock`](crate::workspace::lock::WorkspaceLock), and it is `None`
/// **only** for a dry run. A dry run must not take the lock: acquiring it
/// creates `.forgeplan/.lock`, so a preview would mutate a workspace it claims
/// not to touch. A mutating run with no guard is refused here rather than
/// writing canonical Markdown outside the workspace serialization every other
/// mutation uses. The caller must release the guard before reconciling, since
/// the reconcile re-acquires it per storage commit.
pub async fn scan_and_import_to_workspace(
    project_root: &Path,
    workspace: &Path,
    guard: Option<&crate::workspace::lock::WorkspaceLock>,
    legacy: Option<&LanceStore>,
    options: &ScanImportOptions,
) -> anyhow::Result<ScanImportResult> {
    if !options.dry_run && guard.is_none() {
        anyhow::bail!(
            "scan-import needs the workspace lock for a mutating run; a dry run may pass none"
        );
    }
    // Discover files — with path traversal protection
    let scan_root = if let Some(custom) = &options.custom_path {
        let candidate = project_root.join(custom);
        let canonical = candidate.canonicalize().unwrap_or(candidate.clone());
        let canonical_root = project_root
            .canonicalize()
            .unwrap_or(project_root.to_path_buf());
        if !canonical.starts_with(&canonical_root) {
            anyhow::bail!(
                "Scan path '{}' is outside project root. Path traversal rejected.",
                custom
            );
        }
        candidate
    } else {
        project_root.to_path_buf()
    };

    let files = discover_markdown_files(&scan_root)?;
    let total_found = files.len();

    let mut entries = Vec::with_capacity(total_found);
    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut unknown = 0usize;
    let mut failed = 0usize;

    for file in &files {
        let filename = file
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let detection = detect_kind_with_path(filename, Some(&file.relative_path), &file.content);

        let entry = match detection {
            Some(det) => {
                process_detected_file(file, &det, workspace, guard, legacy, options.dry_run).await
            }
            None => {
                unknown += 1;
                ScanImportEntry {
                    relative_path: file.relative_path.display().to_string(),
                    detected_kind: None,
                    detection_tier: None,
                    artifact_id: None,
                    status: ImportStatus::Unknown,
                    warnings: Vec::new(),
                }
            }
        };

        match entry.status {
            ImportStatus::Imported => imported += 1,
            ImportStatus::Skipped => skipped += 1,
            ImportStatus::Failed(_) => failed += 1,
            ImportStatus::Unknown => {} // already counted
        }

        entries.push(entry);
    }

    // File-level id-collision detection (PRD-008 dup class): surface any id
    // that resolves from more than one source file on disk.
    let duplicate_ids = find_duplicate_ids(&entries);

    Ok(ScanImportResult {
        entries,
        total_found,
        imported,
        skipped,
        unknown,
        failed,
        duplicate_ids,
    })
}

/// What one source document contributes to the canonical artifact.
struct ParsedDocument {
    /// Complete frontmatter map, unknown keys included. Preserved verbatim on
    /// the canonical write so an imported document keeps whatever its author
    /// put there (agent stamps, `last_modified_by`, domain keys).
    frontmatter: BTreeMap<String, serde_yaml::Value>,
    /// Body with the frontmatter block removed — what the artifact body is.
    body: String,
    title: String,
    status: String,
    tags: Vec<String>,
    author: String,
    links: Vec<(String, String)>,
}

/// Parse one discovered document into the fields the canonical write needs,
/// splitting frontmatter from body exactly once (R2 audit rust-pro CRITICAL).
///
/// A document whose frontmatter does not parse is NOT rejected: it becomes a
/// body-only artifact, which is the historical and correct behaviour for a
/// plain markdown file. Its raw bytes stay untouched on disk.
fn parse_document(
    file: &DiscoveredFile,
    detection: &DetectionResult,
    warnings: &mut Vec<String>,
) -> ParsedDocument {
    let title = detection
        .suggested_title
        .clone()
        .or_else(|| {
            file.path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.replace(['-', '_'], " "))
        })
        .unwrap_or_else(|| "Untitled".to_string());

    match crate::artifact::frontmatter::parse_frontmatter(&file.content) {
        Ok((frontmatter, body)) => {
            let raw_status = frontmatter.get("status").and_then(|v| v.as_str());
            let status = match raw_status {
                Some(s) => {
                    let (mapped, warning) = map_external_status(s);
                    if let Some(w) = warning {
                        warnings.push(w);
                    }
                    mapped
                }
                // No frontmatter status → default draft, no warning (AC-5).
                None => "draft".to_string(),
            };
            let tags = crate::artifact::frontmatter::tags_from_frontmatter(&frontmatter);
            let author = crate::artifact::frontmatter::author_from_frontmatter(&frontmatter)
                .unwrap_or_else(|| SCAN_IMPORT_AUTHOR.to_string());
            let links = crate::artifact::frontmatter::links_from_frontmatter(&frontmatter);
            ParsedDocument {
                frontmatter,
                body,
                title,
                status,
                tags,
                author,
                links,
            }
        }
        Err(_) => ParsedDocument {
            frontmatter: BTreeMap::new(),
            body: file.content.clone(),
            title,
            status: "draft".to_string(),
            tags: Vec::new(),
            author: SCAN_IMPORT_AUTHOR.to_string(),
            links: Vec::new(),
        },
    }
}

/// Process one detected document.
///
/// Canonical-first: the id is resolved, existence is checked, the Markdown is
/// written, and only then is the optional legacy projection taken. A repeat run
/// never rewrites an existing document — it reports `Skipped` and leaves both
/// the artifact and the external source byte-identical.
async fn process_detected_file(
    file: &DiscoveredFile,
    detection: &DetectionResult,
    workspace: &Path,
    guard: Option<&crate::workspace::lock::WorkspaceLock>,
    legacy: Option<&LanceStore>,
    dry_run: bool,
) -> ScanImportEntry {
    let mut warnings: Vec<String> = Vec::new();
    let document = parse_document(file, detection, &mut warnings);

    // Build the canonical Markdown BEFORE resolving the id: the id lands in the
    // frontmatter, so the content is otherwise rendered twice. Dry-run never
    // allocates — it must not take the id-alloc lock or create a file.
    let artifact_id = if dry_run {
        preview_artifact_id(detection, legacy).await
    } else {
        match allocate_artifact_id(workspace, legacy, detection).await {
            Ok(id) => id,
            Err(e) => {
                return ScanImportEntry {
                    relative_path: file.relative_path.display().to_string(),
                    detected_kind: Some(detection.kind.clone()),
                    detection_tier: Some(detection.tier.clone()),
                    artifact_id: None,
                    status: ImportStatus::Failed(format!("allocate id: {e}")),
                    warnings,
                };
            }
        }
    };

    let entry_base = ScanImportEntry {
        relative_path: file.relative_path.display().to_string(),
        detected_kind: Some(detection.kind.clone()),
        detection_tier: Some(detection.tier.clone()),
        artifact_id: Some(artifact_id.clone()),
        status: ImportStatus::Imported, // will be overwritten
        warnings: Vec::new(),
    };

    if dry_run {
        // Preview only: no file, no lock file, no row. `imported` here counts
        // documents that WOULD be imported, exactly as before.
        return ScanImportEntry {
            status: ImportStatus::Imported,
            ..entry_base
        };
    }

    // Past this point nothing is a preview, so a guard must exist. The public
    // entry point already refuses that combination; this makes the invariant
    // local rather than assumed.
    let guard = guard.expect("scan-import entry point guarantees a guard for a mutating run");
    let ctx = crate::projection::MutationContext::new(workspace, guard, legacy);

    // Existence is a property of the canonical file, not of the derived index.
    // Checking the projection first is what makes a repeat run a no-op on a
    // migrated workspace, where there is no row to ask.
    let canonical_exists = projection_exists(workspace, &detection.kind, &artifact_id).await;
    let legacy_row = match legacy {
        Some(store) => match store.get_artifact(&artifact_id).await {
            Ok(row) => row,
            Err(e) => {
                return ScanImportEntry {
                    status: ImportStatus::Failed(format!("Check existing: {e}")),
                    ..entry_base
                };
            }
        },
        None => None,
    };

    if canonical_exists || legacy_row.is_some() {
        // Already imported. Do NOT rewrite: the canonical file is the source of
        // truth and a repeat must not overwrite a document the user may have
        // edited since the first import. The missing-projection heal below is
        // the one case that writes, and only because nothing was there.
        if !canonical_exists {
            if let Err(e) =
                write_canonical_document(&ctx, &detection.kind, &artifact_id, &document).await
            {
                warnings.push(format!(
                    "scan-import: {artifact_id} exists in the derived index but its \
                     canonical file could not be restored: {e}"
                ));
            }
            if let Some(store) = legacy {
                for (target, relation) in &document.links {
                    if let Err(e) = store.add_relation(&artifact_id, target, relation).await {
                        warnings.push(format!(
                            "scan-import: could not restore relation {artifact_id}→{target} ({relation}): {e}"
                        ));
                    }
                }
            }
        }
        return ScanImportEntry {
            status: ImportStatus::Skipped,
            warnings,
            ..entry_base
        };
    }

    // Canonical Markdown FIRST. Everything below this line is derived.
    let mut write_warnings = warnings;
    if let Err(e) = write_canonical_document(&ctx, &detection.kind, &artifact_id, &document).await {
        // Nothing was derived yet, so there is nothing to roll back — and the
        // document stays out of the workspace rather than half-imported.
        return ScanImportEntry {
            status: ImportStatus::Failed(format!("canonical write failed: {e}")),
            warnings: write_warnings,
            ..entry_base
        };
    }

    // Optional derived projection, second. Its failure does NOT delete the
    // canonical file: the Markdown is the truth, and reconcile re-derives it.
    if let Some(store) = legacy {
        let new_artifact = NewArtifact {
            id: artifact_id.clone(),
            kind: detection.kind.template_key().to_string(),
            status: document.status.clone(),
            title: document.title.clone(),
            body: document.body.clone(),
            depth: "standard".to_string(),
            author: Some(document.author.clone()),
            parent_epic: None,
            valid_until: None,
            tags: document.tags.clone(),
        };
        if let Err(e) = store.create_artifact(&new_artifact).await {
            return ScanImportEntry {
                status: ImportStatus::Failed(format!("{e}")),
                warnings: write_warnings,
                ..entry_base
            };
        }
        for (target, relation) in &document.links {
            if let Err(e) = store.add_relation(&artifact_id, target, relation).await {
                write_warnings.push(format!(
                    "scan-import: could not restore relation {artifact_id}→{target} ({relation}): {e}"
                ));
            }
        }
    }

    ScanImportEntry {
        status: ImportStatus::Imported,
        warnings: write_warnings,
        ..entry_base
    }
}

/// Write the canonical Markdown for an imported document through the one
/// writer every mutation uses, with the document's own unknown frontmatter
/// carried through as preserved extras.
///
/// `render_projection_record` preserves unknown keys from an EXISTING file at
/// the target path — but there is none on a first import, so the source's own
/// keys would be dropped. Writing the source document first and letting the
/// renderer normalise it is what makes "unknown frontmatter survives the round
/// trip" true rather than hoped for.
async fn write_canonical_document(
    ctx: &crate::projection::MutationContext<'_>,
    kind: &ArtifactKind,
    artifact_id: &str,
    document: &ParsedDocument,
) -> anyhow::Result<()> {
    let path = canonical_path(ctx.workspace(), kind, artifact_id, &document.title);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    // Seed the canonical file with the SOURCE document so its unknown
    // frontmatter keys are on disk before the renderer reads what to preserve.
    let seed = render_frontmatter_with_known_keys(document, kind, artifact_id)?;
    atomic_write(&path, seed.as_bytes()).await?;

    let now = chrono::Utc::now().to_rfc3339();
    let record = crate::db::store::ArtifactRecord {
        id: artifact_id.to_string(),
        kind: kind.template_key().to_string(),
        status: document.status.clone(),
        title: document.title.clone(),
        body: document.body.clone(),
        depth: "standard".to_string(),
        author: Some(document.author.clone()),
        parent_epic: None,
        r_eff_score: 0.0,
        valid_until: None,
        created_at: now.clone(),
        updated_at: now,
        tags: document.tags.clone(),
        body_hash: None,
        embedding: None,
    };
    // Files-first: the renderer keeps the seeded body + unknown keys and
    // normalises the known ones. `force_body = true` because this write IS the
    // body decision — a stale file must not win over the document being
    // imported right now.
    crate::projection::render_projection_record_with_body(ctx.workspace(), &record, &document.links)
        .await
        .map(|_| ())
}

/// The canonical file path for `(kind, id, title)` — the same computation the
/// projection renderer performs, so the seed write and the normalised write
/// land on ONE file rather than two.
fn canonical_path(
    workspace: &Path,
    kind: &ArtifactKind,
    artifact_id: &str,
    title: &str,
) -> PathBuf {
    workspace.join(kind.dir_name()).join(format!(
        "{}-{}.md",
        artifact_id,
        crate::projection::projection_slug(title)
    ))
}

/// Render the source document's frontmatter with the canonical keys
/// (`id`, `kind`, `status`, `depth`, `author`, `tags`, `links`) written last so
/// they always win, and every unknown key preserved verbatim.
fn render_frontmatter_with_known_keys(
    document: &ParsedDocument,
    kind: &ArtifactKind,
    artifact_id: &str,
) -> anyhow::Result<String> {
    let mut fm = document.frontmatter.clone();
    let s = |v: String| serde_yaml::Value::String(v);
    fm.insert("id".to_string(), s(artifact_id.to_string()));
    fm.insert("kind".to_string(), s(kind.template_key().to_string()));
    fm.insert("title".to_string(), s(document.title.clone()));
    fm.insert("status".to_string(), s(document.status.clone()));
    fm.insert(
        "depth".to_string(),
        s(document
            .frontmatter
            .get("depth")
            .and_then(|v| v.as_str())
            .unwrap_or("standard")
            .to_string()),
    );
    fm.insert("author".to_string(), s(document.author.clone()));
    if document.tags.is_empty() {
        fm.remove("tags");
    } else {
        fm.insert(
            "tags".to_string(),
            serde_yaml::Value::Sequence(document.tags.iter().map(|t| s(t.clone())).collect()),
        );
    }
    if document.links.is_empty() {
        fm.remove("links");
    } else {
        fm.insert(
            "links".to_string(),
            serde_yaml::Value::Sequence(
                document
                    .links
                    .iter()
                    .map(|(target, relation)| {
                        // `Mapping::insert` takes `Value` KEYS, not `String`
                        // ones — the same shape `projection::render_markdown`
                        // uses for its links block.
                        let mut m = serde_yaml::Mapping::new();
                        m.insert(s("target".to_string()), s(target.clone()));
                        m.insert(s("relation".to_string()), s(relation.clone()));
                        serde_yaml::Value::Mapping(m)
                    })
                    .collect(),
            ),
        );
    }
    crate::artifact::frontmatter::render_frontmatter(&fm, &document.body)
}

/// Tempfile + rename, so a kill between truncate and write cannot leave a
/// half-written canonical document. Same discipline as
/// `projection::atomic_markdown_write`; duplicated rather than exposed because
/// that helper is private to the projection module and this one is the
/// scan-import seed, not a projection render.
async fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "canonical path has no parent",
        )
    })?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(bytes)?;
    tmp.flush()?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

/// Check whether a projection file for `(kind, artifact_id)` already exists
/// on disk. Used by the Skipped-branch heal path (R2 audit rust-pro HIGH #3).
async fn projection_exists(workspace: &Path, kind: &ArtifactKind, artifact_id: &str) -> bool {
    let dir = workspace.join(kind.dir_name());
    let prefix = format!("{}-", artifact_id);
    let mut rd = match tokio::fs::read_dir(&dir).await {
        Ok(r) => r,
        Err(_) => return false,
    };
    while let Ok(Some(entry)) = rd.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&prefix) && name.ends_with(".md") {
            return true;
        }
    }
    false
}

/// Allocate the id for an imported document under the existing per-kind
/// `id_alloc` lock.
///
/// A document that declares its own well-formed id keeps it — the author's
/// `id:` is a fact about the document, not something a lock may renumber.
/// Everything else is allocated, and the allocation consults BOTH the
/// canonical files on disk and (when present) the legacy rows, so a migrated
/// workspace without a store still cannot collide with an id a sibling
/// worktree just took.
///
/// The workspace lock is NOT taken here: the caller's guard already serializes
/// the canonical writes, and id allocation guards a different thing on a
/// different lock file — the same split `id_alloc` documents for its own
/// callers.
async fn allocate_artifact_id(
    workspace: &Path,
    legacy: Option<&LanceStore>,
    detection: &DetectionResult,
) -> anyhow::Result<String> {
    if let Some(id) = &detection.suggested_id {
        let normalized = id.to_uppercase();
        if is_safe_artifact_id(&normalized) {
            return Ok(normalized);
        }
    }
    let kind = &detection.kind;
    let _id_lock = crate::artifact::id_alloc::acquire_id_alloc_lock(workspace, kind)
        .await
        .with_context(|| format!("acquire id-alloc lock for {}", kind.prefix()))?;
    let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
    for n in 1..=999u32 {
        let candidate = format!("{prefix}-{n:03}");
        if projection_exists(workspace, kind, &candidate).await {
            continue;
        }
        if let Some(store) = legacy
            && let Ok(Some(_)) = store.get_artifact(&candidate).await
        {
            continue;
        }
        return Ok(candidate);
    }
    anyhow::bail!(
        "no free {prefix}-NNN id for kind {} — id space exhausted at 999",
        kind.prefix()
    )
}

/// The id a dry-run would use, computed WITHOUT the id-alloc lock, without
/// touching disk and without writing anything.
///
/// A dry-run that took the allocation lock would create a lock file, which is
/// exactly the "dry-run creates nothing" contract it is supposed to honour.
async fn preview_artifact_id(detection: &DetectionResult, legacy: Option<&LanceStore>) -> String {
    if let Some(id) = &detection.suggested_id {
        let normalized = id.to_uppercase();
        if is_safe_artifact_id(&normalized) {
            return normalized;
        }
    }
    let prefix = detection.kind.prefix().trim_end_matches('-').to_uppercase();
    if let Some(store) = legacy {
        for n in 1..=999u32 {
            let candidate = format!("{prefix}-{n:03}");
            match store.get_artifact(&candidate).await {
                Ok(None) => return candidate,
                Ok(Some(_)) => continue,
                Err(_) => return candidate,
            }
        }
    }
    format!("{prefix}-001")
}

/// Same safety contract as `db::store::validate_id_for_filter` but
/// returns a bool so callers can fall back to auto-ID gracefully.
/// Accepts `[A-Za-z][A-Za-z0-9_-]*` and rejects traversal / null bytes.
fn is_safe_artifact_id(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    if !id.chars().next().unwrap_or(' ').is_ascii_alphabetic() {
        return false;
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return false;
    }
    if id.contains("..") || id.contains('/') || id.contains('\\') || id.contains('\0') {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A `.forgeplan/` with the artifact dirs a real workspace has, and NO
    /// lance/ — i.e. the constellation-managed layout. Every default test
    /// exercises this one, because it is the layout that used to be refused.
    async fn workspace() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        for dir in crate::workspace::ARTIFACT_DIRS {
            std::fs::create_dir_all(ws.join(dir)).unwrap();
        }
        (tmp, ws)
    }

    async fn guard(ws: &Path) -> crate::workspace::lock::WorkspaceLock {
        crate::workspace::lock::acquire_workspace_lock(ws)
            .await
            .unwrap()
    }

    fn source(tmp: &Path, rel: &str, body: &str) -> PathBuf {
        let path = tmp.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
        path
    }

    /// Sorted inventory of every file under `root`, for "nothing was created"
    /// assertions that are stronger than "the one file I know about".
    fn inventory(root: &Path) -> Vec<String> {
        fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
            let Ok(rd) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in rd.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, base, out);
                } else if let Ok(rel) = path.strip_prefix(base) {
                    out.push(rel.to_string_lossy().to_string());
                }
            }
        }
        let mut out = Vec::new();
        walk(root, root, &mut out);
        out.sort();
        out
    }

    fn imported_file(ws: &Path, dir: &str, prefix: &str) -> Option<(PathBuf, String)> {
        let mut entries: Vec<_> = std::fs::read_dir(ws.join(dir))
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .map(|n| {
                        let n = n.to_string_lossy().to_string();
                        n.starts_with(prefix) && n.ends_with(".md")
                    })
                    .unwrap_or(false)
            })
            .collect();
        entries.sort();
        let path = entries.into_iter().next()?;
        let body = std::fs::read_to_string(&path).ok()?;
        Some((path, body))
    }

    #[tokio::test]
    async fn a_dry_run_creates_nothing_at_all() {
        let (tmp, ws) = workspace().await;
        source(
            tmp.path(),
            "docs/PRD-001-test.md",
            "---\nkind: prd\nid: PRD-001\ntitle: Test\n---\n# Test",
        );
        let lock = guard(&ws).await;
        let before = inventory(&ws);

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions {
                dry_run: true,
                custom_path: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(
            result.imported, 1,
            "dry run still previews what it would do"
        );
        assert_eq!(
            inventory(&ws),
            before,
            "dry-run must not create a file, a lock or a table"
        );
    }

    #[tokio::test]
    async fn a_dry_run_on_a_legacy_workspace_creates_no_row() {
        let (tmp, ws) = workspace().await;
        let store = LanceStore::init(&ws).await.unwrap();
        source(
            tmp.path(),
            "docs/PRD-001-test.md",
            "---\nkind: prd\nid: PRD-001\ntitle: Test\n---\n# Test",
        );
        let lock = guard(&ws).await;

        scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            Some(&store),
            &ScanImportOptions {
                dry_run: true,
                custom_path: None,
            },
        )
        .await
        .unwrap();

        assert!(store.get_artifact("PRD-001").await.unwrap().is_none());
    }

    /// A dry run is allowed to run with NO workspace lock — that is the whole
    /// point of it: taking the lock creates `.forgeplan/.lock`, so a preview
    /// behind the lock mutates the workspace it promises not to touch.
    #[tokio::test]
    async fn a_dry_run_needs_no_workspace_lock() {
        let (tmp, ws) = workspace().await;
        source(
            tmp.path(),
            "docs/ADR-030-preview.md",
            "---\nkind: adr\nid: ADR-030\ntitle: Preview\n---\n# Preview",
        );
        let before = inventory(&ws);

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            None,
            None,
            &ScanImportOptions {
                dry_run: true,
                custom_path: None,
            },
        )
        .await
        .expect("a dry run must work without the workspace lock");

        assert_eq!(result.imported, 1);
        assert_eq!(
            inventory(&ws),
            before,
            "a lockless dry run created something in the workspace"
        );
    }

    /// The converse: a MUTATING run with no lock is refused rather than writing
    /// canonical Markdown outside the workspace serialization.
    #[tokio::test]
    async fn a_mutating_run_without_the_lock_is_refused() {
        let (tmp, ws) = workspace().await;
        source(
            tmp.path(),
            "docs/ADR-031-unlocked.md",
            "---\nkind: adr\nid: ADR-031\ntitle: Unlocked\n---\n# Unlocked",
        );
        let before = inventory(&ws);

        let err = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            None,
            None,
            &ScanImportOptions::default(),
        )
        .await
        .expect_err("a mutating run must not proceed without the lock");

        assert!(err.to_string().contains("lock"), "got: {err}");
        assert_eq!(inventory(&ws), before, "the refused run still wrote a file");
    }

    /// A failed batch is visible in the result, not flattened into the
    /// counts — `init --scan` and the `scan-import` command both report from
    /// these entries.
    #[tokio::test]
    async fn a_failed_document_is_reported_as_failed_with_its_reason() {
        let (tmp, ws) = workspace().await;
        source(
            tmp.path(),
            "docs/ADR-031-blocked.md",
            "---\nkind: adr\nid: ADR-031\ntitle: Blocked\n---\n# Blocked",
        );
        // Replace the artifact directory with a regular file: the canonical
        // write cannot create its parent, so the document genuinely fails —
        // as opposed to being skipped, which is what an existing file does.
        let adrs = ws.join("adrs");
        std::fs::remove_dir_all(&adrs).unwrap();
        std::fs::write(&adrs, b"not a directory").unwrap();
        let lock = guard(&ws).await;

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();

        let entry = result
            .entries
            .iter()
            .find(|e| e.relative_path.contains("ADR-031"))
            .expect("the blocked document is reported");
        let ImportStatus::Failed(reason) = &entry.status else {
            panic!(
                "a canonical write that cannot succeed must be Failed, got {:?}",
                entry.status
            );
        };
        assert!(!reason.is_empty(), "a failure must carry a reason");
        assert_eq!(
            result.imported, 0,
            "a failed document must not be counted as imported"
        );
    }

    fn entry(id: Option<&str>, path: &str) -> ScanImportEntry {
        ScanImportEntry {
            relative_path: path.to_string(),
            detected_kind: None,
            detection_tier: None,
            artifact_id: id.map(String::from),
            status: ImportStatus::Skipped,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn find_duplicate_ids_groups_same_id_and_ignores_unique() {
        let entries = vec![
            entry(Some("PRD-008"), "prds/PRD-008-consistent-output.md"),
            entry(Some("PRD-008"), "prds/PRD-008-cli-ux-redesign.md"),
            entry(Some("PRD-001"), "prds/PRD-001-flow.md"), // unique → not flagged
            entry(None, "docs/README.md"),                  // no id → ignored
        ];
        let dups = find_duplicate_ids(&entries);
        assert_eq!(dups.len(), 1, "exactly one collision (PRD-008)");
        assert_eq!(dups[0].id, "PRD-008");
        assert_eq!(dups[0].paths.len(), 2);
        // paths are sorted for a stable report
        assert_eq!(dups[0].paths[0], "prds/PRD-008-cli-ux-redesign.md");
    }

    #[test]
    fn find_duplicate_ids_empty_when_all_unique() {
        let entries = vec![
            entry(Some("PRD-001"), "a.md"),
            entry(Some("PRD-002"), "b.md"),
        ];
        assert!(find_duplicate_ids(&entries).is_empty());
    }

    /// Two files resolving to one id must be visible in the result — the
    /// resolver otherwise mixes status/body across them.
    #[tokio::test]
    async fn two_files_with_one_id_are_reported_as_a_collision() {
        let (tmp, ws) = workspace().await;
        source(
            tmp.path(),
            "docs/PRD-009-first.md",
            "---\nkind: prd\nid: PRD-009\ntitle: First\n---\n# A",
        );
        source(
            tmp.path(),
            "docs/PRD-009-second.md",
            "---\nkind: prd\nid: PRD-009\ntitle: Second\n---\n# B",
        );
        let lock = guard(&ws).await;

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions {
                dry_run: true,
                custom_path: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(result.duplicate_ids.len(), 1);
        assert_eq!(result.duplicate_ids[0].id, "PRD-009");
        assert_eq!(result.duplicate_ids[0].paths.len(), 2);
    }

    /// S1-R2-A: apply preserves the full body, the source author and status,
    /// the typed links and every frontmatter key the importer does not own —
    /// on a constellation-managed workspace with no store at all.
    #[tokio::test]
    async fn apply_preserves_body_author_status_links_and_unknown_frontmatter() {
        let (tmp, ws) = workspace().await;
        source(
            tmp.path(),
            "docs/ADR-001-storage.md",
            "---\n\
             kind: adr\n\
             id: ADR-001\n\
             title: Use Postgres\n\
             status: accepted\n\
             author: Ada Lovelace\n\
             tags:\n  - source=code\n\
             links:\n  - target: RFC-002\n    relation: informs\n\
             review_owner: platform-team\n\
             last_modified_by: agent-x\n\
             ---\n\
             ## Context\n\
             \n\
             Needed reliable storage.\n",
        );
        let lock = guard(&ws).await;

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(result.imported, 1);

        let (_, content) = imported_file(&ws, "adrs", "ADR-001-").expect("canonical ADR written");
        let (fm, body) =
            crate::artifact::frontmatter::parse_frontmatter(&content).expect("valid frontmatter");

        assert_eq!(body.trim(), "## Context\n\nNeeded reliable storage.");
        assert_eq!(fm.get("author").unwrap().as_str(), Some("Ada Lovelace"));
        // `accepted` is an Obsidian spelling; the canonical status is `active`.
        assert_eq!(fm.get("status").unwrap().as_str(), Some("active"));
        assert_eq!(fm.get("tags").unwrap().as_sequence().unwrap().len(), 1);
        let links = fm.get("links").unwrap().as_sequence().unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].get("target").unwrap().as_str(), Some("RFC-002"));
        assert_eq!(links[0].get("relation").unwrap().as_str(), Some("informs"));
        // Unknown keys survive the round trip verbatim.
        assert_eq!(
            fm.get("review_owner").unwrap().as_str(),
            Some("platform-team")
        );
        assert_eq!(
            fm.get("last_modified_by").unwrap().as_str(),
            Some("agent-x")
        );
    }

    /// A document with no frontmatter still imports as a body-only artifact
    /// and still lands on disk — not as a silent no-op.
    #[tokio::test]
    async fn a_document_without_frontmatter_imports_as_a_body_only_artifact() {
        let (tmp, ws) = workspace().await;
        source(
            tmp.path(),
            "docs/ADR-005-plain.md",
            "# Plain\n\nJust prose.\n",
        );
        let lock = guard(&ws).await;

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(result.imported, 1);
        let entry = &result.entries[0];
        assert!(entry.warnings.is_empty(), "got {:?}", entry.warnings);
        let (_, content) = imported_file(&ws, "adrs", "ADR-005-").expect("canonical ADR written");
        let (fm, body) = crate::artifact::frontmatter::parse_frontmatter(&content).unwrap();
        assert_eq!(fm.get("status").unwrap().as_str(), Some("draft"));
        assert!(body.contains("Just prose."));
    }

    /// The external document is the operator's file. An import copies it in;
    /// it never edits or removes the original.
    #[tokio::test]
    async fn apply_never_touches_the_external_source_file() {
        let (tmp, ws) = workspace().await;
        let original = "---\nkind: prd\nid: PRD-010\ntitle: Source\n---\n# Source body\n";
        let path = source(tmp.path(), "docs/PRD-010-source.md", original);
        let lock = guard(&ws).await;

        scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            original,
            "scan-import must not rewrite its input"
        );
    }

    /// S1-R2-A repeat: a second run is a no-op on the artifact AND on the
    /// source, and it reports `Skipped` rather than a fresh import.
    #[tokio::test]
    async fn a_repeat_run_skips_and_changes_nothing() {
        let (tmp, ws) = workspace().await;
        let original =
            "---\nid: ADR-011\nkind: adr\ntitle: Repeatable\nstatus: accepted\n---\n# Once\n";
        let path = source(tmp.path(), "docs/ADR-011-repeat.md", original);
        let lock = guard(&ws).await;

        let first = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(first.imported, 1);
        let (_, after_first) = imported_file(&ws, "adrs", "ADR-011-").unwrap();

        let second = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(second.imported, 0);
        assert_eq!(second.skipped, 1);
        let (_, after_second) = imported_file(&ws, "adrs", "ADR-011-").unwrap();
        assert_eq!(after_first, after_second, "repeat rewrote the artifact");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    /// An existing canonical file the operator has since edited must not be
    /// overwritten by a repeat import.
    #[tokio::test]
    async fn a_repeat_does_not_overwrite_a_locally_edited_artifact() {
        let (tmp, ws) = workspace().await;
        source(
            tmp.path(),
            "docs/ADR-012-edited.md",
            "---\nid: ADR-012\nkind: adr\ntitle: Edited\nstatus: accepted\n---\n# From import\n",
        );
        let lock = guard(&ws).await;
        scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();

        let (path, imported) = imported_file(&ws, "adrs", "ADR-012-").unwrap();
        let edited = imported.replace("# From import", "# Hand edited");
        std::fs::write(&path, &edited).unwrap();

        let second = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(second.skipped, 1);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            edited,
            "a repeat import overwrote a user's local edit"
        );
    }

    /// The migrated layout is the whole point: importing there must not
    /// create a `lance/` directory, let alone the retired artifact tables.
    #[tokio::test]
    async fn a_migrated_workspace_import_leaves_no_legacy_tables() {
        let (tmp, ws) = workspace().await;
        source(
            tmp.path(),
            "docs/RFC-001-design.md",
            "---\nkind: rfc\nid: RFC-001\ntitle: Design\n---\n# Design",
        );
        let lock = guard(&ws).await;

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(result.imported, 1);
        assert!(
            !ws.join("lance").exists(),
            "import created a legacy index on a constellation-managed workspace"
        );
        for legacy in ["artifacts.lance", "evidence.lance", "relations.lance"] {
            assert!(
                !ws.join("lance").join(legacy).exists(),
                "{legacy} must not be recreated"
            );
        }
        let (_, content) = imported_file(&ws, "rfcs", "RFC-001-").expect("canonical RFC written");
        assert!(content.contains("# Design"));
    }

    /// On a legacy workspace the same call still takes the derived row, and
    /// the row agrees with the file the import just wrote.
    #[tokio::test]
    async fn a_legacy_workspace_still_receives_the_derived_row() {
        let (tmp, ws) = workspace().await;
        let store = LanceStore::init(&ws).await.unwrap();
        source(
            tmp.path(),
            "docs/PRD-007-tagged.md",
            "---\nkind: prd\nid: PRD-007\ntitle: Tagged\ntags:\n  - source=code\n  - layer=auth\n---\n# Tagged",
        );
        let lock = guard(&ws).await;

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            Some(&store),
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(result.imported, 1);

        let record = store.get_record("PRD-007").await.unwrap().unwrap();
        assert_eq!(record.tags, vec!["source=code", "layer=auth"]);
        let (_, content) = imported_file(&ws, "prds", "PRD-007-").expect("canonical PRD written");
        assert!(
            content.contains("layer=auth"),
            "tags must reach the file too"
        );
    }

    /// A legacy row whose canonical file is missing is healed from the source
    /// document rather than left orphaned (the case `reindex` would purge).
    #[tokio::test]
    async fn a_legacy_row_without_a_file_is_healed_from_the_source() {
        let (tmp, ws) = workspace().await;
        let store = LanceStore::init(&ws).await.unwrap();
        store
            .create_artifact(&NewArtifact {
                id: "ADR-013".to_string(),
                kind: "adr".to_string(),
                status: "draft".to_string(),
                title: "Healed".to_string(),
                body: String::new(),
                depth: "standard".to_string(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        source(
            tmp.path(),
            "docs/ADR-013-healed.md",
            "---\nkind: adr\nid: ADR-013\ntitle: Healed\nstatus: accepted\nauthor: Grace\n---\n# Healed body\n",
        );
        let lock = guard(&ws).await;

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            Some(&store),
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(result.skipped, 1);
        let (_, content) = imported_file(&ws, "adrs", "ADR-013-")
            .expect("the missing canonical file is restored from the source");
        assert!(content.contains("Grace"), "author must survive the heal");
        assert!(content.contains("# Healed body"));
    }

    #[tokio::test]
    async fn unknown_files_are_counted_not_imported() {
        let (tmp, ws) = workspace().await;
        source(tmp.path(), "docs/random.md", "# Shopping\n- Milk");
        let lock = guard(&ws).await;

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(result.unknown, 1);
        assert_eq!(result.imported, 0);
    }

    /// No declared id ⇒ an allocated one, and the allocation consults the
    /// files already on disk so it cannot collide with them.
    #[tokio::test]
    async fn a_document_without_an_id_gets_the_next_free_one() {
        let (tmp, ws) = workspace().await;
        std::fs::write(
            ws.join("prds").join("PRD-001-existing.md"),
            "---\nid: PRD-001\nkind: prd\ntitle: Existing\n---\n# Existing\n",
        )
        .unwrap();
        source(
            tmp.path(),
            "docs/my-feature.md",
            "---\nkind: prd\ntitle: My Feature\n---\n# Feature",
        );
        let lock = guard(&ws).await;

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(result.imported, 1);
        assert_eq!(result.entries[0].artifact_id.as_deref(), Some("PRD-002"));
        imported_file(&ws, "prds", "PRD-002-").expect("the allocated id is written to disk");
    }

    #[tokio::test]
    async fn a_custom_path_outside_the_project_is_rejected() {
        let (tmp, ws) = workspace().await;
        let lock = guard(&ws).await;

        let err = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions {
                dry_run: false,
                custom_path: Some("../../etc".to_string()),
            },
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("traversal"), "got: {err}");
    }

    // ── status mapping: the four spellings an external document can carry ──

    async fn import_status_of(status: &str) -> (String, Vec<String>) {
        let (tmp, ws) = workspace().await;
        source(
            tmp.path(),
            "docs/ADR-020-status.md",
            &format!("---\nkind: adr\nid: ADR-020\ntitle: Status\nstatus: {status}\n---\n# Body\n"),
        );
        let lock = guard(&ws).await;
        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();
        let entry = &result.entries[0];
        let (_, content) = imported_file(&ws, "adrs", "ADR-020-").expect("canonical ADR written");
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&content).unwrap();
        (
            fm.get("status").unwrap().as_str().unwrap().to_string(),
            entry.warnings.clone(),
        )
    }

    #[tokio::test]
    async fn obsidian_accepted_maps_to_active_without_a_warning() {
        let (status, warnings) = import_status_of("accepted").await;
        assert_eq!(status, "active");
        assert!(warnings.is_empty(), "got {warnings:?}");
    }

    #[tokio::test]
    async fn rejected_maps_to_superseded() {
        let (status, _) = import_status_of("rejected").await;
        assert_eq!(status, "superseded");
    }

    #[tokio::test]
    async fn an_unmappable_status_falls_back_to_draft_and_warns() {
        // Fail-loud: a status the methodology does not know must not be
        // silently rewritten to `draft` with no trace.
        let (status, warnings) = import_status_of("wip").await;
        assert_eq!(status, "draft");
        assert!(
            warnings.iter().any(|w| w.contains("wip")),
            "the warning must name the offending value, got {warnings:?}"
        );
    }
}
