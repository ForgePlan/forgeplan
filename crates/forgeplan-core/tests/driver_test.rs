//! Integration tests: same operations on Lance vs InMemory drivers.
//! Verifies that StorageDriver trait abstraction preserves behavior.

use forgeplan_core::db::store::{ArtifactFilter, NewArtifact};
use forgeplan_core::driver::StorageDriver;
use forgeplan_core::driver::in_memory::InMemoryStore;
use forgeplan_core::driver::lance::LanceDriver;
use tempfile::TempDir;

/// Helper: create a NewArtifact with given fields.
fn make_artifact(id: &str, kind: &str, title: &str, body: &str) -> NewArtifact {
    NewArtifact {
        id: id.to_string(),
        kind: kind.to_string(),
        status: "draft".to_string(),
        title: title.to_string(),
        body: body.to_string(),
        depth: "standard".to_string(),
        author: Some("test".to_string()),
        parent_epic: None,
        valid_until: None,
    }
}

/// Helper: create artifact using next_id for consistent ID generation.
async fn create_with_next_id(
    driver: &dyn StorageDriver,
    kind: &str,
    title: &str,
    body: &str,
) -> String {
    let id = driver.next_id(kind).await.unwrap();
    let art = make_artifact(&id, kind, title, body);
    driver.create_artifact(&art).await.unwrap();
    id
}

// ── Test 1: CRUD lifecycle ─────────────────────────────────────────────

async fn crud_lifecycle(driver: &dyn StorageDriver, label: &str) {
    // create
    let id = create_with_next_id(driver, "PRD", "Auth System", "Login flow").await;
    assert_eq!(id, "PRD-001", "[{label}] first ID should be PRD-001");

    // get
    let summary = driver.get_artifact(&id).await.unwrap();
    assert!(
        summary.is_some(),
        "[{label}] artifact must exist after create"
    );
    let summary = summary.unwrap();
    assert_eq!(summary.title, "Auth System", "[{label}] title mismatch");
    assert_eq!(
        summary.status, "draft",
        "[{label}] default status should be draft"
    );

    // update status
    driver
        .update_artifact(&id, Some("active"), None)
        .await
        .unwrap();
    let updated = driver.get_artifact(&id).await.unwrap().unwrap();
    assert_eq!(
        updated.status, "active",
        "[{label}] status should be updated"
    );

    // delete
    driver.delete_artifact(&id).await.unwrap();
    let gone = driver.get_artifact(&id).await.unwrap();
    assert!(
        gone.is_none(),
        "[{label}] artifact should be None after delete"
    );
}

#[tokio::test]
async fn test_crud_lifecycle_inmemory() {
    let store = InMemoryStore::new();
    crud_lifecycle(&store, "InMemory").await;
}

#[tokio::test]
async fn test_crud_lifecycle_lance() {
    let tmp = TempDir::new().unwrap();
    let driver = LanceDriver::init(tmp.path()).await.unwrap();
    crud_lifecycle(&driver, "Lance").await;
}

// ── Test 2: List with kind filter ──────────────────────────────────────

async fn list_with_kind_filter(driver: &dyn StorageDriver, label: &str) {
    create_with_next_id(driver, "PRD", "PRD One", "body1").await;
    create_with_next_id(driver, "RFC", "RFC One", "body2").await;
    create_with_next_id(driver, "ADR", "ADR One", "body3").await;

    let filter = ArtifactFilter {
        kind: Some("PRD".to_string()),
        status: None,
    };
    let prds = driver.list_artifacts(Some(&filter)).await.unwrap();
    assert_eq!(prds.len(), 1, "[{label}] should find exactly 1 PRD");
    assert_eq!(prds[0].kind, "PRD", "[{label}] filtered item should be PRD");

    // No filter returns all
    let all = driver.list_artifacts(None).await.unwrap();
    assert_eq!(all.len(), 3, "[{label}] should have 3 total artifacts");
}

#[tokio::test]
async fn test_list_with_kind_filter_inmemory() {
    let store = InMemoryStore::new();
    list_with_kind_filter(&store, "InMemory").await;
}

#[tokio::test]
async fn test_list_with_kind_filter_lance() {
    let tmp = TempDir::new().unwrap();
    let driver = LanceDriver::init(tmp.path()).await.unwrap();
    list_with_kind_filter(&driver, "Lance").await;
}

// ── Test 3: Relations ──────────────────────────────────────────────────

async fn relations(driver: &dyn StorageDriver, label: &str) {
    let prd = create_with_next_id(driver, "PRD", "PRD", "body").await;
    let rfc = create_with_next_id(driver, "RFC", "RFC", "body").await;

    driver.add_relation(&prd, &rfc, "informs").await.unwrap();

    let outgoing = driver.get_relations(&prd).await.unwrap();
    assert_eq!(
        outgoing.len(),
        1,
        "[{label}] should have 1 outgoing relation"
    );
    assert_eq!(outgoing[0].0, rfc, "[{label}] target should be RFC");
    assert_eq!(outgoing[0].1, "informs", "[{label}] relation type mismatch");

    let incoming = driver.get_incoming_relations(&rfc).await.unwrap();
    assert_eq!(
        incoming.len(),
        1,
        "[{label}] should have 1 incoming relation"
    );
    assert_eq!(incoming[0].0, prd, "[{label}] source should be PRD");
}

#[tokio::test]
async fn test_relations_inmemory() {
    let store = InMemoryStore::new();
    relations(&store, "InMemory").await;
}

