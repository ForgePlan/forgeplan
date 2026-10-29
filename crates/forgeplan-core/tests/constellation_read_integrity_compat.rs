//! SPEC-008 negative integrity at the shared CLI/MCP captured-read boundary.
//! Real canonical files and publisher-created native tables, never a legacy
//! store, fake forwarding layer or source-text assertion.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use arrow_schema::{DataType, Field, Schema};
use forgeplan_core::constellation::DiagnosticCode;
use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade::{
    MaintainCapability, maintain_index, open_workspace_read, resume_admission,
    workspace_is_migrated,
};
use forgeplan_core::constellation::manifest::{
    BootstrapState, CurrentDescriptor, GenerationManifest, ProtocolMarker,
};
use forgeplan_core::constellation::storage::{
    ACTIVE_TABLES, CURRENT_FILE_NAME, PROTOCOL_RELATIVE_PATH, build_table_uri, read_current,
    read_manifest,
};
use forgeplan_core::workspace::ARTIFACT_DIRS;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const SOURCE: &str = "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Integrity\n\
    links:\n  - target: PRD-002\n    relation: refines\n---\n\n\
    ## Integrity\n\nCanonical sentinel.\n";

fn workspace() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("temporary workspace");
    let root = tmp.path().canonicalize().expect("physical workspace path");
    for directory in ARTIFACT_DIRS {
        std::fs::create_dir_all(root.join(directory)).expect("canonical directory");
    }
    (tmp, root)
}

async fn published() -> (TempDir, PathBuf) {
    let (tmp, root) = workspace();
    std::fs::write(root.join("prds/PRD-001-integrity.md"), SOURCE).expect("primary source");
    std::fs::write(
        root.join("prds/PRD-002-target.md"),
        "---\nid: PRD-002\nkind: prd\nstatus: active\ntitle: Target\n---\n\nTarget body.\n",
    )
    .expect("linked source");
    resume_admission();
    let capability = MaintainCapability::for_strict_maintenance(&root);
    maintain_index(
        &capability,
        CaptureLimits::default(),
        Instant::now() + Duration::from_secs(120),
    )
    .await
    .expect("real publisher creates a complete native generation");
    (tmp, root)
}

#[derive(Debug, PartialEq, Eq)]
enum Entry {
    Directory,
    File(Vec<u8>),
    Symlink(PathBuf),
}

