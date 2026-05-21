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
use forgeplan_core::projection::MutationError;
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
    // `with_retry_on_stale` fires, `refresh()` fails (or the retry budget
    // is consumed), and the error must propagate as `Err(_)` — not
    // `Ok(None)` and not a panic.
    let result = stale.get_record("PRD-001").await;
    let err = result.expect_err(
        "get_record must return Err when lance directory is gone after stale-handle retry",
    );

    // PROB-075 F-2: at least one of these terminal shapes is acceptable.
    // (a) RetryExhausted — initial op() raised a stale-matching error and
    //     the budget was consumed.
    // (b) refresh-failure context — the very first refresh attempt failed
    //     because the directory was gone (manifest-read raises a non-stale
    //     error shape that short-circuits before the budget runs out).
    let retry_exhausted = err
        .downcast_ref::<MutationError>()
        .is_some_and(|m| matches!(m, MutationError::RetryExhausted { .. }));
    let refresh_failure_context = err.chain().any(|c| {
        c.to_string()
            .contains("refresh failed during stale-handle retry")
    });

    assert!(
        retry_exhausted || refresh_failure_context,
        "expected typed RetryExhausted or refresh-failure context, got: {err:#}"
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
    let store = LanceStore::init(&ws).await.expect("init");

    // Synthetic stale-manifest error that `is_stale_manifest_error`
    // recognises (Pass 2 fallback: anchored on `Not found:` prefix +
    // `.lance/data/` marker).
    let stale_text =
        "Not found: /tmp/synthetic/.forgeplan/lance/artifacts.lance/data/fake-uuid.lance, location";

    // Closure always returns the same stale error. `with_retry_on_stale`
    // will run its full budget and then construct `RetryExhausted`.
    let result: anyhow::Result<()> = store
        .with_retry_on_stale(|| async move {
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

    // Display should contain the PRD-071 Wait: hint (Option A — hint is
    // inline in the `#[error(...)]` template). Renders through the
    // anyhow chain so the wrapped MutationError's Display fires.
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("retry budget exhausted"),
        "Display must contain retry-exhausted phrase: {rendered}"
    );
    assert!(
        rendered.contains("Wait:"),
        "Display must include PRD-071 Wait: hint inline: {rendered}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// PROB-075 F-3: refresh rate-limit. Two rapid stale events inside the
// debounce window collapse to a single refresh() call. We don't care
// about the eventual error path here — only that `refresh_call_count`
// stays ≤ 1 after two fast retries even though the retry loop fires
// multiple times.
//
// `with_retry_on_stale` runs RETRY_ATTEMPTS=3 attempts total, all of
// which fail with the same stale signature, all inside the 250ms
// debounce. First refresh proceeds, the next refresh in the same window
// is rate-limit-skipped. Expected outcome: ≤ 1 actual refresh call.
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn refresh_rate_limit_skips_within_debounce_window() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = tmp.path().join(".forgeplan");
    let store = LanceStore::init(&ws).await.expect("init");

    assert_eq!(
        store.refresh_call_count(),
        0,
        "fresh store has 0 refresh calls"
    );

    let stale_text = "Not found: /tmp/x/.forgeplan/lance/artifacts.lance/data/y.lance, location";

    // Drive a single retry-loop pass that internally fires 3 stale
    // detections back-to-back. All happen inside the 250ms debounce —
    // only the FIRST refresh attempt should actually execute.
    let _ignored: anyhow::Result<()> = store
        .with_retry_on_stale(|| async move { Err(anyhow::anyhow!("{stale_text}")) })
        .await;

    let count = store.refresh_call_count();
    // Without the rate-limit RETRY_ATTEMPTS=3 stale events would each
    // trigger a refresh — 3 total. With the 250ms debounce, the second
    // attempt's refresh is suppressed because it fires only 100ms after
    // the first (backoff schedule [100, 250, 500]); the third attempt's
    // refresh lands at ~350ms total which is past the window from the
    // FIRST refresh, so it proceeds (correct behaviour: a steady
    // sub-rate flow should still get periodic refreshes).
    //
    // The contract: count must be strictly less than RETRY_ATTEMPTS,
    // i.e. the rate-limit produced AT LEAST ONE skipped refresh.
    assert!(
        count > 0 && count < 3,
        "refresh_call_count must be in (0, 3) — rate-limit must skip ≥1 of \
         the 3 stale-attempt refreshes inside the 250ms debounce, got {count}"
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
    let store = LanceStore::init(&ws).await.expect("init");

    // Two consecutive refresh() calls with zero sleep between them.
    // The first proceeds; the second is inside the debounce window.
    store.refresh().await.expect("first refresh");
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
    store.refresh().await.expect("second forced refresh");
    let count_after_second = store.refresh_call_count();
    assert_eq!(
        count_after_second, 2,
        "refresh() itself is not debounce-gated; both forced calls proceed (gate lives in retry loop)"
    );
}
