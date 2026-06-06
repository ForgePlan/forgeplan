//! PROB-078 read-after-write reproduction harness.
//!
//! The stdio-printf repro flagged `forgeplan_update` → `forgeplan_get` in the
//! same MCP session as returning a STALE (pre-update) body. The store-layer
//! probe (`store::tests::prob078_reopened_handle_sees_own_update_body`) refuted
//! the simplest mechanism: a handle opened at version V1 DOES observe its own
//! in-place `update_body`. These tests push the same scenario through the FULL
//! MCP handler stack (resolve_workspace + workspace_store_cache + projection),
//! bisecting the structural differences of the stdio repro:
//!   1. artifact pre-created by a SEPARATE handle before the server opens
//!      (closest in-process analog to "created via CLI before the session"),
//!   2. explicit `workspace=` param on the read/write calls (Step-1 resolution),
//!   3. a read that opens+caches its store BEFORE the write lands on a
//!      different store (the two-handle staleness probe).
//!
//! A reliable in-process rmcp client is used throughout — NOT hand-rolled
//! stdio framing — so a PASS here is trustworthy where the stdio repro was not.

mod common;
use common::McpFixture;

use forgeplan_core::db::store::NewArtifact;

const MARKER: &str = "MARKER-PROB078-UNIQUE-DEADBEEF";

fn seed_artifact(id: &str) -> NewArtifact {
    NewArtifact {
        id: id.to_string(),
        kind: "prd".to_string(),
        status: "draft".to_string(),
        title: format!("Seed {id}"),
        body: "## Summary\n\nORIGINAL template body (pre-update).".to_string(),
        depth: "standard".to_string(),
        author: Some("prob078-probe".to_string()),
        parent_epic: None,
        valid_until: None,
        tags: Vec::new(),
    }
}

fn body_of(env: &common::CallToolEnvelope) -> String {
    env.assert_ok()["body"].as_str().unwrap_or("").to_string()
}

/// Variant 1: artifact created by a SEPARATE handle before the server opens,
/// then `update` + `get` through MCP with NO workspace param (default store).
/// This is mechanism A exercised through the full MCP stack.
#[tokio::test]
async fn repro_seed_then_update_get_no_ws() {
    let fx = McpFixture::new_with_seed(|store| async move {
        store
            .create_artifact_for_test(&seed_artifact("PRD-700"))
            .await
            .expect("seed PRD-700");
    })
    .await;

    let upd = fx
        .call_tool_json(
            "forgeplan_update",
            serde_json::json!({ "id": "PRD-700", "body": MARKER }),
        )
        .await;
    upd.assert_ok();

    let got = fx
        .call_tool_json("forgeplan_get", serde_json::json!({ "id": "PRD-700" }))
        .await;
    let body = body_of(&got);
    assert!(
        body.contains(MARKER),
        "PROB-078 (no-ws): read-after-write returned STALE body: {body:?}"
    );
}

/// Variant 2: same seed, but `update` + `get` both carry an explicit
/// `workspace=<root>` param (Step-1 resolution → get_or_open_store keyed by
/// the canonicalized .forgeplan dir). Closest in-process analog to the stdio
/// repro, which passed `workspace=WS` on both calls.
#[tokio::test]
async fn repro_seed_then_update_get_with_ws() {
    let fx = McpFixture::new_with_seed(|store| async move {
        store
            .create_artifact_for_test(&seed_artifact("PRD-701"))
            .await
            .expect("seed PRD-701");
    })
    .await;

    // workspace_path is the `.forgeplan` dir; the agent passes the PROJECT ROOT.
    let root = fx
        .workspace_path
        .parent()
        .expect("project root above .forgeplan")
        .to_path_buf();

    let upd = fx
        .call_tool_json(
            "forgeplan_update",
            serde_json::json!({ "id": "PRD-701", "body": MARKER, "workspace": root }),
        )
        .await;
    upd.assert_ok();

    let got = fx
        .call_tool_json(
            "forgeplan_get",
            serde_json::json!({ "id": "PRD-701", "workspace": root }),
        )
        .await;
    let body = body_of(&got);
    assert!(
        body.contains(MARKER),
        "PROB-078 (with-ws symmetric): read-after-write returned STALE body: {body:?}"
    );
}

/// Variant 3 (the two-store staleness probe): a `get` with `workspace=<root>`
/// runs FIRST — opening + caching a Step-1 store handle at the pre-update
/// snapshot. The `update` then lands with NO ws param (default-store path). If
/// the param path and the default path key DIFFERENT cache entries, the second
/// `get` would read the early-cached (stale) handle. This is the only
/// remaining structural way a single session can serve a stale read.
#[tokio::test]
async fn repro_get_first_then_update_then_get_with_ws() {
    let fx = McpFixture::new_with_seed(|store| async move {
        store
            .create_artifact_for_test(&seed_artifact("PRD-702"))
            .await
            .expect("seed PRD-702");
    })
    .await;

    let root = fx
        .workspace_path
        .parent()
        .expect("project root above .forgeplan")
        .to_path_buf();

    // (a) read FIRST with ws param — opens + caches the Step-1 store handle.
    let first = fx
        .call_tool_json(
            "forgeplan_get",
            serde_json::json!({ "id": "PRD-702", "workspace": root }),
        )
        .await;
    assert!(
        body_of(&first).contains("ORIGINAL template body"),
        "pre-update read should see the template body"
    );

    // (b) write with NO ws param — default-store resolution path.
    let upd = fx
        .call_tool_json(
            "forgeplan_update",
            serde_json::json!({ "id": "PRD-702", "body": MARKER }),
        )
        .await;
    upd.assert_ok();

    // (c) read AGAIN with ws param — if this is a different cached handle than
    //     the write touched, it returns the stale early snapshot.
    let second = fx
        .call_tool_json(
            "forgeplan_get",
            serde_json::json!({ "id": "PRD-702", "workspace": root }),
        )
        .await;
    let body = body_of(&second);
    assert!(
        body.contains(MARKER),
        "PROB-078 (two-store): read-after-write returned STALE body \
         (param-path and default-path resolve different cached stores): {body:?}"
    );
}