fn tree(root: &Path) -> BTreeMap<PathBuf, Entry> {
    fn visit(root: &Path, directory: &Path, entries: &mut BTreeMap<PathBuf, Entry>) {
        for item in std::fs::read_dir(directory).expect("list fixture directory") {
            let item = item.expect("fixture entry");
            let path = item.path();
            let relative = path
                .strip_prefix(root)
                .expect("relative entry")
                .to_path_buf();
            let kind = item.file_type().expect("entry type");
            if kind.is_symlink() {
                entries.insert(relative, Entry::Symlink(std::fs::read_link(path).unwrap()));
            } else if kind.is_dir() {
                entries.insert(relative, Entry::Directory);
                visit(root, &path, entries);
            } else {
                entries.insert(relative, Entry::File(std::fs::read(path).unwrap()));
            }
        }
    }
    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

fn assert_no_legacy_tables(root: &Path) {
    for name in ["artifacts", "relations", "evidence"] {
        assert!(
            !root.join("lance").join(format!("{name}.lance")).exists(),
            "captured reads must not create the old {name} table"
        );
    }
}

async fn assert_read_error_without_repair(root: &Path, expected: DiagnosticCode) {
    let before = tree(root);
    assert!(
        workspace_is_migrated(root),
        "managed state must exclude the legacy store"
    );
    let error = open_workspace_read(root, None, CaptureLimits::default())
        .await
        .expect_err(
            "broken published state must not become empty success or an ephemeral fallback",
        );
    assert_eq!(error.code(), expected, "{error}");
    assert_eq!(
        tree(root),
        before,
        "read errors must not repair any file or table"
    );
    assert_no_legacy_tables(root);
}

async fn assert_unavailable_without_repair(root: &Path) {
    assert_read_error_without_repair(root, DiagnosticCode::IndexGenerationUnavailable).await;
}

fn rewrite_manifest(root: &Path, mutate: impl FnOnce(&mut GenerationManifest)) {
    let index = root.join("lance");
    let mut descriptor = read_current(&index).unwrap().expect("published CURRENT");
    let mut manifest = read_manifest(&index, &descriptor).expect("verified manifest");
    mutate(&mut manifest);
    let bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    descriptor.manifest_sha256 = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    std::fs::write(index.join(&descriptor.manifest_path), bytes).unwrap();
    std::fs::write(
        index.join(CURRENT_FILE_NAME),
        serde_json::to_vec(&descriptor).unwrap(),
    )
    .unwrap();
}

fn set_version(manifest: &mut GenerationManifest, role: &str, version: u64) {
    match role {
        "artifacts" => manifest.table_versions.artifacts = version,
        "relations" => manifest.table_versions.relations = version,
        "sections" => manifest.table_versions.sections = version,
        _ => panic!("unknown fixture role"),
    }
}

#[tokio::test]
async fn an_uninitialized_empty_workspace_is_a_real_empty_read_without_an_index() {
    let (_tmp, root) = workspace();
    let before = tree(&root);
    assert!(!workspace_is_migrated(&root));
    let read = open_workspace_read(&root, None, CaptureLimits::default())
        .await
        .unwrap();
    assert!(!read.is_persisted());
    assert!(read.records().is_empty());
    assert!(read.get_all_relations().is_empty());
    assert!(read.captured().coverage.is_complete());
    assert_eq!(tree(&root), before);
    assert!(!root.join("lance").exists());
}

#[tokio::test]
async fn an_ordinary_legacy_layout_does_not_claim_constellation_ownership() {
    let (_tmp, root) = workspace();
    std::fs::create_dir_all(root.join("lance/artifacts.lance")).unwrap();
    assert!(!workspace_is_migrated(&root));
    let before = tree(&root);
    let read = open_workspace_read(&root, None, CaptureLimits::default())
        .await
        .unwrap();
    assert!(!read.is_persisted());
    assert!(read.records().is_empty());
    assert_eq!(tree(&root), before);
}

#[tokio::test]
async fn a_valid_published_snapshot_returns_exact_records_links_and_pins_without_writes() {
    let (_tmp, root) = published().await;
    let index = root.join("lance");
    let descriptor = read_current(&index).unwrap().unwrap();
    let manifest = read_manifest(&index, &descriptor).unwrap();
    let before = tree(&root);
    let read = open_workspace_read(&root, None, CaptureLimits::default())
        .await
        .unwrap();
    assert!(workspace_is_migrated(&root));
    assert!(read.is_persisted());
    assert_eq!(read.snapshot_id(), descriptor.generation_id);
    assert_eq!(read.records().len(), 2);
    assert_eq!(read.get_record("PRD-001").unwrap().body, SOURCE);
    assert_eq!(
        read.get_relations("PRD-001"),
        vec![("PRD-002".into(), "refines".into())]
    );
    assert_eq!(
        read.view().pinned_versions(),
        Some((
            manifest.table_versions.artifacts,
            manifest.table_versions.relations,
            manifest.table_versions.sections,
        ))
    );
    assert_eq!(tree(&root), before);
    assert_no_legacy_tables(&root);
}

#[tokio::test]
async fn a_valid_generation_does_not_require_recreating_a_lost_marker() {
    let (_tmp, root) = published().await;
    std::fs::remove_file(root.join("lance").join(PROTOCOL_RELATIVE_PATH)).unwrap();
    let before = tree(&root);
    assert!(workspace_is_migrated(&root));
    let read = open_workspace_read(&root, None, CaptureLimits::default())
        .await
        .unwrap();
    assert!(read.is_persisted());
    assert_eq!(read.get_record("PRD-001").unwrap().body, SOURCE);
    assert_eq!(tree(&root), before);
    assert_no_legacy_tables(&root);
}

#[tokio::test]
async fn changed_source_still_yields_a_fresh_ephemeral_capture_over_valid_physical_pins() {
    let (_tmp, root) = published().await;
    let changed = SOURCE.replace("Canonical sentinel.", "Changed sentinel.");
    std::fs::write(root.join("prds/PRD-001-integrity.md"), &changed).unwrap();
    let before = tree(&root);
    let read = open_workspace_read(&root, None, CaptureLimits::default())
        .await
        .unwrap();
    assert!(!read.is_persisted());
    assert_eq!(read.get_record("PRD-001").unwrap().body, changed);
    assert!(read.view().vector_cache().is_some());
    assert_eq!(
        tree(&root),
        before,
        "fresh reads must not republish or rebuild"
    );
    assert_no_legacy_tables(&root);
}

#[tokio::test]
async fn malformed_current_without_a_marker_still_excludes_the_legacy_store() {
    let (_tmp, root) = workspace();
    std::fs::create_dir(root.join("lance")).unwrap();
    std::fs::write(
        root.join("lance").join(CURRENT_FILE_NAME),
        b"{ broken CURRENT",
    )
    .unwrap();
    assert_unavailable_without_repair(&root).await;
}

#[tokio::test]
async fn missing_current_is_unavailable_with_or_without_the_active_marker() {
    for remove_marker in [false, true] {
        let (_tmp, root) = published().await;
        std::fs::remove_file(root.join("lance").join(CURRENT_FILE_NAME)).unwrap();
        if remove_marker {
            std::fs::remove_file(root.join("lance").join(PROTOCOL_RELATIVE_PATH)).unwrap();
        }
        assert_unavailable_without_repair(&root).await;
    }
}

#[tokio::test]
async fn a_generation_manifest_alone_prevents_legacy_recreation_after_pointer_loss() {
    let (_tmp, root) = published().await;
    let index = root.join("lance");
    std::fs::remove_file(index.join(CURRENT_FILE_NAME)).unwrap();
    std::fs::remove_file(index.join(PROTOCOL_RELATIVE_PATH)).unwrap();
    for (_, table) in ACTIVE_TABLES {
        std::fs::rename(
            index.join(format!("{table}.lance")),
            index.join(format!("{table}.preserved")),
        )
        .unwrap();
    }
    assert_unavailable_without_repair(&root).await;
}

#[tokio::test]
async fn interrupted_legacy_backups_without_a_marker_or_current_claim_managed_ownership() {
    for name in [
        "artifacts.legacy-backup.lance",
        "relations.legacy-backup.lance",
        "evidence.legacy-backup.lance",
    ] {
        let (_tmp, root) = workspace();
        std::fs::create_dir_all(root.join("lance").join(name)).unwrap();
        std::fs::write(
            root.join("lance").join(name).join("preserved-data"),
            b"never recreate a legacy writer",
        )
        .unwrap();
        assert_unavailable_without_repair(&root).await;
    }
}

#[tokio::test]
async fn an_unpublished_initializing_layout_keeps_its_read_only_ephemeral_behavior() {
    let (_tmp, root) = workspace();
    let index = root.join("lance");
    let path = index.join(PROTOCOL_RELATIVE_PATH);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        serde_json::to_vec(&ProtocolMarker::initializing(1, "fixture", 1)).unwrap(),
    )
    .unwrap();
    std::fs::create_dir(index.join("artifacts.legacy-backup.lance")).unwrap();
    let before = tree(&root);
    assert!(workspace_is_migrated(&root));
    let read = open_workspace_read(&root, None, CaptureLimits::default())
        .await
        .unwrap();
    assert!(!read.is_persisted());
    assert!(read.records().is_empty());
    assert_eq!(tree(&root), before);
    assert_no_legacy_tables(&root);
}

