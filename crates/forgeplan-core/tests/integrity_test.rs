//! Integration tests for PRD-043 Methodology Integrity (W3).
//!
//! Covers:
//! - FR-002: health duplicate-pair detection (`find_duplicate_pairs`)
//! - FR-003: stub detection (`check_stub`) + activate hard-block

use forgeplan_core::artifact::frontmatter::Frontmatter;
use forgeplan_core::db::store::{ArtifactRecord, LanceStore, NewArtifact};
use forgeplan_core::health::find_duplicate_pairs;
use forgeplan_core::lifecycle::activate;
use forgeplan_core::validation::rules::check_stub;
use tempfile::TempDir;

// ─── helpers ───────────────────────────────────────────────────────────────

fn empty_fm() -> Frontmatter {
    Frontmatter::new()
}

fn make_record(id: &str, kind: &str, title: &str, status: &str) -> ArtifactRecord {
    ArtifactRecord {
        id: id.into(),
        kind: kind.into(),
        status: status.into(),
        title: title.into(),
        body: String::new(),
        depth: "standard".into(),
        author: None,
        parent_epic: None,
        r_eff_score: 0.0,
        valid_until: None,
        created_at: "2026-01-01T00:00:00Z".into(),
        updated_at: "2026-01-01T00:00:00Z".into(),
    }
}

async fn make_store(tmp: &TempDir) -> LanceStore {
    let ws = tmp.path().join(".forgeplan");
    LanceStore::init(&ws).await.unwrap()
}

fn new_prd(id: &str, body: &str) -> NewArtifact {
    NewArtifact {
        id: id.into(),
        kind: "prd".into(),
        status: "draft".into(),
        title: format!("Test {id}"),
        body: body.into(),
        depth: "standard".into(),
        author: Some("tester".into()),
        parent_epic: None,
        valid_until: None,
    }
}

fn new_evidence(id: &str) -> NewArtifact {
    NewArtifact {
        id: id.into(),
        kind: "evidence".into(),
        status: "active".into(),
        title: format!("Evidence {id}"),
        body: "verdict: supports\ncongruence_level: 3\nevidence_type: test".into(),
        depth: "tactical".into(),
        author: None,
        parent_epic: None,
        valid_until: None,
    }
}

// ─── A. Stub detection (FR-003) ────────────────────────────────────────────

#[test]
fn test_check_stub_blocks_template_body() {
    // 4 phrase markers — well above threshold
    let body = "## Problem\n\nЧто мы строим и почему это важно\n\n\
        ## Users\n\nКак проблема влияет на пользователей\n\n\
        ## Scope\n\nЧто входит в минимально жизнеспособный продукт\n\n\
        ## Differentiation\n\nЧем наше решение отличается\n";
    let result = check_stub(body, &empty_fm());
    assert!(result.is_some(), "expected stub to be detected");
    let msg = result.unwrap();
    assert!(
        msg.contains("template"),
        "msg should mention template: {msg}"
    );
}

#[test]
fn test_check_stub_passes_filled_artifact() {
    let body = "## Problem\n\n\
        Users cannot reliably promote artifacts because the gate fails to detect \
        template-only stubs. This pollutes health reports.\n\n\
        ## Goals\n\n\
        Block activation when body is unfilled, preserving length and evidence checks.\n\n\
        ## Functional Requirements\n\n\
        FR-1: activate must call check_stub before promoting.\n";
    assert!(check_stub(body, &empty_fm()).is_none());
}

#[test]
fn test_check_stub_threshold_boundary() {
    // Exactly 3 markers → triggers
    let three = "Что мы строим и почему это важно\n\
        Как проблема влияет на пользователей\n\
        Чем наше решение отличается\n";
    assert!(
        check_stub(three, &empty_fm()).is_some(),
        "3 markers should trigger"
    );

    // Exactly 2 markers → does not trigger
    let two = "Что мы строим и почему это важно\n\
        Как проблема влияет на пользователей\n";
    assert!(
        check_stub(two, &empty_fm()).is_none(),
        "2 markers should not trigger"
    );
}

// ─── B. Health duplicate detection (FR-002) ───────────────────────────────

#[test]
fn test_integration_find_duplicate_pairs_finds_similar_titles() {
    let recs = vec![
        make_record("PRD-001", "prd", "Auth System Design Spec", "draft"),
        make_record("PRD-002", "prd", "Auth System Design Spec", "draft"),
    ];
    let pairs = find_duplicate_pairs(&recs, 0.8);
    assert_eq!(pairs.len(), 1);
    assert!(pairs[0].similarity >= 0.8);
    assert_eq!(pairs[0].kind, "prd");
}

#[test]
fn test_integration_find_duplicate_pairs_skips_different_kinds() {
    let recs = vec![
        make_record("PRD-001", "prd", "Same Title Here", "draft"),
        make_record("RFC-001", "rfc", "Same Title Here", "draft"),
    ];
    let pairs = find_duplicate_pairs(&recs, 0.8);
    assert!(pairs.is_empty(), "different kinds must not pair");
}

