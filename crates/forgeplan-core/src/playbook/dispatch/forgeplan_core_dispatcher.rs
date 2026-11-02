//! Production [`Dispatcher`] for `Delegation::ForgeplanCore` variant (FR-5).
//!
//! Phase 6 Wave 1 — owner: **forgeplan-core-dispatcher** teammate.
//!
//! # Design — direct internal call, no subprocess
//!
//! Unlike the four other Wave-1 dispatchers (plugin / agent / skill / command),
//! [`ForgeplanCoreDispatcher`] does **not** shell out to a binary. Each
//! `ForgeplanOp` variant maps to an existing `forgeplan-core` internal API,
//! invoked in-process. Rationale (RFC-007 §"Five dispatcher variants" row
//! `forgeplan_core`):
//!
//! - Zero process spawn overhead — heavily used by the greenfield-kickoff
//!   playbook (FR-7), which threads PRD/Spec/Validate/Activate calls over a
//!   handful of artifacts. Subprocess dispatch would add ~100–500 ms per step.
//! - The dispatcher already runs inside the same Tokio runtime as the
//!   executor, so we can `await` `LanceStore` calls without a bridge.
//! - There is no security gate to add: every op is the same code path the
//!   user invokes via `forgeplan` CLI. The caller has already accepted
//!   running the playbook; the playbook just composes commands.
//!
//! # Op mapping
//!
//! | `ForgeplanOp` | Internal API |
//! |---|---|
//! | `Ingest`   | `ingest::IngestEngine::apply` + file-first draft writes |
//! | `New`      | `projection::create_artifact_with_projection` |
//! | `Validate` | `validation::validate` (count MUST errors) |
//! | `Activate` | shared lifecycle gates + canonical status projection |
//! | `Search`   | owning read source's `search_body` |
//!
//! # `step.input` contract
//!
//! Each op expects a typed YAML input object. The fields are validated by
//! [`parse_op_input`] before any side-effectful API is called:
//!
//! - `Ingest`   → `{ mapping_path: <path>, source_path: <path>, dry_run?: bool, update?: bool }`
//! - `New`      → `{ kind: <prd|adr|...>, title: <string> }`
//! - `Validate` → `{ id: <ARTIFACT-ID> }`
//! - `Activate` → `{ id: <ARTIFACT-ID>, force?: bool }`
//! - `Search`   → `{ query: <string>, kind?: <string> }`
//!
//! Missing or wrong-shape inputs surface as [`DispatchError::Transport`] —
//! the executor treats them as abort-class step failures.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_yaml::Value as YamlValue;

use super::{DispatchError, DispatchOutcome, Dispatcher};
use crate::artifact::types::ArtifactKind;
use crate::constellation::capture::CaptureLimits;
use crate::constellation::facade::{
    MaintainCapability, ensure_ordinary_mutation_allowed, maintain_index,
    open_workspace_read_with_permit, resolve_workspace_selector_with_permit, workspace_is_migrated,
};
use crate::db::store::{LanceStore, NewArtifact};
use crate::ingest::idempotency::{PlannedAction, PlannedDraft, draft_artifact_kind, plan_drafts};
use crate::ingest::{
    IngestArtifactDraft, IngestEngine, IngestOptions, Mapping, ParsedSource, SourceSpec, parser_for,
};
use crate::lifecycle;
use crate::playbook::types::{Delegation, ForgeplanOp, Step};
use crate::projection::{self, MutationContext};
use crate::template::{get_embedded_template, render_template};
use crate::validation::{self, Severity};
use crate::workspace::lock::{WorkspaceLock, acquire_workspace_lock};

/// FR-5: Production forgeplan_core dispatcher (internal bridge).
///
/// Each operation selects the index owner before opening a read source or
/// mutation sink. Migrated workspaces use one canonical capture and file-first
/// writes; only legacy workspaces open a [`LanceStore`].
pub struct ForgeplanCoreDispatcher {
    /// Path to the workspace root (the `.forgeplan/` directory). All ops
    /// resolve relative paths from here.
    pub workspace_root: PathBuf,
}