#[tokio::test]
async fn test_relations_lance() {
    let tmp = TempDir::new().unwrap();
    let driver = LanceDriver::init(tmp.path()).await.unwrap();
    relations(&driver, "Lance").await;
}

// ── Test 4: Search body ────────────────────────────────────────────────

async fn search_body(driver: &dyn StorageDriver, label: &str) {
    create_with_next_id(driver, "PRD", "Auth", "OAuth2 authentication flow").await;
    create_with_next_id(driver, "RFC", "DB Schema", "PostgreSQL migration").await;

    let results = driver.search_body("oauth", None).await.unwrap();
    assert_eq!(
        results.len(),
        1,
        "[{label}] should find 1 result for 'oauth'"
    );
    assert_eq!(
        results[0].title, "Auth",
        "[{label}] found artifact should be Auth"
    );

    // No match
    let empty = driver.search_body("nonexistent_xyz", None).await.unwrap();
    assert!(
        empty.is_empty(),
        "[{label}] should find nothing for gibberish query"
    );
}

#[tokio::test]
async fn test_search_body_inmemory() {
    let store = InMemoryStore::new();
    search_body(&store, "InMemory").await;
}

#[tokio::test]
async fn test_search_body_lance() {
    let tmp = TempDir::new().unwrap();
    let driver = LanceDriver::init(tmp.path()).await.unwrap();
    search_body(&driver, "Lance").await;
}

// ── Test 5: next_id sequence ───────────────────────────────────────────

async fn next_id_sequence(driver: &dyn StorageDriver, label: &str) {
    // For LanceDB, next_id scans existing artifacts. We must create artifacts
    // with the returned IDs so subsequent calls see them.
    let id1 = driver.next_id("prd").await.unwrap();
    assert_eq!(id1, "PRD-001", "[{label}] first should be PRD-001");
    driver
        .create_artifact(&make_artifact(&id1, "PRD", "T1", "b"))
        .await
        .unwrap();

    let id2 = driver.next_id("prd").await.unwrap();
    assert_eq!(id2, "PRD-002", "[{label}] second should be PRD-002");
    driver
        .create_artifact(&make_artifact(&id2, "PRD", "T2", "b"))
        .await
        .unwrap();

    let id3 = driver.next_id("prd").await.unwrap();
    assert_eq!(id3, "PRD-003", "[{label}] third should be PRD-003");
}

#[tokio::test]
async fn test_next_id_sequence_inmemory() {
    let store = InMemoryStore::new();
    next_id_sequence(&store, "InMemory").await;
}

#[tokio::test]
async fn test_next_id_sequence_lance() {
    let tmp = TempDir::new().unwrap();
    let driver = LanceDriver::init(tmp.path()).await.unwrap();
    next_id_sequence(&driver, "Lance").await;
}

// ── Test 6: update_r_eff_score ─────────────────────────────────────────

async fn update_r_eff(driver: &dyn StorageDriver, label: &str) {
    let id = create_with_next_id(driver, "PRD", "Scored", "body").await;

    // Default r_eff should be 0.0
    let record = driver.get_record(&id).await.unwrap().unwrap();
    assert!(
        (record.r_eff_score - 0.0).abs() < f64::EPSILON,
        "[{label}] default r_eff should be 0.0"
    );

    // Update to 0.75
    driver.update_r_eff_score(&id, 0.75).await.unwrap();
    let record = driver.get_record(&id).await.unwrap().unwrap();
    assert!(
        (record.r_eff_score - 0.75).abs() < f64::EPSILON,
        "[{label}] r_eff should be 0.75 after update"
    );
}

#[tokio::test]
async fn test_update_r_eff_inmemory() {
    let store = InMemoryStore::new();
    update_r_eff(&store, "InMemory").await;
}

#[tokio::test]
async fn test_update_r_eff_lance() {
    let tmp = TempDir::new().unwrap();
    let driver = LanceDriver::init(tmp.path()).await.unwrap();
    update_r_eff(&driver, "Lance").await;
}

// ── Test 7: get_all_relations ──────────────────────────────────────────

async fn get_all_relations(driver: &dyn StorageDriver, label: &str) {
    let prd = create_with_next_id(driver, "PRD", "PRD", "body").await;
    let rfc = create_with_next_id(driver, "RFC", "RFC", "body").await;
    let adr = create_with_next_id(driver, "ADR", "ADR", "body").await;

    driver.add_relation(&prd, &rfc, "informs").await.unwrap();
    driver.add_relation(&rfc, &adr, "implements").await.unwrap();

    let all = driver.get_all_relations().await.unwrap();
    assert_eq!(all.len(), 2, "[{label}] should have 2 relations total");

    // Verify both relations are present (order may vary)
    let has_informs = all
        .iter()
        .any(|(s, t, r)| s == &prd && t == &rfc && r == "informs");
    let has_implements = all
        .iter()
        .any(|(s, t, r)| s == &rfc && t == &adr && r == "implements");
    assert!(has_informs, "[{label}] should contain informs relation");
    assert!(
        has_implements,
        "[{label}] should contain implements relation"
    );
}

#[tokio::test]
async fn test_get_all_relations_inmemory() {
    let store = InMemoryStore::new();
    get_all_relations(&store, "InMemory").await;
}

#[tokio::test]
async fn test_get_all_relations_lance() {
    let tmp = TempDir::new().unwrap();
    let driver = LanceDriver::init(tmp.path()).await.unwrap();
    get_all_relations(&driver, "Lance").await;
}
