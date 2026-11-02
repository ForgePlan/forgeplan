// Included as a child of db::legacy_repair so the recovery plan can be inspected.
use super::*;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use arrow_array::{RecordBatch, StringArray};
use crate::artifact::identity_repair::{self, ApplyOutcome, JournalStatus, RepairJournal};
use crate::db::{convert, schema, store::{self, LanceStore}};

async fn fixture() -> (tempfile::TempDir, PathBuf, ApplyOutcome) {
    let root = tempfile::tempdir().unwrap();
    let ws = migrate::populated_identity_v4_workspace_with_vector_for_test(root.path()).await.unwrap().canonicalize().unwrap();
    std::fs::write(ws.join("config.yaml"), "embedding:\n  model: forgeplan-test\n  chunk_size: 2000\n").unwrap();
    for (name, title) in [("PRD-002-a.md", "Unindexed A"), ("PRD-002-b.md", "Unindexed B")] {
        std::fs::write(ws.join("prds").join(name), format!("---\nid: PRD-002\nkind: prd\nstatus: draft\ntitle: {title}\n---\n\n{title} body.\n")).unwrap();
    }
    let census = identity_repair::census_workspace(&ws).unwrap();
    let plan = identity_repair::build_repair_plan(&ws, &census, &[]).unwrap();
    let outcome = identity_repair::apply_repair_plan(&ws, &plan).unwrap();
    (root, ws, outcome)
}

fn backend(ws: &Path, calls: Arc<AtomicUsize>) -> store::SemanticTestBackendGuard {
    store::install_semantic_backend_for_test(ws, Arc::new(move |text| {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![text.len() as f32; schema::EMBEDDING_DIM as usize])
    })).unwrap()
}

fn journal(outcome: &ApplyOutcome) -> RepairJournal {
    serde_json::from_slice(&std::fs::read(outcome.journal_dir.join("manifest.json")).unwrap()).unwrap()
}

fn original_digests(ws: &Path) -> Vec<String> {
    ["artifacts", "relations", "evidence"].iter().map(|name| {
        migrate::semantic_rebuild_tree_digest(&ws.join("lance").join(format!("{name}.lance"))).unwrap()
    }).collect()
}

async fn rows(ws: &Path, name: &str) -> Vec<RecordBatch> {
    let db = connection(&ws.join("lance")).await.unwrap();
    migrate::table_rows(&db, name).await.unwrap()
}

#[tokio::test]
async fn repair_recomputes_exact_vectors_and_preserves_all_v4_tables() {
    use sha2::{Digest, Sha256};
    let (_root, ws, outcome) = fixture().await;
    let original = original_digests(&ws);
    let old_evidence = rows(&ws, "evidence").await;
    let old_relations = rows(&ws, "relations").await;
    let calls = Arc::new(AtomicUsize::new(0));
    let _backend = backend(&ws, calls.clone());
    LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.unwrap();
    assert_eq!(original_digests(&ws), original);
    assert_eq!(journal(&outcome).status, JournalStatus::CanonicalCommitted);
    let marker = migrate::read_identity_v5_marker(&ws.join("lance")).unwrap().unwrap();
    let profile = marker.embeddings.unwrap();
    let artifacts = rows(&ws, NAMES[0]).await;
    assert_eq!(artifacts.len(), 4);
    assert_eq!(profile.input_digests.len(), artifacts.len());
    assert_eq!(profile.vector_digests.len(), artifacts.len());
    assert_eq!(calls.load(Ordering::SeqCst), artifacts.len());
    for row in &artifacts {
        let key = migrate::required_string(row, "canonical_key").unwrap();
        let title = migrate::required_string(row, "title").unwrap();
        let body = store::record_large_string(row, "body", 0).unwrap();
        let input = format!("{} {}", title, body.chars().take(2000).collect::<String>());
        let hash = crate::constellation::hex_encode(&Sha256::digest(input.as_bytes()));
        assert_eq!(profile.input_digests[&key], hash);
        let vector = convert::extract_embedding(row, 0).unwrap();
        assert_eq!(vector, vec![input.len() as f32; schema::EMBEDDING_DIM as usize]);
        assert_eq!(profile.vector_digests[&key], migrate::identity_vector_digest(&vector));
    }
    let evidence = rows(&ws, NAMES[2]).await;
    assert_eq!(evidence.len(), 1);
    assert_eq!(migrate::required_string(&evidence[0], "content").unwrap(), "original evidence");
    assert_eq!(migrate::required_string(&evidence[0], "artifact_id").unwrap(), "prd-auth-system");
    for (index, field) in evidence[0].schema().fields().iter().enumerate() {
        if field.name() != "artifact_id" {
            assert_eq!(evidence[0].column(index).to_data(), old_evidence[0].column(index).to_data());
        }
    }
    let relations = rows(&ws, NAMES[1]).await;
    assert_eq!(relations.len(), 1);
    assert_eq!(migrate::required_string(&relations[0], "source_id").unwrap(), "prd-auth-system");
    assert_eq!(migrate::required_string(&relations[0], "target_id").unwrap(), "RFC-001");
    for (index, field) in relations[0].schema().fields().iter().enumerate() {
        if field.name() != "source_id" && field.name() != "target_id" {
            assert_eq!(relations[0].column(index).to_data(), old_relations[0].column(index).to_data());
        }
    }
}

