//! Regression test for the `reconcile_source_for_read` bug where
//! `spec.path` already points at `.forgeplan` but the code called
//! `.parent()`, making the federated reconcile dead code (it looked
//! for a lance directory one level too high, found nothing, and
//! early-returned with a stale pin).
//!
//! The test: open a scope, capture the source version, edit an ADR
//! in the live workspace, reopen the scope, and assert the version
//! changed. If `reconcile_source_for_read` steps up to `parent()`,
//! it finds no lance dir → early return → stale pin → test fails.

use std::path::{Path, PathBuf};
use std::time::Duration;

use forgeplan_core::constellation::{capture::CaptureLimits, facade, registry};
use forgeplan_core::workspace;
use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, CallToolResult};
use serde_json::Value;
use serial_test::serial;
use tempfile::TempDir;

mod common;
use common::{CallToolEnvelope, value_to_object};

/// The read-profile server paired with a `()`-handler client over a
/// `tokio::io::duplex` transport — same shape as the fixture in
/// `constellation_read_roles.rs`.
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
    let fp = workspace::init_workspace(root, "federation-lazy-sync").expect("init workspace");
    std::fs::write(
        fp.join("adrs/ADR-001-auth.md"),
        "---\nid: ADR-001\nkind: adr\nstatus: accepted\ntitle: Auth storage\n---\n\nBody about auth.\n",
    )
    .unwrap();
    fp
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
async fn scope_reopen_after_md_edit_pins_new_version() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_live = tmp.path().join("ws-live");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_live).unwrap();
    pin_home(&home);

    // Step 1: seed the workspace.
    let fp = seeded_workspace(&ws_live);

    // Step 2: migrate the source onto constellation via maintain_index.
    let fp_canonical = fp.canonicalize().unwrap();
    facade::maintain_index(
        &facade::MaintainCapability::for_strict_maintenance(fp_canonical.clone()),
        CaptureLimits::default(),
        std::time::Instant::now() + Duration::from_secs(120),
    )
    .await
    .unwrap();

    // Verify CURRENT.json exists under ws-live/.forgeplan/lance.
    let lance_dir = fp_canonical.join("lance");
    assert!(
        lance_dir.join("CURRENT.json").exists(),
        "CURRENT.json must exist under {lance_dir:?} after maintain_index"
    );

    // Step 3: register constellation "crew" with the live source.
    let mut reg = registry::ConstellationRegistry::default();
    registry::create(&mut reg, "crew").unwrap();
    registry::add_source(
        &mut reg,
        "crew",
        &fp_canonical,
        registry::SourceRole::Required,
    )
    .unwrap();
    registry::save(&registry::registry_path(&home), &reg).unwrap();

    // Step 4: open the scope and capture the source version.
    let fixture = ReadFixture::new().await;
    let opened = fixture
        .call("constellation_open", serde_json::json!({}))
        .await;
    let opened = opened.assert_ok();
    let _scope = opened["scope_handle"].as_str().unwrap().to_string();
    let initial_version = opened["sources"][0]["version"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        !initial_version.is_empty(),
        "version must be non-empty: {opened}"
    );

    // Step 5: edit the .md file (append a line).
    let adr_path = fp_canonical.join("adrs/ADR-001-auth.md");
    let original = std::fs::read_to_string(&adr_path).unwrap();
    std::fs::write(
        &adr_path,
        format!("{original}\nEdited line for federation test.\n"),
    )
    .unwrap();

    // Step 6: reopen the scope and assert the version changed.
    let reopened = fixture
        .call("constellation_open", serde_json::json!({}))
        .await;
    let reopened = reopened.assert_ok();
    let new_version = reopened["sources"][0]["version"]
        .as_str()
        .unwrap()
        .to_string();

    assert_ne!(
        initial_version, new_version,
        "reconcile_source_for_read must detect the md edit and pin a new version.\n\
         initial: {initial_version}\n\
         new:     {new_version}\n\
         If this fails, reconcile likely stepped up to parent() and found no lance dir,\n\
         early-returning with a stale pin."
    );
}
