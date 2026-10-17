//! E2E for the required/optional source-role contract on the
//! `constellation-read` MCP profile (`ConstellationReadServer`).
//!
//! The contract under test, over the real JSON-RPC transport (duplex
//! channel, same shape as `common::McpFixture` but bound to the read
//! profile server):
//!
//! * a scope whose **required** source is dead makes `constellation_search`
//!   / `constellation_get` answer a `{"error": "source_unavailable"}`
//!   envelope — the strict-read refusal, not a partial result that would
//!   read as complete;
//! * `strict: false` on the call (the MCP spelling of `--allow-partial`)
//!   demotes the dead source to a `failures` entry and the live source
//!   still answers;
//! * a scope whose dead source is **optional** answers `partial: true`
//!   under a default strict read;
//! * `constellation_open` reports each source's `role` so a caller can
//!   see which sources are load-bearing.
//!
//! HOME is pinned to a tempdir so `constellation_open` reads a hermetic
//! `~/.forgeplan/constellations.yaml`. Tests mutate process-global env —
//! serialized under `env_home` per this crate's convention.

use std::path::{Path, PathBuf};
use std::time::Duration;

use forgeplan_core::constellation::registry;
use forgeplan_core::workspace;
use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, CallToolResult};
use serde_json::Value;
use serial_test::serial;
use tempfile::TempDir;

mod common;
use common::{CallToolEnvelope, value_to_object};

/// The read-profile server paired with a `()`-handler client over a
/// `tokio::io::duplex` transport — `McpFixture`'s shape, minus the
/// workspace store this server never touches.
struct ReadFixture {
    client: rmcp::service::RunningService<rmcp::RoleClient, ()>,
    _server_task: tokio::task::JoinHandle<()>,
}

impl ReadFixture {
    async fn new() -> Self {
        let server = forgeplan_mcp::constellation_read::ConstellationReadServer::new();
        let (server_io, client_io) = tokio::io::duplex(64 * 1024);
        let server_task = tokio::spawn(async move {
            if let Ok(running) = server.serve(server_io).await {
                let _ = running.waiting().await;
            }
        });
        let client = ().serve(client_io).await.expect("client handshake");
        Self {
            client,
            _server_task: server_task,
        }
    }

    async fn call(&self, name: &'static str, args: Value) -> CallToolEnvelope {
        let params = CallToolRequestParams::new(name).with_arguments(value_to_object(args));
        let result: CallToolResult = tokio::time::timeout(
            Duration::from_secs(15),
            self.client.peer().call_tool(params),
        )
        .await
        .unwrap_or_else(|_| panic!("tool `{name}` timed out (15s)"))
        .unwrap_or_else(|e| panic!("tool `{name}` rpc error: {e}"));
        CallToolEnvelope::from(result)
    }
}

/// A `.forgeplan` workspace with one artifact, through the real init path.
fn seeded_workspace(root: &Path) -> PathBuf {
    let fp = workspace::init_workspace(root, "roles-test").expect("init workspace");
    std::fs::write(
        fp.join("adrs/ADR-001-auth.md"),
        "---\nid: ADR-001\nkind: adr\nstatus: accepted\ntitle: Auth storage\n---\n\nBody about auth.\n",
    )
    .unwrap();
    fp
}

/// Write a `constellations.yaml` under `home` through the real atomic
/// `registry::save`, register `live` and `dead`, then delete `dead`'s
/// checkout so the registered path is stale — the outage the contract
/// exists for.
fn registry_with_dead_source(home: &Path, live: &Path, dead: &Path, dead_optional: bool) {
    let mut reg = registry::ConstellationRegistry::default();
    registry::create(&mut reg, "crew").unwrap();
    registry::add_source(&mut reg, "crew", live, registry::SourceRole::Required).unwrap();
    let role = if dead_optional {
        registry::SourceRole::Optional
    } else {
        registry::SourceRole::Required
    };
    registry::add_source(&mut reg, "crew", dead, role).unwrap();
    registry::save(&registry::registry_path(home), &reg).unwrap();
    // The checkout dies after registration — on purpose.
    std::fs::remove_dir_all(dead.parent().unwrap()).unwrap();
}

fn pin_home(home: &Path) {
    // SAFETY-free in practice: every test here is `#[serial(env_home)]`.
    unsafe {
        std::env::set_var("HOME", home);
        std::env::set_var("USERPROFILE", home);
        std::env::set_var("XDG_DATA_HOME", home);
    }
}

