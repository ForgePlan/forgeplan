//! Integration tests: same operations on Lance vs InMemory drivers.
//! Verifies that StorageDriver trait abstraction preserves behavior.

use forgeplan_core::db::store::{ArtifactFilter, NewArtifact};
use forgeplan_core::driver::in_memory::InMemoryStore;
use forgeplan_core::driver::lance::LanceDriver;
use forgeplan_core::driver::{ArtifactStorage, RelationStorage, StorageDriver};
use forgeplan_core::workspace::{self, lock::WorkspaceLock};
use tempfile::TempDir;

fn new_workspace(tmp: &TempDir) -> std::path::PathBuf {
    workspace::init_workspace(tmp.path(), "driver-test").unwrap()
}

async fn lock_workspace(path: &std::path::Path) -> WorkspaceLock {
    workspace::acquire_workspace_lock(path).await.unwrap()
}

/// Helper: create a NewArtifact with given fields.
fn make_artifact(id: &str, kind: &str, title: &str, body: &str) -> NewArtifact {
    NewArtifact {
        canonical_key: id.to_string(),
        id: id.to_string(),
        kind: kind.to_string(),
        status: "draft".to_string(),
        title: title.to_string(),
        body: body.to_string(),
        depth: "standard".to_string(),
        author: Some("test".to_string()),
        parent_epic: None,
        valid_until: None,
        tags: Vec::new(),
    }
}

/// Helper: create artifact using next_id for consistent ID generation.
async fn create_with_next_id(
    driver: &dyn StorageDriver,
    guard: &WorkspaceLock,
    kind: &str,
    title: &str,
    body: &str,
) -> String {
    let id = driver.next_id(guard, kind).await.unwrap();
    let art = make_artifact(&id, kind, title, body);
    driver.create_artifact(guard, &art).await.unwrap();
    id
}

// ── Test 1: CRUD lifecycle ─────────────────────────────────────────────

async fn crud_lifecycle(driver: &dyn StorageDriver, guard: &WorkspaceLock, label: &str) {
    let id = create_with_next_id(driver, guard, "PRD", "Auth System", "Login flow").await;
    assert_eq!(id, "PRD-001", "[{label}] first ID should be PRD-001");

    let summary = driver.get_artifact(guard, &id).await.unwrap();
    assert!(
        summary.is_some(),
        "[{label}] artifact must exist after create"
    );
    let summary = summary.unwrap();
    assert_eq!(summary.title, "Auth System", "[{label}] title mismatch");
    assert_eq!(
        summary.status, "draft",
        "[{label}] create must preserve the explicitly supplied status"
    );

    driver
        .update_artifact(guard, &id, Some("active"), None)
        .await
        .unwrap();
    let updated = driver.get_artifact(guard, &id).await.unwrap().unwrap();
    assert_eq!(
        updated.status, "active",
        "[{label}] status should be updated"
    );

    let before_touch = driver.get_record(guard, &id).await.unwrap().unwrap();
    driver
        .update_artifact(guard, &id, None, None)
        .await
        .unwrap();
    let touched = driver.get_record(guard, &id).await.unwrap().unwrap();
    assert_ne!(
        touched.updated_at, before_touch.updated_at,
        "[{label}] explicit touch must update its timestamp"
    );
    assert_eq!(touched.canonical_key, before_touch.canonical_key);
    assert_eq!(touched.title, before_touch.title);
    assert_eq!(touched.status, before_touch.status);
    assert_eq!(touched.body, before_touch.body);

    driver.delete_artifact(guard, &id).await.unwrap();
    let gone = driver.get_artifact(guard, &id).await.unwrap();
    assert!(
        gone.is_none(),
        "[{label}] artifact should be None after delete"
    );
}

#[tokio::test]
async fn test_crud_lifecycle_inmemory() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let store = InMemoryStore::new();
    let guard = lock_workspace(&ws).await;
    crud_lifecycle(&store, &guard, "InMemory").await;
}

#[tokio::test]
async fn test_crud_lifecycle_lance() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let driver = LanceDriver::init(&ws).await.unwrap();
    let guard = lock_workspace(&ws).await;
    crud_lifecycle(&driver, &guard, "Lance").await;
}

// ── Test 2: List with kind filter ──────────────────────────────────────

