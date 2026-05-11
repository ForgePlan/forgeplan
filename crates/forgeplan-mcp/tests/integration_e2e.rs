//! Phase 2.4 — MCP server end-to-end integration tests.
//!
//! Why this file exists
//! --------------------
//! Pre-Phase-2.4 the `forgeplan-mcp` crate had three integration test files
//! totaling ~12 tests:
//!   * `server_capabilities.rs` — `get_info()`-only smoke tests.
//!   * `fpf_search_handler.rs`  — direct call into the `pub` FPF handler with
//!     no JSON-RPC wire involvement.
//!   * `soft_delete_integration.rs` — `forgeplan_core::undo` primitives, no
//!     MCP server in the loop.
//!
//! None of them booted the real `ForgeplanServer`, opened a JSON-RPC transport,
//! issued `initialize`, and exchanged `tools/call` requests. PR #268 (PROB-060
//! Phase 2 ID-assignment) added eight new tools and a brand new response shape
//! (`slug` + `predicted_number` + `assigned_number` + `id_canonical` +
//! `id_display` + `hint` triple) plus slug-aware methodology hints — none of
//! which were exercised end-to-end.
//!
//! These tests close that gap. They:
//!   1. Spin up a real `ForgeplanServer` rooted at a tempdir workspace.
//!   2. Pair it with an in-memory `tokio::io::duplex` JSON-RPC transport.
//!   3. Drive an `()`-handler client through the rmcp `ServiceExt` initialize
//!      handshake.
//!   4. Issue `peer.call_tool(...)` requests and parse the textual JSON
//!      payload returned in `CallToolResult.content[0]`.
//!
//! Test matrix (PROB-060 / SPEC-005 / ADR-012 Phase 2.4 — CD-2 / CD-5):
//!
//! | T#  | Tool                | Contract pinned                                 |
//! |-----|---------------------|-------------------------------------------------|
//! | T1  | forgeplan_new       | full identity triple in response                |
//! | T2  | forgeplan_get       | accepts slug input                              |
//! | T3  | forgeplan_get       | accepts display id input                        |
//! | T4  | forgeplan_list      | each item carries identity triple               |
//! | T5  | forgeplan_search    | each hit carries identity triple                |
//! | T6  | forgeplan_get       | pre-merge hint uses slug (assigned_number=null) |
//! | T7  | forgeplan_get       | post-merge hint uses display id                 |
//! | T8  | forgeplan_validate  | accepts display id (current contract)           |
//! | T9  | forgeplan_score     | returns R_eff for display id                    |
//! | T10 | forgeplan_link      | typed relation creates link via display ids     |
//! | T11 | forgeplan_health    | runs on a fresh workspace without panicking     |
//! | T12 | multi-tool flow     | new → validate → score → list chain             |
//! | T13 | legacy artifact     | no-slug body falls back to lowercased id        |
//!
//! Pre-merge / legacy fixtures (T6 + T13) are seeded via
//! `LanceStore::create_artifact_for_test` (gated by the `test-helpers`
//! feature) so we can write bodies the server itself would never emit
//! (`assigned_number: null`, missing slug).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::workspace;
use forgeplan_mcp::ForgeplanServer;
use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, CallToolResult, RawContent};
use serde_json::{Map, Value, json};
use tempfile::TempDir;

// ── harness ───────────────────────────────────────────────────────────

