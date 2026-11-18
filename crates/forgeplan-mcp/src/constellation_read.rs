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

use forgeplan_core::constellation::ConstellationError;
use forgeplan_core::constellation::context_pack;
use forgeplan_core::constellation::federation::{self, QueryMode};
use forgeplan_core::constellation::graph::{self, GraphBounds};
use forgeplan_core::constellation::impact::{self, ImpactBounds};
use forgeplan_core::constellation::registry;
use forgeplan_core::constellation::snapshot::{self, ConstellationSnapshot};
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, ProtocolVersion};
use rmcp::schemars::{self, JsonSchema};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, tool, tool_router};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::Mutex;

/// A scope the client opened: the frozen source list of one constellation.
#[derive(Clone)]
struct ScopeHandle {
    name: String,
    specs: Vec<federation::SourceSpec>,
    /// The byte identity each source answered at when the scope was opened.
    /// `None` means the source could not be read at open; that failure is
    /// reported by the first read rather than by `open`.
    versions: Vec<Option<String>>,
    source_ids: Vec<Option<String>>,
}
#[cfg(test)]
struct ReadPause {
    reached: tokio::sync::oneshot::Sender<()>,
    resume: tokio::sync::oneshot::Receiver<()>,
}

impl ScopeHandle {
    fn verify_read(
        &self,
        specs: &[federation::SourceSpec],
        sources: &[federation::SourceRef],
        failures: &[federation::SourceFailure],
    ) -> Result<(), String> {
        for spec in specs {
            let Some(index) = self
                .specs
                .iter()
                .position(|candidate| candidate.path == spec.path && candidate.role == spec.role)
            else {
                return Err(format!(
                    "source {} is not bound to this scope",
                    spec.path.display()
                ));
            };
            let version = &self.versions[index];
            let source_id = &self.source_ids[index];
            let served = sources.iter().find(|source| {
                source_id.as_deref() == Some(source.source_id.as_str())
                    || (source_id.is_none()
                        && spec
                            .path
                            .canonicalize()
                            .ok()
                            .is_some_and(|path| path.to_string_lossy() == source.path))
            });
            match (version, served) {
                (Some(recorded), Some(source))
                    if source.source_set_digest.as_deref() == Some(recorded.as_str()) => {}
                (Some(_), _) => {
                    return Err(format!(
                        "source {} changed or could not be verified against this scope; reopen the scope or take a snapshot",
                        spec.path.display()
                    ));
                }
                (None, Some(_)) => {
                    return Err(format!(
                        "source {} was not readable when this scope opened; reopen the scope before reading its current bytes",
                        spec.path.display()
                    ));
                }
                (None, None)
                    if failures.iter().any(|failure| {
                        failure.path == spec.path.display().to_string()
                            || spec
                                .path
                                .canonicalize()
                                .ok()
                                .is_some_and(|path| path.to_string_lossy() == failure.path)
                    }) => {}
                (None, None) => {
                    return Err(format!(
                        "source {} has no read witness for this scope; reopen the scope",
                        spec.path.display()
                    ));
                }
            }
        }
        Ok(())
    }
    async fn verify_failed_read(
        &self,
        specs: &[federation::SourceSpec],
        error: &ConstellationError,
    ) -> Result<(), String> {
        if let ConstellationError::ResolveRead {
            sources, failures, ..
        } = error
        {
            return self.verify_read(specs, sources, failures);
        }
        if !matches!(error, ConstellationError::SourceUnavailable { .. }) {
            return Ok(());
        }
        for spec in specs {
            let Some(index) = self
                .specs
                .iter()
                .position(|opened| opened.path == spec.path && opened.role == spec.role)
            else {
                return Err(format!(
                    "source {} is not part of this scope; reopen the scope",
                    spec.path.display()
                ));
            };
            let current = federation::live_source_version(spec).await.ok();
            if current != self.versions[index] {
                return Err(format!(
                    "source {} changed or could not be verified against this scope; reopen the scope or take a snapshot",
                    spec.path.display()
                ));
            }
        }
        Ok(())
    }
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
    #[cfg(test)]
    before_live_query: Arc<Mutex<Option<ReadPause>>>,
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
    /// Strict reads (default) fail with `source_unavailable` when a
    /// required source cannot answer; `false` accepts a partial answer.
    /// Ignored for snapshot reads — the lockfile's source set is frozen.
    pub strict: Option<bool>,
    pub scope_handle: Option<String>,
    pub snapshot_handle: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PackParams {
    /// What the agent must do — ranking terms come from this text.
    pub task: String,
    /// Estimated full-result allowance (UTF-8 bytes divided by four, rounded up).
    pub budget: Option<usize>,
    /// Exact cap on serialized MCP tool result bytes.
    pub byte_cap: Option<usize>,
    /// Explicit source-qualified artifact selectors.
    pub seeds: Option<Vec<String>>,
    /// Maximum relation depth (default 4, maximum 64).
    pub max_depth: Option<usize>,
    /// Maximum visited artifacts (default 256, maximum 4096).
    pub max_nodes: Option<usize>,
    /// Maximum declared edges (default 1024, maximum 16384).
    pub max_edges: Option<usize>,
    /// Maximum traversal milliseconds (default 5000, maximum 60000).
    pub max_millis: Option<u64>,
    /// Require complete sources, traversal and delivery in one result.
    pub require_complete: Option<bool>,
    /// Continuation from the previous response with unchanged inputs.
    pub cursor: Option<String>,
    /// Retrieval mode; only `lexical` is supported.
    pub retrieval_mode: Option<String>,
    /// Include Mermaid navigation for the declared graph.
    pub mermaid: Option<bool>,
    /// Strict reads (default) fail with `source_unavailable` when a
    /// required source cannot answer; `false` accepts a partial answer.
    pub strict: Option<bool>,
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
    /// Strict reads (default) fail with `source_unavailable` when a
    /// required source cannot answer; `false` accepts a partial answer.
    pub strict: Option<bool>,
    pub scope_handle: Option<String>,
    pub snapshot_handle: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphParams {
    /// Maximum nodes and edges each (default 500).
    pub limit: Option<usize>,
    /// Strict reads (default) fail with `source_unavailable` when a
    /// required source cannot answer; `false` accepts a partial answer.
    pub strict: Option<bool>,
    pub scope_handle: Option<String>,
    pub snapshot_handle: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetParams {
    /// Canonical key or display id; a `key@source-fragment` qualifier
    /// narrows the scope before reading.
    pub id: String,
    /// Strict reads (default) fail with `source_unavailable` when a
    /// required source cannot answer; `false` accepts a partial answer.
    pub strict: Option<bool>,
    pub scope_handle: Option<String>,
    pub snapshot_handle: Option<String>,
}

fn ok_json(v: serde_json::Value) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![ContentBlock::text(
        serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string()),
    )]))
}

