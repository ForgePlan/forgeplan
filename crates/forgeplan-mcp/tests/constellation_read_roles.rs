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
        (
            "constellation_impact",
            serde_json::json!({"id": "ADR-001@ws-live", "scope_handle": scope}),
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

    let impact = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-001@ws-live", "scope_handle": scope, "strict": false}),
        )
        .await;
    let report = impact.assert_ok();
    assert_eq!(report["changed"], "ADR-001@ws-live");
    assert_eq!(report["sources"].as_array().unwrap().len(), 1);
    assert_eq!(report["failures"].as_array().unwrap().len(), 1);
    assert_eq!(report["failures"][0]["path"], dead_fp.to_str().unwrap());
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

    let impact = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-001@ws-live", "scope_handle": scope}),
        )
        .await;
    let report = impact.assert_ok();
    assert_eq!(report["changed"], "ADR-001@ws-live");
    assert_eq!(report["sources"].as_array().unwrap().len(), 1);
    assert_eq!(report["failures"].as_array().unwrap().len(), 1);
    assert_eq!(report["failures"][0]["path"], dead_fp.to_str().unwrap());
}

fn registry_for_source(home: &Path, source: &Path) {
    let mut reg = registry::ConstellationRegistry::default();
    registry::create(&mut reg, "crew").unwrap();
    registry::add_source(&mut reg, "crew", source, registry::SourceRole::Required).unwrap();
    registry::save(&registry::registry_path(home), &reg).unwrap();
}

async fn open_live_scope(fixture: &ReadFixture) -> String {
    fixture
        .call("constellation_open", serde_json::json!({}))
        .await
        .assert_ok()["scope_handle"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn assert_live_reads_expired(fixture: &ReadFixture, scope: &str) {
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
            "constellation_graph",
            serde_json::json!({"scope_handle": scope}),
        ),
        (
            "constellation_impact",
            serde_json::json!({"id": "ADR-001", "scope_handle": scope}),
        ),
    ] {
        let env = fixture.call(tool, args).await;
        let result = env.assert_ok();
        assert_eq!(
            result["error"], "snapshot_expired",
            "{tool} served a changed source through an old scope: {}",
            env.raw_text
        );
    }
}

#[tokio::test]
#[serial(env_home)]
async fn live_scope_accepts_stable_reads_across_all_four_handlers() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();
    pin_home(&home);
    let fp = seeded_workspace(&workspace);
    registry_for_source(&home, &fp);

    let fixture = ReadFixture::new().await;
    let opened_env = fixture
        .call("constellation_open", serde_json::json!({}))
        .await;
    let opened = opened_env.assert_ok();
    let scope = opened["scope_handle"].as_str().unwrap().to_string();
    let version = opened["sources"][0]["version"].as_str().unwrap();
    let search_env = fixture
        .call(
            "constellation_search",
            serde_json::json!({"query": "auth", "scope_handle": scope}),
        )
        .await;
    let search = search_env.assert_ok();
    assert_eq!(search["hits"][0]["canonical_key"], "ADR-001");
    assert_eq!(search["sources"][0]["version_key"], version);
    let get_env = fixture
        .call(
            "constellation_get",
            serde_json::json!({"id": "ADR-001", "scope_handle": scope}),
        )
        .await;
    let get = get_env.assert_ok();
    assert_eq!(get["result"], "found");
    assert!(get["body"].as_str().unwrap().contains("Body about auth."));
    assert_eq!(get["source"]["version_key"], version);
    let graph_env = fixture
        .call(
            "constellation_graph",
            serde_json::json!({"scope_handle": scope}),
        )
        .await;
    let graph = graph_env.assert_ok();
    assert_eq!(graph["nodes"][0]["canonical_key"], "ADR-001");
    assert_eq!(graph["nodes"][0]["source"]["version_key"], version);
    let impact_env = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-001", "scope_handle": scope}),
        )
        .await;
    let impact = impact_env.assert_ok();
    assert_eq!(impact["changed"], "ADR-001");
    assert!(impact["absent_in"].as_array().unwrap().is_empty());
    assert_eq!(impact["sources"][0]["version_key"], version);
    let empty_search_env = fixture
        .call(
            "constellation_search",
            serde_json::json!({"query": "no-such-term", "scope_handle": scope}),
        )
        .await;
    let empty_search = empty_search_env.assert_ok();
    assert!(empty_search["hits"].as_array().unwrap().is_empty());
    assert_eq!(empty_search["sources"][0]["version_key"], version);
    let miss_env = fixture
        .call(
            "constellation_get",
            serde_json::json!({"id": "ADR-999", "scope_handle": scope}),
        )
        .await;
    let miss = miss_env.assert_ok();
    assert_eq!(miss["result"], "not_found");
    assert_eq!(miss["sources"][0]["version_key"], version);
}