async fn list_with_kind_filter(driver: &dyn StorageDriver, guard: &WorkspaceLock, label: &str) {
    create_with_next_id(driver, guard, "PRD", "PRD One", "body1").await;
    create_with_next_id(driver, guard, "RFC", "RFC One", "body2").await;
    create_with_next_id(driver, guard, "ADR", "ADR One", "body3").await;

    let filter = ArtifactFilter {
        kind: Some("PRD".to_string()),
        status: None,
    };
    let prds = driver.list_artifacts(guard, Some(&filter)).await.unwrap();
    assert_eq!(prds.len(), 1, "[{label}] should find exactly 1 PRD");
    assert_eq!(prds[0].kind, "PRD", "[{label}] filtered item should be PRD");

    let all = driver.list_artifacts(guard, None).await.unwrap();
    assert_eq!(all.len(), 3, "[{label}] should have 3 total artifacts");
}

#[tokio::test]
async fn test_list_with_kind_filter_inmemory() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let store = InMemoryStore::new();
    let guard = lock_workspace(&ws).await;
    list_with_kind_filter(&store, &guard, "InMemory").await;
}

#[tokio::test]
async fn test_list_with_kind_filter_lance() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let driver = LanceDriver::init(&ws).await.unwrap();
    let guard = lock_workspace(&ws).await;
    list_with_kind_filter(&driver, &guard, "Lance").await;
}

// ── Test 3: Relations ──────────────────────────────────────────────────

async fn relations(driver: &dyn StorageDriver, guard: &WorkspaceLock, label: &str) {
    let prd = create_with_next_id(driver, guard, "PRD", "PRD", "body").await;
    let rfc = create_with_next_id(driver, guard, "RFC", "RFC", "body").await;
    driver
        .add_relation(guard, &prd, &rfc, "informs")
        .await
        .unwrap();

    let outgoing = driver.get_relations(guard, &prd).await.unwrap();
    assert_eq!(
        outgoing.len(),
        1,
        "[{label}] should have 1 outgoing relation"
    );
    assert_eq!(outgoing[0].0, rfc, "[{label}] target should be RFC");
    assert_eq!(outgoing[0].1, "informs", "[{label}] relation type mismatch");

    let incoming = driver.get_incoming_relations(guard, &rfc).await.unwrap();
    assert_eq!(
        incoming.len(),
        1,
        "[{label}] should have 1 incoming relation"
    );
    assert_eq!(incoming[0].0, prd, "[{label}] source should be PRD");

    driver
        .delete_relation(guard, &prd, &rfc, "informs")
        .await
        .unwrap();
    assert!(driver.get_relations(guard, &prd).await.unwrap().is_empty());
    assert!(
        driver
            .get_incoming_relations(guard, &rfc)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn test_relations_inmemory() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let store = InMemoryStore::new();
    let guard = lock_workspace(&ws).await;
    relations(&store, &guard, "InMemory").await;
}

#[tokio::test]
async fn test_relations_lance() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let driver = LanceDriver::init(&ws).await.unwrap();
    let guard = lock_workspace(&ws).await;
    relations(&driver, &guard, "Lance").await;
}

// ── Test 4: Search body ────────────────────────────────────────────────

async fn search_body(driver: &dyn StorageDriver, guard: &WorkspaceLock, label: &str) {
    create_with_next_id(driver, guard, "PRD", "Auth", "OAuth2 authentication flow").await;
    create_with_next_id(driver, guard, "RFC", "DB Schema", "PostgreSQL migration").await;

    let results = driver.search_body(guard, "oauth", None).await.unwrap();
    assert_eq!(
        results.len(),
        1,
        "[{label}] should find 1 result for 'oauth'"
    );
    assert_eq!(
        results[0].title, "Auth",
        "[{label}] found artifact should be Auth"
    );

    let empty = driver
        .search_body(guard, "nonexistent_xyz", None)
        .await
        .unwrap();
    assert!(
        empty.is_empty(),
        "[{label}] should find nothing for gibberish query"
    );
}

#[tokio::test]
async fn test_search_body_inmemory() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let store = InMemoryStore::new();
    let guard = lock_workspace(&ws).await;
    search_body(&store, &guard, "InMemory").await;
}

#[tokio::test]
async fn test_search_body_lance() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let driver = LanceDriver::init(&ws).await.unwrap();
    let guard = lock_workspace(&ws).await;
    search_body(&driver, &guard, "Lance").await;
}

// ── Test 5: next_id sequence ───────────────────────────────────────────

