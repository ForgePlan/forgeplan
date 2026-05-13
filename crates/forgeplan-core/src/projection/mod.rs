//! ADR-003 file-first projection — markdown is source of truth, LanceDB
//! is a derived index that MUST be re-projected from the file after every
//! structural mutation.
//!
//! # Mental model
//!
//! Forgeplan stores artifacts (PRDs, RFCs, Evidence, Problems, etc.) as
//! Markdown files under `.forgeplan/<kind>s/<ID>-<slug>.md`. A LanceDB
//! sidecar index at `.forgeplan/lance/` provides fast queries over the
//! same artifacts (full-text search, R_eff scoring, link graph). The
//! ADR-003 invariant is that **the markdown file is authoritative**:
//! every mutator MUST update the file first (or in the same transaction
//! as the index), and any divergence between file и index gets reconciled
//! by re-projecting from the file via `forgeplan reindex`.
//!
//! Pre-PROB-048 closure (PRD-073), command handlers in `forgeplan-cli/`
//! and `forgeplan-mcp/` called `LanceStore::create_artifact / update_* /
//! delete_* / add_relation / delete_relation` directly. Each direct write
//! bypassed the file-first contract — the file и index would silently
//! drift on each unfortunate code path. Phase 3a + 3b + 4 of PRD-073
//! locked these mutations behind the `projection::*` helpers in this
//! module и enforced the boundary at the type level (`pub(crate)`
//! visibility on the raw `LanceStore` mutation API plus a regression
//! test that grep's for the forbidden call patterns).
//!
//! # Helper categories
//!
//! Mutator commands MUST funnel through one of these helpers; never call
//! `LanceStore::*` mutation methods directly from `commands/` or
//! `server.rs`:
//!
//! | Category | Helpers | What they do |
//! |---|---|---|
//! | Create | `create_artifact_with_projection` | Render markdown, then sync into LanceDB |
//! | Update | `update_body_with_projection`, `update_status_with_projection`, `update_*_with_projection` | Modify file, then re-project the row |
//! | Delete | `delete_artifact_with_projection` | Move file to `.forgeplan/.deleted/`, then drop row |
//! | Link | `add_link_with_projection`, `delete_link_with_projection` | Update both sides' frontmatter, then add/delete relation row |
//! | Re-render | `render_projection`, `render_after_mutation`, `sync_before_mutation` | Idempotent file↔store reconciliation used by `activate`, `supersede`, etc. |
//!
//! All mutator helpers take a [`MutationContext`] (workspace path + store
//! reference) so callers don't have to thread two arguments через every
//! invocation.
//!
//! # Failure semantics
//!
//! Helpers return [`MutationResult<T>`] which surfaces typed
//! [`MutationError`] (not `anyhow::Error`) — callers pattern-match on
//! `StoreTransient` (retryable) vs `StoreFatal` (don't retry) per
//! PROB-049 typed-errors lineage. CLI consumers map `MutationError` into
//! a `Fix:` hint per PRD-071 Hint Protocol.
//!
//! # Why this module exists at all
//!
//! Without this single point of indirection, ADR-003 would be enforced
//! by convention only — every new command author would have to remember
//! to call `LanceStore::*` "the right way". The compile-enforced boundary
//! plus the regression test (`tests/adr_003_invariant.rs`) means new
//! direct mutations get caught at code review time, not at production-
//! workspace-corruption time.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::artifact::frontmatter::{self, Frontmatter};
use crate::artifact::types::{ArtifactKind, slugify};

mod context;
pub mod error;
pub use context::MutationContext;
pub use error::{MutationError, MutationResult};
// Wave 9 SEC-H3: re-export the error-chain sanitiser at the projection
// module root so `forgeplan-mcp::server::safe_mcp_error` can reach it
// without spelling out the `error::` submodule.
pub use error::sanitize_error_chain;

/// Compute the on-disk filename slug for a given artifact title.
///
/// Wraps `slugify` with the `untitled` fallback used by Phase 3c
/// (audit LOW-5): when `slugify` produces an empty string — for
/// titles that are entirely non-ASCII (Cyrillic, CJK, emoji) — we
/// substitute `"untitled"` rather than emitting `<id>-.md` (a
/// trailing-dash filename that's valid but ugly). The DB row keeps
/// the original title verbatim; only the on-disk filename is
/// sanitised.
///
/// Audit R1 CRITICAL (rust-expert) + MEDIUM-2 (code-reviewer):
/// previously only `create_artifact_with_projection` applied this
/// fallback. `sync_artifact_from_file`, `sync_body_from_file`,
/// `render_projection*`, `read_file_body_if_newer`,
/// `stamp_agent_identity`, and `remove_projection_at` reconstructed
/// paths with raw `slugify(title)` and would resolve to `<id>-.md`
/// after a non-ASCII-titled artifact had been created — producing
/// spurious `FileNotFound` errors on every subsequent sync. This
/// helper is the single source of truth for the slug-fallback
/// contract; every path-construction site MUST use it.
pub(crate) fn projection_slug(title: &str) -> String {
    let s = slugify(title);
    if s.is_empty() {
        "untitled".to_string()
    } else {
        s
    }
}

/// Frontmatter keys that `render_markdown_with_tags` writes from LanceDB
/// record data. Any key **not** in this set is preserved from the file on
/// disk across renders — this is how agent-owned fields such as
/// `last_modified_by` / `last_modified_at` (PRD-057 FR-009) and future
/// `domain` / `affected_files` survive re-renders triggered by unrelated
/// tool calls (e.g. `forgeplan_link`).
const KNOWN_FM_KEYS: &[&str] = &[
    "id",
    "title",
    "kind",
    "status",
    "depth",
    "author",
    "parent_epic",
    "valid_until",
    "tags",
    "links",
];

/// Render an artifact record as a markdown file in the workspace.
/// Returns the path where the file was written.
///
/// **Files-first (RFC-004)**: If the file already exists and the user has edited
/// the body (body differs from what LanceDB has), the file body is preserved.
/// Only frontmatter (status, links, metadata) is updated from LanceDB.
/// This prevents data loss when a user edits a file then runs forgeplan link/update.
#[allow(clippy::too_many_arguments)]
pub async fn render_projection(
    workspace: &Path,
    id: &str,
    kind: &str,
    title: &str,
    status: &str,
    depth: &str,
    author: Option<&str>,
    parent_epic: Option<&str>,
    valid_until: Option<&str>,
    body: &str,
    links: &[(String, String)],
) -> anyhow::Result<PathBuf> {
    render_projection_inner(
        workspace,
        id,
        kind,
        title,
        status,
        depth,
        author,
        parent_epic,
        valid_until,
        body,
        links,
        false,
    )
    .await
}

/// Render a full ArtifactRecord (includes tags) to its markdown file.
/// Used by mutations like `tag` / `untag` that need to persist tags to
/// frontmatter so they survive a reindex (ADR-003 files-first).
pub async fn render_projection_record(
    workspace: &Path,
    record: &crate::db::store::ArtifactRecord,
    links: &[(String, String)],
) -> anyhow::Result<PathBuf> {
    let artifact_kind = record
        .kind
        .parse::<ArtifactKind>()
        .unwrap_or(ArtifactKind::Note);
    let dir = workspace.join(artifact_kind.dir_name());
    tokio::fs::create_dir_all(&dir).await?;

    let slug = projection_slug(&record.title);
    let filename = format!("{}-{}.md", record.id, slug);
    let filepath = dir.join(&filename);

    // Files-first: preserve existing body + agent-owned fm keys (PRD-057 FR-009).
    let (effective_body, preserved_fm) = if filepath.exists() {
        match tokio::fs::read_to_string(&filepath).await {
            Ok(file_content) => match frontmatter::parse_frontmatter(&file_content) {
                Ok((fm, file_body)) => {
                    let preserved = filter_preserved(&fm);
                    if !file_body.trim().is_empty() {
                        (file_body.to_string(), preserved)
                    } else {
                        (record.body.clone(), preserved)
                    }
                }
                Err(_) => (record.body.clone(), None),
            },
            Err(_) => (record.body.clone(), None),
        }
    } else {
        (record.body.clone(), None)
    };

    let content = render_markdown_with_extras(
        &record.id,
        &record.kind,
        &record.title,
        &record.status,
        &record.depth,
        record.author.as_deref(),
        record.parent_epic.as_deref(),
        record.valid_until.as_deref(),
        &effective_body,
        links,
        &record.tags,
        preserved_fm.as_ref(),
    )?;
    // PRD-073 audit C1 (security): atomic tempfile+rename so a kill -9 between
    // truncate and write cannot zero-out the markdown projection. The previous
    // `tokio::fs::write` was non-atomic on POSIX (open+truncate+write).
    atomic_markdown_write(&filepath, content.as_bytes()).await?;
    Ok(filepath)
}