#[tokio::test]
async fn every_missing_physical_pinned_table_is_an_error_without_read_repair() {
    for (_, table) in ACTIVE_TABLES {
        let (_tmp, root) = published().await;
        let index = root.join("lance");
        let path = index.join(format!("{table}.lance"));
        std::fs::rename(&path, index.join(format!("{table}.preserved"))).unwrap();
        assert_unavailable_without_repair(&root).await;
        assert!(!path.exists(), "a read must not recreate {table}");
    }
}

#[tokio::test]
async fn every_damaged_physical_pinned_table_is_an_error_not_canonical_success() {
    for (_, table) in ACTIVE_TABLES {
        let (_tmp, root) = published().await;
        let index = root.join("lance");
        let path = index.join(format!("{table}.lance"));
        std::fs::rename(&path, index.join(format!("{table}.preserved"))).unwrap();
        std::fs::write(&path, b"not a native dataset").unwrap();
        assert_unavailable_without_repair(&root).await;
    }
}

#[tokio::test]
async fn every_unavailable_exact_version_is_rejected_instead_of_using_the_head() {
    for (role, _) in ACTIVE_TABLES {
        let (_tmp, root) = published().await;
        rewrite_manifest(&root, |manifest| set_version(manifest, role, 1_000_000));
        assert_unavailable_without_repair(&root).await;
    }
}

