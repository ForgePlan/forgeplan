//! PROB-078 gold-standard real-binary subprocess E2E.
//!
//! Every reliable IN-PROCESS harness — the store-layer reopened-handle probe
//! (`store::tests::prob078_reopened_handle_sees_own_update_body`), the
//! full-stack `McpFixture` variants (`prob078_read_after_write_repro.rs`), and
//! the #350 `update → get` tests — returns a FRESH read-after-write. The
//! staleness was only ever observed through a hand-rolled stdio-printf repro.
//!
//! This test isolates the one remaining axis: the real `forgeplan-mcp` BINARY
//! running as a SEPARATE OS PROCESS over real stdio, driven by a RELIABLE rmcp
//! child-process client (byte-identical framing to the server — unlike printf).
//! The seed is identical to the passing in-process variant 1 (a DB row created
//! by a separate handle, then dropped). A divergence here would pin the bug to
//! the process/transport boundary; a PASS confirms PROB-078 was a stdio-printf
//! harness artifact, not a product bug (RED LINE #5: real-binary dogfood).

use std::process::Stdio;

use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::workspace;
use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, CallToolResult, RawContent};
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use tempfile::TempDir;
use tokio::process::Command;

const MARKER: &str = "MARKER-PROB078-SUBPROC-DEADBEEF";
const TEMPLATE: &str = "## Summary\n\nORIGINAL template body (pre-update).";

fn seed_artifact(id: &str) -> NewArtifact {
    NewArtifact {
        id: id.to_string(),
        kind: "prd".to_string(),
        status: "draft".to_string(),
        title: format!("Seed {id}"),
        body: TEMPLATE.to_string(),
        depth: "standard".to_string(),
        author: Some("prob078-subproc".to_string()),
        parent_epic: None,
        valid_until: None,
        tags: Vec::new(),
    }
}

fn obj(v: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    match v {
        serde_json::Value::Object(m) => m,
        other => panic!("expected JSON object for tool args, got: {other}"),
    }
}

/// Concatenate the text content of a `CallToolResult` (handlers serialize their
/// response DTO as stringified JSON inside `content[0].text`).
fn text_of(r: &CallToolResult) -> String {
    let mut s = String::new();
    for c in &r.content {
        if let RawContent::Text(t) = &c.raw {
            s.push_str(&t.text);
        }
    }
    s
}

async fn drive(pass_ws_param: bool) {
    let tempdir = TempDir::new().expect("tempdir");
    let ws = workspace::init_workspace(tempdir.path(), "prob078-subproc").expect("init workspace");

    // Seed a DB row via a separate handle, then drop it — identical to the
    // passing in-process variant 1. The MCP server (a DIFFERENT OS process)
    // will open the store fresh and must observe its own subsequent update.
    {
        let store = LanceStore::init(&ws).await.expect("init store");
        store
            .create_artifact_for_test(&seed_artifact("PRD-700"))
            .await
            .expect("seed PRD-700");
    }

    let root = tempdir.path().to_path_buf();
    let bin = env!("CARGO_BIN_EXE_forgeplan-mcp");

    // Spawn the REAL binary. cwd = project root so `ForgeplanServer::new` finds
    // `.forgeplan/` at startup; clear FORGEPLAN_WORKSPACE so the runner's env
    // cannot leak a different workspace; silence the server's stderr tracing so
    // it does not interleave with the stdout JSON-RPC stream.
    let transport = TokioChildProcess::new(Command::new(bin).configure(|cmd| {
        cmd.current_dir(&root)
            .env_remove("FORGEPLAN_WORKSPACE")
            .stderr(Stdio::null());
    }))
    .expect("spawn forgeplan-mcp");

    // `()` implements ClientHandler; `serve` performs the initialize handshake.
    let client = ().serve(transport).await.expect("mcp initialize handshake");

    let root_str = root.display().to_string();

    // ── forgeplan_update (id=PRD-700, body=MARKER) ──────────────────────────
    let mut upd = serde_json::json!({ "id": "PRD-700", "body": MARKER });
    if pass_ws_param {
        upd["workspace"] = serde_json::json!(root_str);
    }
    let upd_res = client
        .peer()
        .call_tool(CallToolRequestParams::new("forgeplan_update").with_arguments(obj(upd)))
        .await
        .expect("forgeplan_update rpc");
    assert_ne!(
        upd_res.is_error,
        Some(true),
        "forgeplan_update errored: {}",
        text_of(&upd_res)
    );

    // ── forgeplan_get (id=PRD-700) ──────────────────────────────────────────
    let mut get = serde_json::json!({ "id": "PRD-700" });
    if pass_ws_param {
        get["workspace"] = serde_json::json!(root_str);
    }
    let get_res = client
        .peer()
        .call_tool(CallToolRequestParams::new("forgeplan_get").with_arguments(obj(get)))
        .await
        .expect("forgeplan_get rpc");
    let raw = text_of(&get_res);
    let body = serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v["body"].as_str().map(str::to_string))
        .unwrap_or_default();

    assert!(
        body.contains(MARKER),
        "PROB-078 real-binary ({}): read-after-write returned STALE body \
         through the real forgeplan-mcp subprocess. raw_get={raw:?}",
        if pass_ws_param { "with-ws" } else { "no-ws" }
    );

    let _ = client.cancel().await;
}