#[tokio::test]
async fn unavailable_backend_leaves_no_published_marker_or_terminal_journal() {
    let (_root, ws, outcome) = fixture().await;
    let before = original_digests(&ws);
    assert!(LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.is_err());
    assert_eq!(original_digests(&ws), before);
    assert!(migrate::read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
    assert_eq!(journal(&outcome).status, JournalStatus::CanonicalCommittedIndexPending);
    assert!(!current_plan_directory(&ws, &outcome.transaction_id).unwrap().join("plan.json").exists());
}

#[tokio::test]
async fn replay_before_and_after_swap_never_reembeds_and_keeps_frozen_times() {
    for point in ["after_stage", "after_swap", "after_marker"] {
        let (_root, ws, outcome) = fixture().await;
        let original = original_digests(&ws);
        let calls = Arc::new(AtomicUsize::new(0));
        let _backend = backend(&ws, calls.clone());
        fail_repair_once(&ws, point);
        assert!(LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.is_err());
        let count = calls.load(Ordering::SeqCst);
        assert_eq!(count, 4);
        assert_eq!(journal(&outcome).status, JournalStatus::CanonicalCommittedIndexPending);
        let dir = current_plan_directory(&ws, &outcome.transaction_id).unwrap();
        let plan: RepairIndexPlan = serde_json::from_slice(&std::fs::read(dir.join("plan.json")).unwrap()).unwrap();
        let header: RepairHeader = serde_json::from_slice(&std::fs::read(dir.join("header.json")).unwrap()).unwrap();
        assert_eq!(plan.operation_time, header.operation_time);
        LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), count);
        assert_eq!(original_digests(&ws), original);
        assert_eq!(journal(&outcome).status, JournalStatus::CanonicalCommitted);
        for row in rows(&ws, NAMES[0]).await {
            if migrate::required_string(&row, "id").unwrap() == "PRD-002" {
                assert_eq!(migrate::required_string(&row, "created_at").unwrap(), plan.operation_time);
                assert_eq!(migrate::required_string(&row, "updated_at").unwrap(), plan.operation_time);
            }
        }
    }
}

