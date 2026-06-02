//! AC-3 integration test: multi-worktree detection gate in `resolve_workspace`.
//!
//! PRD-078 / ADR-015 Phase 2 (W2) acceptance criterion:
//! > AC-3: When the MCP server starts in a linked git worktree environment
//! > without an explicit `workspace` param or `FORGEPLAN_WORKSPACE` env var,
//! > the tool call returns `-32602` (`invalid_params`) with:
//! >   - message starting with `"multi-worktree"`
//! >   - message containing `workspace="<path>"`
//! >   - `data.code == "multi_worktree_workspace_required"`
//! >   - `data.detection == "git-common-dir-ne-show-toplevel"`
//! >   - `data.suggested_workspace` set to a non-empty path string
//!
//! The test creates an actual git linked worktree in a tempdir and boots a
//! cold `ForgeplanServer` (no `workspace_path` set, no env var) rooted there.
//! A `forgeplan_new` call without `workspace` param must trigger the gate.
//!
//! **Why integration rather than unit:**  The gate lives inside
//! `resolve_workspace`, which is `pub(crate)`.  The unit tests in `server.rs`
//! already cover the resolution chain; this file provides the evidence that the
//! gate fires over the real JSON-RPC transport when the process CWD is a
//! linked worktree — the exact scenario PROB-072 describes.
//!
//! **Worktree test skip policy:** Some CI environments block `git worktree add`
//! (e.g. Github Actions with restricted tmpfs). The test skips gracefully when
//! `git worktree add` exits non-zero, printing a diagnostic. This mirrors the
//! policy used in `workspace::init::detect_tests::detect_returns_true_with_linked_worktree`.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use forgeplan_mcp::ForgeplanServer;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Map, Value, json};
use tempfile::TempDir;

/// Boot a cold `ForgeplanServer` (workspace_path = None) from `root` and
/// pair it with an in-process JSON-RPC client via `tokio::io::duplex`.
/// Returns `(client, _server_task, _tempdir)` — the tempdir keeps all
/// filesystem state alive for the duration of the test.
async fn cold_server_from(
    root: PathBuf,
) -> (
    rmcp::service::RunningService<rmcp::RoleClient, ()>,
    tokio::task::JoinHandle<()>,
    PathBuf,
) {
    // Cold server: explicitly do NOT set workspace_path or store.
    // ForgeplanServer::new walks up from `root` looking for `.forgeplan/`
    // but we intentionally do NOT init a workspace — we want the cold-start
    // path to hit the worktree detection gate.
    let server = ForgeplanServer::new(root.clone()).await;

    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let server_task = tokio::spawn(async move {
        if let Ok(running) = server.serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    let client = ().serve(client_io).await.expect("client initialize handshake");
    (client, server_task, root)
}

fn init_git_repo(dir: &std::path::Path) -> bool {
    let ok = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        return false;
    }
    Command::new("git")
        .args(["commit", "--allow-empty", "-m", "root"])
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "t@t.t")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "t@t.t")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// AC-3: forgeplan_new call from a cold server rooted in a linked worktree,
/// without workspace param, must return -32602 with the §2.3 error shape.
#[tokio::test]
async fn ac3_multi_worktree_no_workspace_returns_32602() {
    // ── Set up: main repo + linked worktree ──────────────────────────────
    let main_tmp = TempDir::new().unwrap();
    if !init_git_repo(main_tmp.path()) {
        eprintln!("SKIP ac3: git init failed — likely CI environment without git");
        return;
    }

    let linked_tmp = TempDir::new().unwrap();
    let wt_output = Command::new("git")
        .args([
            "-C",
            &main_tmp.path().to_string_lossy(),
            "worktree",
            "add",
            &linked_tmp.path().to_string_lossy(),
            "-b",
            "wt-ac3",
        ])
        .output()
        .expect("spawn git worktree add");

    if !wt_output.status.success() {
        eprintln!(
            "SKIP ac3: git worktree add failed ({}): {}",
            wt_output.status,
            String::from_utf8_lossy(&wt_output.stderr)
        );
        return;
    }

    // ── Boot cold server from the linked worktree dir ────────────────────
    let linked_path = linked_tmp.path().to_path_buf();
    let (client, _srv, _root) = cold_server_from(linked_path).await;

    // ── Call forgeplan_new without workspace param ────────────────────────
    // We expect this to return McpError::invalid_params (-32602).
    let mut args: Map<String, Value> = Map::new();
    args.insert("kind".to_string(), json!("prd"));
    args.insert("title".to_string(), json!("AC3 Test PRD"));
    // Explicitly omit "workspace" param.

    let params = CallToolRequestParams::new("forgeplan_new").with_arguments(args);
    let result = tokio::time::timeout(Duration::from_secs(15), client.peer().call_tool(params))
        .await
        .expect("call_tool did not time out");

    // ── Assert: must be a JSON-RPC error (-32602) ────────────────────────
    match result {
        Err(rmcp::service::ServiceError::McpError(err)) => {
            // Error code must be -32602 (invalid_params).
            assert_eq!(
                err.code.0, -32602,
                "expected -32602 (invalid_params), got {}",
                err.code.0
            );

            // Message must start with "multi-worktree" and contain workspace=.
            let msg = &err.message;
            assert!(
                msg.starts_with("multi-worktree"),
                "message must start with 'multi-worktree', got: {msg}"
            );
            assert!(
                msg.contains("workspace="),
                "message must contain 'workspace=', got: {msg}"
            );

            // data.code must equal "multi_worktree_workspace_required".
            if let Some(data) = &err.data {
                let code = data["code"].as_str().unwrap_or("");
                assert_eq!(
                    code, "multi_worktree_workspace_required",
                    "data.code mismatch: {data}"
                );
                let detection = data["detection"].as_str().unwrap_or("");
                assert_eq!(
                    detection, "git-common-dir-ne-show-toplevel",
                    "data.detection mismatch: {data}"
                );
                let resolved_via = data["resolved_via"].as_str().unwrap_or("");
                assert_eq!(
                    resolved_via, "none",
                    "data.resolved_via must be 'none', got: {data}"
                );
                let suggested = data["suggested_workspace"].as_str().unwrap_or("");
                assert!(
                    !suggested.is_empty(),
                    "data.suggested_workspace must be non-empty: {data}"
                );
            } else {
                panic!("expected error data payload, got None; error: {err:?}");
            }
        }
        Err(other) => {
            panic!("expected McpError, got different ServiceError: {other}");
        }
        Ok(result) => {
            panic!(
                "expected -32602 error, got success: is_error={:?}, content={:?}",
                result.is_error, result.content
            );
        }
    }
}

