use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::artifact::types::ArtifactKind;
use crate::db::store::{LanceStore, NewArtifact};
use crate::scan::detect::{DetectionResult, DetectionTier, detect_kind_with_path};
use crate::scan::discovery::{DiscoveredFile, discover_markdown_files};
use crate::scan::status_map::map_external_status;
use crate::workspace::lock::WorkspaceReadPermit;

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

type ScanImportIdentity = (String, String, Vec<(String, String)>);

/// Read-only scan plan, including the exact source snapshot and identities
/// selected for the whole batch under the workspace lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanImportPlan {
    snapshot: Vec<(PathBuf, String)>,
    ids: BTreeMap<PathBuf, (String, bool)>,
    parents: BTreeMap<PathBuf, String>,
    identities: Vec<ScanImportIdentity>,
    links: BTreeMap<PathBuf, Vec<(String, String)>>,
    existing: std::collections::BTreeSet<PathBuf>,
}

fn scan_root(project_root: &Path, options: &ScanImportOptions) -> anyhow::Result<PathBuf> {
    if let Some(custom) = &options.custom_path {
        let candidate = project_root.join(custom);
        let canonical = candidate.canonicalize()?;
        let canonical_root = project_root.canonicalize()?;
        if !canonical.starts_with(&canonical_root) {
            anyhow::bail!(
                "Scan path '{}' is outside project root. Path traversal rejected.",
                custom
            );
        }
        Ok(candidate)
    } else {
        Ok(project_root.to_path_buf())
    }
}

/// Discover the kinds whose cross-worktree allocator locks a scan needs.
pub fn scan_import_kinds(
    project_root: &Path,
    options: &ScanImportOptions,
) -> anyhow::Result<Vec<ArtifactKind>> {
    let files = discover_markdown_files(&scan_root(project_root, options)?)?;
    let mut kinds = Vec::new();
    for file in files {
        let filename = file
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("scan source path is not valid UTF-8"))?;
        if let Some(detection) =
            detect_kind_with_path(filename, Some(&file.relative_path), &file.content)
        {
            kinds.push(detection.kind);
        }
    }
    Ok(kinds)
}

/// Compute every generated id, canonical key, and link without mutating
/// files or indexes. The caller holds the workspace lock throughout planning,
/// migration, re-planning, and execution.
pub async fn plan_scan_import(
    project_root: &Path,
    workspace: &Path,
    guard: &crate::workspace::lock::WorkspaceLock,
    allocation: &crate::artifact::id_alloc::AllocationGuard,
    legacy: Option<&LanceStore>,
    options: &ScanImportOptions,
) -> anyhow::Result<ScanImportPlan> {
    plan_scan_import_inner(
        project_root,
        workspace,
        Some(guard),
        Some((guard, allocation)),
        legacy,
        options,
    )
    .await
}

async fn plan_scan_import_inner(
    project_root: &Path,
    workspace: &Path,
    permit: Option<&dyn WorkspaceReadPermit>,
    locked: Option<(
        &crate::workspace::lock::WorkspaceLock,
        &crate::artifact::id_alloc::AllocationGuard,
    )>,
    legacy: Option<&LanceStore>,
    options: &ScanImportOptions,
) -> anyhow::Result<ScanImportPlan> {
    use crate::artifact::canonical_key::ArtifactCandidate;
    use std::collections::BTreeSet;
    if let Some(permit) = permit {
        permit.matches_workspace(workspace)?;
    }

    let root = scan_root(project_root, options)?;
    let mut files = discover_markdown_files(&root)?;
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    let snapshot = files
        .iter()
        .map(|f| (f.relative_path.clone(), f.content.clone()))
        .collect();
    let mut candidates = crate::artifact::documents::workspace_candidates(workspace).await?;
    let mut ids = BTreeMap::new();
    let mut planned_links = BTreeMap::new();
    let mut identities = Vec::new();
    let mut parents = Vec::new();
    let mut bumps = BTreeMap::<String, u32>::new();
    let mut existing_paths = BTreeSet::new();
    let mut batch_keys = BTreeSet::new();
    files.sort_by_key(|file| {
        let filename = file
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let has_id = detect_kind_with_path(filename, Some(&file.relative_path), &file.content)
            .and_then(|detection| detection.suggested_id)
            .is_some_and(|id| is_safe_artifact_id(&id));
        !has_id
    });
    for file in &files {
        let filename = file
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("scan source path is not valid UTF-8"))?;
        let Some(detection) =
            detect_kind_with_path(filename, Some(&file.relative_path), &file.content)
        else {
            continue;
        };
        let document = parse_document(file, &detection, &mut Vec::new());
        let suggested = document
            .field("id")
            .filter(|id| is_safe_artifact_id(id))
            .or(detection
                .suggested_id
                .as_deref()
                .filter(|id| is_safe_artifact_id(id)))
            .map(str::to_string);
        let generated = suggested.is_none();
        let id = if let Some(id) = suggested {
            id
        } else {
            let prefix = detection.kind.prefix().to_ascii_uppercase();
            if let Some((guard, allocation)) = locked {
                crate::artifact::id_alloc::preview_display_number(
                    workspace,
                    guard,
                    allocation,
                    legacy,
                    &detection.kind,
                    3,
                    bumps.get(&prefix).copied(),
                )
                .await?
                .0
            } else {
                let max = match permit {
                    Some(permit) => {
                        crate::artifact::id_alloc::compute_max_display_number(
                            workspace,
                            permit,
                            legacy,
                            &detection.kind,
                        )
                        .await?
                    }
                    None => unlocked_max_display_number(workspace, &detection.kind).await?,
                };
                let number = max
                    .max(bumps.get(&prefix).copied().unwrap_or(0))
                    .checked_add(1)
                    .ok_or_else(|| {
                        anyhow::anyhow!("display number for kind {prefix} overflows u32")
                    })?;
                format!("{}-{number:03}", prefix.trim_end_matches('-'))
            }
        };
        let key = canonical_key_for_import(&id, &document);
        if !batch_keys.insert(key.clone()) {
            anyhow::bail!("scan batch has duplicate identity {key}");
        }
        let canonical_exists = canonical_key_exists(&key, &candidates)?;
        if canonical_exists {
            existing_paths.insert(file.relative_path.clone());
        }
        if let Some(store) = legacy {
            let permit =
                permit.ok_or_else(|| anyhow::anyhow!("legacy preview needs a read permit"))?;
            let count = store
                .list_artifacts(permit, None)
                .await?
                .into_iter()
                .filter(|row| row.canonical_key == key)
                .count();
            if count > 1 {
                anyhow::bail!("scan identity {key} has duplicate rows in the existing index");
            }
            if (count == 1) != canonical_exists {
                anyhow::bail!("scan identity {key} disagrees with the existing index");
            }
        }
        if !canonical_exists
            && let Some(number) = id
                .rsplit('-')
                .next()
                .and_then(|part| part.parse::<u32>().ok())
        {
            let prefix = detection.kind.prefix().to_ascii_uppercase();
            let bump = bumps.entry(prefix).or_default();
            *bump = (*bump).max(number);
        }
        if !canonical_exists {
            candidates.push(ArtifactCandidate {
                canonical_key: key.clone(),
                display_id: id.clone(),
                title: document.title.clone(),
                relative_path: file.relative_path.clone(),
            });
        }
        ids.insert(file.relative_path.clone(), (id.clone(), generated));
        if let Some(parent) = document
            .frontmatter
            .get("parent_epic")
            .and_then(|v| v.as_str())
        {
            parents.push((file.relative_path.clone(), parent.to_string()));
        }
        planned_links.insert(file.relative_path.clone(), document.links.clone());
        identities.push((id, key, document.links));
    }
    let mut resolved_links_by_path = BTreeMap::new();
    for (path, links) in &planned_links {
        let mut resolved_links = Vec::with_capacity(links.len());
        for (target, relation) in links {
            crate::db::store::validate_link_target(target)?;
            let resolved_target = if target.contains('@') {
                target.clone()
            } else {
                crate::artifact::canonical_key::resolve_structural_target(target, &candidates)?
                    .map(|found| found.identity.canonical_key)
                    .unwrap_or_else(|| target.clone())
            };
            resolved_links.push((resolved_target, relation.clone()));
        }
        resolved_links_by_path.insert(path.clone(), resolved_links);
    }
    let mut resolved_parents = BTreeMap::new();
    for (path, parent) in parents {
        crate::db::store::validate_link_target(&parent)?;
        let resolved = if parent.contains('@') {
            parent
        } else {
            crate::artifact::canonical_key::resolve_structural_target(&parent, &candidates)?
                .map(|found| found.identity.canonical_key)
                .unwrap_or(parent)
        };
        resolved_parents.insert(path, resolved);
    }
    Ok(ScanImportPlan {
        snapshot,
        ids,
        parents: resolved_parents,
        identities,
        links: resolved_links_by_path,
        existing: existing_paths,
    })
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
/// A mutating import requires the caller's exclusive `guard`. A dry run
/// without a legacy projection needs no workspace lock. A legacy dry run
/// requires a caller-owned `read_permit` (or the exclusive `guard`) for all
/// projection reads. This function never acquires a nested workspace lock.
/// Release the guard before a subsequent reconcile, which acquires its own.
pub async fn scan_and_import_to_workspace(
    project_root: &Path,
    workspace: &Path,
    guard: Option<&crate::workspace::lock::WorkspaceLock>,
    read_permit: Option<&dyn WorkspaceReadPermit>,
    legacy: Option<&LanceStore>,
    options: &ScanImportOptions,
) -> anyhow::Result<ScanImportResult> {
    let permit: Option<&dyn WorkspaceReadPermit> = guard
        .map(|held| held as &dyn WorkspaceReadPermit)
        .or(read_permit);
    if legacy.is_some() && permit.is_none() {
        anyhow::bail!("scan-import of a legacy projection requires a workspace read permit");
    }
    if let Some(permit) = permit {
        permit.matches_workspace(workspace)?;
    }
    if options.dry_run {
        let plan =
            plan_scan_import_inner(project_root, workspace, permit, None, legacy, options).await?;
        return scan_import_impl(ScanImportContext {
            project_root,
            workspace,
            guard,
            permit,
            legacy,
            options,
            plan: Some(&plan),
            allocation: None,
        })
        .await;
    }
    let guard = guard.ok_or_else(|| {
        anyhow::anyhow!("scan-import needs the workspace lock for a mutating run")
    })?;
    let kinds = scan_import_kinds(project_root, options)?;
    let allocation =
        crate::artifact::id_alloc::acquire_allocation_guard(workspace, guard, &kinds).await?;
    let plan =
        plan_scan_import(project_root, workspace, guard, &allocation, legacy, options).await?;
    scan_import_impl(ScanImportContext {
        project_root,
        workspace,
        guard: Some(guard),
        permit: Some(guard),
        legacy,
        options,
        plan: Some(&plan),
        allocation: Some(&allocation),
    })
    .await
}