#[tokio::test]
#[serial(env_home)]
async fn live_scope_accepts_stable_ambiguous_get_with_both_witnesses() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let first = tmp.path().join("first");
    let second = tmp.path().join("second");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    pin_home(&home);
    let first_fp = seeded_workspace(&first);
    let second_fp = seeded_workspace(&second);
    let mut reg = registry::ConstellationRegistry::default();
    registry::create(&mut reg, "crew").unwrap();
    registry::add_source(&mut reg, "crew", &first_fp, registry::SourceRole::Required).unwrap();
    registry::add_source(&mut reg, "crew", &second_fp, registry::SourceRole::Required).unwrap();
    registry::save(&registry::registry_path(&home), &reg).unwrap();

    let fixture = ReadFixture::new().await;
    let opened_env = fixture
        .call("constellation_open", serde_json::json!({}))
        .await;
    let opened = opened_env.assert_ok();
    let scope = opened["scope_handle"].as_str().unwrap().to_string();
    let mut versions: Vec<_> = opened["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|source| source["version"].as_str().unwrap().to_string())
        .collect();
    versions.sort();
    let env = fixture
        .call(
            "constellation_get",
            serde_json::json!({"id": "ADR-001", "scope_handle": scope}),
        )
        .await;
    let result = env.assert_ok();
    assert_eq!(result["result"], "ambiguous", "{}", env.raw_text);
    assert_eq!(result["claimers"].as_array().unwrap().len(), 2);
    assert_eq!(result["sources"].as_array().unwrap().len(), 2);
    assert_ne!(
        result["claimers"][0]["source_id"],
        result["claimers"][1]["source_id"]
    );
    let mut served_versions: Vec<_> = result["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|source| source["version_key"].as_str().unwrap().to_string())
        .collect();
    served_versions.sort();
    assert_eq!(served_versions, versions);
}

#[tokio::test]
#[serial(env_home)]
async fn stable_multisource_conflict_keeps_identity_error_after_full_scan() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let conflicted_dir = tmp.path().join("conflicted");
    let healthy_dir = tmp.path().join("healthy");
    for dir in [&home, &conflicted_dir, &healthy_dir] {
        std::fs::create_dir_all(dir).unwrap();
    }
    pin_home(&home);
    let conflicted = seeded_workspace(&conflicted_dir);
    std::fs::write(
        conflicted.join("adrs/ADR-001-copy.md"),
        "---\nid: ADR-001\nkind: adr\nstatus: accepted\ntitle: Alternate auth storage\n---\n\nAlternate body.\n",
    )
    .unwrap();
    let healthy = seeded_workspace(&healthy_dir);
    let mut reg = registry::ConstellationRegistry::default();
    registry::create(&mut reg, "crew").unwrap();
    registry::add_source(
        &mut reg,
        "crew",
        &conflicted,
        registry::SourceRole::Required,
    )
    .unwrap();
    registry::add_source(&mut reg, "crew", &healthy, registry::SourceRole::Required).unwrap();
    registry::save(&registry::registry_path(&home), &reg).unwrap();

    let fixture = ReadFixture::new().await;
    let scope = open_live_scope(&fixture).await;
    let env = fixture
        .call(
            "constellation_get",
            serde_json::json!({"id": "ADR-001", "scope_handle": scope}),
        )
        .await;
    assert!(env.is_error, "{}", env.raw_text);
    let result = env.json.as_ref().unwrap();
    assert_eq!(
        result["error"], "conflicting_canonical_key",
        "{}",
        env.raw_text
    );
    let titles: Vec<_> = result["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|candidate| candidate["title"].as_str().unwrap())
        .collect();
    assert!(titles.contains(&"Auth storage"));
    assert!(titles.contains(&"Alternate auth storage"));

    std::fs::write(
        conflicted.join("adrs/ADR-001-copy.md"),
        "---\nid: ADR-001\nkind: adr\nstatus: accepted\ntitle: Alternate auth storage\n---\n\nChanged after scope open.\n",
    )
    .unwrap();
    let changed = fixture
        .call(
            "constellation_get",
            serde_json::json!({"id": "ADR-001", "scope_handle": scope}),
        )
        .await;
    assert_eq!(changed.assert_ok()["error"], "snapshot_expired");
}

fn cross_source_workspaces(home: &Path, shared: &Path, hub: &Path) -> (PathBuf, PathBuf) {
    for dir in [home, shared, hub] {
        std::fs::create_dir_all(dir).unwrap();
    }
    pin_home(home);
    let shared_fp = seeded_workspace(shared);
    let hub_fp = seeded_workspace(hub);
    std::fs::write(
        hub_fp.join("prds/PRD-002-dependent.md"),
        "---\nid: PRD-002\nkind: prd\nstatus: draft\ntitle: Hub dependent\nlinks:\n- target: ADR-001@shared\n  relation: based_on\n---\n\nDepends on Shared auth.\n",
    )
    .unwrap();
    std::fs::write(
        hub_fp.join("prds/PRD-003-transitive.md"),
        "---\nid: PRD-003\nkind: prd\nstatus: draft\ntitle: Hub transitive\nlinks:\n- target: PRD-002\n  relation: based_on\n---\n\nDepends on Hub dependent.\n",
    )
    .unwrap();
    let mut reg = registry::ConstellationRegistry::default();
    registry::create(&mut reg, "crew").unwrap();
    registry::add_source(&mut reg, "crew", &shared_fp, registry::SourceRole::Required).unwrap();
    registry::add_source(&mut reg, "crew", &hub_fp, registry::SourceRole::Required).unwrap();
    registry::save(&registry::registry_path(home), &reg).unwrap();
    (shared_fp, hub_fp)
}

fn assert_cross_source_impact(report: &Value, shared: &Path, hub: &Path, count: usize) {
    assert_eq!(report["changed"], "ADR-001@shared");
    let sources = report["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 2);
    let shared_path = shared.canonicalize().unwrap();
    let hub_path = hub.canonicalize().unwrap();
    let shared_source = sources
        .iter()
        .find(|source| source["path"] == shared_path.to_str().unwrap())
        .unwrap();
    let hub_source = sources
        .iter()
        .find(|source| source["path"] == hub_path.to_str().unwrap())
        .unwrap();
    assert_ne!(shared_source["source_id"], hub_source["source_id"]);
    assert!(report["failures"].as_array().unwrap().is_empty());
    let impacted = report["impacted"].as_array().unwrap();
    assert_eq!(impacted.len(), count, "{report}");
    let direct = impacted
        .iter()
        .find(|node| node["canonical_key"] == "PRD-002")
        .unwrap();
    assert_eq!(direct["source"]["source_id"], hub_source["source_id"]);
    assert_eq!(direct["severity"], "direct");
    assert_eq!(direct["depth"], 1);
    assert_eq!(direct["path"][0]["dependent"], "PRD-002");
    assert_eq!(direct["path"][0]["source_id"], hub_source["source_id"]);
    assert_eq!(direct["path"][0]["relation"], "based_on");
    let transitive = impacted
        .iter()
        .find(|node| node["canonical_key"] == "PRD-003")
        .unwrap();
    assert_eq!(transitive["source"]["source_id"], hub_source["source_id"]);
    assert_eq!(transitive["severity"], "transitive");
    assert_eq!(transitive["depth"], 2);
    assert_eq!(transitive["path"][0]["dependent"], "PRD-002");
    assert_eq!(transitive["path"][1]["dependent"], "PRD-003");
    assert_eq!(transitive["path"][0]["source_id"], hub_source["source_id"]);
    assert_eq!(transitive["path"][1]["source_id"], hub_source["source_id"]);
    assert!(
        impacted
            .iter()
            .all(|node| node["canonical_key"] != "ADR-001")
    );
}

#[tokio::test]
#[serial(env_home)]
async fn qualified_impact_tracks_all_sources_and_reopens_on_new_incoming_dependency() {
    let tmp = TempDir::new().unwrap();
    let (shared, hub) = cross_source_workspaces(
        &tmp.path().join("home"),
        &tmp.path().join("shared"),
        &tmp.path().join("hub"),
    );
    let fixture = ReadFixture::new().await;
    let scope = open_live_scope(&fixture).await;
    let initial = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-001@shared", "scope_handle": scope}),
        )
        .await;
    assert_cross_source_impact(initial.assert_ok(), &shared, &hub, 2);
    let missing = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-001@no-such-source", "scope_handle": scope}),
        )
        .await;
    assert_eq!(missing.assert_ok()["error"], "source_unavailable");
    let hub_path = hub.canonicalize().unwrap();
    let old_hub_version = initial.assert_ok()["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| source["path"] == hub_path.to_str().unwrap())
        .unwrap()["version_key"]
        .clone();

    std::fs::write(
        hub.join("prds/PRD-004-new-dependent.md"),
        "---\nid: PRD-004\nkind: prd\nstatus: draft\ntitle: New dependency\nlinks:\n- target: ADR-001@shared\n  relation: based_on\n---\n\nAdded after opening the scope.\n",
    )
    .unwrap();
    let stale = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-001@shared", "scope_handle": scope}),
        )
        .await;
    assert_eq!(stale.assert_ok()["error"], "snapshot_expired");
    let stale_error = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-001@no-such-source", "scope_handle": scope}),
        )
        .await;
    assert_eq!(stale_error.assert_ok()["error"], "snapshot_expired");

    let fresh_scope = open_live_scope(&fixture).await;
    let fresh = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-001@shared", "scope_handle": fresh_scope}),
        )
        .await;
    let report = fresh.assert_ok();
    assert_cross_source_impact(report, &shared, &hub, 3);
    let new_hub = report["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| source["path"] == hub_path.to_str().unwrap())
        .unwrap();
    assert_ne!(new_hub["version_key"], old_hub_version);
    let added = report["impacted"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["canonical_key"] == "PRD-004")
        .unwrap();
    assert_eq!(added["source"]["source_id"], new_hub["source_id"]);
    assert_eq!(added["severity"], "direct");
    assert_eq!(added["path"][0]["dependent"], "PRD-004");
    assert_eq!(added["path"][0]["source_id"], new_hub["source_id"]);
    assert_eq!(added["path"][0]["relation"], "based_on");
}