fn pack_result(pack: &context_pack::ContextPack) -> Result<CallToolResult, String> {
    let text = serde_json::to_string(pack).map_err(|e| e.to_string())?;
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

fn pack_measure(pack: &context_pack::ContextPack, modern: bool) -> Result<usize, String> {
    let mut result = pack_result(pack)?;
    // ServerHandler strips complete resultType for protocol versions before
    // 2026-07-28; measure that same public result, not the pre-dispatch value.
    if !modern {
        result.result_type = None;
    }
    serde_json::to_vec(&result)
        .map(|bytes| bytes.len())
        .map_err(|e| e.to_string())
}

fn fail(reason: &str, detail: impl std::fmt::Display) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![ContentBlock::text(
        serde_json::to_string_pretty(&json!({"error": reason, "detail": detail.to_string()}))
            .unwrap(),
    )]))
}

/// The `strict` param translated: absent or `true` is a strict read, an
/// explicit `false` accepts a partial answer over dead sources.
fn mode_of(strict: Option<bool>) -> QueryMode {
    match strict {
        Some(false) => QueryMode::AllowPartial,
        _ => QueryMode::Strict,
    }
}

/// A failed live read: `source_unavailable` gets its own envelope reason
/// so a caller can tell "a required source is down" apart from ordinary
/// I/O trouble — the distinction the strict/partial contract exists for.
fn fail_read(e: ConstellationError) -> Result<CallToolResult, McpError> {
    fail_read_with_input(e, None)
}