async fn corrupt_stage(ws: &Path, outcome: &ApplyOutcome, kind: &str) {
    let dir = current_plan_directory(ws, &outcome.transaction_id).unwrap();
    let path = dir.join("plan.json");
    let mut plan: RepairIndexPlan = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    if kind == "profile" {
        plan.new_marker.embeddings.as_mut().unwrap().input_digests.values_mut().next().map(|digest| *digest = "0".repeat(64));
    } else {
        let (index, column) = match kind { "vector" => (0, "embedding"), "input" => (0, "semantic_input_digest"), "nonsemantic" => (0, "title"), "evidence" => (2, "content"), _ => unreachable!() };
        let stage = repair_stage_path_for_test(ws, &outcome.transaction_id).unwrap();
        let db = connection(&stage).await.unwrap();
        let mut batch = migrate::table_rows(&db, NAMES[index]).await.unwrap();
        let original = &batch[0];
        let mut columns = original.columns().to_vec();
        let col = original.schema().index_of(column).unwrap();
        columns[col] = if kind == "vector" {
            convert::make_embedding_col_from_option(Some(&vec![9.0; schema::EMBEDDING_DIM as usize]))
        } else {
            Arc::new(StringArray::from(vec!["corrupted"]))
        };
        batch[0] = RecordBatch::try_new(original.schema(), columns).unwrap();
        let table = db.open_table(NAMES[index]).execute().await.unwrap();
        table.delete("true").await.unwrap();
        table.add(batch).execute().await.unwrap();
        drop(table);
        drop(db);
        plan.tables[index].after_digest = migrate::semantic_rebuild_tree_digest(&stage.join(format!("{}.lance", NAMES[index]))).unwrap();
    }
    plan.digest = plan_digest(&plan).unwrap();
    std::fs::write(path, serde_json::to_vec(&plan).unwrap()).unwrap();
}

