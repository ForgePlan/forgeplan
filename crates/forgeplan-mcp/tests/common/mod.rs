//! Shared MCP integration-test fixture.
//!
//! Used by `integration_e2e.rs` (PROB-060 Phase 2.4 identity-triple contracts)
//! and `integration_full_coverage.rs` (cross-tool coverage matrix). Extracted
//! to avoid verbatim duplication of the JSON-RPC + tempdir + duplex-transport
//! harness across both files (Wave 4 code-review MAJOR-1).
//!
//! Cargo treats `tests/common/mod.rs` as a private module that does NOT itself
//! become an integration-test target — each test file pulls it in with
//! `mod common; use common::McpFixture;`. The `#[allow(dead_code)]` attributes
//! on individual items silence "unused" warnings for items that are only used
//! by one of the two test files (Cargo compiles `common` once per test binary
//! and unused items show up as warnings in the consumer that doesn't touch
//! them).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use forgeplan_core::db::store::LanceStore;
use forgeplan_core::workspace;
use forgeplan_mcp::ForgeplanServer;
use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, CallToolResult, RawContent};
use serde_json::{Map, Value};
use tempfile::TempDir;

/// Live in-process MCP fixture: tempdir-rooted server, paired with a
/// `()`-handler client over a `tokio::io::duplex` transport. The fixture
/// owns the tempdir so dropping the test releases all temp state.
pub struct McpFixture {
    _tempdir: TempDir,
    pub workspace_path: PathBuf,
    /// rmcp client peer — every test issues `call_tool` through this.
    client: rmcp::service::RunningService<rmcp::RoleClient, ()>,
    /// Holds the spawned server task so it lives as long as the client.
    /// Cancelled on drop via `tokio::task::JoinHandle::abort`.
    _server_task: tokio::task::JoinHandle<()>,
}

impl McpFixture {
    /// Set up tempdir + workspace + LanceStore + ForgeplanServer +
    /// in-memory JSON-RPC client. Awaits the rmcp `initialize` handshake
    /// before returning so the first `call_tool` lands on a fully-ready peer.
    pub async fn new() -> Self {
        Self::new_with_seed(|_| std::future::ready(())).await
    }

    /// Two-phase setup so tests can seed artifacts via
    /// `LanceStore::create_artifact_for_test` BEFORE the MCP server opens
    /// its own Lance handle. LanceDB takes a versioned snapshot at open
    /// time; if seeding happens after `ForgeplanServer::new` (which calls
    /// `LanceStore::open` internally), the server's read path operates on
    /// an older snapshot and seeded rows are invisible until the next
    /// re-open. The async closure runs against the test's exclusive store
    /// handle while the server side is still uninitialized.
    pub async fn new_with_seed<F, Fut>(seed: F) -> Self
    where
        F: FnOnce(Arc<LanceStore>) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let tempdir = TempDir::new().expect("tempdir");
        let workspace_path =
            workspace::init_workspace(tempdir.path(), "mcp-e2e-test").expect("init workspace");
        let store = Arc::new(
            LanceStore::init(&workspace_path)
                .await
                .expect("init lance store"),
        );

        // Run any caller-provided seed BEFORE the server opens its own
        // store handle. Any rows written here are guaranteed to be
        // visible to the server's first read.
        seed(Arc::clone(&store)).await;

        // ForgeplanServer::new walks up from the supplied root looking for
        // `.forgeplan/` and opens a second Lance handle. The fixture's
        // own `store` handle is reserved for seeding fixtures.
        let server = ForgeplanServer::new(tempdir.path().to_path_buf()).await;

        let (server_io, client_io) = tokio::io::duplex(64 * 1024);
        let server_task = tokio::spawn(async move {
            // serve(transport) returns a RunningService whose `waiting()`
            // future resolves when the transport closes. We swallow the
            // result here — the test's lifetime drives the client side and
            // dropping the client closes the duplex pipe, which terminates
            // the server cleanly.
            if let Ok(running) = server.serve(server_io).await {
                let _ = running.waiting().await;
            }
        });

        // `()` implements `ClientHandler` in rmcp, which gives it a
        // `Service<RoleClient>` impl. `serve(transport)` performs the
        // initialize handshake; the returned `RunningService` exposes
        // `peer().call_tool(...)`.
        let client = ().serve(client_io).await.expect("client initialize handshake");

        // Drop fixture's store handle — Round 1 fix moved seeding into the
        // `new_with_seed` closure, so the post-init handle is never read.
        drop(store);