fn commit_snapshot_fixture(root: &Path) {
    for args in [
        &["init", "-q"][..],
        &["add", "-f", ".forgeplan"][..],
        &["commit", "-q", "-m", "snapshot fixture"][..],
    ] {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_AUTHOR_NAME", "Eli Rum")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Eli Rum")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .output()
            .expect("git must be available for the snapshot consumer test");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[tokio::test]
#[serial(env_home)]
async fn qualified_impact_snapshot_uses_pinned_cross_source_dependency() {
    let tmp = TempDir::new().unwrap();
    let shared_root = tmp.path().join("shared");
    let hub_root = tmp.path().join("hub");
    let (shared, hub) = cross_source_workspaces(&tmp.path().join("home"), &shared_root, &hub_root);
    commit_snapshot_fixture(&shared_root);
    commit_snapshot_fixture(&hub_root);
    let fixture = ReadFixture::new().await;
    let scope = open_live_scope(&fixture).await;
    let live = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-001@shared", "scope_handle": scope}),
        )
        .await;
    assert_cross_source_impact(live.assert_ok(), &shared, &hub, 2);
    let snapshot = fixture
        .call(
            "constellation_snapshot",
            serde_json::json!({"handle": scope}),
        )
        .await;
    let snapshot_result = snapshot.assert_ok();
    assert_eq!(snapshot_result["pinned"].as_array().unwrap().len(), 2);
    assert!(snapshot_result["skipped"].as_array().unwrap().is_empty());
    let snapshot_handle = snapshot_result["snapshot_handle"].as_str().unwrap();

    std::fs::write(
        hub.join("prds/PRD-002-dependent.md"),
        "---\nid: PRD-002\nkind: prd\nstatus: draft\ntitle: Hub dependent\n---\n\nDependency removed in working tree.\n",
    )
    .unwrap();
    let expired = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-001@shared", "scope_handle": scope}),
        )
        .await;
    assert_eq!(expired.assert_ok()["error"], "snapshot_expired");
    let pinned = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-001@shared", "snapshot_handle": snapshot_handle}),
        )
        .await;
    let report = pinned.assert_ok();
    assert_cross_source_impact(report, &shared, &hub, 2);
    assert!(
        report["sources"]
            .as_array()
            .unwrap()
            .iter()
            .all(|source| source["channel"] == "pinned_git")
    );
    let new_scope = open_live_scope(&fixture).await;
    let current = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-001@shared", "scope_handle": new_scope}),
        )
        .await;
    assert!(
        current.assert_ok()["impacted"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

fn assert_impact_ambiguity(env: &CallToolEnvelope, expected_alpha_title: &str) {
    assert!(env.is_error, "{}", env.raw_text);
    let response = env.json.as_ref().unwrap();
    assert_eq!(response["error"], "ambiguous", "{}", env.raw_text);
    assert_eq!(response["input"], "ADR-101@shared");
    let candidates = response["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 2, "{}", env.raw_text);
    for (key, title, file) in [
        ("adr-choice-alpha", expected_alpha_title, "ADR-101-alpha.md"),
        ("adr-choice-beta", "Beta choice", "ADR-101-beta.md"),
    ] {
        let candidate = candidates
            .iter()
            .find(|candidate| candidate["canonical_key"] == key)
            .unwrap();
        assert_eq!(candidate["display_id"], "ADR-101");
        assert_eq!(candidate["title"], title);
        let path = candidate["relative_path"].as_str().unwrap();
        assert!(path.starts_with("src:"), "{path}");
        assert!(path.ends_with(&format!("adrs/{file}")), "{path}");
    }
}

#[tokio::test]
#[serial(env_home)]
async fn ambiguous_impact_checks_live_witnesses_and_preserves_snapshot_candidates() {
    let tmp = TempDir::new().unwrap();
    let shared_root = tmp.path().join("shared");
    let hub_root = tmp.path().join("hub");
    let (shared, hub) = cross_source_workspaces(&tmp.path().join("home"), &shared_root, &hub_root);
    let alpha = shared.join("adrs/ADR-101-alpha.md");
    std::fs::write(
        &alpha,
        "---\nid: ADR-101\nslug: adr-choice-alpha\nkind: adr\nstatus: accepted\ntitle: Alpha choice\n---\n\nFirst choice.\n",
    )
    .unwrap();
    std::fs::write(
        shared.join("adrs/ADR-101-beta.md"),
        "---\nid: ADR-101\nslug: adr-choice-beta\nkind: adr\nstatus: accepted\ntitle: Beta choice\n---\n\nSecond choice.\n",
    )
    .unwrap();
    commit_snapshot_fixture(&shared_root);
    commit_snapshot_fixture(&hub_root);
    let fixture = ReadFixture::new().await;
    let scope = open_live_scope(&fixture).await;
    let snapshot = fixture
        .call(
            "constellation_snapshot",
            serde_json::json!({"handle": scope}),
        )
        .await;
    let pinned = snapshot.assert_ok();
    assert_eq!(pinned["pinned"].as_array().unwrap().len(), 2);
    assert!(pinned["skipped"].as_array().unwrap().is_empty());
    let snapshot_handle = pinned["snapshot_handle"].as_str().unwrap();
    let original = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-101@shared", "scope_handle": scope}),
        )
        .await;
    assert_impact_ambiguity(&original, "Alpha choice");

    std::fs::write(
        hub.join("adrs/ADR-001-auth.md"),
        "---\nid: ADR-001\nkind: adr\nstatus: accepted\ntitle: Hub auth changed\n---\n\nNew hub basis.\n",
    )
    .unwrap();
    let stale_hub = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-101@shared", "scope_handle": scope}),
        )
        .await;
    let stale_hub = stale_hub.assert_ok();
    assert_eq!(stale_hub["error"], "snapshot_expired");
    assert!(stale_hub.get("candidates").is_none());

    let reopened = open_live_scope(&fixture).await;
    let stable = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-101@shared", "scope_handle": reopened}),
        )
        .await;
    assert_impact_ambiguity(&stable, "Alpha choice");
    std::fs::write(
        &alpha,
        "---\nid: ADR-101\nslug: adr-choice-alpha\nkind: adr\nstatus: accepted\ntitle: Alpha revised\n---\n\nRevised choice.\n",
    )
    .unwrap();
    let stale_shared = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-101@shared", "scope_handle": reopened}),
        )
        .await;
    let stale_shared = stale_shared.assert_ok();
    assert_eq!(stale_shared["error"], "snapshot_expired");
    assert!(stale_shared.get("candidates").is_none());
    let fresh_scope = open_live_scope(&fixture).await;
    let fresh = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-101@shared", "scope_handle": fresh_scope}),
        )
        .await;
    assert_impact_ambiguity(&fresh, "Alpha revised");
    let frozen = fixture
        .call(
            "constellation_impact",
            serde_json::json!({"id": "ADR-101@shared", "snapshot_handle": snapshot_handle}),
        )
        .await;
    assert_impact_ambiguity(&frozen, "Alpha choice");
}