/// AC-3b: when workspace param IS provided (even in a linked worktree), the
/// gate must NOT fire. The call proceeds to the normal resolution path.
///
/// This test verifies that param resolution (Step 1) short-circuits before
/// the detection gate, so agents that explicitly pass `workspace` are unaffected.
#[tokio::test]
async fn ac3b_explicit_workspace_param_bypasses_gate() {
    // ── Set up: linked worktree with a forgeplan workspace inside ────────
    let main_tmp = TempDir::new().unwrap();
    if !init_git_repo(main_tmp.path()) {
        eprintln!("SKIP ac3b: git init failed — likely CI environment without git");
        return;
    }

    // Init a forgeplan workspace in the main repo.
    let ws =
        forgeplan_core::workspace::init_workspace(main_tmp.path(), "ac3b").expect("init workspace");
    let _ = forgeplan_core::db::store::LanceStore::init(&ws)
        .await
        .expect("init store");

    let linked_tmp = TempDir::new().unwrap();
    let wt_output = Command::new("git")
        .args([
            "-C",
            &main_tmp.path().to_string_lossy(),
            "worktree",
            "add",
            &linked_tmp.path().to_string_lossy(),
            "-b",
            "wt-ac3b",
        ])
        .output()
        .expect("spawn git worktree add");

    if !wt_output.status.success() {
        eprintln!(
            "SKIP ac3b: git worktree add failed ({}): {}",
            wt_output.status,
            String::from_utf8_lossy(&wt_output.stderr)
        );
        return;
    }

    // ── Boot cold server from the linked worktree dir ────────────────────
    let linked_path = linked_tmp.path().to_path_buf();
    let (client, _srv, _root) = cold_server_from(linked_path).await;

    // ── Call forgeplan_new WITH explicit workspace pointing to main repo ──
    let mut args: Map<String, Value> = Map::new();
    args.insert("kind".to_string(), json!("prd"));
    args.insert("title".to_string(), json!("AC3b Test PRD"));
    // Explicit absolute workspace param — must bypass detection gate.
    args.insert(
        "workspace".to_string(),
        json!(main_tmp.path().to_string_lossy().to_string()),
    );

    let params = CallToolRequestParams::new("forgeplan_new").with_arguments(args);
    let result = tokio::time::timeout(Duration::from_secs(15), client.peer().call_tool(params))
        .await
        .expect("call_tool did not time out");

    // Must NOT be a -32602 worktree error.
    match result {
        Err(rmcp::service::ServiceError::McpError(err)) => {
            assert_ne!(
                err.code.0, -32602,
                "workspace param should bypass detection gate, got -32602: {err:?}"
            );
        }
        Ok(_) => {
            // Success — gate bypassed correctly. No assertion needed on the artifact.
        }
        Err(other) => {
            // Other transport errors are tolerated — the important thing is
            // that the worktree detection gate did not fire.
            eprintln!("ac3b: non-McpError transport error (acceptable): {other}");
        }
    }
}
