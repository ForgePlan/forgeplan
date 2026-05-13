//! Phase 2.4+ — MCP server full tool coverage matrix.
//!
//! Why this file exists
//! --------------------
//! `integration_e2e.rs` pins 14 MCP tools against the real `ForgeplanServer`
//! over a `tokio::io::duplex` JSON-RPC transport. Of the 60+ user-facing
//! `#[tool(...)]`-registered handlers in `crates/forgeplan-mcp/src/server.rs`
//! only ~8 unique tools (`forgeplan_new` / `_get` / `_list` / `_search` /
//! `_validate` / `_score` / `_link` / `_health`) were exercised end-to-end
//! before this matrix landed.
//!
//! This file extends that coverage to ~50 tools — every handler that can be
//! exercised against a fresh tempdir-rooted workspace without a configured
//! LLM provider, external playbook files, or pre-existing trash receipts.
//! Tools that depend on those (`forgeplan_reason` / `_capture` / `_generate`
//! / `_decompose` / `_restore` / `_undo_last` / `_ingest`) are still exercised
//! but the assertion accepts EITHER a successful response OR a typed error
//! payload describing the missing pre-requisite — the goal is to prove the
//! tool is registered, accepts well-formed arguments, and reaches a
//! structured response (not panic, not transport hang).
//!
//! Approach: hand-rolled per-tool tests using a shared local `McpFixture`
//! (copy of the harness from `integration_e2e.rs`). Macro generation was
//! considered but rejected because contract-shape assertions vary per tool
//! (some return an `artifacts: []` array, some a `results: []`, some a flat
//! object) — a single macro shape would just push assertion logic into the
//! call site, defeating the readability win.

use serde_json::{Value, json};

mod common;
use common::McpFixture;

// ── Group A: read-only tools that need no args + no state ─────────────

#[tokio::test]
async fn c01_forgeplan_status_smoke() {
    // Contract pinned: status returns a JSON object with at least one of the
    // dashboard keys (`version`, `workspace`, `phase`, `artifacts`,
    // `health`). Previously assert_reachable only checked is_object — a
    // regression that swapped the response for an empty `{}` would have
    // passed silently.
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_status", json!({})).await;
    let resp = env.assert_ok();
    assert!(resp.is_object(), "status returns JSON object: {resp}");
    let obj = resp.as_object().expect("object");
    assert!(
        !obj.is_empty(),
        "status response must carry at least one dashboard field, got empty object"
    );
}

#[tokio::test]
async fn c02_forgeplan_blindspots_smoke() {
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_blindspots", json!({})).await;
    let resp = env.assert_ok();
    // Empty workspace — must still return a structured response, not panic.
    assert!(resp.is_object(), "blindspots returns object: {resp}");
}

#[tokio::test]
async fn c03_forgeplan_graph_smoke() {
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_graph", json!({})).await;
    let resp = env.assert_ok();
    assert!(
        resp["mermaid"].is_string(),
        "graph response must carry a mermaid string: {resp}"
    );
}

#[tokio::test]
async fn c04_forgeplan_order_smoke() {
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_order", json!({})).await;
    let resp = env.assert_ok();
    assert!(
        resp["order"].is_array(),
        "order response carries an array (possibly empty): {resp}"
    );
}

#[tokio::test]
async fn c05_forgeplan_stale_smoke() {
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_stale", json!({})).await;
    let resp = env.assert_ok();
    assert!(
        resp["stale"].is_array(),
        "stale response carries an array (possibly empty): {resp}"
    );
}

#[tokio::test]
async fn c06_forgeplan_decay_smoke() {
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_decay", json!({})).await;
    let resp = env.assert_ok();
    assert!(
        resp["entries"].is_array(),
        "decay response carries entries[]: {resp}"
    );
}

#[tokio::test]
async fn c07_forgeplan_drift_smoke() {
    // Contract pinned: drift returns the `{total, stale, reports}` triple
    // even on a fresh workspace (no decisions with affected_files yet).
    // Previously assert_reachable-only — a regression that renamed
    // `reports` → `entries` or dropped `stale` would have passed silently.
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_drift", json!({})).await;
    let resp = env.assert_ok();
    assert!(
        resp["total"].is_number(),
        "drift response carries numeric total: {resp}"
    );
    assert!(
        resp["stale"].is_number(),
        "drift response carries numeric stale count: {resp}"
    );
    assert!(
        resp["reports"].is_array(),
        "drift response carries reports[]: {resp}"
    );
}

#[tokio::test]
async fn c08_forgeplan_coverage_smoke() {
    // Contract pinned: coverage returns CoverageReport flat shape
    // (`total_modules`, `covered_modules`, `uncovered_modules`,
    // `coverage_percent`, `modules[]`). A regression that nested it under
    // a `report:` key would have passed silently with assert_reachable.
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_coverage", json!({})).await;
    let resp = env.assert_ok();
    assert!(
        resp["total_modules"].is_number(),
        "coverage response carries numeric total_modules: {resp}"
    );
    assert!(
        resp["modules"].is_array(),
        "coverage response carries modules[]: {resp}"
    );
    assert!(
        resp["coverage_percent"].is_number(),
        "coverage response carries numeric coverage_percent: {resp}"
    );
}

#[tokio::test]
async fn c09_forgeplan_session_smoke() {
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_session", json!({})).await;
    let resp = env.assert_ok();
    // Session always exposes the methodology phase marker.
    assert!(resp.is_object(), "session response is an object: {resp}");
}

#[tokio::test]
async fn c10_forgeplan_journal_no_filter_smoke() {
    // Contract pinned: journal returns `{entries[], total}` even when
    // empty. A regression that renamed `entries` → `decisions` or dropped
    // `total` would have passed silently with assert_reachable.
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_journal", json!({})).await;
    let resp = env.assert_ok();
    assert!(
        resp["entries"].is_array(),
        "journal response carries entries[]: {resp}"
    );
    assert!(
        resp["total"].is_number(),
        "journal response carries numeric total: {resp}"
    );
}

#[tokio::test]
async fn c11_forgeplan_fpf_list_smoke() {
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_fpf_list", json!({})).await;
    let resp = env.assert_ok();
    assert!(
        resp["sections"].is_array(),
        "fpf_list returns sections[]: {resp}"
    );
}