/// Like render_projection, but `force_body = true` uses the passed body
/// instead of reading from file. Used by `update --body` to ensure the
/// CLI-provided body takes precedence over the file on disk.
#[allow(clippy::too_many_arguments)]
pub async fn render_projection_with_body(
    workspace: &Path,
    id: &str,
    kind: &str,
    title: &str,
    status: &str,
    depth: &str,
    author: Option<&str>,
    parent_epic: Option<&str>,
    valid_until: Option<&str>,
    body: &str,
    links: &[(String, String)],
) -> anyhow::Result<PathBuf> {
    render_projection_inner(
        workspace,
        id,
        kind,
        title,
        status,
        depth,
        author,
        parent_epic,
        valid_until,
        body,
        links,
        true,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn render_projection_inner(
    workspace: &Path,
    id: &str,
    kind: &str,
    title: &str,
    status: &str,
    depth: &str,
    author: Option<&str>,
    parent_epic: Option<&str>,
    valid_until: Option<&str>,
    body: &str,
    links: &[(String, String)],
    force_body: bool,
) -> anyhow::Result<PathBuf> {
    let artifact_kind = kind.parse::<ArtifactKind>().unwrap_or(ArtifactKind::Note);
    let dir = workspace.join(artifact_kind.dir_name());
    tokio::fs::create_dir_all(&dir).await?;

    let slug = projection_slug(title);
    let filename = format!("{}-{}.md", id, slug);
    let filepath = dir.join(&filename);

    // Files-first (RFC-004): preserve file body if file exists and has content.
    // Only frontmatter is updated from LanceDB. Body comes from the file on disk.
    // Exception: force_body=true (from `update --body`) uses the passed body.
    //
    // PRD-057 FR-009: also preserve unknown frontmatter keys (anything not in
    // KNOWN_FM_KEYS) so agent-owned fields such as `last_modified_by` /
    // `last_modified_at` survive re-renders.
    let (effective_body, preserved_fm) = if force_body {
        (body.to_string(), read_preserved_fm(&filepath).await)
    } else if filepath.exists() {
        match tokio::fs::read_to_string(&filepath).await {
            Ok(file_content) => match frontmatter::parse_frontmatter(&file_content) {
                Ok((fm, file_body)) => {
                    let preserved = filter_preserved(&fm);
                    if !file_body.trim().is_empty() {
                        (file_body.to_string(), preserved)
                    } else {
                        (body.to_string(), preserved)
                    }
                }
                Err(_) => (body.to_string(), None),
            },
            Err(_) => (body.to_string(), None),
        }
    } else {
        (body.to_string(), None)
    };

    let content = render_markdown_with_extras(
        id,
        kind,
        title,
        status,
        depth,
        author,
        parent_epic,
        valid_until,
        &effective_body,
        links,
        &[],
        preserved_fm.as_ref(),
    )?;
    // PRD-073 audit C1: atomic write protects against kill-mid-truncate window.
    atomic_markdown_write(&filepath, content.as_bytes()).await?;

    Ok(filepath)
}

/// Read the file at `path`, parse its frontmatter, and return the subset of
/// keys that aren't regenerated from LanceDB (see `KNOWN_FM_KEYS`).
/// Returns `None` if the file is missing or malformed — callers should treat
/// that as "nothing to preserve".
async fn read_preserved_fm(path: &Path) -> Option<Frontmatter> {
    if !path.exists() {
        return None;
    }
    let content = tokio::fs::read_to_string(path).await.ok()?;
    let (fm, _body) = frontmatter::parse_frontmatter(&content).ok()?;
    filter_preserved(&fm)
}

/// Retain only keys outside `KNOWN_FM_KEYS`. Returns `None` when nothing is
/// left so call sites can cheaply check `Option` instead of empty-map sentinels.
fn filter_preserved(fm: &Frontmatter) -> Option<Frontmatter> {
    let mut out = BTreeMap::new();
    for (k, v) in fm {
        if !KNOWN_FM_KEYS.contains(&k.as_str()) {
            out.insert(k.clone(), v.clone());
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Sync file → store BEFORE mutation. Preserves any user edits to the
/// markdown body that haven't been picked up yet.
///
/// Convenience wrapper around `sync_file_to_store` that handles the
/// `get_record + Option` boilerplate in the common case where caller
/// only has the artifact ID. PROB-048 / ADR-003 enforcement helper —
/// use as the first step of every mutating handler in `commands/` and
/// `server.rs`.
///
/// No-op if the artifact does not exist (e.g., create flow).
pub async fn sync_before_mutation(
    workspace: &Path,
    store: &crate::db::store::LanceStore,
    id: &str,
) -> anyhow::Result<()> {
    if let Some(record) = store.get_record(id).await? {
        sync_file_to_store(store, workspace, &record).await?;
    }
    Ok(())
}

/// Render store → file AFTER mutation. Reads the current artifact state
/// from LanceDB and writes the markdown projection.
///
/// Convenience wrapper around `render_projection_record` that fetches
/// the record + relations. PROB-048 / ADR-003 enforcement helper — use
/// as the last step of every mutating handler.
///
/// No-op if the artifact does not exist (e.g., delete flow already
/// removed both file and record).
pub async fn render_after_mutation(
    workspace: &Path,
    store: &crate::db::store::LanceStore,
    id: &str,
) -> anyhow::Result<()> {
    if let Some(record) = store.get_record(id).await? {
        let links = store.get_relations(id).await.unwrap_or_default();
        render_projection_record(workspace, &record, &links).await?;
    }
    Ok(())
}

/// Sync file body to LanceDB store if file was edited by user.
/// Call this before render_projection to ensure LanceDB has the latest body.
/// Returns true if sync happened (file was newer).
pub async fn sync_file_to_store(
    store: &crate::db::store::LanceStore,
    workspace: &Path,
    record: &crate::db::store::ArtifactRecord,
) -> anyhow::Result<bool> {
    if let Some(file_body) = read_file_body_if_newer(
        workspace,
        &record.id,
        &record.kind,
        &record.title,
        &record.body,
    )
    .await
    {
        store.update_body(&record.id, &file_body).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Check if a file's body differs from the LanceDB body.
/// Returns Some(file_body) if the file was edited by the user, None otherwise.
pub async fn read_file_body_if_newer(
    workspace: &Path,
    id: &str,
    kind: &str,
    title: &str,
    db_body: &str,
) -> Option<String> {
    let artifact_kind = kind.parse::<ArtifactKind>().ok()?;
    let dir = workspace.join(artifact_kind.dir_name());
    let slug = projection_slug(title);
    let filename = format!("{}-{}.md", id, slug);
    let filepath = dir.join(&filename);

    let file_content = tokio::fs::read_to_string(&filepath).await.ok()?;
    let (_fm, file_body) = frontmatter::parse_frontmatter(&file_content).ok()?;

    let file_trimmed = file_body.trim();
    let db_trimmed = db_body.trim();

    if !file_trimmed.is_empty() && file_trimmed != db_trimmed {
        Some(file_body.to_string())
    } else {
        None
    }
}

/// Render markdown content with YAML frontmatter + body.
///
/// Regenerates the known frontmatter keys from args and merges
/// `preserved_fm` entries on top. `preserved_fm` MUST contain only keys
/// outside `KNOWN_FM_KEYS` (caller uses `filter_preserved`). This is how
/// `last_modified_by` / `last_modified_at` (PRD-057 FR-009) survive
/// re-renders triggered by unrelated tool calls.
#[allow(clippy::too_many_arguments)]
fn render_markdown_with_extras(
    id: &str,
    kind: &str,
    title: &str,
    status: &str,
    depth: &str,
    author: Option<&str>,
    parent_epic: Option<&str>,
    valid_until: Option<&str>,
    body: &str,
    links: &[(String, String)],
    tags: &[String],
    preserved_fm: Option<&Frontmatter>,
) -> anyhow::Result<String> {
    let mut fm = BTreeMap::new();
    fm.insert("id".to_string(), serde_yaml::Value::String(id.to_string()));
    fm.insert(
        "title".to_string(),
        serde_yaml::Value::String(title.to_string()),
    );
    fm.insert(
        "kind".to_string(),
        serde_yaml::Value::String(kind.to_string()),
    );
    fm.insert(
        "status".to_string(),
        serde_yaml::Value::String(status.to_string()),
    );
    fm.insert(
        "depth".to_string(),
        serde_yaml::Value::String(depth.to_string()),
    );

    if let Some(a) = author {
        fm.insert(
            "author".to_string(),
            serde_yaml::Value::String(a.to_string()),
        );
    }
    if let Some(pe) = parent_epic {
        fm.insert(
            "parent_epic".to_string(),
            serde_yaml::Value::String(pe.to_string()),
        );
    }
    if let Some(vu) = valid_until {
        fm.insert(
            "valid_until".to_string(),
            serde_yaml::Value::String(vu.to_string()),
        );
    }

    if !tags.is_empty() {
        let tag_seq: Vec<serde_yaml::Value> = tags
            .iter()
            .map(|t| serde_yaml::Value::String(t.clone()))
            .collect();
        fm.insert("tags".to_string(), serde_yaml::Value::Sequence(tag_seq));
    }

    if !links.is_empty() {
        let links_seq: Vec<serde_yaml::Value> = links
            .iter()
            .map(|(target, relation)| {
                let mut m = serde_yaml::Mapping::new();
                m.insert(
                    serde_yaml::Value::String("target".to_string()),
                    serde_yaml::Value::String(target.clone()),
                );
                m.insert(
                    serde_yaml::Value::String("relation".to_string()),
                    serde_yaml::Value::String(relation.clone()),
                );
                serde_yaml::Value::Mapping(m)
            })
            .collect();
        fm.insert("links".to_string(), serde_yaml::Value::Sequence(links_seq));
    }

    // Merge preserved (agent-owned / user-custom) fields last — guaranteed
    // to be outside KNOWN_FM_KEYS by construction (filter_preserved), so
    // they cannot override id/status/links/etc.
    if let Some(extras) = preserved_fm {
        for (k, v) in extras {
            // Double-guard: if caller violates the contract and passes a
            // known key, drop it rather than corrupt the regenerated field.
            if !KNOWN_FM_KEYS.contains(&k.as_str()) {
                fm.insert(k.clone(), v.clone());
            }
        }
    }

    let yaml = serde_yaml::to_string(&fm)?;
    Ok(format!("---\n{}---\n\n{}\n", yaml, body))
}

/// Stamp `last_modified_by` / `last_modified_at` onto an already-rendered
/// projection file (PRD-057 FR-009 + AC-5). Reads the markdown, merges the
/// two keys into the YAML frontmatter, and writes back.
///
/// Designed to run **after** `render_projection*` so the merge survives the
/// regenerate-from-LanceDB step. Since `KNOWN_FM_KEYS` excludes these keys,
/// subsequent renders preserve them via `filter_preserved`.
///
/// Failures are returned as errors — callers may choose to treat them as
/// best-effort (log + continue) so that identity tracking never blocks a
/// legitimate artifact write.
pub async fn stamp_agent_identity(
    workspace: &Path,
    id: &str,
    kind: &str,
    title: &str,
    identity: &crate::artifact::identity::AgentIdentity,
) -> anyhow::Result<()> {
    let artifact_kind = kind.parse::<ArtifactKind>().unwrap_or(ArtifactKind::Note);
    let dir = workspace.join(artifact_kind.dir_name());
    let slug = projection_slug(title);
    let filename = format!("{}-{}.md", id, slug);
    let filepath = dir.join(&filename);

    let content = tokio::fs::read_to_string(&filepath).await?;
    let (mut fm, body) = frontmatter::parse_frontmatter(&content)?;

    fm.insert(
        "last_modified_by".to_string(),
        serde_yaml::Value::String(identity.as_frontmatter_value()),
    );
    fm.insert(
        "last_modified_at".to_string(),
        serde_yaml::Value::String(crate::artifact::identity::now_rfc3339()),
    );

    let new_content = frontmatter::render_frontmatter(&fm, &body)?;
    // R2 audit MED (rust-pro + architect): use atomic tempfile+rename so
    // a kill -9 between truncate and write cannot corrupt the markdown.
    // `tokio::fs::write` was non-atomic on POSIX.
    atomic_markdown_write(&filepath, new_content.as_bytes()).await?;
    Ok(())
}

/// Atomic markdown write via tempfile + rename. Mirrors the pattern used
/// in `claim::atomic_write` but kept local to avoid cross-module coupling
/// until the shared `kv_yaml` abstraction (arch audit MED) is extracted.
async fn atomic_markdown_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "atomic_markdown_write: path has no parent",
        )
    })?;
    tokio::fs::create_dir_all(parent).await?;
    let tmp = parent.join(format!(
        ".{}.tmp.{}",
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "anon".to_string()),
        std::process::id(),
    ));
    tokio::fs::write(&tmp, bytes).await?;
    match tokio::fs::rename(&tmp, path).await {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp).await;
            Err(e)
        }
    }
}

/// Remove a projection file for a deleted artifact.
///
/// PRD-073 audit C2 (security): the previous implementation matched
/// `name.to_uppercase().starts_with(&id.to_uppercase())` and broke on
/// the first match found via `read_dir` ordering. That collided in two
/// ways: (a) memory IDs like `mem-auth` are a prefix of `mem-auth-system`,
/// so deleting the first could clobber the second; (b) numeric IDs once
/// they cross 999 (`PRD-100` is a prefix of `PRD-1000`).
///
/// The current implementation requires `<id>-` (trailing hyphen) as the
/// prefix, which fixes the numeric case, AND walks the entire directory
/// removing **every** matching file rather than just the first. The
/// trailing-hyphen requirement also rejects bare-`<id>.md` names that
/// don't follow the slug convention. For full correctness on the memory
/// case, callers that know the title can use `remove_projection_at` to
/// pin the exact filename.
pub async fn remove_projection(workspace: &Path, id: &str, kind: &str) -> anyhow::Result<()> {
    crate::db::store::validate_artifact_id(id)?;
    let artifact_kind: ArtifactKind = kind
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid kind '{kind}' for {id}: {e}"))?;
    let dir = workspace.join(artifact_kind.dir_name());
    if !dir.exists() {
        return Ok(());
    }
    let needle = format!("{}-", id.to_uppercase());
    let mut read_dir = tokio::fs::read_dir(&dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".md") && name.to_uppercase().starts_with(&needle) {
            // Also reject candidates whose ID prefix is a strict-prefix of
            // another ID (e.g. PRD-100 → "PRD-100-"; PRD-1000 → "PRD-1000-"
            // — both start with "PRD-100-"? No: PRD-1000- contains digit '0'
            // after the prefix-and-hyphen, so the trailing-hyphen check
            // separates `prd-100-foo.md` from `prd-1000-foo.md`).
            tokio::fs::remove_file(entry.path()).await?;
        }
    }
    Ok(())
}