#[tokio::test]
#[serial(env_home)]
async fn recovered_optional_source_requires_a_new_scope_before_pack() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let live = tmp.path().join("live");
    let recovering = tmp.path().join("recovering");
    for dir in [&home, &live, &recovering] {
        std::fs::create_dir_all(dir).unwrap();
    }
    pin_home(&home);
    let live_fp = seeded_workspace(&live);
    let recovering_fp = seeded_workspace(&recovering);
    let mut reg = registry::ConstellationRegistry::default();
    registry::create(&mut reg, "crew").unwrap();
    registry::add_source(&mut reg, "crew", &live_fp, registry::SourceRole::Required).unwrap();
    registry::add_source(
        &mut reg,
        "crew",
        &recovering_fp,
        registry::SourceRole::Optional,
    )
    .unwrap();
    registry::save(&registry::registry_path(&home), &reg).unwrap();
    std::fs::remove_dir_all(&recovering_fp).unwrap();
    let fixture = ReadFixture::new().await;
    let old_scope = open_live_scope(&fixture).await;
    let restored = seeded_workspace(&recovering);
    std::fs::write(
        restored.join("adrs/ADR-001-auth.md"),
        "---\nid: ADR-001\nkind: adr\nstatus: accepted\ntitle: Recovered storage\n---\n\nRecovered source constraint.\n",
    )
    .unwrap();
    let expired = fixture
        .call(
            "constellation_pack",
            serde_json::json!({
                "task": "recovered storage", "scope_handle": old_scope,
                "seeds": ["ADR-001@recovering"]
            }),
        )
        .await;
    assert_eq!(expired.assert_ok()["error"], "snapshot_expired");
    let new_scope = open_live_scope(&fixture).await;
    let fresh = fixture
        .call(
            "constellation_pack",
            serde_json::json!({
                "task": "recovered storage", "scope_handle": new_scope,
                "seeds": ["ADR-001@recovering"]
            }),
        )
        .await;
    let pack = fresh.assert_ok();
    assert!(pack["failures"].as_array().unwrap().is_empty());
    let restored_path = restored.canonicalize().unwrap();
    assert!(pack["units"].as_array().unwrap().iter().any(|unit| {
        unit["source"]["path"] == restored_path.to_str().unwrap()
            && unit["excerpt"]
                .as_str()
                .is_some_and(|text| text.contains("Recovered source constraint."))
    }));
}