impl ForgeplanCoreDispatcher {
    /// Construct with an explicit workspace root.
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

impl Default for ForgeplanCoreDispatcher {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

// =====================================================================
// Typed `step.input` payload
// =====================================================================

/// Typed view of `step.input` for each [`ForgeplanOp`] variant.
///
/// Constructed by [`parse_op_input`] before any I/O. Keeping the parsed
/// shape in one enum lets us do a single match in [`Dispatcher::dispatch`]
/// without re-deriving fields per op.
#[derive(Debug)]
enum OpInput {
    Ingest {
        mapping_path: PathBuf,
        source_path: PathBuf,
        dry_run: bool,
        update: bool,
    },
    New {
        kind: String,
        title: String,
    },
    Validate {
        id: String,
    },
    Activate {
        id: String,
        force: bool,
    },
    Search {
        query: String,
        kind: Option<String>,
    },
}

/// Parse the raw `step.input` YAML value into a typed [`OpInput`] for `op`.
///
/// Returns [`DispatchError::Transport`] when the input is missing entirely,
/// not a mapping, or missing required fields. Optional fields default to
/// safe values (`dry_run: false`, `update: false`, `force: false`).
fn parse_op_input(op: &ForgeplanOp, input: Option<&YamlValue>) -> Result<OpInput, DispatchError> {
    let map = match input {
        Some(YamlValue::Mapping(m)) => m,
        Some(other) => {
            return Err(DispatchError::Transport(format!(
                "ForgeplanCore step.input must be a mapping, got {:?}",
                other
            )));
        }
        None => {
            return Err(DispatchError::Transport(format!(
                "ForgeplanCore op {:?} requires step.input",
                op
            )));
        }
    };

    let get_str = |key: &str| -> Option<String> {
        map.get(YamlValue::String(key.to_string()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };
    let get_bool = |key: &str| -> Option<bool> {
        map.get(YamlValue::String(key.to_string()))
            .and_then(|v| v.as_bool())
    };

    match op {
        ForgeplanOp::Ingest => {
            let mapping_path = get_str("mapping_path").ok_or_else(|| {
                DispatchError::Transport("Ingest input missing `mapping_path`".to_string())
            })?;
            let source_path = get_str("source_path").ok_or_else(|| {
                DispatchError::Transport("Ingest input missing `source_path`".to_string())
            })?;
            Ok(OpInput::Ingest {
                mapping_path: PathBuf::from(mapping_path),
                source_path: PathBuf::from(source_path),
                dry_run: get_bool("dry_run").unwrap_or(false),
                update: get_bool("update").unwrap_or(false),
            })
        }
        ForgeplanOp::New => {
            let kind = get_str("kind")
                .ok_or_else(|| DispatchError::Transport("New input missing `kind`".to_string()))?;
            let title = get_str("title")
                .ok_or_else(|| DispatchError::Transport("New input missing `title`".to_string()))?;
            Ok(OpInput::New { kind, title })
        }
        ForgeplanOp::Validate => {
            let id = get_str("id").ok_or_else(|| {
                DispatchError::Transport("Validate input missing `id`".to_string())
            })?;
            Ok(OpInput::Validate { id })
        }
        ForgeplanOp::Activate => {
            let id = get_str("id").ok_or_else(|| {
                DispatchError::Transport("Activate input missing `id`".to_string())
            })?;
            Ok(OpInput::Activate {
                id,
                force: get_bool("force").unwrap_or(false),
            })
        }
        ForgeplanOp::Search => {
            let query = get_str("query").ok_or_else(|| {
                DispatchError::Transport("Search input missing `query`".to_string())
            })?;
            Ok(OpInput::Search {
                query,
                kind: get_str("kind"),
            })
        }
    }
}

// =====================================================================
// Dispatcher impl
// =====================================================================

#[async_trait]
impl Dispatcher for ForgeplanCoreDispatcher {
    async fn dispatch(&self, step: &Step) -> Result<DispatchOutcome, DispatchError> {
        // 1. Variant guard — caller must not route a non-ForgeplanCore step here.
        let op = match &step.delegate_to {
            Delegation::ForgeplanCore { target } => *target,
            other => {
                return Err(DispatchError::Transport(format!(
                    "ForgeplanCoreDispatcher received non-ForgeplanCore delegate: {other:?}"
                )));
            }
        };

        // 2. Parse + validate typed input before opening the store.
        let parsed = parse_op_input(&op, step.input.as_ref())?;

        // 3. Dispatch to the matching op handler.
        match parsed {
            OpInput::Ingest {
                mapping_path,
                source_path,
                dry_run,
                update,
            } => {
                run_ingest(
                    &self.workspace_root,
                    &mapping_path,
                    &source_path,
                    dry_run,
                    update,
                )
                .await
            }
            OpInput::New { kind, title } => run_new(&self.workspace_root, &kind, &title).await,
            OpInput::Validate { id } => run_validate(&self.workspace_root, &id).await,
            OpInput::Activate { id, force } => run_activate(&self.workspace_root, &id, force).await,
            OpInput::Search { query, kind } => {
                run_search(&self.workspace_root, &query, kind.as_deref()).await
            }
        }
    }
}

// =====================================================================
// Op handlers
// =====================================================================

/// Select ownership before opening: a migrated index has no legacy sink.
async fn open_legacy_sink(
    workspace_root: &Path,
    lock: &WorkspaceLock,
) -> Result<Option<LanceStore>, DispatchError> {
    if workspace_is_migrated(workspace_root) {
        return Ok(None);
    }
    LanceStore::open_for_write(workspace_root, lock)
        .await
        .map(Some)
        .map_err(|e| DispatchError::Transport(format!("LanceStore::open_for_write failed: {e}")))
}

/// The canonical edit already stands. Publication errors must not disguise
/// that success, and the caller must release its workspace lock first.
async fn reconcile_after_mutation(workspace_root: &Path) {
    let canonical = match workspace_root.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!(%error, "native playbook write stands; run `forgeplan index` to publish it");
            return;
        }
    };
    let capability = MaintainCapability::for_strict_maintenance(canonical);
    if let Err(error) = maintain_index(
        &capability,
        CaptureLimits::default(),
        std::time::Instant::now() + std::time::Duration::from_secs(120),
    )
    .await
    {
        tracing::warn!(%error, "native playbook write stands; run `forgeplan index` to publish it");
    }
}

/// The workspace guard must already be released: the drain takes its own lock.
async fn drain_after_legacy_mutation(workspace_root: &Path, outcome: &mut DispatchOutcome) {
    if let Err(error) = LanceStore::drain_identity_semantic_backlog(workspace_root).await {
        outcome.success = false;
        let detail = format!(
            "canonical mutation may have committed; semantic backlog drain failed: {error:#}"
        );
        match &mut outcome.stderr {
            Some(stderr) => {
                stderr.push_str("; ");
                stderr.push_str(&detail);
            }
            None => outcome.stderr = Some(detail),
        }
    }
}

async fn run_ingest(
    workspace_root: &Path,
    mapping_path: &Path,
    source_path: &Path,
    dry_run: bool,
    update: bool,
) -> Result<DispatchOutcome, DispatchError> {
    if !mapping_path.exists() {
        return Ok(DispatchOutcome::failure(format!(
            "mapping file not found: {}",
            mapping_path.display()
        )));
    }
    if !source_path.exists() {
        return Ok(DispatchOutcome::failure(format!(
            "source path not found: {}",
            source_path.display()
        )));
    }

    let mapping_yaml = match std::fs::read_to_string(mapping_path) {
        Ok(s) => s,
        Err(e) => {
            return Ok(DispatchOutcome::failure(format!(
                "failed to read mapping {}: {e}",
                mapping_path.display()
            )));
        }
    };
    let mapping: Mapping = match serde_yaml::from_str(&mapping_yaml) {
        Ok(m) => m,
        Err(e) => {
            return Ok(DispatchOutcome::failure(format!(
                "invalid mapping YAML: {e}"
            )));
        }
    };

    let parsed_sources = match collect_parsed_sources(&mapping, source_path) {
        Ok(s) => s,
        Err(e) => {
            return Ok(DispatchOutcome::failure(format!("source ingest: {e}")));
        }
    };

    let engine = IngestEngine::new()
        .map_err(|e| DispatchError::Transport(format!("IngestEngine::new: {e}")))?;
    let report = match engine.apply(&mapping, parsed_sources, IngestOptions { dry_run }) {
        Ok(r) => r,
        Err(e) => {
            return Ok(DispatchOutcome::failure(format!("ingest engine: {e}")));
        }
    };

    if dry_run {
        // Dry-run never writes — surface the draft count via stderr so the
        // executor's report shows what would have been produced.
        let stderr = if report.drafts.is_empty() {
            None
        } else {
            Some(format!("dry-run: {} draft(s) planned", report.drafts.len()))
        };
        return Ok(DispatchOutcome {
            success: report.errors.is_empty(),
            output_path: None,
            stderr,
        });
    }

    if !report.errors.is_empty() {
        return Ok(DispatchOutcome::failure(format!("ingest engine: {:?}", report.errors)));
    }
    if report.drafts.is_empty() {
        return Ok(DispatchOutcome::success());
    }
    let lock = acquire_workspace_lock(workspace_root)
        .await
        .map_err(|e| DispatchError::Transport(format!("workspace lock: {e}")))?;
    let kinds: Vec<_> = report.drafts.iter().map(|draft| draft_artifact_kind(&draft.kind)).collect();
    let allocation = crate::artifact::id_alloc::acquire_allocation_guard(workspace_root, &lock, &kinds)
        .await
        .map_err(|e| DispatchError::Transport(format!("allocator lock: {e}")))?;
    let before_store = if workspace_is_migrated(workspace_root) {
        None
    } else {
        Some(LanceStore::open(workspace_root).await.map_err(|e| {
            DispatchError::Transport(format!("legacy read: {e}"))
        })?)
    };
    let before = match plan_drafts(
        workspace_root, &lock, &allocation, before_store.as_ref(), &report.drafts, update, None,
    ).await {
        Ok(plan) => plan,
        Err(error) => return Ok(DispatchOutcome::failure(format!("ingest preflight: {error:#}"))),
    };
    drop(before_store);
    if before.iter().all(|draft| matches!(draft.action, PlannedAction::Skip { .. })) {
        return Ok(DispatchOutcome::success());
    }
    ensure_ordinary_mutation_allowed(workspace_root)
        .map_err(|e| DispatchError::Transport(format!("mutation refused: {e}")))?;
    let store = match open_legacy_sink(workspace_root, &lock).await {
        Ok(store) => store,
        Err(error) => return Ok(DispatchOutcome::failure(format!(
            "index preparation failed; legacy migration may be partial: {error}"
        ))),
    };
    let migrated = store.is_none();
    let plan = plan_drafts(
        workspace_root, &lock, &allocation, store.as_ref(), &report.drafts, update, Some(&before),
    ).await;
    let mut last_path = None;
    let mut write_errors = Vec::new();
    let mut attempted = false;
    match plan {
        Ok(plan) => {
            let ctx = MutationContext::new(workspace_root, &lock, store.as_ref());
            for (draft, planned) in report.drafts.iter().zip(&plan) {
                if matches!(planned.action, PlannedAction::Skip { .. }) {
                    continue;
                }
                if let Err(error) = ensure_ordinary_mutation_allowed(workspace_root) {
                    write_errors.push(format!("mutation refused: {error}"));
                    break;
                }
                attempted = true;
                match write_ingest_draft(&ctx, &allocation, draft, planned).await {
                    Ok(path) => last_path = Some(path),
                    Err(error) => {
                        write_errors.push(format!("{}: {error:#}; canonical mutation may have committed", draft.title));
                        break;
                    }
                }
            }
            if write_errors.is_empty() {
                for (draft, planned) in report.drafts.iter().zip(&plan) {
                    let key = match &planned.action {
                        PlannedAction::Skip { .. } => continue,
                        PlannedAction::Create { key, .. } | PlannedAction::Update { key, .. } => key,
                    };
                    for link in &planned.links {
                        if let Err(error) = ensure_ordinary_mutation_allowed(workspace_root) {
                            write_errors.push(format!("mutation refused: {error}"));
                            break;
                        }
                        attempted = true;
                        if let Err(error) = projection::add_link_with_projection(
                            &ctx, key, &link.target, &link.relation,
                        ).await {
                            write_errors.push(format!("{}: {error}; canonical mutation may have committed", draft.title));
                            break;
                        }
                    }
                    if !write_errors.is_empty() {
                        break;
                    }
                }
            }
        }
        Err(error) => write_errors.push(format!("ingest preflight changed after index preparation: {error:#}")),
    }
    drop(store);
    drop(allocation);
    drop(lock);
    let mut outcome = DispatchOutcome {
        success: write_errors.is_empty(),
        output_path: last_path,
        stderr: if write_errors.is_empty() { None } else { Some(write_errors.join("; ")) },
    };
    if migrated && attempted {
        reconcile_after_mutation(workspace_root).await;
    } else if !migrated {
        drain_after_legacy_mutation(workspace_root, &mut outcome).await;
    }
    Ok(outcome)
}

/// Walk `source_path` (file or dir) and parse every file matching one of
/// the mapping's `sources[*].pattern` globs. Mirrors the CLI implementation
/// in `forgeplan-cli/src/commands/ingest.rs` but trimmed to the shape
/// needed inside the dispatcher (no resource-limit guards, no soft-warning
/// fallback — failures abort the op instead of skipping).
fn collect_parsed_sources(
    mapping: &Mapping,
    source_path: &Path,
) -> anyhow::Result<Vec<ParsedSource>> {
    let mut out: Vec<ParsedSource> = Vec::new();
    if mapping.sources.is_empty() {
        return Ok(out);
    }

    if source_path.is_file() {
        let spec = pick_spec_for_path(&mapping.sources, source_path).unwrap_or(&mapping.sources[0]);
        if let Some(parsed) = parse_one(spec, source_path)? {
            out.push(parsed);
        }
        return Ok(out);
    }

    let files = walk_files(source_path)?;
    for file in files {
        let rel = file.strip_prefix(source_path).unwrap_or(&file);
        let mut matched: Option<&SourceSpec> = None;
        for spec in &mapping.sources {
            if simple_glob_match(&spec.pattern, rel) || simple_glob_match(&spec.pattern, &file) {
                matched = Some(spec);
                break;
            }
        }
        if let Some(spec) = matched
            && let Some(parsed) = parse_one(spec, &file)?
        {
            out.push(parsed);
        }
    }
    Ok(out)
}

fn pick_spec_for_path<'a>(specs: &'a [SourceSpec], path: &Path) -> Option<&'a SourceSpec> {
    specs.iter().find(|s| simple_glob_match(&s.pattern, path))
}

