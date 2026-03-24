use std::path::Path;

use crate::artifact::types::ArtifactKind;
use crate::db::store::{LanceStore, NewArtifact};
use crate::scan::detect::{detect_kind, DetectionResult, DetectionTier};
use crate::scan::discovery::{discover_markdown_files, DiscoveredFile};

/// Options for scan-import operation.
#[derive(Debug, Clone)]
pub struct ScanImportOptions {
    /// If true, only show what would be imported without making changes.
    pub dry_run: bool,
    /// Custom path to scan (overrides default doc directories).
    pub custom_path: Option<String>,
}

impl Default for ScanImportOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            custom_path: None,
        }
    }
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
pub async fn scan_and_import(
    project_root: &Path,
    store: &LanceStore,
    options: &ScanImportOptions,
) -> anyhow::Result<ScanImportResult> {
    // Discover files — with path traversal protection
    let scan_root = if let Some(ref custom) = options.custom_path {
        let candidate = project_root.join(custom);
        let canonical = candidate.canonicalize().unwrap_or(candidate.clone());
        let canonical_root = project_root.canonicalize().unwrap_or(project_root.to_path_buf());
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

        let detection = detect_kind(filename, &file.content);

        let entry = match detection {
            Some(det) => {
                process_detected_file(file, &det, store, options.dry_run).await
            }
            None => {
                unknown += 1;
                ScanImportEntry {
                    relative_path: file.relative_path.display().to_string(),
                    detected_kind: None,
                    detection_tier: None,
                    artifact_id: None,
                    status: ImportStatus::Unknown,
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

/// Process a file with a successful detection result.
async fn process_detected_file(
    file: &DiscoveredFile,
    detection: &DetectionResult,
    store: &LanceStore,
    dry_run: bool,
) -> ScanImportEntry {
    let artifact_id = resolve_artifact_id(detection, store).await;

    let entry_base = ScanImportEntry {
        relative_path: file.relative_path.display().to_string(),
        detected_kind: Some(detection.kind.clone()),
        detection_tier: Some(detection.tier.clone()),
        artifact_id: Some(artifact_id.clone()),
        status: ImportStatus::Imported, // will be overwritten
    };

    if dry_run {
        return ScanImportEntry {
            status: ImportStatus::Imported, // preview: would be imported
            ..entry_base
        };
    }

    // Check if artifact already exists
    match store.get_artifact(&artifact_id).await {
        Ok(Some(_)) => {
            return ScanImportEntry {
                status: ImportStatus::Skipped,
                ..entry_base
            };
        }
        Ok(None) => {} // proceed with import
        Err(e) => {
            return ScanImportEntry {
                status: ImportStatus::Failed(format!("Check existing: {e}")),
                ..entry_base
            };
        }
    }

    // Build title: prefer detection → filename → "Untitled"
    let title = detection
        .suggested_title
        .clone()
        .or_else(|| {
            file.path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.replace('-', " ").replace('_', " "))
        })
        .unwrap_or_else(|| "Untitled".to_string());

    let new_artifact = NewArtifact {
        id: artifact_id.clone(),
        kind: detection.kind.template_key().to_string(),
        status: "draft".to_string(),
        title,
        body: file.content.clone(),
        depth: "standard".to_string(),
        author: Some("scan-import".to_string()),
        parent_epic: None,
        valid_until: None,
    };

    match store.create_artifact(&new_artifact).await {
        Ok(_) => ScanImportEntry {
            status: ImportStatus::Imported,
            ..entry_base
        },
        Err(e) => ScanImportEntry {
            status: ImportStatus::Failed(format!("{e}")),
            ..entry_base
        },
    }
}

/// Resolve the artifact ID: use suggested_id from detection, or generate next available.
async fn resolve_artifact_id(detection: &DetectionResult, store: &LanceStore) -> String {
    // If detection found an ID, use it (normalized to uppercase)
    if let Some(ref id) = detection.suggested_id {
        return id.to_uppercase();
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

#[cfg(test)]
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
        ).unwrap();

        let opts = ScanImportOptions { dry_run: true, custom_path: None };
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
        ).unwrap();

        let opts = ScanImportOptions::default();
        let result = scan_and_import(tmp.path(), &store, &opts).await.unwrap();

        assert_eq!(result.imported, 1);
        assert!(store.get_artifact("RFC-001").await.unwrap().is_some());
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
        };
        store.create_artifact(&existing).await.unwrap();

        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(
            docs.join("PRD-001-dup.md"),
            "---\nkind: prd\nid: PRD-001\ntitle: Duplicate\n---\n# Dup",
        ).unwrap();

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
        ).unwrap();

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
}