/// Live in-process MCP fixture: tempdir-rooted server, paired with a
/// `()`-handler client over a `tokio::io::duplex` transport. The fixture
/// owns the tempdir so dropping the test releases all temp state.
struct McpFixture {
    _tempdir: TempDir,
    workspace_path: PathBuf,
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
    async fn new() -> Self {
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
    async fn new_with_seed<F, Fut>(seed: F) -> Self
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
    async fn call_tool_json(&self, name: &'static str, args: Value) -> CallToolEnvelope {
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
}

/// Coerce a `serde_json::Value::Object` into the `JsonObject` shape rmcp
/// expects in `CallToolRequestParams.arguments`. Panics on non-object values
/// because every tool in this crate takes an object body.
fn value_to_object(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(map) => map,
        Value::Null => Map::new(),
        other => panic!("expected JSON object for tool args, got: {other}"),
    }
}

/// Lightly normalised view of `CallToolResult` so each test can read fields
/// without re-parsing JSON six times.
struct CallToolEnvelope {
    /// `is_error == Some(true)` from the handler. `false` for success.
    is_error: bool,
    /// Concatenated text of all `Content::Text` items in `content`.
    raw_text: String,
    /// Best-effort parse of `raw_text` into a JSON value. `None` when the
    /// handler returned a plain (non-JSON) error string — typical of
    /// `err_result(...)`.
    json: Option<Value>,
}

impl CallToolEnvelope {
    fn assert_ok(&self) -> &Value {
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

/// Build a frontmatter+body string with explicit `assigned_number: null`
/// — the Phase 2 "pre-merge" form that the CI bot will eventually mint.
/// `forgeplan_new` itself never emits this shape today (Phase 1.x sets
/// `assigned_number = predicted_number` immediately), so the only way to
/// land it in storage is via the `test-helpers` create path.
fn body_with_null_assigned(slug: &str, kind: &str, predicted: u32, title: &str) -> String {
    let display = format!("{}-{predicted}?", kind.to_uppercase());
    format!(
        "---\n\
id: {display}\n\
slug: \"{slug}\"\n\
predicted_number: {predicted}\n\
assigned_number: null\n\
title: \"{title}\"\n\
status: draft\n\
---\n\
\n\
# {display}: {title}\n\
\n\
Body for the pre-merge fixture.\n"
    )
}

/// Build a body with no `slug` / `predicted_number` / `assigned_number`
/// fields at all — the "legacy" pre-Phase-2 shape. `identity_from_record`
/// must surface `slug = None` and `id_canonical = lowercased id`.
fn body_legacy(id: &str, title: &str) -> String {
    format!(
        "---\n\
id: {id}\n\
title: \"{title}\"\n\
status: active\n\
---\n\
\n\
# {id}: {title}\n\
\n\
Legacy body without identity fields.\n"
    )
}

// ── T1: forgeplan_new full identity triple ────────────────────────────

/// PROB-060 Phase 2 W1.A (CD-2 binding): a fresh artifact response carries
/// the slug + predicted + assigned + id_canonical + id_display triple in
/// addition to the legacy `id` field, plus a `hint` narrating which form
/// to put into commit `Refs:`.
#[tokio::test]
async fn t1_forgeplan_new_returns_full_identity_triple() {
    let fx = McpFixture::new().await;

    let env = fx
        .call_tool_json("forgeplan_new", json!({"kind": "prd", "title": "Test PRD"}))
        .await;
    let resp = env.assert_ok();

    // Legacy fields preserved exactly.
    assert_eq!(resp["id"], "PRD-001", "first PRD must be PRD-001");
    assert_eq!(resp["kind"], "prd");
    assert_eq!(resp["title"], "Test PRD");
    assert!(
        resp["filepath"].as_str().unwrap_or("").ends_with(".md"),
        "filepath must point at a markdown projection: {}",
        resp["filepath"]
    );

    // CD-2 identity triple. slug derives from kind + title via slugify.
    assert_eq!(
        resp["slug"], "prd-test-prd",
        "slug = `prd-` + slugified title"
    );
    assert_eq!(resp["predicted_number"], 1);
    // Phase 1.x: assigned_number == predicted_number (immediate stamp).
    // The Phase-2 CI bot will eventually emit `null` here on create — the
    // `_next_action`/`hint` test below covers the null branch via fixture.
    assert_eq!(resp["assigned_number"], 1);
    assert_eq!(resp["id_canonical"], "prd-test-prd");
    assert_eq!(
        resp["id_display"], "PRD-001",
        "post-merge display form: zero-padded, no `?`"
    );

    // Identity-explainer hint should reference the canonical slug + display.
    let hint = resp["hint"].as_str().expect("hint string");
    assert!(
        hint.contains("PRD-001"),
        "post-merge hint mentions display id: {hint}"
    );
    assert!(
        hint.contains("prd-test-prd"),
        "post-merge hint mentions slug: {hint}"
    );

    // Methodology hint follows the post-merge contract: display-id form is
    // canonical (because Phase 1.x auto-assigns).
    let next = resp["_next_action"].as_str().expect("_next_action string");
    assert!(
        next.contains("PRD-001") && next.contains("validate"),
        "_next_action chains to validate via display id: {next}"
    );
}

// ── T2: forgeplan_get accepts slug ────────────────────────────────────

/// CD-2: `forgeplan_get` resolves a slug input via `LanceStore::resolve_id`
/// and returns the same record it would for the display id. Same identity
/// triple on the way out.
#[tokio::test]
async fn t2_forgeplan_get_accepts_slug() {
    let fx = McpFixture::new().await;

    fx.call_tool_json("forgeplan_new", json!({"kind": "prd", "title": "Test PRD"}))
        .await
        .assert_ok();

    let env = fx
        .call_tool_json("forgeplan_get", json!({"id": "prd-test-prd"}))
        .await;
    let resp = env.assert_ok();

    // The DB-canonical id is the display form (Phase 1.x).
    assert_eq!(resp["id"], "PRD-001");
    assert_eq!(resp["slug"], "prd-test-prd");
    assert_eq!(resp["id_canonical"], "prd-test-prd");
    assert_eq!(resp["id_display"], "PRD-001");
    // body must be present — `forgeplan_get` returns the full record.
    assert!(
        resp["body"].as_str().unwrap_or("").contains("Test PRD"),
        "body must round-trip: {}",
        resp["body"]
    );
}

// ── T3: forgeplan_get accepts display id ──────────────────────────────

/// CD-2 / ADR-012 invariant I-3: lookup accepts both forms and resolves to
/// the same canonical artifact.
#[tokio::test]
async fn t3_forgeplan_get_accepts_display_id() {
    let fx = McpFixture::new().await;

    fx.call_tool_json(
        "forgeplan_new",
        json!({"kind": "rfc", "title": "Sample RFC"}),
    )
    .await
    .assert_ok();

    // Display-id form, also exercising the `resolve_id` upper-case
    // normalisation path: lowercase prefix → uppercase canonical.
    let env = fx
        .call_tool_json("forgeplan_get", json!({"id": "rfc-001"}))
        .await;
    let resp = env.assert_ok();
    assert_eq!(resp["id"], "RFC-001");
    assert_eq!(resp["slug"], "rfc-sample-rfc");
    assert_eq!(resp["id_display"], "RFC-001");

    // Mixed-case must also resolve.
    let env2 = fx
        .call_tool_json("forgeplan_get", json!({"id": "Rfc-1"}))
        .await;
    let resp2 = env2.assert_ok();
    assert_eq!(resp2["id"], "RFC-001");
}

// ── T4: forgeplan_list returns identity per item ──────────────────────

/// Each `ListResponse.artifacts` item carries the full identity triple so
/// agents can pick a hit and immediately use the slug in commit refs
/// without a second `forgeplan_get` round-trip.
#[tokio::test]
async fn t4_forgeplan_list_returns_identity_per_item() {
    let fx = McpFixture::new().await;

    for title in ["Auth System", "Billing Tile", "Search Page"] {
        fx.call_tool_json("forgeplan_new", json!({"kind": "prd", "title": title}))
            .await
            .assert_ok();
    }

    let env = fx
        .call_tool_json("forgeplan_list", json!({"kind": "prd"}))
        .await;
    let resp = env.assert_ok();

    let items = resp["artifacts"].as_array().expect("artifacts array");
    assert_eq!(items.len(), 3, "three PRDs created, three listed");
    assert_eq!(resp["total"], 3);

    for item in items {
        assert!(item["id"].is_string(), "id present: {item}");
        // CD-2: identity triple per item.
        assert!(
            item["slug"].is_string(),
            "slug present (Phase 1.x augments frontmatter on create): {item}"
        );
        assert!(
            item["id_canonical"].is_string() && !item["id_canonical"].as_str().unwrap().is_empty(),
            "id_canonical always populated: {item}"
        );
        assert!(
            item["id_display"].is_string() && !item["id_display"].as_str().unwrap().is_empty(),
            "id_display always populated: {item}"
        );
    }

    // Spot-check one slug to ensure slugify wired through.
    let auth = items
        .iter()
        .find(|a| a["title"] == "Auth System")
        .expect("Auth System present");
    assert_eq!(auth["slug"], "prd-auth-system");
    assert_eq!(auth["id_canonical"], "prd-auth-system");
}

// ── T5: forgeplan_search returns identity per hit ─────────────────────

/// Search hits carry the identity triple so a follow-up tool call can use
/// the slug directly. Exercises both the smart-mode path (default) and
/// pins that the augmented frontmatter is reachable through search.
#[tokio::test]
async fn t5_forgeplan_search_returns_identity_per_hit() {
    let fx = McpFixture::new().await;

    fx.call_tool_json(
        "forgeplan_new",
        json!({"kind": "prd", "title": "Authentication Service"}),
    )
    .await
    .assert_ok();
    fx.call_tool_json(
        "forgeplan_new",
        json!({"kind": "prd", "title": "Billing Pipeline"}),
    )
    .await
    .assert_ok();

    // Smart mode (default) — should at minimum return our two artifacts.
    let env = fx
        .call_tool_json(
            "forgeplan_search",
            json!({"query": "Authentication", "limit": 5}),
        )
        .await;
    let resp = env.assert_ok();
    let results = resp["results"].as_array().expect("results array");
    assert!(
        !results.is_empty(),
        "smart search must surface at least one hit for `Authentication`: {resp}"
    );

    // Every hit carries identity fields. The schema is `Option<>` so legacy
    // hits would emit `slug = null` — for fresh artifacts we expect the
    // augmented form, but we assert on `id_canonical`/`id_display` which
    // are unconditionally populated.
    for hit in results {
        assert!(
            hit["id"].is_string() && !hit["id"].as_str().unwrap().is_empty(),
            "hit must carry id: {hit}"
        );
        assert!(
            hit["id_canonical"].is_string() && !hit["id_canonical"].as_str().unwrap().is_empty(),
            "hit must carry id_canonical: {hit}"
        );
        assert!(
            hit["id_display"].is_string() && !hit["id_display"].as_str().unwrap().is_empty(),
            "hit must carry id_display: {hit}"
        );
    }
}

// ── T6: pre-merge — slug-aware hint ───────────────────────────────────

/// W1.B / CD-5: when `assigned_number: null` (Phase-2 pre-merge form),
/// `forgeplan_get`'s `_next_action` hint must use the **slug**, not the
/// `?`-marked predicted display id, so that downstream agents stamp the
/// correct ref form into commit messages.
///
/// We seed the artifact directly via `create_artifact_for_test` because
/// the live MCP `forgeplan_new` path always sets `assigned_number` to the
/// predicted number in Phase 1.x; only the future CI-bot flow emits the
/// null shape.
#[tokio::test]
async fn t6_forgeplan_get_pre_merge_hint_uses_slug() {
    // Round 1 fix: use `new_with_seed` so seeding happens BEFORE server
    // opens its own LanceStore handle (which captures a versioned snapshot).
    // Post-init seeding via `forgeplan_list` refresh не работает на
    // current LanceStore caching layer — server's handle persists pre-seed
    // snapshot. The fixture's `new_with_seed` was designed для exactly
    // this case (см. fixture docstring).
    let fx = McpFixture::new_with_seed(|store| async move {
        let body = body_with_null_assigned("prd-pre-merge-feature", "prd", 7, "Pre Merge Feature");
        store
            .create_artifact_for_test(&NewArtifact {
                id: "PRD-007".into(),
                kind: "prd".into(),
                status: "draft".into(),
                title: "Pre Merge Feature".into(),
                body,
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .expect("seed pre-merge artifact");
    })
    .await;

    let env = fx
        .call_tool_json("forgeplan_get", json!({"id": "PRD-007"}))
        .await;
    let resp = env.assert_ok();

    assert_eq!(resp["slug"], "prd-pre-merge-feature");
    assert!(
        resp["assigned_number"].is_null(),
        "fixture preserves explicit null: {}",
        resp["assigned_number"]
    );
    assert_eq!(
        resp["id_display"], "PRD-7?",
        "pre-merge display form carries `?` marker"
    );
    assert_eq!(resp["id_canonical"], "prd-pre-merge-feature");

    // CD-5: the next-action hint uses the slug, not `PRD-7?`. Otherwise
    // an agent would commit `Refs: PRD-7?` which is a broken pointer.
    let next = resp["_next_action"].as_str().expect("_next_action present");
    assert!(
        next.contains("prd-pre-merge-feature"),
        "pre-merge hint must reference the slug: {next}"
    );
    assert!(
        !next.contains("PRD-7?"),
        "pre-merge hint must NOT reference the predicted display id: {next}"
    );
}

// ── T7: post-merge — display-id hint ──────────────────────────────────

/// Mirror of T6 for the post-merge case: when `assigned_number` is set to
/// a u32, the hint uses the zero-padded display id (`PRD-007`). This is
/// the Phase 1.x default, so the live `forgeplan_new` path produces it
/// without a fixture.
#[tokio::test]
async fn t7_forgeplan_get_post_merge_hint_uses_display_id() {
    let fx = McpFixture::new().await;

    fx.call_tool_json(
        "forgeplan_new",
        json!({"kind": "prd", "title": "Stable Feature"}),
    )
    .await
    .assert_ok();

    let env = fx
        .call_tool_json("forgeplan_get", json!({"id": "PRD-001"}))
        .await;
    let resp = env.assert_ok();

    assert_eq!(resp["assigned_number"], 1, "Phase 1.x auto-assigns");
    assert_eq!(resp["id_display"], "PRD-001");

    // For active/draft transitions the hint chains to validate; both
    // variants put the display id (post-merge ref form) into the command.
    let next = resp["_next_action"].as_str().expect("_next_action present");
    assert!(
        next.contains("PRD-001"),
        "post-merge hint must reference the display id: {next}"
    );
}

// ── T8: forgeplan_validate via display id ─────────────────────────────

/// `forgeplan_validate` accepts the artifact's display id and returns a
/// structured `ValidateResponse`. The handler currently does NOT route
/// through `resolve_id`, so this test pins the existing contract: display
/// id input works, slug input lands as "not found" today.
#[tokio::test]
async fn t8_forgeplan_validate_via_display_id() {
    let fx = McpFixture::new().await;

    fx.call_tool_json(
        "forgeplan_new",
        json!({"kind": "prd", "title": "Validation Subject"}),
    )
    .await
    .assert_ok();

    let env = fx
        .call_tool_json("forgeplan_validate", json!({"id": "PRD-001"}))
        .await;
    let resp = env.assert_ok();
    assert_eq!(resp["total_artifacts"], 1, "scoped to the requested id");
    let results = resp["results"].as_array().expect("results array");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["artifact_id"], "PRD-001");

    // Slug input is NOT yet resolved by validate — pin current behaviour
    // so any future change is intentional and tested.
    let slug_env = fx
        .call_tool_json(
            "forgeplan_validate",
            json!({"id": "prd-validation-subject"}),
        )
        .await;
    assert!(
        slug_env.is_error,
        "validate via slug not yet supported — pinning current contract: {}",
        slug_env.raw_text
    );
}

// ── T9: forgeplan_score returns R_eff for display id ──────────────────

/// `forgeplan_score` returns a numeric `r_eff` even when no evidence is
/// linked (R_eff defaults to 0.0 in that case). Pins that the score path
/// reaches the artifact and emits the F-G-R breakdown.
#[tokio::test]
async fn t9_forgeplan_score_returns_r_eff_for_display_id() {
    let fx = McpFixture::new().await;

    fx.call_tool_json(
        "forgeplan_new",
        json!({"kind": "prd", "title": "Scoring Subject"}),
    )
    .await
    .assert_ok();

    let env = fx
        .call_tool_json("forgeplan_score", json!({"id": "PRD-001"}))
        .await;
    let resp = env.assert_ok();
    assert_eq!(resp["id"], "PRD-001");
    assert!(
        resp["r_eff"].is_number(),
        "r_eff must be numeric: {}",
        resp["r_eff"]
    );
    assert!(
        resp["evidence"].is_array(),
        "evidence breakdown is always an array (possibly empty)"
    );
    // F-G-R fields must be present — they're part of the v0.10+ contract.
    for field in ["self_score", "formality", "granularity", "reliability"] {
        assert!(
            resp[field].is_number(),
            "F-G-R field `{field}` must be numeric: {}",
            resp[field]
        );
    }
    assert!(
        resp["overall_grade"].is_string(),
        "overall_grade letter must be present"
    );
}

// ── T10: forgeplan_link with typed relation ───────────────────────────

/// `forgeplan_link` resolves source + target by display id (current Phase
/// 2.4 contract — slug resolution lives only in `forgeplan_get`) and
/// creates a typed link surfaced through both `LinkResponse.message` and
/// the `_next_action` chaining hint.
#[tokio::test]
async fn t10_forgeplan_link_typed_relation() {
    let fx = McpFixture::new().await;

    fx.call_tool_json(
        "forgeplan_new",
        json!({"kind": "prd", "title": "Linkable PRD"}),
    )
    .await
    .assert_ok();
    fx.call_tool_json(
        "forgeplan_new",
        json!({"kind": "rfc", "title": "Linkable RFC"}),
    )
    .await
    .assert_ok();

    let env = fx
        .call_tool_json(
            "forgeplan_link",
            json!({
                "source": "PRD-001",
                "target": "RFC-001",
                "relation": "informs",
            }),
        )
        .await;
    let resp = env.assert_ok();
    let msg = resp["message"].as_str().expect("message string");
    assert!(
        msg.contains("PRD-001") && msg.contains("RFC-001") && msg.contains("informs"),
        "link confirmation must reference both ids and the relation: {msg}"
    );
    let next = resp["_next_action"].as_str().expect("_next_action present");
    assert!(
        next.contains("forgeplan_score_all") || next.contains("forgeplan_score"),
        "informs/based_on chains into the scoring reconciliation step: {next}"
    );
}

// ── T11: forgeplan_health on a fresh workspace ────────────────────────

/// `forgeplan_health` runs without panicking on a freshly initialized
/// workspace (no artifacts, no evidence). The current contract returns a
/// JSON object describing the workspace state — we don't pin specific
/// numbers (those evolve with new health checks) but we DO pin that the
/// call returns success.
#[tokio::test]
async fn t11_forgeplan_health_on_fresh_workspace() {
    let fx = McpFixture::new().await;

    let env = fx.call_tool_json("forgeplan_health", json!({})).await;
    // Health succeeded without a panic and returned some response body.
    assert!(
        !env.is_error,
        "fresh workspace must not yield a tool error from health: {}",
        env.raw_text
    );
    assert!(
        !env.raw_text.is_empty(),
        "health response must carry text content"
    );
}

// ── T12: multi-tool conversation flow ─────────────────────────────────

/// End-to-end chain: a single client connection issues new → validate →
/// score → list. Each step independently returns success and consults
/// state mutated by the previous step. This is the closest analogue to
/// what an LLM agent does in practice.
#[tokio::test]
async fn t12_multi_tool_flow_new_validate_score_list() {
    let fx = McpFixture::new().await;

    // Step 1 — create.
    let new_env = fx
        .call_tool_json(
            "forgeplan_new",
            json!({"kind": "prd", "title": "Multi Tool"}),
        )
        .await;
    let new_resp = new_env.assert_ok();
    assert_eq!(new_resp["id"], "PRD-001");

    // Step 2 — validate the freshly-created draft.
    let val_env = fx
        .call_tool_json("forgeplan_validate", json!({"id": "PRD-001"}))
        .await;
    let val_resp = val_env.assert_ok();
    assert_eq!(val_resp["total_artifacts"], 1);

    // Step 3 — score (no evidence yet, so r_eff is 0).
    let score_env = fx
        .call_tool_json("forgeplan_score", json!({"id": "PRD-001"}))
        .await;
    let score_resp = score_env.assert_ok();
    assert_eq!(score_resp["id"], "PRD-001");

    // Step 4 — list reflects all three tool calls before it.
    let list_env = fx.call_tool_json("forgeplan_list", json!({})).await;
    let list_resp = list_env.assert_ok();
    let items = list_resp["artifacts"].as_array().expect("artifacts array");
    assert!(
        items.iter().any(|a| a["id"] == "PRD-001"),
        "PRD-001 must be visible after the chain: {list_resp}"
    );
}

// ── T13: legacy artifact (no slug field) handled gracefully ───────────

/// CD-2 fallback rule: artifacts that pre-date Phase 1 frontmatter (no
/// `slug` / `predicted_number` / `assigned_number`) MUST surface as:
///   * `slug = None`               (omitted from JSON via skip_serializing_if)
///   * `id_canonical = lowercased display id` (legacy fallback)
///   * `id_display = original id`  (no `?`, no zero-padding)
/// so that downstream tools never null-pointer on missing fields.
#[tokio::test]
async fn t13_legacy_artifact_handled_gracefully() {
    // Round 1 fix: see T6 — seed BEFORE server boot via `new_with_seed`.
    let fx = McpFixture::new_with_seed(|store| async move {
        let legacy_body = body_legacy("PRD-099", "Legacy Artifact");
        store
            .create_artifact_for_test(&NewArtifact {
                id: "PRD-099".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "Legacy Artifact".into(),
                body: legacy_body,
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .expect("seed legacy artifact");
    })
    .await;

    // forgeplan_get must surface the legacy fallback shape.
    let get_env = fx
        .call_tool_json("forgeplan_get", json!({"id": "PRD-099"}))
        .await;
    let get_resp = get_env.assert_ok();
    assert_eq!(get_resp["id"], "PRD-099");
    assert!(
        get_resp.get("slug").map(|v| v.is_null()).unwrap_or(true),
        "legacy artifact slug is None or absent: {get_resp}"
    );
    assert_eq!(
        get_resp["id_canonical"], "prd-099",
        "legacy fallback: lowercased display id"
    );
    assert_eq!(
        get_resp["id_display"], "PRD-099",
        "legacy fallback: verbatim display id"
    );

    // forgeplan_list must include the legacy artifact with the same fallback.
    let list_env = fx
        .call_tool_json("forgeplan_list", json!({"kind": "prd"}))
        .await;
    let list_resp = list_env.assert_ok();
    let items = list_resp["artifacts"].as_array().expect("artifacts");
    let legacy = items
        .iter()
        .find(|a| a["id"] == "PRD-099")
        .expect("legacy listed");
    assert!(
        legacy.get("slug").map(|v| v.is_null()).unwrap_or(true),
        "legacy slug is None or absent in list: {legacy}"
    );
    assert_eq!(legacy["id_canonical"], "prd-099");
    assert_eq!(legacy["id_display"], "PRD-099");
}

// ── MED-1: phase_mismatches dual-key emission on MCP surface ──────────

/// w4-security-audit MED-1 (inverse PROB-064): pre-fix MCP
/// `forgeplan_health` emitted only the canonical
/// `advisory_phase_mismatches` key; agents authored against legacy CLI
/// (which still carried `phase_mismatches`) saw `null` when port'ing
/// their branching logic to MCP. After the fix the MCP surface emits
/// the same payload under both names — sourced from a single binding
/// so drift between the two keys is impossible by construction.
///
/// Mirrors the symmetric CLI guard
/// `health_json_phase_mismatches_aliases_have_identical_payload` in
/// `crates/forgeplan-cli/tests/cli_integration_test.rs:3807-3846`. The
/// CLI test pins identity on the CLI's `--json` surface; this one pins
/// it on the MCP `forgeplan_health` surface. Both together form the
/// cross-surface contract.
///
/// We do not pin specific phase_mismatches content (depends on workspace
/// config + phase-advance heuristics that evolve); identity + presence
/// is sufficient for the regression contract.
#[tokio::test]
async fn health_response_emits_both_phase_mismatches_aliases_identical_payload() {
    let fx = McpFixture::new().await;

    // Add a couple of artifacts so the emitter executes the non-trivial
    // path (matches the CLI test which также seeds two artifacts).
    let _ = fx
        .call_tool_json(
            "forgeplan_new",
            json!({"kind": "prd", "title": "Aliasing Feature"}),
        )
        .await;
    let _ = fx
        .call_tool_json(
            "forgeplan_new",
            json!({"kind": "note", "title": "Aliasing Note"}),
        )
        .await;

    let env = fx.call_tool_json("forgeplan_health", json!({})).await;
    let resp = env.assert_ok();

    let legacy = &resp["phase_mismatches"];
    let canonical = &resp["advisory_phase_mismatches"];

    // PROB-064 (inverse): both keys MUST surface — pre-fix only the
    // canonical key existed and the legacy alias resolved to `null`,
    // exactly the failure mode the audit caught.
    assert!(
        !legacy.is_null(),
        "PROB-064 inverse: legacy `phase_mismatches` MUST be present on \
         MCP surface (was null pre-fix). response = {resp}"
    );
    assert!(
        !canonical.is_null(),
        "PROB-064: canonical `advisory_phase_mismatches` MUST remain \
         present. response = {resp}"
    );

    // PROB-064 alias contract (mirrors CLI assertion verbatim — see
    // cli_integration_test.rs:3841-3845).
    assert_eq!(
        legacy, canonical,
        "PROB-064: phase_mismatches and advisory_phase_mismatches must carry \
         identical payloads (alias contract). legacy = {legacy}, canonical = {canonical}"
    );

    // Type guard — consumers branch on `.length`; an empty workspace
    // gives `[]` but never `null` / object.
    assert!(
        legacy.is_array(),
        "phase_mismatches MUST be a JSON array, got: {legacy}"
    );
    assert!(
        canonical.is_array(),
        "advisory_phase_mismatches MUST be a JSON array, got: {canonical}"
    );
}

// ── housekeeping: workspace path discoverable ─────────────────────────

/// Sanity: the fixture's own `init_workspace` succeeded and produced a
/// `.forgeplan/` directory. This check defends against silent test-harness
/// rot (a future refactor that breaks `init_workspace` would otherwise
/// only show up as cascading failures across every other test).
#[tokio::test]
async fn fixture_workspace_is_initialized() {
    let fx = McpFixture::new().await;
    assert!(
        fx.workspace_path.exists(),
        "fixture workspace path must exist: {}",
        fx.workspace_path.display()
    );
    assert!(
        fx.workspace_path.join("config.yaml").exists(),
        "config.yaml landed in fresh workspace"
    );
}