fn parse_one(spec: &SourceSpec, path: &Path) -> anyhow::Result<Option<ParsedSource>> {
    let content = std::fs::read_to_string(path)?;
    let parser = parser_for(&spec.parser);
    match parser.parse(path, &content) {
        Ok(p) => Ok(Some(p)),
        Err(_) => Ok(None),
    }
}

fn walk_files(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let ft = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() {
                out.push(path);
            }
        }
    }
    Ok(out)
}

fn simple_glob_match(pattern: &str, path: &Path) -> bool {
    let path_str = path.to_string_lossy().replace('\\', "/");
    glob_match_str(pattern, &path_str)
}

fn glob_match_str(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    glob_match_inner(&p, 0, &t, 0)
}

fn glob_match_inner(p: &[char], pi: usize, t: &[char], ti: usize) -> bool {
    if pi == p.len() {
        return ti == t.len();
    }
    match p[pi] {
        '*' => {
            let is_double = pi + 1 < p.len() && p[pi + 1] == '*';
            if is_double {
                let mut rest_start = pi + 2;
                if rest_start < p.len() && p[rest_start] == '/' {
                    rest_start += 1;
                }
                if glob_match_inner(p, rest_start, t, ti) {
                    return true;
                }
                let mut k = ti;
                while k < t.len() {
                    k += 1;
                    if glob_match_inner(p, rest_start, t, k) {
                        return true;
                    }
                }
                false
            } else {
                if glob_match_inner(p, pi + 1, t, ti) {
                    return true;
                }
                let mut k = ti;
                while k < t.len() && t[k] != '/' {
                    k += 1;
                    if glob_match_inner(p, pi + 1, t, k) {
                        return true;
                    }
                }
                false
            }
        }
        '?' => {
            if ti < t.len() && t[ti] != '/' {
                glob_match_inner(p, pi + 1, t, ti + 1)
            } else {
                false
            }
        }
        c => {
            if ti < t.len() && t[ti] == c {
                glob_match_inner(p, pi + 1, t, ti + 1)
            } else {
                false
            }
        }
    }
}