async fn publish_diagnostic_collision(workspace: &Path) {
    use forgeplan_core::artifact::identity_repair::{
        apply_repair_plan, build_repair_plan, census_workspace, rollback_repair,
    };
    use forgeplan_core::constellation::facade::{MaintainCapability, reconcile_identity_repair};
    let census = census_workspace(workspace).unwrap();
    let plan = build_repair_plan(workspace, &census, &[]).unwrap();
    let applied = apply_repair_plan(workspace, &plan).unwrap();
    rollback_repair(workspace, &applied.transaction_id).unwrap();
    reconcile_identity_repair(
        &MaintainCapability::for_strict_maintenance(workspace.to_path_buf()),
        &applied.transaction_id,
        std::time::Instant::now() + Duration::from_secs(120),
    )
    .await
    .unwrap();
}

#[tokio::test]
#[serial(env_home)]
async fn published_conflict_scope_tracks_source_digest_not_generation() {
    use forgeplan_core::constellation::{capture::CaptureLimits, facade};
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let root = tmp.path().join("published");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&root).unwrap();
    pin_home(&home);
    let fp = seeded_workspace(&root).canonicalize().unwrap();
    facade::maintain_index(
        &facade::MaintainCapability::for_strict_maintenance(fp.clone()),
        CaptureLimits::default(),
        std::time::Instant::now() + Duration::from_secs(120),
    )
    .await
    .unwrap();
    std::fs::write(
        fp.join("adrs/ADR-001-copy.md"),
        "---\nid: ADR-001\nkind: adr\nstatus: accepted\ntitle: Alternate auth storage\n---\n\nAlternate body.\n",
    )
    .unwrap();
    publish_diagnostic_collision(&fp).await;
    registry_for_source(&home, &fp);
    let fixture = ReadFixture::new().await;
    let scope = open_live_scope(&fixture).await;
    for republish_same_bytes in [false, true] {
        if republish_same_bytes {
            publish_diagnostic_collision(&fp).await;
        }
        let env = fixture
            .call(
                "constellation_get",
                serde_json::json!({"id": "ADR-001", "scope_handle": scope}),
            )
            .await;
        assert!(env.is_error, "{}", env.raw_text);
        let response = env.json.as_ref().unwrap();
        assert_eq!(response["error"], "conflicting_canonical_key");
        let candidates = response["candidates"].as_array().unwrap();
        let titles: Vec<_> = candidates
            .iter()
            .map(|c| c["title"].as_str().unwrap())
            .collect();
        assert_eq!(candidates.len(), 2);
        assert!(titles.contains(&"Auth storage"));
        assert!(titles.contains(&"Alternate auth storage"));
    }
    std::fs::write(
        fp.join("adrs/ADR-002-new.md"),
        "---\nid: ADR-002\nkind: adr\nstatus: accepted\ntitle: New constraint\n---\n\nChanged source census.\n",
    )
    .unwrap();
    publish_diagnostic_collision(&fp).await;
    let changed = fixture
        .call(
            "constellation_get",
            serde_json::json!({"id": "ADR-001", "scope_handle": scope}),
        )
        .await;
    assert_eq!(changed.assert_ok()["error"], "snapshot_expired");
}

