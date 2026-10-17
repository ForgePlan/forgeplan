//! Constellation read-profile server (stage 5).
//!
//! `forgeplan serve --profile constellation-read` exposes a read-only MCP
//! surface: nothing here can mutate a source, and nothing accepts a raw
//! filesystem path. The allowlist is the registry — a scope handle is just
//! "the sources of constellation X resolved once", a snapshot handle is a
//! recorded pin; both live server-side for the life of the process, and an
//! agent refers to them by the opaque id it was handed. `use` in another
//! terminal cannot widen a running process's scope — the set is bound at
//! `open` time, per spec.
//!
//! Handles expire when closed or when the process exits: after that the
//! tool answers `snapshot_expired` / `unknown_handle`, and there is no way
//! to re-read fresh bytes under a dead handle.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use forgeplan_core::constellation::context_pack;
use forgeplan_core::constellation::federation::{self, FederatedGet};
use forgeplan_core::constellation::graph::{self, GraphBounds};
use forgeplan_core::constellation::impact::{self, ImpactBounds};
use forgeplan_core::constellation::registry;
use forgeplan_core::constellation::snapshot::{self, ConstellationSnapshot};
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content};
use rmcp::schemars::{self, JsonSchema};
use rmcp::{ErrorData as McpError, tool, tool_router};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::Mutex;

/// A scope the client opened: the frozen source list of one constellation.
struct ScopeHandle {
    name: String,
    specs: Vec<federation::SourceSpec>,
}

enum Handle {
    Scope(ScopeHandle),
    Snapshot(Box<ConstellationSnapshot>),
}

/// The read-profile server. All state it keeps is the handle table —
/// handles live exactly as long as this process, per spec.
pub struct ConstellationReadServer {
    handles: Arc<Mutex<HashMap<String, Handle>>>,
    next_handle: Arc<AtomicU64>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpenParams {
    /// Constellation name. Omit for the active one.
    pub constellation: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HandleParams {
    /// A `scope-…` or `snap-…` handle returned by `open`/`snapshot`.
    pub handle: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadParams {
    /// Scope handle to read the live sources through, or snapshot handle
    /// to read the pinned revision. Exactly one is required.
    pub scope_handle: Option<String>,
    /// Snapshot handle — wins over `scope_handle` when both are given.
    pub snapshot_handle: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchParams {
    pub query: String,
    /// Maximum hits (default 10).
    pub limit: Option<usize>,
    pub scope_handle: Option<String>,
    pub snapshot_handle: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PackParams {
    /// What the agent must do — ranking terms come from this text.
    pub task: String,
    /// Token budget (estimated tokens, chars/4; default 8000).
    pub budget: Option<usize>,
    pub scope_handle: Option<String>,
    pub snapshot_handle: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImpactParams {
    /// The changed artifact's canonical key or display id.
    pub id: String,
    /// Maximum structural hops (default 4).
    pub depth: Option<usize>,
    /// Maximum impacted nodes (default 256).
    pub max_nodes: Option<usize>,
    pub scope_handle: Option<String>,
    pub snapshot_handle: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphParams {
    /// Maximum nodes and edges each (default 500).
    pub limit: Option<usize>,
    pub scope_handle: Option<String>,
    pub snapshot_handle: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetParams {
    /// Canonical key or display id; a `key@source-fragment` qualifier
    /// narrows the scope before reading.
    pub id: String,
    pub scope_handle: Option<String>,
    pub snapshot_handle: Option<String>,
}

fn ok_json(v: serde_json::Value) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string()),
    )]))
}

fn fail(reason: &str, detail: impl std::fmt::Display) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&json!({"error": reason, "detail": detail.to_string()}))
            .unwrap(),
    )]))
}

impl Default for ConstellationReadServer {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstellationReadServer {
    pub fn new() -> Self {
        Self {
            handles: Arc::new(Mutex::new(HashMap::new())),
            next_handle: Arc::new(AtomicU64::new(1)),
            tool_router: Self::tool_router(),
        }
    }

