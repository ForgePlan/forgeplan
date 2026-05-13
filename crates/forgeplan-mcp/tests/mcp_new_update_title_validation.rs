//! Wave 9 SEC-C2 closure — `forgeplan_new` AND `forgeplan_update` MCP
//! tools route incoming titles through
//! `forgeplan_core::artifact::validate_title` BEFORE any LanceDB write.
//!
//! Pre-fix only `forgeplan new` (CLI) ran this validator. The MCP
//! surfaces had only a coarse byte-length cap (`mcp_max_title_len`,
//! default 200) that did not reject control chars, bidi overrides, or
//! ANSI escape sequences. A malicious agent could plant
//! `prd-\u{202E}<reversed payload>` and corrupt rendered `Next:`
//! hint suggestions that downstream LLM agents would consume verbatim
//! (Trojan Source — CWE-1007).
//!
//! These tests pin the wiring: each invalid-title case MUST surface
//! as `McpError::invalid_params` (JSON-RPC error -32602), NOT as a
//! successful response with an `is_error: true` body. invalid_params
//! is the contract because the failure mode is a malformed argument,
//! not an internal server error — agents can recover by re-issuing
//! with a valid title.

mod common;
use common::McpFixture;

/// Helper: assert that the rmcp `ServiceError` rendering contains the
/// expected message substring. The exact ServiceError variant depends
/// on the rmcp version (`McpError`, `Service(McpError)`, etc.) — we
/// rely on Display to surface the inner `invalid_params` message text.
fn assert_err_contains(err: rmcp::service::ServiceError, needle: &str) {
    let msg = format!("{err}");
    assert!(
        msg.contains(needle),
        "expected error message to contain {needle:?}, got: {msg}"
    );
}

// ── forgeplan_new ─────────────────────────────────────────────

#[tokio::test]
async fn mcp_new_rejects_bidi_override_title() {
    let fx = McpFixture::new().await;
    let err = fx
        .try_call_tool(
            "forgeplan_new",
            serde_json::json!({"kind": "prd", "title": "rtl\u{202E}payload"}),
        )
        .await
        .expect_err("expected invalid_params for bidi-override title");
    assert_err_contains(err, "BIDI override");
}

#[tokio::test]
async fn mcp_new_rejects_ansi_escape_title() {
    let fx = McpFixture::new().await;
    let err = fx
        .try_call_tool(
            "forgeplan_new",
            serde_json::json!({"kind": "prd", "title": "\u{001B}[2Jpwn"}),
        )
        .await
        .expect_err("expected invalid_params for ANSI-escape title");
    assert_err_contains(err, "control character");
}

#[tokio::test]
async fn mcp_new_rejects_newline_in_title() {
    let fx = McpFixture::new().await;
    let err = fx
        .try_call_tool(
            "forgeplan_new",
            serde_json::json!({"kind": "prd", "title": "first\nsecond"}),
        )
        .await
        .expect_err("expected invalid_params for newline-in-title");
    assert_err_contains(err, "control character");
}

#[tokio::test]
async fn mcp_new_rejects_empty_title() {
    let fx = McpFixture::new().await;
    let err = fx
        .try_call_tool(
            "forgeplan_new",
            serde_json::json!({"kind": "prd", "title": ""}),
        )
        .await
        .expect_err("expected invalid_params for empty title");
    assert_err_contains(err, "Title cannot be empty");
}

#[tokio::test]
async fn mcp_new_accepts_benign_title() {
    // Regression guard: validator gate must not block normal traffic.
    let fx = McpFixture::new().await;
    let env = fx
        .call_tool_json(
            "forgeplan_new",
            serde_json::json!({"kind": "prd", "title": "Healthy MCP-created PRD"}),
        )
        .await;
    let resp = env.assert_ok();
    let title = resp["title"].as_str().expect("title in response");
    assert_eq!(title, "Healthy MCP-created PRD");
}

// ── forgeplan_update ──────────────────────────────────────────

async fn seed_and_update_with_bad_title(payload: &str) -> rmcp::service::ServiceError {
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Initial title").await;
    fx.try_call_tool(
        "forgeplan_update",
        serde_json::json!({"id": id, "title": payload}),
    )
    .await
    .expect_err("expected invalid_params for adversarial title in update")
}

#[tokio::test]
async fn mcp_update_rejects_bidi_override_title() {
    let err = seed_and_update_with_bad_title("rename\u{202E}reverse").await;
    assert_err_contains(err, "BIDI override");
}

#[tokio::test]
async fn mcp_update_rejects_ansi_escape_title() {
    let err = seed_and_update_with_bad_title("\u{001B}[31mred\u{001B}[0m").await;
    assert_err_contains(err, "control character");
}

#[tokio::test]
async fn mcp_update_rejects_bel_control_char() {
    let err = seed_and_update_with_bad_title("\u{0007}alert").await;
    assert_err_contains(err, "control character");
}

#[tokio::test]
async fn mcp_update_accepts_benign_rename() {
    // Regression: a normal rename through MCP must still succeed.
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("Original title").await;
    let env = fx
        .call_tool_json(
            "forgeplan_update",
            serde_json::json!({"id": id, "title": "Renamed via MCP safely"}),
        )
        .await;
    let resp = env.assert_ok();
    assert_eq!(
        resp["title"].as_str().unwrap(),
        "Renamed via MCP safely",
        "rename response title must reflect the new value"
    );
}
