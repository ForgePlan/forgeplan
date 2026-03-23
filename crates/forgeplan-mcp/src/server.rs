use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{NaiveDate, Utc};
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo};
use rmcp::schemars;
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::sync::RwLock;

use forgeplan_core::artifact::types::{ArtifactKind, Mode};
use forgeplan_core::db::store::{ArtifactFilter, ArtifactRecord, LanceStore, NewArtifact};
use forgeplan_core::graph;
use forgeplan_core::link;
use forgeplan_core::progress;
use forgeplan_core::projection;
use forgeplan_core::scoring::reff::{self, EvidenceItem};
use forgeplan_core::template::{get_embedded_template, render_template};
use forgeplan_core::validation;
use forgeplan_core::workspace;

use crate::types::*;

// ── Server struct ────────────────────────────────────────────

#[derive(Clone)]
pub struct ForgeplanServer {
    store: Arc<RwLock<Option<Arc<LanceStore>>>>,
    workspace_root: PathBuf,
    workspace_path: Arc<RwLock<Option<PathBuf>>>,
    tool_router: ToolRouter<Self>,
}

impl ForgeplanServer {
    pub async fn new(workspace_root: PathBuf) -> Self {
        let ws = workspace::find_workspace(&workspace_root);
        let store = if let Some(ref ws_path) = ws {
            LanceStore::open(ws_path).await.ok().map(Arc::new)
        } else {
            None
        };

        Self {
            store: Arc::new(RwLock::new(store)),
            workspace_root,
            workspace_path: Arc::new(RwLock::new(ws)),
            tool_router: Self::tool_router(),
        }
    }

    /// Clone the Arc<LanceStore> and immediately release the RwLock guard.
    /// This prevents holding the lock across .await points in tool handlers.
    async fn require_store(&self) -> Result<Arc<LanceStore>, String> {
        self.store
            .read()
            .await
            .clone()
            .ok_or_else(|| "Workspace not initialized. Call forgeplan_init first.".into())
    }

    async fn require_workspace(&self) -> Result<PathBuf, String> {
        self.workspace_path
            .read()
            .await
            .clone()
            .ok_or_else(|| "Workspace not initialized. Call forgeplan_init first.".into())
    }
}

// ── Helpers ──────────────────────────────────────────────────

fn json_result<T: serde::Serialize>(data: &T) -> CallToolResult {
    match serde_json::to_string_pretty(data) {
        Ok(json) => CallToolResult::success(vec![Content::text(json)]),
        Err(e) => CallToolResult::error(vec![Content::text(format!("Serialization error: {e}"))]),
    }
}

fn text_result(msg: &str) -> CallToolResult {
    CallToolResult::success(vec![Content::text(msg)])
}

fn err_result(msg: &str) -> CallToolResult {
    CallToolResult::error(vec![Content::text(msg.to_string())])
}