#[tokio::test]
#[serial(env_home)]
async fn dead_required_source_fails_strict_reads_and_strict_false_recovers() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_live = tmp.path().join("ws-live");
    let ws_dead = tmp.path().join("ws-dead");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_live).unwrap();
    std::fs::create_dir_all(&ws_dead).unwrap();
    pin_home(&home);

    let live_fp = seeded_workspace(&ws_live);
    let dead_fp = seeded_workspace(&ws_dead);
    registry_with_dead_source(&home, &live_fp, &dead_fp, false);

    let fixture = ReadFixture::new().await;
    let opened = fixture
        .call("constellation_open", serde_json::json!({}))
        .await;
    let opened = opened.assert_ok();
    let scope = opened["scope_handle"].as_str().unwrap().to_string();
    // `open` reports each source's role — the caller can see which are
    // load-bearing before a read ever runs.
    let roles: Vec<&str> = opened["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles, vec!["required", "required"], "{opened}");

    // Strict by default: the dead required source aborts the read.
    for (tool, args) in [
        (
            "constellation_search",
            serde_json::json!({"query": "auth", "scope_handle": scope}),
        ),
        (
            "constellation_get",
            serde_json::json!({"id": "ADR-001", "scope_handle": scope}),
        ),
        (
            "constellation_pack",
            serde_json::json!({"task": "auth storage", "scope_handle": scope}),
        ),
        (
            "constellation_graph",
            serde_json::json!({"scope_handle": scope}),
        ),
        (
            "constellation_impact",
            serde_json::json!({"id": "ADR-001", "scope_handle": scope}),
        ),
    ] {
        let env = fixture.call(tool, args).await;
        let json = env.assert_ok();
        assert_eq!(
            json["error"].as_str().unwrap_or(""),
            "source_unavailable",
            "{tool} must refuse a strict read over a dead required source: {}",
            env.raw_text
        );
    }

    // `strict: false` — the caller accepts a partial answer; the live
    // source responds and the dead one is a named failure.
    let env = fixture
        .call(
            "constellation_search",
            serde_json::json!({"query": "auth", "scope_handle": scope, "strict": false}),
        )
        .await;
    let json = env.assert_ok();
    assert_eq!(json["partial"], true, "{}", env.raw_text);
    assert_eq!(json["failures"].as_array().unwrap().len(), 1);
    assert_eq!(json["hits"].as_array().unwrap().len(), 1);

    let env = fixture
        .call(
            "constellation_get",
            serde_json::json!({"id": "ADR-001", "scope_handle": scope, "strict": false}),
        )
        .await;
    let json = env.assert_ok();
    assert_eq!(json["result"], "found", "{}", env.raw_text);
}

#[tokio::test]
#[serial(env_home)]
async fn dead_optional_source_is_partial_not_fatal() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_live = tmp.path().join("ws-live");
    let ws_dead = tmp.path().join("ws-dead");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_live).unwrap();
    std::fs::create_dir_all(&ws_dead).unwrap();
    pin_home(&home);

    let live_fp = seeded_workspace(&ws_live);
    let dead_fp = seeded_workspace(&ws_dead);
    registry_with_dead_source(&home, &live_fp, &dead_fp, true);

    let fixture = ReadFixture::new().await;
    let opened = fixture
        .call("constellation_open", serde_json::json!({}))
        .await;
    let opened = opened.assert_ok();
    let scope = opened["scope_handle"].as_str().unwrap().to_string();
    let roles: Vec<&str> = opened["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles, vec!["required", "optional"], "{opened}");

    // Default strict read: the optional outage is `partial`, not an
    // error — the live source's hit still arrives.
    let env = fixture
        .call(
            "constellation_search",
            serde_json::json!({"query": "auth", "scope_handle": scope}),
        )
        .await;
    let json = env.assert_ok();
    assert_eq!(json["partial"], true, "{}", env.raw_text);
    assert_eq!(json["failures"].as_array().unwrap().len(), 1);
    assert_eq!(json["hits"].as_array().unwrap().len(), 1);
    assert_eq!(json["hits"][0]["canonical_key"], "ADR-001");

    let env = fixture
        .call(
            "constellation_get",
            serde_json::json!({"id": "ADR-001", "scope_handle": scope}),
        )
        .await;
    let json = env.assert_ok();
    assert_eq!(json["result"], "found", "{}", env.raw_text);
}