async fn write_ingest_draft(
    ctx: &MutationContext<'_>,
    allocation: &crate::artifact::id_alloc::AllocationGuard,
    draft: &IngestArtifactDraft,
    planned: &PlannedDraft,
) -> anyhow::Result<PathBuf> {
    let (expected_id, key) = match &planned.action {
        PlannedAction::Skip { .. } => anyhow::bail!("cannot write a skipped draft"),
        PlannedAction::Update { key, .. } => {
            let resolved = crate::artifact::documents::resolve_in_workspace(ctx.workspace, key)
                .await?
                .ok_or_else(|| anyhow::anyhow!("ingest target '{key}' is missing"))?;
            projection::update_body_with_projection(ctx, key, &draft.body).await?;
            return Ok(ctx.workspace.join(resolved.relative_path));
        }
        PlannedAction::Create { id, key } => (id, key),
    };
    let kind = draft_artifact_kind(&draft.kind);
    let template_key = kind.template_key();
    let build = |id: &str, _number: u32, key: &str| -> anyhow::Result<NewArtifact> {
        Ok(NewArtifact {
            canonical_key: key.to_string(),
            id: id.to_string(),
            kind: template_key.to_string(),
            status: "draft".to_string(),
            title: draft.title.clone(),
            body: draft.body.clone(),
            depth: "tactical".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: vec![format!("source=ingest:{}", draft.rule_id)],
        })
    };
    let created = crate::artifact::id_alloc::create_artifact_with_prepared_key(
        ctx.workspace, ctx.guard, allocation, ctx.legacy_sink, &kind, 3,
        key.clone(), expected_id, &build,
    ).await?;
    Ok(created.filepath)
}