        Self {
            _tempdir: tempdir,
            workspace_path,
            client,
            _server_task: server_task,
        }
    }

    /// Issue a `tools/call` JSON-RPC request and parse the JSON payload
    /// returned by the tool. Handlers serialize their response DTO via
    /// `serde_json::to_string_pretty` and wrap it as `Content::text(...)`,
    /// so the wire format is "stringified JSON inside content[0].text".
    pub async fn call_tool_json(&self, name: &'static str, args: Value) -> CallToolEnvelope {
        // `CallToolRequestParams` is `#[non_exhaustive]`, so we go through
        // its public builder rather than struct-literal init. `Map::new()`
        // for empty/null args matches the wire shape an MCP client emits
        // when a tool takes no parameters.
        let params = CallToolRequestParams::new(name).with_arguments(value_to_object(args));
        // Bound runtime so a regression that hangs the handler surfaces as
        // a panic with the offending tool name rather than a stuck CI job.
        let result = tokio::time::timeout(
            Duration::from_secs(15),
            self.client.peer().call_tool(params),
        )
        .await
        .unwrap_or_else(|_| panic!("tool `{name}` timed out (15s)"))
        .unwrap_or_else(|e| panic!("tool `{name}` rpc error: {e}"));

        CallToolEnvelope::from(result)
    }

    /// Issue a `tools/list` JSON-RPC request and return the paginated tool
    /// catalog. Used by tool-schema regression tests (e.g. PROB-065
    /// disambiguation) that need to assert on the description string an
    /// MCP client sees. Bounded by a 15s timeout to surface hangs as
    /// panics rather than stuck CI jobs.
    #[allow(dead_code)]
    pub async fn peer_list_all_tools(
        &self,
    ) -> Result<Vec<rmcp::model::Tool>, rmcp::service::ServiceError> {
        tokio::time::timeout(Duration::from_secs(15), self.client.peer().list_all_tools())
            .await
            .unwrap_or_else(|_| panic!("`tools/list` timed out (15s)"))
    }

    /// Helper: create a PRD via the live tool and return its display id.
    /// Only used from `integration_full_coverage.rs`, but lives here so
    /// the fixture stays a single source of truth.
    #[allow(dead_code)]
    pub async fn seed_prd(&self, title: &str) -> String {
        let env = self
            .call_tool_json(
                "forgeplan_new",
                serde_json::json!({"kind": "prd", "title": title}),
            )
            .await;
        let resp = env.assert_ok();
        resp["id"].as_str().expect("id present").to_string()
    }
}

/// Coerce a `serde_json::Value::Object` into the `JsonObject` shape rmcp
/// expects in `CallToolRequestParams.arguments`. Panics on non-object values
/// because every tool in this crate takes an object body.
pub fn value_to_object(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(map) => map,
        Value::Null => Map::new(),
        other => panic!("expected JSON object for tool args, got: {other}"),
    }
}

/// Lightly normalised view of `CallToolResult` so each test can read fields
/// without re-parsing JSON six times.
pub struct CallToolEnvelope {
    /// `is_error == Some(true)` from the handler. `false` for success.
    pub is_error: bool,
    /// Concatenated text of all `Content::Text` items in `content`.
    pub raw_text: String,
    /// Best-effort parse of `raw_text` into a JSON value. `None` when the
    /// handler returned a plain (non-JSON) error string — typical of
    /// `err_result(...)`.
    pub json: Option<Value>,
}

impl CallToolEnvelope {
    pub fn assert_ok(&self) -> &Value {
        assert!(
            !self.is_error,
            "expected success, got error: {}",
            self.raw_text
        );
        self.json.as_ref().unwrap_or_else(|| {
            panic!(
                "expected JSON payload but content was non-JSON text: {}",
                self.raw_text
            )
        })
    }

    /// Accept EITHER a successful JSON payload OR an `is_error=true` envelope.
    /// Used by handlers that depend on optional pre-requisites (LLM provider,
    /// external file, pre-existing trash). The contract we pin here is "the
    /// handler is reachable, well-formed args don't panic, and the response
    /// shape is parseable" — not "the underlying operation succeeded".
    #[allow(dead_code)]
    pub fn assert_reachable(&self) {
        // No panic from server, no transport error, and we got SOME body.
        assert!(
            !self.raw_text.is_empty(),
            "tool returned empty content — likely panicked: {self:?}"
        );
    }
}

impl std::fmt::Debug for CallToolEnvelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallToolEnvelope")
            .field("is_error", &self.is_error)
            .field("raw_text", &self.raw_text)
            .field("json_is_some", &self.json.is_some())
            .finish()
    }
}

impl From<CallToolResult> for CallToolEnvelope {
    fn from(r: CallToolResult) -> Self {
        let is_error = r.is_error.unwrap_or(false);
        let mut raw_text = String::new();
        for c in &r.content {
            if let RawContent::Text(t) = &c.raw {
                raw_text.push_str(&t.text);
            }
        }
        let json = serde_json::from_str::<Value>(&raw_text).ok();
        Self {
            is_error,
            raw_text,
            json,
        }
    }
}
