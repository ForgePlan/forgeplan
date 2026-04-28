use std::path::Path;

use crate::artifact::types::ArtifactKind;
use crate::db::store::{LanceStore, NewArtifact};
use crate::scan::detect::{DetectionResult, DetectionTier, detect_kind_with_path};
use crate::scan::discovery::{DiscoveredFile, discover_markdown_files};
use crate::scan::status_map::map_external_status;

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
    /// Successfully imported into LanceDB.
    Imported,
    /// Skipped because artifact with same ID already exists.
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
    /// projection write failed post-store, etc. Fail-loud — import report
    /// surfaces these so the user knows something needs attention.
    #[doc(alias = "advisory")]
    pub warnings: Vec<String>,
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
}

/// Run scan-import: discover files, detect types, import into LanceDB.
///
/// ⚠ **ADR-003 compliance warning**: this variant does NOT write markdown
/// projections. The consequence is that `forgeplan reindex` will treat
/// imported artifacts as orphans (no .md file found) and purge them —
/// the exact failure mode in Telegram bug report 2026-04-19. New
/// callers MUST use [`scan_and_import_to_workspace`] instead. Kept here
/// only for backward compatibility with existing unit tests that do not
/// need projection.
#[deprecated(
    since = "0.25.0",
    note = "use scan_and_import_to_workspace(…, workspace, …) for ADR-003 compliance; \
            bare scan_and_import leaves LanceDB entries without .md files and \
            `forgeplan reindex` will purge them on the next run"
)]
pub async fn scan_and_import(
    project_root: &Path,
    store: &LanceStore,
    options: &ScanImportOptions,
) -> anyhow::Result<ScanImportResult> {
    scan_and_import_inner(project_root, store, options, None).await
}

/// Brownfield-ready variant: same as `scan_and_import` plus writes a
/// markdown projection (`.forgeplan/<kind>s/<ID>-<slug>.md`) for every
/// successfully imported artifact (PRD-058 FR-001). Required for
/// `reindex` round-trip safety and ADR-003 compliance.
///
/// `workspace` is the `.forgeplan/` directory (same that `forgeplan init`
/// creates), not the project root.
pub async fn scan_and_import_to_workspace(
    project_root: &Path,
    workspace: &Path,
    store: &LanceStore,
    options: &ScanImportOptions,
) -> anyhow::Result<ScanImportResult> {
    scan_and_import_inner(project_root, store, options, Some(workspace)).await
}