#[tokio::test]
async fn rehashed_corrupt_stages_and_profile_are_rejected_before_publication() {
    for kind in ["vector", "input", "nonsemantic", "evidence", "profile"] {
        let (_root, ws, outcome) = fixture().await;
        let original = original_digests(&ws);
        let calls = Arc::new(AtomicUsize::new(0));
        let _backend = backend(&ws, calls.clone());
        fail_repair_once(&ws, "after_stage");
        assert!(LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.is_err());
        let count = calls.load(Ordering::SeqCst);
        corrupt_stage(&ws, &outcome, kind).await;
        assert!(LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.is_err(), "accepted {kind}");
        assert_eq!(calls.load(Ordering::SeqCst), count, "reembedded {kind}");
        assert_eq!(original_digests(&ws), original);
        assert!(migrate::read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
        assert_eq!(journal(&outcome).status, JournalStatus::CanonicalCommittedIndexPending);
    }
}

#[tokio::test]
async fn rollback_to_duplicate_keys_records_diagnostic_and_remains_blocked() {
    let (_root, ws, outcome) = fixture().await;
    let before = original_digests(&ws);
    identity_repair::rollback_repair(&ws, &outcome.transaction_id).unwrap();
    assert!(LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.is_err());
    let state = journal(&outcome);
    assert_eq!(state.status, JournalStatus::RolledBackIndexBlocked);
    assert!(state.legacy_diagnostic.is_some());
    assert_eq!(original_digests(&ws), before);
    assert!(migrate::read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
    assert!(crate::workspace::lock::acquire_workspace_read_lock(&ws).await.is_err());
}

#[tokio::test]
async fn legacy_rollback_without_postimages_records_diagnostic_and_admits_linked_repair() {
    let (_root, ws, outcome) = fixture().await;
    let original = original_digests(&ws);
    let mut old = journal(&outcome);
    old.version = None;
    old.phase = None;
    old.moves.clear();
    old.manifest_sha256 = identity_repair::compute_journal_digest(&old);
    std::fs::write(outcome.journal_dir.join("manifest.json"), serde_json::to_vec_pretty(&old).unwrap()).unwrap();
    std::fs::remove_dir_all(outcome.journal_dir.join("postimages")).unwrap();
    std::fs::remove_dir_all(outcome.journal_dir.join("staged")).unwrap();

    let rollback = identity_repair::rollback_repair(&ws, &outcome.transaction_id).unwrap();
    assert_eq!(rollback.status, JournalStatus::RolledBackIndexPending);
    let error = LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.unwrap_err();
    assert!(format!("{error:#}").contains("diagnostic recorded"));
    let blocked = journal(&outcome);
    assert_eq!(blocked.status, JournalStatus::RolledBackIndexBlocked);
    assert!(blocked.legacy_diagnostic.is_some());
    assert_eq!(original_digests(&ws), original);
    assert!(migrate::read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
    assert!(crate::workspace::lock::acquire_workspace_read_lock(&ws).await.is_err());

    let census = identity_repair::census_workspace(&ws).unwrap();
    assert!(census.inventory_digest.starts_with("v2:"));
    let plan = identity_repair::build_repair_plan(&ws, &census, &[]).unwrap();
    let successor = identity_repair::apply_repair_plan_after_blocked(&ws, &plan, &outcome.transaction_id).await.unwrap();
    let (predecessor, next) = identity_repair::inspect_linked_repair_journals(
        &ws, &outcome.transaction_id, &successor.transaction_id,
    ).await.unwrap();
    assert_eq!(predecessor.status, JournalStatus::RolledBackIndexBlocked);
    assert_eq!(next.status, JournalStatus::CanonicalCommittedIndexPending);
    assert_eq!(original_digests(&ws), original);
}

#[tokio::test]
async fn staged_forward_rollback_records_diagnostic_without_removing_stage() {
    let (_root, ws, outcome) = fixture().await;
    let original = original_digests(&ws);
    let _backend = backend(&ws, Arc::new(AtomicUsize::new(0)));
    fail_repair_once(&ws, "after_stage");
    assert!(LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.is_err());
    let forward = current_plan_directory(&ws, &outcome.transaction_id).unwrap();
    let plan = std::fs::read(forward.join("plan.json")).unwrap();
    let stage = repair_stage_path_for_test(&ws, &outcome.transaction_id).unwrap();
    let stage_digest = migrate::semantic_rebuild_tree_digest(&stage).unwrap();
    assert!(crate::workspace::lock::acquire_workspace_read_lock(&ws).await.is_err());
    assert!(crate::workspace::lock::acquire_workspace_lock(&ws).await.is_err());

    identity_repair::rollback_repair(&ws, &outcome.transaction_id).unwrap();
    let rollback = current_plan_directory(&ws, &outcome.transaction_id).unwrap();
    assert_ne!(forward, rollback);
    assert!(LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.is_err());
    let state = journal(&outcome);
    assert_eq!(state.status, JournalStatus::RolledBackIndexBlocked);
    assert!(state.legacy_diagnostic.is_some());
    assert_eq!(std::fs::read(forward.join("plan.json")).unwrap(), plan);
    assert_eq!(migrate::semantic_rebuild_tree_digest(&stage).unwrap(), stage_digest);
    assert_eq!(original_digests(&ws), original);
    assert!(crate::workspace::lock::acquire_workspace_read_lock(&ws).await.is_err());
    assert!(crate::workspace::lock::acquire_workspace_lock(&ws).await.is_err());
}

#[tokio::test]
async fn wrong_workspace_journal_guard_and_changed_snapshot_cannot_publish() {
    let (_root, ws, outcome) = fixture().await;
    let (_other_root, other, other_outcome) = fixture().await;
    let before = original_digests(&ws);
    assert!(LanceStore::reconcile_legacy_repair(&ws, &other_outcome.transaction_id).await.is_err());
    assert!(LanceStore::reconcile_legacy_repair(&other, &outcome.transaction_id).await.is_err());
    let guard = crate::workspace::lock::acquire_legacy_repair_workspace_lock(&other, &other_outcome.transaction_id).await.unwrap();
    assert!(identity_repair::validate_legacy_repair(&ws, &guard, &outcome.transaction_id).is_err());
    drop(guard);
    std::fs::write(ws.join("prds/PRD-001-auth.md"), b"changed after journal").unwrap();
    assert!(LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.is_err());
    assert_eq!(original_digests(&ws), before);
    assert_eq!(journal(&outcome).status, JournalStatus::CanonicalCommittedIndexPending);
    assert!(migrate::read_identity_v5_marker(&ws.join("lance")).unwrap().is_none());
}

#[tokio::test]
async fn published_postimage_then_rollback_retains_both_claimants_as_blocked() {
    let (_root, ws, outcome) = fixture().await;
    let _backend = backend(&ws, Arc::new(AtomicUsize::new(0)));
    LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.unwrap();
    let marker = migrate::read_identity_v5_marker(&ws.join("lance")).unwrap().unwrap();
    let post = migrate::read_identity_v5_source(&ws).await.unwrap();
    assert_eq!(marker.source_digest, post.source_digest);
    let v5_before = NAMES.map(|name| migrate::semantic_rebuild_tree_digest(
        &ws.join("lance").join(format!("{name}.lance"))
    ).unwrap());
    identity_repair::rollback_repair(&ws, &outcome.transaction_id).unwrap();
    let pre = migrate::read_identity_v5_source(&ws).await.unwrap();
    assert_ne!(pre.source_digest, marker.source_digest);
    assert_eq!(pre.documents.iter().filter(|doc| doc.display_id == "PRD-002").count(), 2);
    assert!(LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.is_err());
    let result = journal(&outcome);
    assert_eq!(result.status, JournalStatus::RolledBackIndexBlocked);
    assert!(result.legacy_diagnostic.is_some());
    assert_eq!(migrate::read_identity_v5_marker(&ws.join("lance")).unwrap().unwrap().source_digest, marker.source_digest);
    assert_eq!(v5_before, NAMES.map(|name| migrate::semantic_rebuild_tree_digest(
        &ws.join("lance").join(format!("{name}.lance"))
    ).unwrap()));
    assert!(crate::workspace::lock::acquire_workspace_read_lock(&ws).await.is_err());
}

#[tokio::test]
async fn v5_marker_matching_neither_journal_image_is_rejected_before_stage() {
    let (_root, ws, first) = fixture().await;
    let _backend = backend(&ws, Arc::new(AtomicUsize::new(0)));
    LanceStore::reconcile_legacy_repair(&ws, &first.transaction_id).await.unwrap();
    for (name, title) in [("PRD-003-a.md", "Unindexed C"), ("PRD-003-b.md", "Unindexed D")] {
        std::fs::write(ws.join("prds").join(name), format!("---\nid: PRD-003\nkind: prd\nstatus: draft\ntitle: {title}\n---\n\n{title} body.\n")).unwrap();
    }
    let census = identity_repair::census_workspace(&ws).unwrap();
    let plan = identity_repair::build_repair_plan(&ws, &census, &[]).unwrap();
    let outcome = identity_repair::apply_repair_plan(&ws, &plan).unwrap();
    let marker_path = migrate::identity_v5_marker_path(&ws.join("lance"));
    let mut marker = migrate::read_identity_v5_marker(&ws.join("lance")).unwrap().unwrap();
    marker.source_digest = "0".repeat(64);
    std::fs::write(&marker_path, serde_json::to_vec_pretty(&marker).unwrap()).unwrap();
    let marker_bytes = std::fs::read(&marker_path).unwrap();
    let before = NAMES.map(|name| migrate::semantic_rebuild_tree_digest(
        &ws.join("lance").join(format!("{name}.lance"))
    ).unwrap());
    let error = LanceStore::reconcile_legacy_repair(&ws, &outcome.transaction_id).await.unwrap_err();
    assert!(format!("{error:#}").contains("matches neither journal source image"));
    assert!(!current_plan_directory(&ws, &outcome.transaction_id).unwrap().join("plan.json").exists());
    assert_eq!(std::fs::read(marker_path).unwrap(), marker_bytes);
    assert_eq!(before, NAMES.map(|name| migrate::semantic_rebuild_tree_digest(
        &ws.join("lance").join(format!("{name}.lance"))
    ).unwrap()));
    assert_eq!(journal(&outcome).status, JournalStatus::CanonicalCommittedIndexPending);
}