/// Remove a projection file at a specific path computed from `(id, kind, title)`.
/// Use when you know the exact title (typically `original.title` from a
/// fetched record) and want to avoid prefix collisions entirely.
///
/// Returns Ok(true) if the file existed and was removed, Ok(false) if
/// nothing was there.
pub async fn remove_projection_at(
    workspace: &Path,
    id: &str,
    kind: &str,
    title: &str,
) -> anyhow::Result<bool> {
    crate::db::store::validate_artifact_id(id)?;
    let artifact_kind: ArtifactKind = kind
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid kind '{kind}' for {id}: {e}"))?;
    let dir = workspace.join(artifact_kind.dir_name());
    let slug = projection_slug(title);
    let filepath = dir.join(format!("{}-{}.md", id, slug));
    if filepath.exists() {
        tokio::fs::remove_file(&filepath).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

// =============================================================================
// PRD-073 file-first mutation helpers
// -----------------------------------------------------------------------------
// ADR-003 says: markdown is the source of truth, LanceDB is a derived index.
// These helpers wrap the {sync_before, store mutation, render_after} triplet
// so command handlers and MCP handlers don't have to repeat the pattern (and
// don't get a chance to forget a step). Each commits to a defined ordering
// chosen so a process kill between steps leaves the workspace recoverable.
// =============================================================================

/// Create a new artifact with file-first ordering: write the markdown
/// projection first, then sync to LanceDB. If the LanceDB insert fails,
/// the orphan file is reconciled by the next `forgeplan reindex`.
///
/// PRD-073 audit C3 fix: uses `render_projection_with_body` (force_body=true)
/// so a stale file already at the slug path (e.g. left over from a previous
/// failed create, or an unrelated `git checkout` artefact) is overwritten
/// by the caller-supplied body. The previous force_body=false path would
/// silently keep the on-disk body and stamp it into the new LanceDB row,
/// guaranteeing file/DB divergence on creation.
///
/// PRD-073 FR-001 helper. Used by `capture` / `remember` / `promote` /
/// `reason` (note-creating commands).
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `artifact.id` fails
///   `validate_artifact_id` (path-traversal payload, empty, etc.).
/// - [`MutationError::InvalidKind`] if `artifact.kind` does not parse as
///   an `ArtifactKind`.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   the underlying `LanceStore::create_artifact` call or the
///   markdown-projection write fails. Categorisation via
///   [`MutationError::from_store_err`] — fatal for schema corruption /
///   ENOENT, transient for EACCES / lock contention.
pub async fn create_artifact_with_projection(
    ctx: &MutationContext<'_>,
    artifact: &crate::db::store::NewArtifact,
) -> MutationResult<PathBuf> {
    let MutationContext { workspace, store } = *ctx;
    // Audit 2026-05-01 #1 (security CRITICAL): validate id BEFORE composing
    // it into a filesystem path. Without this, a JSON import with
    // `"id": "../../etc/evil"` would write outside the workspace via
    // `format!("{id}-{slug}.md")` resolved against `workspace/<kind>/`.
    crate::db::store::validate_artifact_id(&artifact.id)
        .map_err(|_| MutationError::InvalidId(artifact.id.clone()))?;
    let _: ArtifactKind = artifact
        .kind
        .parse()
        .map_err(
            |e: crate::error::ForgeplanError| MutationError::InvalidKind {
                id: artifact.id.clone(),
                kind: artifact.kind.clone(),
                reason: e.to_string(),
            },
        )?;

    // PRD-073 Phase 3c CRITICAL fix (R1 audit): the slug-fallback rule for
    // non-ASCII titles (Cyrillic, CJK, emoji) lives entirely inside
    // `projection_slug` now, and `render_projection_with_body` calls it via
    // `render_projection_inner`. So we pass the **real** title — the file
    // frontmatter keeps the user's title verbatim, only the on-disk filename
    // is sanitised. Previously this helper substituted "untitled" for the
    // entire title, which lost the original from the file's frontmatter
    // (the DB row was fine, but a `cat <id>-untitled.md` showed `title:
    // untitled` instead of the user's Cyrillic).
    //
    // 1. File first — projection is the source of truth, with caller body
    //    forced over any pre-existing file at the same slug path.
    let path = render_projection_with_body(
        workspace,
        &artifact.id,
        &artifact.kind,
        &artifact.title,
        &artifact.status,
        &artifact.depth,
        artifact.author.as_deref(),
        artifact.parent_epic.as_deref(),
        artifact.valid_until.as_deref(),
        &artifact.body,
        &[],
    )
    .await?;

    // 2. Then derived index. If this fails, file remains and reindex recovers.
    store.create_artifact(artifact).await?;
    Ok(path)
}

/// Delete an artifact, its relations, and its markdown file. File is removed
/// FIRST (so any failure on the DB side doesn't leave a phantom file behind);
/// then relations are cascaded; then the row itself.
///
/// No-op if the artifact does not exist in LanceDB.
///
/// PRD-073 FR-001 helper. Used by `delete` / `promote` (delete-after-move) /
/// `remember` (forget).
///
/// Audit C2 fix: removes the projection file at the EXACT path computed from
/// the record's `(id, kind, title)` rather than via prefix scan, so deleting
/// `mem-auth` cannot accidentally remove `mem-auth-system-...md`. If the
/// on-disk filename doesn't match the DB title (e.g. user renamed a file
/// without `forgeplan reindex`), the orphan is intentionally left behind to
/// be surfaced by `forgeplan health` rather than guessed at.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `id` fails `validate_artifact_id`.
/// - [`MutationError::InvalidKind`] if the existing record's `kind` field
///   is corrupt (audit H1: bail rather than silently fall back to
///   `Note` and remove a wrong-directory file).
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   any of `get_record` / `delete_relations_for_artifact` /
///   `delete_artifact` / projection-file removal fails. **Note**:
///   missing row is *not* an error here — delete is idempotent (asymmetric
///   with `update_body_with_projection` which returns `RowNotFound`).
pub async fn delete_artifact_with_projection(
    ctx: &MutationContext<'_>,
    id: &str,
) -> MutationResult<()> {
    let MutationContext { workspace, store } = *ctx;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    // R1 audit H-2 (rust+architect+security): missing-row is *idempotent
    // success* for delete (callers re-run delete during cleanup loops and
    // expect this to be a no-op), unlike `update_body_with_projection` which
    // returns `RowNotFound`. The asymmetry is intentional — documented here
    // and in the variant taxonomy. TODO(PROB-049): if a unified
    // contract emerges, revisit; current state is two helpers, two policies,
    // both correct for their semantic class (idempotent destructor vs
    // input-validating mutator).
    let record = match store.get_record(id).await? {
        Some(r) => r,
        None => return Ok(()),
    };
    // Audit 2026-05-01 H1 (typescript-type-auditor + rust-pro): bail on
    // unknown kind rather than silently falling back to ArtifactKind::Note,
    // which would let `delete` remove a wrong-directory file.
    let _: ArtifactKind = record
        .kind
        .parse()
        .map_err(
            |e: crate::error::ForgeplanError| MutationError::InvalidKind {
                id: id.to_string(),
                kind: record.kind.clone(),
                reason: e.to_string(),
            },
        )?;

    // 1. Remove file first (source of truth gone → workspace state unambiguous).
    //    Exact-path removal — no prefix collision with sibling IDs.
    let _ = remove_projection_at(workspace, id, &record.kind, &record.title).await?;

    // 2. Cascade relations + record from the derived index.
    store.delete_relations_for_artifact(id).await?;
    store.delete_artifact(id).await?;
    Ok(())
}

/// Update artifact metadata (status and/or title) with file-first guarantees:
/// sync any user edits to the markdown body into LanceDB, mutate, then
/// re-render the projection so frontmatter reflects the new state.
///
/// PRD-073 audit fix: short-circuits on `(None, None)` instead of paying
/// the sync round-trip to bump `updated_at` for nothing. Debug-asserts so
/// future callers see the contract violation in test builds.
///
/// PRD-073 FR-001 helper. Used by `update` (status/title path).
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `id` fails the inline validator
///   (alphanumeric / `-` / `_`, must start with a letter, non-empty).
/// - [`MutationError::EmptyField`] (`field: "status"` or `"title"`) if
///   either argument is `Some` of a blank / whitespace string.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   the sync-before / `update_artifact` / render-after triplet fails.
///
/// `(None, None)` short-circuits with `Ok(())` — no DB round-trip.
pub async fn update_metadata_with_projection(
    ctx: &MutationContext<'_>,
    id: &str,
    status: Option<&str>,
    title: Option<&str>,
) -> MutationResult<()> {
    let MutationContext { workspace, store } = *ctx;
    // Audit follow-up: this helper is the canary for the `MutationError`
    // enum migration. Other helpers will follow in PRD-073 Phase 3c when
    // the typed-error contract is stable.
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        || id.is_empty()
        || !id.chars().next().unwrap_or(' ').is_ascii_alphabetic()
    {
        return Err(MutationError::InvalidId(id.to_string()));
    }
    // Short-circuit on `(None, None)` instead of paying the sync round-trip
    // and bumping `updated_at` for nothing. Callers should gate upstream
    // (e.g. `update.rs` does `if status.is_some() || title.is_some()`); the
    // helper-level guard is defensive.
    if status.is_none() && title.is_none() {
        return Ok(());
    }
    // Audit 2026-05-01 H2: reject empty-string status/title at the helper
    // boundary instead of letting them write `status: ` (yaml null) into
    // the DB row that fails every subsequent parse.
    if let Some(s) = status
        && s.trim().is_empty()
    {
        return Err(MutationError::EmptyField { field: "status" });
    }
    if let Some(t) = title
        && t.trim().is_empty()
    {
        return Err(MutationError::EmptyField { field: "title" });
    }
    sync_before_mutation(workspace, store, id).await?;
    store.update_artifact(id, status, title).await?;
    render_after_mutation(workspace, store, id).await?;
    Ok(())
}

/// Replace artifact body with the supplied content. Unlike other mutation
/// helpers, this does NOT call `sync_before_mutation` — the caller is
/// explicitly overwriting whatever is on disk with a CLI/MCP-supplied body,
/// so reading the file first would be a no-op at best and a race at worst.
///
/// PRD-073 audit H2 (security): file is written FIRST, then LanceDB is
/// updated. If the process is killed between the two writes, the file holds
/// the user's intended body and the next `forgeplan reindex` propagates it
/// to LanceDB. The previous order (DB-first, then file) would let a
/// kill-mid-flow strand the user's edit only in LanceDB, where the next
/// reindex would silently overwrite it with the stale on-disk body.
///
/// PRD-073 FR-001 helper. Used by `update --body` and MCP body-update paths.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `id` fails `validate_artifact_id`.
/// - [`MutationError::RowNotFound`] if the artifact is not in the store
///   (unlike `delete_artifact_with_projection`'s idempotent missing-row
///   policy — `update_body` is an input-validating mutator).
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   the projection write or `update_body` call fails.
pub async fn update_body_with_projection(
    ctx: &MutationContext<'_>,
    id: &str,
    body: &str,
) -> MutationResult<()> {
    let MutationContext { workspace, store } = *ctx;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    let record = match store.get_record(id).await? {
        Some(r) => r,
        None => {
            // Wave 1A audit follow-up: typed `RowNotFound` distinguishes the
            // input-side missing-row case from transient `StoreError` so MCP
            // strict mode can react without string-matching the message.
            return Err(MutationError::RowNotFound { id: id.to_string() });
        }
    };
    let links = store.get_relations(id).await.unwrap_or_default();

    // 1. File first — write the user's body to the markdown projection. This
    //    establishes the source of truth before the derived index is touched.
    render_projection_with_body(
        workspace,
        &record.id,
        &record.kind,
        &record.title,
        &record.status,
        &record.depth,
        record.author.as_deref(),
        record.parent_epic.as_deref(),
        record.valid_until.as_deref(),
        body,
        &links,
    )
    .await?;

    // 2. DB second — sync the derived index. If this fails, reindex recovers.
    store.update_body(id, body).await?;
    Ok(())
}

/// Update artifact depth (Tactical / Standard / Deep / Critical) with
/// file-first guarantees.
///
/// PRD-073 FR-001 helper. Used by `update --depth`.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `id` fails `validate_artifact_id`.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   the sync-before / `update_depth` / render-after triplet fails.
pub async fn update_depth_with_projection(
    ctx: &MutationContext<'_>,
    id: &str,
    depth: &str,
) -> MutationResult<()> {
    let MutationContext { workspace, store } = *ctx;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    sync_before_mutation(workspace, store, id).await?;
    store.update_depth(id, depth).await?;
    render_after_mutation(workspace, store, id).await?;
    Ok(())
}

/// Add tags to an artifact with file-first guarantees: sync any user file
/// edits, mutate, render the projection so frontmatter `tags:` reflects the
/// new state.
///
/// PRD-073 FR-001 helper. Used by `forgeplan tag`.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `id` fails `validate_artifact_id`.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   the sync-before / `add_tags` / render-after triplet fails.
pub async fn add_tags_with_projection(
    ctx: &MutationContext<'_>,
    id: &str,
    tags: &[String],
) -> MutationResult<()> {
    let MutationContext { workspace, store } = *ctx;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    sync_before_mutation(workspace, store, id).await?;
    store.add_tags(id, tags).await?;
    render_after_mutation(workspace, store, id).await?;
    Ok(())
}

/// Remove tags from an artifact with file-first guarantees.
///
/// PRD-073 FR-001 helper. Used by `forgeplan untag`.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `id` fails `validate_artifact_id`.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   the sync-before / `remove_tags` / render-after triplet fails.
pub async fn remove_tags_with_projection(
    ctx: &MutationContext<'_>,
    id: &str,
    tags: &[String],
) -> MutationResult<()> {
    let MutationContext { workspace, store } = *ctx;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    sync_before_mutation(workspace, store, id).await?;
    store.remove_tags(id, tags).await?;
    render_after_mutation(workspace, store, id).await?;
    Ok(())
}

/// Add a typed relation between two artifacts, then re-render the projection
/// for **both** source and target. Without re-rendering both sides, the
/// target file's frontmatter never picks up any drift correction it needed
/// (see PROB-048 observed-symptom #2: phantom orphan health signal because
/// only the source side was rendered). This is FR-005 in PRD-073.
///
/// Note: outgoing-only frontmatter means the new edge appears in the source
/// file's `links:` block; the target file gets re-rendered as a side effect
/// so its own outgoing links stay synchronized with LanceDB.
///
/// PRD-073 FR-001 / FR-005 helper. Used by `link` / `reason` (auto-linking).
///
/// Audit fix: only the source-side pre-sync and the relation write itself
/// are fatal. Target-side pre-sync and BOTH post-render calls are
/// best-effort with `tracing::warn!` because the target may legitimately
/// be in a state that doesn't have a local file (cross-workspace
/// reference) and a transient FS error on rendering should not strand
/// the relation that already landed in LanceDB.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `source` or `target` fails
///   `validate_artifact_id`.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   source-side pre-sync or `add_relation` fails. Target-side pre-sync
///   and post-render failures are logged via `tracing::warn!` and
///   swallowed (best-effort) so a missing-target file does not strand the
///   relation that already landed in LanceDB.
pub async fn add_link_with_projection(
    ctx: &MutationContext<'_>,
    source: &str,
    target: &str,
    relation: &str,
) -> MutationResult<()> {
    let MutationContext { workspace, store } = *ctx;
    crate::db::store::validate_artifact_id(source)
        .map_err(|_| MutationError::InvalidId(source.to_string()))?;
    crate::db::store::validate_artifact_id(target)
        .map_err(|_| MutationError::InvalidId(target.to_string()))?;
    sync_before_mutation(workspace, store, source).await?;
    if let Err(e) = sync_before_mutation(workspace, store, target).await {
        tracing::warn!("add_link: pre-sync target {target} failed (continuing): {e}");
    }

    store.add_relation(source, target, relation).await?;

    if let Err(e) = render_after_mutation(workspace, store, source).await {
        tracing::warn!("add_link: post-render source {source} failed (continuing): {e}");
    }
    if let Err(e) = render_after_mutation(workspace, store, target).await {
        tracing::warn!("add_link: post-render target {target} failed (continuing): {e}");
    }
    Ok(())
}

/// Remove a typed relation between two artifacts and re-render projections
/// for both source and target so neither file's frontmatter retains a stale
/// edge.
///
/// Audit fix: same warn-and-continue policy as `add_link_with_projection`.
/// Source pre-sync and the relation delete are fatal; target pre-sync and
/// both renders are best-effort.
///
/// PRD-073 FR-005 helper. Used by `unlink`.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `source` or `target` fails
///   `validate_artifact_id`.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   source-side pre-sync or `delete_relation` fails. Target-side
///   pre-sync and post-render failures are best-effort (warn-and-continue),
///   matching `add_link_with_projection`.
pub async fn delete_link_with_projection(
    ctx: &MutationContext<'_>,
    source: &str,
    target: &str,
    relation: &str,
) -> MutationResult<()> {
    let MutationContext { workspace, store } = *ctx;
    crate::db::store::validate_artifact_id(source)
        .map_err(|_| MutationError::InvalidId(source.to_string()))?;
    crate::db::store::validate_artifact_id(target)
        .map_err(|_| MutationError::InvalidId(target.to_string()))?;
    sync_before_mutation(workspace, store, source).await?;
    if let Err(e) = sync_before_mutation(workspace, store, target).await {
        tracing::warn!("delete_link: pre-sync target {target} failed (continuing): {e}");
    }

    store.delete_relation(source, target, relation).await?;

    if let Err(e) = render_after_mutation(workspace, store, source).await {
        tracing::warn!("delete_link: post-render source {source} failed (continuing): {e}");
    }
    if let Err(e) = render_after_mutation(workspace, store, target).await {
        tracing::warn!("delete_link: post-render target {target} failed (continuing): {e}");
    }
    Ok(())
}

// =============================================================================
// PRD-073 Phase 3b — sync-mechanism helpers (file→DB direction)
// -----------------------------------------------------------------------------
// `reindex`, `git_sync`, `watch` read FROM the markdown file (which is already
// authoritative per ADR-003) and write INTO LanceDB. The `_with_projection`
// helpers above are the wrong shape here — they would re-render the file
// from DB after syncing, producing a no-op write that perturbs mtime and
// could trigger watcher loops.
//
// These thin wrappers exist so that:
// (a) command handlers in `forgeplan-cli` keep going through the
//     `forgeplan_core::projection::*` namespace (visible to the regression
//     guard test as approved channels)
// (b) Phase 4 visibility lockdown can demote `LanceStore::*` mutating
//     methods to `pub(crate)` while these wrappers stay `pub`
// (c) intent is documented at the call site: "sync from file" vs "mutate
//     and re-render" — future maintainers can't confuse the two.
//
// The wrappers are intentionally minimal — they validate the artifact ID
// against path-traversal (audit CRITICAL #1) and forward to the underlying
// store. Callers are responsible for ensuring the file is authoritative.
// =============================================================================

/// Sync a fully-formed `NewArtifact` into LanceDB. Caller has already
/// read+parsed the markdown file (which is the source of truth) and wants
/// the DB row to mirror it. Used by `reindex` / `git_sync` / `watch`.
///
/// PRD-073 Phase 3c: takes `workspace` so the helper can enforce the
/// file-first invariant — if the markdown file is gone between the caller
/// reading it and the helper running (TOCTOU), refuse to write a DB row
/// that has no on-disk source. `MutationError::FileNotFound` is fatal.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `artifact.id` fails
///   `validate_artifact_id`.
/// - [`MutationError::InvalidKind`] if `artifact.kind` does not parse as
///   an `ArtifactKind`.
/// - [`MutationError::FileNotFound`] if the markdown projection at the
///   resolved path is missing on disk (TOCTOU between caller's read and
///   this call). The path is workspace-relative for log safety.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   `metadata` returns a non-ENOENT I/O error or `create_artifact`
///   fails. EACCES on the parent directory routes to `StoreTransient`.
pub async fn sync_artifact_from_file(
    ctx: &MutationContext<'_>,
    artifact: &crate::db::store::NewArtifact,
) -> MutationResult<()> {
    let MutationContext { workspace, store } = *ctx;
    crate::db::store::validate_artifact_id(&artifact.id)
        .map_err(|_| MutationError::InvalidId(artifact.id.clone()))?;
    let kind: ArtifactKind = artifact
        .kind
        .parse()
        .map_err(
            |e: crate::error::ForgeplanError| MutationError::InvalidKind {
                id: artifact.id.clone(),
                kind: artifact.kind.clone(),
                reason: e.to_string(),
            },
        )?;
    let path = workspace.join(kind.dir_name()).join(format!(
        "{}-{}.md",
        artifact.id,
        projection_slug(&artifact.title)
    ));
    // R1 audit M-1: distinguish ENOENT (real file-not-found) from EACCES /
    // other I/O errors. Treating "permission denied" as `FileNotFound`
    // would mislead an MCP client trying to remediate.
    match tokio::fs::metadata(&path).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // R1 audit H-8 + R2 audit M-R2-1: strip the workspace prefix so
            // the error Display does not leak the user's home directory /
            // project location. R2 hardening: if `strip_prefix` fails (which
            // can happen under symlink/canonicalization mismatch on macOS
            // `/Users/...` vs `/private/Users/...`), fall back to file_name
            // only — NEVER to the absolute path. Previous `unwrap_or(&path)`
            // silently reintroduced the leak in those edge cases.
            let rel_path = path
                .strip_prefix(workspace)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| path.file_name().map(PathBuf::from).unwrap_or_default());
            return Err(MutationError::FileNotFound {
                id: artifact.id.clone(),
                path: rel_path,
            });
        }
        Err(e) => {
            // PROB-049 H-1: route through `from_store_err` so EACCES /
            // EBUSY / EWOULDBLOCK stay transient (operator can fix and
            // retry) while `tokio::fs::metadata`'s rare ENOENT-adjacent
            // shapes promote to `StoreFatal`. Identical semantics to the
            // legacy `StoreError(e.into())` for the existing test
            // contract — `PermissionDenied` → recoverable=true.
            return Err(MutationError::from_store_err(e.into()));
        }
    }
    store.create_artifact(artifact).await?;
    Ok(())
}