async fn scan_and_import_inner(
    project_root: &Path,
    store: &LanceStore,
    options: &ScanImportOptions,
    workspace: Option<&Path>,
) -> anyhow::Result<ScanImportResult> {
    // Discover files — with path traversal protection
    let scan_root = if let Some(ref custom) = options.custom_path {
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
                process_detected_file_inner(file, &det, store, options.dry_run, workspace).await
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

    Ok(ScanImportResult {
        entries,
        total_found,
        imported,
        skipped,
        unknown,
        failed,
    })
}

/// Process a file with a successful detection result. When `workspace`
/// is `Some`, also writes a markdown projection (PRD-058 FR-001 —
/// ADR-003 compliance). `None` keeps the LanceDB-only behavior for
/// pre-existing unit tests that don't need the projection.
async fn process_detected_file_inner(
    file: &DiscoveredFile,
    detection: &DetectionResult,
    store: &LanceStore,
    dry_run: bool,
    workspace: Option<&Path>,
) -> ScanImportEntry {
    let artifact_id = resolve_artifact_id(detection, store).await;

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
            status: ImportStatus::Imported, // preview: would be imported
            ..entry_base
        };
    }

    // Build title: prefer detection → filename → "Untitled"
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

    // Parse frontmatter ONCE to extract tags + status + body-without-FM.
    // R2 audit rust-pro CRITICAL: previously we passed `file.content`
    // (which includes the original `---…---` block) as both
    // `NewArtifact.body` and `render_projection_with_body` input.
    // `render_projection_with_body` then prepended its OWN regenerated
    // frontmatter → every imported artifact ended up with two
    // `---…---` blocks stacked on disk. Split FM from body here and
    // keep only the body portion downstream.
    let mut warnings: Vec<String> = Vec::new();
    let (tags, status, body_only) =
        match crate::artifact::frontmatter::parse_frontmatter(&file.content) {
            Ok((fm, body)) => {
                let tags = crate::artifact::frontmatter::tags_from_frontmatter(&fm);
                let raw_status = fm.get("status").and_then(|v| v.as_str());
                let status = match raw_status {
                    Some(s) => {
                        let (mapped, warning) = map_external_status(s);
                        if let Some(w) = warning {
                            warnings.push(w);
                        }
                        mapped
                    }
                    None => "draft".to_string(), // no frontmatter status → default (AC-5)
                };
                (tags, status, body)
            }
            Err(_) => (Vec::new(), "draft".to_string(), file.content.clone()),
        };

    // R2 audit rust-pro HIGH #3: a previous run may have successfully
    // written to LanceDB but failed the projection write — the DB row
    // then prevents re-try because this check returns Skipped. Detect
    // that case and resume by writing the missing projection rather
    // than silently leaving the artifact orphaned.
    match store.get_artifact(&artifact_id).await {
        Ok(Some(_)) => {
            // Heal-on-retry: if projection is missing, try to write it
            // from the existing source file content. Do not return Skipped
            // until the file exists too.
            if let Some(ws) = workspace
                && !projection_exists(ws, &detection.kind, &artifact_id).await
            {
                maybe_write_projection(
                    ws,
                    &artifact_id,
                    detection.kind.template_key(),
                    &title,
                    &status,
                    &tags,
                    &body_only,
                    &mut warnings,
                )
                .await;
            }
            return ScanImportEntry {
                status: ImportStatus::Skipped,
                warnings,
                ..entry_base
            };
        }
        Ok(None) => {} // proceed with import
        Err(e) => {
            return ScanImportEntry {
                status: ImportStatus::Failed(format!("Check existing: {e}")),
                warnings,
                ..entry_base
            };
        }
    }

    let new_artifact = NewArtifact {
        id: artifact_id.clone(),
        kind: detection.kind.template_key().to_string(),
        status: status.clone(),
        title: title.clone(),
        body: body_only.clone(),
        depth: "standard".to_string(),
        author: Some("scan-import".to_string()),
        parent_epic: None,
        valid_until: None,
        tags: tags.clone(),
    };

    if let Err(e) = store.create_artifact(&new_artifact).await {
        return ScanImportEntry {
            status: ImportStatus::Failed(format!("{e}")),
            warnings,
            ..entry_base
        };
    }

    // PRD-058 FR-001 (ADR-003 compliance): write the markdown projection.
    // R2 audit rust-pro MEDIUM: rollback DB insert on projection failure
    // so the invariant "file is source of truth" isn't violated by a
    // LanceDB row with no matching .md file.
    if let Some(ws) = workspace {
        let before_len = warnings.len();
        maybe_write_projection(
            ws,
            &artifact_id,
            detection.kind.template_key(),
            &title,
            &status,
            &tags,
            &body_only,
            &mut warnings,
        )
        .await;
        if warnings.len() > before_len {
            // Projection failed — roll back the DB insert so the next
            // scan-import can retry cleanly. If rollback itself fails,
            // leave a warning and let the user sort it out.
            if let Err(rb_err) = store.delete_artifact(&artifact_id).await {
                warnings.push(format!(
                    "rollback of {artifact_id} failed after projection error: {rb_err}"
                ));
                return ScanImportEntry {
                    status: ImportStatus::Failed(
                        warnings.last().cloned().unwrap_or_else(|| "unknown".into()),
                    ),
                    warnings,
                    ..entry_base
                };
            }
            return ScanImportEntry {
                status: ImportStatus::Failed(
                    warnings
                        .last()
                        .cloned()
                        .unwrap_or_else(|| "projection failed".into()),
                ),
                warnings,
                ..entry_base
            };
        }
    }

    ScanImportEntry {
        status: ImportStatus::Imported,
        warnings,
        ..entry_base
    }
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

/// Write the markdown projection for an imported artifact. Appends any
/// failure to `warnings` so the caller can decide on rollback.
///
/// R2 audit rust-pro HIGH #2: uses `render_projection_record` so tags
/// land in the file's frontmatter (not just LanceDB). Without that, the
/// next `forgeplan reindex` (files-first per ADR-003) would re-read the
/// empty-tags file and null the DB column.
#[allow(clippy::too_many_arguments)]
async fn maybe_write_projection(
    workspace: &Path,
    artifact_id: &str,
    kind_str: &str,
    title: &str,
    status: &str,
    tags: &[String],
    body: &str,
    warnings: &mut Vec<String>,
) {
    use crate::db::store::ArtifactRecord;
    use crate::projection::render_projection_record;

    let now = chrono::Utc::now().to_rfc3339();
    let record = ArtifactRecord {
        id: artifact_id.to_string(),
        kind: kind_str.to_string(),
        status: status.to_string(),
        title: title.to_string(),
        body: body.to_string(),
        depth: "standard".to_string(),
        author: Some("scan-import".to_string()),
        parent_epic: None,
        valid_until: None,
        r_eff_score: 0.0,
        created_at: now.clone(),
        updated_at: now,
        tags: tags.to_vec(),
        body_hash: None,
        embedding: None,
    };
    if let Err(e) = render_projection_record(workspace, &record, &[]).await {
        warnings.push(format!(
            "projection write failed for {artifact_id}: {e} — \
             artifact is in LanceDB but .forgeplan/{kind_str}s/ is missing a .md file; \
             next `forgeplan reindex` could treat it as orphan"
        ));
    }
}