/// Execute the exact plan that was checked before and after reopening the
/// index; rediscovery must match its complete source snapshot before writes.
pub async fn execute_scan_import_plan(
    project_root: &Path,
    workspace: &Path,
    guard: &crate::workspace::lock::WorkspaceLock,
    allocation: &crate::artifact::id_alloc::AllocationGuard,
    legacy: Option<&LanceStore>,
    options: &ScanImportOptions,
    plan: &ScanImportPlan,
) -> anyhow::Result<ScanImportResult> {
    if options.dry_run {
        anyhow::bail!("a mutating scan plan cannot be executed as a dry run");
    }
    if plan_scan_import(project_root, workspace, guard, allocation, legacy, options).await? != *plan
    {
        anyhow::bail!("scan identities or sources changed after planning; nothing was imported");
    }
    scan_import_impl(ScanImportContext {
        project_root,
        workspace,
        guard: Some(guard),
        permit: Some(guard),
        legacy,
        options,
        plan: Some(plan),
        allocation: Some(allocation),
    })
    .await
}

struct ScanImportContext<'a> {
    project_root: &'a Path,
    workspace: &'a Path,
    guard: Option<&'a crate::workspace::lock::WorkspaceLock>,
    permit: Option<&'a dyn WorkspaceReadPermit>,
    legacy: Option<&'a LanceStore>,
    options: &'a ScanImportOptions,
    plan: Option<&'a ScanImportPlan>,
    allocation: Option<&'a crate::artifact::id_alloc::AllocationGuard>,
}