/// Sync a freshly-read body from file into LanceDB. Caller is `reindex` /
/// `git_sync` / `watch` after detecting the file is newer than the DB row.
///
/// PRD-073 Phase 3c: takes `workspace`, `kind`, `title` so the helper can
/// resolve the projection path and refuse to write `body` if the file is
/// missing on disk (TOCTOU between caller's read and this call). The DB
/// row exists by precondition (`update_body` would otherwise fail), but
/// the on-disk file may have been deleted by a concurrent operation.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `id` fails `validate_artifact_id`.
/// - [`MutationError::InvalidKind`] if `kind` does not parse as an
///   `ArtifactKind`.
/// - [`MutationError::FileNotFound`] if the markdown projection is
///   missing on disk (workspace-relative path).
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   `metadata` fails with a non-ENOENT shape, or `update_body` fails.
pub async fn sync_body_from_file(
    ctx: &MutationContext<'_>,
    id: &str,
    kind: &str,
    title: &str,
    body: &str,
) -> MutationResult<()> {
    let MutationContext { workspace, store } = *ctx;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    let parsed_kind: ArtifactKind =
        kind.parse().map_err(
            |e: crate::error::ForgeplanError| MutationError::InvalidKind {
                id: id.to_string(),
                kind: kind.to_string(),
                reason: e.to_string(),
            },
        )?;
    let path = workspace.join(parsed_kind.dir_name()).join(format!(
        "{}-{}.md",
        id,
        projection_slug(title)
    ));
    // R1 audit M-1: ENOENT only — EACCES / other I/O fall through as StoreError.
    // R1 audit H-8 + R2 M-R2-1: strip workspace prefix; on strip failure, fall
    // back to file_name (never the absolute path) to avoid leaks under
    // symlink/canonicalization mismatch.
    match tokio::fs::metadata(&path).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let rel_path = path
                .strip_prefix(workspace)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| path.file_name().map(PathBuf::from).unwrap_or_default());
            return Err(MutationError::FileNotFound {
                id: id.to_string(),
                path: rel_path,
            });
        }
        Err(e) => {
            // PROB-049 H-1: see equivalent comment in
            // `create_artifact_with_projection` — non-ENOENT I/O errors
            // funnel through `from_store_err` for accurate transient/fatal
            // routing instead of being lumped into a single recoverable
            // bucket.
            return Err(MutationError::from_store_err(e.into()));
        }
    }
    store.update_body(id, body).await?;
    Ok(())
}

/// Sync metadata (status / title) from frontmatter into LanceDB.
/// Used by `git_sync` when the pulled file's frontmatter differs from DB.
///
/// Symmetric with `update_metadata_with_projection`:
/// - `(None, None)` short-circuits without bumping `updated_at`.
/// - empty-string status/title is rejected at the helper boundary
///   (audit MEDIUM-1, code-reviewer 2026-05-01) — a YAML frontmatter
///   `status: ""` parses as `Some("")` and would otherwise write a
///   yaml-null status into the DB row that fails every subsequent
///   `parse::<Status>()`. Same H2 silent-corruption class as the
///   sibling helper closes.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `id` fails `validate_artifact_id`.
/// - [`MutationError::EmptyField`] if either argument is `Some` of a
///   blank / whitespace string.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   the `update_artifact` call fails.
///
/// `(None, None)` short-circuits with `Ok(())` — no DB round-trip.
pub async fn sync_metadata_from_file(
    ctx: &MutationContext<'_>,
    id: &str,
    status: Option<&str>,
    title: Option<&str>,
) -> MutationResult<()> {
    // Path-blind helper: `ctx.workspace` is unused today but kept on the
    // signature so Phase 3d can wire projection-mismatch checks without
    // breaking call sites again.
    let store = ctx.store;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    if status.is_none() && title.is_none() {
        return Ok(());
    }
    if let Some(s) = status
        && s.trim().is_empty()
    {
        return Err(MutationError::EmptyField { field: "status" });
    }
    if let Some(t) = title
        && t.trim().is_empty()
    {
        return Err(MutationError::EmptyField { field: "title" });
    }
    store.update_artifact(id, status, title).await?;
    Ok(())
}

/// Sync a relation from a markdown `links:` block into LanceDB.
/// Used by `reindex` / `git_sync` to restore typed relations after
/// rebuilding the DB from files.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `source` or `target` fails
///   `validate_artifact_id`.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   `add_relation` fails.
pub async fn sync_relation_from_file(
    ctx: &MutationContext<'_>,
    source: &str,
    target: &str,
    relation: &str,
) -> MutationResult<()> {
    // Path-blind helper — see `sync_metadata_from_file` note.
    let store = ctx.store;
    crate::db::store::validate_artifact_id(source)
        .map_err(|_| MutationError::InvalidId(source.to_string()))?;
    crate::db::store::validate_artifact_id(target)
        .map_err(|_| MutationError::InvalidId(target.to_string()))?;
    store.add_relation(source, target, relation).await?;
    Ok(())
}

/// Delete an orphan artifact row whose markdown file is already gone or
/// whose `kind` field is corrupt. Used by `reindex` Phase 2 cleanup and
/// `git_sync` on `'D'` (file-deleted) entries. The file is NOT removed
/// because (a) reindex assumes file is already missing, (b) git_sync was
/// triggered BY the deletion. If you want to delete an artifact AND its
/// projection, use `delete_artifact_with_projection`.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `id` fails `validate_artifact_id`.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   `delete_artifact` fails.
pub async fn delete_orphan_artifact(ctx: &MutationContext<'_>, id: &str) -> MutationResult<()> {
    let store = ctx.store;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    store.delete_artifact(id).await?;
    Ok(())
}

/// Delete an orphan relation whose source or target artifact no longer
/// exists. Used by `reindex` Phase 3 cleanup.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `source` or `target` fails
///   `validate_artifact_id`.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   `delete_relation` fails.
pub async fn delete_orphan_relation(
    ctx: &MutationContext<'_>,
    source: &str,
    target: &str,
    relation: &str,
) -> MutationResult<()> {
    let store = ctx.store;
    crate::db::store::validate_artifact_id(source)
        .map_err(|_| MutationError::InvalidId(source.to_string()))?;
    crate::db::store::validate_artifact_id(target)
        .map_err(|_| MutationError::InvalidId(target.to_string()))?;
    store.delete_relation(source, target, relation).await?;
    Ok(())
}