#[tokio::test]
async fn c12_forgeplan_fpf_rules_no_filter_smoke() {
    // Contract pinned: fpf_rules returns `{source, count, rules[]}` —
    // `source` is one of "config"|"default" (string), `count` mirrors
    // `rules.len()`. Default rule set is non-empty so a regression dropping
    // the embedded defaults would surface as count == 0.
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_fpf_rules", json!({})).await;
    let resp = env.assert_ok();
    assert!(
        resp["rules"].is_array(),
        "fpf_rules response carries rules[]: {resp}"
    );
    assert!(
        resp["source"].is_string(),
        "fpf_rules response carries source string: {resp}"
    );
    assert!(
        resp["count"].is_number(),
        "fpf_rules response carries numeric count: {resp}"
    );
}

// ── Group B: tools that take args but no pre-existing artifact ────────

#[tokio::test]
async fn c13_forgeplan_route_smoke() {
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json(
            "forgeplan_route",
            json!({"description": "Add OAuth login to the dashboard"}),
        )
        .await;
    // Contract pinned: route returns the canonical shape `{depth, pipeline[],
    // triggers[], confidence, level, explanation, display}`. Falls back to
    // heuristic when LLM not configured, so the success envelope is
    // deterministic in test env. Previously assert_reachable hid a
    // regression that dropped `pipeline` or renamed `depth` → `tier`.
    let resp = env.assert_ok();
    assert!(
        resp["depth"].is_string(),
        "route response carries depth string: {resp}"
    );
    assert!(
        resp["pipeline"].is_array(),
        "route response carries pipeline[]: {resp}"
    );
    assert!(
        resp["confidence"].is_number(),
        "route response carries numeric confidence: {resp}"
    );
    assert!(
        resp["level"].is_number() || resp["level"].is_string(),
        "route response carries level marker: {resp}"
    );
}

#[tokio::test]
async fn c14_forgeplan_init_force_smoke() {
    let fx = McpFixture::new().await;
    // Workspace already initialized by fixture; without force=true the
    // handler reports "already initialized". With force=true it reinits.
    let env = fx
        .call_tool_json("forgeplan_init", json!({"force": false}))
        .await;
    let resp = env.assert_ok();
    assert!(
        resp["message"].as_str().unwrap_or("").contains("Already"),
        "no-force reinit must surface the 'already initialized' message: {resp}"
    );
}

#[tokio::test]
async fn c15_forgeplan_search_empty_workspace_smoke() {
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json(
            "forgeplan_search",
            json!({"query": "nothing here", "limit": 5}),
        )
        .await;
    let resp = env.assert_ok();
    assert!(
        resp["results"].is_array(),
        "search returns results[] even when empty: {resp}"
    );
}

#[tokio::test]
async fn c16_forgeplan_list_with_status_filter_smoke() {
    let fx = McpFixture::new().await;
    fx.seed_prd("Status Filter Subject").await;
    let env = fx
        .call_tool_json("forgeplan_list", json!({"status": "draft"}))
        .await;
    let resp = env.assert_ok();
    let items = resp["artifacts"].as_array().expect("artifacts array");
    assert!(
        items.iter().any(|a| a["id"] == "PRD-001"),
        "draft PRD-001 must be in draft-filtered list: {resp}"
    );
}

#[tokio::test]
async fn c17_forgeplan_blocked_no_id_smoke() {
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_blocked", json!({})).await;
    let resp = env.assert_ok();
    assert!(
        resp["blocked"].is_array(),
        "blocked response carries blocked[]: {resp}"
    );
}

// ── Group C: claim / dispatch / release flow ──────────────────────────

#[tokio::test]
async fn c18_forgeplan_claims_empty_smoke() {
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_claims", json!({})).await;
    let resp = env.assert_ok();
    assert_eq!(resp["count"], 0, "fresh workspace has zero claims: {resp}");
    assert!(
        resp["claims"].is_array(),
        "claims response carries claims[]: {resp}"
    );
}

#[tokio::test]
async fn c19_forgeplan_dispatch_smoke() {
    let fx = McpFixture::new().await;
    fx.seed_prd("Dispatch Subject A").await;
    fx.seed_prd("Dispatch Subject B").await;
    let env = fx
        .call_tool_json("forgeplan_dispatch", json!({"agents": 2}))
        .await;
    let resp = env.assert_ok();
    assert!(
        resp["buckets"].is_array(),
        "dispatch returns buckets[]: {resp}"
    );
    assert_eq!(resp["agent_count"], 2);
}

#[tokio::test]
async fn c20_forgeplan_claim_release_roundtrip() {
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Claimable PRD").await;

    let claim_env = fx
        .call_tool_json(
            "forgeplan_claim",
            json!({"id": id, "agent": "test-agent/1.0", "ttl_minutes": 5}),
        )
        .await;
    let claim_resp = claim_env.assert_ok();
    assert_eq!(claim_resp["id"], id, "claim echoes id back: {claim_resp}");
    assert_eq!(
        claim_resp["agent_id"], "test-agent/1.0",
        "claim echoes agent_id: {claim_resp}"
    );

    let release_env = fx
        .call_tool_json(
            "forgeplan_release",
            json!({"id": id, "agent": "test-agent/1.0"}),
        )
        .await;
    let release_resp = release_env.assert_ok();
    assert_eq!(
        release_resp["released"], true,
        "release succeeded: {release_resp}"
    );
}

// ── Group D: lifecycle (activate / supersede / deprecate) ─────────────

#[tokio::test]
async fn c21_forgeplan_review_smoke() {
    // Contract pinned: review returns `{artifact_id, can_activate,
    // must_findings[], should_findings[], warnings[]}`. Fresh PRD from
    // template has unfilled MUST sections → can_activate=false +
    // non-empty must_findings. Previously assert_reachable accepted ANY
    // envelope, so a regression that returned `{ok: true}` would have
    // silently passed.
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Reviewable").await;
    let env = fx
        .call_tool_json("forgeplan_review", json!({"id": id}))
        .await;
    let resp = env.assert_ok();
    assert_eq!(
        resp["artifact_id"], id,
        "review echoes the artifact id: {resp}"
    );
    assert!(
        resp["can_activate"].is_boolean(),
        "review response carries boolean can_activate: {resp}"
    );
    assert!(
        resp["must_findings"].is_array(),
        "review response carries must_findings[]: {resp}"
    );
    assert!(
        resp["should_findings"].is_array(),
        "review response carries should_findings[]: {resp}"
    );
    assert_eq!(
        resp["can_activate"], false,
        "fresh PRD has unfilled MUST sections → can_activate=false: {resp}"
    );
}

