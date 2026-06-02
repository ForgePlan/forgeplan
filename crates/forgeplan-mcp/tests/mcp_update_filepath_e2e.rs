//! Issue #350 (CRITICAL silent data-loss) regression — `forgeplan_update` and
//! `forgeplan_discover_finding` MCP tools must expand a `@/path/to/file.md`
//! body into the file's content (CLI parity), NOT write the literal `@path`
//! string into the artifact body.
//!
//! Pre-fix, an agent mirroring the documented CLI `--body @file` pattern through
//! MCP got `Updated successfully` while the artifact body was silently
//! overwritten with the literal string `@/path/...`. The failure was invisible
//! until a later `forgeplan_get`. These tests pin the fix: the file content
//! lands in the body; a missing file errors loudly (not silently); a plain body
//! without a leading `@` is stored verbatim.

mod common;
use common::McpFixture;

fn assert_err_contains(err: rmcp::service::ServiceError, needle: &str) {
    let msg = format!("{err}");
    assert!(
        msg.contains(needle),
        "expected error message to contain {needle:?}, got: {msg}"
    );
}

/// THE #350 regression: `forgeplan_update` with `body=@file` stores the file's
/// CONTENT, not the literal `@path` string.
#[tokio::test]
async fn mcp_update_at_filepath_expands_into_body() {
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("issue-350 sandbox").await;

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("body.md");
    let content = "# Real body content\n\nThis text came from a FILE, not the path string.";
    std::fs::write(&file, content).unwrap();

    let env = fx
        .call_tool_json(
            "forgeplan_update",
            serde_json::json!({ "id": id, "body": format!("@{}", file.display()) }),
        )
        .await;
    env.assert_ok();

    // Read back — body MUST be the file content, never the literal "@path".
    let get = fx
        .call_tool_json("forgeplan_get", serde_json::json!({ "id": id }))
        .await;
    let body = get.assert_ok()["body"].as_str().unwrap_or("").to_string();
    assert!(
        body.contains("Real body content"),
        "body must be the FILE content (issue #350), got: {body:?}"
    );
    assert!(
        !body.trim_start().starts_with('@'),
        "body must NOT be the literal @path string (issue #350 regression): {body:?}"
    );
}

/// A `@file` whose content carries YAML frontmatter must have the frontmatter
/// stripped (CLI parity — we persist body only).
#[tokio::test]
async fn mcp_update_at_filepath_strips_frontmatter() {
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("issue-350 frontmatter").await;

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("with_fm.md");
    std::fs::write(
        &file,
        "---\nid: PRD-999\nkind: prd\n---\n\n## Problem\nBody after frontmatter.",
    )
    .unwrap();

    let env = fx
        .call_tool_json(
            "forgeplan_update",
            serde_json::json!({ "id": id, "body": format!("@{}", file.display()) }),
        )
        .await;
    env.assert_ok();

    let get = fx
        .call_tool_json("forgeplan_get", serde_json::json!({ "id": id }))
        .await;
    let body = get.assert_ok()["body"].as_str().unwrap_or("").to_string();
    assert!(
        body.contains("Body after frontmatter"),
        "expected body content, got: {body:?}"
    );
    assert!(
        !body.contains("kind: prd"),
        "frontmatter must be stripped from a @file body (issue #350), got: {body:?}"
    );
}

/// A missing `@file` must error LOUDLY (invalid_params), never silently write
/// the literal `@path` — the whole point of #350 is no silent data loss.
#[tokio::test]
async fn mcp_update_at_nonexistent_file_errors_not_silent() {
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("issue-350 negative").await;

    let err = fx
        .try_call_tool(
            "forgeplan_update",
            serde_json::json!({ "id": id, "body": "@/nonexistent/issue-350/missing.md" }),
        )
        .await
        .expect_err("a missing @file must error, not silently write the @path string");
    assert_err_contains(err, "cannot read file");
}

/// A plain body with no leading `@` is stored verbatim (no accidental file read,
/// no `@` interpretation mid-string).
#[tokio::test]
async fn mcp_update_literal_body_without_at_is_verbatim() {
    let fx = McpFixture::new().await;
    let id = fx.seed_prd("issue-350 literal").await;

    let env = fx
        .call_tool_json(
            "forgeplan_update",
            serde_json::json!({ "id": id, "body": "Plain body. Email a@b.com stays inline." }),
        )
        .await;
    env.assert_ok();

    let get = fx
        .call_tool_json("forgeplan_get", serde_json::json!({ "id": id }))
        .await;
    let body = get.assert_ok()["body"].as_str().unwrap_or("").to_string();
    assert!(
        body.contains("Plain body. Email a@b.com stays inline."),
        "a non-@ body must be stored verbatim, got: {body:?}"
    );
}
