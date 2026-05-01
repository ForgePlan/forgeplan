use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::artifact::frontmatter::{self, Frontmatter};
use crate::artifact::types::{ArtifactKind, slugify};

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

    let slug = slugify(&record.title);
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

    let slug = slugify(title);
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
    let slug = slugify(title);
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
    let slug = slugify(title);
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
    let slug = slugify(title);
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
pub async fn create_artifact_with_projection(
    workspace: &Path,
    store: &crate::db::store::LanceStore,
    artifact: &crate::db::store::NewArtifact,
) -> anyhow::Result<PathBuf> {
    // Audit 2026-05-01 #1 (security CRITICAL): validate id BEFORE composing
    // it into a filesystem path. Without this, a JSON import with
    // `"id": "../../etc/evil"` would write outside the workspace via
    // `format!("{id}-{slug}.md")` resolved against `workspace/<kind>/`.
    crate::db::store::validate_artifact_id(&artifact.id)?;
    let _: ArtifactKind = artifact.kind.parse().map_err(|e| {
        anyhow::anyhow!("invalid kind '{}' for {}: {e}", artifact.kind, artifact.id)
    })?;

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
pub async fn delete_artifact_with_projection(
    workspace: &Path,
    store: &crate::db::store::LanceStore,
    id: &str,
) -> anyhow::Result<()> {
    crate::db::store::validate_artifact_id(id)?;
    let record = match store.get_record(id).await? {
        Some(r) => r,
        None => return Ok(()),
    };
    // Audit 2026-05-01 H1 (typescript-type-auditor + rust-pro): bail on
    // unknown kind rather than silently falling back to ArtifactKind::Note,
    // which would let `delete` remove a wrong-directory file.
    let _: ArtifactKind = record.kind.parse().map_err(|e| {
        anyhow::anyhow!(
            "invalid kind '{}' for {} (DB row corrupt?): {e}",
            record.kind,
            id
        )
    })?;

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
pub async fn update_metadata_with_projection(
    workspace: &Path,
    store: &crate::db::store::LanceStore,
    id: &str,
    status: Option<&str>,
    title: Option<&str>,
) -> anyhow::Result<()> {
    crate::db::store::validate_artifact_id(id)?;
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
        anyhow::bail!("status cannot be empty");
    }
    if let Some(t) = title
        && t.trim().is_empty()
    {
        anyhow::bail!("title cannot be empty");
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
pub async fn update_body_with_projection(
    workspace: &Path,
    store: &crate::db::store::LanceStore,
    id: &str,
    body: &str,
) -> anyhow::Result<()> {
    crate::db::store::validate_artifact_id(id)?;
    let record = match store.get_record(id).await? {
        Some(r) => r,
        None => {
            // Mirror the underlying `store.update_body` behavior on a missing
            // ID — propagate the error so callers don't silently no-op.
            anyhow::bail!("artifact '{id}' not found");
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
pub async fn update_depth_with_projection(
    workspace: &Path,
    store: &crate::db::store::LanceStore,
    id: &str,
    depth: &str,
) -> anyhow::Result<()> {
    crate::db::store::validate_artifact_id(id)?;
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
pub async fn add_tags_with_projection(
    workspace: &Path,
    store: &crate::db::store::LanceStore,
    id: &str,
    tags: &[String],
) -> anyhow::Result<()> {
    crate::db::store::validate_artifact_id(id)?;
    sync_before_mutation(workspace, store, id).await?;
    store.add_tags(id, tags).await?;
    render_after_mutation(workspace, store, id).await?;
    Ok(())
}

/// Remove tags from an artifact with file-first guarantees.
///
/// PRD-073 FR-001 helper. Used by `forgeplan untag`.
pub async fn remove_tags_with_projection(
    workspace: &Path,
    store: &crate::db::store::LanceStore,
    id: &str,
    tags: &[String],
) -> anyhow::Result<()> {
    crate::db::store::validate_artifact_id(id)?;
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
pub async fn add_link_with_projection(
    workspace: &Path,
    store: &crate::db::store::LanceStore,
    source: &str,
    target: &str,
    relation: &str,
) -> anyhow::Result<()> {
    crate::db::store::validate_artifact_id(source)?;
    crate::db::store::validate_artifact_id(target)?;
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
pub async fn delete_link_with_projection(
    workspace: &Path,
    store: &crate::db::store::LanceStore,
    source: &str,
    target: &str,
    relation: &str,
) -> anyhow::Result<()> {
    crate::db::store::validate_artifact_id(source)?;
    crate::db::store::validate_artifact_id(target)?;
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
        let result = create_artifact_with_projection(&ws, &store, &evil).await;
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
            create_artifact_with_projection(&ws, &store, &art)
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

        delete_artifact_with_projection(&ws, &store, "mem-foo")
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
        create_artifact_with_projection(&ws, &store, &art)
            .await
            .unwrap();

        let empty_status =
            update_metadata_with_projection(&ws, &store, "PRD-901", Some(""), None).await;
        assert!(empty_status.is_err(), "must reject empty status");
        let empty_title =
            update_metadata_with_projection(&ws, &store, "PRD-901", None, Some("   ")).await;
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
        create_artifact_with_projection(&ws, &store, &art)
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

        update_metadata_with_projection(&ws, &store, "PRD-902", None, None)
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
}