    fn issue(&self, prefix: &str) -> String {
        format!(
            "{prefix}-{}",
            self.next_handle.fetch_add(1, Ordering::SeqCst)
        )
    }
}

#[tool_router]
impl ConstellationReadServer {
    #[tool(
        description = "Open a read scope: resolve a named (or the active) constellation's                        source list once and return an opaque `scope-…` handle. Later reads                        use the handle — the source set cannot widen mid-session even if                        `constellation use` moves elsewhere.",
        annotations(
            title = "Constellation Open",
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false,
        )
    )]
    async fn constellation_open(
        &self,
        Parameters(p): Parameters<OpenParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = match registry::default_registry_path() {
            Ok(p) => p,
            Err(e) => return fail("io", e),
        };
        let registry = match registry::load(&path) {
            Ok(r) => r,
            Err(e) => return fail("io", e),
        };
        let name = match p.constellation.clone().or_else(|| registry.current.clone()) {
            Some(n) => n,
            None => return fail("no_scope", "no constellation selected and none is active"),
        };
        let specs = match registry.constellations.get(&name) {
            Some(def) => def
                .sources
                .iter()
                .map(|s| federation::SourceSpec::new(std::path::PathBuf::from(&s.path)))
                .collect::<Vec<_>>(),
            None => return fail("no_scope", format!("no constellation named {name:?}")),
        };
        let handle = self.issue("scope");
        let sources: Vec<serde_json::Value> = specs
            .iter()
            .map(|s| {
                json!({
                    "path": s.path.display().to_string(),
                    "source_id": federation::spec_source_id(s),
                })
            })
            .collect();
        self.handles.lock().await.insert(
            handle.clone(),
            Handle::Scope(ScopeHandle {
                name: name.clone(),
                specs,
            }),
        );
        ok_json(json!({
            "scope_handle": handle,
            "constellation": name,
            "sources": sources,
        }))
    }

    #[tool(
        description = "List the sources bound to a scope handle.",
        annotations(title = "Constellation Sources", read_only_hint = true)
    )]
    async fn constellation_sources(
        &self,
        Parameters(p): Parameters<HandleParams>,
    ) -> Result<CallToolResult, McpError> {
        let handles = self.handles.lock().await;
        match handles.get(&p.handle) {
            Some(Handle::Scope(scope)) => {
                let sources: Vec<serde_json::Value> = scope
                    .specs
                    .iter()
                    .map(|s| {
                        json!({
                            "path": s.path.display().to_string(),
                            "source_id": federation::spec_source_id(s),
                        })
                    })
                    .collect();
                ok_json(json!({
                    "scope_handle": p.handle,
                    "constellation": scope.name,
                    "sources": sources,
                }))
            }
            Some(Handle::Snapshot(_)) => fail(
                "wrong_handle_kind",
                "a snapshot handle has no live source list",
            ),
            None => fail("unknown_handle", &p.handle),
        }
    }

    #[tool(
        description = "Pin the scope's git sources at their current HEAD and return a                        `snap-…` handle. Non-git sources are recorded in the lockfile as                        skipped — a snapshot read reports them, it does not silently                        read them live.",
        annotations(title = "Constellation Snapshot", read_only_hint = true)
    )]
    async fn constellation_snapshot(
        &self,
        Parameters(p): Parameters<HandleParams>,
    ) -> Result<CallToolResult, McpError> {
        let snapshot = {
            let handles = self.handles.lock().await;
            match handles.get(&p.handle) {
                Some(Handle::Scope(scope)) => {
                    snapshot::capture_snapshot(&scope.specs, Some(&scope.name))
                }
                Some(Handle::Snapshot(_)) => {
                    return fail("wrong_handle_kind", "snapshot of a snapshot is not a thing");
                }
                None => return fail("unknown_handle", &p.handle),
            }
        };
        let snapshot = match snapshot {
            Ok(s) => s,
            Err(e) => return fail("io", e),
        };
        let handle = self.issue("snap");
        let pinned: Vec<serde_json::Value> = snapshot
            .sources
            .iter()
            .map(|e| {
                json!({
                    "checkout_root": e.checkout_root,
                    "git_oid": e.git_oid,
                    "files": e.files,
                })
            })
            .collect();
        let skipped: Vec<serde_json::Value> = snapshot
            .skipped
            .iter()
            .map(|s| json!({"path": s.path, "reason": s.reason}))
            .collect();
        self.handles
            .lock()
            .await
            .insert(handle.clone(), Handle::Snapshot(Box::new(snapshot)));
        ok_json(json!({
            "snapshot_handle": handle,
            "pinned": pinned,
            "skipped": skipped,
        }))
    }

    #[tool(
        description = "Lexical search across the scope's live sources or a snapshot's                        pinned revisions. Each hit names its source and the version it                        was read at.",
        annotations(title = "Constellation Search", read_only_hint = true)
    )]
    async fn constellation_search(
        &self,
        Parameters(p): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = p.limit.unwrap_or(10);
        let handles = self.handles.lock().await;
        if let Some(snap_handle) = &p.snapshot_handle {
            return match handles.get(snap_handle) {
                Some(Handle::Snapshot(snap)) => {
                    match snapshot::snapshot_search(snap, &p.query, limit) {
                        Ok(outcome) => ok_json(json!({
                            "snapshot_handle": snap_handle,
                            "hits": hit_json(&outcome.hits),
                            "sources": outcome.sources,
                            "failures": outcome.failures,
                            "partial": outcome.partial,
                            "truncated": outcome.truncated,
                        })),
                        Err(e) => fail("io", e),
                    }
                }
                Some(Handle::Scope(_)) => {
                    fail("wrong_handle_kind", "scope handle is not a snapshot")
                }
                None => fail("snapshot_expired", snap_handle),
            };
        }
        match p.scope_handle.as_deref().and_then(|h| handles.get(h)) {
            Some(Handle::Scope(scope)) => {
                match federation::federated_search(&scope.specs, &p.query, limit).await {
                    Ok(outcome) => ok_json(json!({
                        "scope_handle": p.scope_handle,
                        "hits": hit_json(&outcome.hits),
                        "sources": outcome.sources,
                        "failures": outcome.failures,
                        "partial": outcome.partial,
                        "truncated": outcome.truncated,
                    })),
                    Err(e) => fail("io", e),
                }
            }
            Some(Handle::Snapshot(_)) => {
                fail("wrong_handle_kind", "use snapshot_handle for a snapshot")
            }
            None => fail(
                "unknown_handle",
                p.scope_handle.unwrap_or_else(|| "(none given)".into()),
            ),
        }
    }

    #[tool(
        description = "Fetch one artifact by canonical key or display id. A `key@fragment`                        qualifier narrows the scope first; an unqualified id claimed by                        two sources answers `ambiguous` with the claimers.",
        annotations(title = "Constellation Get", read_only_hint = true)
    )]
    async fn constellation_get(
        &self,
        Parameters(p): Parameters<GetParams>,
    ) -> Result<CallToolResult, McpError> {
        let handles = self.handles.lock().await;
        if let Some(snap_handle) = &p.snapshot_handle {
            return match handles.get(snap_handle) {
                Some(Handle::Snapshot(snap)) => match snapshot::snapshot_get(snap, &p.id) {
                    Ok(found) => match found.len() {
                        0 => ok_json(json!({"result": "not_found", "id": p.id})),
                        1 => {
                            let (source, rel, bytes) = &found[0];
                            ok_json(json!({
                                "result": "found",
                                "id": p.id,
                                "source": source,
                                "relative_path": rel,
                                "body": String::from_utf8_lossy(bytes),
                            }))
                        }
                        _ => {
                            let claimers: Vec<serde_json::Value> = found
                                .iter()
                                .map(|(s, rel, _)| json!({"source": s, "relative_path": rel}))
                                .collect();
                            ok_json(
                                json!({"result": "ambiguous", "id": p.id, "claimers": claimers}),
                            )
                        }
                    },
                    Err(e) => fail("io", e),
                },
                Some(Handle::Scope(_)) => {
                    fail("wrong_handle_kind", "scope handle is not a snapshot")
                }
                None => fail("snapshot_expired", snap_handle),
            };
        }
        // Qualified refs: `key@fragment` narrows the scope before reading.
        let (bare, qualifier) = {
            let (k, q) = federation::parse_qualified_ref(&p.id);
            (k.to_string(), q.map(str::to_string))
        };
        match p.scope_handle.as_deref().and_then(|h| handles.get(h)) {
            Some(Handle::Scope(scope)) => {
                let mut specs = scope.specs.clone();
                if let Some(q) = &qualifier {
                    specs = federation::specs_matching(&specs, q)
                        .into_iter()
                        .cloned()
                        .collect();
                    if specs.is_empty() {
                        return fail(
                            "no_source",
                            format!("qualifier @{q} matches nothing in scope"),
                        );
                    }
                }
                match federation::federated_get(&specs, &bare).await {
                    Ok(FederatedGet::Found(a)) => ok_json(json!({
                        "result": "found",
                        "id": bare,
                        "canonical_key": a.canonical_key,
                        "display_id": a.display_id,
                        "title": a.title,
                        "kind": a.kind,
                        "status": a.status,
                        "semantic_state": a.semantic_state,
                        "body": a.body,
                        "source": a.source,
                    })),
                    Ok(FederatedGet::Ambiguous { sources }) => ok_json(json!({
                        "result": "ambiguous",
                        "id": bare,
                        "claimers": sources,
                    })),
                    Ok(FederatedGet::NotFound { failures }) => ok_json(json!({
                        "result": "not_found",
                        "id": bare,
                        "failures": failures,
                    })),
                    Err(e) => fail("io", e),
                }
            }
            Some(Handle::Snapshot(_)) => {
                fail("wrong_handle_kind", "use snapshot_handle for a snapshot")
            }
            None => fail(
                "unknown_handle",
                p.scope_handle.unwrap_or_else(|| "(none given)".into()),
            ),
        }
    }

    #[tool(
        description = "Build a budgeted ContextPack for a task over the scope's live                        sources or a snapshot's pinned revisions: lexically ranked                        evidence units with roles, named omissions and a spend receipt.",
        annotations(title = "Constellation Pack", read_only_hint = true)
    )]
    async fn constellation_pack(
        &self,
        Parameters(p): Parameters<PackParams>,
    ) -> Result<CallToolResult, McpError> {
        let budget = p.budget.unwrap_or(8000);
        let handles = self.handles.lock().await;
        if let Some(snap_handle) = &p.snapshot_handle {
            return match handles.get(snap_handle) {
                Some(Handle::Snapshot(snap)) => {
                    match context_pack::pack_snapshot(snap, &p.task, budget) {
                        Ok(pack) => ok_json(
                            serde_json::to_value(&pack)
                                .unwrap_or_else(|_| json!({"error":"serialize"})),
                        ),
                        Err(e) => fail("io", e),
                    }
                }
                Some(Handle::Scope(_)) => {
                    fail("wrong_handle_kind", "scope handle is not a snapshot")
                }
                None => fail("snapshot_expired", snap_handle),
            };
        }
        match p.scope_handle.as_deref().and_then(|h| handles.get(h)) {
            Some(Handle::Scope(scope)) => {
                match context_pack::pack_live(&scope.specs, &p.task, budget).await {
                    Ok(pack) => ok_json(
                        serde_json::to_value(&pack)
                            .unwrap_or_else(|_| json!({"error":"serialize"})),
                    ),
                    Err(e) => fail("io", e),
                }
            }
            Some(Handle::Snapshot(_)) => {
                fail("wrong_handle_kind", "use snapshot_handle for a snapshot")
            }
            None => fail(
                "unknown_handle",
                p.scope_handle.unwrap_or_else(|| "(none given)".into()),
            ),
        }
    }

    #[tool(
        description = "Explainable impact: who depends on the artifact, by which                        relation path, and where the bounded walk stopped. `based_on`                        and friends propagate transitively; `references`/`informs`                        only mark the direct neighbor.",
        annotations(title = "Constellation Impact", read_only_hint = true)
    )]
    async fn constellation_impact(
        &self,
        Parameters(p): Parameters<ImpactParams>,
    ) -> Result<CallToolResult, McpError> {
        let bounds = ImpactBounds {
            max_depth: p.depth.unwrap_or(4),
            max_nodes: p.max_nodes.unwrap_or(256),
        };
        let handles = self.handles.lock().await;
        if let Some(snap_handle) = &p.snapshot_handle {
            return match handles.get(snap_handle) {
                Some(Handle::Snapshot(snap)) => {
                    match impact::impact_snapshot(snap, &p.id, bounds) {
                        Ok(report) => ok_json(
                            serde_json::to_value(&report)
                                .unwrap_or_else(|_| json!({"error":"serialize"})),
                        ),
                        Err(e) => fail("io", e),
                    }
                }
                Some(Handle::Scope(_)) => {
                    fail("wrong_handle_kind", "scope handle is not a snapshot")
                }
                None => fail("snapshot_expired", snap_handle),
            };
        }
        match p.scope_handle.as_deref().and_then(|h| handles.get(h)) {
            Some(Handle::Scope(scope)) => {
                match impact::impact_live(&scope.specs, &p.id, bounds).await {
                    Ok(report) => ok_json(
                        serde_json::to_value(&report)
                            .unwrap_or_else(|_| json!({"error":"serialize"})),
                    ),
                    Err(e) => fail("io", e),
                }
            }
            Some(Handle::Snapshot(_)) => {
                fail("wrong_handle_kind", "use snapshot_handle for a snapshot")
            }
            None => fail(
                "unknown_handle",
                p.scope_handle.unwrap_or_else(|| "(none given)".into()),
            ),
        }
    }

    #[tool(
        description = "The relation graph of the scope's live sources or a snapshot's                        pinned revisions: every artifact as a source-qualified node,                        every declared link as a typed edge carrying its relation and                        the source/file that declared it. Bounded — `truncated` marks                        a view cut at the limit, `partial` marks sources that did not                        answer.",
        annotations(title = "Constellation Graph", read_only_hint = true)
    )]
    async fn constellation_graph(
        &self,
        Parameters(p): Parameters<GraphParams>,
    ) -> Result<CallToolResult, McpError> {
        let bounds = GraphBounds {
            limit: p.limit.unwrap_or(500),
        };
        let handles = self.handles.lock().await;
        if let Some(snap_handle) = &p.snapshot_handle {
            return match handles.get(snap_handle) {
                Some(Handle::Snapshot(snap)) => match graph::graph_snapshot(snap, bounds) {
                    Ok(outcome) => ok_json(
                        serde_json::to_value(&outcome)
                            .unwrap_or_else(|_| json!({"error":"serialize"})),
                    ),
                    Err(e) => fail("io", e),
                },
                Some(Handle::Scope(_)) => {
                    fail("wrong_handle_kind", "scope handle is not a snapshot")
                }
                None => fail("snapshot_expired", snap_handle),
            };
        }
        match p.scope_handle.as_deref().and_then(|h| handles.get(h)) {
            Some(Handle::Scope(scope)) => match graph::graph_live(&scope.specs, bounds).await {
                Ok(outcome) => ok_json(
                    serde_json::to_value(&outcome).unwrap_or_else(|_| json!({"error":"serialize"})),
                ),
                Err(e) => fail("io", e),
            },
            Some(Handle::Snapshot(_)) => {
                fail("wrong_handle_kind", "use snapshot_handle for a snapshot")
            }
            None => fail(
                "unknown_handle",
                p.scope_handle.unwrap_or_else(|| "(none given)".into()),
            ),
        }
    }

    #[tool(
        description = "Close a scope or snapshot handle. After closing, reads through                        it answer `snapshot_expired`/`unknown_handle` — there is no way                        to re-read fresh bytes under a dead handle.",
        annotations(
            title = "Constellation Close",
            read_only_hint = false,
            idempotent_hint = true
        )
    )]
    async fn constellation_close(
        &self,
        Parameters(p): Parameters<HandleParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut handles = self.handles.lock().await;
        match handles.remove(&p.handle) {
            Some(Handle::Scope(_)) => ok_json(json!({"closed": p.handle, "kind": "scope"})),
            Some(Handle::Snapshot(_)) => ok_json(
                json!({"closed": p.handle, "kind": "snapshot", "note": "reads now answer snapshot_expired"}),
            ),
            None => fail("unknown_handle", &p.handle),
        }
    }
}