#[tokio::test]
async fn every_physical_schema_is_checked_even_when_the_replacement_table_is_empty() {
    for (role, table) in ACTIVE_TABLES {
        let (_tmp, root) = published().await;
        let index = root.join("lance");
        std::fs::rename(
            index.join(format!("{table}.lance")),
            index.join(format!("{table}.preserved")),
        )
        .unwrap();
        let uri = build_table_uri(&index, table);
        let native = lancedb::table::NativeTable::create_empty(
            &uri,
            role,
            Vec::new(),
            Arc::new(Schema::new(vec![Field::new(
                "canonical_key",
                DataType::Utf8,
                false,
            )])),
            None,
            None,
            None,
            None,
            false,
        )
        .await
        .expect("real native table with a deliberately invalid schema");
        drop(native);
        rewrite_manifest(&root, |manifest| set_version(manifest, role, 1));
        assert_unavailable_without_repair(&root).await;
    }
}

#[tokio::test]
async fn a_real_but_stale_artifacts_pin_cannot_hide_rows_in_the_confirmed_capture() {
    let (_tmp, root) = published().await;
    rewrite_manifest(&root, |manifest| {
        assert!(
            manifest.table_versions.artifacts > 1,
            "the source rows required a commit"
        );
        manifest.table_versions.artifacts = 1;
    });
    assert_unavailable_without_repair(&root).await;
}

#[tokio::test]
async fn moved_sources_do_not_hide_a_broken_published_cache_pin() {
    let (_tmp, root) = published().await;
    std::fs::write(
        root.join("prds/PRD-001-integrity.md"),
        SOURCE.replace("sentinel", "changed"),
    )
    .unwrap();
    let index = root.join("lance");
    let table = ACTIVE_TABLES[2].1;
    std::fs::rename(
        index.join(format!("{table}.lance")),
        index.join(format!("{table}.preserved")),
    )
    .unwrap();
    assert_unavailable_without_repair(&root).await;
}

#[tokio::test]
async fn corrupt_manifest_bytes_are_not_downgraded_to_an_empty_read() {
    let (_tmp, root) = published().await;
    let index = root.join("lance");
    let descriptor: CurrentDescriptor = read_current(&index).unwrap().unwrap();
    std::fs::write(index.join(descriptor.manifest_path), b"{}").unwrap();
    assert_unavailable_without_repair(&root).await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_dangling_current_is_physical_managed_state_not_an_absent_pointer() {
    let (_tmp, root) = workspace();
    std::fs::create_dir(root.join("lance")).unwrap();
    std::os::unix::fs::symlink(
        "missing-pointer-target",
        root.join("lance").join(CURRENT_FILE_NAME),
    )
    .unwrap();
    assert_unavailable_without_repair(&root).await;
}

#[tokio::test]
async fn staged_initial_state_can_answer_before_its_first_current_is_published() {
    let (_tmp, root) = workspace();
    let index = root.join("lance");
    let path = index.join(PROTOCOL_RELATIVE_PATH);
    let mut marker = ProtocolMarker::initializing(1, "fixture", 1);
    marker.state = BootstrapState::StagedInitial;
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::create_dir(index.join("generations")).unwrap();
    std::fs::write(index.join("generations/unpublished.json"), b"{}").unwrap();
    std::fs::write(path, serde_json::to_vec(&marker).unwrap()).unwrap();
    let before = tree(&root);
    assert!(workspace_is_migrated(&root));
    let read = open_workspace_read(&root, None, CaptureLimits::default())
        .await
        .unwrap();
    assert!(!read.is_persisted());
    assert!(read.records().is_empty());
    assert_eq!(tree(&root), before);
    assert_no_legacy_tables(&root);
}

#[tokio::test]
async fn published_bootstrap_state_cannot_silently_lose_its_current_pointer() {
    let (_tmp, root) = workspace();
    let index = root.join("lance");
    let path = index.join(PROTOCOL_RELATIVE_PATH);
    let mut marker = ProtocolMarker::initializing(1, "fixture", 1);
    marker.state = BootstrapState::Published;
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec(&marker).unwrap()).unwrap();
    assert_unavailable_without_repair(&root).await;
}

#[tokio::test]
async fn a_corrupt_marker_is_a_named_protocol_failure_not_empty_success() {
    let (_tmp, root) = published().await;
    std::fs::write(
        root.join("lance").join(PROTOCOL_RELATIVE_PATH),
        b"{ broken marker",
    )
    .unwrap();
    assert_read_error_without_repair(&root, DiagnosticCode::IndexProtocolMismatch).await;
}