#[tokio::test]
#[serial(env_home)]
async fn unchanged_missing_required_source_remains_source_unavailable() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let live = tmp.path().join("live");
    let dead = tmp.path().join("dead");
    for dir in [&home, &live, &dead] {
        std::fs::create_dir_all(dir).unwrap();
    }
    pin_home(&home);
    let live_fp = seeded_workspace(&live);
    let dead_fp = seeded_workspace(&dead);
    registry_with_dead_source(&home, &live_fp, &dead_fp, false);

    let fixture = ReadFixture::new().await;
    let scope = open_live_scope(&fixture).await;
    let env = fixture
        .call(
            "constellation_search",
            serde_json::json!({"query": "auth", "scope_handle": scope}),
        )
        .await;
    let result = env.assert_ok();
    assert_eq!(result["error"], "source_unavailable", "{}", env.raw_text);
}

#[tokio::test]
#[serial(env_home)]
async fn live_scope_rejects_content_change_after_open_for_all_four_handlers() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();
    pin_home(&home);
    let fp = seeded_workspace(&workspace);
    registry_for_source(&home, &fp);

    let fixture = ReadFixture::new().await;
    let scope = open_live_scope(&fixture).await;
    std::fs::write(
        fp.join("adrs/ADR-001-auth.md"),
        "---\nid: ADR-001\nkind: adr\nstatus: accepted\ntitle: Auth storage\n---\n\nChanged body.\n",
    )
    .unwrap();
    assert_live_reads_expired(&fixture, &scope).await;
}