async fn next_id_sequence(driver: &dyn StorageDriver, guard: &WorkspaceLock, label: &str) {
    let id1 = driver.next_id(guard, "prd").await.unwrap();
    assert_eq!(id1, "PRD-001", "[{label}] first should be PRD-001");
    driver
        .create_artifact(guard, &make_artifact(&id1, "PRD", "T1", "b"))
        .await
        .unwrap();

    let id2 = driver.next_id(guard, "prd").await.unwrap();
    assert_eq!(id2, "PRD-002", "[{label}] second should be PRD-002");
    driver
        .create_artifact(guard, &make_artifact(&id2, "PRD", "T2", "b"))
        .await
        .unwrap();

    let id3 = driver.next_id(guard, "prd").await.unwrap();
    assert_eq!(id3, "PRD-003", "[{label}] third should be PRD-003");
}

#[tokio::test]
async fn test_next_id_sequence_inmemory() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let store = InMemoryStore::new();
    let guard = lock_workspace(&ws).await;
    next_id_sequence(&store, &guard, "InMemory").await;
}

#[tokio::test]
async fn test_next_id_sequence_lance() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let driver = LanceDriver::init(&ws).await.unwrap();
    let guard = lock_workspace(&ws).await;
    next_id_sequence(&driver, &guard, "Lance").await;
}

// ── Test 6: update_r_eff_score ─────────────────────────────────────────

async fn update_r_eff(driver: &dyn StorageDriver, guard: &WorkspaceLock, label: &str) {
    let id = create_with_next_id(driver, guard, "PRD", "Scored", "body").await;

    // The supplied score, not the storage backend's incidental default, is the contract.
    driver.update_r_eff_score(guard, &id, 0.75).await.unwrap();
    let record = driver.get_record(guard, &id).await.unwrap().unwrap();
    assert!(
        (record.r_eff_score - 0.75).abs() < f64::EPSILON,
        "[{label}] r_eff should be 0.75 after update"
    );
}

#[tokio::test]
async fn test_update_r_eff_inmemory() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let store = InMemoryStore::new();
    let guard = lock_workspace(&ws).await;
    update_r_eff(&store, &guard, "InMemory").await;
}

#[tokio::test]
async fn test_update_r_eff_lance() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let driver = LanceDriver::init(&ws).await.unwrap();
    let guard = lock_workspace(&ws).await;
    update_r_eff(&driver, &guard, "Lance").await;
}

// ── Test 7: get_all_relations ──────────────────────────────────────────

async fn get_all_relations(driver: &dyn StorageDriver, guard: &WorkspaceLock, label: &str) {
    let prd = create_with_next_id(driver, guard, "PRD", "PRD", "body").await;
    let rfc = create_with_next_id(driver, guard, "RFC", "RFC", "body").await;
    let adr = create_with_next_id(driver, guard, "ADR", "ADR", "body").await;

    driver
        .add_relation(guard, &prd, &rfc, "informs")
        .await
        .unwrap();
    driver
        .add_relation(guard, &rfc, &adr, "implements")
        .await
        .unwrap();

    let all = driver.get_all_relations(guard).await.unwrap();
    assert_eq!(all.len(), 2, "[{label}] should have 2 relations total");

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

    driver
        .add_relation(guard, &prd, &adr, "based_on")
        .await
        .unwrap();
    driver
        .delete_relations_for_artifact(guard, &rfc)
        .await
        .unwrap();
    assert_eq!(
        driver.get_all_relations(guard).await.unwrap(),
        vec![(prd, adr, "based_on".to_string())],
        "[{label}] cascade must remove both directions and preserve unrelated edges"
    );
    assert_eq!(driver.list_artifacts(guard, None).await.unwrap().len(), 3);
}

#[tokio::test]
async fn test_get_all_relations_inmemory() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let store = InMemoryStore::new();
    let guard = lock_workspace(&ws).await;
    get_all_relations(&store, &guard, "InMemory").await;
}

#[tokio::test]
async fn test_get_all_relations_lance() {
    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let driver = LanceDriver::init(&ws).await.unwrap();
    let guard = lock_workspace(&ws).await;
    get_all_relations(&driver, &guard, "Lance").await;
}