async fn run_new(
    workspace_root: &Path,
    kind_str: &str,
    title: &str,
) -> Result<DispatchOutcome, DispatchError> {
    if title.trim().is_empty() {
        return Ok(DispatchOutcome::failure("title cannot be empty"));
    }
    let kind: ArtifactKind = match kind_str.parse() {
        Ok(k) => k,
        Err(e) => {
            return Ok(DispatchOutcome::failure(format!(
                "invalid kind `{kind_str}`: {e}"
            )));
        }
    };

    let template_key = kind.template_key();
    let template = match get_embedded_template(template_key) {
        Some(t) => t,
        None => {
            return Ok(DispatchOutcome::failure(format!(
                "no template for kind `{template_key}`"
            )));
        }
    };
    let lock = acquire_workspace_lock(workspace_root)
        .await
        .map_err(|e| DispatchError::Transport(format!("workspace lock: {e}")))?;
    ensure_ordinary_mutation_allowed(workspace_root)
        .map_err(|e| DispatchError::Transport(format!("mutation refused: {e}")))?;
    let store = match open_legacy_sink(workspace_root, &lock).await {
        Ok(store) => store,
        Err(error) => return Ok(DispatchOutcome::failure(format!(
            "index preparation failed; legacy migration may be partial: {error}"
        ))),
    };
    if store.is_none() {
        open_workspace_read_with_permit(workspace_root, &lock)
            .await
            .map_err(|e| DispatchError::Transport(format!("canonical source: {e}")))?;
    }


    let depth = match kind {
        ArtifactKind::Note
        | ArtifactKind::EvidencePack
        | ArtifactKind::ProblemCard
        | ArtifactKind::SolutionPortfolio
        | ArtifactKind::RefreshReport => "tactical",
        _ => "standard",
    };

    let build = |id: &str, _number: u32, key: &str| -> anyhow::Result<NewArtifact> {
        let nnn = id.split('-').next_back().unwrap_or("001").to_string();
        let mut vars = HashMap::new();
        vars.insert("NNN".to_string(), nnn);
        vars.insert("title".to_string(), title.to_string());
        vars.insert("Title".to_string(), title.to_string());
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let mut rendered = render_template(template, &vars);
        rendered = rendered.replace("YYYY-MM-DD", &today);
        Ok(NewArtifact {
            canonical_key: key.to_string(),
            id: id.to_string(),
            kind: template_key.to_string(),
            status: "draft".to_string(),
            title: title.to_string(),
            body: rendered,
            depth: depth.to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        })
    };
    // One creation mechanism: mint the canonical key, reserve the display
    // number under the shared per-kind ledger, and publish the document
    // under its key.
    ensure_ordinary_mutation_allowed(workspace_root)
        .map_err(|e| DispatchError::Transport(format!("mutation refused: {e}")))?;
    let created = crate::artifact::id_alloc::create_artifact_with_minted_key(
        workspace_root,
        &lock,
        store.as_ref(),
        &kind,
        3,
        title,
        &build,
    )
    .await;
    let migrated = store.is_none();
    drop(store);
    drop(lock);
    let mut outcome = match created {
        Ok(created) => DispatchOutcome {
            success: true,
            output_path: Some(created.filepath),
            stderr: None,
        },
        Err(error) => DispatchOutcome::failure(format!(
            "create_artifact failed (canonical mutation may have committed): {error}"
        )),
    };
    if migrated {
        reconcile_after_mutation(workspace_root).await;
    } else {
        drain_after_legacy_mutation(workspace_root, &mut outcome).await;
    }
    Ok(outcome)
}

async fn run_validate(workspace_root: &Path, id: &str) -> Result<DispatchOutcome, DispatchError> {
    let validate_record =
        |record: &crate::db::store::ArtifactRecord,
         fm: &crate::artifact::frontmatter::Frontmatter| {
            let kind = record
                .kind
                .parse::<ArtifactKind>()
                .unwrap_or(ArtifactKind::Note);
            let depth = record
                .depth
                .parse()
                .unwrap_or(crate::artifact::types::Mode::Standard);
            validation::validate(&record.id, &record.body, fm, &kind, &depth)
        };
    let lock = acquire_workspace_lock(workspace_root)
        .await
        .map_err(|e| DispatchError::Transport(format!("workspace lock: {e}")))?;
    let result = if workspace_is_migrated(workspace_root) {
        let read = open_workspace_read_with_permit(workspace_root, &lock).await
            .map_err(|e| DispatchError::Transport(format!("canonical source: {e}")))?;
        match read.resolve_artifact(id) {
            Ok(Some(resolved)) => read.get_record_by_key(&resolved.identity.canonical_key)
                .map(|record| validate_record(record, &read.frontmatter_map_with_links(record))),
            Ok(None) => None,
            Err(error) => return Ok(DispatchOutcome::failure(format!("validate: {error}"))),
        }
    } else {
        let store = LanceStore::open(workspace_root).await
            .map_err(|e| DispatchError::Transport(format!("legacy read: {e}")))?;
        match store.get_record(&lock, id).await
            .map_err(|e| DispatchError::Transport(format!("get_record: {e}")))?
        {
            Some(record) => {
                let fm = store.frontmatter_map_with_links(&lock, &record).await
                    .map_err(|e| DispatchError::Transport(format!("frontmatter: {e}")))?;
                Some(validate_record(&record, &fm))
            }
            None => None,
        }
    };
    let Some(result) = result else {
        return Ok(DispatchOutcome::failure(format!(
            "artifact not found: {id}"
        )));
    };
    let must_errors: Vec<String> = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Must)
        .map(|f| format!("{}: {}", f.rule_id, f.message))
        .collect();
    let success = must_errors.is_empty();
    let stderr = if must_errors.is_empty() {
        None
    } else {
        Some(format!(
            "{} MUST error(s): {}",
            must_errors.len(),
            must_errors.join("; ")
        ))
    };
    Ok(DispatchOutcome {
        success,
        output_path: None,
        stderr,
    })
}