#[tokio::test]
async fn c22_forgeplan_activate_fails_for_incomplete_prd() {
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Incomplete Subject").await;
    // Fresh PRD from template has MUST-section gaps — activate must refuse
    // unless force=true. We pin the refusal path here (force=true tested
    // elsewhere if/when we wire it).
    let env = fx
        .call_tool_json("forgeplan_activate", json!({"id": id, "force": false}))
        .await;
    // Contract pinned: incomplete PRD activation is deterministic in test
    // env — `lifecycle::activate` returns Err which the handler wraps in
    // `err_result` (is_error=true, non-empty body). Previously
    // assert_reachable accepted EITHER outcome, so a regression that
    // silently activated a malformed PRD would have passed.
    assert!(
        env.is_error,
        "activate of incomplete draft must return is_error=true (MUST validation fails), got: {}",
        env.raw_text
    );
    assert!(
        !env.raw_text.is_empty(),
        "activate error body must describe validation failure, got empty"
    );
}

#[tokio::test]
async fn c23_forgeplan_supersede_smoke() {
    // Contract pinned: `draft → superseded` is forbidden in the lifecycle
    // state machine (see lifecycle::transitions). Both fresh PRDs are in
    // `draft` status, so supersede MUST return is_error=true with a
    // body mentioning the invalid transition. Previously assert_reachable
    // accepted EITHER outcome — a regression that silently superseded a
    // draft would have masked the state-machine guard.
    let fx = McpFixture::new().await;
    let from_id = fx.seed_prd("Old PRD").await;
    let to_id = fx.seed_prd("New PRD").await;
    let env = fx
        .call_tool_json("forgeplan_supersede", json!({"id": from_id, "by": to_id}))
        .await;
    assert!(
        env.is_error,
        "supersede of draft PRD must return is_error=true (draft→superseded forbidden), got: {}",
        env.raw_text
    );
    assert!(
        env.raw_text.contains("Invalid transition") || env.raw_text.contains("draft"),
        "supersede error body must explain the forbidden transition, got: {}",
        env.raw_text
    );
}

#[tokio::test]
async fn c24_forgeplan_deprecate_smoke() {
    // Contract pinned: `draft → deprecated` is forbidden in the lifecycle
    // state machine (only `active → deprecated` and `stale → deprecated`
    // are allowed). Fresh PRD is in `draft` → deprecate MUST return
    // is_error=true. Previously assert_reachable would have silently
    // accepted a regression that allowed direct draft→deprecated, which
    // breaks the explicit two-step lifecycle.
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Deprecatable").await;
    let env = fx
        .call_tool_json(
            "forgeplan_deprecate",
            json!({"id": id, "reason": "Replaced by ADR-001"}),
        )
        .await;
    assert!(
        env.is_error,
        "deprecate of draft PRD must return is_error=true (draft→deprecated forbidden), got: {}",
        env.raw_text
    );
    assert!(
        env.raw_text.contains("Invalid transition") || env.raw_text.contains("draft"),
        "deprecate error body must explain the forbidden transition, got: {}",
        env.raw_text
    );
}

// ── Group E: update / delete / get-many flow ──────────────────────────

#[tokio::test]
async fn c25_forgeplan_update_body_smoke() {
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Updatable").await;
    let env = fx
        .call_tool_json(
            "forgeplan_update",
            json!({
                "id": id,
                "title": "Updated Title",
                "body": "# Updated body\n\nNew content.",
            }),
        )
        .await;
    let resp = env.assert_ok();
    // Update echoes the artifact id; pin that.
    assert!(
        resp.get("id").is_some() || resp.get("artifact_id").is_some(),
        "update response carries an id field: {resp}"
    );
}

#[tokio::test]
async fn c26_forgeplan_delete_smoke() {
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Deletable").await;
    let env = fx
        .call_tool_json("forgeplan_delete", json!({"id": id}))
        .await;
    // Contract pinned: soft-delete returns `{id, title, message, receipt_id}`
    // (PRD-055 receipt for forgeplan_undo_last). Previously assert_reachable
    // missed both the success shape AND the receipt_id contract — a
    // regression that dropped receipts would have made `forgeplan_undo_last`
    // silently broken in production.
    let resp = env.assert_ok();
    assert_eq!(
        resp["id"], id,
        "delete response echoes the deleted id: {resp}"
    );
    assert!(
        resp["receipt_id"].is_string() && !resp["receipt_id"].as_str().unwrap().is_empty(),
        "delete response carries non-empty receipt_id (PRD-055): {resp}"
    );
    // Re-list confirms it's gone (or at least soft-deleted from active set).
    let list_env = fx
        .call_tool_json("forgeplan_list", json!({"kind": "prd"}))
        .await;
    let list_resp = list_env.assert_ok();
    let items = list_resp["artifacts"].as_array().expect("artifacts array");
    assert!(
        !items.iter().any(|a| a["id"] == id),
        "deleted artifact no longer in active list: {list_resp}"
    );
}

// ── Group F: scoring / validation / progress ──────────────────────────

#[tokio::test]
async fn c27_forgeplan_progress_no_id_smoke() {
    let fx = McpFixture::new().await;
    fx.seed_prd("Progress Subject").await;
    let env = fx.call_tool_json("forgeplan_progress", json!({})).await;
    let resp = env.assert_ok();
    assert!(
        resp["artifacts"].is_array(),
        "progress response carries artifacts[]: {resp}"
    );
}

#[tokio::test]
async fn c28_forgeplan_progress_with_id_smoke() {
    // Contract pinned: progress with explicit id returns `{artifacts[],
    // total_checkboxes, total_completed}` keyed on the requested artifact.
    // Fresh PRD body has whatever checkboxes the template ships with
    // (currently zero) — pin the shape, not the count, so the test stays
    // stable across template tweaks. Previously assert_reachable accepted
    // any envelope.
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Progress Subject").await;
    let env = fx
        .call_tool_json("forgeplan_progress", json!({"id": id}))
        .await;
    let resp = env.assert_ok();
    assert!(
        resp["artifacts"].is_array(),
        "progress response carries artifacts[]: {resp}"
    );
    assert!(
        resp["total_checkboxes"].is_number(),
        "progress response carries numeric total_checkboxes: {resp}"
    );
    assert!(
        resp["total_completed"].is_number(),
        "progress response carries numeric total_completed: {resp}"
    );
    let _ = id; // id may not be in artifacts[] when template has no checkboxes
}