/// Add multiple links in one batch with deduplicated rendering.
///
/// Naive `add_link_with_projection` in a loop costs `4 × N` `get_record`
/// calls + `2 × N` file reads + `2 × N` file writes. For a 100-link
/// import bundle that's 600 LanceDB calls + 400 file ops. This batch
/// helper:
///
/// 1. Pre-syncs each unique source/target ONCE before any add_relation
/// 2. Adds all relations
/// 3. Renders each affected file ONCE at the end
///
/// Result: `~2 × U` `get_record` calls + `~2 × U` file writes where U
/// is the count of unique source-or-target IDs (typically << N).
///
/// Per-link errors are collected and returned as a count; the function
/// itself returns `Ok` after attempting every link unless one of the
/// pre-sync calls fails (those are fatal because they snapshot user
/// edits). Audit H6 (code-analyzer 2026-05-01).
///
/// PRD-073 FR-001 / FR-005 helper. Used by `import_cmd` /
/// `forgeplan_import` / `ingest` (any caller that adds many relations
/// in one shot).
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if any source or target id in the
///   batch fails `validate_artifact_id`. Validation runs up front so a
///   bad id rejects the batch before any write lands.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   any pre-sync call fails (those are fatal because they snapshot
///   user edits — losing them would corrupt the workspace).
///
/// Per-link `add_relation` failures are counted and returned in the
/// `Ok(usize)` payload (number of relations actually applied) — they do
/// not abort the batch. See helper body for the full ordering contract.
pub async fn add_links_batch_with_projection(
    ctx: &MutationContext<'_>,
    links: &[(String, String, String)],
) -> MutationResult<usize> {
    let MutationContext { workspace, store } = *ctx;
    if links.is_empty() {
        return Ok(0);
    }

    // Validate all IDs up front so we don't half-apply the batch.
    // Audit LOW-4 (Vec::contains O(N²)) is intentionally NOT changed in this
    // PR — Phase 3c migrates the signature only; algorithmic dedup is
    // tracked for Phase 3d.
    for (source, target, _) in links {
        crate::db::store::validate_artifact_id(source)
            .map_err(|_| MutationError::InvalidId(source.clone()))?;
        crate::db::store::validate_artifact_id(target)
            .map_err(|_| MutationError::InvalidId(target.clone()))?;
    }

    // Collect unique participants for one pre-sync per file.
    let mut unique_ids: Vec<&str> = Vec::with_capacity(links.len() * 2);
    for (source, target, _) in links {
        if !unique_ids.contains(&source.as_str()) {
            unique_ids.push(source);
        }
        if !unique_ids.contains(&target.as_str()) {
            unique_ids.push(target);
        }
    }

    // Phase 1: pre-sync each unique participant ONCE (best-effort —
    // missing target is a legitimate cross-workspace ref, not an error).
    for id in &unique_ids {
        if let Err(e) = sync_before_mutation(workspace, store, id).await {
            tracing::warn!("add_links_batch: pre-sync {id} failed (continuing): {e}");
        }
    }

    // Phase 2: add all relations. Count failures rather than bail so
    // bulk imports surface "imported N of M" instead of stopping cold.
    let mut applied = 0usize;
    for (source, target, relation) in links {
        match store.add_relation(source, target, relation).await {
            Ok(_) => applied += 1,
            Err(e) => {
                tracing::warn!(
                    "add_links_batch: add_relation {source} --{relation}--> {target} failed: {e}",
                );
            }
        }
    }

    // Phase 3: render each affected file ONCE (warn-and-continue —
    // see `add_link_with_projection` for the same policy on single ops).
    for id in &unique_ids {
        if let Err(e) = render_after_mutation(workspace, store, id).await {
            tracing::warn!("add_links_batch: post-render {id} failed (continuing): {e}");
        }
    }

    Ok(applied)
}

