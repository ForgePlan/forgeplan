//! Wave 9 edge-case worker — CR-001 deeper parity: `possible_duplicates`
//! and `active_stubs` arrays must appear in MCP `forgeplan_health` JSON
//! AND each entry MUST have the exact shape the CLI `--json` surface
//! produces.
//!
//! Pre-CR-001 MCP omitted both fields entirely. Agents reading
//! `verdict == "unhealthy"` had no way to inspect WHY without an extra
//! round-trip to a different tool. CR-001 closes that by emitting the
//! arrays under the same field names with the same per-entry shape as
//! the CLI `--json`. This test pins the contract end-to-end.
//!
//! The existing `verdict_cli_vs_mcp_consistency_test.rs` covers
//! `verdict` + `verdict_summary` parity. This file extends to the
//! detail arrays (`possible_duplicates`, `active_stubs`) so a future
//! refactor that drops or renames a per-entry field surfaces here.

use forgeplan_core::db::store::NewArtifact;
use serde_json::Value;

mod common;
use common::McpFixture;

/// Seed a fixture workspace with:
/// - Two identical-titled PRDs (similarity 100% → duplicate pair).
/// - One PRD whose body is mostly TBD/TODO markers (active stub).
///
/// All artifacts are status=active so they appear in the panels.
async fn seed_dup_and_stub_fixture(store: std::sync::Arc<forgeplan_core::db::store::LanceStore>) {
    // Two duplicate PRDs.
    store
        .create_artifact_for_test(&NewArtifact {
            id: "PRD-DUP-A".to_string(),
            kind: "prd".to_string(),
            status: "active".to_string(),
            title: "Same-shape PRD".to_string(),
            body: "## Problem\nReal text.\n\n## Goals\nReal goals.\n".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        })
        .await
        .expect("seed PRD-DUP-A");
    store
        .create_artifact_for_test(&NewArtifact {
            id: "PRD-DUP-B".to_string(),
            kind: "prd".to_string(),
            status: "active".to_string(),
            title: "Same-shape PRD".to_string(),
            body: "## Problem\nReal text.\n\n## Goals\nReal goals.\n".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        })
        .await
        .expect("seed PRD-DUP-B");

    // One active stub: PRD with 3+ stub markers in body. The
    // stub detector in validation::rules::check_stub_detailed
    // triggers on `count >= 3`. Use canonical English template
    // phrases (matches PHRASE_MARKERS) + a {placeholder} marker
    // to exceed the threshold.
    store
        .create_artifact_for_test(&NewArtifact {
            id: "PRD-STUB".to_string(),
            kind: "prd".to_string(),
            status: "active".to_string(),
            title: "Stub PRD title".to_string(),
            body: "## Problem\nWhat we are building and why\n\n\
                   ## Goals\nHow the problem affects users\n\n\
                   ## Non-Goals\nWhat's in the MVP\n\n\
                   ## Notes\nSee {placeholder} below.\n"
                .to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        })
        .await
        .expect("seed PRD-STUB");
}

/// MCP `forgeplan_health` MUST emit `possible_duplicates` AND
/// `active_stubs` arrays with the exact per-entry shape the CLI
/// `--json` surface produces.
#[tokio::test]
async fn mcp_health_emits_possible_duplicates_and_active_stubs_arrays() {
    let fixture = McpFixture::new_with_seed(seed_dup_and_stub_fixture).await;
    // Touch workspace_path to silence the `dead_code` warning on
    // common/mod.rs — this test binary doesn't otherwise read it,
    // but the field is part of the fixture contract.
    assert!(
        fixture.workspace_path.exists(),
        "fixture workspace path must exist"
    );

    let envelope = fixture
        .call_tool_json("forgeplan_health", serde_json::json!({}))
        .await;
    let payload = envelope.assert_ok();

    // ── possible_duplicates: present, non-empty, correct entry shape ──
    let dups = payload
        .get("possible_duplicates")
        .and_then(Value::as_array)
        .expect("possible_duplicates field present (CR-001 contract)");
    assert!(
        !dups.is_empty(),
        "duplicate pair MUST be detected for two identical-titled PRDs, got: {dups:?}"
    );
    for entry in dups {
        let obj = entry
            .as_object()
            .expect("each duplicate entry MUST be an object");
        for key in ["id_a", "id_b", "similarity", "title_a", "title_b", "kind"] {
            assert!(
                obj.contains_key(key),
                "duplicate entry missing required field {key:?}; entry={obj:?}"
            );
        }
        // Specific shape: similarity is a number, ids/titles/kind are strings.
        assert!(
            obj["id_a"].is_string(),
            "id_a must be string, got: {:?}",
            obj["id_a"]
        );
        assert!(
            obj["id_b"].is_string(),
            "id_b must be string, got: {:?}",
            obj["id_b"]
        );
        assert!(
            obj["similarity"].is_number(),
            "similarity must be number, got: {:?}",
            obj["similarity"]
        );
        assert!(obj["title_a"].is_string(), "title_a must be string");
        assert!(obj["title_b"].is_string(), "title_b must be string");
        assert!(obj["kind"].is_string(), "kind must be string");
    }

    // ── active_stubs: present, non-empty, correct entry shape ─────────
    let stubs = payload
        .get("active_stubs")
        .and_then(Value::as_array)
        .expect("active_stubs field present (CR-001 contract)");
    assert!(
        !stubs.is_empty(),
        "active stub MUST be detected for PRD-STUB with TBD/TODO/FIXME body, got: {stubs:?}"
    );
    for entry in stubs {
        let obj = entry
            .as_object()
            .expect("each active_stub entry MUST be an object");
        for key in ["id", "kind", "title", "markers_found", "message"] {
            assert!(
                obj.contains_key(key),
                "active_stub entry missing required field {key:?}; entry={obj:?}"
            );
        }
        assert!(obj["id"].is_string(), "id must be string");
        assert!(obj["kind"].is_string(), "kind must be string");
        assert!(obj["title"].is_string(), "title must be string");
        assert!(
            obj["markers_found"].is_number(),
            "markers_found must be number, got: {:?}",
            obj["markers_found"]
        );
        assert!(obj["message"].is_string(), "message must be string");
    }
}

/// Empty workspace MUST emit both arrays as empty (NOT absent).
/// Field presence is part of the contract — agents shouldn't have to
/// check key-existence and array-emptiness separately.
#[tokio::test]
async fn mcp_health_empty_workspace_still_emits_empty_arrays() {
    let fixture = McpFixture::new().await;
    let envelope = fixture
        .call_tool_json("forgeplan_health", serde_json::json!({}))
        .await;
    let payload = envelope.assert_ok();

    let dups = payload
        .get("possible_duplicates")
        .and_then(Value::as_array)
        .expect("possible_duplicates MUST be present even on empty workspace (presence contract)");
    assert!(
        dups.is_empty(),
        "empty workspace → possible_duplicates is empty array, got: {dups:?}"
    );

    let stubs = payload
        .get("active_stubs")
        .and_then(Value::as_array)
        .expect("active_stubs MUST be present even on empty workspace");
    assert!(
        stubs.is_empty(),
        "empty workspace → active_stubs is empty array, got: {stubs:?}"
    );

    // Sanity: verdict on empty workspace is Empty (not Healthy, not
    // NeedsAttention) — pins the Verdict::Empty short-circuit through
    // the live MCP surface (not just the unit fixture).
    let verdict = payload
        .get("verdict")
        .and_then(Value::as_str)
        .expect("verdict field present");
    assert_eq!(
        verdict, "empty",
        "empty workspace MUST return verdict='empty' (short-circuit), got: {verdict:?}"
    );
}