fn fail_read_with_input(
    e: ConstellationError,
    requested_input: Option<&str>,
) -> Result<CallToolResult, McpError> {
    if let ConstellationError::Resolve(error) | ConstellationError::ResolveRead { error, .. } = &e {
        use forgeplan_core::artifact::canonical_key::ResolveError;
        let (input, candidates) = match error {
            ResolveError::Ambiguous { input, candidates } => {
                (Some(input.as_str()), Some(candidates))
            }
            ResolveError::ConflictingCanonicalKey { key, candidates } => {
                (Some(key.as_str()), Some(candidates))
            }
            ResolveError::InvalidSelector { input } => (Some(input.as_str()), None),
            ResolveError::IncompleteCapture { .. } => (None, None),
        };
        let candidates: Vec<_> = candidates
            .into_iter()
            .flatten()
            .map(|candidate| {
                json!({
                    "canonical_key": candidate.canonical_key,
                    "display_id": candidate.display_id,
                    "title": candidate.title,
                    "relative_path": candidate.relative_path,
                })
            })
            .collect();
        return Ok(CallToolResult::error(vec![ContentBlock::text(
            json!({"error": e.code().as_str(), "input": requested_input.or(input), "candidates": candidates, "detail": error.to_string()}).to_string(),
        )]));
    }
    // Every error already carries its SPEC-008 code; collapsing the typed
    // ones into "io" hides the only fact a caller can act on.
    fail(e.code().as_str(), &e)
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
            #[cfg(test)]
            before_live_query: Arc::new(Mutex::new(None)),
        }
    }

    fn issue(&self, prefix: &str) -> String {
        format!(
            "{prefix}-{}",
            self.next_handle.fetch_add(1, Ordering::SeqCst)
        )
    }
    #[cfg(test)]
    async fn pause_before_live_query(&self) {
        let pause = self.before_live_query.lock().await.take();
        if let Some(pause) = pause {
            let _ = pause.reached.send(());
            let _ = pause.resume.await;
        }
    }
    #[cfg(test)]
    async fn precheck_scope_for_test(
        &self,
        scope: &ScopeHandle,
        specs: &[federation::SourceSpec],
    ) -> Result<(), String> {
        for spec in specs {
            let Some(index) = scope
                .specs
                .iter()
                .position(|opened| opened.path == spec.path && opened.role == spec.role)
            else {
                return Err("source is not in opened scope".to_string());
            };
            let current = federation::live_source_version(spec).await.ok();
            if current != scope.versions[index] {
                return Err(format!("{} changed before the query", spec.path.display()));
            }
        }
        Ok(())
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
        // `named_sources` carries the registered required/optional role
        // into the specs — the scope must know it because a dead required
        // source fails a strict read while an optional one degrades.
        let specs = match registry::named_sources(&registry, &name) {
            Some(specs) => specs,
            None => return fail("no_scope", format!("no constellation named {name:?}")),
        };
        let handle = self.issue("scope");
        let mut versions = Vec::with_capacity(specs.len());
        let mut source_ids = Vec::with_capacity(specs.len());
        let mut sources: Vec<serde_json::Value> = Vec::with_capacity(specs.len());
        for spec in &specs {
            let version = federation::live_source_version(spec).await.ok();
            let source_id = federation::spec_source_id(spec);
            sources.push(json!({
                "path": spec.path.display().to_string(),
                "source_id": source_id,
                "role": spec.role.as_str(),
                "version": version,
            }));
            versions.push(version);
            source_ids.push(source_id);
        }
        self.handles.lock().await.insert(
            handle.clone(),
            Handle::Scope(ScopeHandle {
                name: name.clone(),
                specs,
                versions,
                source_ids,
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
                            "role": s.role.as_str(),
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
                #[cfg(test)]
                if let Err(detail) = self.precheck_scope_for_test(scope, &scope.specs).await {
                    return fail("snapshot_expired", detail);
                }
                #[cfg(test)]
                self.pause_before_live_query().await;
                match federation::federated_search(&scope.specs, &p.query, limit, mode_of(p.strict))
                    .await
                {
                    Ok(outcome) => {
                        if let Err(error) =
                            scope.verify_read(&scope.specs, &outcome.sources, &outcome.failures)
                        {
                            return fail("snapshot_expired", &error);
                        }
                        ok_json(json!({
                            "scope_handle": p.scope_handle,
                            "hits": hit_json(&outcome.hits),
                            "sources": outcome.sources,
                            "failures": outcome.failures,
                            "partial": outcome.partial,
                            "truncated": outcome.truncated,
                        }))
                    }
                    Err(e) => match scope.verify_failed_read(&scope.specs, &e).await {
                        Err(detail) => fail("snapshot_expired", detail),
                        Ok(()) => fail_read(e),
                    },
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
                Some(Handle::Snapshot(snap)) => {
                    let (bare, qualifier) = federation::parse_qualified_ref(&p.id);
                    match snapshot::snapshot_get(snap, bare, qualifier) {
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
                        Err(e) => fail_read_with_input(e, Some(&p.id)),
                    }
                }
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
                #[cfg(test)]
                if let Err(detail) = self.precheck_scope_for_test(scope, &specs).await {
                    return fail("snapshot_expired", detail);
                }
                #[cfg(test)]
                self.pause_before_live_query().await;
                match federation::federated_get(&specs, &bare, mode_of(p.strict)).await {
                    Ok(read) => {
                        if let Err(error) =
                            scope.verify_read(&specs, &read.read.sources, &read.read.failures)
                        {
                            return fail("snapshot_expired", &error);
                        }
                        let mut response = match read.result {
                            federation::FederatedGetResult::Found(a) => json!({
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
                            }),
                            federation::FederatedGetResult::Ambiguous { sources } => json!({
                                "result": "ambiguous",
                                "id": bare,
                                "claimers": sources,
                            }),
                            federation::FederatedGetResult::NotFound => json!({
                                "result": "not_found",
                                "id": bare,
                            }),
                            federation::FederatedGetResult::ResolveError(error) => {
                                return fail_read_with_input(
                                    ConstellationError::Resolve(error),
                                    Some(&p.id),
                                );
                            }
                        };
                        response["sources"] = json!(read.read.sources);
                        response["failures"] = json!(read.read.failures);
                        response["partial"] = json!(read.read.partial);
                        ok_json(response)
                    }
                    Err(e) => match scope.verify_failed_read(&specs, &e).await {
                        Err(detail) => fail("snapshot_expired", detail),
                        Ok(()) => fail_read_with_input(e, Some(&p.id)),
                    },
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
        description = "Build a source-qualified lexical ContextPack with declared relation closure, whole evidence units, bounded delivery and continuation.",
        annotations(title = "Constellation Pack", read_only_hint = true)
    )]
    async fn constellation_pack(
        &self,
        Parameters(p): Parameters<PackParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        if p.scope_handle.is_some() && p.snapshot_handle.is_some() {
            return fail("invalid_request", "provide exactly one handle");
        }
        let modern = context
            .protocol_version()
            .as_ref()
            .is_some_and(|version| version.as_str() >= ProtocolVersion::V_2026_07_28.as_str());
        let measure = |pack: &context_pack::ContextPack| pack_measure(pack, modern);
        let options = context_pack::PackOptions {
            budget_tokens: p.budget.unwrap_or(8000),
            byte_cap: p.byte_cap.unwrap_or(1_000_000),
            seeds: p.seeds.unwrap_or_default(),
            max_depth: p.max_depth.unwrap_or(4),
            max_nodes: p.max_nodes.unwrap_or(256),
            max_edges: p.max_edges.unwrap_or(1024),
            max_millis: p.max_millis.unwrap_or(5000),
            require_complete: p.require_complete.unwrap_or(false),
            cursor: p.cursor,
            presentation: if modern {
                "mcp-json-2026-07-28"
            } else {
                "mcp-json-legacy"
            }
            .into(),
            retrieval_mode: p.retrieval_mode.unwrap_or_else(|| "lexical".into()),
            request_context: p
                .snapshot_handle
                .as_ref()
                .or(p.scope_handle.as_ref())
                .cloned()
                .unwrap_or_default(),
            mermaid: p.mermaid.unwrap_or(false),
        };
        let handles = self.handles.lock().await;
        if let Some(snap_handle) = &p.snapshot_handle {
            return match handles.get(snap_handle) {
                Some(Handle::Snapshot(snap)) => {
                    match context_pack::pack_snapshot(snap, &p.task, &options, &measure) {
                        Ok(pack) => {
                            pack_result(&pack).map_err(|e| McpError::internal_error(e, None))
                        }
                        Err(e) => fail_read(e),
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
                match context_pack::pack_live(
                    &scope.specs,
                    &p.task,
                    &options,
                    mode_of(p.strict),
                    &measure,
                )
                .await
                {
                    Ok(pack) => {
                        if let Err(error) =
                            scope.verify_read(&scope.specs, &pack.sources, &pack.failures)
                        {
                            return fail("snapshot_expired", error);
                        }
                        pack_result(&pack).map_err(|e| McpError::internal_error(e, None))
                    }
                    Err(e) => match scope.verify_failed_read(&scope.specs, &e).await {
                        Err(detail) => fail("snapshot_expired", detail),
                        Ok(()) => fail_read(e),
                    },
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
                        Err(e) => fail_read_with_input(e, Some(&p.id)),
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
                #[cfg(test)]
                if let Err(detail) = self.precheck_scope_for_test(scope, &scope.specs).await {
                    return fail("snapshot_expired", detail);
                }
                #[cfg(test)]
                self.pause_before_live_query().await;
                match impact::impact_live(&scope.specs, &p.id, bounds, mode_of(p.strict)).await {
                    Ok(report) => {
                        if let Err(error) =
                            scope.verify_read(&scope.specs, &report.sources, &report.failures)
                        {
                            return fail("snapshot_expired", &error);
                        }
                        ok_json(
                            serde_json::to_value(&report)
                                .unwrap_or_else(|_| json!({"error":"serialize"})),
                        )
                    }
                    Err(e) => match scope.verify_failed_read(&scope.specs, &e).await {
                        Err(detail) => fail("snapshot_expired", detail),
                        Ok(()) => fail_read_with_input(e, Some(&p.id)),
                    },
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
            Some(Handle::Scope(scope)) => {
                #[cfg(test)]
                if let Err(detail) = self.precheck_scope_for_test(scope, &scope.specs).await {
                    return fail("snapshot_expired", detail);
                }
                #[cfg(test)]
                self.pause_before_live_query().await;
                match graph::graph_live(&scope.specs, bounds, mode_of(p.strict)).await {
                    Ok(outcome) => {
                        if let Err(error) =
                            scope.verify_read(&scope.specs, &outcome.sources, &outcome.failures)
                        {
                            return fail("snapshot_expired", &error);
                        }
                        ok_json(
                            serde_json::to_value(&outcome)
                                .unwrap_or_else(|_| json!({"error":"serialize"})),
                        )
                    }
                    Err(e) => match scope.verify_failed_read(&scope.specs, &e).await {
                        Err(detail) => fail("snapshot_expired", detail),
                        Ok(()) => fail_read(e),
                    },
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
    fn get_info(&self) -> rmcp::model::ServerConfig {
        rmcp::model::ServerConfig::new(
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
        Ok(rmcp::model::ListToolsResult::with_all_items(
            self.tool_router.list_all(),
        ))
    }

    async fn call_tool(
        &self,
        params: rmcp::model::CallToolRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::CallToolResponse, McpError> {
        let tcc = rmcp::handler::server::tool::ToolCallContext::new(self, params, context);
        self.tool_router.call(tcc).await
    }
}

#[cfg(test)]
mod identity_error_tests {
    use super::*;
    use forgeplan_core::artifact::canonical_key::{ArtifactCandidate, ResolveError};
    use rmcp::model::ContentBlock;

    #[test]
    fn conflict_is_a_tool_error_with_every_source_qualified_candidate() {
        let candidates = ["first", "second"].map(|name| ArtifactCandidate {
            canonical_key: "prd-shared".to_string(),
            display_id: format!("PRD-{}", if name == "first" { "001" } else { "002" }),
            title: name.to_string(),
            relative_path: format!("src:source-a/prds/{name}.md").into(),
        });
        let result = fail_read_with_input(
            ConstellationError::Resolve(ResolveError::ConflictingCanonicalKey {
                key: "prd-shared".to_string(),
                candidates: candidates.into(),
            }),
            Some("PRD-001"),
        )
        .unwrap();
        assert_eq!(result.is_error, Some(true));
        let ContentBlock::Text(text) = &result.content[0] else {
            panic!("expected JSON error");
        };
        let value: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(value["error"], "conflicting_canonical_key");
        assert_eq!(value["input"], "PRD-001");
        assert_eq!(
            value["candidates"][0]["relative_path"],
            "src:source-a/prds/first.md"
        );
        assert_eq!(value["candidates"][1]["title"], "second");
    }

    #[test]
    fn incomplete_snapshot_capture_remains_a_typed_tool_error() {
        let result = fail_read_with_input(
            ConstellationError::Resolve(ResolveError::IncompleteCapture {
                message: "pinned source could not be parsed".to_string(),
            }),
            Some("PRD-001"),
        )
        .unwrap();
        assert_eq!(result.is_error, Some(true));
        let ContentBlock::Text(text) = &result.content[0] else {
            panic!("expected JSON error");
        };
        let value: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(value["error"], "incomplete_capture");
        assert_eq!(value["input"], "PRD-001");
        assert!(
            value["detail"]
                .as_str()
                .unwrap()
                .contains("could not be parsed")
        );
    }
}

#[cfg(test)]
mod live_scope_interleaving_tests {
    use super::*;
    use rmcp::ServiceExt;
    use rmcp::model::{CallToolRequestParams, CallToolResult};
    use serde_json::Value;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;

    struct Fixture {
        client: rmcp::service::RunningService<rmcp::RoleClient, ()>,
        _server_task: tokio::task::JoinHandle<()>,
        gate: Arc<Mutex<Option<ReadPause>>>,
    }

    impl Fixture {
        async fn new(server: ConstellationReadServer) -> Arc<Self> {
            let gate = Arc::clone(&server.before_live_query);
            let (server_io, client_io) = tokio::io::duplex(64 * 1024);
            let server_task = tokio::spawn(async move {
                if let Ok(running) = server.serve(server_io).await {
                    let _ = running.waiting().await;
                }
            });
            let client = ().serve(client_io).await.expect("client handshake");
            Arc::new(Self {
                client,
                _server_task: server_task,
                gate,
            })
        }

        async fn call(&self, name: &'static str, args: Value) -> CallToolResult {
            let params =
                CallToolRequestParams::new(name).with_arguments(args.as_object().unwrap().clone());
            tokio::time::timeout(
                Duration::from_secs(15),
                self.client.peer().call_tool(params),
            )
            .await
            .expect("tool timeout")
            .expect("tool RPC")
        }

        async fn arm(
            &self,
        ) -> (
            tokio::sync::oneshot::Receiver<()>,
            tokio::sync::oneshot::Sender<()>,
        ) {
            let (reached, reached_rx) = tokio::sync::oneshot::channel();
            let (resume_tx, resume) = tokio::sync::oneshot::channel();
            *self.gate.lock().await = Some(ReadPause { reached, resume });
            (reached_rx, resume_tx)
        }
    }

    struct TestWorkspace {
        _temp: TempDir,
        fp: PathBuf,
        fixture: Arc<Fixture>,
    }

    async fn workspace(published: bool) -> TestWorkspace {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let root = temp.path().join("workspace");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("USERPROFILE", &home);
            std::env::set_var("XDG_DATA_HOME", &home);
        }
        let fp = forgeplan_core::workspace::init_workspace(&root, "interleaving-test")
            .unwrap()
            .canonicalize()
            .unwrap();
        write_auth(&fp, "Body about auth.");
        let mut reg = registry::ConstellationRegistry::default();
        registry::create(&mut reg, "crew").unwrap();
        registry::add_source(&mut reg, "crew", &fp, registry::SourceRole::Required).unwrap();
        registry::save(&registry::registry_path(&home), &reg).unwrap();
        if published {
            publish(&fp).await;
        }
        let fixture = Fixture::new(ConstellationReadServer::new()).await;
        TestWorkspace {
            _temp: temp,
            fp,
            fixture,
        }
    }

    fn write_auth(fp: &std::path::Path, body: &str) {
        std::fs::write(
            fp.join("adrs/ADR-001-auth.md"),
            format!(
                "---\nid: ADR-001\nkind: adr\nstatus: accepted\ntitle: Auth storage\n---\n\n{body}\n"
            ),
        )
        .unwrap();
    }

    async fn publish(fp: &std::path::Path) {
        let capability =
            forgeplan_core::constellation::facade::MaintainCapability::for_strict_maintenance(
                fp.to_path_buf(),
            );
        forgeplan_core::constellation::facade::maintain_index(
            &capability,
            forgeplan_core::constellation::capture::CaptureLimits::default(),
            std::time::Instant::now() + Duration::from_secs(120),
        )
        .await
        .unwrap();
    }

    fn response_json(result: &CallToolResult) -> Value {
        let rmcp::model::ContentBlock::Text(text) = &result.content[0] else {
            panic!("expected tool JSON text");
        };
        serde_json::from_str(&text.text).unwrap()
    }

    async fn open(fixture: &Fixture) -> String {
        let result = fixture.call("constellation_open", json!({})).await;
        let opened = response_json(&result);
        opened["scope_handle"].as_str().unwrap().to_string()
    }

    async fn interleave(
        fixture: &Arc<Fixture>,
        name: &'static str,
        args: Value,
        mutate: impl FnOnce() + Send + 'static,
    ) -> Value {
        let (reached, resume) = fixture.arm().await;
        let fixture = Arc::clone(fixture);
        let request = tokio::spawn(async move { fixture.call(name, args).await });
        reached
            .await
            .expect("handler reached the post-precheck gate");
        mutate();
        resume.send(()).unwrap();
        let response = request.await.unwrap();
        response_json(&response)
    }

    fn read_args(name: &'static str, scope: &str) -> Value {
        match name {
            "constellation_search" => json!({"query": "auth", "scope_handle": scope}),
            "constellation_get" => json!({"id": "ADR-001", "scope_handle": scope}),
            "constellation_graph" => json!({"scope_handle": scope}),
            "constellation_impact" => json!({"id": "ADR-001", "scope_handle": scope}),
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    #[serial_test::serial(env_home)]
    async fn post_precheck_markdown_change_expires_every_live_reader() {
        for name in [
            "constellation_search",
            "constellation_get",
            "constellation_graph",
            "constellation_impact",
        ] {
            let workspace = workspace(false).await;
            let scope = open(&workspace.fixture).await;
            let fp = workspace.fp.clone();
            let response = interleave(
                &workspace.fixture,
                name,
                read_args(name, &scope),
                move || write_auth(&fp, "Changed body."),
            )
            .await;
            assert_eq!(
                response["error"], "snapshot_expired",
                "{name} served a changed corpus after its precheck: {response}"
            );
            assert_ne!(response["body"], "Changed body.");
        }
    }

    #[tokio::test]
    #[serial_test::serial(env_home)]
    async fn post_precheck_delete_and_new_ambiguity_expire_get() {
        let deleted = workspace(false).await;
        let scope = open(&deleted.fixture).await;
        let artifact = deleted.fp.join("adrs/ADR-001-auth.md");
        let response = interleave(
            &deleted.fixture,
            "constellation_get",
            read_args("constellation_get", &scope),
            move || std::fs::remove_file(artifact).unwrap(),
        )
        .await;
        assert_eq!(response["error"], "snapshot_expired", "{response}");
        assert_ne!(response["result"], "not_found");

        let ambiguous = workspace(false).await;
        let scope = open(&ambiguous.fixture).await;
        let fp = ambiguous.fp.clone();
        let response = interleave(
            &ambiguous.fixture,
            "constellation_get",
            read_args("constellation_get", &scope),
            move || {
                std::fs::write(
                    fp.join("adrs/ADR-001-copy.md"),
                    "---\nid: ADR-001\nkind: adr\nstatus: accepted\ntitle: Alternate auth storage\n---\n\nAlternate body.\n",
                )
                .unwrap();
            },
        )
        .await;
        assert_eq!(response["error"], "snapshot_expired", "{response}");
        assert_ne!(response["result"], "ambiguous");
    }

    #[tokio::test]
    #[serial_test::serial(env_home)]
    async fn post_precheck_new_published_generation_expires_get() {
        let workspace = workspace(true).await;
        let scope = open(&workspace.fixture).await;
        let fp = workspace.fp.clone();
        let fixture = Arc::clone(&workspace.fixture);
        let (reached, resume) = fixture.arm().await;
        let args = read_args("constellation_get", &scope);
        let request_fixture = Arc::clone(&fixture);
        let request =
            tokio::spawn(async move { request_fixture.call("constellation_get", args).await });
        reached
            .await
            .expect("handler reached the post-precheck gate");
        write_auth(&fp, "Published B body.");
        publish(&fp).await;
        resume.send(()).unwrap();
        let response = response_json(&request.await.unwrap());
        assert_eq!(response["error"], "snapshot_expired", "{response}");
        assert_ne!(response["body"], "Published B body.");
    }
}