#[tokio::test]
async fn checksummed_manifest_membership_must_still_match_the_confirmed_source() {
    let (_tmp, root) = published().await;
    rewrite_manifest(&root, |manifest| {
        manifest.source_entries[0].source_digest = "not-the-captured-digest".to_string();
    });
    assert_unavailable_without_repair(&root).await;
}

#[tokio::test]
async fn current_and_manifest_must_agree_about_configuration() {
    let (_tmp, root) = published().await;
    let index = root.join("lance");
    let mut descriptor = read_current(&index).unwrap().unwrap();
    descriptor.config_digest = "not-the-published-config".to_string();
    std::fs::write(
        index.join(CURRENT_FILE_NAME),
        serde_json::to_vec(&descriptor).unwrap(),
    )
    .unwrap();
    assert_unavailable_without_repair(&root).await;
}

#[tokio::test]
async fn foreign_current_format_is_refused_without_migration() {
    let (_tmp, root) = published().await;
    let index = root.join("lance");
    let mut descriptor = read_current(&index).unwrap().unwrap();
    descriptor.format_version += 1;
    std::fs::write(
        index.join(CURRENT_FILE_NAME),
        serde_json::to_vec(&descriptor).unwrap(),
    )
    .unwrap();
    assert_read_error_without_repair(&root, DiagnosticCode::IndexProtocolMismatch).await;
}

#[tokio::test]
async fn damaged_native_data_is_not_hidden_by_intact_table_and_manifest_metadata() {
    for (_, table) in &ACTIVE_TABLES[..2] {
        let (_tmp, root) = published().await;
        let dataset = PathBuf::from("lance").join(format!("{table}.lance"));
        let data_file = tree(&root)
            .into_iter()
            .find_map(|(path, entry)| {
                (matches!(entry, Entry::File(_))
                    && path.starts_with(dataset.join("data"))
                    && path
                        .extension()
                        .is_some_and(|extension| extension == "lance"))
                .then_some(path)
            })
            .expect("the real artifact/edge commit wrote a native data fragment");
        std::fs::write(root.join(data_file), b"damaged native data").unwrap();
        assert_unavailable_without_repair(&root).await;
    }
}

#[tokio::test]
async fn a_retired_constellation_sections_dataset_is_not_an_ordinary_legacy_index() {
    let (_tmp, root) = workspace();
    std::fs::create_dir_all(root.join("lance/constellation_sections_v1.lance")).unwrap();
    assert_unavailable_without_repair(&root).await;
}

#[tokio::test]
async fn an_incomplete_published_manifest_is_not_reported_as_a_complete_read() {
    let (_tmp, root) = published().await;
    rewrite_manifest(&root, |manifest| {
        manifest.coverage.invalid = 1;
        manifest.coverage.expected += 1;
    });
    assert_unavailable_without_repair(&root).await;
}

#[tokio::test]
async fn a_valid_snapshot_ignores_unreadable_legacy_table_contents() {
    let (_tmp, root) = published().await;
    for table in ["artifacts", "relations", "evidence"] {
        std::fs::write(
            root.join("lance").join(format!("{table}.lance")),
            b"not a legacy Lance dataset; must never be opened",
        )
        .unwrap();
    }
    let before = tree(&root);
    assert!(workspace_is_migrated(&root));
    let read = open_workspace_read(&root, None, CaptureLimits::default())
        .await
        .expect("captured reads never consult the poisoned legacy tables");
    assert!(read.is_persisted());
    assert_eq!(read.get_record("PRD-001").unwrap().body, SOURCE);
    assert_eq!(tree(&root), before);
}

#[tokio::test]
async fn a_healthy_published_empty_generation_remains_a_real_empty_snapshot() {
    let (_tmp, root) = workspace();
    resume_admission();
    maintain_index(
        &MaintainCapability::for_strict_maintenance(&root),
        CaptureLimits::default(),
        Instant::now() + Duration::from_secs(120),
    )
    .await
    .expect("empty canonical source still publishes the complete native triple");
    let before = tree(&root);
    let read = open_workspace_read(&root, None, CaptureLimits::default())
        .await
        .unwrap();
    assert!(read.is_persisted());
    assert!(read.records().is_empty());
    assert!(read.get_all_relations().is_empty());
    assert_eq!(read.captured().coverage.read, 0);
    assert!(read.captured().coverage.is_complete());
    assert_eq!(tree(&root), before);
    assert_no_legacy_tables(&root);
}