fn hit_json(hits: &[federation::FederatedHit]) -> Vec<serde_json::Value> {
    hits.iter()
        .map(|h| {
            json!({
                "canonical_key": h.canonical_key,
                "display_id": h.display_id,
                "title": h.title,
                "kind": h.kind,
                "status": h.status,
                "semantic_state": h.semantic_state,
                "source": h.source,
            })
        })
        .collect()
}

/// Run the read-profile server over stdio.
pub async fn run_read_stdio() -> anyhow::Result<()> {
    let server = ConstellationReadServer::new();
    let service = rmcp::ServiceExt::serve(server, rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}

impl rmcp::ServerHandler for ConstellationReadServer {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo::new(
            rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(rmcp::model::Implementation::new(
            "forgeplan-constellation-read",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(
            "ForgePlan Constellation read profile: open a scope to bind a \
             constellation's sources, take snapshots pinned at Git OIDs, \
             and search/get through the handles you were handed. No tool \
             here writes, and none accepts a raw filesystem path.",
        )
    }

    async fn list_tools(
        &self,
        _params: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, McpError> {
        Ok(rmcp::model::ListToolsResult {
            next_cursor: None,
            tools: self.tool_router.list_all(),
            meta: Default::default(),
        })
    }

    async fn call_tool(
        &self,
        params: rmcp::model::CallToolRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tcc = rmcp::handler::server::tool::ToolCallContext::new(self, params, context);
        self.tool_router.call(tcc).await
    }
}
