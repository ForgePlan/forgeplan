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
use forgeplan_core::scoring::reff::{self, EvidenceItem, EvidenceType, Verdict};
use forgeplan_core::template::{get_embedded_template, render_template};
use forgeplan_core::validation;
use forgeplan_core::workspace;

use crate::types::*;

// ── Server struct ────────────────────────────────────────────

#[derive(Clone)]
pub struct ForgeplanServer {
    store: Arc<RwLock<Option<LanceStore>>>,
    workspace_root: PathBuf,
    workspace_path: Arc<RwLock<Option<PathBuf>>>,
    tool_router: ToolRouter<Self>,
}

impl ForgeplanServer {
    pub async fn new(workspace_root: PathBuf) -> Self {
        let ws = workspace::find_workspace(&workspace_root);
        let store = if let Some(ref ws_path) = ws {
            LanceStore::open(ws_path).await.ok()
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

    async fn require_store(&self) -> Result<tokio::sync::RwLockReadGuard<'_, Option<LanceStore>>, String> {
        let guard = self.store.read().await;
        if guard.is_none() {
            return Err("Workspace not initialized. Call forgeplan_init first.".into());
        }
        Ok(guard)
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

        *self.store.write().await = Some(new_store);
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
        let guard = match self.require_store().await {
            Ok(g) => g,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = guard.as_ref().unwrap();

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
        let guard = match self.require_store().await {
            Ok(g) => g,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = guard.as_ref().unwrap();

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
        let guard = match self.require_store().await {
            Ok(g) => g,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = guard.as_ref().unwrap();

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
        let guard = match self.require_store().await {
            Ok(g) => g,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = guard.as_ref().unwrap();

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
        let guard = match self.require_store().await {
            Ok(g) => g,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = guard.as_ref().unwrap();

        let target = match store.get_record(&p.id).await {
            Ok(Some(r)) => r,
            Ok(None) => return Ok(err_result(&format!("Artifact '{}' not found", p.id))),
            Err(e) => return Ok(err_result(&format!("{e}"))),
        };

        let outgoing = store.get_relations(&p.id).await.unwrap_or_default();
        let evidence_targets: Vec<String> = outgoing
            .iter()
            .filter(|(_, rel)| rel == "informs" || rel == "based_on" || rel == "refines")
            .map(|(t, _)| t.clone())
            .collect();

        let filter = ArtifactFilter {
            kind: Some("evidence".into()),
            status: None,
        };
        let evidence_records = store.list_records(Some(&filter)).await.unwrap_or_default();

        let mut evidence_items: Vec<EvidenceItem> = Vec::new();
        let mut evidence_dtos: Vec<EvidenceDto> = Vec::new();

        for ev in &evidence_records {
            let is_linked = evidence_targets
                .iter()
                .any(|eid| eid.eq_ignore_ascii_case(&ev.id));

            if !is_linked {
                let ev_rels = store.get_relations(&ev.id).await.unwrap_or_default();
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
        let guard = match self.require_store().await {
            Ok(g) => g,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = guard.as_ref().unwrap();

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

    #[tool(description = "Generate a mermaid dependency graph of all linked artifacts. Includes explicit links and parent_epic belongs_to edges.")]
    async fn forgeplan_graph(&self) -> Result<CallToolResult, McpError> {
        let guard = match self.require_store().await {
            Ok(g) => g,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = guard.as_ref().unwrap();

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
        let guard = match self.require_store().await {
            Ok(g) => g,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = guard.as_ref().unwrap();

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
        let guard = match self.require_store().await {
            Ok(g) => g,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = guard.as_ref().unwrap();

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
        let guard = match self.require_store().await {
            Ok(g) => g,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = guard.as_ref().unwrap();

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
        let guard = match self.require_store().await {
            Ok(g) => g,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = guard.as_ref().unwrap();

        let entries = forgeplan_core::scoring::decay::decay_report(store)
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
        Parameters(p): Parameters<CalibrateRequest>,
    ) -> Result<CallToolResult, McpError> {
        let guard = match self.require_store().await {
            Ok(g) => g,
            Err(e) => return Ok(err_result(&e)),
        };
        let store = guard.as_ref().unwrap();

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
            let link_count = store.get_relations(&record.id).await.unwrap_or_default().len();
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

// ── Evidence parsing (ported from CLI score command) ──────────

fn parse_evidence_from_record(record: &ArtifactRecord) -> EvidenceItem {
    let verdict = extract_field(&record.body, "verdict")
        .map(|s| match s.to_lowercase().as_str() {
            "supports" => Verdict::Supports,
            "weakens" => Verdict::Weakens,
            "refutes" => Verdict::Refutes,
            _ => Verdict::Supports,
        })
        .unwrap_or(Verdict::Supports);

    let cl = extract_field(&record.body, "congruence_level")
        .and_then(|s| s.parse::<u8>().ok())
        .map(|v| v.min(3))
        .unwrap_or(0);

    let valid_until = record.valid_until.as_deref().and_then(|s| {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
            .ok()
            .or_else(|| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(23, 59, 59))
            })
    });

    EvidenceItem {
        id: record.id.clone(),
        evidence_type: EvidenceType::Measurement,
        verdict,
        congruence_level: cl,
        valid_until,
    }
}

fn extract_field(body: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let val = rest.trim();
            if !val.is_empty() {
                return Some(val.into());
            }
        }
    }
    None
}