// ── Parameter types (inline for tools) ───────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct InitParams {
    /// Force reinitialize even if workspace exists
    #[serde(default)]
    force: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct NewParams {
    /// Artifact kind: prd, epic, spec, rfc, adr, problem, solution, evidence, note, refresh
    kind: String,
    /// Artifact title
    title: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListParams {
    /// Filter by kind (optional)
    #[serde(default)]
    kind: Option<String>,
    /// Filter by status (optional)
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ValidateParams {
    /// Artifact ID to validate (validates all if omitted)
    #[serde(default)]
    id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ScoreParams {
    /// Artifact ID to score
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct LinkParams {
    /// Source artifact ID
    source: String,
    /// Target artifact ID
    target: String,
    /// Relationship type: informs, based_on, supersedes, contradicts, refines (default: informs)
    #[serde(default = "default_relation")]
    relation: String,
}

fn default_relation() -> String {
    "informs".into()
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetParams {
    /// Artifact ID to read
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateParams {
    /// Artifact ID to update
    id: String,
    /// New status (draft, active, superseded, deprecated)
    #[serde(default)]
    status: Option<String>,
    /// New title
    #[serde(default)]
    title: Option<String>,
    /// New body content
    #[serde(default)]
    body: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DeleteParams {
    /// Artifact ID to delete
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RouteParams {
    /// Task description in natural language
    description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ReviewParams {
    /// Artifact ID to review
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ActivateParams {
    /// Artifact ID to activate
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SupersedeParams {
    /// Artifact ID to supersede
    id: String,
    /// Replacement artifact ID
    by: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DeprecateParams {
    /// Artifact ID to deprecate
    id: String,
    /// Reason for deprecation
    reason: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct JournalParams {
    /// Filter by kind (adr, note, problem, solution)
    #[serde(default)]
    kind: Option<String>,
    /// Show only at-risk decisions
    #[serde(default)]
    risk: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CaptureParams {
    /// The decision statement to capture
    decision: String,
    /// Additional context (optional)
    #[serde(default)]
    context: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CalibrateParams {
    /// Artifact ID (checks all if omitted)
    #[serde(default)]
    id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ReasonParams {
    /// Artifact ID to analyze with ADI reasoning cycle
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DecomposeParams {
    /// PRD artifact ID to decompose into RFC tasks
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GenerateParams {
    /// Artifact kind: prd, epic, spec, rfc, adr, problem, solution, evidence
    kind: String,
    /// Natural language description of what to generate
    description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchParams {
    /// Search query (case-insensitive substring)
    query: String,
    /// Filter by artifact kind (optional)
    #[serde(default)]
    kind: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ProgressParams {
    /// Artifact ID (shows all artifacts with checkboxes if omitted)
    #[serde(default)]
    id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ExportParams {
    /// Optional output file path. If omitted, returns JSON directly.
    #[serde(default)]
    output: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ImportParams {
    /// JSON export data as a string
    data: String,
    /// Overwrite existing artifacts (default: false)
    #[serde(default)]
    force: Option<bool>,
}

// ── Tool implementations ─────────────────────────────────────

#[tool_router]
impl ForgeplanServer {
    #[tool(description = "Initialize a new .forgeplan/ workspace. Creates LanceDB tables, config, and artifact subdirectories.")]
    async fn forgeplan_init(
        &self,
        Parameters(p): Parameters<InitParams>,
    ) -> Result<CallToolResult, McpError> {
        let force = p.force.unwrap_or(false);

        if let Some(existing) = workspace::find_workspace(&self.workspace_root) {
            if !force {
                return Ok(json_result(&InitResponse {
                    workspace: existing.display().to_string(),
                    message: "Already initialized. Use force=true to reinitialize.".into(),
                }));
            }
            tokio::fs::remove_dir_all(&existing)
                .await
                .map_err(|e| McpError::internal_error(format!("Failed to remove workspace: {e}"), None))?;
        }

        let project_name = self
            .workspace_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".into());

        let ws = workspace::init_workspace(&self.workspace_root, &project_name)
            .map_err(|e| McpError::internal_error(format!("Init failed: {e}"), None))?;

        let new_store = LanceStore::init(&ws)
            .await
            .map_err(|e| McpError::internal_error(format!("LanceDB init failed: {e}"), None))?;

        *self.store.write().await = Some(Arc::new(new_store));
        *self.workspace_path.write().await = Some(ws.clone());

        Ok(json_result(&InitResponse {
            workspace: ws.display().to_string(),
            message: format!("Initialized .forgeplan/ for project '{project_name}'"),
        }))
    }

    #[tool(description = "Create a new artifact from template. Generates a sequential ID (e.g., PRD-001), renders the template, stores in LanceDB, and writes a markdown projection.")]
    async fn forgeplan_new(
        &self,
        Parameters(p): Parameters<NewParams>,
    ) -> Result<CallToolResult, McpError> {
        let ws = match self.require_workspace().await {
            Ok(ws) => ws,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let artifact_kind: ArtifactKind = match p.kind.parse() {
            Ok(k) => k,
            Err(e) => return Ok(err_result(&format!("{e}"))),
        };

        let prefix = artifact_kind.prefix().trim_end_matches('-').to_uppercase();
        let id = store
            .next_id(&prefix)
            .await
            .map_err(|e| McpError::internal_error(format!("ID generation failed: {e}"), None))?;

        let template_key = artifact_kind.template_key();
        let template = match get_embedded_template(template_key) {
            Some(t) => t,
            None => return Ok(err_result(&format!("No template for kind '{template_key}'"))),
        };

        let today = Utc::now().format("%Y-%m-%d").to_string();
        let nnn = id.split('-').last().unwrap_or("001").to_string();

        let mut vars = std::collections::HashMap::new();
        vars.insert("NNN".into(), nnn.clone());
        vars.insert("title".into(), p.title.clone());
        vars.insert("Title".into(), p.title.clone());

        let mut rendered = render_template(template, &vars);
        rendered = rendered.replace("YYYY-MM-DD", &today);

        let heading_pattern = format!("# {prefix}-{nnn}: ");
        if let Some(pos) = rendered.find(&heading_pattern) {
            let line_start = pos + heading_pattern.len();
            if let Some(nl) = rendered[line_start..].find('\n') {
                let old = &rendered[line_start..line_start + nl];
                if old.contains('{') || old.contains('/') {
                    let before = &rendered[..line_start];
                    let after = &rendered[line_start + nl..];
                    rendered = format!("{before}{}{after}", p.title);
                }
            }
        }

        let artifact = NewArtifact {
            id: id.clone(),
            kind: template_key.into(),
            status: "draft".into(),
            title: p.title.clone(),
            body: rendered.clone(),
            depth: "standard".into(),
            author: None,
            parent_epic: None,
            valid_until: None,
        };

        store
            .create_artifact(&artifact)
            .await
            .map_err(|e| McpError::internal_error(format!("Create failed: {e}"), None))?;

        let filepath = projection::render_projection(
            &ws, &id, template_key, &p.title, "draft", "standard",
            None, None, None, &rendered, &[],
        )
        .await
        .map_err(|e| McpError::internal_error(format!("Projection failed: {e}"), None))?;

        Ok(json_result(&NewArtifactResponse {
            id,
            kind: template_key.into(),
            title: p.title,
            filepath: filepath.display().to_string(),
        }))
    }

    #[tool(description = "List artifacts with optional kind/status filters. Returns ID, kind, status, and title for each artifact.")]
    async fn forgeplan_list(
        &self,
        Parameters(p): Parameters<ListParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let filter = if p.kind.is_some() || p.status.is_some() {
            Some(ArtifactFilter {
                kind: p.kind,
                status: p.status,
            })
        } else {
            None
        };

        let artifacts = store
            .list_artifacts(filter.as_ref())
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let total = artifacts.len();
        let dtos: Vec<ArtifactSummaryDto> = artifacts.into_iter().map(Into::into).collect();

        Ok(json_result(&ListResponse {
            artifacts: dtos,
            total,
        }))
    }

    #[tool(description = "Show project status dashboard — total artifacts, counts by kind and status.")]
    async fn forgeplan_status(&self) -> Result<CallToolResult, McpError> {
        let ws = match self.require_workspace().await {
            Ok(ws) => ws,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let config = workspace::load_config(&ws)
            .map_err(|e| McpError::internal_error(format!("Config error: {e}"), None))?;

        let artifacts = store
            .list_artifacts(None)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
        for a in &artifacts {
            *by_kind.entry(a.kind.clone()).or_default() += 1;
            *by_status.entry(a.status.clone()).or_default() += 1;
        }

        Ok(json_result(&StatusResponse {
            project: config.project_name,
            workspace: ws.display().to_string(),
            total: artifacts.len(),
            by_kind: by_kind
                .into_iter()
                .map(|(kind, count)| KindCount { kind, count })
                .collect(),
            by_status: by_status
                .into_iter()
                .map(|(status, count)| StatusCount { status, count })
                .collect(),
        }))
    }

    #[tool(description = "Validate artifact completeness against schema rules. Checks required sections per artifact kind and depth level. Returns structured findings with severity (MUST/SHOULD/COULD).")]
    async fn forgeplan_validate(
        &self,
        Parameters(p): Parameters<ValidateParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let all_records = store
            .list_records(None)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let to_validate: Vec<&ArtifactRecord> = if let Some(ref target_id) = p.id {
            let upper = target_id.to_uppercase();
            let filtered: Vec<_> = all_records
                .iter()
                .filter(|r| r.id.to_uppercase() == upper)
                .collect();
            if filtered.is_empty() {
                return Ok(err_result(&format!("Artifact '{target_id}' not found")));
            }
            filtered
        } else {
            all_records.iter().collect()
        };

        let mut results = Vec::new();
        let mut total_errors = 0;
        let mut total_warnings = 0;
        let mut total_passed = 0;

        for record in &to_validate {
            let fm = record.frontmatter_map();
            let kind = record.kind.parse::<ArtifactKind>().unwrap_or(ArtifactKind::Note);
            let depth = record.depth.parse::<Mode>().unwrap_or(Mode::Standard);

            let result = validation::validate(&record.id, &record.body, &fm, &kind, &depth);
            total_errors += result.error_count();
            total_warnings += result.warning_count();
            if result.passed() {
                total_passed += 1;
            }
            results.push(ValidationResultDto::from(result));
        }

        Ok(json_result(&ValidateResponse {
            total_artifacts: to_validate.len(),
            total_passed,
            total_errors,
            total_warnings,
            results,
        }))
    }

    #[tool(description = "Compute R_eff quality score for an artifact based on linked evidence. R_eff uses the weakest-link principle: score = min(evidence_scores).")]
    async fn forgeplan_score(
        &self,
        Parameters(p): Parameters<ScoreParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let target = match store.get_record(&p.id).await {
            Ok(Some(r)) => r,
            Ok(None) => return Ok(err_result(&format!("Artifact '{}' not found", p.id))),
            Err(e) => return Ok(err_result(&format!("{e}"))),
        };

        let outgoing = store.get_relations(&p.id).await.unwrap_or_else(|e| {
            tracing::warn!("Failed to get relations for {}: {e}", p.id);
            Vec::new()
        });
        let evidence_targets: Vec<String> = outgoing
            .iter()
            .filter(|(_, rel)| rel == "informs" || rel == "based_on" || rel == "refines")
            .map(|(t, _)| t.clone())
            .collect();

        let filter = ArtifactFilter {
            kind: Some("evidence".into()),
            status: None,
        };
        let evidence_records = store.list_records(Some(&filter)).await.unwrap_or_else(|e| {
            tracing::warn!("Failed to list evidence records: {e}");
            Vec::new()
        });

        let mut evidence_items: Vec<EvidenceItem> = Vec::new();
        let mut evidence_dtos: Vec<EvidenceDto> = Vec::new();

        for ev in &evidence_records {
            let is_linked = evidence_targets
                .iter()
                .any(|eid| eid.eq_ignore_ascii_case(&ev.id));

            if !is_linked {
                let ev_rels = store.get_relations(&ev.id).await.unwrap_or_else(|e| {
                    tracing::warn!("Failed to get relations for {}: {e}", ev.id);
                    Vec::new()
                });
                if !ev_rels.iter().any(|(t, _)| t.eq_ignore_ascii_case(&p.id)) {
                    continue;
                }
            }

            let item = parse_evidence_from_record(ev);
            let item_score = reff::r_eff(&[item.clone()]);
            let expired = item
                .valid_until
                .map(|dt| Utc::now().naive_utc() > dt)
                .unwrap_or(false);

            evidence_dtos.push(EvidenceDto {
                id: item.id.clone(),
                verdict: format!("{:?}", item.verdict),
                congruence_level: item.congruence_level,
                score: item_score,
                expired,
            });
            evidence_items.push(item);
        }

        let r_eff = reff::r_eff(&evidence_items);

        Ok(json_result(&ScoreResponse {
            id: target.id,
            title: target.title,
            r_eff,
            evidence: evidence_dtos,
        }))
    }

    #[tool(description = "Link two artifacts with a typed relationship. Valid types: informs, based_on, supersedes, contradicts, refines.")]
    async fn forgeplan_link(
        &self,
        Parameters(p): Parameters<LinkParams>,
    ) -> Result<CallToolResult, McpError> {
        let ws = match self.require_workspace().await {
            Ok(ws) => ws,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let relation = match link::normalize_relation(&p.relation) {
            Ok(r) => r,
            Err(e) => return Ok(err_result(&format!("{e}"))),
        };

        match store.get_artifact(&p.source).await {
            Ok(None) => return Ok(err_result(&format!("Source artifact '{}' not found", p.source))),
            Err(e) => return Ok(err_result(&format!("{e}"))),
            _ => {}
        }

        if let Err(e) = store.add_relation(&p.source, &p.target, &relation).await {
            return Ok(err_result(&format!("{e}")));
        }

        // Update markdown projection
        if let Ok(Some(record)) = store.get_record(&p.source).await {
            let links = store.get_relations(&p.source).await.unwrap_or_default();
            let _ = projection::render_projection(
                &ws, &record.id, &record.kind, &record.title, &record.status,
                &record.depth, record.author.as_deref(), record.parent_epic.as_deref(),
                record.valid_until.as_deref(), &record.body, &links,
            )
            .await;
        }

        Ok(json_result(&LinkResponse {
            message: format!("Linked: {} --{}--> {}", p.source, relation, p.target),
        }))
    }

    #[tool(description = "Read a full artifact by ID. Returns all metadata and body content.")]
    async fn forgeplan_get(
        &self,
        Parameters(p): Parameters<GetParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        match store.get_record(&p.id).await {
            Ok(Some(r)) => Ok(json_result(&ArtifactRecordDto::from(r))),
            Ok(None) => Ok(err_result(&format!("Artifact '{}' not found", p.id))),
            Err(e) => Ok(err_result(&format!("{e}"))),
        }
    }

    #[tool(description = "Update artifact metadata (status, title) and/or body. Re-renders markdown projection after update.")]
    async fn forgeplan_update(
        &self,
        Parameters(p): Parameters<UpdateParams>,
    ) -> Result<CallToolResult, McpError> {
        let ws = match self.require_workspace().await {
            Ok(ws) => ws,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        // Verify exists
        if store.get_record(&p.id).await.map_err(|e| McpError::internal_error(format!("{e}"), None))?.is_none() {
            return Ok(err_result(&format!("Artifact '{}' not found", p.id)));
        }

        if p.status.is_none() && p.title.is_none() && p.body.is_none() {
            return Ok(err_result("Nothing to update. Provide status, title, or body."));
        }

        if p.status.is_some() || p.title.is_some() {
            store
                .update_artifact(&p.id, p.status.as_deref(), p.title.as_deref())
                .await
                .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
        }

        if let Some(ref body) = p.body {
            store
                .update_body(&p.id, body)
                .await
                .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
        }

        // Re-render projection
        let updated = store.get_record(&p.id).await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?
            .ok_or_else(|| McpError::internal_error("Artifact disappeared after update", None))?;
        let links = store.get_relations(&p.id).await.unwrap_or_default();
        let _ = projection::render_projection(
            &ws, &updated.id, &updated.kind, &updated.title, &updated.status,
            &updated.depth, updated.author.as_deref(), updated.parent_epic.as_deref(),
            updated.valid_until.as_deref(), &updated.body, &links,
        ).await;

        Ok(json_result(&serde_json::json!({
            "id": p.id,
            "message": "Updated successfully",
            "status": updated.status,
            "title": updated.title,
        })))
    }

    #[tool(description = "Delete an artifact from LanceDB and remove its markdown projection file.")]
    async fn forgeplan_delete(
        &self,
        Parameters(p): Parameters<DeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        let ws = match self.require_workspace().await {
            Ok(ws) => ws,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let record = match store.get_record(&p.id).await {
            Ok(Some(r)) => r,
            Ok(None) => return Ok(err_result(&format!("Artifact '{}' not found", p.id))),
            Err(e) => return Ok(err_result(&format!("{e}"))),
        };

        store
            .delete_artifact(&p.id)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        // Remove projection file
        if let Ok(kind) = record.kind.parse::<ArtifactKind>() {
            let slug = forgeplan_core::artifact::types::slugify(&record.title);
            let filename = format!("{}-{}.md", record.id, slug);
            let filepath = ws.join(kind.dir_name()).join(&filename);
            let _ = tokio::fs::remove_file(&filepath).await;
        }

        Ok(json_result(&serde_json::json!({
            "id": p.id,
            "title": record.title,
            "message": "Deleted successfully",
        })))
    }

    #[tool(description = "Suggest depth level (Tactical/Standard/Deep/Critical) and artifact pipeline for a task description. Rule-based, instant, no LLM needed.")]
    async fn forgeplan_route(
        &self,
        Parameters(p): Parameters<RouteParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = forgeplan_core::routing::route(&p.description);
        Ok(json_result(&serde_json::json!({
            "depth": format!("{:?}", result.depth),
            "pipeline": result.pipeline.iter().map(|k| k.template_key()).collect::<Vec<_>>(),
            "triggers": result.triggers.iter().map(|t| &t.id).collect::<Vec<_>>(),
            "confidence": result.confidence,
            "display": format!("{result}"),
        })))
    }

    #[tool(description = "Review an artifact — run validation and show lifecycle checklist. Shows MUST/SHOULD findings and whether artifact can be activated.")]
    async fn forgeplan_review(
        &self,
        Parameters(p): Parameters<ReviewParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };
        match forgeplan_core::lifecycle::review(&store, &p.id).await {
            Ok(result) => Ok(json_result(&serde_json::json!({
                "artifact_id": result.artifact_id,
                "can_activate": result.can_activate,
                "must_findings": result.must_findings,
                "should_findings": result.should_findings,
                "warnings": result.warnings,
            }))),
            Err(e) => Ok(err_result(&e.to_string())),
        }
    }

    #[tool(description = "Activate an artifact (draft → active). Requires all MUST validation rules to pass.")]
    async fn forgeplan_activate(
        &self,
        Parameters(p): Parameters<ActivateParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };
        match forgeplan_core::lifecycle::activate(&store, &p.id).await {
            Ok(()) => Ok(text_result(&format!("Activated {} (draft → active)", p.id))),
            Err(e) => Ok(err_result(&e.to_string())),
        }
    }

    #[tool(description = "Supersede an artifact (active → superseded). Creates link to replacement and notifies dependents.")]
    async fn forgeplan_supersede(
        &self,
        Parameters(p): Parameters<SupersedeParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };
        match forgeplan_core::lifecycle::supersede(&store, &p.id, &p.by).await {
            Ok(dependents) => Ok(json_result(&serde_json::json!({
                "superseded": p.id,
                "replacement": p.by,
                "dependents_affected": dependents,
            }))),
            Err(e) => Ok(err_result(&e.to_string())),
        }
    }

    #[tool(description = "Deprecate an artifact (active → deprecated) with a reason.")]
    async fn forgeplan_deprecate(
        &self,
        Parameters(p): Parameters<DeprecateParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };
        match forgeplan_core::lifecycle::deprecate(&store, &p.id, &p.reason).await {
            Ok(dependents) => Ok(json_result(&serde_json::json!({
                "deprecated": p.id,
                "reason": p.reason,
                "dependents_affected": dependents,
            }))),
            Err(e) => Ok(err_result(&e.to_string())),
        }
    }

    #[tool(description = "Show project health dashboard — gaps, risks, blind spots, orphans, stale evidence, and recommended next actions. No LLM needed.")]
    async fn forgeplan_health(&self) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let report = forgeplan_core::health::health_report(&store)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        Ok(json_result(&serde_json::json!({
            "total": report.total,
            "by_kind": report.by_kind,
            "by_status": report.by_status,
            "at_risk": report.at_risk.iter().map(|a| serde_json::json!({
                "id": a.id, "title": a.title, "reason": a.reason
            })).collect::<Vec<_>>(),
            "blind_spots": report.blind_spots.iter().map(|b| serde_json::json!({
                "id": b.id, "title": b.title, "issue": b.issue
            })).collect::<Vec<_>>(),
            "stale_count": report.stale_count,
            "orphans": report.orphans,
            "next_actions": report.next_actions,
        })))
    }

    #[tool(description = "Show decision journal — chronological timeline of ADR, Note, Problem, Solution artifacts with R_eff scores and evidence status.")]
    async fn forgeplan_journal(
        &self,
        Parameters(p): Parameters<JournalParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let entries = forgeplan_core::journal::build_journal(
            &store, p.kind.as_deref(), p.risk.unwrap_or(false),
        )
        .await
        .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let dtos: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| serde_json::json!({
                "id": e.id, "title": e.title, "kind": e.kind,
                "created_at": e.created_at, "r_eff": e.r_eff,
                "evidence_count": e.evidence_count,
                "has_stale_evidence": e.has_stale_evidence,
            }))
            .collect();

        Ok(json_result(&serde_json::json!({
            "entries": dtos, "total": entries.len(),
        })))
    }

    #[tool(description = "Show blind spots — decisions (PRD/RFC/ADR/Epic) without linked evidence, and orphan artifacts with no connections.")]
    async fn forgeplan_blindspots(&self) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let report = forgeplan_core::health::health_report(&store)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        Ok(json_result(&serde_json::json!({
            "blind_spots": report.blind_spots.iter().map(|b| serde_json::json!({
                "id": b.id, "title": b.title, "issue": b.issue
            })).collect::<Vec<_>>(),
            "orphans": report.orphans,
            "total_blind_spots": report.blind_spots.len(),
            "total_orphans": report.orphans.len(),
        })))
    }

    #[tool(description = "Capture a decision from conversation into a Note or ADR artifact. Auto-detects type: simple decisions become Notes, architectural decisions become ADRs. Requires LLM provider.")]
    async fn forgeplan_capture(
        &self,
        Parameters(p): Parameters<CaptureParams>,
    ) -> Result<CallToolResult, McpError> {
        let ws = match self.require_workspace().await {
            Ok(ws) => ws,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let config = workspace::load_config(&ws)
            .map_err(|e| McpError::internal_error(format!("Config error: {e}"), None))?;
        let llm_config = config.llm.unwrap_or_default().with_env_overrides();

        let (kind_str, body) = forgeplan_core::llm::capture::capture(
            &llm_config, &p.decision, p.context.as_deref(),
        )
        .await
        .map_err(|e| McpError::internal_error(format!("Capture failed: {e}"), None))?;

        let kind: ArtifactKind = kind_str.parse().unwrap_or(ArtifactKind::Note);
        let template_key = kind.template_key();
        let prefix = kind.prefix().trim_end_matches('-').to_uppercase();
        let id = store.next_id(&prefix).await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let title: String = p.decision.lines().next().unwrap_or(&p.decision).chars().take(80).collect();

        let artifact = NewArtifact {
            id: id.clone(),
            kind: template_key.into(),
            status: "draft".into(),
            title: title.clone(),
            body: body.clone(),
            depth: "tactical".into(),
            author: None,
            parent_epic: None,
            valid_until: None,
        };

        store.create_artifact(&artifact).await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let filepath = projection::render_projection(
            &ws, &id, template_key, &title, "draft", "tactical",
            None, None, None, &body, &[],
        ).await
        .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        Ok(json_result(&serde_json::json!({
            "id": id,
            "kind": template_key,
            "title": title,
            "filepath": filepath.display().to_string(),
            "auto_detected_type": kind_str,
            "provider": llm_config.provider,
            "model": llm_config.model,
        })))
    }

    #[tool(description = "Generate a mermaid dependency graph of all linked artifacts. Includes explicit links and parent_epic belongs_to edges.")]
    async fn forgeplan_graph(&self) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let relations = store
            .get_all_relations()
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let mut edges: Vec<graph::Edge> = relations
            .into_iter()
            .map(|(from, to, relation)| graph::Edge { from, to, relation })
            .collect();

        let records = store
            .list_records(None)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        for record in &records {
            if let Some(parent) = &record.parent_epic {
                if !parent.is_empty() {
                    edges.push(graph::Edge {
                        from: record.id.clone(),
                        to: parent.clone(),
                        relation: "belongs_to".into(),
                    });
                }
            }
        }

        edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));
        let mermaid = graph::render_mermaid(&edges);

        Ok(json_result(&GraphResponse { mermaid }))
    }

    #[tool(description = "Search artifacts by keyword (case-insensitive substring match on title and body). Returns matching artifacts with highlighted lines.")]
    async fn forgeplan_search(
        &self,
        Parameters(p): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let hits = store
            .search_body(&p.query, p.kind.as_deref())
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let query_lower = p.query.to_lowercase();
        let results: Vec<SearchResultDto> = hits
            .iter()
            .map(|record| {
                let matched_lines: Vec<String> = record
                    .body
                    .lines()
                    .enumerate()
                    .filter(|(_, line)| line.to_lowercase().contains(&query_lower))
                    .take(5)
                    .map(|(i, line)| {
                        if line.chars().count() > 120 {
                            format!("L{}: {}...", i + 1, line.chars().take(120).collect::<String>())
                        } else {
                            format!("L{}: {}", i + 1, line.trim())
                        }
                    })
                    .collect();

                SearchResultDto {
                    id: record.id.clone(),
                    kind: record.kind.clone(),
                    title: record.title.clone(),
                    matched_lines,
                }
            })
            .collect();

        let total = results.len();
        Ok(json_result(&SearchResponse { results, total }))
    }

    #[tool(description = "Detect stale artifacts with expired valid_until dates. Returns the list of expired artifacts with days since expiry.")]
    async fn forgeplan_stale(&self) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let stale_records = store
            .find_stale()
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let today = Utc::now().date_naive();

        let stale: Vec<StaleArtifactDto> = stale_records
            .iter()
            .map(|r| {
                let valid_until_str = r.valid_until.as_deref().unwrap_or("unknown");
                let days = r
                    .valid_until
                    .as_deref()
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                    .map(|d| (today - d).num_days())
                    .unwrap_or(0);

                StaleArtifactDto {
                    id: r.id.clone(),
                    title: r.title.clone(),
                    valid_until: valid_until_str.into(),
                    days_expired: days,
                }
            })
            .collect();

        let total = stale.len();
        Ok(json_result(&StaleResponse { stale, total }))
    }

    #[tool(description = "Show checkbox progress for artifacts. Parses markdown checkboxes (- [ ] / - [x]) and computes completion percentages.")]
    async fn forgeplan_progress(
        &self,
        Parameters(p): Parameters<ProgressParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let records = store
            .list_records(None)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let to_report: Vec<&ArtifactRecord> = if let Some(ref target_id) = p.id {
            let upper = target_id.to_uppercase();
            let filtered: Vec<_> = records
                .iter()
                .filter(|r| r.id.to_uppercase() == upper)
                .collect();
            if filtered.is_empty() {
                return Ok(err_result(&format!("Artifact '{target_id}' not found")));
            }
            filtered
        } else {
            records.iter().collect()
        };

        let mut dtos = Vec::new();
        let mut total_checkboxes = 0usize;
        let mut total_completed = 0usize;

        for record in &to_report {
            let count = progress::count_checkboxes(&record.body);
            if p.id.is_some() || count.total > 0 {
                let percent = if count.total > 0 {
                    ((count.completed as f64 / count.total as f64) * 100.0).round()
                } else {
                    0.0
                };
                total_checkboxes += count.total;
                total_completed += count.completed;
                dtos.push(ProgressDto {
                    id: record.id.clone(),
                    title: record.title.clone(),
                    kind: record.kind.clone(),
                    total: count.total,
                    completed: count.completed,
                    percent,
                });
            }
        }

        Ok(json_result(&ProgressResponse {
            artifacts: dtos,
            total_checkboxes,
            total_completed,
        }))
    }

    #[tool(description = "Show evidence decay impact on R_eff scores. Lists artifacts where expired evidence has degraded quality scores, with current vs fresh R_eff comparison.")]
    async fn forgeplan_decay(&self) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let entries = forgeplan_core::scoring::decay::decay_report(&store)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let total = entries.len();
        let dtos: Vec<DecayEntryDto> = entries
            .into_iter()
            .map(|e| DecayEntryDto {
                r_eff_drop: e.fresh_r_eff - e.current_r_eff,
                artifact_id: e.artifact_id,
                artifact_title: e.artifact_title,
                current_r_eff: e.current_r_eff,
                fresh_r_eff: e.fresh_r_eff,
                expired_evidence: e
                    .expired_evidence
                    .into_iter()
                    .map(|ev| ExpiredEvidenceDto {
                        id: ev.id,
                        valid_until: ev.valid_until,
                        days_expired: ev.days_expired,
                        score: ev.individual_score,
                    })
                    .collect(),
            })
            .collect();

        Ok(json_result(&DecayResponse {
            entries: dtos,
            total_affected: total,
        }))
    }

    #[tool(description = "Suggest depth level (Tactical/Standard/Deep/Critical) for artifacts based on content analysis. Detects security sections, breaking changes, link count, body complexity.")]
    async fn forgeplan_calibrate(
        &self,
        Parameters(p): Parameters<CalibrateParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let records = store
            .list_records(None)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let to_check: Vec<&ArtifactRecord> = if let Some(ref target_id) = p.id {
            let upper = target_id.to_uppercase();
            let filtered: Vec<_> = records
                .iter()
                .filter(|r| r.id.to_uppercase() == upper)
                .collect();
            if filtered.is_empty() {
                return Ok(err_result(&format!("Artifact '{target_id}' not found")));
            }
            filtered
        } else {
            records.iter().collect()
        };

        let mut results = Vec::new();
        let mut total_escalations = 0;

        for record in &to_check {
            let link_count = store.get_relations(&record.id).await.unwrap_or_else(|e| {
                tracing::warn!("Failed to get relations for {}: {e}", record.id);
                Vec::new()
            }).len();
            let cal = forgeplan_core::depth::suggest_depth(record, link_count);

            if cal.escalation_needed {
                total_escalations += 1;
            }

            results.push(CalibrationDto {
                artifact_id: cal.artifact_id,
                artifact_title: cal.artifact_title,
                current_depth: cal.current_depth,
                suggested_depth: format!("{:?}", cal.suggested_depth),
                escalation_needed: cal.escalation_needed,
                signals: cal
                    .signals
                    .into_iter()
                    .map(|s| SignalDto {
                        name: s.name,
                        value: s.value,
                        minimum_depth: format!("{:?}", s.minimum_depth),
                    })
                    .collect(),
            });
        }

        Ok(json_result(&CalibrateResponse {
            results,
            total_escalations,
        }))
    }

    #[tool(description = "Analyze an artifact using FPF ADI reasoning cycle: Abduction (3+ hypotheses) → Deduction (evaluate each) → Induction (synthesize recommendation). Requires LLM provider.")]
    async fn forgeplan_reason(
        &self,
        Parameters(p): Parameters<ReasonParams>,
    ) -> Result<CallToolResult, McpError> {
        let ws = match self.require_workspace().await {
            Ok(ws) => ws,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let record = match store.get_record(&p.id).await {
            Ok(Some(r)) => r,
            Ok(None) => return Ok(err_result(&format!("Artifact '{}' not found", p.id))),
            Err(e) => return Ok(err_result(&format!("{e}"))),
        };

        let config = workspace::load_config(&ws)
            .map_err(|e| McpError::internal_error(format!("Config error: {e}"), None))?;
        let llm_config = config.llm.unwrap_or_default().with_env_overrides();

        let analysis = forgeplan_core::llm::reason::reason(
            &llm_config, &record.id, &record.title, &record.kind, &record.body,
        )
        .await
        .map_err(|e| McpError::internal_error(format!("ADI reasoning failed: {e}"), None))?;

        Ok(json_result(&ReasonResponse {
            artifact_id: record.id,
            artifact_title: record.title,
            analysis,
            provider: llm_config.provider,
            model: llm_config.model,
        }))
    }

    #[tool(description = "Decompose a PRD into RFC tasks using AI. Analyzes functional requirements and suggests 3-7 RFCs with titles, descriptions, scope, and dependencies. Requires LLM provider.")]
    async fn forgeplan_decompose(
        &self,
        Parameters(p): Parameters<DecomposeParams>,
    ) -> Result<CallToolResult, McpError> {
        let ws = match self.require_workspace().await {
            Ok(ws) => ws,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let record = match store.get_record(&p.id).await {
            Ok(Some(r)) => r,
            Ok(None) => return Ok(err_result(&format!("Artifact '{}' not found", p.id))),
            Err(e) => return Ok(err_result(&format!("{e}"))),
        };

        let config = workspace::load_config(&ws)
            .map_err(|e| McpError::internal_error(format!("Config error: {e}"), None))?;
        let llm_config = config.llm.unwrap_or_default().with_env_overrides();

        let tasks = forgeplan_core::llm::decompose::decompose(
            &llm_config, &record.id, &record.title, &record.body,
        )
        .await
        .map_err(|e| McpError::internal_error(format!("Decompose failed: {e}"), None))?;

        Ok(json_result(&DecomposeResponse {
            prd_id: record.id,
            prd_title: record.title,
            tasks,
            provider: llm_config.provider,
            model: llm_config.model,
        }))
    }

    #[tool(description = "Generate an artifact using AI from a natural language description. Requires LLM provider configured in .forgeplan/config.yaml. Supports OpenAI, Claude, Gemini, Ollama, and any OpenAI-compatible endpoint.")]
    async fn forgeplan_generate(
        &self,
        Parameters(p): Parameters<GenerateParams>,
    ) -> Result<CallToolResult, McpError> {
        let ws = match self.require_workspace().await {
            Ok(ws) => ws,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let artifact_kind: ArtifactKind = match p.kind.parse() {
            Ok(k) => k,
            Err(e) => return Ok(err_result(&format!("{e}"))),
        };

        let config = workspace::load_config(&ws)
            .map_err(|e| McpError::internal_error(format!("Config error: {e}"), None))?;
        let llm_config = config.llm.unwrap_or_default().with_env_overrides();

        let title = p
            .description
            .lines()
            .next()
            .unwrap_or(&p.description)
            .chars()
            .take(80)
            .collect::<String>();

        let template_key = artifact_kind.template_key();

        let body = forgeplan_core::llm::generate::generate_body(
            &llm_config,
            template_key,
            &p.description,
            &title,
        )
        .await
        .map_err(|e| McpError::internal_error(format!("LLM generation failed: {e}"), None))?;

        let prefix = artifact_kind.prefix().trim_end_matches('-').to_uppercase();
        let id = store
            .next_id(&prefix)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let artifact = NewArtifact {
            id: id.clone(),
            kind: template_key.into(),
            status: "draft".into(),
            title: title.clone(),
            body: body.clone(),
            depth: "standard".into(),
            author: None,
            parent_epic: None,
            valid_until: None,
        };

        store
            .create_artifact(&artifact)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let filepath = projection::render_projection(
            &ws, &id, template_key, &title, "draft", "standard",
            None, None, None, &body, &[],
        )
        .await
        .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        Ok(json_result(&GenerateResponse {
            id,
            kind: template_key.into(),
            title,
            filepath: filepath.display().to_string(),
            provider: llm_config.provider,
            model: llm_config.model,
        }))
    }

    #[tool(description = "Export all artifacts and relations to a JSON bundle. Returns the exported data directly for programmatic use, or writes to a file path.")]
    async fn forgeplan_export(
        &self,
        Parameters(p): Parameters<ExportParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let records = store
            .list_records(None)
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let artifacts: Vec<serde_json::Value> = records
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "kind": r.kind,
                    "status": r.status,
                    "title": r.title,
                    "body": r.body,
                    "depth": r.depth,
                    "author": r.author,
                    "parent_epic": r.parent_epic,
                    "r_eff_score": r.r_eff_score,
                    "valid_until": r.valid_until,
                    "created_at": r.created_at,
                    "updated_at": r.updated_at,
                })
            })
            .collect();

        let all_relations = store
            .get_all_relations()
            .await
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;

        let relations: Vec<serde_json::Value> = all_relations
            .into_iter()
            .map(|(s, t, r)| serde_json::json!({"source": s, "target": t, "relation": r}))
            .collect();

        let data = serde_json::json!({
            "version": 1,
            "artifacts": artifacts,
            "relations": relations,
        });

        if let Some(ref output_path) = p.output {
            let ws = match self.require_workspace().await {
                Ok(ws) => ws,
                Err(e) => return Ok(err_result(&e)),
            };
            let full_path = if std::path::Path::new(output_path).is_absolute() {
                std::path::PathBuf::from(output_path)
            } else {
                ws.parent().unwrap_or(&ws).join(output_path)
            };
            let json_str = serde_json::to_string_pretty(&data)
                .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
            tokio::fs::write(&full_path, &json_str)
                .await
                .map_err(|e| McpError::internal_error(format!("Write failed: {e}"), None))?;
            return Ok(text_result(&format!(
                "Exported {} artifacts, {} relations to {}",
                artifacts.len(),
                relations.len(),
                full_path.display()
            )));
        }

        Ok(json_result(&data))
    }

    #[tool(description = "Import artifacts and relations from a JSON export bundle. Set force=true to overwrite existing artifacts.")]
    async fn forgeplan_import(
        &self,
        Parameters(p): Parameters<ImportParams>,
    ) -> Result<CallToolResult, McpError> {
        let store = match self.require_store().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(&e)),
        };

        let data: serde_json::Value = serde_json::from_str(&p.data).map_err(|e| {
            McpError::internal_error(format!("Invalid JSON: {e}"), None)
        })?;

        let artifacts = data["artifacts"]
            .as_array()
            .ok_or_else(|| McpError::internal_error("Missing 'artifacts' array", None))?;

        let force = p.force.unwrap_or(false);
        let mut imported = 0usize;
        let mut skipped = 0usize;

        for art in artifacts {
            let id = art["id"].as_str().unwrap_or_default();
            if id.is_empty() {
                continue;
            }

            let existing = store.get_record(id).await.unwrap_or(None);
            if existing.is_some() && !force {
                skipped += 1;
                continue;
            }

            if existing.is_some() {
                let _ = store.delete_artifact(id).await;
            }

            let new_art = NewArtifact {
                id: id.to_string(),
                kind: art["kind"].as_str().unwrap_or("note").to_string(),
                status: art["status"].as_str().unwrap_or("draft").to_string(),
                title: art["title"].as_str().unwrap_or("").to_string(),
                body: art["body"].as_str().unwrap_or("").to_string(),
                depth: art["depth"].as_str().unwrap_or("standard").to_string(),
                author: art["author"].as_str().map(String::from),
                parent_epic: art["parent_epic"].as_str().map(String::from),
                valid_until: art["valid_until"].as_str().map(String::from),
            };

            if let Err(e) = store.create_artifact(&new_art).await {
                return Ok(err_result(&format!("Failed to import {}: {}", id, e)));
            }
            imported += 1;
        }

        let mut relations_imported = 0usize;
        if let Some(relations) = data["relations"].as_array() {
            for rel in relations {
                let source = rel["source"].as_str().unwrap_or_default();
                let target = rel["target"].as_str().unwrap_or_default();
                let relation = rel["relation"].as_str().unwrap_or("informs");
                if !source.is_empty() && !target.is_empty() {
                    if store.add_relation(source, target, relation).await.is_ok() {
                        relations_imported += 1;
                    }
                }
            }
        }

        Ok(json_result(&serde_json::json!({
            "imported": imported,
            "skipped": skipped,
            "relations_imported": relations_imported,
        })))
    }
}

// ── ServerHandler ────────────────────────────────────────────

#[tool_handler]
impl rmcp::ServerHandler for ForgeplanServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::default())
            .with_server_info(Implementation::new(
                "forgeplan",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Forgeplan MCP server: manage structured project artifacts \
                 (PRDs, RFCs, ADRs, Epics, Specs) with quality scoring, \
                 validation, dependency graphs, and search.",
            )
    }
}

// Evidence parsing delegated to forgeplan_core::scoring::evidence
use forgeplan_core::scoring::evidence::parse_evidence_from_record;
