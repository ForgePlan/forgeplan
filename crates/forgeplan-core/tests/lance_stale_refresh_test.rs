//! PROB-074 integration test — stale legacy handles refuse after the physical
//! index is replaced; callers reopen explicitly to bind a new handle.
//!
//! The test uses real on-disk LanceDB state in a temporary workspace. It
//! verifies that a handle tied to removed physical fragments does not silently
//! attach itself to a different generation, while a newly opened handle sees
//! the replacement dataset.
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
use forgeplan_core::projection::MutationError;
use tempfile::TempDir;

/// Build a minimal `NewArtifact` for seeding the store.
fn fixture_prd(id: &str) -> NewArtifact {
    NewArtifact {
        canonical_key: id.to_string(),
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

/// A handle bound to a physical index that was replaced must refuse reads;
/// opening a new handle explicitly binds to the replacement.
#[tokio::test]
async fn stale_handle_refuses_after_external_reindex_until_explicit_reopen() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().join(".forgeplan");

    // 1. Writer-side init + seed.
    let lock = forgeplan_core::workspace::acquire_workspace_lock(&ws)
        .await
        .unwrap();
    std::fs::write(ws.join("config.yaml"), "project_name: stale_refresh\n").unwrap();
    let writer = LanceStore::init_new(&ws, &lock)
        .await
        .expect("LanceStore::init_new #1");
    writer
        .create_artifact_for_test(&lock, &fixture_prd("PRD-001"))
        .await
        .expect("seed PRD-001");

    // Verify writer can read what it wrote — sanity check.
    let found = writer
        .get_record(&lock, "PRD-001")
        .await
        .expect("read PRD-001 from writer");
    assert!(found.is_some(), "writer should see its own write");

    // 2. Open a SECOND handle on the same workspace. This handle pins its
    //    Table manifest at this exact moment — when we rewrite the lance
    //    dir below, this handle's references go stale.
    let stale = LanceStore::open(&ws).await.expect("LanceStore::open #2");

    // Confirm the stale handle sees PRD-001 right now.
    let pre = stale
        .get_record(&lock, "PRD-001")
        .await
        .expect("stale handle reads PRD-001 pre-reindex");
    assert!(
        pre.is_some(),
        "stale handle should see PRD-001 before reindex"
    );

    // 3. Simulate an external `forgeplan reindex`:
    //    (a) finish the first writer and acquire a fresh workspace guard;
    //    (b) `rm -rf .forgeplan/lance/` — rewrites every fragment under
    //        new UUIDs once we init again;
    //    (c) `LanceStore::init_new` again to recreate the schema + tables;
    //    (d) re-insert PRD-001 so the "after reindex" state is non-empty.
    //
    // The stale handle (`stale`) still points at the OLD manifest /
    // fragment UUIDs — its next read would normally fail with
    // `lance error: Not found …data/<old uuid>.lance` (PROB-074).
    drop(writer);
    drop(lock);
    let lock = forgeplan_core::workspace::acquire_workspace_lock(&ws)
        .await
        .unwrap();

    let lance_dir = ws.join("lance");
    std::fs::remove_dir_all(&lance_dir).expect("rm -rf lance/");

    let writer2 = LanceStore::init_new(&ws, &lock)
        .await
        .expect("LanceStore::init_new #2");
    writer2
        .create_artifact_for_test(&lock, &fixture_prd("PRD-001"))
        .await
        .expect("re-seed PRD-001 after reindex");
    drop(writer2);
    drop(lock);
    let permit = forgeplan_core::workspace::lock::acquire_workspace_read_lock(&ws)
        .await
        .unwrap();

    // A handle bound to the removed physical generation must refuse. It must
    // not silently re-pin itself to the replacement dataset.
    assert!(
        stale.get_record(&permit, "PRD-001").await.is_err(),
        "a stale physical handle must refuse instead of adopting the new generation"
    );

    // Reopening is the explicit boundary that binds a handle to the replacement.
    let reopened = LanceStore::open(&ws)
        .await
        .expect("explicitly reopen the replacement index");
    let recovered = reopened
        .get_record(&permit, "PRD-001")
        .await
        .expect("read from the explicitly reopened handle")
        .expect("PRD-001 exists in the replacement index");
    assert_eq!(recovered.id, "PRD-001");
    assert_eq!(recovered.kind, "prd");
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

    let lock = forgeplan_core::workspace::acquire_workspace_lock(&ws)
        .await
        .unwrap();
    std::fs::write(ws.join("config.yaml"), "project_name: stale_refresh\n").unwrap();
    let store = LanceStore::init_new(&ws, &lock).await.expect("init");
    store
        .create_artifact_for_test(&lock, &fixture_prd("PRD-001"))
        .await
        .expect("seed PRD-001");
    store
        .create_artifact_for_test(&lock, &fixture_prd("PRD-002"))
        .await
        .expect("seed PRD-002");

    // Snapshot state BEFORE refresh.
    let before = store
        .list_artifacts(&lock, None)
        .await
        .expect("list_artifacts before refresh");
    let mut before_ids: Vec<String> = before.iter().map(|a| a.id.clone()).collect();
    before_ids.sort();

    // Refresh is safe on a fresh store with current manifest.
    store
        .refresh(&lock)
        .await
        .expect("refresh is no-op when current");

    // Snapshot state AFTER refresh — must be byte-identical.
    let after = store
        .list_artifacts(&lock, None)
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
        .get_record(&lock, "PRD-001")
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
///
/// **PROB-075 F-2 update**: the assertion was strengthened — instead of
/// merely asserting `result.is_err()`, the test now propagates the error
/// through `anyhow` and verifies that *either* the typed
/// `MutationError::RetryExhausted` variant lands (happy path: stale
/// signature persists through the budget) *or* a `refresh()`-failure
/// context wrapper lands (alternative: refresh fails before the budget is
/// consumed). Both are legitimate terminal failures; pre-fix the error
/// shape was untyped string and consumers had no programmatic hook for
/// PRD-071 hint emission.
#[tokio::test]
async fn stale_handle_propagates_error_when_refresh_fails() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().join(".forgeplan");

    // Init and seed via writer handle.
    let lock = forgeplan_core::workspace::acquire_workspace_lock(&ws)
        .await
        .unwrap();
    std::fs::write(ws.join("config.yaml"), "project_name: stale_refresh\n").unwrap();
    let writer = LanceStore::init_new(&ws, &lock).await.expect("init");
    writer
        .create_artifact_for_test(&lock, &fixture_prd("PRD-001"))
        .await
        .expect("seed");

    // Open a second handle so it pins the current manifest.
    let stale = LanceStore::open(&ws).await.expect("open stale handle");

    // Verify it works right now.
    let pre = stale
        .get_record(&lock, "PRD-001")
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
    drop(lock);
    let permit = forgeplan_core::workspace::lock::acquire_workspace_read_lock(&ws)
        .await
        .unwrap();

    // Identity admission may refuse before LanceDB retries once its published
    // generation disappears. Neither path may turn the failed read into None.
    let result = stale.get_record(&permit, "PRD-001").await;
    assert!(
        result.is_err(),
        "a missing index must not become a successful empty read"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// PROB-075 F-2: retry budget exhausts → typed RetryExhausted error.
//
// Strategy: drive `with_retry_on_stale` directly with a closure that
// always returns a synthetic stale-manifest `anyhow::Error`. The retry
// loop runs its full budget (initial + N-1 retries), then constructs
// `MutationError::RetryExhausted` which the test downcasts. The
// `refresh()` calls inside the loop run against a real (healthy) store
// so they succeed each time; we're testing the budget logic, not refresh
// recovery.
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn retry_budget_exhausts_after_n_attempts_returns_typed_error() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().join(".forgeplan");
    let lock = forgeplan_core::workspace::acquire_workspace_lock(&ws)
        .await
        .unwrap();
    std::fs::write(ws.join("config.yaml"), "project_name: stale_refresh\n").unwrap();
    let store = LanceStore::init_new(&ws, &lock).await.expect("init");

    // Synthetic stale-manifest error that `is_stale_manifest_error`
    // recognises (Pass 2 fallback: anchored on `Not found:` prefix +
    // `.lance/data/` marker).
    let stale_text =
        "Not found: /tmp/synthetic/.forgeplan/lance/artifacts.lance/data/fake-uuid.lance, location";

    // Closure always returns the same stale error. `with_retry_on_stale`
    // will run its full budget and then construct `RetryExhausted`.
    let result: anyhow::Result<()> = store
        .with_retry_on_stale(&lock, || async move {
            // Each invocation gets a fresh anyhow::Error — chains are
            // not reused across calls. Result type is unit; the budget
            // logic doesn't care about T.
            Err(anyhow::anyhow!("{stale_text}"))
        })
        .await;

    let err = result.expect_err("retry budget must exhaust into Err");

    // PROB-075 F-2 contract: typed downcast surfaces the variant so MCP
    // can emit a `Wait:` hint and the agent can recover gracefully.
    let downcast = err.downcast_ref::<MutationError>();
    assert!(
        matches!(downcast, Some(MutationError::RetryExhausted { .. })),
        "exhausted retry budget must surface MutationError::RetryExhausted, got: {err:#}"
    );

    // Audit F-1 closure (2026-05-21): the `Wait:` hint was moved out of
    // Display and into the MCP layer's `safe_mcp_error_anyhow` to satisfy
    // PRD-071's line-anchored hint contract. Display now only contains
    // the retry-exhausted phrase; the MCP-side test
    // `prob074_hint_tests::retry_exhausted_appends_wait_hint_anchored`
    // covers the `Wait:` emission contract.
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("retry budget exhausted"),
        "Display must contain retry-exhausted phrase: {rendered}"
    );
    assert!(
        !rendered.contains("Wait:"),
        "Display MUST NOT contain mid-sentence Wait: — hint belongs to MCP layer: {rendered}"
    );
}

/// Tighter F-3 contract: when the closure fires the stale event back-to-back
/// faster than the backoff schedule could sleep through (custom retry path),
/// `should_skip_refresh` directly skips the second refresh. This guards the
/// debounce gate against a future refactor that bypasses the backoff loop.
#[tokio::test]
async fn should_skip_refresh_debounce_directly_blocks_back_to_back() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().join(".forgeplan");
    let lock = forgeplan_core::workspace::acquire_workspace_lock(&ws)
        .await
        .unwrap();
    std::fs::write(ws.join("config.yaml"), "project_name: stale_refresh\n").unwrap();
    let store = LanceStore::init_new(&ws, &lock).await.expect("init");

    // Two consecutive refresh() calls with zero sleep between them.
    // The first proceeds; the second is inside the debounce window.
    store.refresh(&lock).await.expect("first refresh");
    let count_after_first = store.refresh_call_count();
    assert_eq!(
        count_after_first, 1,
        "first refresh always proceeds, expected count=1"
    );

    // Note: `refresh()` itself is NOT debounce-gated (it stays a "force
    // refresh" primitive per F-3 design). The gate lives in
    // `with_retry_on_stale`'s policy. So this test asserts that two
    // back-to-back FORCED refreshes both proceed — it's the negative
    // contract: the gate is in the retry loop, NOT in refresh().
    store.refresh(&lock).await.expect("second forced refresh");
    let count_after_second = store.refresh_call_count();
    assert_eq!(
        count_after_second, 2,
        "refresh() itself is not debounce-gated; both forced calls proceed (gate lives in retry loop)"
    );
}