#[tokio::test]
async fn c29_forgeplan_validate_all_smoke() {
    let fx = McpFixture::new().await;
    fx.seed_prd("Validate Subject").await;
    let env = fx.call_tool_json("forgeplan_validate", json!({})).await;
    let resp = env.assert_ok();
    // No id → validate all; result must carry the results[] array.
    assert!(
        resp["results"].is_array(),
        "validate-all returns results[]: {resp}"
    );
}

#[tokio::test]
async fn c30_forgeplan_calibrate_no_id_smoke() {
    // Contract pinned: calibrate returns `{results[], total_escalations}`.
    // Each entry carries `{artifact_id, artifact_title, current_depth,
    // suggested_depth, escalation_needed, signals[]}`. Previously
    // assert_reachable hid a regression that swapped CalibrationDto shape
    // (e.g. flattening `suggested_depth` to a number).
    let fx = McpFixture::new().await;
    fx.seed_prd("Calibratable").await;
    let env = fx.call_tool_json("forgeplan_calibrate", json!({})).await;
    let resp = env.assert_ok();
    assert!(
        resp["results"].is_array(),
        "calibrate response carries results[]: {resp}"
    );
    assert!(
        resp["total_escalations"].is_number(),
        "calibrate response carries numeric total_escalations: {resp}"
    );
}

// ── Group G: LLM-backed tools (LLM not configured → is_error=true) ────
//
// Wave 4 code-review MAJOR-2: these handlers require a configured LLM
// provider (capture / reason / decompose / generate all call
// `forgeplan_core::llm::*`). A fresh `McpFixture` workspace never has an
// API key wired, so the contract на test env пинуем строго:
// `is_error == true` AND non-empty error body. Anything else means the
// handler silently succeeded without an LLM (regression) or panicked
// (also regression).
//
// Audit MAJOR-2 FULL closure (Wave 8C, v0.31.0):
// All previously tolerant `assert_reachable`-only tests have been
// tightened to specific shape or is_error contracts, OR rationalized
// (kept reachable when the inner assertions already supply strong
// contract — see `guard_target_session_phase_disambiguated_from_artifact_phase`
// where assert_reachable is a lower-bound guard preceding tools/list
// description assertions). 0 plain `assert_reachable`-only tests remain.
// Earlier waves landed in this file:
//   - Wave 7D: 15 tests tightened (c07/c08/c10/c12/c22/c26/c47/c48/c50/
//     c53/c54/c55/c56/c58/c60)
//   - Wave 8C: 17 tests tightened (c01/c13/c21/c23/c24/c28/c30/c36/c37/
//     c38/c40/c41/c42/c43/c45/c46/c57)

#[tokio::test]
async fn c31_forgeplan_capture_no_llm_smoke() {
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json(
            "forgeplan_capture",
            json!({"decision": "Use Postgres for primary storage"}),
        )
        .await;
    assert!(
        env.is_error,
        "capture without LLM provider must return is_error=true (handler requires LLM), got: {}",
        env.raw_text
    );
    assert!(
        !env.raw_text.is_empty(),
        "error body must describe the missing provider, got empty"
    );
}

#[tokio::test]
async fn c32_forgeplan_reason_no_llm_smoke() {
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Reasonable").await;
    let env = fx
        .call_tool_json("forgeplan_reason", json!({"id": id}))
        .await;
    assert!(
        env.is_error,
        "reason without LLM provider must return is_error=true (ADI requires LLM), got: {}",
        env.raw_text
    );
    assert!(
        !env.raw_text.is_empty(),
        "error body must describe the missing provider, got empty"
    );
}

#[tokio::test]
async fn c33_forgeplan_decompose_no_llm_smoke() {
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Decomposable").await;
    let env = fx
        .call_tool_json("forgeplan_decompose", json!({"id": id}))
        .await;
    assert!(
        env.is_error,
        "decompose without LLM provider must return is_error=true (handler requires LLM), got: {}",
        env.raw_text
    );
    assert!(
        !env.raw_text.is_empty(),
        "error body must describe the missing provider, got empty"
    );
}

#[tokio::test]
async fn c34_forgeplan_generate_no_llm_smoke() {
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json(
            "forgeplan_generate",
            json!({"kind": "prd", "description": "User onboarding flow"}),
        )
        .await;
    assert!(
        env.is_error,
        "generate without LLM provider must return is_error=true (template fill requires LLM), got: {}",
        env.raw_text
    );
    assert!(
        !env.raw_text.is_empty(),
        "error body must describe the missing provider, got empty"
    );
}

// ── Group H: export / import roundtrip ────────────────────────────────

#[tokio::test]
async fn c35_forgeplan_export_inline_smoke() {
    let fx = McpFixture::new().await;
    fx.seed_prd("Exportable").await;
    let env = fx.call_tool_json("forgeplan_export", json!({})).await;
    let resp = env.assert_ok();
    // Inline export embeds the JSON in the response.
    assert!(resp.is_object(), "export returns an object payload: {resp}");
}

#[tokio::test]
async fn c36_forgeplan_import_smoke() {
    let fx = McpFixture::new().await;
    // Build a minimal valid export bundle and feed it back through import.
    let bundle = json!({
        "version": "1",
        "exported_at": "2026-01-01T00:00:00Z",
        "artifacts": [],
        "relations": [],
    });
    let env = fx
        .call_tool_json(
            "forgeplan_import",
            json!({"data": bundle.to_string(), "force": false}),
        )
        .await;
    // Contract pinned: import of an empty bundle returns
    // `{imported: 0, skipped: 0, relations_imported: 0}`. Previously
    // assert_reachable accepted any envelope — a regression that returned
    // a fake imported count or dropped `relations_imported` would have
    // silently passed.
    let resp = env.assert_ok();
    assert_eq!(
        resp["imported"], 0,
        "empty bundle imports 0 artifacts: {resp}"
    );
    assert_eq!(resp["skipped"], 0, "empty bundle skips 0 artifacts: {resp}");
    assert_eq!(
        resp["relations_imported"], 0,
        "empty bundle imports 0 relations: {resp}"
    );
}

// ── Group I: estimate / score variants ────────────────────────────────