#[tokio::test]
#[serial(env_home)]
async fn live_scope_rejects_deleted_source_after_open_for_all_four_handlers() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();
    pin_home(&home);
    let fp = seeded_workspace(&workspace);
    registry_for_source(&home, &fp);

    let fixture = ReadFixture::new().await;
    let scope = open_live_scope(&fixture).await;
    std::fs::remove_file(fp.join("adrs/ADR-001-auth.md")).unwrap();
    assert_live_reads_expired(&fixture, &scope).await;
}

#[tokio::test]
#[serial(env_home)]
async fn live_scope_rejects_new_ambiguity_after_open() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();
    pin_home(&home);
    let fp = seeded_workspace(&workspace);
    registry_for_source(&home, &fp);

    let fixture = ReadFixture::new().await;
    let scope = open_live_scope(&fixture).await;
    std::fs::write(
        fp.join("adrs/ADR-001-copy.md"),
        "---\nid: ADR-001\nkind: adr\nstatus: accepted\ntitle: Alternate auth storage\n---\n\nAlternate body.\n",
    )
    .unwrap();
    assert_live_reads_expired(&fixture, &scope).await;
}

#[tokio::test]
#[serial(env_home)]
async fn pack_counts_the_serialized_tool_result_envelope() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();
    pin_home(&home);
    let fp = seeded_workspace(&workspace);
    let mut reg = registry::ConstellationRegistry::default();
    registry::create(&mut reg, "crew").unwrap();
    registry::add_source(&mut reg, "crew", &fp, registry::SourceRole::Required).unwrap();
    registry::save(&registry::registry_path(&home), &reg).unwrap();
    let fixture = ReadFixture::new().await;
    let opened = fixture
        .call("constellation_open", serde_json::json!({}))
        .await;
    let scope = opened.assert_ok()["scope_handle"].as_str().unwrap();
    let args = serde_json::json!({"task": "auth \"雪\"", "scope_handle": scope, "byte_cap": 20000});
    let params =
        CallToolRequestParams::new("constellation_pack").with_arguments(value_to_object(args));
    let result = fixture.client.peer().call_tool(params).await.unwrap();
    let measured = serde_json::to_vec(&result).unwrap().len();
    let envelope = CallToolEnvelope::from(result);
    let pack = envelope.assert_ok();
    assert_eq!(
        pack["receipt"]["measured_bytes"].as_u64().unwrap() as usize,
        measured
    );
    assert!(measured <= 20000);
    assert!(
        pack["units"][0]["excerpt"]
            .as_str()
            .unwrap()
            .contains("Body about auth")
    );
}