#[test]
fn test_integration_find_duplicate_pairs_below_threshold() {
    let recs = vec![
        make_record("PRD-001", "prd", "Authentication module rewrite", "draft"),
        make_record("PRD-002", "prd", "Database migration tooling", "draft"),
    ];
    let pairs = find_duplicate_pairs(&recs, 0.8);
    assert!(pairs.is_empty());
}

#[test]
fn test_integration_find_duplicate_pairs_skips_deprecated() {
    let a = make_record("PRD-001", "prd", "FPF Knowledge Base", "draft");
    let mut b = make_record("PRD-002", "prd", "FPF Knowledge Base", "draft");
    b.status = "deprecated".into();
    let pairs = find_duplicate_pairs(&[a.clone(), b.clone()], 0.8);
    assert!(pairs.is_empty(), "deprecated must be skipped");

    // Also test superseded
    b.status = "superseded".into();
    let pairs = find_duplicate_pairs(&[a, b], 0.8);
    assert!(pairs.is_empty(), "superseded must be skipped");
}

#[test]
fn test_integration_find_duplicate_pairs_sorts_by_similarity_desc() {
    let recs = vec![
        make_record("PRD-001", "prd", "Alpha Beta Gamma Delta", "draft"),
        // ~0.75 with PRD-001 (3/4 shared)
        make_record("PRD-002", "prd", "Alpha Beta Gamma Epsilon", "draft"),
        // identical to PRD-001 → similarity 1.0
        make_record("PRD-003", "prd", "Alpha Beta Gamma Delta", "draft"),
    ];
    let pairs = find_duplicate_pairs(&recs, 0.5);
    assert!(pairs.len() >= 2, "expected ≥2 pairs, got {}", pairs.len());
    for w in pairs.windows(2) {
        assert!(
            w[0].similarity >= w[1].similarity,
            "pairs must be sorted desc"
        );
    }
}

// ─── C. Lifecycle activate gate (FR-003 enforcement) ──────────────────────

#[tokio::test]
async fn test_integration_activate_blocks_stub_artifact() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(&tmp).await;

    let stub_body = "## Problem\n\nЧто мы строим и почему это важно\n\n\
        ## Goals\n\n[Actor] can [capability]\n\n\
        ## Users\n\nКак проблема влияет на пользователей\n\n\
        ## Scope\n\nЧто входит в минимально жизнеспособный продукт\n\n\
        ## Differentiation\n\nЧем наше решение отличается\n";
    let padded = format!(
        "{stub_body}\n\nExtra padding text to exceed the minimum length threshold for activation gates."
    );

    store
        .create_artifact(&new_prd("PRD-700", &padded))
        .await
        .unwrap();
    store
        .create_artifact(&new_evidence("EVID-700"))
        .await
        .unwrap();
    store
        .add_relation("EVID-700", "PRD-700", "informs")
        .await
        .unwrap();

    let result = activate(&store, "PRD-700", false).await;
    assert!(result.is_err(), "stub activate must be blocked");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("stub artifact") || msg.contains("PRD-043"),
        "error should mention stub gate, got: {msg}"
    );
}

#[tokio::test]
async fn test_integration_activate_does_not_block_filled_body_on_stub_gate() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(&tmp).await;

    let body = "## Problem\n\n\
        Users cannot reliably promote artifacts to active state because the gate \
        does not detect template-only stubs. This leads to false-active artifacts \
        that pollute health reports and erode trust in the methodology.\n\n\
        ## Goals\n\n\
        Block activation when the body is still an unfilled template, while \
        preserving the existing length and evidence checks.\n\n\
        ## Functional Requirements\n\n\
        FR-1: activate must call check_stub before promoting.\n";

    store
        .create_artifact(&new_prd("PRD-701", body))
        .await
        .unwrap();
    store
        .create_artifact(&new_evidence("EVID-701"))
        .await
        .unwrap();
    store
        .add_relation("EVID-701", "PRD-701", "informs")
        .await
        .unwrap();

    // Filled body must NOT trip the stub gate. May still fail other MUST gates.
    let result = activate(&store, "PRD-701", false).await;
    if let Err(e) = &result {
        let msg = e.to_string();
        assert!(
            !msg.contains("stub artifact"),
            "filled body should not trigger stub gate, got: {msg}"
        );
    }
}

#[tokio::test]
async fn test_integration_activate_already_active_no_recheck() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(&tmp).await;

    // Notes don't go through the validation gate and can be created active.
    let note = NewArtifact {
        id: "NOTE-700".into(),
        kind: "note".into(),
        status: "active".into(),
        title: "Already active".into(),
        body: "Some short body.".into(),
        depth: "tactical".into(),
        author: Some("tester".into()),
        parent_epic: None,
        valid_until: None,
    };
    store.create_artifact(&note).await.unwrap();

    // active → active should be a no-op transition error (or success), but
    // crucially must NOT panic / hit the stub gate.
    let result = activate(&store, "NOTE-700", false).await;
    // Either the transition validator rejects active→active, or it succeeds.
    // What matters: no stub-content error surfaces for an already-active artifact.
    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            !msg.contains("stub artifact"),
            "stub gate must not trigger on already-active artifact, got: {msg}"
        );
    }
}