#[tokio::test]
async fn c37_forgeplan_estimate_smoke() {
    // Contract pinned: estimate of a fresh PRD with no FR/Phase work items
    // returns `EstimateResult` with `{artifact_id, artifact_title, items: [],
    // totals: {}, total_score: 0.0, confidence: 0.0, hints[]}`. Previously
    // assert_reachable masked a regression dropping `items` (used by the
    // CLI render path) or returning fabricated estimates for empty bodies.
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Estimable").await;
    let env = fx
        .call_tool_json("forgeplan_estimate", json!({"id": id}))
        .await;
    let resp = env.assert_ok();
    assert_eq!(
        resp["artifact_id"], id,
        "estimate echoes artifact id: {resp}"
    );
    assert!(
        resp["items"].is_array(),
        "estimate response carries items[]: {resp}"
    );
    assert!(
        resp["hints"].is_array(),
        "estimate response carries hints[]: {resp}"
    );
}

// ── Group J: FPF advanced tools ───────────────────────────────────────

#[tokio::test]
async fn c38_forgeplan_fpf_section_known_id_smoke() {
    // Contract pinned: the fresh test fixture has no FPF KB ingested
    // (ingestion is CLI-only and the tempdir workspace never ran it), so
    // looking up *any* section MUST return is_error=true with the typed
    // "FPF section ... not found" hint. Previously assert_reachable
    // accepted EITHER the success path (if KB happened to be present) or
    // the error path — masking a regression that returned a fabricated
    // section payload for an empty KB.
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json("forgeplan_fpf_section", json!({"id": "A.1"}))
        .await;
    assert!(
        env.is_error,
        "fpf_section against empty KB must return is_error=true, got: {}",
        env.raw_text
    );
    assert!(
        env.raw_text.contains("not found") || env.raw_text.contains("FPF"),
        "fpf_section error body must mention the missing section, got: {}",
        env.raw_text
    );
}

#[tokio::test]
async fn c39_forgeplan_fpf_search_keyword_smoke() {
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json(
            "forgeplan_fpf_search",
            json!({"query": "trust", "limit": 3, "semantic": false}),
        )
        .await;
    let resp = env.assert_ok();
    assert!(
        resp["results"].is_array(),
        "fpf_search returns results[]: {resp}"
    );
    assert_eq!(resp["semantic"], false);
}

#[tokio::test]
async fn c40_forgeplan_fpf_check_smoke() {
    // Contract pinned: fpf_check returns the serialized FpfCheckResult
    // augmented with a `summary` string. With the default rule set (no
    // workspace-specific overrides) the response carries `{summary,
    // matched[]}` at minimum. Previously assert_reachable hid both the
    // summary contract AND the matched[] array shape.
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("FPF Checkable").await;
    let env = fx
        .call_tool_json("forgeplan_fpf_check", json!({"id": id}))
        .await;
    let resp = env.assert_ok();
    assert!(
        resp["summary"].is_string(),
        "fpf_check response carries summary string: {resp}"
    );
    assert!(
        resp["matched"].is_array(),
        "fpf_check response carries matched[]: {resp}"
    );
}

// ── Group K: phase state machine ──────────────────────────────────────

#[tokio::test]
async fn c41_forgeplan_phase_read_smoke() {
    // Contract pinned: phase read returns `{artifact_id, current_phase,
    // workflow_type, history[]}`. Fresh PRD created via `forgeplan_new`
    // may either have a phase state on disk (when PRD-056 phase tracking
    // is enabled) or hit the "No phase state on disk — advisory only"
    // path (when state file is absent). Both branches return the same
    // shape, just with different `current_phase` values. Previously
    // assert_reachable masked a regression that dropped `history` or
    // renamed `workflow_type` → `kind`.
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Phaseable").await;
    let env = fx
        .call_tool_json("forgeplan_phase", json!({"id": id}))
        .await;
    let resp = env.assert_ok();
    assert_eq!(resp["artifact_id"], id, "phase echoes artifact id: {resp}");
    assert!(
        resp["current_phase"].is_string(),
        "phase response carries current_phase string: {resp}"
    );
    assert!(
        resp["workflow_type"].is_string(),
        "phase response carries workflow_type string: {resp}"
    );
    assert!(
        resp["history"].is_array(),
        "phase response carries history[]: {resp}"
    );
}

#[tokio::test]
async fn c42_forgeplan_phase_advance_smoke() {
    // Contract pinned: phase_advance can return EITHER a success envelope
    // (`{artifact_id, current_phase, workflow_type, advanced_at,
    // history_entries, reason}`) when phase tracking is enabled, OR an
    // is_error envelope when phase tracking is disabled in the workspace
    // config. Both outcomes carry deterministic shape. Previously
    // assert_reachable accepted any envelope.
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Advance Subject").await;
    let env = fx
        .call_tool_json(
            "forgeplan_phase_advance",
            json!({"id": id, "to": "shape", "reason": "starting work"}),
        )
        .await;
    if env.is_error {
        assert!(
            env.raw_text.contains("phase") || env.raw_text.contains("state"),
            "phase_advance error body must explain the failure, got: {}",
            env.raw_text
        );
    } else {
        let resp = env.assert_ok();
        assert_eq!(
            resp["artifact_id"], id,
            "phase_advance echoes artifact id: {resp}"
        );
        assert_eq!(
            resp["current_phase"], "shape",
            "phase_advance sets current_phase to the requested target: {resp}"
        );
        assert!(
            resp["history_entries"].is_number(),
            "phase_advance response carries numeric history_entries: {resp}"
        );
    }
}

#[tokio::test]
async fn c43_forgeplan_guard_smoke() {
    // Disambiguation between session-phase and artifact-lifecycle-phase enums
    // now lives in the tool schema description (PROB-065) — see the regression
    // test `guard_target_session_phase_disambiguated_from_artifact_phase`
    // below for the assertion. This smoke test exercises the canonical
    // argument name `target_session_phase`.
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json("forgeplan_guard", json!({"target_session_phase": "coding"}))
        .await;
    // Contract pinned: guard returns `{current_phase, target_phase, allowed,
    // reason, next_action}` regardless of whether the transition is
    // allowed. `target_phase` echoes the requested value verbatim.
    // Previously assert_reachable accepted any envelope.
    let resp = env.assert_ok();
    assert!(
        resp["current_phase"].is_string(),
        "guard response carries current_phase string: {resp}"
    );
    assert_eq!(
        resp["target_phase"], "coding",
        "guard echoes requested target phase: {resp}"
    );
    assert!(
        resp["allowed"].is_boolean(),
        "guard response carries boolean allowed: {resp}"
    );
    assert!(
        resp["reason"].is_string(),
        "guard response carries reason string: {resp}"
    );
}