async fn run_activate(
    workspace_root: &Path,
    id: &str,
    force: bool,
) -> Result<DispatchOutcome, DispatchError> {
    let lock = acquire_workspace_lock(workspace_root)
        .await
        .map_err(|e| DispatchError::Transport(format!("workspace lock: {e}")))?;
    let key = match resolve_workspace_selector_with_permit(workspace_root, &lock, id).await {
        Ok(Some(resolved)) => resolved.identity.canonical_key,
        Ok(None) => return Ok(DispatchOutcome::failure(format!("Artifact not found: {id}"))),
        Err(error) => return Ok(DispatchOutcome::failure(format!("activate: {error}"))),
    };
    let read = open_workspace_read_with_permit(workspace_root, &lock)
        .await
        .map_err(|e| DispatchError::Transport(format!("canonical source: {e}")))?;
    ensure_ordinary_mutation_allowed(workspace_root)
        .map_err(|e| DispatchError::Transport(format!("mutation refused: {e}")))?;
    let store = match open_legacy_sink(workspace_root, &lock).await {
        Ok(store) => store,
        Err(error) => return Ok(DispatchOutcome::failure(format!(
            "index preparation failed; legacy migration may be partial: {error}"
        ))),
    };
    ensure_ordinary_mutation_allowed(workspace_root)
        .map_err(|e| DispatchError::Transport(format!("mutation refused: {e}")))?;
    let result = lifecycle::canonical::activate_captured(
        &MutationContext::new(workspace_root, &lock, store.as_ref()),
        &read,
        &key,
        force,
    )
    .await;
    let migrated = store.is_none();
    drop(store);
    drop(lock);
    if migrated && result.is_ok() {
        reconcile_after_mutation(workspace_root).await;
    }
    let mut outcome = match result {
        Ok(result) => DispatchOutcome {
            success: true,
            output_path: None,
            stderr: if result.must_errors.is_empty() {
                None
            } else {
                Some(format!(
                    "activated with {} forced MUST error(s)",
                    result.must_errors.len()
                ))
            },
        },
        Err(e) => DispatchOutcome::failure(format!(
            "activate failed (canonical mutation may have committed): {e}"
        )),
    };
    if !migrated {
        drain_after_legacy_mutation(workspace_root, &mut outcome).await;
    }
    Ok(outcome)
}

