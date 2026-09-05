//! What R_eff means when the graph is not a clean chain — PRD-086.
//!
//! Three questions the recursive scorer answers badly enough that people filed
//! issues about all three:
//!
//! - can a leaf EvidencePack be trusted at all (#325)
//! - does trust flow the wrong way along an `informs` edge (#325, FR-002)
//! - does a Note the methodology exempts from evidence poison everything
//!   built on it (#392, narrowed)
//!
//! These run against a real store with real relations, because every one of
//! them is about how the walk behaves, not about arithmetic on a Vec.

use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::scoring::reff::r_eff_recursive;
use std::collections::HashSet;
use tempfile::TempDir;

async fn make_store(tmp: &TempDir) -> LanceStore {
    let ws = tmp.path().join(".forgeplan");
    LanceStore::init(&ws).await.unwrap()
}

fn artifact(id: &str, kind: &str, status: &str, body: &str) -> NewArtifact {
    NewArtifact {
        id: id.into(),
        kind: kind.into(),
        status: status.into(),
        title: format!("Test {id}"),
        body: body.into(),
        depth: "standard".into(),
        author: None,
        parent_epic: None,
        valid_until: None,
        tags: Vec::new(),
    }
}

const CANONICAL: &str = "verdict: supports\ncongruence_level: 3\nevidence_type: measurement\n";

async fn score(store: &LanceStore, id: &str) -> f64 {
    let mut seen: HashSet<String> = HashSet::new();
    r_eff_recursive(id, store, &mut seen).await.unwrap().r_eff
}

/// #325. A pack has no packs, so asking it for its evidence found none and
/// returned 0.0 with "No evidence found (L0)". The intrinsic score already
/// existed — it is applied to this same pack whenever it scores for someone
/// else — and was simply never applied to the pack itself.
#[tokio::test]
async fn a_canonical_leaf_pack_is_worth_something_on_its_own() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(&tmp).await;
    store
        .create_artifact_for_test(&artifact("EVID-001", "evidence", "active", CANONICAL))
        .await
        .unwrap();

    let s = score(&store, "EVID-001").await;
    assert!(
        (s - 1.0).abs() < 1e-9,
        "supports + CL3 is the strongest a pack can state; got {s}"
    );
}

/// The intrinsic score is a score, not a rubber stamp: a weakening pack at a
/// distant congruence level must land low.
#[tokio::test]
async fn the_leaf_score_still_discriminates() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(&tmp).await;
    store
        .create_artifact_for_test(&artifact(
            "EVID-002",
            "evidence",
            "active",
            "verdict: weakens\ncongruence_level: 3\nevidence_type: measurement\n",
        ))
        .await
        .unwrap();

    // `weakens` is 0.5, CL3 costs nothing, `measurement` costs nothing — so the
    // computed value is exactly 0.5, and only the fix can produce it.
    //
    // The first draft of this test used `weakens` at CL1 with `audit` and
    // asserted `s < 0.2`. That passes on the broken code too, where every leaf
    // is 0.0 — and worse, the honest score for those inputs IS 0.0 after
    // penalties, so tightening it to `s > 0.0` made the test wrong rather than
    // stronger. Mutation testing found the first mistake; running the fixed
    // tree found the second.
    let s = score(&store, "EVID-002").await;
    assert!(
        (s - 0.5).abs() < 1e-9,
        "a weakening pack states half a case, not none and not all; got {s}"
    );
}

/// FR-002. An outgoing `informs` points at what the pack SUPPORTS. Treating it
/// as a dependency made trust in a measurement flow down from the decision it
/// justifies — so a well-formed pack attached to an unevidenced PRD scored
/// zero, which is the tail wagging the dog.
#[tokio::test]
async fn a_pack_is_not_dragged_down_by_the_artifact_it_supports() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(&tmp).await;
    store
        .create_artifact_for_test(&artifact("PRD-001", "prd", "active", "no evidence here"))
        .await
        .unwrap();
    store
        .create_artifact_for_test(&artifact("EVID-003", "evidence", "active", CANONICAL))
        .await
        .unwrap();
    store
        .add_relation_for_test("EVID-003", "PRD-001", "informs")
        .await
        .unwrap();

    let s = score(&store, "EVID-003").await;
    assert!(
        (s - 1.0).abs() < 1e-9,
        "the pack's own quality does not depend on what it informs; got {s}"
    );
}

/// #392, narrowed. The routing table calls a Note the artifact for trivial
/// reversible work — no evidence required. The cascade then read an active
/// unevidenced Note as zero trust and poisoned everything based on it, so
/// forgeplan contradicted itself: no evidence needed here, everything
/// downstream unevidenced.
#[tokio::test]
async fn an_exempt_note_does_not_poison_what_is_built_on_it() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(&tmp).await;
    store
        .create_artifact_for_test(&artifact("NOTE-001", "note", "active", "a design choice"))
        .await
        .unwrap();
    store
        .create_artifact_for_test(&artifact("PRD-002", "prd", "active", "well evidenced"))
        .await
        .unwrap();
    store
        .create_artifact_for_test(&artifact("EVID-004", "evidence", "active", CANONICAL))
        .await
        .unwrap();
    store
        .add_relation_for_test("EVID-004", "PRD-002", "informs")
        .await
        .unwrap();
    store
        .add_relation_for_test("PRD-002", "NOTE-001", "based_on")
        .await
        .unwrap();

    let s = score(&store, "PRD-002").await;
    assert!(
        s > 0.0,
        "a Note the methodology exempts from evidence must not zero its dependants; got {s}"
    );
}

/// The other half of the same decision, and the one that keeps the product
/// honest: a kind that CAN owe evidence and has none is real debt, and the
/// weakest-link cascade surfacing it is the whole point. The #392 proposals
/// that would have softened this were declined.
#[tokio::test]
async fn an_unevidenced_prd_still_drags_its_dependants_down() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(&tmp).await;
    store
        .create_artifact_for_test(&artifact("PRD-100", "prd", "active", "no evidence"))
        .await
        .unwrap();
    store
        .create_artifact_for_test(&artifact("PRD-101", "prd", "active", "well evidenced"))
        .await
        .unwrap();
    store
        .create_artifact_for_test(&artifact("EVID-005", "evidence", "active", CANONICAL))
        .await
        .unwrap();
    store
        .add_relation_for_test("EVID-005", "PRD-101", "informs")
        .await
        .unwrap();
    store
        .add_relation_for_test("PRD-101", "PRD-100", "based_on")
        .await
        .unwrap();

    let s = score(&store, "PRD-101").await;
    assert_eq!(
        s, 0.0,
        "an unevidenced PRD in the chain is debt the score must keep showing"
    );
}