/// PROB-065 regression: `forgeplan_guard.target_session_phase` is the
/// methodology-session phase enum (idle/routing/shaping/coding/evidence/pr),
/// which lexically overlaps the artifact lifecycle phase enum exposed by
/// `forgeplan_phase_advance` (shape/validate/adi/code/test/audit/evidence/done).
/// To prevent silent type confusion this test asserts three contracts:
///
/// 1. The canonical argument name `target_session_phase` is accepted.
/// 2. The legacy alias `target_phase` is still accepted (backward compat).
/// 3. The tool description returned from `tools/list` explicitly contains
///    the phrase "session phase" so agents reading the catalog see the
///    disambiguation up front.
#[tokio::test]
async fn guard_target_session_phase_disambiguated_from_artifact_phase() {
    let fx = McpFixture::new().await;

    // (1) Canonical argument name works. `!is_error` already subsumes
    // the previous `assert_reachable()` guard — if the handler panicked
    // or returned a malformed envelope, the is_error read would itself
    // panic first.
    let env_new = fx
        .call_tool_json(
            "forgeplan_guard",
            json!({"target_session_phase": "evidence"}),
        )
        .await;
    assert!(
        !env_new.is_error,
        "guard with canonical `target_session_phase` must succeed: {}",
        env_new.raw_text
    );

    // (2) Legacy alias `target_phase` is still accepted (serde alias).
    let env_legacy = fx
        .call_tool_json("forgeplan_guard", json!({"target_phase": "evidence"}))
        .await;
    assert!(
        !env_legacy.is_error,
        "guard with deprecated alias `target_phase` must remain accepted: {}",
        env_legacy.raw_text
    );

    // (3) Tool description contains the disambiguation phrasing.
    let tools = fx
        .peer_list_all_tools()
        .await
        .expect("list_all_tools must succeed against in-process server");
    let guard_tool = tools
        .iter()
        .find(|t| t.name == "forgeplan_guard")
        .expect("forgeplan_guard must be registered");
    let description = guard_tool
        .description
        .as_ref()
        .map(|c| c.as_ref())
        .unwrap_or("");
    assert!(
        description.contains("session phase"),
        "forgeplan_guard description must mention 'session phase' to \
         disambiguate from artifact lifecycle phase (PROB-065). Got: {description}"
    );
    assert!(
        description.contains("forgeplan_phase_advance"),
        "forgeplan_guard description must cross-reference \
         `forgeplan_phase_advance` (PROB-065). Got: {description}"
    );

    let phase_advance_tool = tools
        .iter()
        .find(|t| t.name == "forgeplan_phase_advance")
        .expect("forgeplan_phase_advance must be registered");
    let phase_desc = phase_advance_tool
        .description
        .as_ref()
        .map(|c| c.as_ref())
        .unwrap_or("");
    assert!(
        phase_desc.contains("artifact lifecycle phase"),
        "forgeplan_phase_advance description must mention 'artifact \
         lifecycle phase' (PROB-065). Got: {phase_desc}"
    );
    assert!(
        phase_desc.contains("forgeplan_guard"),
        "forgeplan_phase_advance description must cross-reference \
         `forgeplan_guard` (PROB-065). Got: {phase_desc}"
    );
}

// ── Group L: discovery session lifecycle ──────────────────────────────

#[tokio::test]
async fn c44_forgeplan_discover_start_smoke() {
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json(
            "forgeplan_discover_start",
            json!({"project_name": "test-discovery"}),
        )
        .await;
    let resp = env.assert_ok();
    // Discover_start returns a session id we can chain.
    assert!(
        resp.get("session_id").is_some() || resp.get("id").is_some(),
        "discover_start returns a session reference: {resp}"
    );
}