/// Delete an artifact's LanceDB row AFTER its markdown projection has
/// already been moved to trash by `undo::soft_delete_capture`. The file
/// is intentionally NOT touched by this helper — `soft_delete_capture`
/// owns the file→trash move.
///
/// Used by MCP `forgeplan_delete` (PRD-055 soft-delete pattern). CLI
/// `forgeplan delete` since 2026-05-01 also goes through soft_delete +
/// this helper for parity (audit follow-up).
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `id` fails `validate_artifact_id`.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   `delete_artifact` fails.
pub async fn delete_artifact_after_soft_delete(
    ctx: &MutationContext<'_>,
    id: &str,
) -> MutationResult<()> {
    // Path-blind helper — see `sync_metadata_from_file` note. The file
    // has already been moved to trash by `soft_delete_capture` so this
    // helper deliberately does not touch `ctx.workspace`.
    let store = ctx.store;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    store.delete_artifact(id).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn render_markdown_basic() {
        let content = render_markdown_with_extras(
            "PRD-001",
            "prd",
            "Auth System",
            "draft",
            "standard",
            Some("alice"),
            None,
            None,
            "## Summary\n\nThis is the body.",
            &[],
            &[],
            None,
        )
        .unwrap();
        assert!(content.starts_with("---\n"));
        assert!(content.contains("id: PRD-001"));
        assert!(content.contains("title: Auth System"));
        assert!(content.contains("## Summary"));
    }

    #[test]
    fn render_markdown_with_links() {
        let links = vec![
            ("RFC-001".to_string(), "informs".to_string()),
            ("ADR-001".to_string(), "based_on".to_string()),
        ];
        let content = render_markdown_with_extras(
            "PRD-001",
            "prd",
            "Auth",
            "draft",
            "standard",
            None,
            None,
            None,
            "Body.",
            &links,
            &[],
            None,
        )
        .unwrap();
        assert!(content.contains("links:"));
        assert!(content.contains("target: RFC-001"));
        assert!(content.contains("relation: informs"));
    }

    #[test]
    fn render_markdown_optional_fields_omitted() {
        let content = render_markdown_with_extras(
            "NOTE-001",
            "note",
            "Quick Note",
            "draft",
            "tactical",
            None,
            None,
            None,
            "Content.",
            &[],
            &[],
            None,
        )
        .unwrap();
        assert!(!content.contains("author:"));
        assert!(!content.contains("parent_epic:"));
        assert!(!content.contains("valid_until:"));
        assert!(!content.contains("links:"));
    }

    #[tokio::test]
    async fn render_projection_creates_file() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();

        let path = render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Auth System",
            "draft",
            "standard",
            Some("alice"),
            None,
            None,
            "## Summary\n\nBody.",
            &[],
        )
        .await
        .unwrap();

        assert!(path.exists());
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(content.contains("id: PRD-001"));
        assert!(content.contains("## Summary"));
    }

    #[tokio::test]
    async fn remove_projection_deletes_file() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let prds = ws.join("prds");
        tokio::fs::create_dir_all(&prds).await.unwrap();
        tokio::fs::write(prds.join("PRD-001-auth.md"), "test")
            .await
            .unwrap();

        remove_projection(&ws, "PRD-001", "prd").await.unwrap();
        assert!(!prds.join("PRD-001-auth.md").exists());
    }

    #[tokio::test]
    async fn render_projection_preserves_user_edited_body() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        // Step 1: create projection with template body
        let template_body = "## Summary\n\n{Fill in summary here}";
        let path = render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Auth System",
            "draft",
            "standard",
            None,
            None,
            None,
            template_body,
            &[],
        )
        .await
        .unwrap();

        // Step 2: user edits the file with real content
        let user_content = "---\nid: PRD-001\ntitle: Auth System\nkind: prd\nstatus: draft\ndepth: standard\n---\n\n## Summary\n\nReal user content that should be preserved.\n\n## Goals\n\n- Support OAuth2\n";
        tokio::fs::write(&path, user_content).await.unwrap();

        // Step 3: forgeplan link triggers render_projection with LanceDB body (template)
        let path2 = render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Auth System",
            "draft",
            "standard",
            None,
            None,
            None,
            template_body,
            &[("RFC-001".to_string(), "based_on".to_string())],
        )
        .await
        .unwrap();

        // Verify: user body preserved, NOT overwritten with template
        let result = tokio::fs::read_to_string(&path2).await.unwrap();
        assert!(
            result.contains("Real user content"),
            "user body must be preserved"
        );
        assert!(
            result.contains("Support OAuth2"),
            "user goals must be preserved"
        );
        assert!(
            !result.contains("{Fill in summary"),
            "template must NOT overwrite user content"
        );
        // But frontmatter should have the new link
        assert!(
            result.contains("RFC-001"),
            "new link should be in frontmatter"
        );
    }

    #[tokio::test]
    async fn render_projection_updates_frontmatter_preserves_body() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        let body = "## Summary\n\nSame content.";

        // Create initial projection
        render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Test",
            "draft",
            "standard",
            None,
            None,
            None,
            body,
            &[],
        )
        .await
        .unwrap();

        // Re-render with same body but updated status — frontmatter updates, body preserved from file
        let path = render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Test",
            "active",
            "standard",
            None,
            None,
            None,
            body,
            &[],
        )
        .await
        .unwrap();

        let result = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(
            result.contains("status: active"),
            "status should be updated"
        );
        assert!(
            result.contains("Same content"),
            "body should be preserved from file"
        );
    }

    #[tokio::test]
    async fn render_projection_preserves_file_body_even_when_db_has_template() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        let template_body = "## Template\n\n{placeholder}";

        // Step 1: create projection with template
        let path = render_projection(
            &ws,
            "RFC-001",
            "rfc",
            "My RFC",
            "draft",
            "standard",
            None,
            None,
            None,
            template_body,
            &[],
        )
        .await
        .unwrap();

        // Step 2: user edits file with real content (270 lines)
        let user_content = format!(
            "---\nid: RFC-001\ntitle: My RFC\nkind: rfc\nstatus: draft\ndepth: standard\n---\n\n{}\n",
            "## Summary\n\nReal RFC content with 270 lines of architecture.\n\n## Motivation\n\nThis is important because...\n\n## Proposed Direction\n\nWe should do X."
        );
        tokio::fs::write(&path, user_content).await.unwrap();

        // Step 3: forgeplan link re-renders with TEMPLATE body from LanceDB (the bug scenario)
        let path2 = render_projection(
            &ws,
            "RFC-001",
            "rfc",
            "My RFC",
            "draft",
            "standard",
            None,
            None,
            None,
            template_body, // <-- LanceDB still has template!
            &[("PRD-011".to_string(), "based_on".to_string())],
        )
        .await
        .unwrap();

        // Verify: file body preserved, NOT reset to template
        let result = tokio::fs::read_to_string(&path2).await.unwrap();
        assert!(
            result.contains("Real RFC content"),
            "user body must be preserved after link"
        );
        assert!(
            result.contains("This is important"),
            "motivation must survive"
        );
        assert!(
            !result.contains("{placeholder}"),
            "template must NOT overwrite user content"
        );
        // Frontmatter updated with link
        assert!(result.contains("PRD-011"), "link should be in frontmatter");
    }

    #[tokio::test]
    async fn read_file_body_if_newer_detects_edit() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        let db_body = "## Template\n\n{placeholder}";
        render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Test",
            "draft",
            "standard",
            None,
            None,
            None,
            db_body,
            &[],
        )
        .await
        .unwrap();

        // Simulate user editing the file
        let prds = ws.join("prds");
        let filepath = prds.join("PRD-001-test.md");
        let edited = "---\nid: PRD-001\nkind: prd\nstatus: draft\ndepth: standard\ntitle: Test\n---\n\n## Real Content\n\nUser wrote this.\n";
        tokio::fs::write(&filepath, edited).await.unwrap();

        let result = read_file_body_if_newer(&ws, "PRD-001", "prd", "Test", db_body).await;
        assert!(result.is_some(), "should detect user edit");
        assert!(result.unwrap().contains("User wrote this"));
    }

    #[tokio::test]
    async fn read_file_body_if_newer_returns_none_when_same() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        let body = "## Summary\n\nSame content.";
        render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Test",
            "draft",
            "standard",
            None,
            None,
            None,
            body,
            &[],
        )
        .await
        .unwrap();

        let result = read_file_body_if_newer(&ws, "PRD-001", "prd", "Test", body).await;
        assert!(result.is_none(), "should return None when body matches");
    }

    #[tokio::test]
    async fn render_projection_with_body_overrides_file() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        // Step 1: create projection with template body
        let template_body = "## Template\n\n{placeholder}";
        render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Test",
            "draft",
            "standard",
            None,
            None,
            None,
            template_body,
            &[],
        )
        .await
        .unwrap();

        // Step 2: render_projection_with_body should override file content
        let new_body = "## Problem\n\nNew body from CLI update.";
        let path = render_projection_with_body(
            &ws,
            "PRD-001",
            "prd",
            "Test",
            "draft",
            "standard",
            None,
            None,
            None,
            new_body,
            &[],
        )
        .await
        .unwrap();

        let result = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(
            result.contains("New body from CLI update"),
            "force_body must override file"
        );
        assert!(
            !result.contains("{placeholder}"),
            "old file body must not survive"
        );
    }

    // ── PRD-057 Inc 2: agent identity + unknown-fm preservation ─────────

    #[test]
    fn filter_preserved_drops_known_keys() {
        let fm: Frontmatter = serde_yaml::from_str(
            "id: PRD-001\ntitle: Test\nkind: prd\nstatus: draft\ndepth: standard\nauthor: alice",
        )
        .unwrap();
        assert!(filter_preserved(&fm).is_none(), "all keys are known");
    }

    #[test]
    fn filter_preserved_keeps_unknown_keys() {
        let fm: Frontmatter = serde_yaml::from_str(
            "id: PRD-001\ntitle: Test\nkind: prd\nstatus: draft\ndepth: standard\nlast_modified_by: orchestrator/1.0\ndomain: backend",
        )
        .unwrap();
        let preserved = filter_preserved(&fm).expect("should preserve custom keys");
        assert_eq!(preserved.len(), 2);
        assert!(preserved.contains_key("last_modified_by"));
        assert!(preserved.contains_key("domain"));
        assert!(!preserved.contains_key("id"));
    }

    #[test]
    fn render_extras_merges_preserved_fm() {
        let mut extras = Frontmatter::new();
        extras.insert(
            "last_modified_by".to_string(),
            serde_yaml::Value::String("orchestrator/1.0".to_string()),
        );
        extras.insert(
            "domain".to_string(),
            serde_yaml::Value::String("backend".to_string()),
        );
        let content = render_markdown_with_extras(
            "PRD-001",
            "prd",
            "Test",
            "draft",
            "standard",
            None,
            None,
            None,
            "Body.",
            &[],
            &[],
            Some(&extras),
        )
        .unwrap();
        assert!(content.contains("last_modified_by: orchestrator/1.0"));
        assert!(content.contains("domain: backend"));
    }

    #[test]
    fn render_extras_ignores_known_keys_in_preserved() {
        // Contract violation: caller passes a known key in preserved_fm.
        // The render MUST use the arg-derived value, not the extras one.
        let mut bad_extras = Frontmatter::new();
        bad_extras.insert(
            "status".to_string(),
            serde_yaml::Value::String("active".to_string()),
        );
        bad_extras.insert(
            "id".to_string(),
            serde_yaml::Value::String("PRD-999".to_string()),
        );
        let content = render_markdown_with_extras(
            "PRD-001",
            "prd",
            "Test",
            "draft", // <- truth for status
            "standard",
            None,
            None,
            None,
            "Body.",
            &[],
            &[],
            Some(&bad_extras),
        )
        .unwrap();
        assert!(content.contains("id: PRD-001"));
        assert!(!content.contains("id: PRD-999"));
        assert!(content.contains("status: draft"));
        assert!(!content.contains("status: active"));
    }

    #[tokio::test]
    async fn render_projection_preserves_unknown_fm_across_rerender() {
        // Simulates: agent A stamps last_modified_by, then agent B calls
        // a tool that triggers render_projection for an unrelated reason
        // (e.g. forgeplan_link). The stamp must survive.
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        // Step 1: render initial projection
        let path = render_projection(
            &ws,
            "PRD-010",
            "prd",
            "Multi Agent",
            "draft",
            "standard",
            None,
            None,
            None,
            "## Summary\n\nBody.",
            &[],
        )
        .await
        .unwrap();

        // Step 2: stamp it with agent identity
        let identity =
            crate::artifact::identity::AgentIdentity::new("orchestrator", "1.0").unwrap();
        stamp_agent_identity(&ws, "PRD-010", "prd", "Multi Agent", &identity)
            .await
            .unwrap();

        let stamped = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(stamped.contains("last_modified_by: orchestrator/1.0"));
        assert!(stamped.contains("last_modified_at:"));

        // Step 3: call render_projection again (simulating unrelated tool)
        let path2 = render_projection(
            &ws,
            "PRD-010",
            "prd",
            "Multi Agent",
            "draft",
            "standard",
            None,
            None,
            None,
            "## Summary\n\nBody.",
            &[],
        )
        .await
        .unwrap();

        let after = tokio::fs::read_to_string(&path2).await.unwrap();
        assert!(
            after.contains("last_modified_by: orchestrator/1.0"),
            "identity stamp must survive re-render"
        );
        assert!(after.contains("last_modified_at:"));
    }

    #[tokio::test]
    async fn stamp_overwrites_previous_identity() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        render_projection(
            &ws,
            "PRD-011",
            "prd",
            "Overwrite",
            "draft",
            "standard",
            None,
            None,
            None,
            "body",
            &[],
        )
        .await
        .unwrap();

        let id1 = crate::artifact::identity::AgentIdentity::new("agent-a", "1.0").unwrap();
        stamp_agent_identity(&ws, "PRD-011", "prd", "Overwrite", &id1)
            .await
            .unwrap();

        let id2 = crate::artifact::identity::AgentIdentity::new("agent-b", "2.0").unwrap();
        stamp_agent_identity(&ws, "PRD-011", "prd", "Overwrite", &id2)
            .await
            .unwrap();

        let path = ws.join("prds").join("PRD-011-overwrite.md");
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(content.contains("last_modified_by: agent-b/2.0"));
        assert!(!content.contains("agent-a/1.0"));
    }

    #[tokio::test]
    async fn stamp_fails_cleanly_on_missing_file() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(ws.join("prds")).await.unwrap();

        let id = crate::artifact::identity::AgentIdentity::unknown();
        let result = stamp_agent_identity(&ws, "PRD-404", "prd", "Missing", &id).await;
        assert!(result.is_err(), "should propagate the read error");
    }

    // ============================================================
    // PROB-048 / ADR-003 helpers — sync_before_mutation +
    // render_after_mutation
    // ============================================================

    #[tokio::test]
    async fn sync_before_mutation_is_noop_for_missing_artifact() {
        let temp = TempDir::new().unwrap();
        let ws = temp.path().to_path_buf();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        // Artifact does not exist — helper must not error.
        let result = sync_before_mutation(&ws, &store, "PRD-NEVER-CREATED").await;
        assert!(
            result.is_ok(),
            "sync_before_mutation should be no-op for missing artifact, got {result:?}"
        );
    }

    #[tokio::test]
    async fn render_after_mutation_is_noop_for_missing_artifact() {
        let temp = TempDir::new().unwrap();
        let ws = temp.path().to_path_buf();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let result = render_after_mutation(&ws, &store, "PRD-NEVER-CREATED").await;
        assert!(
            result.is_ok(),
            "render_after_mutation should be no-op for missing artifact, got {result:?}"
        );
    }

    #[tokio::test]
    async fn render_after_mutation_writes_file_with_status_from_store() {
        // The PROB-048 deprecate bug pattern, in miniature:
        //   1. create artifact (status = draft) — file gets status: draft
        //   2. mutate store directly (status → deprecated)
        //   3. call render_after_mutation
        //   4. file should now reflect status: deprecated
        let temp = TempDir::new().unwrap();
        let ws = temp.path().to_path_buf();
        tokio::fs::create_dir_all(ws.join("prds")).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let new_artifact = crate::db::store::NewArtifact {
            id: "PRD-901".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Test artifact".to_string(),
            body: "## Summary\n\nA test PRD.".to_string(),
            depth: "standard".to_string(),
            author: Some("test".to_string()),
            parent_epic: None,
            valid_until: None,
            tags: vec![],
        };
        store.create_artifact(&new_artifact).await.unwrap();

        // Initial render — file has status: draft
        render_after_mutation(&ws, &store, "PRD-901").await.unwrap();
        let file_path = ws.join("prds/PRD-901-test-artifact.md");
        let initial = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(
            initial.contains("status: draft"),
            "initial file should have status: draft, got:\n{initial}"
        );

        // Mutate store: simulate lifecycle::activate behaviour bypassing
        // the projection. This is the bug pattern — store updated, file
        // stale.
        store
            .update_artifact("PRD-901", Some("active"), None)
            .await
            .unwrap();

        // The bug: without render_after_mutation, file would still say draft.
        // With our helper: file gets refreshed.
        render_after_mutation(&ws, &store, "PRD-901").await.unwrap();
        let after = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(
            after.contains("status: active"),
            "post-mutation file should have status: active, got:\n{after}"
        );
        assert!(
            !after.contains("status: draft"),
            "post-mutation file should NOT have status: draft, got:\n{after}"
        );
    }

    // ─── Audit-fix regression tests (2026-05-01) ───────────────────

    /// Audit CRITICAL #1: `create_artifact_with_projection` must reject
    /// path-traversal IDs before composing the filesystem path.
    #[tokio::test]
    async fn create_artifact_rejects_path_traversal_id() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let evil = crate::db::store::NewArtifact {
            id: "../../etc/evil".to_string(),
            kind: "note".to_string(),
            status: "draft".to_string(),
            title: "evil".to_string(),
            body: "owned".to_string(),
            depth: "tactical".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        };
        let result =
            create_artifact_with_projection(&MutationContext::new(&ws, &store), &evil).await;
        assert!(result.is_err(), "must reject path-traversal id");
        // No file written outside the workspace.
        assert!(!tmp.path().join("../../etc/evil-evil.md").exists());
    }

    /// Audit C2: `delete_artifact_with_projection` uses exact-path
    /// removal so deleting `mem-foo` cannot clobber sibling
    /// `mem-foo-bar` whose filename also starts with `mem-foo-`.
    #[tokio::test]
    async fn delete_artifact_exact_path_does_not_clobber_sibling() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        // Create two memory artifacts where one ID is a strict prefix of the other.
        for (id, title) in [("mem-foo", "foo"), ("mem-foo-bar", "foo bar")] {
            let art = crate::db::store::NewArtifact {
                id: id.to_string(),
                kind: "memory".to_string(),
                status: "active".to_string(),
                title: title.to_string(),
                body: "body".to_string(),
                depth: "tactical".to_string(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            };
            create_artifact_with_projection(&MutationContext::new(&ws, &store), &art)
                .await
                .unwrap();
        }

        let mem_dir = ws.join("memory");
        let foo_path = mem_dir.join("mem-foo-foo.md");
        let foo_bar_path = mem_dir.join("mem-foo-bar-foo-bar.md");
        assert!(
            foo_path.exists() && foo_bar_path.exists(),
            "both files must exist before delete"
        );

        delete_artifact_with_projection(&MutationContext::new(&ws, &store), "mem-foo")
            .await
            .unwrap();

        assert!(!foo_path.exists(), "mem-foo file must be removed");
        assert!(
            foo_bar_path.exists(),
            "mem-foo-bar file must NOT be removed (sibling protected)"
        );
    }

    /// Audit H2: `update_metadata_with_projection` must reject empty
    /// status/title at the helper boundary instead of writing yaml-null
    /// values into LanceDB.
    #[tokio::test]
    async fn update_metadata_rejects_empty_status_and_title() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let art = crate::db::store::NewArtifact {
            id: "PRD-901".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Test".to_string(),
            body: "body".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        };
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &art)
            .await
            .unwrap();

        let empty_status = update_metadata_with_projection(
            &MutationContext::new(&ws, &store),
            "PRD-901",
            Some(""),
            None,
        )
        .await;
        assert!(empty_status.is_err(), "must reject empty status");
        let empty_title = update_metadata_with_projection(
            &MutationContext::new(&ws, &store),
            "PRD-901",
            None,
            Some("   "),
        )
        .await;
        assert!(empty_title.is_err(), "must reject whitespace-only title");
    }

    /// Audit H2 + early-return: `update_metadata_with_projection(None, None)`
    /// must short-circuit without the sync round-trip + DB touch.
    #[tokio::test]
    async fn update_metadata_short_circuits_on_no_op() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let art = crate::db::store::NewArtifact {
            id: "PRD-902".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Test".to_string(),
            body: "body".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        };
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &art)
            .await
            .unwrap();

        // Capture the original updated_at — short-circuit MUST NOT bump it.
        let before = store
            .get_record("PRD-902")
            .await
            .unwrap()
            .unwrap()
            .updated_at;
        // Sleep just enough that any DB touch would yield a different timestamp.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        update_metadata_with_projection(&MutationContext::new(&ws, &store), "PRD-902", None, None)
            .await
            .unwrap();

        let after = store
            .get_record("PRD-902")
            .await
            .unwrap()
            .unwrap()
            .updated_at;
        assert_eq!(before, after, "short-circuit must not bump updated_at");
    }

    // ─── Phase 3b/4 audit-fix regression tests (2026-05-01) ──────────

    fn art(id: &str, kind: &str) -> crate::db::store::NewArtifact {
        crate::db::store::NewArtifact {
            id: id.to_string(),
            kind: kind.to_string(),
            status: "draft".to_string(),
            title: format!("Test {id}"),
            body: "body".to_string(),
            depth: "tactical".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        }
    }

    /// A1.1 — sync_artifact_from_file rejects path-traversal IDs.
    #[tokio::test]
    async fn sync_artifact_from_file_rejects_path_traversal_id() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let evil = art("../../etc/evil", "note");
        assert!(
            sync_artifact_from_file(&MutationContext::new(&ws, &store), &evil)
                .await
                .is_err(),
            "must reject path-traversal id at sync_from_file boundary",
        );
    }

    /// A1.2 — sync_body_from_file validates id.
    #[tokio::test]
    async fn sync_body_from_file_rejects_bad_id() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let result = sync_body_from_file(
            &MutationContext::new(&ws, &store),
            "../etc/evil",
            "prd",
            "Title",
            "body",
        )
        .await;
        assert!(result.is_err(), "must reject bad id");
    }

    /// A1.3 — sync_relation_from_file validates BOTH ids.
    #[tokio::test]
    async fn sync_relation_from_file_validates_both_ids() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        assert!(
            sync_relation_from_file(
                &MutationContext::new(&ws, &store),
                "../bad",
                "PRD-001",
                "informs"
            )
            .await
            .is_err(),
            "must reject bad source",
        );
        assert!(
            sync_relation_from_file(
                &MutationContext::new(&ws, &store),
                "PRD-001",
                "../bad",
                "informs"
            )
            .await
            .is_err(),
            "must reject bad target",
        );
    }

    /// A1.4 — delete_orphan_artifact only mutates DB, never disk.
    #[tokio::test]
    async fn delete_orphan_artifact_does_not_touch_disk() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        // Place an artifact on disk + in DB
        let a = art("PRD-905", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &a)
            .await
            .unwrap();
        let file = ws.join("prds").join("PRD-905-test-prd-905.md");
        assert!(file.exists(), "setup: file should exist");

        delete_orphan_artifact(&MutationContext::new(&ws, &store), "PRD-905")
            .await
            .unwrap();
        // DB row gone
        assert!(store.get_record("PRD-905").await.unwrap().is_none());
        // File untouched (caller's responsibility — orphan helper assumes file already gone)
        assert!(
            file.exists(),
            "delete_orphan_artifact must NOT touch the file (caller already removed it)",
        );
    }

    /// A1.5 — delete_orphan_relation cleanup.
    #[tokio::test]
    async fn delete_orphan_relation_drops_edge() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-906", "prd");
        let b = art("EVID-906", "evidence");
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &a)
            .await
            .unwrap();
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &b)
            .await
            .unwrap();
        add_link_with_projection(
            &MutationContext::new(&ws, &store),
            "EVID-906",
            "PRD-906",
            "informs",
        )
        .await
        .unwrap();

        delete_orphan_relation(
            &MutationContext::new(&ws, &store),
            "EVID-906",
            "PRD-906",
            "informs",
        )
        .await
        .unwrap();

        let rels = store.get_relations("EVID-906").await.unwrap();
        assert!(rels.is_empty(), "edge must be gone");
    }

    /// A1.6 — add_links_batch validates all ids up front, no partial state.
    #[tokio::test]
    async fn add_links_batch_validates_up_front_and_no_partial_state() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-907", "prd");
        let b = art("EVID-907", "evidence");
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &a)
            .await
            .unwrap();
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &b)
            .await
            .unwrap();

        // Mix: first link is good, second is path-traversal payload.
        let links = vec![
            ("EVID-907".into(), "PRD-907".into(), "informs".into()),
            ("../../etc/evil".into(), "PRD-907".into(), "informs".into()),
        ];
        let result =
            add_links_batch_with_projection(&MutationContext::new(&ws, &store), &links).await;
        assert!(
            result.is_err(),
            "must bail on bad id BEFORE any side effect"
        );
        // No relations applied — even the first (good) link.
        let rels = store.get_relations("EVID-907").await.unwrap();
        assert!(
            rels.is_empty(),
            "no partial state — the good link must NOT have landed when batch validation failed",
        );
    }

    /// A1.7 — add_links_batch deduplicates renders for repeated participants.
    #[tokio::test]
    async fn add_links_batch_deduplicates_unique_participants() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        // 1 source, 3 distinct targets → 4 unique participants, 3 relations.
        for id in ["EVID-908", "PRD-908", "PRD-909", "PRD-910"] {
            let kind = if id.starts_with("EVID") {
                "evidence"
            } else {
                "prd"
            };
            create_artifact_with_projection(&MutationContext::new(&ws, &store), &art(id, kind))
                .await
                .unwrap();
        }
        let links = vec![
            ("EVID-908".into(), "PRD-908".into(), "informs".into()),
            ("EVID-908".into(), "PRD-909".into(), "informs".into()),
            ("EVID-908".into(), "PRD-910".into(), "informs".into()),
        ];

        let applied = add_links_batch_with_projection(&MutationContext::new(&ws, &store), &links)
            .await
            .unwrap();
        assert_eq!(applied, 3, "all 3 links applied");
        let rels = store.get_relations("EVID-908").await.unwrap();
        assert_eq!(rels.len(), 3, "all 3 outgoing edges in DB");
    }

    /// A1.8 — delete_artifact_after_soft_delete only mutates DB.
    #[tokio::test]
    async fn delete_artifact_after_soft_delete_only_drops_db_row() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-911", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &a)
            .await
            .unwrap();
        let file = ws.join("prds").join("PRD-911-test-prd-911.md");
        assert!(file.exists());

        delete_artifact_after_soft_delete(&MutationContext::new(&ws, &store), "PRD-911")
            .await
            .unwrap();

        assert!(store.get_record("PRD-911").await.unwrap().is_none());
        assert!(
            file.exists(),
            "helper must NOT touch the file (caller already moved it to trash)",
        );
    }

    /// A1.9 — sync_metadata_from_file accepts (None, None) without bumping
    /// updated_at — sync mechanisms shouldn't pay for a no-op.
    #[tokio::test]
    async fn sync_metadata_from_file_no_op_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-912", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &a)
            .await
            .unwrap();

        // Calling with (None, None) should NOT error and should NOT mutate.
        // (Audit code-reviewer #3: catches the inconsistency vs update_metadata_with_projection.)
        let result =
            sync_metadata_from_file(&MutationContext::new(&ws, &store), "PRD-912", None, None)
                .await;
        assert!(result.is_ok(), "no-op sync should succeed");
    }

    /// MEDIUM-1 (code-reviewer 2026-05-01) — sync_metadata_from_file must
    /// reject empty-string status/title, mirroring the sibling helper's
    /// H2 silent-corruption guard. A YAML `status: ""` from a pulled file
    /// must NOT land in DB as yaml-null.
    #[tokio::test]
    async fn sync_metadata_from_file_rejects_empty_status_and_title() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-913", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &a)
            .await
            .unwrap();

        let empty_status = sync_metadata_from_file(
            &MutationContext::new(&ws, &store),
            "PRD-913",
            Some(""),
            None,
        )
        .await;
        assert!(empty_status.is_err(), "must reject empty status");
        let empty_title = sync_metadata_from_file(
            &MutationContext::new(&ws, &store),
            "PRD-913",
            None,
            Some("   "),
        )
        .await;
        assert!(empty_title.is_err(), "must reject whitespace-only title");
    }

    // =========================================================================
    // PRD-073 Phase 3c (Wave 1C) — typed-error branches.
    // Each helper got `MutationResult<T>`; these tests pin the variant a caller
    // can match on so future audits catch regressions where the helper falls
    // back to a generic `StoreError` instead of the precise typed signal.
    // =========================================================================

    #[tokio::test]
    async fn sync_metadata_from_file_returns_invalid_id_for_traversal() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = sync_metadata_from_file(
            &MutationContext::new(&ws, &store),
            "../../etc/passwd",
            Some("draft"),
            None,
        )
        .await
        .expect_err("traversal id must be rejected");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s == "../../etc/passwd"),
            "expected InvalidId, got {err:?}",
        );
    }

    #[tokio::test]
    async fn sync_metadata_from_file_returns_empty_field_for_blank_status() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-921", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &a)
            .await
            .unwrap();

        let err = sync_metadata_from_file(
            &MutationContext::new(&ws, &store),
            "PRD-921",
            Some("   "),
            None,
        )
        .await
        .expect_err("whitespace status must be rejected");
        assert!(
            matches!(err, MutationError::EmptyField { field: "status" }),
            "expected EmptyField{{status}}, got {err:?}",
        );
    }

    #[tokio::test]
    async fn sync_relation_from_file_returns_invalid_id_for_bad_source() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = sync_relation_from_file(
            &MutationContext::new(&ws, &store),
            "../bad",
            "PRD-001",
            "informs",
        )
        .await
        .expect_err("bad source id must be rejected");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s == "../bad"),
            "expected InvalidId(source), got {err:?}",
        );
    }

    #[tokio::test]
    async fn delete_orphan_artifact_returns_invalid_id_for_traversal() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = delete_orphan_artifact(&MutationContext::new(&ws, &store), "../../boom")
            .await
            .expect_err("traversal id must be rejected");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s == "../../boom"),
            "expected InvalidId, got {err:?}",
        );
    }

    #[tokio::test]
    async fn delete_orphan_relation_returns_invalid_id_for_bad_target() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = delete_orphan_relation(
            &MutationContext::new(&ws, &store),
            "PRD-001",
            "../bad-target",
            "informs",
        )
        .await
        .expect_err("bad target id must be rejected");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s == "../bad-target"),
            "expected InvalidId(target), got {err:?}",
        );
    }

    #[tokio::test]
    async fn add_links_batch_returns_invalid_id_before_any_write() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        for id in ["PRD-922", "EVID-922"] {
            let kind = if id.starts_with("EVID") {
                "evidence"
            } else {
                "prd"
            };
            create_artifact_with_projection(&MutationContext::new(&ws, &store), &art(id, kind))
                .await
                .unwrap();
        }

        let links = vec![
            ("EVID-922".into(), "PRD-922".into(), "informs".into()),
            ("../../evil".into(), "PRD-922".into(), "informs".into()),
        ];
        let err = add_links_batch_with_projection(&MutationContext::new(&ws, &store), &links)
            .await
            .expect_err("traversal id must bail batch");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s == "../../evil"),
            "expected InvalidId, got {err:?}",
        );
        let rels = store.get_relations("EVID-922").await.unwrap();
        assert!(
            rels.is_empty(),
            "no relations should land when validation fails up front",
        );
    }

    #[tokio::test]
    async fn delete_artifact_after_soft_delete_returns_invalid_id_for_traversal() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = delete_artifact_after_soft_delete(&MutationContext::new(&ws, &store), "..\\evil")
            .await
            .expect_err("traversal id must be rejected");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s == "..\\evil"),
            "expected InvalidId, got {err:?}",
        );
    }

    // ─── PRD-073 Phase 3c Wave 1B: typed-error regression tests ──────

    /// Wave 1B — `remove_tags_with_projection` propagates `InvalidId` for
    /// path-traversal payloads instead of leaking the underlying anyhow
    /// error to MCP. Closes audit H1 follow-up for the tag path.
    #[tokio::test]
    async fn remove_tags_with_projection_returns_invalid_id_for_traversal() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = remove_tags_with_projection(
            &MutationContext::new(&ws, &store),
            "../../evil",
            &["x".to_string()],
        )
        .await
        .expect_err("traversal id must be rejected");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s == "../../evil"),
            "expected InvalidId(\"../../evil\"), got {err:?}",
        );
    }

    /// Wave 1B — `add_link_with_projection` rejects path-traversal in either
    /// source or target. Without typed errors MCP would warn-and-continue
    /// the bad source, leaving the relation only partially observed.
    #[tokio::test]
    async fn add_link_with_projection_returns_invalid_id_for_bad_source_or_target() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = add_link_with_projection(
            &MutationContext::new(&ws, &store),
            "../bad",
            "PRD-001",
            "informs",
        )
        .await
        .expect_err("bad source must be rejected");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s == "../bad"),
            "expected InvalidId source, got {err:?}",
        );

        let err = add_link_with_projection(
            &MutationContext::new(&ws, &store),
            "PRD-001",
            "../bad",
            "informs",
        )
        .await
        .expect_err("bad target must be rejected");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s == "../bad"),
            "expected InvalidId target, got {err:?}",
        );
    }

    /// Wave 1B — symmetric guard for `delete_link_with_projection`.
    #[tokio::test]
    async fn delete_link_with_projection_returns_invalid_id_for_bad_source_or_target() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = delete_link_with_projection(
            &MutationContext::new(&ws, &store),
            "../bad",
            "PRD-001",
            "informs",
        )
        .await
        .expect_err("bad source must be rejected");
        assert!(matches!(err, MutationError::InvalidId(ref s) if s == "../bad"));

        let err = delete_link_with_projection(
            &MutationContext::new(&ws, &store),
            "PRD-001",
            "../bad",
            "informs",
        )
        .await
        .expect_err("bad target must be rejected");
        assert!(matches!(err, MutationError::InvalidId(ref s) if s == "../bad"));
    }

    /// Wave 1B — `sync_artifact_from_file` returns `FileNotFound` when the
    /// caller-supplied artifact has no on-disk markdown projection. This is
    /// the file-first invariant from ADR-003: refuse to land a DB row
    /// without a markdown source. TOCTOU class — caller may have read the
    /// file moments ago, but a concurrent delete strands us here.
    #[tokio::test]
    async fn sync_artifact_from_file_returns_file_not_found_when_missing() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        // Construct a NewArtifact with a valid id+kind+title but DON'T
        // write the projection on disk. Sync must refuse.
        let ghost = art("PRD-930", "prd");
        let err = sync_artifact_from_file(&MutationContext::new(&ws, &store), &ghost)
            .await
            .expect_err("missing file must be rejected with FileNotFound");
        match err {
            MutationError::FileNotFound { id, path } => {
                assert_eq!(id, "PRD-930");
                assert_eq!(
                    path,
                    std::path::PathBuf::from("prds/PRD-930-test-prd-930.md"),
                    "path must be workspace-relative (no abs-path leak per R1 H-8)",
                );
                assert!(!path.is_absolute(), "FileNotFound.path must be relative",);
            }
            other => panic!("expected FileNotFound, got {other:?}"),
        }
        assert!(
            store.get_record("PRD-930").await.unwrap().is_none(),
            "no DB row should land when file is missing",
        );
    }

    /// Wave 1B — `sync_body_from_file` returns `FileNotFound` when the
    /// projection markdown is gone, even if the DB row exists. Closes the
    /// silent-corruption gap where `update_body` would land in LanceDB but
    /// the next reindex would clobber it from a missing/stale on-disk body.
    #[tokio::test]
    async fn sync_body_from_file_returns_file_not_found_when_missing() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        // Create the artifact (so DB row exists) and then nuke the file
        // to simulate the TOCTOU race.
        let a = art("PRD-931", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &a)
            .await
            .unwrap();
        let file = ws.join("prds").join("PRD-931-test-prd-931.md");
        assert!(file.exists(), "setup precondition: file must exist");
        tokio::fs::remove_file(&file).await.unwrap();

        let err = sync_body_from_file(
            &MutationContext::new(&ws, &store),
            "PRD-931",
            "prd",
            "Test PRD-931",
            "new body",
        )
        .await
        .expect_err("missing file must be rejected with FileNotFound");
        // R1 audit H-8: path is now workspace-relative — strip prefix to compare.
        let expected_rel = std::path::PathBuf::from("prds/PRD-931-test-prd-931.md");
        match err {
            MutationError::FileNotFound { id, path } => {
                assert_eq!(id, "PRD-931");
                assert_eq!(path, expected_rel);
                assert!(
                    !path.is_absolute(),
                    "FileNotFound.path must be relative (no abs-path leak)"
                );
            }
            other => panic!("expected FileNotFound, got {other:?}"),
        }
    }

    /// R1 audit CRITICAL (rust-expert): regression guard for the slug-fallback
    /// drift between `create_artifact_with_projection` and `sync_*_from_file`.
    /// Pre-fix: a Cyrillic / CJK / emoji title slugified to "" in `create`,
    /// where the `untitled` fallback triggered, but `sync_artifact_from_file`
    /// and `sync_body_from_file` rebuilt the path with raw `slugify(title)` →
    /// `prds/PRD-XXX-.md`, which never existed → spurious `FileNotFound`.
    ///
    /// Post-fix: the single `projection_slug()` helper applies the fallback
    /// at every path-construction site. Sync round-trips on a Cyrillic-title
    /// artifact must succeed.
    #[tokio::test]
    async fn projection_slug_fallback_consistent_across_create_and_sync() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        // R2 audit M-R2-2: replaced fragile premise check
        // `assert!(slugify(cyrillic_title).is_empty())` with a *behavioral*
        // guard on `projection_slug`. The original premise would panic
        // (silently disabling the real round-trip assertions) if slugify
        // ever transliterated Cyrillic. The contract this test depends on
        // is "projection_slug always produces a non-empty slug" — that's
        // invariant under slugify impl changes.
        let cyrillic_title = "Тестовый артефакт";
        let slug = projection_slug(cyrillic_title);
        assert!(
            !slug.is_empty(),
            "projection_slug must always produce a non-empty slug regardless of slugify impl, got: {slug:?}",
        );

        let mut a = art("PRD-940", "prd");
        a.title = cyrillic_title.to_string();
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &a)
            .await
            .expect("create with Cyrillic title must succeed");

        // The on-disk file is at the slug computed by `projection_slug`.
        // Today that's `<id>-untitled.md` (slugify returns "" for Cyrillic
        // → fallback fires). After a future transliteration change it
        // could be `<id>-testovyy-artefakt.md`. Either way the round-trip
        // must work.
        let expected = ws.join("prds").join(format!("PRD-940-{slug}.md"));
        assert!(
            expected.exists(),
            "create_artifact_with_projection must write the projection at the `untitled` fallback path, but {expected:?} does not exist",
        );

        // The DB row keeps the original Cyrillic title.
        let rec = store.get_record("PRD-940").await.unwrap().unwrap();
        assert_eq!(
            rec.title, cyrillic_title,
            "DB title must preserve the user's original (only the on-disk slug is sanitised)",
        );

        // The file frontmatter ALSO keeps the Cyrillic title (regression guard
        // for the prior `effective_title = "untitled"` body bug — the file
        // body must not say `title: "untitled"`).
        let body = tokio::fs::read_to_string(&expected).await.unwrap();
        assert!(
            body.contains(cyrillic_title),
            "file frontmatter must keep the Cyrillic title, got: {body}",
        );

        // Now exercise the round-trip — sync_body_from_file must resolve the
        // path to the SAME file and NOT return FileNotFound. (Skip
        // sync_artifact_from_file — that is the bootstrap-from-file path and
        // would conflict with the row we just created via create_artifact.)
        sync_body_from_file(
            &MutationContext::new(&ws, &store),
            "PRD-940",
            "prd",
            cyrillic_title,
            "new body",
        )
        .await
        .expect(
            "sync_body_from_file with Cyrillic title must succeed — \
                 a `FileNotFound` here is the slug-fallback drift regression",
        );
    }

    /// R2 audit M-R2-1 fix: actually exercise the M-1 disambiguation
    /// between ENOENT and EACCES. Before this test, only ENOENT was
    /// covered (and that test name overclaimed). On Unix we can chmod the
    /// parent directory to 0o000 and confirm `metadata()` returns
    /// `PermissionDenied` (NOT `NotFound`) → typed-error contract routes
    /// it as `StoreError` (recoverable: true), NOT `FileNotFound`.
    ///
    /// This is the regression guard for "permission-denied wrongly
    /// classified as file-missing" — the audit risk that motivated M-1.
    #[cfg(unix)]
    #[tokio::test]
    async fn sync_artifact_returns_store_error_for_eacces_not_filenotfound() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();
        let prds_dir = ws.join("prds");
        tokio::fs::create_dir_all(&prds_dir).await.unwrap();

        // Lock the parent dir so any `metadata()` on a child returns
        // EACCES, not ENOENT.
        let mut perms = tokio::fs::metadata(&prds_dir).await.unwrap().permissions();
        perms.set_mode(0o000);
        tokio::fs::set_permissions(&prds_dir, perms.clone())
            .await
            .unwrap();

        let ghost = art("PRD-942", "prd");
        let result = sync_artifact_from_file(&MutationContext::new(&ws, &store), &ghost).await;

        // Restore perms before asserting so a panic doesn't leave an
        // unreadable temp dir behind.
        let mut restored = perms.clone();
        restored.set_mode(0o755);
        tokio::fs::set_permissions(&prds_dir, restored)
            .await
            .unwrap();

        let err = result.expect_err("EACCES must produce some MutationError");
        // The disambiguation contract: EACCES routes to StoreTransient,
        // NOT FileNotFound. If this assertion ever flips to FileNotFound,
        // the M-1 fix has regressed. PROB-049 H-1: split `StoreError` →
        // `StoreTransient` (recoverable) / `StoreFatal` (not). EACCES is
        // operator-fixable, so it must land in the transient bucket.
        assert!(
            matches!(err, MutationError::StoreTransient(_)),
            "EACCES must surface as StoreTransient (recoverable=true), NOT FileNotFound. got: {err:?}",
        );
        assert!(
            err.is_recoverable(),
            "StoreTransient from EACCES must be classified as recoverable (operator can fix perms and retry)",
        );
    }

    /// Companion to the EACCES test: confirm the typed contract for the
    /// happy `ErrorKind::NotFound` branch. Names the contract being tested
    /// directly — no overclaim.
    #[tokio::test]
    async fn sync_artifact_returns_filenotfound_for_missing_file() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let ghost = art("PRD-941", "prd");
        let err = sync_artifact_from_file(&MutationContext::new(&ws, &store), &ghost)
            .await
            .expect_err("missing file must be rejected with FileNotFound");
        assert!(
            matches!(err, MutationError::FileNotFound { ref id, .. } if id == "PRD-941"),
            "ENOENT must surface as typed FileNotFound, got: {err:?}",
        );
    }

    // =========================================================================
    // PRD-073 Phase 3c (Wave 1A) — typed-error branches for the 5 owned helpers.
    // Each test pins the typed `MutationError` variant a caller can match on so
    // future audits catch regressions where the helper falls back to a generic
    // anyhow string-error instead of the precise typed signal.
    // =========================================================================

    #[tokio::test]
    async fn wave1a_create_artifact_rejects_path_traversal_id() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let mut a = art("PRD-001", "prd");
        a.id = "../../etc/evil".to_string();

        let err = create_artifact_with_projection(&MutationContext::new(&ws, &store), &a)
            .await
            .expect_err("path-traversal id must be rejected");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s == "../../etc/evil"),
            "expected MutationError::InvalidId, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn wave1a_create_artifact_rejects_unknown_kind() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let mut a = art("PRD-700", "prd");
        a.kind = "bogus_kind".to_string();

        let err = create_artifact_with_projection(&MutationContext::new(&ws, &store), &a)
            .await
            .expect_err("unknown kind must be rejected");
        assert!(
            matches!(err, MutationError::InvalidKind { ref id, ref kind, .. }
                if id == "PRD-700" && kind == "bogus_kind"),
            "expected MutationError::InvalidKind, got: {err:?}"
        );
    }

    /// PRD-073 Phase 3c LOW-5: a title that slugifies to empty (e.g. all
    /// non-ASCII characters such as Cyrillic) must produce a sane filename
    /// with the "untitled" fallback rather than `<id>-.md`.
    #[tokio::test]
    async fn wave1a_create_artifact_falls_back_to_untitled_when_slug_empty() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let mut a = art("PRD-701", "prd");
        a.title = "Задача".to_string();

        let path = create_artifact_with_projection(&MutationContext::new(&ws, &store), &a)
            .await
            .expect("cyrillic title must not error");
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(
            filename, "PRD-701-untitled.md",
            "expected `untitled` fallback slug, got: {filename}"
        );
        // DB row keeps the original title — only the on-disk slug changes.
        let row = store.get_record("PRD-701").await.unwrap().unwrap();
        assert_eq!(row.title, "Задача");
    }

    #[tokio::test]
    async fn wave1a_delete_artifact_rejects_path_traversal_id() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = delete_artifact_with_projection(&MutationContext::new(&ws, &store), "../escape")
            .await
            .expect_err("path-traversal id must be rejected");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s == "../escape"),
            "expected MutationError::InvalidId, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn wave1a_update_body_rejects_invalid_id() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err =
            update_body_with_projection(&MutationContext::new(&ws, &store), "1bad-start", "body")
                .await
                .expect_err("id starting with digit must be rejected");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s == "1bad-start"),
            "expected MutationError::InvalidId, got: {err:?}"
        );
    }

    /// `update_body_with_projection` distinguishes the missing-row case from
    /// transient I/O: a well-formed but unknown id surfaces as `RowNotFound`,
    /// which `is_recoverable() == false`. Lead-added (post-Wave 1A) per the
    /// audit follow-up that flagged the prior `StoreError` mapping as
    /// misleading for MCP strict-mode callers.
    #[tokio::test]
    async fn lead_update_body_missing_artifact_is_row_not_found() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err =
            update_body_with_projection(&MutationContext::new(&ws, &store), "PRD-9999", "body")
                .await
                .expect_err("missing artifact must surface an error");
        assert!(
            matches!(err, MutationError::RowNotFound { ref id } if id == "PRD-9999"),
            "expected MutationError::RowNotFound, got: {err:?}"
        );
        assert!(
            !err.is_recoverable(),
            "missing-row is an input error, not a transient I/O failure"
        );
    }

    /// R1 audit HIGH-3 (code-review): explicit happy-path test for
    /// `update_body_with_projection`. The pre-existing `wave1a_*` suite
    /// covered invalid-id and missing-row, but no test asserted that the
    /// happy path actually persists the body. Without this, a Phase 3c
    /// migration mistake (e.g. flipped `?` operators) could only be caught
    /// by callers at runtime.
    #[tokio::test]
    async fn update_body_with_projection_happy_path_persists_body() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-942", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &store), &a)
            .await
            .unwrap();

        let new_body = "## Updated body\n\nfresh content for happy-path";
        update_body_with_projection(&MutationContext::new(&ws, &store), "PRD-942", new_body)
            .await
            .expect("happy path must succeed");

        // DB row body is updated.
        let rec = store.get_record("PRD-942").await.unwrap().unwrap();
        assert!(
            rec.body.contains("fresh content for happy-path"),
            "DB body must reflect the update, got: {}",
            rec.body
        );

        // File body is updated too (file-first invariant).
        let path = ws.join("prds").join("PRD-942-test-prd-942.md");
        let on_disk = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(
            on_disk.contains("fresh content for happy-path"),
            "file body must reflect the update, got: {on_disk}"
        );
    }

    /// R1 audit HIGH-1 (rust-expert): pin the priority order when both
    /// `status` and `title` are blank. The current implementation rejects
    /// `status` first; this test makes the contract explicit so future
    /// refactors don't silently flip the order.
    #[tokio::test]
    async fn sync_metadata_from_file_rejects_blank_status_before_blank_title() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = sync_metadata_from_file(
            &MutationContext::new(&ws, &store),
            "PRD-943",
            Some(""),
            Some(""),
        )
        .await
        .expect_err("both fields blank must surface a typed error");
        assert!(
            matches!(err, MutationError::EmptyField { field } if field == "status"),
            "status is rejected first by current contract; got: {err:?}",
        );
    }

    #[tokio::test]
    async fn wave1a_update_depth_rejects_invalid_id() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = update_depth_with_projection(
            &MutationContext::new(&ws, &store),
            "bad/slash",
            "tactical",
        )
        .await
        .expect_err("id with slash must be rejected");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s == "bad/slash"),
            "expected MutationError::InvalidId, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn wave1a_add_tags_rejects_empty_id() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = add_tags_with_projection(
            &MutationContext::new(&ws, &store),
            "",
            &["tag-one".to_string()],
        )
        .await
        .expect_err("empty id must be rejected");
        assert!(
            matches!(err, MutationError::InvalidId(ref s) if s.is_empty()),
            "expected MutationError::InvalidId, got: {err:?}"
        );
    }
}