#[tokio::test]
async fn test_lance_mutations_persist_canonical_source() {
    use forgeplan_core::artifact::{documents, frontmatter};

    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let driver = LanceDriver::init(&ws).await.unwrap();
    let guard = lock_workspace(&ws).await;
    for (id, key, kind) in [
        ("PRD-001", "prd-driver-auth", "PRD"),
        ("RFC-001", "rfc-driver-api", "RFC"),
        ("ADR-001", "adr-driver-storage", "ADR"),
    ] {
        let mut artifact = make_artifact(id, kind, "Original title", "Original body");
        artifact.canonical_key = key.to_string();
        assert_eq!(driver.create_artifact(&guard, &artifact).await.unwrap(), id);
    }

    driver
        .update_artifact(&guard, "PRD-001", Some("active"), Some("Renamed auth"))
        .await
        .unwrap();
    driver
        .update_body(&guard, "prd-driver-auth", "Revised login flow\n")
        .await
        .unwrap();
    driver
        .add_relation(&guard, "PRD-001", "RFC-001", "informs")
        .await
        .unwrap();
    driver
        .add_relation(&guard, "ADR-001", "PRD-001", "based_on")
        .await
        .unwrap();
    driver
        .add_relation(&guard, "ADR-001", "RFC-001", "refines")
        .await
        .unwrap();

    let selected = documents::resolve_in_workspace(&ws, "PRD-001")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(selected.identity.canonical_key, "prd-driver-auth");
    let path = ws.join(selected.relative_path);
    let source = tokio::fs::read_to_string(&path).await.unwrap();
    let (fm, body) = frontmatter::parse_frontmatter(&source).unwrap();
    assert_eq!(fm["status"].as_str(), Some("active"));
    assert_eq!(fm["title"].as_str(), Some("Renamed auth"));
    assert_eq!(body, "Revised login flow\n");
    assert_eq!(
        frontmatter::links_from_frontmatter(&fm),
        vec![("rfc-driver-api".to_string(), "informs".to_string())]
    );

    driver
        .update_artifact(&guard, "PRD-001", None, None)
        .await
        .unwrap();
    assert_eq!(tokio::fs::read_to_string(&path).await.unwrap(), source);

    driver
        .delete_relation(&guard, "PRD-001", "RFC-001", "informs")
        .await
        .unwrap();
    driver
        .delete_relations_for_artifact(&guard, "PRD-001")
        .await
        .unwrap();
    let adr = documents::resolve_in_workspace(&ws, "ADR-001")
        .await
        .unwrap()
        .unwrap();
    let adr_path = ws.join(adr.relative_path);
    let (fm, _) =
        frontmatter::parse_frontmatter(&tokio::fs::read_to_string(&adr_path).await.unwrap())
            .unwrap();
    assert_eq!(
        frontmatter::links_from_frontmatter(&fm),
        vec![("rfc-driver-api".to_string(), "refines".to_string())]
    );

    driver.delete_artifact(&guard, "RFC-001").await.unwrap();
    assert!(
        documents::resolve_in_workspace(&ws, "RFC-001")
            .await
            .unwrap()
            .is_none()
    );
    let (fm, _) =
        frontmatter::parse_frontmatter(&tokio::fs::read_to_string(&adr_path).await.unwrap())
            .unwrap();
    assert!(frontmatter::links_from_frontmatter(&fm).is_empty());
    assert_eq!(documents::workspace_candidates(&ws).await.unwrap().len(), 2);

    let reopened = LanceDriver::open(&ws).await.unwrap();
    assert_eq!(
        reopened
            .get_artifact(&guard, "PRD-001")
            .await
            .unwrap()
            .unwrap()
            .title,
        "Renamed auth"
    );
    assert!(
        reopened
            .get_artifact(&guard, "RFC-001")
            .await
            .unwrap()
            .is_none()
    );
    assert!(reopened.get_all_relations(&guard).await.unwrap().is_empty());
}

#[tokio::test]
async fn test_lance_empty_key_fallback_and_foreign_guard_refusal() {
    use forgeplan_core::artifact::documents;

    let tmp = TempDir::new().unwrap();
    let ws = new_workspace(&tmp);
    let driver = LanceDriver::init(&ws).await.unwrap();
    let guard = lock_workspace(&ws).await;
    let mut artifact = make_artifact("NOTE-001", "NOTE", "Fallback key", "Keep this body");
    artifact.canonical_key.clear();
    driver.create_artifact(&guard, &artifact).await.unwrap();
    let selected = documents::resolve_in_workspace(&ws, "NOTE-001")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(selected.identity.canonical_key, "NOTE-001");
    let path = ws.join(selected.relative_path);
    let before = tokio::fs::read(&path).await.unwrap();

    let other = TempDir::new().unwrap();
    let other_ws = new_workspace(&other);
    let foreign_guard = lock_workspace(&other_ws).await;
    assert!(
        driver
            .update_body(&foreign_guard, "NOTE-001", "Must not be written")
            .await
            .is_err()
    );
    assert_eq!(tokio::fs::read(&path).await.unwrap(), before);
    assert_eq!(
        driver
            .get_record(&guard, "NOTE-001")
            .await
            .unwrap()
            .unwrap()
            .canonical_key,
        "NOTE-001"
    );
}
