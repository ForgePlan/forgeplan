//! PROB-074 integration test — `LanceStore::with_retry_on_stale` must
//! transparently recover when the on-disk lance fragments are rewritten
//! by an external process (CLI `forgeplan reindex`, manual
//! `rm -rf .forgeplan/lance && forgeplan init`, migration tooling).
//!
//! # Test strategy
//!
//! 1. Init a workspace via `LanceStore::init` → "writer" handle.
//! 2. Insert one artifact via the writer.
//! 3. Open a SECOND `LanceStore` on the same path → "stale-able" handle
//!    (currently fresh, but its `Table` references will be invalidated
//!    once we rewrite the fragments below).
//! 4. Through the FIRST handle (or directly via fs) blow away
//!    `.forgeplan/lance/` and rebuild it: drop the writer handle, then
//!    `rm -rf` the directory, then `LanceStore::init` again so we end up
//!    with the empty schema. We then re-insert the artifact into the
//!    rebuilt store (otherwise the stale handle's retry would hit the
//!    new manifest but find no rows — that's a different failure mode).
//! 5. Through the SECOND (now stale) handle call `get_record`. Without
//!    the PROB-074 fix this fails with `lance error: Not found …data/<old uuid>.lance`.
//!    With the fix, `with_retry_on_stale` detects the stale-manifest
//!    error, calls `checkout_latest` on every table, and re-runs the
//!    closure — returning the artifact transparently.
//!
//! # Why this is an integration test (not a unit test)
//!
//! The bug surfaces only when (a) two handles point at the same lance
//! directory and (b) the directory is rewritten between handle-open and
//! handle-read. Both are off-process effects; mocking the lancedb
//! Table layer would just exercise the retry plumbing, not the actual
//! manifest-version interaction. The test runs against real on-disk
//! state in a `tempfile::TempDir`.
//!
//! # Hermetic
//!
//! Uses `tempfile::TempDir` so no shared state with the repo or other
//! tests. Runs single-threaded inside the test runtime; doesn't spawn
//! subprocesses (no `forgeplan` CLI binary needed).

use forgeplan_core::db::store::{LanceStore, NewArtifact};
use tempfile::TempDir;

/// Build a minimal `NewArtifact` for seeding the store.
fn fixture_prd(id: &str) -> NewArtifact {
    NewArtifact {
        id: id.to_string(),
        kind: "prd".to_string(),
        status: "active".to_string(),
        title: format!("Test PRD for {id}"),
        body: "## Problem\n\nStale-handle reproducer.\n\n## Goals\n\nNone.\n".to_string(),
        depth: "tactical".to_string(),
        author: Some("prob-074-test".to_string()),
        parent_epic: None,
        valid_until: None,
        tags: vec![],
    }
}

/// PROB-074 — stale handle survives external reindex through transparent
/// `checkout_latest` retry.
///
/// Without the fix this test panics on `get_record` with
/// `lance error: Not found: …/.lance/data/<uuid>.lance`. With the fix
/// `with_retry_on_stale` catches the stale-manifest signature, refreshes
/// the handle, and re-runs the read — which then returns the artifact
/// the new writer inserted.
#[tokio::test]
async fn stale_handle_auto_recovers_after_external_reindex() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().join(".forgeplan");

    // 1. Writer-side init + seed.
    let writer = LanceStore::init(&ws).await.expect("LanceStore::init #1");
    writer
        .create_artifact_for_test(&fixture_prd("PRD-001"))
        .await
        .expect("seed PRD-001");

    // Verify writer can read what it wrote — sanity check.
    let found = writer
        .get_record("PRD-001")
        .await
        .expect("read PRD-001 from writer");
    assert!(found.is_some(), "writer should see its own write");

    // 2. Open a SECOND handle on the same workspace. This handle pins its
    //    Table manifest at this exact moment — when we rewrite the lance
    //    dir below, this handle's references go stale.
    let stale = LanceStore::open(&ws).await.expect("LanceStore::open #2");

    // Confirm the stale handle sees PRD-001 right now.
    let pre = stale
        .get_record("PRD-001")
        .await
        .expect("stale handle reads PRD-001 pre-reindex");
    assert!(
        pre.is_some(),
        "stale handle should see PRD-001 before reindex"
    );

    // 3. Simulate an external `forgeplan reindex`:
    //    (a) drop the writer so no FS locks are held;
    //    (b) `rm -rf .forgeplan/lance/` — rewrites every fragment under
    //        new UUIDs once we init again;
    //    (c) `LanceStore::init` again to recreate the schema + tables;
    //    (d) re-insert PRD-001 so the "after reindex" state is non-empty.
    //
    // The stale handle (`stale`) still points at the OLD manifest /
    // fragment UUIDs — its next read would normally fail with
    // `lance error: Not found …data/<old uuid>.lance` (PROB-074).
    drop(writer);

    let lance_dir = ws.join("lance");
    std::fs::remove_dir_all(&lance_dir).expect("rm -rf lance/");

    let writer2 = LanceStore::init(&ws).await.expect("LanceStore::init #2");
    writer2
        .create_artifact_for_test(&fixture_prd("PRD-001"))
        .await
        .expect("re-seed PRD-001 after reindex");
    drop(writer2);

    // 4. Now read through the stale handle. With the PROB-074 fix the
    //    stale-manifest error is caught by `with_retry_on_stale`, the
    //    handle is refreshed via `checkout_latest`, and the read
    //    succeeds against the new manifest.
    let recovered = stale
        .get_record("PRD-001")
        .await
        .expect("stale handle recovers via with_retry_on_stale (PROB-074)");
    assert!(
        recovered.is_some(),
        "expected stale handle to auto-recover and see PRD-001 after external reindex"
    );

    let rec = recovered.unwrap();
    assert_eq!(rec.id, "PRD-001");
    assert_eq!(rec.kind, "prd");

    // 5. Subsequent reads on the (now-refreshed) handle should succeed
    //    without further retries — checkout_latest is sticky, the handle
    //    now references the current manifest.
    let again = stale
        .get_record("PRD-001")
        .await
        .expect("second read after recovery");
    assert!(again.is_some(), "post-recovery reads stay green");
}