async fn run_search(
    workspace_root: &Path,
    query: &str,
    kind_filter: Option<&str>,
) -> Result<DispatchOutcome, DispatchError> {
    let lock = acquire_workspace_lock(workspace_root)
        .await
        .map_err(|e| DispatchError::Transport(format!("workspace lock: {e}")))?;
    let hits = if workspace_is_migrated(workspace_root) {
        open_workspace_read_with_permit(workspace_root, &lock)
            .await
            .map_err(|e| DispatchError::Transport(format!("canonical source: {e}")))?
            .search_body(query, kind_filter)
    } else {
        let store = LanceStore::open(workspace_root).await
            .map_err(|e| DispatchError::Transport(format!("legacy read: {e}")))?;
        store.search_body(&lock, query, kind_filter).await
            .map_err(|e| DispatchError::Transport(format!("search_body: {e}")))?
    };
    let json: Vec<serde_json::Value> = hits
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "kind": r.kind,
                "status": r.status,
                "title": r.title,
            })
        })
        .collect();
    let payload = serde_json::json!({ "results": json });
    let stdout = serde_json::to_string(&payload)
        .map_err(|e| DispatchError::Transport(format!("json encode: {e}")))?;
    Ok(DispatchOutcome {
        success: true,
        output_path: None,
        // Stash stdout in `stderr` for now — `DispatchOutcome` does not have
        // a dedicated stdout field. Executor's report aggregator surfaces
        // both fields, and the test contract pins the JSON payload there.
        stderr: Some(stdout),
    })
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playbook::types::{Delegation, OnError};
    use crate::workspace::init_workspace;
    use serde_yaml::Mapping as YamlMapping;
    use tempfile::TempDir;

    /// Spin up a temp workspace with a fully-initialised LanceDB so the
    /// dispatcher's ops can run end-to-end. Returns (TempDir guard,
    /// workspace_root path). The TempDir must be kept alive for the
    /// duration of the test — drop deletes the workspace.
    async fn setup_workspace() -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("tempdir");
        let ws = init_workspace(dir.path(), "phase6-fcd-test").expect("init");
        let lock = acquire_workspace_lock(&ws).await.expect("init lock");
        let store = LanceStore::init_new(&ws, &lock).await.expect("lance init");
        drop(store);
        drop(lock);
        (dir, ws)
    }

    fn make_step(id: &str, op: ForgeplanOp, input: Option<YamlValue>) -> Step {
        Step {
            id: id.to_string(),
            delegate_to: Delegation::ForgeplanCore { target: op },
            input,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: OnError::Abort,
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        }
    }

    fn yaml_map(pairs: &[(&str, YamlValue)]) -> YamlValue {
        let mut m = YamlMapping::new();
        for (k, v) in pairs {
            m.insert(YamlValue::String((*k).to_string()), v.clone());
        }
        YamlValue::Mapping(m)
    }

    #[tokio::test]
    async fn activation_rechecks_selector_after_prior_admission() {
        let (_dir, ws) = setup_workspace().await;
        let created = run_new(&ws, "note", "Admission race").await.unwrap();
        assert!(created.success, "{:?}", created.stderr);
        let path = created.output_path.unwrap();
        let original = std::fs::read(&path).unwrap();
        let guard = acquire_workspace_lock(&ws).await.unwrap();
        let admitted = resolve_workspace_selector_with_permit(&ws, &guard, "NOTE-001")
            .await.unwrap().unwrap();
        drop(guard);

        let duplicate = path.with_file_name("duplicate-admission-race.md");
        std::fs::write(&duplicate, &original).unwrap();
        let outcome = run_activate(&ws, &admitted.identity.canonical_key, false).await.unwrap();
        assert!(!outcome.success);
        let message = outcome.stderr.unwrap();
        assert!(message.contains("claimed by 2 documents"), "{message}");
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read(&duplicate).unwrap(), original);
    }

    #[tokio::test]
    async fn failed_semantic_drain_reports_already_committed_output() {
        let (_dir, ws) = setup_workspace().await;
        let mut outcome = run_new(&ws, "note", "Committed before drain").await.unwrap();
        assert!(outcome.success, "{:?}", outcome.stderr);
        let path = outcome.output_path.clone().unwrap();
        let committed = std::fs::read(&path).unwrap();
        std::fs::write(ws.join("config.yaml"), "invalid: [\n").unwrap();
        drain_after_legacy_mutation(&ws, &mut outcome).await;
        assert!(!outcome.success);
        assert_eq!(outcome.output_path.as_ref(), Some(&path));
        assert!(outcome.stderr.unwrap().contains("semantic backlog drain failed"));
        assert_eq!(std::fs::read(path).unwrap(), committed);
    }

    #[cfg(not(feature = "semantic-search"))]
    #[tokio::test]
    async fn new_reports_semantic_failure_after_canonical_commit() {
        let (_dir, ws) = setup_workspace().await;
        let config = ws.join("config.yaml");
        let original = std::fs::read_to_string(&config).unwrap();
        std::fs::write(&config, format!("{original}\nembedding:\n  model: bge-m3\n")).unwrap();

        let outcome = run_new(&ws, "note", "Semantic failure").await.unwrap();
        assert!(!outcome.success, "semantic drain unexpectedly succeeded");
        let path = outcome.output_path.expect("committed output remains visible");
        assert!(std::fs::read_to_string(path).unwrap().contains("Semantic failure"));
        assert!(outcome.stderr.unwrap().contains("semantic backlog drain failed"));
    }

    #[tokio::test]
    async fn dry_run_does_not_touch_or_create_the_workspace() {
        let dir = TempDir::new().unwrap();
        let ws = dir.path().join("missing-workspace");
        let mapping = dir.path().join("mapping.yaml");
        let source = dir.path().join("source.md");
        std::fs::write(&mapping, r#"
schema_version: "1.0"
name: dry-run-test
title: "Dry run test"
compat_spec_version: "c4-architecture: ^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: "*.md"
    type: markdown
    parser: front_matter_plus_sections
rules:
  - id: note
    when:
      file_glob: "*.md"
    target: { kind: note }
    fields:
      title: "Dry run"
      summary: "Test"
"#).unwrap();
        std::fs::write(&source, "# Input\n\nText\n").unwrap();
        let outcome = run_ingest(&ws, &mapping, &source, true, false).await.unwrap();
        assert!(outcome.success, "{:?}", outcome.stderr);
        assert!(!ws.exists());
    }

    /// Construction defaults to cwd-relative workspace root.
    #[test]
    fn default_impl_uses_cwd() {
        let d = ForgeplanCoreDispatcher::default();
        assert_eq!(d.workspace_root, PathBuf::from("."));
    }

    /// Wrong delegate variant is a programming error → Transport.
    #[tokio::test]
    async fn forgeplan_core_dispatcher_rejects_non_core_delegation() {
        let d = ForgeplanCoreDispatcher::new(PathBuf::from("."));
        let step = Step {
            id: "wrong".to_string(),
            delegate_to: Delegation::Agent {
                name: "a".to_string(),
            },
            input: None,
            produces_at: None,
            mapping: None,
            requires: None,
            fallback_hint: None,
            on_error: OnError::Abort,
            timeout_seconds: None,
            budget_usd: None,
            allowed_tools: None,
        };
        let err = d.dispatch(&step).await.expect_err("must reject");
        match err {
            DispatchError::Transport(msg) => {
                assert!(msg.contains("non-ForgeplanCore"), "unexpected msg: {msg}");
            }
            other => panic!("expected Transport, got {other:?}"),
        }
    }

    /// `New { kind: note, title: "test" }` → success + output_path created.
    #[tokio::test]
    async fn forgeplan_core_dispatcher_handles_new_op() {
        let (_dir, ws) = setup_workspace().await;
        let d = ForgeplanCoreDispatcher::new(ws.clone());
        let input = yaml_map(&[
            ("kind", YamlValue::String("note".to_string())),
            (
                "title",
                YamlValue::String("Phase6 dispatcher test".to_string()),
            ),
        ]);
        let step = make_step("new-1", ForgeplanOp::New, Some(input));
        let outcome = d.dispatch(&step).await.expect("dispatch ok");
        assert!(outcome.success, "stderr: {:?}", outcome.stderr);
        let path = outcome.output_path.expect("output_path");
        assert!(path.exists(), "projection at {} missing", path.display());
        // File should live under .forgeplan/notes/.
        assert!(
            path.to_string_lossy().contains("notes"),
            "expected notes dir, got {}",
            path.display()
        );
    }

    /// `Validate` on a freshly-created Note — Notes have minimal MUST rules,
    /// so a default note created via the `New` op validates clean.
    #[tokio::test]
    async fn forgeplan_core_dispatcher_handles_validate_op_pass() {
        let (_dir, ws) = setup_workspace().await;
        let d = ForgeplanCoreDispatcher::new(ws.clone());
        // Seed a note via New op so we have a known-good ID.
        let new_input = yaml_map(&[
            ("kind", YamlValue::String("note".to_string())),
            ("title", YamlValue::String("Smoke note".to_string())),
        ]);
        let new_step = make_step("seed", ForgeplanOp::New, Some(new_input));
        let new_outcome = d.dispatch(&new_step).await.expect("seed new");
        assert!(new_outcome.success);
        // Pull the actual ID from the store — projection path encodes it.
        let read_lock = acquire_workspace_lock(&ws).await.expect("read lock");
        let store = LanceStore::open(&ws).await.expect("reopen");
        let summaries = store.list_artifacts(&read_lock, None).await.expect("list");
        let id = summaries
            .iter()
            .find(|s| s.kind == "note")
            .expect("note exists")
            .id
            .clone();
        drop(read_lock);

        let val_input = yaml_map(&[("id", YamlValue::String(id.clone()))]);
        let val_step = make_step("val-1", ForgeplanOp::Validate, Some(val_input));
        let outcome = d.dispatch(&val_step).await.expect("dispatch ok");
        assert!(outcome.success, "stderr: {:?}", outcome.stderr);
    }

    /// `Validate { id: nonexistent }` → success=false + stderr populated.
    #[tokio::test]
    async fn forgeplan_core_dispatcher_handles_validate_op_fail() {
        let (_dir, ws) = setup_workspace().await;
        let d = ForgeplanCoreDispatcher::new(ws.clone());
        let input = yaml_map(&[("id", YamlValue::String("PRD-9999".to_string()))]);
        let step = make_step("val-miss", ForgeplanOp::Validate, Some(input));
        let outcome = d.dispatch(&step).await.expect("dispatch ok");
        assert!(!outcome.success, "expected failure for missing artifact");
        let stderr = outcome.stderr.expect("stderr populated");
        assert!(
            stderr.contains("not found") || stderr.contains("PRD-9999"),
            "unexpected stderr: {stderr}"
        );
    }

    /// Activate a draft Note → success=true. Notes skip the validation
    /// gate (`supports_lifecycle("note") == false`), so this exercises
    /// the lightweight-kind branch of `lifecycle::activate`.
    #[tokio::test]
    async fn forgeplan_core_dispatcher_handles_activate_op() {
        let (_dir, ws) = setup_workspace().await;
        let d = ForgeplanCoreDispatcher::new(ws.clone());
        // Seed a note.
        let new_input = yaml_map(&[
            ("kind", YamlValue::String("note".to_string())),
            ("title", YamlValue::String("Activate me".to_string())),
        ]);
        let new_step = make_step("seed-act", ForgeplanOp::New, Some(new_input));
        d.dispatch(&new_step).await.expect("seed new");

        let read_lock = acquire_workspace_lock(&ws).await.expect("read lock");
        let store = LanceStore::open(&ws).await.expect("reopen");
        let id = store
            .list_artifacts(&read_lock, None)
            .await
            .expect("list")
            .into_iter()
            .find(|s| s.kind == "note")
            .expect("note")
            .id;
        drop(read_lock);

        let act_input = yaml_map(&[("id", YamlValue::String(id.clone()))]);
        let act_step = make_step("act-1", ForgeplanOp::Activate, Some(act_input));
        let outcome = d.dispatch(&act_step).await.expect("dispatch ok");
        assert!(outcome.success, "stderr: {:?}", outcome.stderr);

        // Confirm status flipped in the store.
        let read_lock = acquire_workspace_lock(&ws).await.expect("read lock");
        let store2 = LanceStore::open(&ws).await.expect("reopen2");
        let after = store2.get_record(&read_lock, &id).await.expect("get").expect("exists");
        assert_eq!(after.status, "active");
    }

    /// `Search { query: "playbook" }` → success + stdout JSON-encoded.
    #[tokio::test]
    async fn forgeplan_core_dispatcher_handles_search_op() {
        let (_dir, ws) = setup_workspace().await;
        let d = ForgeplanCoreDispatcher::new(ws.clone());
        // Seed a note containing the query token in its title.
        let new_input = yaml_map(&[
            ("kind", YamlValue::String("note".to_string())),
            ("title", YamlValue::String("Playbook smoke".to_string())),
        ]);
        let new_step = make_step("seed-search", ForgeplanOp::New, Some(new_input));
        d.dispatch(&new_step).await.expect("seed new");

        let input = yaml_map(&[("query", YamlValue::String("playbook".to_string()))]);
        let step = make_step("search-1", ForgeplanOp::Search, Some(input));
        let outcome = d.dispatch(&step).await.expect("dispatch ok");
        assert!(outcome.success);
        let stdout = outcome.stderr.expect("stdout payload");
        let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid json payload");
        let arr = parsed
            .get("results")
            .and_then(|v| v.as_array())
            .expect("results array");
        assert!(
            arr.iter().any(|r| r
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_lowercase()
                .contains("playbook")),
            "expected playbook hit in: {arr:?}"
        );
    }

    /// `parse_op_input` rejects missing required fields with a clear message.
    #[test]
    fn parse_op_input_rejects_missing_fields() {
        // New without `kind`.
        let input = yaml_map(&[("title", YamlValue::String("x".to_string()))]);
        let err = parse_op_input(&ForgeplanOp::New, Some(&input)).expect_err("must error");
        match err {
            DispatchError::Transport(msg) => assert!(msg.contains("`kind`"), "msg: {msg}"),
            other => panic!("expected Transport, got {other:?}"),
        }

        // Validate with no input at all.
        let err = parse_op_input(&ForgeplanOp::Validate, None).expect_err("must error");
        match err {
            DispatchError::Transport(msg) => {
                assert!(msg.contains("requires step.input"), "msg: {msg}");
            }
            other => panic!("expected Transport, got {other:?}"),
        }

        // Search with non-mapping input.
        let scalar = YamlValue::String("just-a-string".to_string());
        let err = parse_op_input(&ForgeplanOp::Search, Some(&scalar)).expect_err("must error");
        match err {
            DispatchError::Transport(msg) => {
                assert!(msg.contains("must be a mapping"), "msg: {msg}");
            }
            other => panic!("expected Transport, got {other:?}"),
        }
    }
}