async fn scan_import_impl(context: ScanImportContext<'_>) -> anyhow::Result<ScanImportResult> {
    let ScanImportContext {
        project_root,
        workspace,
        guard,
        permit,
        legacy,
        options,
        plan,
        allocation,
    } = context;
    if !options.dry_run && guard.is_none() {
        anyhow::bail!(
            "scan-import needs the workspace lock for a mutating run; a dry run may pass none"
        );
    }
    if let Some(permit) = permit {
        permit.matches_workspace(workspace)?;
    } else if legacy.is_some() {
        anyhow::bail!("scan-import of a legacy projection requires a workspace read permit");
    }
    let root = scan_root(project_root, options)?;
    let mut files = discover_markdown_files(&root)?;
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    if let Some(plan) = plan {
        let snapshot: Vec<_> = files
            .iter()
            .map(|f| (f.relative_path.clone(), f.content.clone()))
            .collect();
        if snapshot != plan.snapshot {
            anyhow::bail!("scan sources changed after planning; nothing was imported");
        }
    }
    files.sort_by_key(|file| {
        let filename = file
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let has_id = detect_kind_with_path(filename, Some(&file.relative_path), &file.content)
            .and_then(|detection| detection.suggested_id)
            .is_some_and(|id| is_safe_artifact_id(&id));
        !has_id
    });
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
                let planned_id = plan.and_then(|p| p.ids.get(&file.relative_path));
                process_detected_file(
                    file,
                    &det,
                    DetectedFileContext {
                        workspace,
                        guard,
                        legacy,
                        dry_run: options.dry_run,
                        planned_id,
                        planned_existing: plan
                            .is_some_and(|p| p.existing.contains(&file.relative_path)),
                        planned_parent: plan.and_then(|p| p.parents.get(&file.relative_path)),
                        planned_links: plan.and_then(|p| p.links.get(&file.relative_path)),
                        allocation,
                    },
                )
                .await
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
#[derive(Debug, Clone)]
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

impl ParsedDocument {
    fn field(&self, name: &str) -> Option<&str> {
        self.frontmatter.get(name).and_then(|value| value.as_str())
    }

    fn depth(&self) -> &str {
        self.field("depth").unwrap_or("standard")
    }
}

/// The canonical key an imported document carries.
///
/// ADR-030: preserve a valid declared slug; otherwise keep the source ID.
/// The planner preserves its original spelling before any filename-derived ID.
fn canonical_key_for_import(artifact_id: &str, document: &ParsedDocument) -> String {
    document
        .frontmatter
        .get("slug")
        .and_then(|value| value.as_str())
        .filter(|slug| crate::artifact::types::is_canonical_key(slug))
        .unwrap_or(artifact_id)
        .to_string()
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
struct DetectedFileContext<'a> {
    workspace: &'a Path,
    guard: Option<&'a crate::workspace::lock::WorkspaceLock>,
    legacy: Option<&'a LanceStore>,
    dry_run: bool,
    planned_id: Option<&'a (String, bool)>,
    planned_existing: bool,
    planned_parent: Option<&'a String>,
    planned_links: Option<&'a Vec<(String, String)>>,
    allocation: Option<&'a crate::artifact::id_alloc::AllocationGuard>,
}

async fn process_detected_file(
    file: &DiscoveredFile,
    detection: &DetectionResult,
    context: DetectedFileContext<'_>,
) -> ScanImportEntry {
    let DetectedFileContext {
        workspace,
        guard,
        legacy,
        dry_run,
        planned_id,
        planned_existing,
        planned_parent,
        planned_links,
        allocation,
    } = context;
    let mut warnings: Vec<String> = Vec::new();
    let mut document = parse_document(file, detection, &mut warnings);
    if !planned_existing && let Some(links) = planned_links {
        document.links.clone_from(links);
    }
    if let Some(parent) = planned_parent {
        document.frontmatter.insert(
            "parent_epic".to_string(),
            serde_yaml::Value::String(parent.clone()),
        );
    }

    // Build the canonical Markdown BEFORE resolving the id: the id lands in the
    // frontmatter, so the content is otherwise rendered twice. Dry-run never
    // allocates — it must not take the id-alloc lock or create a file.
    let artifact_id = if let Some((id, _)) = planned_id {
        id.clone()
    } else {
        return ScanImportEntry {
            relative_path: file.relative_path.display().to_string(),
            detected_kind: Some(detection.kind.clone()),
            detection_tier: Some(detection.tier.clone()),
            artifact_id: None,
            status: ImportStatus::Failed("scan plan is missing an artifact id".to_string()),
            warnings,
        };
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
        return ScanImportEntry {
            status: if planned_existing {
                ImportStatus::Skipped
            } else {
                ImportStatus::Imported
            },
            warnings,
            ..entry_base
        };
    }

    // Past this point nothing is a preview, so a guard must exist. The public
    // entry point already refuses that combination; this makes the invariant
    // local rather than assumed.
    let guard = guard.expect("scan-import entry point guarantees a guard for a mutating run");
    let ctx = crate::projection::MutationContext::new(workspace, guard, legacy);

    let canonical_key = canonical_key_for_import(&artifact_id, &document);
    let canonical_exists = match crate::artifact::documents::workspace_candidates(workspace)
        .await
        .and_then(|candidates| {
            canonical_key_exists(&canonical_key, &candidates).map_err(Into::into)
        }) {
        Ok(exists) => exists,
        Err(error) => {
            return ScanImportEntry {
                status: ImportStatus::Failed(format!("Check projection: {error}")),
                ..entry_base
            };
        }
    };

    let legacy_row = match legacy {
        Some(store) => match store.list_artifacts(guard, None).await {
            Ok(rows) => rows
                .into_iter()
                .find(|row| row.canonical_key == canonical_key),
            Err(e) => {
                return ScanImportEntry {
                    status: ImportStatus::Failed(format!("Check existing: {e}")),
                    ..entry_base
                };
            }
        },
        None => None,
    };
    if !canonical_exists && planned_id.is_some_and(|(_, generated)| *generated) {
        let Some(allocation) = allocation else {
            return ScanImportEntry {
                status: ImportStatus::Failed("allocator guard is missing".to_string()),
                ..entry_base
            };
        };
        if let Err(error) = crate::artifact::id_alloc::reserve_prepared_display_number(
            workspace,
            guard,
            allocation,
            legacy,
            &detection.kind,
            3,
            &artifact_id,
        )
        .await
        {
            return ScanImportEntry {
                status: ImportStatus::Failed(format!("reserve planned id: {error}")),
                ..entry_base
            };
        }
    }

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
                    if let Err(e) = store
                        .add_relation(guard, &canonical_key, target, relation)
                        .await
                    {
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
            // ADR-030: the canonical key of a document is the slug its
            // frontmatter declares, and its display id when it declares none.
            // An import never mints a key.
            canonical_key: canonical_key.clone(),
            id: artifact_id.clone(),
            kind: detection.kind.template_key().to_string(),
            status: document.status.clone(),
            title: document.title.clone(),
            body: document.body.clone(),
            depth: document.depth().to_string(),
            author: Some(document.author.clone()),
            parent_epic: document.field("parent_epic").map(str::to_string),
            valid_until: document.field("valid_until").map(str::to_string),
            tags: document.tags.clone(),
        };
        if let Err(e) = store.create_artifact(guard, &new_artifact).await {
            return ScanImportEntry {
                status: ImportStatus::Failed(format!("{e}")),
                warnings: write_warnings,
                ..entry_base
            };
        }
        for (target, relation) in &document.links {
            if let Err(e) = store
                .add_relation(guard, &canonical_key, target, relation)
                .await
            {
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

/// Publish the source frontmatter and body without replacing any existing file.
async fn write_canonical_document(
    ctx: &crate::projection::MutationContext<'_>,
    kind: &ArtifactKind,
    artifact_id: &str,
    document: &ParsedDocument,
) -> anyhow::Result<()> {
    let preferred = canonical_path(ctx.workspace(), kind, artifact_id, &document.title);
    let path = if preferred.exists() {
        let dir = preferred.parent().expect("canonical path has a parent");
        let key = canonical_key_for_import(artifact_id, document);
        let mut suffix = 0u64;
        loop {
            let filename = if suffix == 0 {
                format!("{}-{}.md", artifact_id, key)
            } else {
                format!("{}-{}-{}.md", artifact_id, key, suffix)
            };
            let candidate = dir.join(filename);
            if !candidate.exists() {
                break candidate;
            }
            suffix += 1;
        }
    } else {
        preferred
    };
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let content = render_frontmatter_with_known_keys(document, kind, artifact_id)?;
    atomic_write(&path, content.as_bytes()).await?;
    Ok(())
}

/// Preferred filename for an imported document.
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
    fm.insert(
        "slug".to_string(),
        s(canonical_key_for_import(artifact_id, document)),
    );
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
    tmp.persist_noclobber(path).map_err(|e| e.error)?;
    Ok(())
}

fn canonical_key_exists(
    key: &str,
    candidates: &[crate::artifact::canonical_key::ArtifactCandidate],
) -> Result<bool, crate::artifact::canonical_key::ResolveError> {
    let mut matches: Vec<_> = candidates
        .iter()
        .filter(|candidate| candidate.canonical_key == key)
        .cloned()
        .collect();
    matches.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    if matches.len() > 1 {
        return Err(
            crate::artifact::canonical_key::ResolveError::ConflictingCanonicalKey {
                key: key.to_string(),
                candidates: matches,
            },
        );
    }
    Ok(!matches.is_empty())
}

/// Lockless preview uses the allocator's same read-only sources; apply still
/// acquires the allocator guard and recalculates before reserving.
fn worktree_roots_from_porcelain(output: &[u8]) -> anyhow::Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    for record in output.split(|byte| *byte == 0) {
        let Some(path) = record.strip_prefix(b"worktree ") else {
            continue;
        };
        if path.is_empty() {
            anyhow::bail!("git worktree list returned an empty worktree path");
        }
        #[cfg(unix)]
        let root = {
            use std::os::unix::ffi::OsStringExt;
            PathBuf::from(std::ffi::OsString::from_vec(path.to_vec()))
        };
        #[cfg(not(unix))]
        let root = PathBuf::from(std::str::from_utf8(path)?);
        roots.push(root);
    }
    Ok(roots)
}

async fn unlocked_max_display_number(workspace: &Path, kind: &ArtifactKind) -> anyhow::Result<u32> {
    use crate::artifact::frontmatter::{
        assigned_number_from_frontmatter, parse_frontmatter, predicted_number_from_frontmatter,
    };
    let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
    let display_prefix = format!("{prefix}-");
    let mut max = crate::artifact::id_alloc::load_id_ledger(workspace, kind)
        .await?
        .map_or(0, |ledger| ledger.high_watermark);
    let mut roots = vec![workspace.to_path_buf()];
    // The `git worktree list` root is the CURRENT worktree's top level, not
    // the parent of the git common dir. Those differ whenever the git
    // directory is not `<repo>/.git`: a submodule's git dir lives under the
    // superproject's `.git/modules/<name>`, and `--separate-git-dir` puts it
    // anywhere at all. Deriving the root from the common dir's parent then
    // enumerates the SUPERPROJECT's worktrees — repositories that are not
    // this one — and reads numbers from unrelated checkouts. Running the
    // command inside this worktree always enumerates the worktrees of the
    // repository this worktree belongs to, which is the set the numbering
    // has to cover. Same rule as `id_alloc::linked_worktree_roots`.
    if let Some(repo_root) =
        crate::workspace::git::git_show_toplevel(workspace.parent().unwrap_or(workspace))
    {
        let output = tokio::process::Command::new("git")
            .arg("-C")
            .arg(&repo_root)
            .args(["worktree", "list", "--porcelain", "-z"])
            .output()
            .await?;
        if !output.status.success() {
            anyhow::bail!("git worktree list failed with {}", output.status);
        }
        for root in worktree_roots_from_porcelain(&output.stdout)? {
            let path = root.join(".forgeplan");
            if !roots.contains(&path) {
                roots.push(path);
            }
        }
    }
    for root in roots {
        let mut stack = vec![root.join(kind.dir_name())];
        while let Some(dir) = stack.pop() {
            let mut entries = match tokio::fs::read_dir(&dir).await {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if tokio::fs::metadata(&path).await?.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                    continue;
                }
                let name = entry.file_name();
                if let Some(name) = name.to_str() {
                    let stem = name.strip_suffix(".md").unwrap_or(name);
                    if let Some(number) = stem
                        .strip_prefix(&display_prefix)
                        .and_then(|rest| rest.split('-').next())
                        .and_then(|number| number.parse::<u32>().ok())
                    {
                        max = max.max(number);
                    }
                }
                let content = tokio::fs::read_to_string(&path).await?;
                let (fm, _) = parse_frontmatter(&content)?;
                if let Some(number) = assigned_number_from_frontmatter(&fm) {
                    max = max.max(number);
                }
                if let Some(number) = predicted_number_from_frontmatter(&fm) {
                    max = max.max(number);
                }
                if let Some(id) = fm.get("id").and_then(|v| v.as_str())
                    && let Some(number) = id
                        .to_uppercase()
                        .strip_prefix(&format!("{prefix}-"))
                        .and_then(|n| n.parse::<u32>().ok())
                {
                    max = max.max(number);
                }
            }
        }
    }
    Ok(max)
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

    #[test]
    fn worktree_porcelain_paths_preserve_whitespace_and_unix_bytes() {
        let roots =
            worktree_roots_from_porcelain(b"worktree /checkout with trailing  \0HEAD abc\0\0")
                .unwrap();
        assert_eq!(roots, vec![PathBuf::from("/checkout with trailing  ")]);

        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let roots = worktree_roots_from_porcelain(b"worktree /checkout/\xff \0").unwrap();
            assert_eq!(roots.len(), 1);
            assert_eq!(roots[0].as_os_str().as_bytes(), b"/checkout/\xff ");
        }
    }

    #[tokio::test]
    async fn preview_ignores_digits_in_canonical_key_filenames() {
        let (_tmp, ws) = workspace().await;
        std::fs::write(
            ws.join("prds/prd-2024-review-k0123456789abcdef0123456789abcdef.md"),
            "---\nkind: prd\nid: PRD-007\nslug: prd-2024-review-k0123456789abcdef0123456789abcdef\ntitle: Review\n---\nBody\n",
        )
        .unwrap();

        assert_eq!(
            unlocked_max_display_number(&ws, &ArtifactKind::Prd)
                .await
                .unwrap(),
            7
        );
    }

    #[tokio::test]
    async fn import_preserves_lowercase_legacy_identity_and_existing_document() {
        let (tmp, ws) = workspace().await;
        let destination = ws.join("prds/PRD-074-existing.md");
        let original =
            "---\nkind: prd\nid: prd-074\nslug: prd-074\ntitle: Existing\n---\nExisting body\n";
        std::fs::write(&destination, original).unwrap();
        source(
            tmp.path(),
            "docs/prd-074-source.md",
            "---\nkind: prd\nid: prd-074\ntitle: Existing\n---\nSource body\n",
        );
        let lock = guard(&ws).await;
        let options = ScanImportOptions::default();
        let kinds = scan_import_kinds(tmp.path(), &options).unwrap();
        let allocation = crate::artifact::id_alloc::acquire_allocation_guard(&ws, &lock, &kinds)
            .await
            .unwrap();
        let plan = plan_scan_import(tmp.path(), &ws, &lock, &allocation, None, &options)
            .await
            .unwrap();

        assert_eq!(plan.identities[0].0, "prd-074");
        assert_eq!(plan.identities[0].1, "prd-074");
        assert!(plan.existing.contains(Path::new("docs/prd-074-source.md")));

        let result =
            execute_scan_import_plan(tmp.path(), &ws, &lock, &allocation, None, &options, &plan)
                .await
                .unwrap();
        assert_eq!(result.skipped, 1);
        assert_eq!(std::fs::read_to_string(&destination).unwrap(), original);
        let candidates = crate::artifact::documents::workspace_candidates(&ws)
            .await
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].canonical_key, "prd-074");
        assert_eq!(candidates[0].display_id, "prd-074");
    }

    #[tokio::test]
    async fn imported_legacy_id_becomes_a_stable_canonical_key() {
        let (tmp, ws) = workspace().await;
        source(
            tmp.path(),
            "docs/prd-074-legacy.md",
            "---\nkind: prd\nid: prd-074\ntitle: Legacy\n---\nLegacy body\n",
        );
        let lock = guard(&ws).await;
        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(result.imported, 1);

        let candidates = crate::artifact::documents::workspace_candidates(&ws)
            .await
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].canonical_key, "prd-074");
        assert_eq!(candidates[0].display_id, "prd-074");
        let content = std::fs::read_to_string(ws.join(&candidates[0].relative_path)).unwrap();
        let (_, body) = crate::artifact::frontmatter::parse_frontmatter(&content).unwrap();
        assert_eq!(body.trim(), "Legacy body");
    }

    #[tokio::test]
    async fn imported_local_links_use_canonical_keys_but_existing_links_stay_unchanged() {
        let (tmp, ws) = workspace().await;
        let historical = ws.join("prds/PRD-003-existing.md");
        let historical_bytes = "---\nkind: prd\nid: PRD-003\nslug: prd-existing\ntitle: Existing\nlinks:\n  - target: PRD-002\n    relation: informs\n---\nHistorical\n";
        std::fs::write(&historical, historical_bytes).unwrap();
        source(
            tmp.path(),
            "docs/PRD-002-target.md",
            "---\nkind: prd\nid: PRD-002\nslug: prd-target\ntitle: Target\n---\nTarget\n",
        );
        source(
            tmp.path(),
            "docs/PRD-001-new.md",
            "---\nkind: prd\nid: PRD-001\nslug: prd-new\ntitle: New\nlinks:\n  - target: PRD-002\n    relation: informs\n---\nNew\n",
        );
        source(
            tmp.path(),
            "docs/PRD-003-existing.md",
            "---\nkind: prd\nid: PRD-003\nslug: prd-existing\ntitle: Existing\nlinks:\n  - target: PRD-002\n    relation: informs\n---\nDifferent source body\n",
        );
        let lock = guard(&ws).await;
        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(result.imported, 2);
        assert_eq!(result.skipped, 1);
        assert_eq!(
            std::fs::read_to_string(&historical).unwrap(),
            historical_bytes
        );
        let (_, content) = imported_file(&ws, "prds", "PRD-001-").unwrap();
        let (frontmatter, _) = crate::artifact::frontmatter::parse_frontmatter(&content).unwrap();
        let links = frontmatter.get("links").unwrap().as_sequence().unwrap();
        assert_eq!(
            links[0].get("target").and_then(|value| value.as_str()),
            Some("prd-target")
        );
    }

    fn git_is_available() -> bool {
        std::process::Command::new("git")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn git(cwd: &Path, args: &[&str]) -> std::process::Output {
        std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_AUTHOR_NAME", "scan-import")
            .env("GIT_AUTHOR_EMAIL", "scan-import@example.com")
            .env("GIT_COMMITTER_NAME", "scan-import")
            .env("GIT_COMMITTER_EMAIL", "scan-import@example.com")
            .output()
            .expect("git invocation failed")
    }

    /// The lockless preview counts the numbers of the repository the
    /// workspace belongs to — never of a neighbouring repository.
    ///
    /// The worktree list is rooted at the CURRENT worktree's top level. A
    /// root derived from the git common dir's parent names a different
    /// repository whenever the git directory is not `<repo>/.git`: a
    /// submodule's git dir lives under the superproject's
    /// `.git/modules/<name>`, and `--separate-git-dir` puts it anywhere.
    /// The preview would then read numbers from unrelated checkouts and
    /// hand out a number another repository already uses.
    ///
    /// The superproject below is exactly that case: its `.git/modules/<name>`
    /// is the submodule's common dir, and the superproject carries a
    /// `PRD-900` that the submodule must never see.
    #[tokio::test]
    async fn the_preview_counts_the_numbers_of_its_own_repository_only() {
        if !git_is_available() {
            return;
        }
        let tmp = TempDir::new().unwrap();

        // An origin repository the submodule is cloned from.
        let origin = tmp.path().join("origin");
        std::fs::create_dir_all(&origin).unwrap();
        git(&origin, &["init", "-q"]);
        std::fs::write(origin.join("README.md"), "seed\n").unwrap();
        git(&origin, &["add", "."]);
        git(&origin, &["commit", "-q", "-m", "init"]);

        let superproject = tmp.path().join("superproject");
        std::fs::create_dir_all(&superproject).unwrap();
        git(&superproject, &["init", "-q"]);
        std::fs::write(superproject.join("README.md"), "seed\n").unwrap();
        git(&superproject, &["add", "."]);
        git(&superproject, &["commit", "-q", "-m", "init"]);

        // The superproject holds a document with a high number of its own.
        let super_ws = superproject.join(".forgeplan");
        std::fs::create_dir_all(super_ws.join("prds")).unwrap();
        std::fs::write(
            super_ws.join("prds/PRD-900-super.md"),
            "---\nid: PRD-900\nkind: prd\ntitle: Super\n---\nSuper body\n",
        )
        .unwrap();

        // A submodule of that superproject: its git dir is
        // `<superproject>/.git/modules/<name>`.
        let out = git(
            &superproject,
            &[
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "add",
                "-q",
                origin.to_str().unwrap(),
                "module",
            ],
        );
        if !out.status.success() {
            // A sandbox without the file protocol cannot build a submodule;
            // the assertion below needs one, so skip rather than fail.
            return;
        }
        let module = superproject.join("module");

        // The submodule's own document — the number it must hand out next.
        let ws = module.join(".forgeplan");
        for dir in crate::workspace::ARTIFACT_DIRS {
            std::fs::create_dir_all(ws.join(dir)).unwrap();
        }
        std::fs::write(
            ws.join("prds/PRD-007-module.md"),
            "---\nid: PRD-007\nkind: prd\ntitle: Module\n---\nModule body\n",
        )
        .unwrap();

        let max = unlocked_max_display_number(&ws, &ArtifactKind::Prd)
            .await
            .unwrap();
        assert_eq!(
            max, 7,
            "the preview must count its own repository's numbers and no other's"
        );
    }

    /// The same rule for a repository whose git directory does not live in
    /// the working tree at all (`--separate-git-dir`).
    ///
    /// The common dir's parent is then an unrelated directory that is not a
    /// repository at all, so the worktree list either fails outright or names
    /// somebody else's checkouts. The preview must still read the numbers of
    /// the repository the workspace belongs to.
    #[tokio::test]
    async fn the_preview_counts_the_numbers_of_a_repository_with_a_separate_git_dir() {
        if !git_is_available() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let git_dir = tmp.path().join("git-dir");
        let worktree = tmp.path().join("worktree");
        std::fs::create_dir_all(&worktree).unwrap();
        let out = git(
            &worktree,
            &[
                "init",
                "-q",
                "--separate-git-dir",
                git_dir.to_str().unwrap(),
            ],
        );
        assert!(
            out.status.success(),
            "git init --separate-git-dir failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );

        let ws = worktree.join(".forgeplan");
        for dir in crate::workspace::ARTIFACT_DIRS {
            std::fs::create_dir_all(ws.join(dir)).unwrap();
        }
        std::fs::write(
            ws.join("prds/PRD-007-separate.md"),
            "---\nid: PRD-007\nkind: prd\ntitle: Separate\n---\nSeparate body\n",
        )
        .unwrap();

        let max = unlocked_max_display_number(&ws, &ArtifactKind::Prd)
            .await
            .unwrap();
        assert_eq!(
            max, 7,
            "the preview must count the numbers of the repository the workspace belongs to"
        );
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
    async fn conflicting_existing_canonical_key_refuses_before_any_import() {
        let (tmp, ws) = workspace().await;
        for (name, id) in [("one.md", "PRD-001"), ("two.md", "PRD-002")] {
            std::fs::write(
                ws.join("prds").join(name),
                format!(
                    "---\nkind: prd\nid: {id}\nslug: prd-conflict\ntitle: Existing\n---\nExisting\n"
                ),
            )
            .unwrap();
        }
        source(
            tmp.path(),
            "docs/a.md",
            "---\nkind: prd\nid: PRD-001\nslug: prd-conflict\ntitle: Existing\n---\nSource\n",
        );
        source(
            tmp.path(),
            "docs/b.md",
            "---\nkind: prd\nid: PRD-003\nslug: prd-new\ntitle: New\n---\nNew\n",
        );
        let lock = guard(&ws).await;
        let kinds = scan_import_kinds(tmp.path(), &ScanImportOptions::default()).unwrap();
        let allocation = crate::artifact::id_alloc::acquire_allocation_guard(&ws, &lock, &kinds)
            .await
            .unwrap();
        let before = inventory(&ws);
        let error = plan_scan_import(
            tmp.path(),
            &ws,
            &lock,
            &allocation,
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap_err();
        match error.downcast_ref::<crate::artifact::canonical_key::ResolveError>() {
            Some(crate::artifact::canonical_key::ResolveError::ConflictingCanonicalKey {
                key,
                candidates,
            }) => {
                assert_eq!(key, "prd-conflict");
                assert_eq!(candidates.len(), 2);
            }
            other => panic!("expected canonical conflict, got {other:?}"),
        }
        assert_eq!(inventory(&ws), before);
        assert!(!ws.join("prds/PRD-003-new.md").exists());
    }

    #[tokio::test]
    async fn invalid_whitespace_slug_uses_identical_key_in_markdown_and_index() {
        let (tmp, ws) = workspace().await;
        let lock = guard(&ws).await;
        let store = LanceStore::init_new(&ws, &lock).await.unwrap();
        source(
            tmp.path(),
            "docs/PRD-001.md",
            "---\nkind: prd\nid: PRD-001\nslug: ' prd-child '\ntitle: Child\n---\nBody\n",
        );
        let options = ScanImportOptions::default();
        let kinds = scan_import_kinds(tmp.path(), &options).unwrap();
        let allocation = crate::artifact::id_alloc::acquire_allocation_guard(&ws, &lock, &kinds)
            .await
            .unwrap();
        let plan = plan_scan_import(tmp.path(), &ws, &lock, &allocation, Some(&store), &options)
            .await
            .unwrap();
        assert_eq!(plan.identities[0].1, "PRD-001");
        let result = execute_scan_import_plan(
            tmp.path(),
            &ws,
            &lock,
            &allocation,
            Some(&store),
            &options,
            &plan,
        )
        .await
        .unwrap();
        assert_eq!(result.imported, 1, "{:?}", result.entries);
        let (_, content) = imported_file(&ws, "prds", "PRD-001-").unwrap();
        assert_eq!(
            crate::artifact::canonical_key::canonical_key_from_document("PRD-001", &content),
            "PRD-001"
        );
        let row = store.get_record(&lock, "PRD-001").await.unwrap().unwrap();
        assert_eq!(row.canonical_key, "PRD-001");
    }

    #[tokio::test]
    async fn dry_run_matches_apply_for_generated_ids_and_existing_documents() {
        let (tmp, ws) = workspace().await;
        std::fs::write(
            ws.join("prds/PRD-004-existing.md"),
            "---\nkind: prd\nid: PRD-004\nslug: prd-existing\ntitle: Existing\n---\nBody\n",
        )
        .unwrap();
        source(
            tmp.path(),
            "docs/a.md",
            "---\nkind: prd\ntitle: First\n---\nFirst\n",
        );
        source(
            tmp.path(),
            "docs/b.md",
            "---\nkind: prd\ntitle: Second\n---\nSecond\n",
        );
        source(
            tmp.path(),
            "docs/c.md",
            "---\nkind: prd\nid: PRD-004\nslug: prd-existing\ntitle: Existing\n---\nBody\n",
        );
        let before = inventory(&ws);
        let preview = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            None,
            None,
            None,
            &ScanImportOptions {
                dry_run: true,
                custom_path: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(inventory(&ws), before);
        let lock = guard(&ws).await;
        let applied = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(preview.failed, 0);
        assert_eq!(applied.failed, 0, "{:?}", applied.entries);
        let summarize = |result: &ScanImportResult| {
            result
                .entries
                .iter()
                .map(|entry| {
                    (
                        entry.relative_path.clone(),
                        entry.artifact_id.clone(),
                        entry.status.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(summarize(&preview), summarize(&applied));
        assert_eq!(preview.imported, 2);
        assert_eq!(preview.skipped, 1);
        assert!(
            preview
                .entries
                .iter()
                .any(|e| e.artifact_id.as_deref() == Some("PRD-005"))
        );
        assert!(
            preview
                .entries
                .iter()
                .any(|e| e.artifact_id.as_deref() == Some("PRD-006"))
        );
    }

    #[tokio::test]
    async fn apply_preserves_forward_and_qualified_parents() {
        let (tmp, ws) = workspace().await;
        for (id, parent) in [("PRD-001", "EPIC-A01"), ("PRD-002", "EPIC-A02@other")] {
            source(
                tmp.path(),
                &format!("docs/{id}.md"),
                &format!(
                    "---\nkind: prd\nid: {id}\ntitle: {id}\nparent_epic: {parent}\n---\nBody\n"
                ),
            );
        }
        let lock = guard(&ws).await;
        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            None,
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(result.imported, 2, "{:?}", result.entries);
        for (id, parent) in [("PRD-001", "EPIC-A01"), ("PRD-002", "EPIC-A02@other")] {
            let (_, content) = imported_file(&ws, "prds", id).unwrap();
            let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&content).unwrap();
            assert_eq!(fm.get("parent_epic").and_then(|v| v.as_str()), Some(parent));
        }
    }

    #[tokio::test]
    async fn apply_preserves_metadata_and_resolves_batch_parent_in_both_paths() {
        for legacy_enabled in [false, true] {
            for prepared in [false, true] {
                let (tmp, ws) = workspace().await;
                let lock = guard(&ws).await;
                let store = if legacy_enabled {
                    Some(LanceStore::init_new(&ws, &lock).await.unwrap())
                } else {
                    None
                };
                source(
                    tmp.path(),
                    "docs/EPIC-001-parent.md",
                    "---\nkind: epic\nid: EPIC-001\nslug: epic-parent\ntitle: Parent\n---\nParent body\n",
                );
                source(
                    tmp.path(),
                    "docs/PRD-001-child.md",
                    "---\nkind: prd\nid: PRD-001\nslug: prd-child\ntitle: Child\ndepth: deep\nparent_epic: EPIC-001\nvalid_until: '2030-12-31'\n---\nChild body\n",
                );
                let options = ScanImportOptions::default();
                let result = if prepared {
                    let kinds = scan_import_kinds(tmp.path(), &options).unwrap();
                    let allocation =
                        crate::artifact::id_alloc::acquire_allocation_guard(&ws, &lock, &kinds)
                            .await
                            .unwrap();
                    let plan = plan_scan_import(
                        tmp.path(),
                        &ws,
                        &lock,
                        &allocation,
                        store.as_ref(),
                        &options,
                    )
                    .await
                    .unwrap();
                    execute_scan_import_plan(
                        tmp.path(),
                        &ws,
                        &lock,
                        &allocation,
                        store.as_ref(),
                        &options,
                        &plan,
                    )
                    .await
                    .unwrap()
                } else {
                    scan_and_import_to_workspace(
                        tmp.path(),
                        &ws,
                        Some(&lock),
                        None,
                        store.as_ref(),
                        &options,
                    )
                    .await
                    .unwrap()
                };
                assert_eq!(result.imported, 2, "{:?}", result.entries);
                let (_, content) = imported_file(&ws, "prds", "PRD-001-").unwrap();
                let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&content).unwrap();
                assert_eq!(fm.get("depth").and_then(|v| v.as_str()), Some("deep"));
                assert_eq!(
                    fm.get("parent_epic").and_then(|v| v.as_str()),
                    Some("epic-parent")
                );
                assert_eq!(
                    fm.get("valid_until").and_then(|v| v.as_str()),
                    Some("2030-12-31")
                );
                if let Some(store) = &store {
                    let row = store.get_record(&lock, "prd-child").await.unwrap().unwrap();
                    assert_eq!(row.depth, "deep");
                    assert_eq!(row.parent_epic.as_deref(), Some("epic-parent"));
                    assert_eq!(row.valid_until.as_deref(), Some("2030-12-31"));
                }
            }
        }
    }

    /// A parent the workspace cannot resolve stays literal in BOTH records.
    ///
    /// A forward reference (`EPIC-A01`) and a qualified cross-source
    /// reference (`EPIC-A02@other`) name no local document, so the import
    /// must keep the bytes the source declared. The derived row keeps them
    /// too, together with the document's own `depth` and `valid_until` — a
    /// row that loses the parent or the metadata misreports the document it
    /// indexes.
    #[tokio::test]
    async fn apply_keeps_unresolved_parents_and_metadata_in_the_legacy_row() {
        let (tmp, ws) = workspace().await;
        let lock = guard(&ws).await;
        let store = LanceStore::init_new(&ws, &lock).await.unwrap();
        source(
            tmp.path(),
            "docs/PRD-001-forward.md",
            "---\nkind: prd\nid: PRD-001\nslug: prd-forward\ntitle: Forward\ndepth: deep\nparent_epic: EPIC-A01\nvalid_until: '2030-12-31'\n---\nForward body\n",
        );
        source(
            tmp.path(),
            "docs/PRD-002-qualified.md",
            "---\nkind: prd\nid: PRD-002\nslug: prd-qualified\ntitle: Qualified\nparent_epic: EPIC-A02@other\n---\nQualified body\n",
        );
        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            Some(&store),
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(result.imported, 2, "{:?}", result.entries);

        let (_, content) = imported_file(&ws, "prds", "PRD-001-").unwrap();
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&content).unwrap();
        assert_eq!(
            fm.get("parent_epic").and_then(|v| v.as_str()),
            Some("EPIC-A01")
        );
        assert_eq!(fm.get("depth").and_then(|v| v.as_str()), Some("deep"));
        assert_eq!(
            fm.get("valid_until").and_then(|v| v.as_str()),
            Some("2030-12-31")
        );

        let forward = store
            .get_record(&lock, "prd-forward")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            forward.parent_epic.as_deref(),
            Some("EPIC-A01"),
            "a forward reference keeps its declared bytes in the index"
        );
        assert_eq!(forward.depth, "deep");
        assert_eq!(forward.valid_until.as_deref(), Some("2030-12-31"));

        let (_, content) = imported_file(&ws, "prds", "PRD-002-").unwrap();
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&content).unwrap();
        assert_eq!(
            fm.get("parent_epic").and_then(|v| v.as_str()),
            Some("EPIC-A02@other")
        );
        let qualified = store
            .get_record(&lock, "prd-qualified")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            qualified.parent_epic.as_deref(),
            Some("EPIC-A02@other"),
            "a qualified reference keeps its declared bytes in the index"
        );
    }

    /// Two documents may share a display id as long as they keep different
    /// canonical keys.
    ///
    /// The import must admit both, keep each body with its own document, and
    /// leave the shared alias an explicit ambiguity — never a silent winner
    /// (SPEC-009 "One resolution algorithm", rule 3). The derived index
    /// refuses the shared alias for the same reason.
    #[tokio::test]
    async fn imported_documents_sharing_a_display_id_stay_separate_by_key() {
        let (tmp, ws) = workspace().await;
        let lock = guard(&ws).await;
        let store = LanceStore::init_new(&ws, &lock).await.unwrap();
        for (file, slug, title, body) in [
            ("one.md", "prd-one", "One", "First body"),
            ("two.md", "prd-two", "Two", "Second body"),
        ] {
            source(
                tmp.path(),
                &format!("docs/{file}"),
                &format!(
                    "---\nkind: prd\nid: PRD-001\nslug: {slug}\ntitle: {title}\n---\n{body}\n"
                ),
            );
        }
        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            Some(&store),
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(result.imported, 2, "{:?}", result.entries);

        let candidates = crate::artifact::documents::workspace_candidates(&ws)
            .await
            .unwrap();
        assert_eq!(candidates.len(), 2, "{candidates:?}");
        for (key, body) in [("prd-one", "First body"), ("prd-two", "Second body")] {
            let resolved = crate::artifact::canonical_key::resolve_artifact(key, &candidates)
                .unwrap()
                .unwrap();
            assert_eq!(resolved.identity.canonical_key, key);
            let content = std::fs::read_to_string(ws.join(&resolved.relative_path)).unwrap();
            assert!(content.contains(body), "{key} must keep its own body");
        }
        match crate::artifact::canonical_key::resolve_artifact("PRD-001", &candidates) {
            Err(crate::artifact::canonical_key::ResolveError::Ambiguous { input, candidates }) => {
                assert_eq!(input, "PRD-001");
                assert_eq!(candidates.len(), 2, "the ambiguity names both claimants");
            }
            other => panic!("expected an ambiguity for the shared display id, got {other:?}"),
        }
        assert!(
            store.get_record(&lock, "PRD-001").await.is_err(),
            "the index must not pick a winner for a shared display id"
        );
    }

    #[tokio::test]
    async fn distinct_keys_share_display_id_without_clobber_and_generated_id_is_fresh() {
        for legacy_enabled in [false, true] {
            let (tmp, ws) = workspace().await;
            let lock = guard(&ws).await;
            let store = if legacy_enabled {
                Some(LanceStore::init_new(&ws, &lock).await.unwrap())
            } else {
                None
            };
            source(
                tmp.path(),
                "docs/a-generated.md",
                "---\nkind: prd\ntitle: Generated\n---\nGenerated body\n",
            );
            for (file, slug, body) in [
                ("b.md", "prd-one", "First body"),
                ("c.md", "prd-two", "Second body"),
            ] {
                source(
                    tmp.path(),
                    &format!("docs/{file}"),
                    &format!(
                        "---\nkind: prd\nid: PRD-001\nslug: {slug}\ntitle: Same title\n---\n{body}\n"
                    ),
                );
            }
            let result = scan_and_import_to_workspace(
                tmp.path(),
                &ws,
                Some(&lock),
                None,
                store.as_ref(),
                &ScanImportOptions::default(),
            )
            .await
            .unwrap();
            assert_eq!(result.imported, 3, "{:?}", result.entries);
            assert!(
                result
                    .entries
                    .iter()
                    .any(|entry| entry.relative_path.ends_with("a-generated.md")
                        && entry.artifact_id.as_deref() == Some("PRD-002"))
            );
            let candidates = crate::artifact::documents::workspace_candidates(&ws)
                .await
                .unwrap();
            for (key, body) in [("prd-one", "First body"), ("prd-two", "Second body")] {
                let candidate = candidates
                    .iter()
                    .find(|candidate| candidate.canonical_key == key)
                    .unwrap();
                let content = std::fs::read_to_string(ws.join(&candidate.relative_path)).unwrap();
                assert!(content.contains(body));
                if let Some(store) = &store {
                    assert_eq!(
                        store
                            .get_record(&lock, key)
                            .await
                            .unwrap()
                            .unwrap()
                            .body
                            .trim(),
                        body
                    );
                }
            }
            let repeat = scan_and_import_to_workspace(
                tmp.path(),
                &ws,
                Some(&lock),
                None,
                store.as_ref(),
                &ScanImportOptions::default(),
            )
            .await
            .unwrap();
            assert_eq!(repeat.skipped, 2);
            assert_eq!(repeat.failed, 0, "{:?}", repeat.entries);
            assert_eq!(repeat.imported, 1);
            assert_eq!(repeat.entries.len(), 3);
        }
    }

    #[tokio::test]
    async fn seed_publication_does_not_replace_an_existing_destination() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("existing.md");
        std::fs::write(&path, "original").unwrap();
        assert!(atomic_write(&path, b"replacement").await.is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "original");
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
        let store = {
            let guard = crate::workspace::lock::acquire_workspace_lock(&ws)
                .await
                .unwrap();
            LanceStore::init_new(&ws, &guard).await.unwrap()
        };
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
            None,
            Some(&store),
            &ScanImportOptions {
                dry_run: true,
                custom_path: None,
            },
        )
        .await
        .unwrap();

        assert!(
            store
                .get_artifact(&lock, "PRD-001")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn legacy_preview_requires_and_reuses_shared_read_permit() {
        let (tmp, ws) = workspace().await;
        let store = {
            let lock = guard(&ws).await;
            LanceStore::init_new(&ws, &lock).await.unwrap()
        };
        source(
            tmp.path(),
            "docs/draft.md",
            "---\nkind: prd\ntitle: Preview\n---\n# Preview",
        );
        let options = ScanImportOptions {
            dry_run: true,
            custom_path: None,
        };
        assert!(
            scan_and_import_to_workspace(tmp.path(), &ws, None, None, Some(&store), &options)
                .await
                .is_err()
        );
        let permit = crate::workspace::lock::acquire_workspace_read_lock(&ws)
            .await
            .unwrap();
        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            None,
            Some(&permit),
            Some(&store),
            &options,
        )
        .await
        .unwrap();
        assert_eq!(result.imported, 1);
        assert!(
            store
                .get_artifact(&permit, "PRD-001")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn planner_refuses_a_guard_from_another_workspace() {
        let (tmp, ws) = workspace().await;
        source(
            tmp.path(),
            "docs/PRD-001.md",
            "---\nkind: prd\nid: PRD-001\ntitle: Test\n---\n# Test\n",
        );
        let lock = guard(&ws).await;
        let other = tempfile::tempdir().unwrap();
        let other_lock = guard(&other.path().join(".forgeplan")).await;
        let options = ScanImportOptions::default();
        let kinds = scan_import_kinds(tmp.path(), &options).unwrap();
        let allocation = crate::artifact::id_alloc::acquire_allocation_guard(&ws, &lock, &kinds)
            .await
            .unwrap();
        let before = inventory(&ws);
        assert!(
            plan_scan_import(tmp.path(), &ws, &other_lock, &allocation, None, &options)
                .await
                .is_err()
        );
        assert_eq!(inventory(&ws), before);
    }

    #[tokio::test]
    async fn scan_preflight_accepts_forward_links_and_parents_but_not_invalid_targets() {
        let (tmp, ws) = workspace().await;
        let input = |link: &str, parent: &str| {
            format!(
                "---\nkind: prd\nid: PRD-001\ntitle: Source\nparent_epic: {parent}\nlinks:\n  - target: {link}\n    relation: informs\n---\n# Source\n"
            )
        };
        let path = source(tmp.path(), "docs/PRD-001.md", &input("RFC-A01", "EPIC-A01"));
        let lock = guard(&ws).await;
        let options = ScanImportOptions::default();
        let kinds = scan_import_kinds(tmp.path(), &options).unwrap();
        let allocation = crate::artifact::id_alloc::acquire_allocation_guard(&ws, &lock, &kinds)
            .await
            .unwrap();
        let before = inventory(&ws);
        let plan = plan_scan_import(tmp.path(), &ws, &lock, &allocation, None, &options)
            .await
            .unwrap();
        assert!(
            plan.identities
                .iter()
                .any(|(_, _, links)| links.iter().any(|(target, _)| target == "RFC-A01"))
        );
        assert_eq!(inventory(&ws), before);
        for bad in ["---", "EPIC-4294967296"] {
            std::fs::write(&path, input(bad, "EPIC-A01")).unwrap();
            assert!(
                plan_scan_import(tmp.path(), &ws, &lock, &allocation, None, &options)
                    .await
                    .is_err()
            );
            std::fs::write(&path, input("RFC-A01", bad)).unwrap();
            assert!(
                plan_scan_import(tmp.path(), &ws, &lock, &allocation, None, &options)
                    .await
                    .is_err()
            );
        }
        std::fs::create_dir_all(ws.join("rfcs")).unwrap();
        for (name, slug) in [("one.md", "rfc-one"), ("two.md", "rfc-two")] {
            std::fs::write(
                ws.join("rfcs").join(name),
                format!("---\nid: RFC-001\nkind: rfc\ntitle: {slug}\nslug: {slug}\n---\n\nBody\n"),
            )
            .unwrap();
        }
        std::fs::write(&path, input("RFC-001", "EPIC-A01")).unwrap();
        let error = plan_scan_import(tmp.path(), &ws, &lock, &allocation, None, &options)
            .await
            .unwrap_err();
        assert!(matches!(
            error.downcast_ref::<crate::artifact::canonical_key::ResolveError>(),
            Some(crate::artifact::canonical_key::ResolveError::Ambiguous { .. })
        ));
    }

    #[tokio::test]
    async fn prepared_scan_rejects_changed_source_without_importing() {
        let (tmp, ws) = workspace().await;
        let path = source(
            tmp.path(),
            "docs/PRD-001.md",
            "---\nkind: prd\nid: PRD-001\ntitle: Before\n---\n# Before\n",
        );
        let lock = guard(&ws).await;
        let options = ScanImportOptions::default();
        let kinds = scan_import_kinds(tmp.path(), &options).unwrap();
        let allocation = crate::artifact::id_alloc::acquire_allocation_guard(&ws, &lock, &kinds)
            .await
            .unwrap();
        let plan = plan_scan_import(tmp.path(), &ws, &lock, &allocation, None, &options)
            .await
            .unwrap();
        std::fs::write(
            path,
            "---\nkind: prd\nid: PRD-001\ntitle: After\n---\n# After\n",
        )
        .unwrap();
        let before = inventory(&ws);
        assert!(
            execute_scan_import_plan(tmp.path(), &ws, &lock, &allocation, None, &options, &plan)
                .await
                .is_err()
        );
        assert_eq!(inventory(&ws), before);
        assert!(imported_file(&ws, "prds", "PRD-001-").is_none());
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
            None,
            &ScanImportOptions::default(),
        )
        .await
        .expect_err("a mutating run must not proceed without the lock");

        assert!(err.to_string().contains("lock"), "got: {err}");
        assert_eq!(inventory(&ws), before, "the refused run still wrote a file");
    }

    /// Invalid canonical directories reject the batch before any document is written.
    #[tokio::test]
    async fn an_invalid_workspace_is_rejected_before_importing() {
        let (tmp, ws) = workspace().await;
        source(
            tmp.path(),
            "docs/ADR-031-blocked.md",
            "---\nkind: adr\nid: ADR-031\ntitle: Blocked\n---\n# Blocked",
        );
        let existing = ws.join("prds/PRD-008-existing.md");
        let existing_bytes = "---\nid: PRD-008\nkind: prd\ntitle: Existing\n---\nExisting body\n";
        std::fs::write(&existing, existing_bytes).unwrap();
        let adrs = ws.join("adrs");
        std::fs::remove_dir_all(&adrs).unwrap();
        std::fs::write(&adrs, b"not a directory").unwrap();
        let lock = guard(&ws).await;

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            None,
            &ScanImportOptions::default(),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(existing).unwrap(), existing_bytes);
        assert_eq!(std::fs::read(adrs).unwrap(), b"not a directory");
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("docs/ADR-031-blocked.md")).unwrap(),
            "---\nkind: adr\nid: ADR-031\ntitle: Blocked\n---\n# Blocked",
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
            "---\nkind: prd\nid: PRD-009\nslug: prd-first\ntitle: First\n---\n# A",
        );
        source(
            tmp.path(),
            "docs/PRD-009-second.md",
            "---\nkind: prd\nid: PRD-009\nslug: prd-second\ntitle: Second\n---\n# B",
        );
        let lock = guard(&ws).await;

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
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
        let store = {
            let guard = crate::workspace::lock::acquire_workspace_lock(&ws)
                .await
                .unwrap();
            LanceStore::init_new(&ws, &guard).await.unwrap()
        };
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
            None,
            Some(&store),
            &ScanImportOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(result.imported, 1);

        let record = store.get_record(&lock, "PRD-007").await.unwrap().unwrap();
        assert_eq!(record.tags, vec!["source=code", "layer=auth"]);
        let (_, content) = imported_file(&ws, "prds", "PRD-007-").expect("canonical PRD written");
        assert!(
            content.contains("layer=auth"),
            "tags must reach the file too"
        );
    }

    /// An orphaned row cannot be safely attributed to an imported document.
    #[tokio::test]
    async fn an_orphaned_legacy_row_is_refused_without_mutation() {
        let (tmp, ws) = workspace().await;
        let store = {
            let guard = crate::workspace::lock::acquire_workspace_lock(&ws)
                .await
                .unwrap();
            LanceStore::init_new(&ws, &guard).await.unwrap()
        };
        let lock = guard(&ws).await;
        store
            .create_artifact(
                &lock,
                &NewArtifact {
                    canonical_key: "ADR-013".to_string(),
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
                },
            )
            .await
            .unwrap();
        source(
            tmp.path(),
            "docs/ADR-013-healed.md",
            "---\nkind: adr\nid: ADR-013\ntitle: Healed\nstatus: accepted\nauthor: Grace\n---\n# Healed body\n",
        );

        let options = ScanImportOptions::default();
        let kinds = scan_import_kinds(tmp.path(), &options).unwrap();
        let allocation = crate::artifact::id_alloc::acquire_allocation_guard(&ws, &lock, &kinds)
            .await
            .unwrap();
        let before = inventory(&ws);
        let before_record = store.get_record(&lock, "ADR-013").await.unwrap().unwrap();
        let error = plan_scan_import(tmp.path(), &ws, &lock, &allocation, Some(&store), &options)
            .await
            .expect_err("an unattributed orphan must fail closed");
        assert!(
            error
                .to_string()
                .contains("disagrees with the existing index")
        );
        assert_eq!(inventory(&ws), before);
        let after_record = store.get_record(&lock, "ADR-013").await.unwrap().unwrap();
        assert_eq!(after_record.body, before_record.body);
        assert_eq!(after_record.title, before_record.title);
        assert_eq!(after_record.status, before_record.status);
        assert!(imported_file(&ws, "adrs", "ADR-013-").is_none());
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
        let outside = TempDir::new().unwrap();
        let outside_bytes = "---\nkind: adr\nid: ADR-011\ntitle: External\n---\nExternal body\n";
        source(outside.path(), "ADR-011-external.md", outside_bytes);

        let result = scan_and_import_to_workspace(
            tmp.path(),
            &ws,
            Some(&lock),
            None,
            None,
            &ScanImportOptions {
                dry_run: false,
                custom_path: Some(outside.path().to_str().unwrap().to_string()),
            },
        )
        .await;

        assert!(result.is_err());
        assert!(
            crate::artifact::documents::workspace_candidates(&ws)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            std::fs::read_to_string(outside.path().join("ADR-011-external.md")).unwrap(),
            outside_bytes,
        );
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

    /// ADR-030: an imported document keeps the key it was written with — the
    /// slug its frontmatter declares, and its display id when it declares
    /// none. An import never mints a key.
    #[test]
    fn an_imported_document_keeps_the_key_it_declares() {
        let mut frontmatter: BTreeMap<String, serde_yaml::Value> = BTreeMap::new();
        frontmatter.insert(
            "slug".to_string(),
            serde_yaml::Value::String("prd-auth-system".to_string()),
        );
        let document = ParsedDocument {
            frontmatter: frontmatter.clone(),
            body: "body".to_string(),
            title: "Auth".to_string(),
            status: "draft".to_string(),
            tags: Vec::new(),
            author: "ada".to_string(),
            links: Vec::new(),
        };
        assert_eq!(
            canonical_key_for_import("PRD-001", &document),
            "prd-auth-system",
            "the declared slug is the key"
        );

        // A document without a slug keeps its display id.
        let without_slug = ParsedDocument {
            frontmatter: BTreeMap::new(),
            ..document.clone()
        };
        assert_eq!(
            canonical_key_for_import("PRD-002", &without_slug),
            "PRD-002",
            "a document without a slug keeps its display id"
        );

        // An invalid slug is not a slug: the display id is the key.
        let invalid = ParsedDocument {
            frontmatter: {
                let mut frontmatter = frontmatter.clone();
                frontmatter.insert(
                    "slug".to_string(),
                    serde_yaml::Value::String("Not A Slug".to_string()),
                );
                frontmatter
            },
            ..document
        };
        assert_eq!(
            canonical_key_for_import("PRD-003", &invalid),
            "PRD-003",
            "an invalid slug does not become a key"
        );
    }
}