#[derive(Debug, Clone)]
struct VersionedPackClient(rmcp::model::ProtocolVersion);

impl rmcp::ClientHandler for VersionedPackClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut info = rmcp::model::ClientConfig::default();
        info.protocol_version = self.0.clone();
        info
    }
}

#[tokio::test]
#[serial(env_home)]
async fn pack_measurement_matches_both_negotiated_result_shapes() {
    use rmcp::model::{InitializeResult, ProtocolVersion};
    use rmcp::service::serve_directly;
    use rmcp::{ClientHandler, RoleClient, RoleServer};

    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();
    pin_home(&home);
    let fp = seeded_workspace(&workspace);
    let mut reg = registry::ConstellationRegistry::default();
    registry::create(&mut reg, "crew").unwrap();
    registry::add_source(&mut reg, "crew", &fp, registry::SourceRole::Required).unwrap();
    registry::save(&registry::registry_path(&home), &reg).unwrap();

    for version in [ProtocolVersion::V_2025_11_25, ProtocolVersion::V_2026_07_28] {
        let modern = version == ProtocolVersion::V_2026_07_28;
        let handler = VersionedPackClient(version.clone());
        let mut server_info = InitializeResult::default();
        server_info.protocol_version = version;
        let (server_io, client_io) = tokio::io::duplex(64 * 1024);
        let server = serve_directly::<RoleServer, _, _, _, _>(
            forgeplan_mcp::constellation_read::ConstellationReadServer::new(),
            server_io,
            Some(handler.get_info()),
        );
        let server_task = tokio::spawn(async move { server.waiting().await });
        let client =
            serve_directly::<RoleClient, _, _, _, _>(handler, client_io, Some(server_info.into()));
        let open = client
            .call_tool(
                CallToolRequestParams::new("constellation_open")
                    .with_arguments(value_to_object(serde_json::json!({}))),
            )
            .await
            .unwrap();
        let open = CallToolEnvelope::from(open);
        let scope = open.assert_ok()["scope_handle"].as_str().unwrap();
        let result = client
            .call_tool(
                CallToolRequestParams::new("constellation_pack").with_arguments(value_to_object(
                    serde_json::json!({"task":"auth \"雪\"","scope_handle":scope,"byte_cap":20000}),
                )),
            )
            .await
            .unwrap();
        assert_eq!(result.result_type.is_some(), modern);
        let bytes = serde_json::to_vec(&result).unwrap().len();
        let envelope = CallToolEnvelope::from(result);
        assert_eq!(
            envelope.assert_ok()["receipt"]["measured_bytes"]
                .as_u64()
                .unwrap() as usize,
            bytes
        );
        assert!(bytes <= 20000);
        client.cancel().await.unwrap();
        server_task.await.unwrap().unwrap();
    }
}

#[cfg(unix)]
#[tokio::test]
#[serial(env_home)]
async fn pack_refuses_symlink_replacement_after_unreadable_scope_open() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let source = tmp.path().join("source");
    let outside = tmp.path().join("outside");
    for dir in [&home, &source, &outside] {
        std::fs::create_dir_all(dir).unwrap();
    }
    pin_home(&home);
    let root = seeded_workspace(&source);
    let outside_root = seeded_workspace(&outside);
    let mut reg = registry::ConstellationRegistry::default();
    registry::create(&mut reg, "crew").unwrap();
    registry::add_source(&mut reg, "crew", &root, registry::SourceRole::Required).unwrap();
    registry::save(&registry::registry_path(&home), &reg).unwrap();
    std::fs::rename(&root, source.join("saved")).unwrap();
    let fixture = ReadFixture::new().await;
    let opened = fixture
        .call("constellation_open", serde_json::json!({}))
        .await;
    let scope = opened.assert_ok()["scope_handle"].as_str().unwrap();
    std::os::unix::fs::symlink(&outside_root, &root).unwrap();
    let strict = fixture
        .call(
            "constellation_pack",
            serde_json::json!({"task":"auth","scope_handle":scope}),
        )
        .await;
    assert_eq!(strict.assert_ok()["error"], "source_unavailable");
    let partial = fixture
        .call(
            "constellation_pack",
            serde_json::json!({"task":"auth","scope_handle":scope,"strict":false}),
        )
        .await;
    let result = partial.assert_ok();
    assert_eq!(result["source_complete"], false);
    assert!(result["units"].as_array().unwrap().is_empty());
    assert!(
        result["failures"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("symlink")
    );
}