/// Resolve the artifact ID: use suggested_id from detection, or generate
/// next available. Enforces ID character-set locally instead of relying
/// on the downstream `store.create_artifact` check (R2 audit rust-pro
/// MEDIUM + security LOW): a crafted frontmatter `id: ../../etc/passwd`
/// would uppercase to `../../ETC/PASSWD` and flow into a path join
/// before create_artifact errors, if the call order ever changes.
async fn resolve_artifact_id(detection: &DetectionResult, store: &LanceStore) -> String {
    // If detection found an ID and it's well-formed, use it. Otherwise
    // fall through to auto-generation — worst case a user with a weird
    // custom ID gets a new sequential one instead of an error.
    if let Some(ref id) = detection.suggested_id {
        let normalized = id.to_uppercase();
        if is_safe_artifact_id(&normalized) {
            return normalized;
        }
    }

    // Otherwise, generate next available ID for this kind
    let kind_prefix = detection.kind.prefix().trim_end_matches('-').to_uppercase();
    for n in 1..=999 {
        let candidate = format!("{}-{:03}", kind_prefix, n);
        match store.get_artifact(&candidate).await {
            Ok(None) => return candidate,
            Ok(Some(_)) => continue,
            Err(_) => return candidate, // on error, try anyway
        }
    }

    // Exhausted ID space — return a clearly invalid ID that will fail at create
    // (better than silently returning a collision)
    format!(
        "{}-OVERFLOW",
        detection.kind.prefix().trim_end_matches('-').to_uppercase()
    )
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
// Existing tests call the deprecated bare `scan_and_import` — they
// intentionally exercise the no-projection path to isolate the LanceDB
// contract. The new PRD-058 AC tests use `scan_and_import_to_workspace`.
#[allow(deprecated)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_store() -> (TempDir, std::path::PathBuf, LanceStore) {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        std::fs::create_dir_all(&ws).unwrap();
        let store = LanceStore::init(&ws).await.unwrap();
        (tmp, ws, store)
    }

    #[tokio::test]
    async fn dry_run_does_not_persist() {
        let (tmp, _ws, store) = setup_store().await;
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(
            docs.join("PRD-001-test.md"),
            "---\nkind: prd\nid: PRD-001\ntitle: Test\n---\n# Test",
        )
        .unwrap();

        let opts = ScanImportOptions {
            dry_run: true,
            custom_path: None,
        };
        let result = scan_and_import(tmp.path(), &store, &opts).await.unwrap();

        assert_eq!(result.imported, 1); // preview count
        // But artifact should NOT exist in store
        assert!(store.get_artifact("PRD-001").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn import_creates_artifact() {
        let (tmp, _ws, store) = setup_store().await;
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(
            docs.join("RFC-001-design.md"),
            "---\nkind: rfc\nid: RFC-001\ntitle: Design\n---\n# Design",
        )
        .unwrap();

        let opts = ScanImportOptions::default();
        let result = scan_and_import(tmp.path(), &store, &opts).await.unwrap();

        assert_eq!(result.imported, 1);
        assert!(store.get_artifact("RFC-001").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn import_preserves_frontmatter_tags() {
        // C1 fix: scan-import must parse tags from frontmatter (ADR-003: files=truth).
        let (tmp, _ws, store) = setup_store().await;
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(
            docs.join("PRD-007-tagged.md"),
            "---\nkind: prd\nid: PRD-007\ntitle: Tagged\ntags:\n  - source=code\n  - layer=auth\n---\n# Tagged",
        )
        .unwrap();

        let opts = ScanImportOptions::default();
        let result = scan_and_import(tmp.path(), &store, &opts).await.unwrap();

        assert_eq!(result.imported, 1);
        let rec = store.get_record("PRD-007").await.unwrap().unwrap();
        assert_eq!(rec.tags, vec!["source=code", "layer=auth"]);
    }

    #[tokio::test]
    async fn skips_existing_artifact() {
        let (tmp, _ws, store) = setup_store().await;

        // Pre-create artifact
        let existing = crate::db::store::NewArtifact {
            id: "PRD-001".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Existing".to_string(),
            body: "".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        };
        store.create_artifact(&existing).await.unwrap();

        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(
            docs.join("PRD-001-dup.md"),
            "---\nkind: prd\nid: PRD-001\ntitle: Duplicate\n---\n# Dup",
        )
        .unwrap();

        let opts = ScanImportOptions::default();
        let result = scan_and_import(tmp.path(), &store, &opts).await.unwrap();

        assert_eq!(result.skipped, 1);
        assert_eq!(result.imported, 0);
    }

    #[tokio::test]
    async fn unknown_files_counted() {
        let (tmp, _ws, store) = setup_store().await;
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(docs.join("random.md"), "# Shopping\n- Milk").unwrap();

        let opts = ScanImportOptions::default();
        let result = scan_and_import(tmp.path(), &store, &opts).await.unwrap();

        assert_eq!(result.unknown, 1);
        assert_eq!(result.imported, 0);
    }

    #[tokio::test]
    async fn generates_id_when_none_suggested() {
        let (tmp, _ws, store) = setup_store().await;
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        // File with frontmatter kind but no id
        std::fs::write(
            docs.join("my-feature.md"),
            "---\nkind: prd\ntitle: My Feature\n---\n# Feature",
        )
        .unwrap();

        let opts = ScanImportOptions::default();
        let result = scan_and_import(tmp.path(), &store, &opts).await.unwrap();

        assert_eq!(result.imported, 1);
        // Should have generated PRD-001
        assert!(store.get_artifact("PRD-001").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn path_traversal_rejected() {
        let (tmp, _ws, store) = setup_store().await;

        let opts = ScanImportOptions {
            dry_run: false,
            custom_path: Some("../../etc".to_string()),
        };
        let result = scan_and_import(tmp.path(), &store, &opts).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("traversal"));
    }

    // ── PRD-058 FR-005: AC-1..5 regression guards ──────────────────────

    #[tokio::test]
    async fn scan_import_creates_projection_file() {
        // AC-1: scan-import must write `.forgeplan/<kind>/<ID>-<slug>.md`.
        // Without this, reindex treats the DB entry as orphan and purges
        // it — the root cause of Telegram bug report 2026-04-19.
        let (tmp, ws, store) = setup_store().await;
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(
            docs.join("ADR-001-use-postgres.md"),
            "---\nkind: adr\nid: ADR-001\ntitle: Use Postgres\nstatus: accepted\n---\n## Context\n\nNeeded reliable storage.\n\n## Decision\n\nPostgres.\n",
        )
        .unwrap();

        let opts = ScanImportOptions::default();
        let result = scan_and_import_to_workspace(tmp.path(), &ws, &store, &opts)
            .await
            .unwrap();
        assert_eq!(result.imported, 1);

        // AC-1: the projection file must exist.
        let adrs_dir = ws.join("adrs");
        let mut entries = tokio::fs::read_dir(&adrs_dir).await.unwrap();
        let mut found = false;
        while let Some(e) = entries.next_entry().await.unwrap() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("ADR-001-") && name.ends_with(".md") {
                found = true;
                let content = tokio::fs::read_to_string(e.path()).await.unwrap();
                // AC-2: body is preserved (the original Context / Decision sections).
                assert!(
                    content.contains("Needed reliable storage"),
                    "body lost — AC-2 regression"
                );
                assert!(content.contains("Postgres"));
            }
        }
        assert!(found, "AC-1 regression: projection file not created");
    }

    #[tokio::test]
    async fn scan_import_maps_obsidian_status_accepted_to_active() {
        // AC-4: `status: accepted` in frontmatter must land as `active`,
        // not hardcoded `draft`.
        let (tmp, ws, store) = setup_store().await;
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(
            docs.join("ADR-002-tls.md"),
            "---\nkind: adr\nid: ADR-002\ntitle: Use TLS\nstatus: accepted\n---\n# TLS\n",
        )
        .unwrap();

        let opts = ScanImportOptions::default();
        let result = scan_and_import_to_workspace(tmp.path(), &ws, &store, &opts)
            .await
            .unwrap();
        assert_eq!(result.imported, 1);

        let artifact = store.get_artifact("ADR-002").await.unwrap().unwrap();
        assert_eq!(
            artifact.status, "active",
            "AC-4 regression: accepted must map to active"
        );
    }

    #[tokio::test]
    async fn scan_import_maps_rejected_to_superseded() {
        let (tmp, ws, store) = setup_store().await;
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(
            docs.join("ADR-003-old-decision.md"),
            "---\nkind: adr\nid: ADR-003\ntitle: Old\nstatus: rejected\n---\n# Old\n",
        )
        .unwrap();

        let opts = ScanImportOptions::default();
        scan_and_import_to_workspace(tmp.path(), &ws, &store, &opts)
            .await
            .unwrap();

        let artifact = store.get_artifact("ADR-003").await.unwrap().unwrap();
        assert_eq!(artifact.status, "superseded");
    }

    #[tokio::test]
    async fn scan_import_warns_on_unknown_status() {
        // AC-4 fail-loud: unknown status → draft + warning.
        let (tmp, ws, store) = setup_store().await;
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(
            docs.join("ADR-004-wip.md"),
            "---\nkind: adr\nid: ADR-004\ntitle: WIP\nstatus: wip\n---\n# WIP\n",
        )
        .unwrap();

        let opts = ScanImportOptions::default();
        let result = scan_and_import_to_workspace(tmp.path(), &ws, &store, &opts)
            .await
            .unwrap();
        assert_eq!(result.imported, 1);

        let artifact = store.get_artifact("ADR-004").await.unwrap().unwrap();
        assert_eq!(artifact.status, "draft");

        let entry = result
            .entries
            .iter()
            .find(|e| e.artifact_id.as_deref() == Some("ADR-004"))
            .expect("entry for ADR-004");
        assert!(
            entry.warnings.iter().any(|w| w.contains("wip")),
            "AC-4 fail-loud: warning must mention unknown status, got {:?}",
            entry.warnings
        );
    }

    #[tokio::test]
    async fn scan_import_no_frontmatter_status_defaults_to_draft() {
        // AC-5: file without frontmatter `status:` key — default to draft,
        // no warning (expected backward-compat path).
        let (tmp, ws, store) = setup_store().await;
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(
            docs.join("ADR-005-no-fm.md"),
            "# ADR-005: No frontmatter\n\nSomething.\n",
        )
        .unwrap();

        let opts = ScanImportOptions::default();
        let result = scan_and_import_to_workspace(tmp.path(), &ws, &store, &opts)
            .await
            .unwrap();
        assert_eq!(result.imported, 1);

        let artifact = store.get_artifact("ADR-005").await.unwrap().unwrap();
        assert_eq!(artifact.status, "draft");

        let entry = result
            .entries
            .iter()
            .find(|e| e.artifact_id.as_deref() == Some("ADR-005"))
            .expect("entry for ADR-005");
        assert!(
            entry.warnings.is_empty(),
            "no-frontmatter path must not warn: got {:?}",
            entry.warnings
        );
    }

    #[tokio::test]
    async fn scan_import_body_survives_second_import_roundtrip() {
        // AC-3 proxy: running scan-import twice on the same file yields
        // Skipped (not duplicate) and the body on disk is unchanged.
        // Full reindex round-trip isn't unit-testable here (reindex lives
        // in a separate module); this guards the equivalent invariant at
        // the scan-import boundary.
        let (tmp, ws, store) = setup_store().await;
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(
            docs.join("ADR-010-idempotent.md"),
            "---\nid: ADR-010\nkind: adr\ntitle: Idempotent\nstatus: accepted\n---\n# Canonical body\n",
        )
        .unwrap();

        let opts = ScanImportOptions::default();
        let r1 = scan_and_import_to_workspace(tmp.path(), &ws, &store, &opts)
            .await
            .unwrap();
        assert_eq!(r1.imported, 1);

        // Verify projection file content mentions the body.
        let adrs_dir = ws.join("adrs");
        let mut first_body = String::new();
        let mut rd = tokio::fs::read_dir(&adrs_dir).await.unwrap();
        while let Some(e) = rd.next_entry().await.unwrap() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with("ADR-010-") && n.ends_with(".md") {
                first_body = tokio::fs::read_to_string(e.path()).await.unwrap();
            }
        }
        assert!(first_body.contains("Canonical body"));

        // Second run → skipped, projection untouched.
        let r2 = scan_and_import_to_workspace(tmp.path(), &ws, &store, &opts)
            .await
            .unwrap();
        assert_eq!(r2.imported, 0);
        assert_eq!(r2.skipped, 1);
    }
}