/// The faithful stdio-repro shape, but reliable: separate-process server, real
/// stdio, no `workspace` param (server resolves via its launch cwd).
#[tokio::test]
async fn real_binary_read_after_write_no_ws_param() {
    drive(false).await;
}

/// Same, but both calls carry an explicit `workspace=<root>` param — the exact
/// shape the original stdio repro used (`workspace=WS` on update and get).
#[tokio::test]
async fn real_binary_read_after_write_with_ws_param() {
    drive(true).await;
}

/// Belt-and-suspenders, fully faithful to the ORIGINAL stdio repro: the
/// artifact is created by the real `forgeplan` CLI (a `NOTE`, separate
/// process), then the real `forgeplan-mcp` binary does `update → get`. This
/// closes the last two fidelity gaps vs the in-test seed (CLI-create + NOTE
/// kind). Skips LOUDLY (never silently) when the sibling CLI binary is not
/// built in the same target dir, so a green run is never a hidden no-op.
#[tokio::test]
async fn real_binary_cli_create_then_mcp_update_get() {
    let mcp_bin = std::path::PathBuf::from(env!("CARGO_BIN_EXE_forgeplan-mcp"));
    let cli_bin = mcp_bin.parent().expect("target dir").join("forgeplan");
    if !cli_bin.exists() {
        eprintln!(
            "SKIP real_binary_cli_create_then_mcp_update_get: sibling CLI binary not built \
             at {} — run `cargo build -p forgeplan` (or the workspace) to enable this variant.",
            cli_bin.display()
        );
        return;
    }

    let tempdir = TempDir::new().expect("tempdir");
    let root = tempdir.path().to_path_buf();

    // ── CLI: init + create a NOTE (separate process, writes file + DB + state) ─
    let init = Command::new(&cli_bin)
        .current_dir(&root)
        .args(["init", "-y"])
        .output()
        .await
        .expect("run forgeplan init");
    assert!(init.status.success(), "forgeplan init failed");

    let newout = Command::new(&cli_bin)
        .current_dir(&root)
        .args(["new", "note", "PROB078 CLI created note"])
        .output()
        .await
        .expect("run forgeplan new note");
    assert!(
        newout.status.success(),
        "forgeplan new failed: {}",
        String::from_utf8_lossy(&newout.stderr)
    );

    // Recover the created id robustly from the projection filename
    // (`.forgeplan/notes/NOTE-NNN-*.md`) rather than parsing stdout format.
    let notes_dir = root.join(".forgeplan").join("notes");
    let id = std::fs::read_dir(&notes_dir)
        .expect("notes dir exists")
        .filter_map(Result::ok)
        .find_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.to_uppercase().starts_with("NOTE-") {
                return None;
            }
            let stem = name.strip_suffix(".md")?;
            // `NOTE-001-some-slug` → `NOTE-001`
            Some(stem.split('-').take(2).collect::<Vec<_>>().join("-"))
        })
        .expect("a NOTE-NNN artifact file was created by the CLI");

    // ── MCP: real binary subprocess, reliable client, update → get ──────────
    let transport = TokioChildProcess::new(Command::new(&mcp_bin).configure(|cmd| {
        cmd.current_dir(&root)
            .env_remove("FORGEPLAN_WORKSPACE")
            .stderr(Stdio::null());
    }))
    .expect("spawn forgeplan-mcp");
    let client = ().serve(transport).await.expect("mcp initialize handshake");

    // Exact replica of the original repro: pass `workspace=<root>` on both the
    // update and the get (the `workspace=WS` shape PROB-078 reported as stale).
    let root_str = root.display().to_string();

    let upd_res = client
        .peer()
        .call_tool(
            CallToolRequestParams::new("forgeplan_update").with_arguments(obj(
                serde_json::json!({ "id": id, "body": MARKER, "workspace": root_str }),
            )),
        )
        .await
        .expect("forgeplan_update rpc");
    assert_ne!(
        upd_res.is_error,
        Some(true),
        "forgeplan_update errored: {}",
        text_of(&upd_res)
    );

    let get_res = client
        .peer()
        .call_tool(
            CallToolRequestParams::new("forgeplan_get")
                .with_arguments(obj(serde_json::json!({ "id": id, "workspace": root_str }))),
        )
        .await
        .expect("forgeplan_get rpc");
    let raw = text_of(&get_res);
    let body = serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v["body"].as_str().map(str::to_string))
        .unwrap_or_default();

    assert!(
        body.contains(MARKER),
        "PROB-078 (CLI-create + real MCP, NOTE kind): read-after-write returned STALE \
         body. id={id} raw_get={raw:?}"
    );

    let _ = client.cancel().await;
}
