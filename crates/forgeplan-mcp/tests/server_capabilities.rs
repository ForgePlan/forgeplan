//! Regression test for PROB-039: ServerCapabilities must advertise `tools`.
//!
//! Without `tools` capability declared, MCP clients (Claude Code, Cursor,
//! Windsurf) never call `tools/list` after initialize, so registered tools
//! are invisible to agents — a silent failure. This bit v0.19.0 in prod.
//!
//! This test calls `get_info()` directly on `ForgeplanServer` and asserts
//! that `capabilities.tools` is present. It does NOT require a tokio runtime
//! or spawned process — catches the regression at Rust level before any
//! JSON-RPC plumbing is involved.

use forgeplan_mcp::ForgeplanServer;
use rmcp::ServerHandler;

#[tokio::test]
async fn server_advertises_tools_capability() {
    // Use tempdir so this test doesn't collide with real workspace.
    let dir = tempfile::tempdir().unwrap();
    let server = ForgeplanServer::new(dir.path().to_path_buf()).await;

    let info = server.get_info();

    assert!(
        info.capabilities.tools.is_some(),
        "SERVER MUST DECLARE `tools` capability — without it, MCP clients \
         silently skip tool registration. Regression of v0.19.0 bug (PROB-039). \
         Fix: ServerCapabilities::builder().enable_tools().build() in server.rs."
    );

    // Wire-format check — catches `skip_serializing_if` quirks that
    // could drop the field at JSON serialization even when the Rust
    // struct has Some(..). The MCP client reads the serialized
    // `initialize` response, not the Rust struct directly.
    let caps_json = serde_json::to_value(&info.capabilities).expect("serialize");
    assert!(
        caps_json.get("tools").is_some(),
        "`capabilities.tools` must appear in serialized initialize response \
         — the actual wire format MCP clients parse. Got: {}",
        caps_json
    );
}

#[tokio::test]
async fn server_info_has_name_and_version() {
    let dir = tempfile::tempdir().unwrap();
    let server = ForgeplanServer::new(dir.path().to_path_buf()).await;

    let info = server.get_info();

    assert_eq!(info.server_info.name, "forgeplan");
    assert!(
        !info.server_info.version.is_empty(),
        "version must come from CARGO_PKG_VERSION"
    );
}

#[tokio::test]
async fn server_info_has_instructions() {
    let dir = tempfile::tempdir().unwrap();
    let server = ForgeplanServer::new(dir.path().to_path_buf()).await;

    let info = server.get_info();
    let instructions = info
        .instructions
        .expect("instructions help agents understand server purpose");

    assert!(
        instructions.contains("Forgeplan"),
        "instructions should mention server name"
    );
    assert!(
        instructions.contains("_next_action") || instructions.contains("workflow"),
        "instructions should mention workflow chaining hint"
    );
}