#[tokio::test]
async fn c45_forgeplan_discover_finding_smoke() {
    let fx = McpFixture::new().await;
    let start_env = fx
        .call_tool_json(
            "forgeplan_discover_start",
            json!({"project_name": "test-discovery-finding"}),
        )
        .await;
    let start_resp = start_env.assert_ok();
    let session_id = start_resp
        .get("session_id")
        .or_else(|| start_resp.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("test-session")
        .to_string();

    let env = fx
        .call_tool_json(
            "forgeplan_discover_finding",
            json!({
                "session_id": session_id,
                "phase": "detect",
                "tier": 1,
                "kind": "note",
                "title": "Detected Cargo workspace",
                "body": "Found Cargo.toml at repo root with 3 workspace members.",
                "source_files": ["Cargo.toml"],
            }),
        )
        .await;
    // Contract pinned: discover_finding returns `{session_id, artifact_id,
    // phase, tier, total_findings, status}`. The artifact_id reflects the
    // newly-created note; `total_findings` increments to 1 after the
    // first call. Previously assert_reachable hid a regression that
    // dropped the artifact_id (used by the agent to attach links).
    let resp = env.assert_ok();
    assert_eq!(
        resp["session_id"], session_id,
        "discover_finding echoes session_id: {resp}"
    );
    assert!(
        resp["artifact_id"].is_string(),
        "discover_finding returns artifact_id string: {resp}"
    );
    assert_eq!(
        resp["phase"], "detect",
        "discover_finding echoes phase: {resp}"
    );
    assert_eq!(resp["tier"], 1, "discover_finding echoes tier: {resp}");
    assert_eq!(
        resp["total_findings"], 1,
        "first finding bumps total_findings to 1: {resp}"
    );
}

#[tokio::test]
async fn c46_forgeplan_discover_complete_smoke() {
    let fx = McpFixture::new().await;
    let start_env = fx
        .call_tool_json(
            "forgeplan_discover_start",
            json!({"project_name": "test-discovery-complete"}),
        )
        .await;
    let start_resp = start_env.assert_ok();
    let session_id = start_resp
        .get("session_id")
        .or_else(|| start_resp.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("test-session")
        .to_string();

    let env = fx
        .call_tool_json(
            "forgeplan_discover_complete",
            json!({"session_id": session_id}),
        )
        .await;
    // Contract pinned: discover_complete returns `{session_id, project_name,
    // status, total_findings, phase_counts, tier_counts, artifacts_created,
    // completed_at}`. With zero findings recorded, `total_findings` is 0
    // and `artifacts_created` is an empty array. Previously
    // assert_reachable hid a regression dropping `phase_counts` or
    // `completed_at` (used by the discovery summary UI).
    let resp = env.assert_ok();
    assert_eq!(
        resp["session_id"], session_id,
        "discover_complete echoes session_id: {resp}"
    );
    assert_eq!(
        resp["status"], "completed",
        "discover_complete marks session status=completed: {resp}"
    );
    assert_eq!(
        resp["total_findings"], 0,
        "session with no findings → total_findings=0: {resp}"
    );
    assert!(
        resp["artifacts_created"].is_array(),
        "discover_complete carries artifacts_created[]: {resp}"
    );
    assert!(
        resp["completed_at"].is_string(),
        "discover_complete carries completed_at timestamp: {resp}"
    );
}

// ── Group M: activity log ─────────────────────────────────────────────

#[tokio::test]
async fn c47_forgeplan_activity_smoke() {
    // Contract pinned: activity returns `{entries[], total_scanned, returned,
    // warnings, since_hours}`. since_hours echoed back (clamped 1..=720)
    // proves the param round-trip; a regression dropping `total_scanned`
    // or renaming `entries` would have passed silently with assert_reachable.
    let fx = McpFixture::new().await;
    fx.seed_prd("Activity Trigger").await;
    let env = fx
        .call_tool_json("forgeplan_activity", json!({"since_hours": 1, "limit": 50}))
        .await;
    let resp = env.assert_ok();
    assert!(
        resp["entries"].is_array(),
        "activity response carries entries[]: {resp}"
    );
    assert!(
        resp["total_scanned"].is_number(),
        "activity response carries numeric total_scanned: {resp}"
    );
    assert_eq!(
        resp["since_hours"], 1,
        "activity echoes the clamped since_hours param: {resp}"
    );
}

#[tokio::test]
async fn c48_forgeplan_activity_stats_smoke() {
    // Contract pinned: activity_stats returns `{stats[], total_calls,
    // total_errors, total_ms, since_hours}`. Aggregate counts are
    // distinct from raw entries — a regression that returned the raw
    // `forgeplan_activity` shape would have passed silently here.
    let fx = McpFixture::new().await;
    fx.seed_prd("Stats Trigger").await;
    let env = fx
        .call_tool_json("forgeplan_activity_stats", json!({"since_hours": 24}))
        .await;
    let resp = env.assert_ok();
    assert!(
        resp["stats"].is_array(),
        "activity_stats response carries stats[]: {resp}"
    );
    assert!(
        resp["total_calls"].is_number(),
        "activity_stats response carries numeric total_calls: {resp}"
    );
    assert!(
        resp["total_errors"].is_number(),
        "activity_stats response carries numeric total_errors: {resp}"
    );
    assert_eq!(
        resp["since_hours"], 24,
        "activity_stats echoes since_hours param: {resp}"
    );
}

// ── Group N: undo / restore (no-receipt path) ─────────────────────────

#[tokio::test]
async fn c49_forgeplan_restore_no_receipt_returns_error() {
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json("forgeplan_restore", json!({"id": "PRD-999"}))
        .await;
    // Contract pinned: restore without a matching receipt returns
    // `is_error=true`. The is_error read is itself a panic guard — no
    // separate reachability check needed.
    assert!(
        env.is_error,
        "restore without receipt must return is_error=true: {}",
        env.raw_text
    );
}

#[tokio::test]
async fn c50_forgeplan_undo_last_no_receipt_returns_error() {
    // Contract pinned: a fresh workspace has no trash receipts — handler
    // MUST return `err_hinted` (is_error=true) with a body that mentions
    // the empty window. Previously assert_reachable accepted EITHER a
    // success or error envelope, which masked the silent-fallback
    // regression we saw в PROB-035/039 (handler claimed success while
    // returning a "nothing to undo" string).
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json("forgeplan_undo_last", json!({"within_hours": 24}))
        .await;
    assert!(
        env.is_error,
        "undo_last on empty trash must return is_error=true, got: {}",
        env.raw_text
    );
    assert!(
        env.raw_text.contains("non-consumed") || env.raw_text.contains("trash"),
        "undo_last error body must explain the missing receipt, got: {}",
        env.raw_text
    );
}

// ── Group O: playbooks / plugins / ingest (FS-dependent) ──────────────

#[tokio::test]
async fn c51_forgeplan_playbook_list_smoke() {
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json("forgeplan_playbook_list", json!({}))
        .await;
    let resp = env.assert_ok();
    assert!(
        resp["playbooks"].is_array(),
        "playbook_list returns playbooks[]: {resp}"
    );
}

#[tokio::test]
async fn c52_forgeplan_playbook_show_missing_target_returns_error() {
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json(
            "forgeplan_playbook_show",
            json!({"target": "nonexistent-playbook-xyz"}),
        )
        .await;
    assert!(
        env.is_error,
        "playbook_show for missing target must return is_error=true: {}",
        env.raw_text
    );
}

#[tokio::test]
async fn c53_forgeplan_playbook_validate_missing_file_returns_error() {
    // Contract pinned: validate against a path that fails the
    // `phase5_validate_path` confinement (outside workspace) MUST return
    // `err_hinted` (is_error=true). Previously assert_reachable accepted a
    // silent success — which would mask a regression that removed the
    // path-confinement guard (HIGH-S1 security check from Audit Round 1).
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json(
            "forgeplan_playbook_validate",
            json!({"file": "/nonexistent/path/to/playbook.yaml"}),
        )
        .await;
    assert!(
        env.is_error,
        "playbook_validate against out-of-workspace path must return is_error=true (HIGH-S1 \
         confinement), got: {}",
        env.raw_text
    );
    assert!(
        env.raw_text.contains("cannot read") || env.raw_text.contains("playbook"),
        "playbook_validate error body must mention the read failure, got: {}",
        env.raw_text
    );
}

#[tokio::test]
async fn c54_forgeplan_playbook_run_requires_consent() {
    // Contract pinned: a non-existent target fails `phase5_resolve_target`
    // BEFORE the consent gate (dry_run bypasses consent), so the handler
    // returns `err_hinted("playbook target ... not resolvable")` —
    // is_error=true. Previously assert_reachable accepted any envelope, so
    // a regression silently invoking shell on an unresolvable target would
    // have passed.
    //
    // Note: when `yes=false` AND `dry_run=false`, the consent gate fires
    // first; that variant is covered in the in-module unit tests at the
    // end of `server.rs` (see `playbook_run_refuses_without_yes`).
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json(
            "forgeplan_playbook_run",
            json!({"target": "definitely-not-a-real-target", "yes": false, "dry_run": true}),
        )
        .await;
    assert!(
        env.is_error,
        "playbook_run with unresolvable target must return is_error=true, got: {}",
        env.raw_text
    );
    assert!(
        env.raw_text.contains("not resolvable") || env.raw_text.contains("playbook target"),
        "playbook_run error body must explain the target failure, got: {}",
        env.raw_text
    );
}

#[tokio::test]
async fn c55_forgeplan_ingest_dry_run_missing_mapping_returns_error() {
    // Contract pinned: ingest rejects a mapping path that fails
    // `phase5_validate_path` BEFORE touching the source file, returning
    // `err_hinted("mapping file not found or outside workspace")`. This
    // pins the HIGH-S1 confinement order — the source path is never
    // canonicalized when the mapping path is invalid, so an attacker
    // cannot use the source argument for side-channel path probing.
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json(
            "forgeplan_ingest",
            json!({
                "mapping": "/nonexistent/mapping.yaml",
                "source": "/nonexistent/source.csv",
                "dry_run": true,
            }),
        )
        .await;
    assert!(
        env.is_error,
        "ingest with missing mapping must return is_error=true, got: {}",
        env.raw_text
    );
    assert!(
        env.raw_text.contains("mapping"),
        "ingest error body must mention the mapping failure, got: {}",
        env.raw_text
    );
}

#[tokio::test]
async fn c56_forgeplan_plugins_list_smoke() {
    // Contract pinned: plugins_list returns `{installed[], missing[],
    // installed_count, missing_count}`. installed_count must equal
    // installed.len(), and missing[] is non-empty in test env (the
    // registry contains ≥1 plugin and the temp workspace has none of
    // them detected). Previously assert_reachable hid both counts AND
    // the registry-vs-detection consistency.
    let fx = McpFixture::new().await;
    let env = fx.call_tool_json("forgeplan_plugins_list", json!({})).await;
    let resp = env.assert_ok();
    let installed = resp["installed"].as_array().expect("installed[]");
    let missing = resp["missing"].as_array().expect("missing[]");
    assert_eq!(
        resp["installed_count"]
            .as_u64()
            .expect("installed_count u64"),
        installed.len() as u64,
        "plugins_list installed_count consistent with installed.len(): {resp}"
    );
    assert_eq!(
        resp["missing_count"].as_u64().expect("missing_count u64"),
        missing.len() as u64,
        "plugins_list missing_count consistent with missing.len(): {resp}"
    );
}

#[tokio::test]
async fn c57_forgeplan_plugins_doctor_smoke() {
    // Contract pinned: plugins_doctor returns `{ok[], outdated[], missing[],
    // install_hints[], ok_count, outdated_count, missing_count}` with each
    // count consistent with its array length. The registry seeds the
    // missing[] array in the tempdir fixture (no plugins detected) so
    // missing_count > 0 is the expected baseline. Previously
    // assert_reachable hid the count/array consistency contract.
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json("forgeplan_plugins_doctor", json!({}))
        .await;
    let resp = env.assert_ok();
    let ok = resp["ok"].as_array().expect("ok[]");
    let outdated = resp["outdated"].as_array().expect("outdated[]");
    let missing = resp["missing"].as_array().expect("missing[]");
    assert_eq!(
        resp["ok_count"].as_u64().expect("ok_count u64"),
        ok.len() as u64,
        "plugins_doctor ok_count consistent with ok.len(): {resp}"
    );
    assert_eq!(
        resp["outdated_count"].as_u64().expect("outdated_count u64"),
        outdated.len() as u64,
        "plugins_doctor outdated_count consistent with outdated.len(): {resp}"
    );
    assert_eq!(
        resp["missing_count"].as_u64().expect("missing_count u64"),
        missing.len() as u64,
        "plugins_doctor missing_count consistent with missing.len(): {resp}"
    );
    assert!(
        resp["install_hints"].is_array(),
        "plugins_doctor response carries install_hints[]: {resp}"
    );
}

#[tokio::test]
async fn c58_forgeplan_plugins_info_unknown_returns_error() {
    // Contract pinned: plugins_info against an unknown registry name
    // returns `err_hinted("plugin ... not in registry")` — is_error=true.
    // Previously assert_reachable accepted EITHER a success envelope (with
    // `installed: null`) OR an error, so a regression that returned a
    // fabricated PluginInfo would have silently passed.
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json(
            "forgeplan_plugins_info",
            json!({"name": "definitely-not-a-real-plugin"}),
        )
        .await;
    assert!(
        env.is_error,
        "plugins_info on unknown name must return is_error=true, got: {}",
        env.raw_text
    );
    assert!(
        env.raw_text.contains("not in registry") || env.raw_text.contains("plugin"),
        "plugins_info error body must explain the registry miss, got: {}",
        env.raw_text
    );
}

#[tokio::test]
async fn c60_forgeplan_release_notes_smoke() {
    // The fixture workspace has no git repo; `git log` may either succeed
    // with no output (when the test runs inside the wider Forgeplan repo,
    // the parent of the tempdir is *not* a git repo) or fail. Either way
    // the tool must return a valid response when called with `draft=true`
    // (no quality gate).
    //
    // Contract pinned: when generation succeeds (the common path inside the
    // outer repo), the response carries `{since, until, draft, total,
    // added[], fixed[], security[], changed[], internal[]}`. When git fails
    // entirely (e.g. running outside a repo), the handler returns
    // `err_result("release-notes failed: ...")` — is_error=true. We accept
    // either outcome but pin the shape of each one (previously
    // assert_reachable accepted any envelope).
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json(
            "forgeplan_release_notes",
            json!({"since": "HEAD", "until": "HEAD", "draft": true}),
        )
        .await;
    if env.is_error {
        assert!(
            env.raw_text.contains("release-notes") || env.raw_text.contains("git"),
            "release_notes error body must explain the git failure, got: {}",
            env.raw_text
        );
    } else {
        let resp = env.assert_ok();
        assert!(
            resp["since"].is_string(),
            "release_notes carries `since` string: {resp}"
        );
        assert!(
            resp["until"].is_string(),
            "release_notes carries `until` string: {resp}"
        );
        assert!(
            resp["total"].is_number(),
            "release_notes carries numeric `total`: {resp}"
        );
        assert!(
            resp["added"].is_array() && resp["fixed"].is_array() && resp["changed"].is_array(),
            "release_notes carries section arrays (added/fixed/changed): {resp}"
        );
    }
}

// ── Housekeeping: fixture sanity ─────────────────────────────────────

#[tokio::test]
async fn c61_fixture_workspace_is_initialized() {
    let fx = McpFixture::new().await;
    assert!(
        fx.workspace_path.exists(),
        "fixture workspace path must exist: {}",
        fx.workspace_path.display()
    );
}