/// Bonus: refresh() is idempotent on an up-to-date handle (no-op).
/// Documents the contract; protects against a future change that would
/// regress refresh into a destructive operation.
///
/// CHEAP 3 (code-reviewer #5): strengthened to assert record count + IDs
/// are identical before and after refresh, not just "no panic + data
/// survives". The original test was too weak — a refresh that silently
/// doubled or dropped rows would have passed.
#[tokio::test]
async fn refresh_is_noop_when_manifest_is_current() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().join(".forgeplan");

    let store = LanceStore::init(&ws).await.expect("init");
    store
        .create_artifact_for_test(&fixture_prd("PRD-001"))
        .await
        .expect("seed PRD-001");
    store
        .create_artifact_for_test(&fixture_prd("PRD-002"))
        .await
        .expect("seed PRD-002");

    // Snapshot state BEFORE refresh.
    let before = store
        .list_artifacts(None)
        .await
        .expect("list_artifacts before refresh");
    let mut before_ids: Vec<String> = before.iter().map(|a| a.id.clone()).collect();
    before_ids.sort();

    // Refresh is safe on a fresh store with current manifest.
    store
        .refresh()
        .await
        .expect("refresh is no-op when current");

    // Snapshot state AFTER refresh — must be byte-identical.
    let after = store
        .list_artifacts(None)
        .await
        .expect("list_artifacts after refresh");
    let mut after_ids: Vec<String> = after.iter().map(|a| a.id.clone()).collect();
    after_ids.sort();

    assert_eq!(
        before_ids, after_ids,
        "refresh must not add, remove, or reorder artifacts"
    );
    assert_eq!(
        before.len(),
        after.len(),
        "record count must be identical after no-op refresh"
    );

    // Original contract: data still readable.
    let record = store
        .get_record("PRD-001")
        .await
        .expect("read after refresh");
    assert!(record.is_some(), "data still visible after no-op refresh");
}

/// CHEAP 4 (code-reviewer #6): terminal-failure scenario — once the lance
/// directory is completely gone, `get_record` on the stale handle must
/// propagate an `Err` rather than panicking or returning `Ok(None)`.
///
/// This guards the error-propagation contract: `with_retry_on_stale`
/// refreshes handles and retries once; when the underlying directory has
/// been removed entirely the retry also fails and the `Err` propagates to
/// the MCP layer, which injects the actionable hint.
#[tokio::test]
async fn stale_handle_propagates_error_when_refresh_fails() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().join(".forgeplan");

    // Init and seed via writer handle.
    let writer = LanceStore::init(&ws).await.expect("init");
    writer
        .create_artifact_for_test(&fixture_prd("PRD-001"))
        .await
        .expect("seed");

    // Open a second handle so it pins the current manifest.
    let stale = LanceStore::open(&ws).await.expect("open stale handle");

    // Verify it works right now.
    let pre = stale
        .get_record("PRD-001")
        .await
        .expect("pre-drop read succeeds");
    assert!(
        pre.is_some(),
        "handle should see data before directory drop"
    );

    // Drop the writer and erase the entire lance directory — no re-init.
    // The stale handle now references UUIDs that no longer exist on disk.
    drop(writer);
    let lance_dir = ws.join("lance");
    std::fs::remove_dir_all(&lance_dir).expect("rm -rf lance/");

    // After the lance directory is gone there is no manifest to checkout.
    // `with_retry_on_stale` fires, `refresh()` fails, and the error must
    // propagate as `Err(_)` — not `Ok(None)` and not a panic.
    let result = stale.get_record("PRD-001").await;
    assert!(
        result.is_err(),
        "get_record must return Err when lance directory is gone after stale-handle retry"
    );
}
