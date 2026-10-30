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
//! All mutator helpers take a [`MutationContext`] (workspace path + the
//! caller's workspace-lock guard + the optional legacy derived-index sink)
//! so callers don't have to thread those arguments through every
//! invocation.
//!
//! # Two workspace layouts, one mutation flow
//!
//! [`MutationContext::legacy_sink`] is `Some` on a workspace whose derived
//! index is still the legacy `artifacts.lance`, and `None` on a workspace
//! the constellation protocol has claimed. Every helper here handles both:
//!
//! - **`Some`** — the historical file-first triplet: canonical Markdown
//!   write, then the derived row.
//! - **`None`** — the canonical Markdown write still happens, in full, with
//!   the same validation and the same error ordering. Only the derived-sync
//!   step is skipped, because there is no legacy index to write to and
//!   `forgeplan index` / reconcile publishes the row afterwards.
//!
//! The rule that keeps this honest: a helper whose job includes a canonical
//! Markdown mutation MUST NOT return a named refusal when the sink is
//! `None`. Doing so would leave a real mutation unperformed. Only
//! derived-index-ONLY helpers — orphan-row prunes, and the row drop after a
//! soft delete whose file already moved — use
//! [`MutationContext::require_legacy_sink`] and can therefore report
//! [`MutationError::NoLegacySink`].
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
// Wave 1.5 SEC-H3 follow-up: re-export the path-display sanitiser as
// well so tracing log sites (e.g. `phase::store::read_phase` symlink
// rejection) can mask `path.display()` through the same HOME /
// scratch-dir discipline (CWE-200).
pub use error::sanitize_path_for_display;

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
/// `render_projection*`, `compare_projection`,
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
        &[],
        false,
        ProjectionBodyMode::Body,
        None,
        None,
        false,
    )
    .await
}

/// Render a full ArtifactRecord (includes tags) to its markdown file.
/// Used by mutations like `tag` / `untag` that need to persist tags to
/// frontmatter so they survive a reindex (ADR-003 files-first).
///
/// Files-first: an existing non-empty file body wins over `record.body`.
/// For a mutation that *appends to the body* (the lifecycle sections), use
/// [`render_projection_record_with_body`] instead — see #478.
pub async fn render_projection_record(
    workspace: &Path,
    record: &crate::db::store::ArtifactRecord,
    links: &[(String, String)],
) -> anyhow::Result<PathBuf> {
    render_projection_record_inner(workspace, record, links, false).await
}

/// Like [`render_projection_record`], but writes `record.body` verbatim
/// instead of preserving whatever the file already holds.
///
/// #478: `deprecate` / `renew` / `reopen` append a `## Deprecation` /
/// `## Renewal` / `## Reopened` section to the body. Rendered files-first,
/// that section is discarded — the status reaches the file (it is
/// frontmatter) and the reason does not. Because `lance/` is gitignored and
/// the next mutation syncs the section-less file body back over the DB, the
/// reason ends up in neither place.
///
/// Only safe when the caller has just synced file → store, so that
/// `record.body` is the file body plus the appended section. Do not reach
/// for this in `link` / `tag` / `activate`: they pass a possibly-stale DB
/// body, and files-first is what protects a user's edits there.
pub async fn render_projection_record_with_body(
    workspace: &Path,
    record: &crate::db::store::ArtifactRecord,
    links: &[(String, String)],
) -> anyhow::Result<PathBuf> {
    render_projection_record_inner(workspace, record, links, true).await
}

async fn render_projection_record_inner(
    workspace: &Path,
    record: &crate::db::store::ArtifactRecord,
    links: &[(String, String)],
    force_body: bool,
) -> anyhow::Result<PathBuf> {
    let artifact_kind = record
        .kind
        .parse::<ArtifactKind>()
        .unwrap_or(ArtifactKind::Note);
    let dir = workspace.join(artifact_kind.dir_name());
    tokio::fs::create_dir_all(&dir).await?;

    let slug = projection_slug(&record.title);
    let filename = format!("{}-{}.md", record.id, slug);
    // An existing document keeps its path: a document published under its
    // canonical key is found by the key it declares, not by a name derived
    // from the title.
    let legacy = dir.join(&filename);
    let filepath = if legacy.exists() {
        legacy
    } else {
        find_existing_projection_file(
            workspace,
            &dir,
            &record.id,
            declared_canonical_key(Some(&record.canonical_key)),
        )
        .await?
        .unwrap_or(legacy)
    };

    // Files-first: preserve existing body + agent-owned fm keys (PRD-057 FR-009).
    // `force_body` (#478) opts out for mutations that append to the body.
    //
    // L02: an existing, parseable file's body is the canonical body **including
    // when it is empty**. The earlier `!file_body.trim().is_empty()` guard read an
    // intentionally cleared body as "no body yet" and restored `record.body` over
    // it, so the very next mutation after a clear resurrected the text the user had
    // deleted — and `sync_file_to_store` had already judged the cleared body a real
    // change, so the two directions disagreed about the same file. A caller that
    // genuinely means "write the row's body over the file" uses the `_with_body`
    // forcing variant; there is no third reading of an empty body.
    let (effective_body, preserved_fm) = if force_body {
        (record.body.clone(), read_preserved_fm(&filepath).await)
    } else if filepath.exists() {
        match tokio::fs::read_to_string(&filepath).await {
            Ok(file_content) => match frontmatter::parse_frontmatter(&file_content) {
                Ok((fm, mut file_body)) => {
                    // The renderer supplies this newline; do not accumulate
                    // another one on each metadata-only mutation.
                    if file_body.ends_with('\n') {
                        file_body.pop();
                    }
                    (file_body, filter_preserved(&fm))
                }
                // Unparseable markdown is not a body and not an edit: this is
                // the "could not look" case, and it leaves the row's text alone
                // rather than claiming the file declared an empty body.
                Err(_) => (record.body.clone(), None),
            },
            Err(_) => (record.body.clone(), None),
        }
    } else {
        (record.body.clone(), None)
    };

    // A document that does not exist yet declares the key it is created
    // under. An existing document keeps the key it already carries.
    let preserved_fm = match preserved_fm {
        Some(fm) => Some(fm),
        None => declared_canonical_key(Some(&record.canonical_key)).map(|key| {
            let mut fm = Frontmatter::new();
            fm.insert(
                "slug".to_string(),
                serde_yaml::Value::String(key.to_string()),
            );
            fm
        }),
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
        ProjectionBodyMode::Body,
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
        &[],
        true,
        ProjectionBodyMode::Body,
        None,
        None,
        false,
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
    tags: &[String],
    force_body: bool,
    body_mode: ProjectionBodyMode,
    canonical_key: Option<&str>,
    explicit_path: Option<PathBuf>,
    create_only: bool,
) -> anyhow::Result<PathBuf> {
    let artifact_kind = kind.parse::<ArtifactKind>().unwrap_or(ArtifactKind::Note);
    let dir = workspace.join(artifact_kind.dir_name());
    tokio::fs::create_dir_all(&dir).await?;

    let slug = projection_slug(title);
    let filename = format!("{}-{}.md", id, slug);
    let filepath = match explicit_path {
        Some(path) => path,
        None => {
            // An existing document keeps its path: a document published
            // under its canonical key is found by the key it declares, not
            // by a name derived from the title.
            let legacy = dir.join(&filename);
            if legacy.exists() {
                legacy
            } else {
                find_existing_projection_file(workspace, &dir, id, canonical_key)
                    .await?
                    .unwrap_or(legacy)
            }
        }
    };

    // Files-first (RFC-004): preserve the file body when the file exists.
    // Only frontmatter is updated from LanceDB. Body comes from the file on disk.
    // Exception: force_body=true (from `update --body`) uses the passed body.
    //
    // L02: an existing, parseable file's body is canonical **including when it is
    // empty** — same rule and same reason as `render_projection_record_inner`. A
    // `link` / `tag` / `activate` re-render after a user cleared the body must not
    // put the cleared text back, and must not need `update --body` to avoid it.
    //
    // PRD-057 FR-009: also preserve unknown frontmatter keys (anything not in
    // KNOWN_FM_KEYS) so agent-owned fields such as `last_modified_by` /
    // `last_modified_at` survive re-renders.
    let (effective_body, preserved_fm) = if force_body {
        (body.to_string(), read_preserved_fm(&filepath).await)
    } else if filepath.exists() {
        match tokio::fs::read_to_string(&filepath).await {
            Ok(file_content) => match frontmatter::parse_frontmatter(&file_content) {
                Ok((fm, mut file_body)) => {
                    if file_body.ends_with('\n') {
                        file_body.pop();
                    }
                    (file_body, filter_preserved(&fm))
                }
                Err(_) => (body.to_string(), None),
            },
            Err(_) => (body.to_string(), None),
        }
    } else {
        (body.to_string(), None)
    };

    // A document that does not exist yet declares the key it is created
    // under. An existing document keeps the key it already carries — the
    // canonical key is immutable through ordinary updates, and the
    // preserved frontmatter is where it survives a re-render.
    let preserved_fm = match preserved_fm {
        Some(fm) => Some(fm),
        None => canonical_key.map(|key| {
            let mut fm = Frontmatter::new();
            fm.insert(
                "slug".to_string(),
                serde_yaml::Value::String(key.to_string()),
            );
            fm
        }),
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
        tags,
        preserved_fm.as_ref(),
        body_mode,
    )?;
    // PRD-073 audit C1: atomic write protects against kill-mid-truncate window.
    if create_only {
        // Publication never clobbers: the write fails when the target path
        // already exists, so an existing document is never overwritten.
        atomic_markdown_create(&filepath, content.as_bytes()).await?;
    } else {
        atomic_markdown_write(&filepath, content.as_bytes()).await?;
    }

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

/// The canonical key a freshly written document declares, if there is one.
///
/// An empty key is not a key: a row that carries none simply does not
/// declare one, and the document keeps whatever it already had.
fn declared_canonical_key(canonical_key: Option<&str>) -> Option<&str> {
    canonical_key.filter(|key| !key.trim().is_empty())
}

/// Find the file an existing document already lives in.
///
/// A document published under its canonical key keeps that path. The lookup
/// never invents a path: a document that is not on disk has no path yet, and
/// the caller falls back to the name it would create.
///
/// The canonical key is checked first (one stat). Only when the key is
/// unknown does the directory have to be read — a document whose file name
/// drifted is found by the `id:` it declares.
async fn find_existing_projection_file(
    _workspace: &Path,
    dir: &Path,
    id: &str,
    canonical_key: Option<&str>,
) -> anyhow::Result<Option<PathBuf>> {
    if let Some(key) = declared_canonical_key(canonical_key) {
        let candidate = dir.join(crate::artifact::store::canonical_filename(key)?);
        if candidate.exists() {
            return Ok(Some(candidate));
        }
    }
    let Ok(mut read_dir) = tokio::fs::read_dir(dir).await else {
        return Ok(None);
    };
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let Ok(content) = tokio::fs::read_to_string(&path).await else {
            continue;
        };
        let Ok((fm, _body)) = frontmatter::parse_frontmatter(&content) else {
            continue;
        };
        if fm
            .get("id")
            .and_then(|v| v.as_str())
            .is_some_and(|declared| declared.eq_ignore_ascii_case(id))
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
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

/// Like [`render_after_mutation`], but writes the store's body verbatim.
///
/// For handlers whose mutation *appended to the body* — the lifecycle
/// sections. The plain variant is files-first and silently drops them
/// (#478). Requires that the handler synced file → store first, which
/// `sync_before_mutation` does.
pub async fn render_after_mutation_with_body(
    workspace: &Path,
    store: &crate::db::store::LanceStore,
    id: &str,
) -> anyhow::Result<()> {
    if let Some(record) = store.get_record(id).await? {
        let links = store.get_relations(id).await.unwrap_or_default();
        render_projection_record_with_body(workspace, &record, &links).await?;
    }
    Ok(())
}

/// Which facets of the canonical projection the markdown file and the
/// derived row disagree on.
///
/// Markdown is the source of truth (ADR-003), so every facet here is a case
/// where the row has drifted away from the file. The three digests are
/// deliberately separate fields rather than one "changed" boolean: `source`
/// says the canonical bytes moved, `input` says the text that feeds the
/// embedding moved, and `profile` says the embedding space moved. Folding
/// them together is how a superseded vector keeps being served as current.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectionDrift {
    pub source: bool,
    pub input: bool,
    pub profile: bool,
    pub body: bool,
    pub status: bool,
    pub title: bool,
    pub depth: bool,
    pub valid_until: bool,
    pub tags: bool,
    pub outgoing: bool,
    /// Facets that differ but have no writer on this path. Reported, never
    /// silently applied — a facet nobody writes back is a facet that stays
    /// wrong, and hiding that is how it stays wrong for years.
    pub unwritable: Vec<crate::constellation::DiagnosticCode>,
}

impl ProjectionDrift {
    /// Whether the file and the row agree on every comparable facet.
    pub fn is_empty(&self) -> bool {
        !self.source
            && !self.input
            && !self.profile
            && !self.body
            && !self.status
            && !self.title
            && !self.depth
            && !self.valid_until
            && !self.tags
            && !self.outgoing
            && self.unwritable.is_empty()
    }

    /// Whether applying this drift writes anything to the derived row.
    ///
    /// `source` and `input` alone do not: they describe the embedding
    /// binding, and the vector column is cleared by the body write that a
    /// real body change already triggers.
    pub fn needs_store_write(&self) -> bool {
        self.body
            || self.status
            || self.title
            || self.depth
            || self.valid_until
            || self.tags
            || self.outgoing
    }
}

/// Resolve the projection path for a record, plus its workspace-relative
/// form. `None` when the row's kind is not a kind this build knows.
fn projection_path(
    workspace: &Path,
    id: &str,
    kind: &str,
    title: &str,
) -> Option<(PathBuf, String)> {
    let artifact_kind = kind.parse::<ArtifactKind>().ok()?;
    let dir_name = artifact_kind.dir_name();
    let filename = format!("{}-{}.md", id, projection_slug(title));
    Some((
        workspace.join(dir_name).join(&filename),
        format!("{dir_name}/{filename}"),
    ))
}

/// Normalise a body for comparison.
///
/// The renderer writes `<body>\n`, so a row produced by
/// `render_projection*` and the same file read back differ by exactly one
/// trailing newline. Everything else is compared verbatim: the previous
/// `trim()`-based check is what made an emptied body look like "no change"
/// and left a deleted document searchable forever.
fn canonical_body(raw: &str) -> String {
    let unified = if raw.contains('\r') {
        raw.replace("\r\n", "\n")
    } else {
        raw.to_string()
    };
    match unified.strip_suffix('\n') {
        Some(trimmed) => trimmed.to_string(),
        None => unified,
    }
}

/// Sync the canonical markdown projection into the derived row.
///
/// Comparison covers the **full captured bytes and config**, not just the
/// body. Two defects this closes:
///
/// * A metadata-only edit — `status`, `title`, `depth`, `valid_until`,
///   `tags`, `links` — used to be invisible here, so the next mutation
///   re-rendered the file from a row that had never seen it and the edit was
///   gone from both places.
/// * A body replaced by an **empty** body used to be skipped entirely
///   (`!file_trimmed.is_empty()`), so the previous text stayed in the row and
///   kept answering searches for a document whose content was gone.
///
/// Missing frontmatter keys are not nulls: a field the file does not declare
/// is left alone rather than written as empty, and a declared-but-empty body
/// is content.
///
/// Returns `true` when anything was written to the row. Relations are
/// replaced as an exact set, never unioned — a link deleted from the file
/// disappears from the row, while a target that does not resolve locally
/// keeps its edge.
pub async fn sync_file_to_store(
    store: &crate::db::store::LanceStore,
    workspace: &Path,
    record: &crate::db::store::ArtifactRecord,
) -> anyhow::Result<bool> {
    Ok(compare_projection(store, workspace, record)
        .await?
        .needs_store_write())
}

/// Compare the canonical file with the derived row and apply every facet
/// that differs.
///
/// Returns the observed drift either way — including the facets that were
/// detected but not written — so callers that need to explain *why* a row
/// still disagrees with its file can say so.
pub async fn compare_projection(
    store: &crate::db::store::LanceStore,
    workspace: &Path,
    record: &crate::db::store::ArtifactRecord,
) -> anyhow::Result<ProjectionDrift> {
    let mut drift = ProjectionDrift::default();

    let Some((filepath, relative_path)) =
        projection_path(workspace, &record.id, &record.kind, &record.title)
    else {
        return Ok(drift);
    };

    // No file means nothing to sync: the create and delete flows legitimately
    // reach here with no projection on disk.
    let Ok(bytes) = tokio::fs::read(&filepath).await else {
        return Ok(drift);
    };
    let chunk_size = body_prefix_chars(workspace);
    let parsed = match crate::constellation::capture::parse_artifact_file(
        &relative_path,
        std::sync::Arc::<[u8]>::from(bytes),
        chunk_size,
    ) {
        Ok(parsed) => parsed,
        Err(diagnostic) => {
            // Unreadable or invalid markdown is a diagnostic, never a
            // delete and never a silent "nothing changed".
            tracing::warn!("projection sync skipped {}: {diagnostic}", relative_path);
            return Ok(drift);
        }
    };

    // A file whose own identity is not this row's is a different artifact
    // that happens to sit at this path. Overwriting the row with it would
    // republish one artifact's bytes under another's identity.
    if !parsed.display_id.is_empty() && parsed.display_id != record.id {
        tracing::warn!(
            "projection sync skipped {}: declares id {}, row is {}",
            relative_path,
            parsed.display_id,
            record.id
        );
        return Ok(drift);
    }

    let file_body = canonical_body(&parsed.body);
    let db_body = canonical_body(&record.body);
    drift.body = file_body != db_body;

    drift.status = parsed.frontmatter.contains_key("status") && parsed.status != record.status;
    drift.title = parsed.frontmatter.contains_key("title") && parsed.title != record.title;
    drift.depth = match parsed.depth.as_deref() {
        Some(depth) => record.depth.as_str() != depth,
        None => false,
    };

    // The three digests stay separate facts.
    drift.input =
        crate::constellation::jobs::embedding_input_digest(&parsed.title, &parsed.body, chunk_size)
            != crate::constellation::jobs::embedding_input_digest(
                &record.title,
                &record.body,
                chunk_size,
            );
    // `compute_content_hash` returns an owned `String`; compare it as `&str`
    // against the row's optional `&str`.
    drift.source = record.body_hash.as_deref()
        != Some(crate::db::store::compute_content_hash(&parsed.title, &parsed.body).as_str());
    // The derived row carries no profile binding until the publisher writes
    // one, so a profile can never drift here — and the flag stays visible so
    // that the day a profile column exists it is compared rather than
    // forgotten.
    drift.profile = false;

    // `valid_until` can be set but not cleared on this path: the mutator
    // takes a value, not an option. Detected and reported instead of
    // silently ignored.
    match parsed.valid_until.as_deref() {
        Some(valid_until) if !valid_until.is_empty() => {
            drift.valid_until = record.valid_until.as_deref() != Some(valid_until);
        }
        _ => {
            if record.valid_until.is_some() {
                drift
                    .unwritable
                    .push(crate::constellation::DiagnosticCode::UnreadableSource);
            }
        }
    }

    if record.author.as_deref() != parsed.author.as_deref() {
        drift
            .unwritable
            .push(crate::constellation::DiagnosticCode::UnreadableSource);
    }
    if record.parent_epic.as_deref() != parsed.parent_epic.as_deref() {
        drift
            .unwritable
            .push(crate::constellation::DiagnosticCode::UnreadableSource);
    }

    let mut file_tags = parsed.tags.clone();
    file_tags.sort();
    file_tags.dedup();
    let mut db_tags = record.tags.clone();
    db_tags.sort();
    db_tags.dedup();
    drift.tags = file_tags != db_tags;

    // The declared endpoints name documents by their display id; the store
    // holds the edges by their canonical key. The declared side is
    // translated first, so the comparison and the write below speak one
    // language — otherwise every sync would find a difference, delete the
    // stored edge and write it back under the same display id.
    let declared = declared_edges(workspace, &parsed.outgoing).await;
    let stored_edges: std::collections::BTreeSet<crate::constellation::capture::OutgoingEdge> =
        store
            .get_relations(&record.id)
            .await?
            .into_iter()
            .map(|(target, relation)| {
                crate::constellation::capture::OutgoingEdge::new(target, relation)
            })
            .collect();
    drift.outgoing = stored_edges != declared;

    if !drift.needs_store_write() {
        return Ok(drift);
    }

    // Title before body: `update_body` stamps `body_hash` from the row's
    // *current* title, so a title that changed in the same pass has to land
    // first or the hash would be computed over the superseded title.
    if drift.title {
        store
            .update_artifact(&record.id, None, Some(&parsed.title))
            .await?;
    }
    if drift.status {
        store
            .update_artifact(&record.id, Some(&parsed.status), None)
            .await?;
    }
    if drift.body {
        store.update_body(&record.id, &file_body).await?;
    }
    if drift.depth
        && let Some(depth) = parsed.depth.as_deref()
    {
        store.update_depth(&record.id, depth).await?;
    }
    if drift.valid_until
        && let Some(valid_until) = parsed.valid_until.as_deref()
    {
        store.update_valid_until(&record.id, valid_until).await?;
    }
    if drift.tags {
        let removed: Vec<String> = db_tags
            .iter()
            .filter(|t| !file_tags.contains(t))
            .cloned()
            .collect();
        if !removed.is_empty() {
            store.remove_tags(&record.id, &removed).await?;
        }
        let added: Vec<String> = file_tags
            .iter()
            .filter(|t| !db_tags.contains(t))
            .cloned()
            .collect();
        if !added.is_empty() {
            store.add_tags(&record.id, &added).await?;
        }
    }
    if drift.outgoing {
        // Exact replacement: first remove what the file no longer declares,
        // then add what it newly does. Removing first also keeps
        // `add_relation`'s duplicate guard honest for a retargeted edge.
        for edge in stored_edges.difference(&declared) {
            store
                .delete_relation(&record.id, &edge.target, &edge.relation)
                .await?;
        }
        for edge in declared.difference(&stored_edges) {
            if let Err(e) = store
                .add_relation(&record.id, &edge.target, &edge.relation)
                .await
            {
                // A target we cannot represent locally keeps its diagnostic
                // instead of being dropped without a word.
                tracing::warn!(
                    "projection sync could not apply edge {} --{}--> {} from {}: {e}",
                    record.id,
                    edge.relation,
                    edge.target,
                    relative_path
                );
                drift
                    .unwritable
                    .push(crate::constellation::DiagnosticCode::InvalidLinks);
            }
        }
    }

    Ok(drift)
}

/// The declared outgoing edges, with every target named by its canonical key.
///
/// A document declares its link targets by their display id; the store holds
/// the edges by the key of the document they name. The workspace knows which
/// key a display id names even before the row is written, so the translation
/// does not depend on the order the rows were written in.
async fn declared_edges(
    workspace: &Path,
    outgoing: &std::collections::BTreeSet<crate::constellation::capture::OutgoingEdge>,
) -> std::collections::BTreeSet<crate::constellation::capture::OutgoingEdge> {
    if outgoing.is_empty() {
        return outgoing.clone();
    }
    let documents = crate::db::store::workspace_documents(workspace).await;
    let by_display = crate::db::store::document_keys_by_display(&documents);
    outgoing
        .iter()
        .map(|edge| {
            crate::constellation::capture::OutgoingEdge::new(
                crate::db::store::declared_endpoint_key(&edge.target, &by_display),
                edge.relation.clone(),
            )
        })
        .collect()
}

/// Body prefix length that feeds the embedding input digest.
///
/// Read from the same config the rest of the tool uses, so the digest here
/// and the digest in `constellation::capture` cannot disagree.
fn body_prefix_chars(workspace: &Path) -> usize {
    crate::workspace::load_config(workspace)
        .ok()
        .and_then(|config| config.embedding.map(|embedding| embedding.chunk_size))
        .unwrap_or(crate::constellation::jobs::DEFAULT_BODY_PREFIX_CHARS)
}

#[cfg(test)]
mod sync_drift_tests {
    use super::*;
    use crate::db::store::{ArtifactRecord, LanceStore, NewArtifact};
    use crate::workspace::lock::WorkspaceLock;
    use tempfile::TempDir;

    fn art(id: &str, kind: &str, title: &str) -> NewArtifact {
        NewArtifact {
            canonical_key: id.to_string(),
            id: id.to_string(),
            kind: kind.to_string(),
            status: "draft".to_string(),
            title: title.to_string(),
            body: "Original body.".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        }
    }

    /// Legacy-index workspace: dir + LanceStore + the workspace lock every
    /// `MutationContext` requires. The lock is acquired the way production
    /// does (before the store) so a test cannot accidentally rely on an
    /// ordering the real call sites do not have.
    async fn fixture() -> (TempDir, PathBuf, LanceStore, WorkspaceLock) {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = LanceStore::init(&ws).await.unwrap();
        (tmp, ws, store, lock)
    }

    /// Overwrite a projection file with exactly the frontmatter given and
    /// exactly the body given.
    fn write_projection(
        ws: &Path,
        id: &str,
        kind: &str,
        title: &str,
        frontmatter: &str,
        body: &str,
    ) {
        let dir = ws.join(kind.parse::<ArtifactKind>().unwrap().dir_name());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{}-{}.md", id, projection_slug(title)));
        std::fs::write(&path, format!("---\n{frontmatter}---\n\n{body}")).unwrap();
    }

    async fn record(store: &LanceStore, id: &str) -> ArtifactRecord {
        store.get_record(id).await.unwrap().expect("row exists")
    }

    #[tokio::test]
    async fn a_freshly_created_projection_has_nothing_to_sync() {
        let (_tmp, ws, store, lock) = fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art("PRD-001", "prd", "Auth"),
        )
        .await
        .unwrap();

        let drift = compare_projection(&store, &ws, &record(&store, "PRD-001").await)
            .await
            .unwrap();
        assert!(
            !drift.needs_store_write(),
            "an unchanged projection must not churn the row: {drift:?}"
        );
        assert!(
            !sync_file_to_store(&store, &ws, &record(&store, "PRD-001").await)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn a_metadata_only_edit_reaches_the_row() {
        let (_tmp, ws, store, lock) = fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art("PRD-002", "prd", "Auth"),
        )
        .await
        .unwrap();

        // Only the status changes in the file; the body is byte-identical.
        write_projection(
            &ws,
            "PRD-002",
            "prd",
            "Auth",
            "id: PRD-002\nkind: prd\nstatus: active\ntitle: Auth\ndepth: standard\n",
            "Original body.",
        );

        assert!(
            sync_file_to_store(&store, &ws, &record(&store, "PRD-002").await)
                .await
                .unwrap(),
            "a metadata-only edit must not be skipped"
        );
        let after = record(&store, "PRD-002").await;
        assert_eq!(after.status, "active");
        assert_eq!(after.body, "Original body.", "the body was not touched");
    }

    #[tokio::test]
    async fn a_tag_edit_reaches_the_row_as_an_exact_set() {
        let (_tmp, ws, store, lock) = fixture().await;
        let mut artifact = art("PRD-003", "prd", "Auth");
        artifact.tags = vec!["source=code".to_string()];
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &artifact)
            .await
            .unwrap();

        write_projection(
            &ws,
            "PRD-003",
            "prd",
            "Auth",
            "id: PRD-003\nkind: prd\nstatus: draft\ntitle: Auth\ndepth: standard\ntags:\n  - legacy\n",
            "Original body.",
        );
        assert!(
            sync_file_to_store(&store, &ws, &record(&store, "PRD-003").await)
                .await
                .unwrap()
        );
        let mut tags = record(&store, "PRD-003").await.tags;
        tags.sort();
        assert_eq!(tags, vec!["legacy".to_string()]);
    }

    #[tokio::test]
    async fn an_emptied_body_is_a_change_not_a_no_op() {
        let (_tmp, ws, store, lock) = fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art("PRD-004", "prd", "Auth"),
        )
        .await
        .unwrap();
        assert_eq!(record(&store, "PRD-004").await.body, "Original body.");

        write_projection(
            &ws,
            "PRD-004",
            "prd",
            "Auth",
            "id: PRD-004\nkind: prd\nstatus: draft\ntitle: Auth\ndepth: standard\n",
            "",
        );

        assert!(
            sync_file_to_store(&store, &ws, &record(&store, "PRD-004").await)
                .await
                .unwrap(),
            "an emptied body is a content change, not an absence of one"
        );
        assert_eq!(
            record(&store, "PRD-004").await.body,
            "",
            "the previous text must not survive in the row"
        );
    }

    #[tokio::test]
    async fn a_removed_link_disappears_and_the_rest_are_kept() {
        let (_tmp, ws, store, lock) = fixture().await;
        let ctx = MutationContext::new(&ws, &lock, Some(&store));
        create_artifact_with_projection(&ctx, &art("PRD-005", "prd", "Auth"))
            .await
            .unwrap();
        for id in ["RFC-005", "ADR-005", "EVID-005"] {
            create_artifact_with_projection(
                &MutationContext::new(&ws, &lock, Some(&store)),
                &art(
                    id,
                    if id.starts_with("RFC") {
                        "rfc"
                    } else if id.starts_with("ADR") {
                        "adr"
                    } else {
                        "evidence"
                    },
                    id,
                ),
            )
            .await
            .unwrap();
        }
        for id in ["RFC-005", "ADR-005", "EVID-005"] {
            add_link_with_projection(&ctx, "PRD-005", id, "informs")
                .await
                .unwrap();
        }
        assert_eq!(store.get_relations("PRD-005").await.unwrap().len(), 3);

        // Hand-edit the source file: drop ADR-005, keep the other two.
        write_projection(
            &ws,
            "PRD-005",
            "prd",
            "Auth",
            "id: PRD-005\nkind: prd\nstatus: draft\ntitle: Auth\ndepth: standard\nlinks:\n  - target: RFC-005\n    relation: informs\n  - target: EVID-005\n    relation: informs\n",
            "Original body.",
        );

        assert!(
            sync_file_to_store(&store, &ws, &record(&store, "PRD-005").await)
                .await
                .unwrap()
        );
        let mut edges = store.get_relations("PRD-005").await.unwrap();
        edges.sort();
        assert_eq!(
            edges,
            vec![
                ("EVID-005".to_string(), "informs".to_string()),
                ("RFC-005".to_string(), "informs".to_string()),
            ],
            "the outgoing set is replaced exactly: no residue, no union"
        );
    }

    #[tokio::test]
    async fn an_invalid_frontmatter_leaves_the_row_alone() {
        let (_tmp, ws, store, lock) = fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art("PRD-006", "prd", "Auth"),
        )
        .await
        .unwrap();
        let dir = ws.join("prds");
        std::fs::write(dir.join("PRD-006-auth.md"), "---\nid: [unclosed\n").unwrap();

        assert!(
            !sync_file_to_store(&store, &ws, &record(&store, "PRD-006").await)
                .await
                .unwrap(),
            "an unparseable file is a diagnostic, not a write"
        );
        let after = record(&store, "PRD-006").await;
        assert_eq!(after.body, "Original body.");
        assert_eq!(after.status, "draft");
    }

    #[tokio::test]
    async fn a_missing_projection_file_is_a_no_op() {
        let (_tmp, ws, store, lock) = fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art("PRD-007", "prd", "Auth"),
        )
        .await
        .unwrap();
        std::fs::remove_file(ws.join("prds").join("PRD-007-auth.md")).unwrap();

        assert!(
            !sync_file_to_store(&store, &ws, &record(&store, "PRD-007").await)
                .await
                .unwrap()
        );
        assert_eq!(record(&store, "PRD-007").await.body, "Original body.");
    }

    #[test]
    fn canonical_body_strips_only_the_renderers_trailing_newline() {
        assert_eq!(canonical_body("text\n"), "text");
        assert_eq!(canonical_body("text"), "text");
        assert_eq!(canonical_body(""), "");
        assert_eq!(canonical_body("\n"), "");
        assert_eq!(canonical_body("  text  "), "  text  ");
        assert_eq!(canonical_body("text\r\n"), "text");
        assert_eq!(canonical_body("line\n\n"), "line\n");
    }

    #[test]
    fn drift_reports_whether_a_store_write_is_needed() {
        let mut drift = ProjectionDrift::default();
        assert!(!drift.needs_store_write());
        assert!(drift.is_empty());

        drift.input = true;
        drift.source = true;
        assert!(
            !drift.needs_store_write(),
            "an embedding-binding change alone is not a row write"
        );
        assert!(!drift.is_empty());

        drift.body = true;
        assert!(drift.needs_store_write());
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProjectionBodyMode {
    Body,
    CanonicalFile,
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
    body_mode: ProjectionBodyMode,
) -> anyhow::Result<String> {
    // Captured exports carry a complete canonical file, while old bundles
    // carry its body. Do not add another wrapper around the same artifact.
    let embedded =
        if body_mode == ProjectionBodyMode::CanonicalFile && body.trim_start().starts_with("---") {
            frontmatter::parse_frontmatter_borrowed(body)
                .ok()
                .filter(|(fm, _)| {
                    fm.get("id").and_then(|v| v.as_str()) == Some(id)
                        && fm.get("kind").and_then(|v| v.as_str()) == Some(kind)
                })
        } else {
            None
        };
    let supplied_body = body;
    let body = embedded.as_ref().map(|(_, body)| *body).unwrap_or(body);

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

    // Missing values are explicit clears. Omitting their keys would allow an
    // older nested projection to supply the value again on the next read.
    for (key, value) in [
        ("author", author),
        ("parent_epic", parent_epic),
        ("valid_until", valid_until),
    ] {
        fm.insert(
            key.to_string(),
            value
                .map(|value| serde_yaml::Value::String(value.to_string()))
                .unwrap_or(serde_yaml::Value::Null),
        );
    }

    let tag_seq: Vec<serde_yaml::Value> = tags
        .iter()
        .map(|t| serde_yaml::Value::String(t.clone()))
        .collect();
    fm.insert("tags".to_string(), serde_yaml::Value::Sequence(tag_seq));

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

    if let Some((source_fm, _)) = &embedded {
        let same_known_fields = KNOWN_FM_KEYS.iter().all(|key| match *key {
            // Only canonical-file import owns the embedded edge set.
            "links" if links.is_empty() => true,
            "tags" => same_tag_set(source_fm, tags),
            // Captured records expose an omitted depth as the empty string.
            "depth" => {
                source_fm
                    .get("depth")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    == depth
            }
            _ => {
                fm.get(*key).filter(|value| !value.is_null())
                    == source_fm.get(*key).filter(|value| !value.is_null())
            }
        });
        let same_preserved_fields = preserved_fm.is_none_or(|extras| {
            extras.iter().all(|(key, value)| {
                KNOWN_FM_KEYS.contains(&key.as_str()) || source_fm.get(key) == Some(value)
            })
        });
        if same_known_fields && same_preserved_fields {
            return Ok(supplied_body.to_string());
        }

        for (key, value) in source_fm {
            if !KNOWN_FM_KEYS.contains(&key.as_str()) {
                fm.insert(key.clone(), value.clone());
            }
        }
        // This branch exists only for canonical-file import. Normal mutations
        // never interpret a body as a source of edges.
        if links.is_empty()
            && let Some(value) = source_fm.get("links")
        {
            fm.insert("links".to_string(), value.clone());
        }
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

fn same_tag_set(source: &Frontmatter, tags: &[String]) -> bool {
    let mut original = frontmatter::tags_from_frontmatter(source);
    original.sort_unstable();
    original.dedup();
    let mut expected: Vec<&str> = tags.iter().map(String::as_str).collect();
    expected.sort_unstable();
    expected.dedup();
    original.iter().map(String::as_str).eq(expected)
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

/// Publish a new destination without replacing a file an external writer created.
async fn atomic_markdown_create(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "projection has no parent")
    })?;
    let temporary = tempfile::Builder::new()
        .prefix(".projection-")
        .tempfile_in(parent)?;
    tokio::fs::write(temporary.path(), bytes).await?;
    tokio::fs::hard_link(temporary.path(), path).await
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

/// Remove the OLD projection file after a title change — but only if the title
/// actually renamed the file.
///
/// A title edit that slugifies to the *same* filename (a case-only or
/// punctuation-only change, or the identical title re-submitted) must NOT delete
/// anything: the mutation path writes the new file at `slug(new_title)` first,
/// and when `slug(old) == slug(new)` that is the very file we would then remove.
/// `slugify` lowercases and collapses every non-alphanumeric run to `-`, so
/// `"Auth system"`, `"Auth system!"` and `"auth system"` all map to
/// `auth-system` — without this guard, `forgeplan_update --title` on any of
/// those against an existing `auth-system` artifact deletes the markdown source
/// of truth. `forgeplan_get` keeps working (the LanceDB row survives), so the
/// loss is silent until the next `reindex` prunes the now-orphaned row —
/// unrecoverable (ADR-003: markdown is the source of truth).
///
/// The comparison uses `projection_slug` — the same function that builds the
/// on-disk filename from the title — so slug-equality is exactly file-path
/// equality here (both paths share this artifact's id, kind and directory).
///
/// # Errors
/// Same as [`remove_projection_at`]: invalid id/kind, or a filesystem error
/// during removal.
pub async fn remove_stale_projection_after_rename(
    workspace: &Path,
    id: &str,
    kind: &str,
    old_title: &str,
    new_title: &str,
) -> anyhow::Result<bool> {
    if projection_slug(old_title) == projection_slug(new_title) {
        // Same filename — the "old" file IS the file we just wrote. No-op.
        return Ok(false);
    }
    remove_projection_at(workspace, id, kind, old_title).await
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
/// With `ctx.legacy_sink == None` (constellation-managed workspace) the
/// markdown write is the whole mutation and no database is opened; the
/// derived row is published by the reconcile pass. The canonical bytes are
/// identical either way.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `artifact.id` fails
///   `validate_artifact_id` (path-traversal payload, empty, etc.).
/// - [`MutationError::InvalidKind`] if `artifact.kind` does not parse as
///   an `ArtifactKind`.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   the underlying `LanceStore::create_artifact` call (only when a sink is
///   present) or the markdown-projection write fails. Categorisation via
///   [`MutationError::from_store_err`] — fatal for schema corruption /
///   ENOENT, transient for EACCES / lock contention.
pub async fn create_artifact_with_projection(
    ctx: &MutationContext<'_>,
    artifact: &crate::db::store::NewArtifact,
) -> MutationResult<PathBuf> {
    create_artifact_projection(ctx, artifact, ProjectionBodyMode::Body).await
}

/// Restore a bundle artifact whose body may be a complete canonical file.
///
/// Old body-only payloads retain the normal creation contract. A complete
/// source belonging to this identity is restored without another wrapper.
pub async fn import_artifact_with_projection(
    ctx: &MutationContext<'_>,
    artifact: &crate::db::store::NewArtifact,
) -> MutationResult<PathBuf> {
    create_artifact_projection(ctx, artifact, ProjectionBodyMode::CanonicalFile).await
}

/// The row a document's creation publishes.
///
/// The row is keyed by the document's **real** canonical key — the slug its
/// frontmatter declares, or the display id when it declares none (ADR-030).
/// A row keyed by the display id instead would put two rows in the
/// workspace for one document, and that document would then read as an
/// ambiguous alias.
fn creation_row(artifact: &crate::db::store::NewArtifact) -> crate::db::store::NewArtifact {
    let canonical_key = crate::artifact::canonical_key::canonical_key_from_document(
        &artifact.id,
        &artifact.body,
    );
    crate::db::store::NewArtifact {
        id: artifact.id.clone(),
        canonical_key,
        kind: artifact.kind.clone(),
        status: artifact.status.clone(),
        title: artifact.title.clone(),
        body: artifact.body.clone(),
        depth: artifact.depth.clone(),
        author: artifact.author.clone(),
        parent_epic: artifact.parent_epic.clone(),
        valid_until: artifact.valid_until.clone(),
        tags: artifact.tags.clone(),
    }
}

/// Publish a newly created document under its canonical key.
///
/// SPEC-009 "New file naming": a newly created document is written to
/// `<canonical_key>.md`. Existing files keep their paths — this path is
/// for a document that is being created, never for one that already exists.
///
/// Publication never clobbers: an existing path aborts the creation, the
/// existing document is not overwritten, and no second key is minted after
/// relations have already been published against the first.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `artifact.id` fails
///   `validate_artifact_id`.
/// - [`MutationError::InvalidKind`] if `artifact.kind` does not parse as
///   an `ArtifactKind`.
/// - [`MutationError::StoreFatal`] if the canonical key is not a usable
///   file name, or if the target path already exists.
pub async fn create_artifact_with_canonical_key(
    ctx: &MutationContext<'_>,
    artifact: &crate::db::store::NewArtifact,
) -> MutationResult<PathBuf> {
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
    crate::db::store::validate_artifact_id(&artifact.id)
        .map_err(|_| MutationError::InvalidId(artifact.id.clone()))?;
    let artifact_kind: ArtifactKind = artifact
        .kind
        .parse()
        .map_err(
            |e: crate::error::ForgeplanError| MutationError::InvalidKind {
                id: artifact.id.clone(),
                kind: artifact.kind.clone(),
                reason: e.to_string(),
            },
        )?;
    let filename = crate::artifact::store::canonical_filename(&artifact.canonical_key)
        .map_err(MutationError::StoreFatal)?;
    let filepath = workspace.join(artifact_kind.dir_name()).join(&filename);

    // Publication never clobbers. The check is a courtesy — the write
    // itself refuses to replace an existing file.
    if tokio::fs::metadata(&filepath).await.is_ok() {
        return Err(MutationError::StoreFatal(anyhow::anyhow!(
            "refusing to publish canonical key '{}': {} already exists",
            artifact.canonical_key,
            filepath.display()
        )));
    }

    // The document declares the key it is filed under — the file, the row
    // and the file name then name one identity.
    let body = crate::artifact::frontmatter::ensure_canonical_key(
        &artifact.body,
        &artifact.canonical_key,
    )
    .map_err(MutationError::StoreFatal)?;

    let path = render_projection_inner(
        workspace,
        &artifact.id,
        &artifact.kind,
        &artifact.title,
        &artifact.status,
        &artifact.depth,
        artifact.author.as_deref(),
        artifact.parent_epic.as_deref(),
        artifact.valid_until.as_deref(),
        &body,
        &[],
        &artifact.tags,
        true,
        ProjectionBodyMode::Body,
        Some(&artifact.canonical_key),
        Some(filepath),
        true,
    )
    .await?;

    // Then the derived index — only when this workspace still has a legacy
    // index. On a constellation-managed workspace the markdown write above
    // is the whole mutation.
    if let Some(store) = store {
        // If this fails, the file remains and reindex recovers.
        let mut row = creation_row(artifact);
        row.body = body;
        store.create_artifact(&row).await?;
    }
    Ok(path)
}

async fn create_artifact_projection(
    ctx: &MutationContext<'_>,
    artifact: &crate::db::store::NewArtifact,
    body_mode: ProjectionBodyMode,
) -> MutationResult<PathBuf> {
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
    // Audit 2026-05-01 #1 (security CRITICAL): validate id BEFORE composing
    // it into a filesystem path. Without this, a JSON import with
    // `"id": "../../etc/evil"` would write outside the workspace via
    // `format!("{id}-{slug}.md")` resolved against `workspace/<kind>/`.
    crate::db::store::validate_artifact_id(&artifact.id)
        .map_err(|_| MutationError::InvalidId(artifact.id.clone()))?;
    let artifact_kind: ArtifactKind = artifact
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
    // The legacy creation path writes the name it has always written; the
    // canonical-key path is the one that publishes under `<key>.md`.
    let legacy_path = workspace
        .join(artifact_kind.dir_name())
        .join(format!(
            "{}-{}.md",
            artifact.id,
            projection_slug(&artifact.title)
        ));
    let path = render_projection_inner(
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
        &artifact.tags,
        true,
        body_mode,
        None,
        Some(legacy_path),
        false,
    )
    .await?;

    // 2. Then derived index — only when this workspace still HAS a legacy
    //    index. On a constellation-managed workspace the markdown write
    //    above is the whole mutation and `forgeplan index` publishes the
    //    derived row. There is no store to open here, and opening
    //    `artifacts.lance` on a migrated workspace would resurrect a
    //    retired table that shadows the real index.
    if let Some(store) = store {
        // If this fails, file remains and reindex recovers.
        store.create_artifact(&creation_row(artifact)).await?;
    }
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
/// With `ctx.legacy_sink == None` there is no record to read the exact path
/// from, so the file-first resolver (`resolve_artifact_path`) locates the
/// canonical file instead — the same resolver the migrated CLI paths use.
/// The missing-artifact case stays an idempotent no-op on both layouts.
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
    let workspace = ctx.workspace;
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
    //
    // Both branches below keep that policy: with no legacy sink the
    // idempotence test is "no markdown file resolves for this id", which
    // is the source-of-truth equivalent of "no row exists".
    let Some(store) = ctx.legacy_sink else {
        // Constellation-managed workspace: there is no row to consult and
        // no row to cascade. The canonical mutation is the file removal,
        // resolved by the same file-first resolver the no-store CLI paths
        // use (`resolve_artifact_path`), so slug / display-id / literal-id
        // input all bind the same file the index would have.
        let path = crate::artifact::store::resolve_artifact_path(workspace, id).await;
        match path {
            Some(path) => {
                tokio::fs::remove_file(&path)
                    .await
                    .map_err(anyhow::Error::from)?;
            }
            // Idempotent no-op — mirrors the missing-row policy above.
            None => return Ok(()),
        }
        return Ok(());
    };

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
/// - [`MutationError::RowNotFound`] with no legacy sink, when no markdown
///   file resolves for `id` — the canonical mutation cannot be performed,
///   so it is reported rather than silently skipped.
///
/// `(None, None)` short-circuits with `Ok(())` — no DB round-trip.
pub async fn update_metadata_with_projection(
    ctx: &MutationContext<'_>,
    id: &str,
    status: Option<&str>,
    title: Option<&str>,
) -> MutationResult<()> {
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
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
    let Some(store) = store else {
        // Constellation-managed workspace: edit the canonical frontmatter
        // in place (same `EmptyField` / `(None, None)` gates as the
        // store path, applied above) and let reconcile publish the derived
        // row. `update_metadata_file_first` renames to the canonical slug
        // on a title change and is a no-op write when nothing changed.
        let path = crate::artifact::store::resolve_artifact_path(workspace, id)
            .await
            .ok_or_else(|| MutationError::RowNotFound { id: id.to_string() })?;
        update_metadata_file_first(&path, status, title, None).await?;
        return Ok(());
    };
    if title.is_some() {
        let record = store
            .get_record(id)
            .await?
            .ok_or_else(|| MutationError::RowNotFound { id: id.to_string() })?;
        let kind: ArtifactKind =
            record
                .kind
                .parse()
                .map_err(|error| MutationError::InvalidKind {
                    id: id.to_string(),
                    kind: record.kind.clone(),
                    reason: format!("{error}"),
                })?;
        let old_path = workspace
            .join(kind.dir_name())
            .join(format!("{id}-{}.md", projection_slug(&record.title)));
        sync_before_mutation(workspace, store, id).await?;
        update_metadata_file_first(&old_path, status, title, None).await?;
    } else {
        sync_before_mutation(workspace, store, id).await?;
    }
    store.update_artifact(id, status, title).await?;
    render_after_mutation(workspace, store, id).await?;
    Ok(())
}

/// Re-attach body-frontmatter fields from `old_body` that `new_body` does not
/// carry and that the projection does not regenerate.
///
/// The body has its own frontmatter block (PRD-073 layout: synthetic
/// projection block on top, canonical body below). A body replacement is
/// therefore a *field deletion* for everything in that block unless it is
/// carried across — which is why a mature workspace ends up with no artifact
/// carrying a slug at all: `forgeplan new` writes one, and the very next
/// `update --body` that CLAUDE.md prescribes silently drops it.
///
/// ## Why the rule is `KNOWN_FM_KEYS`, not a hand-listed triple
///
/// This carried exactly `["slug", "predicted_number", "assigned_number"]` —
/// the three fields whose loss someone noticed. The argument that justified
/// carrying those three applies verbatim to every other field in the block,
/// so the list was a fix for three instances of a class, and everything
/// outside it kept being deleted in silence.
///
/// Measured on a real workspace: a no-op round-trip of RFC-022 (read the body
/// back with `forgeplan get --json`, hand it straight to `update --body`
/// unchanged) dropped eight lines. Five of them — `author`, `depth`, `id`,
/// `status`, `title` — are in [`KNOWN_FM_KEYS`] and are legitimately
/// regenerated into the projection block above, so their in-body copies are
/// redundant. Three were pure loss: `created`, `updated`, `prd`.
///
/// So the rule is the one the neighbouring [`filter_preserved`] already
/// applies to the projection block's own unknown keys (PRD-057 FR-009):
/// **anything the record does not own survives**. Deriving it from
/// `KNOWN_FM_KEYS` rather than from a second authored list means a field
/// added to the record is automatically excluded, and a field added to the
/// body is automatically preserved — neither needs anyone to remember this
/// function exists. Carrying a `KNOWN_FM_KEYS` field would be worse than
/// dropping it: `update_body_with_projection` parses `status` back out of the
/// new body to sync LanceDB, so a stale in-body copy could resurrect a
/// superseded status.
///
/// Consequence, stated rather than discovered later: a caller cannot delete a
/// body-frontmatter field by omitting it. That is deliberate — silent data
/// loss is the failure this function exists to prevent, and it is the more
/// expensive of the two. Removing a field means writing the block without it
/// AND having the record not own it; for the record-owned ones there are
/// dedicated mutators (`update --status`, `--title`, `--depth`).
fn carry_identity_forward(old_body: &str, new_body: &str) -> String {
    let Ok((old_fm, _)) = frontmatter::parse_frontmatter(old_body) else {
        return new_body.to_string();
    };
    let carried: Vec<(&str, serde_yaml::Value)> = old_fm
        .iter()
        .filter(|(k, _)| !KNOWN_FM_KEYS.contains(&k.as_str()))
        .map(|(k, v)| (k.as_str(), v.clone()))
        .collect();
    if carried.is_empty() {
        return new_body.to_string();
    }

    // Start from whatever frontmatter the replacement itself declares so a
    // caller round-tripping a full document keeps their own values.
    let (mut fm, prose) = match frontmatter::parse_frontmatter(new_body) {
        Ok((fm, prose)) => (fm, prose),
        Err(_) => (Frontmatter::new(), new_body.to_string()),
    };
    let mut changed = false;
    for (k, v) in carried {
        if !fm.contains_key(k) {
            fm.insert(k.to_string(), v);
            changed = true;
        }
    }
    if !changed {
        return new_body.to_string();
    }
    frontmatter::render_frontmatter(&fm, &prose).unwrap_or_else(|_| new_body.to_string())
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
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    let Some(store) = store else {
        // Constellation-managed workspace: the markdown body IS the row.
        // `update_body_file_first` carries the same invariants as the
        // store path below — `carry_identity_forward` on the replacement,
        // the new body's own `status:` propagating into the projection
        // block (audit-r7 F1), atomic write, no-op when unchanged — and
        // resolves its own path, so a title rename from an earlier
        // `update_metadata_with_projection` in the same command is
        // followed correctly.
        let path = crate::artifact::store::resolve_artifact_path(workspace, id)
            .await
            .ok_or_else(|| MutationError::RowNotFound { id: id.to_string() })?;
        update_body_file_first(&path, body).await?;
        return Ok(());
    };
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

    // The canonical key is immutable through an ordinary update. A body
    // that declares a different key — in its own frontmatter or in a
    // nested one — is refused instead of written.
    let current_key = if record.canonical_key.trim().is_empty() {
        crate::artifact::canonical_key::canonical_key_from_document(&record.id, &record.body)
    } else {
        record.canonical_key.clone()
    };
    frontmatter::refuse_canonical_key_change(&current_key, body)
        .map_err(MutationError::StoreFatal)?;

    // PROB-060 / ADR-012: preserve the body-frontmatter fields the replacement
    // would otherwise delete — every field the record does not own, not just
    // the identity triple this originally carried. Must happen before
    // `derived_status` is parsed and before either write, so file and index
    // agree on one body.
    let carried = carry_identity_forward(&record.body, body);
    let body: &str = &carried;

    // Audit-r7 F1 (CRITICAL): parse the new body's frontmatter and sync
    // the `status` column in LanceDB if it changed. Without this, callers
    // who mutate body+status atomically (e.g. `apply_transition_to_body`
    // for hypothesis lifecycle) would see the file frontmatter advance
    // while consumers reading `record.status` from LanceDB still see the
    // stale value — silently breaking health/anomalies/release_notes/
    // forgeplan_get hints. ADR-003 demands LanceDB reflect the markdown
    // source of truth; without this hook, body-only writes diverge.
    //
    // Title sync is intentionally NOT included here: title changes are
    // structural (touch slug, file path, links) and need dedicated path
    // through `update_title_with_projection`. Only `status` is in scope.
    let derived_status = crate::artifact::frontmatter::parse_frontmatter(body)
        .ok()
        .and_then(|(fm, _)| {
            fm.get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

    // 1. File first — write the user's body to the markdown projection. This
    //    establishes the source of truth before the derived index is touched.
    //    Use the *new* status if the frontmatter carries one, so the rendered
    //    file frontmatter matches the projection row.
    let effective_status = derived_status.as_deref().unwrap_or(&record.status);
    render_projection_with_body(
        workspace,
        &record.id,
        &record.kind,
        &record.title,
        effective_status,
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

    // 3. Status sync (audit-r7 F1): if the new frontmatter declares a
    //    `status` that differs from the LanceDB row, propagate it.
    //    update_artifact() handles `updated_at` bump.
    if let Some(new_status) = derived_status.as_deref()
        && new_status != record.status
    {
        store
            .update_artifact(id, Some(new_status), None)
            .await
            .map_err(MutationError::from)?;
    }
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
/// - [`MutationError::RowNotFound`] with no legacy sink and no resolvable
///   markdown file for `id`.
pub async fn update_depth_with_projection(
    ctx: &MutationContext<'_>,
    id: &str,
    depth: &str,
) -> MutationResult<()> {
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    let Some(store) = store else {
        // Constellation-managed workspace: `depth:` is ordinary canonical
        // frontmatter. Same helper as the metadata path — it validates a
        // blank depth and no-ops an unchanged write.
        let path = crate::artifact::store::resolve_artifact_path(workspace, id)
            .await
            .ok_or_else(|| MutationError::RowNotFound { id: id.to_string() })?;
        update_metadata_file_first(&path, None, None, Some(depth)).await?;
        return Ok(());
    };
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
/// - [`MutationError::RowNotFound`] with no legacy sink and no resolvable
///   markdown file for `id`.
pub async fn add_tags_with_projection(
    ctx: &MutationContext<'_>,
    id: &str,
    tags: &[String],
) -> MutationResult<()> {
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    let Some(store) = store else {
        let path = crate::artifact::store::resolve_artifact_path(workspace, id)
            .await
            .ok_or_else(|| MutationError::RowNotFound { id: id.to_string() })?;
        update_tags_file_first(&path, tags, &[]).await?;
        return Ok(());
    };
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
/// - [`MutationError::RowNotFound`] with no legacy sink and no resolvable
///   markdown file for `id`.
pub async fn remove_tags_with_projection(
    ctx: &MutationContext<'_>,
    id: &str,
    tags: &[String],
) -> MutationResult<()> {
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    let Some(store) = store else {
        let path = crate::artifact::store::resolve_artifact_path(workspace, id)
            .await
            .ok_or_else(|| MutationError::RowNotFound { id: id.to_string() })?;
        update_tags_file_first(&path, &[], tags).await?;
        return Ok(());
    };
    sync_before_mutation(workspace, store, id).await?;
    store.remove_tags(id, tags).await?;
    render_after_mutation(workspace, store, id).await?;
    Ok(())
}

/// Add a typed relation and render its source's outgoing `links` block.
///
/// Pre-syncing both participants corrects derived-index drift from canonical
/// files. The target's source bytes must not be rendered from its cache: an
/// incoming edge does not change its outgoing links or authorize clearing
/// source-only metadata that the legacy row cannot represent.
///
/// PRD-073 FR-001 / FR-005 helper. Used by `link` / `reason` (auto-linking).
///
/// Only the source-side pre-sync and relation write are fatal. Target-side
/// pre-sync and source post-render are best-effort so a non-local target or
/// transient rendering error does not strand an already indexed relation.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `source` fails `validate_artifact_id`
///   or `target` fails `validate_link_target` (plain id or
///   `KEY@qualifier`).
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   source-side pre-sync or `add_relation` fails. Target-side pre-sync
///   and post-render failures are logged via `tracing::warn!` and
///   swallowed (best-effort) so a missing-target file does not strand the
///   relation that already landed in LanceDB.
/// - [`MutationError::RowNotFound`] with no legacy sink and no resolvable
///   source file.
pub async fn add_link_with_projection(
    ctx: &MutationContext<'_>,
    source: &str,
    target: &str,
    relation: &str,
) -> MutationResult<()> {
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
    crate::db::store::validate_artifact_id(source)
        .map_err(|_| MutationError::InvalidId(source.to_string()))?;
    // Targets may be qualified cross-source refs (`KEY@qualifier`) — the
    // edge is stored verbatim in the source file's `links:` block and the
    // relations table; nothing foreign is opened or rendered.
    crate::db::store::validate_link_target(target)
        .map_err(|_| MutationError::InvalidId(target.to_string()))?;
    let Some(store) = store else {
        // Constellation-managed workspace: the edge lands in the source
        // file's `links:` block, which is the canonical form reconcile
        // reads. `add_link_verbatim` applies the same guards the store
        // path's `add_relation` does — `validate_link_target`, the
        // self-link refusal, and the exact-duplicate refusal — and stores
        // the target verbatim, so a qualified `KEY@qualifier` target
        // survives intact. Only the source file is touched on either layout.
        let path = crate::artifact::store::resolve_artifact_path(workspace, source)
            .await
            .ok_or_else(|| MutationError::RowNotFound {
                id: source.to_string(),
            })?;
        crate::link::add_link_verbatim(&path, target, relation).await?;
        return Ok(());
    };
    sync_before_mutation(workspace, store, source).await?;
    if let Err(e) = sync_before_mutation(workspace, store, target).await {
        tracing::warn!("add_link: pre-sync target {target} failed (continuing): {e}");
    }

    store.add_relation(source, target, relation).await?;

    if let Err(e) = render_after_mutation(workspace, store, source).await {
        tracing::warn!("add_link: post-render source {source} failed (continuing): {e}");
    }
    Ok(())
}

/// Remove a typed relation and update only its source's outgoing links.
///
/// Audit fix: same warn-and-continue policy as `add_link_with_projection`.
/// Source pre-sync and the relation delete are fatal; target pre-sync and
/// source render are best-effort.
///
/// PRD-073 FR-005 helper. Used by `unlink`.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `source` fails `validate_artifact_id`
///   or `target` fails `validate_link_target` (plain id or
///   `KEY@qualifier`).
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   source-side pre-sync or `delete_relation` fails. Target-side
///   pre-sync and post-render failures are best-effort (warn-and-continue),
///   matching `add_link_with_projection`.
/// - [`MutationError::RowNotFound`] with no legacy sink and no resolvable
///   source file.
pub async fn delete_link_with_projection(
    ctx: &MutationContext<'_>,
    source: &str,
    target: &str,
    relation: &str,
) -> MutationResult<()> {
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
    crate::db::store::validate_artifact_id(source)
        .map_err(|_| MutationError::InvalidId(source.to_string()))?;
    // `unlink` must accept what `link` can store — qualified targets too.
    crate::db::store::validate_link_target(target)
        .map_err(|_| MutationError::InvalidId(target.to_string()))?;
    let Some(store) = store else {
        // Constellation-managed workspace: drop the entry from the source
        // file's `links:` block. `remove_link` matches the target
        // case-insensitively and the relation exactly — the same rule the
        // store path's `delete_relation` applies — and returns how many
        // entries it removed. A zero count is success here, matching
        // `store.delete_relation`'s documented "nonexistent is silent"
        // behaviour; the caller's own "relation not found" pre-check is
        // what produces the user-facing refusal.
        let path = crate::artifact::store::resolve_artifact_path(workspace, source)
            .await
            .ok_or_else(|| MutationError::RowNotFound {
                id: source.to_string(),
            })?;
        crate::link::remove_link(&path, target, relation).await?;
        return Ok(());
    };
    sync_before_mutation(workspace, store, source).await?;
    if let Err(e) = sync_before_mutation(workspace, store, target).await {
        tracing::warn!("delete_link: pre-sync target {target} failed (continuing): {e}");
    }

    store.delete_relation(source, target, relation).await?;

    if let Err(e) = render_after_mutation(workspace, store, source).await {
        tracing::warn!("delete_link: post-render source {source} failed (continuing): {e}");
    }
    Ok(())
}

/// Outcome of `replace_link_with_projection` — describes whether the call
/// created a new edge, replaced an existing edge's relation, or made no
/// change because the requested edge already existed.
///
/// Issue #286: callers (CLI / MCP) use this to render an accurate
/// human-readable summary AND to populate the `_next_action` hint chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkUpsertOutcome {
    /// No edge between `(source, target)` existed; a new one was added.
    Created,
    /// An edge between `(source, target)` existed with a different
    /// relation. Audit-r2 ordering: the new edge was added first, then
    /// the old edge deleted. Partial-failure safety — if `add` fails,
    /// the OLD edge survives intact and the caller retries cleanly.
    Replaced { old_relation: String },
    /// An edge between `(source, target)` already existed with the same
    /// relation. No-op.
    Unchanged,
}

/// Idempotent link operation on the `(source, target)` pair.
///
/// Correct an edge's relation without changing its target's canonical file.
/// Exact duplicates stay unchanged; different relations follow the upsert
/// policy below rather than ordinary additive-link semantics.
///
/// Semantics: find any existing edge `(source, target, *)`. If found
/// with the requested relation → return `Unchanged`. If found with a
/// different relation → delete the old edge and add the new one →
/// return `Replaced { old_relation }`. Otherwise add the edge and
/// return `Created`.
///
/// Only the FIRST existing edge between `(source, target)` is considered
/// when deciding the outcome. Forgeplan's graph model permits at most one
/// edge per `(source, target, relation)` triple but does not prohibit
/// multiple `(source, target, *)` rows with different relations — those
/// only arise from explicit double-link calls. This helper does NOT
/// collapse a pre-existing many-edge state; it only ensures that AFTER
/// this call, the requested edge exists exactly once.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `source` fails `validate_artifact_id`
///   or `target` fails `validate_link_target` (plain id or
///   `KEY@qualifier`).
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`]
///   if source pre-sync, relation read, delete, or add fails. Target
///   pre-sync and source rendering are best-effort, matching `add_link_with_projection`.
pub async fn replace_link_with_projection(
    ctx: &MutationContext<'_>,
    source: &str,
    target: &str,
    relation: &str,
) -> MutationResult<LinkUpsertOutcome> {
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
    crate::db::store::validate_artifact_id(source)
        .map_err(|_| MutationError::InvalidId(source.to_string()))?;
    crate::db::store::validate_link_target(target)
        .map_err(|_| MutationError::InvalidId(target.to_string()))?;
    let Some(store) = store else {
        // Constellation-managed workspace: the upsert is a rewrite of the
        // source file's `links:` entry. `link::replace_link` returns the
        // same `LinkUpsertOutcome` taxonomy the store path computes from
        // `get_relations`, with the same "only the FIRST matching edge
        // decides" rule and the same no-write-on-`Unchanged` behaviour, so
        // a CLI/MCP caller's printed outcome is identical on both layouts.
        let path = crate::artifact::store::resolve_artifact_path(workspace, source)
            .await
            .ok_or_else(|| MutationError::RowNotFound {
                id: source.to_string(),
            })?;
        return Ok(crate::link::replace_link(&path, target, relation).await?);
    };
    sync_before_mutation(workspace, store, source).await?;
    if let Err(e) = sync_before_mutation(workspace, store, target).await {
        tracing::warn!("replace_link: pre-sync target {target} failed (continuing): {e}");
    }

    // Read existing edges from `source`. We compare case-insensitively on
    // the target id because Forgeplan stores ids in their canonical form
    // (uppercase prefix for display ids, lowercase for slugs) but callers
    // may pass either form.
    let existing = store
        .get_relations(source)
        .await
        .map_err(MutationError::from_store_err)?;

    let existing_same_target: Vec<(String, String)> = existing
        .into_iter()
        .filter(|(t, _)| t.eq_ignore_ascii_case(target))
        .collect();

    let outcome = if let Some((_, r)) = existing_same_target.iter().find(|(_, r)| r == relation) {
        // Already present with the same relation — no-op.
        let _ = r;
        LinkUpsertOutcome::Unchanged
    } else if let Some((_, old_rel)) = existing_same_target.first() {
        // Present with a different relation — replace.
        //
        // SEC HIGH-1 closure (audit-r2): add THEN delete, not the reverse.
        // The two store operations are not transactional at the LanceDB
        // layer; an `add_relation` failure after `delete_relation`
        // succeeded would leave the source artifact silently edgeless to
        // the target (CL penalty cascade through the rest of the graph,
        // no recovery signal). Reversed ordering: if `add_relation`
        // fails, the OLD edge survives intact and the caller gets a
        // clean error to retry; if it succeeds and then `delete_relation`
        // fails (rarer — delete is more reliable than add in LanceDB),
        // both edges exist temporarily but the workspace is still
        // navigable, and `forgeplan_anomalies` will flag the duplicate
        // for cleanup. `add_relation` returns `Err` on duplicate-edge
        // attempts (db/store.rs:706-713) but the existing edge here has
        // a DIFFERENT relation, so the dedup check passes.
        store
            .add_relation(source, target, relation)
            .await
            .map_err(MutationError::from_store_err)?;
        store
            .delete_relation(source, target, old_rel)
            .await
            .map_err(MutationError::from_store_err)?;
        LinkUpsertOutcome::Replaced {
            old_relation: old_rel.clone(),
        }
    } else {
        // No prior edge — plain add.
        store
            .add_relation(source, target, relation)
            .await
            .map_err(MutationError::from_store_err)?;
        LinkUpsertOutcome::Created
    };

    if let Err(e) = render_after_mutation(workspace, store, source).await {
        tracing::warn!("replace_link: post-render source {source} failed (continuing): {e}");
    }
    Ok(outcome)
}

// =============================================================================
// File-first helpers for constellation-migrated workspaces (no LanceStore)
// -----------------------------------------------------------------------------
// On a workspace whose index moved to the constellation protocol the legacy
// `artifacts.lance` is retired and there is no store to sync through — the
// markdown edit IS the whole mutation, and the reconcile pass
// (`maintain_index` / `forgeplan index`) owns the derived row afterwards.
// These are the no-store siblings of `update_*_with_projection`: they edit
// the projection frontmatter / body in place with the same invariants
// (files-first, carry-forward, atomic write) minus the store round-trip.
// =============================================================================

/// Update `status:` / `title:` / `depth:` inside an artifact's projection
/// frontmatter in place — the no-store sibling of
/// [`update_metadata_with_projection`] + [`update_depth_with_projection`].
/// Every other frontmatter key and the body are preserved byte-for-byte.
///
/// A `--title` change also renames the file to the canonical
/// `{id}-{slug}.md` (via [`projection_slug`]) — the convention
/// `resolve_artifact_path` and reconcile both key on. The id comes from the
/// file's own `id:` field; a title that slugifies to the same filename
/// writes in place (the same-slug rule
/// [`remove_stale_projection_after_rename`] documents).
///
/// Returns the file's path after the mutation — which differs from the
/// input exactly when the title renamed it, so a following
/// [`update_body_file_first`] in the same command hits the new path.
/// No-change input leaves the file untouched (mtime stays put, reconcile
/// sees an unmoved source).
///
/// # Errors
///
/// - blank `status` / `title` / `depth` value (mirrors the `EmptyField`
///   guards on `update_metadata_with_projection`)
/// - unparseable frontmatter, or a missing `id:` field on a rename
/// - I/O failures on read, atomic write, or the stale-file removal —
///   leaving a second `{id}-*.md` behind would double-publish the id, so
///   the removal is fatal rather than best-effort
pub async fn update_metadata_file_first(
    artifact_path: &Path,
    status: Option<&str>,
    title: Option<&str>,
    depth: Option<&str>,
) -> anyhow::Result<PathBuf> {
    for (field, value) in [("status", status), ("title", title), ("depth", depth)] {
        if let Some(v) = value
            && v.trim().is_empty()
        {
            anyhow::bail!("update: --{field} cannot be blank");
        }
    }

    let content = tokio::fs::read_to_string(artifact_path).await?;
    let (mut fm, body) = frontmatter::parse_frontmatter(&content)?;
    if let Some(s) = status {
        fm.insert(
            "status".to_string(),
            serde_yaml::Value::String(s.to_string()),
        );
    }
    if let Some(t) = title {
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String(t.to_string()),
        );
    }
    if let Some(d) = depth {
        fm.insert(
            "depth".to_string(),
            serde_yaml::Value::String(d.to_string()),
        );
    }
    let mut rendered = frontmatter::render_frontmatter(&fm, &body)?;
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    // Title change → the canonical filename follows the NEW slug — unless
    // the document is published under its canonical key. A document filed
    // under its key keeps that path: a title change renames nothing and
    // changes no key.
    let mut final_path = artifact_path.to_path_buf();
    if let Some(t) = title {
        let id = fm.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
            anyhow::anyhow!(
                "cannot rename {}: frontmatter has no `id:` field",
                artifact_path.display()
            )
        })?;
        let canonical_key =
            crate::artifact::canonical_key::canonical_key_from_document(id, &content);
        let published_under_key = artifact_path
            .file_stem()
            .is_some_and(|stem| stem.to_string_lossy() == canonical_key);
        if !published_under_key {
            let candidate = artifact_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(format!("{id}-{}.md", projection_slug(t)));
            if candidate != artifact_path {
                final_path = candidate;
            }
        }
    }

    if final_path == artifact_path && rendered == content {
        return Ok(artifact_path.to_path_buf());
    }
    if final_path == artifact_path {
        atomic_markdown_write(&final_path, rendered.as_bytes()).await?;
    } else {
        atomic_markdown_create(&final_path, rendered.as_bytes()).await?;
        anyhow::ensure!(
            tokio::fs::read_to_string(artifact_path).await? == content,
            "artifact changed while renaming; both files are preserved"
        );
        tokio::fs::remove_file(artifact_path).await?;
    }
    Ok(final_path)
}

/// Replace an artifact's body — the no-store sibling of
/// [`update_body_with_projection`]. "Body" is everything below the
/// projection frontmatter block: the canonical body-frontmatter plus prose.
///
/// Same invariants as the store path:
/// - [`carry_identity_forward`] — body-frontmatter fields the projection
///   does not own (`slug`, `assigned_number`, `created`, …) survive a body
///   replacement that omits them.
/// - status sync — a `status:` declared inside the NEW body's own
///   frontmatter propagates into the projection block (audit-r7 F1 rule).
/// - the write is atomic and skipped when nothing would change.
pub async fn update_body_file_first(artifact_path: &Path, new_body: &str) -> anyhow::Result<()> {
    let content = tokio::fs::read_to_string(artifact_path).await?;
    let (mut fm, body) = frontmatter::parse_frontmatter(&content)?;

    // The canonical key is immutable through an ordinary update. A body
    // that declares a different key — in its own frontmatter or in a
    // nested one — is refused instead of written.
    let current_key = {
        let id = fm.get("id").and_then(|v| v.as_str()).unwrap_or_default();
        crate::artifact::canonical_key::canonical_key_from_document(id, &content)
    };
    frontmatter::refuse_canonical_key_change(&current_key, new_body)?;

    let carried = carry_identity_forward(&body, new_body);
    if let Some(derived_status) =
        frontmatter::parse_frontmatter(&carried)
            .ok()
            .and_then(|(bfm, _)| {
                bfm.get("status")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
    {
        fm.insert(
            "status".to_string(),
            serde_yaml::Value::String(derived_status),
        );
    }

    let mut rendered = frontmatter::render_frontmatter(&fm, &carried)?;
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    if rendered == content {
        return Ok(());
    }
    atomic_markdown_write(artifact_path, rendered.as_bytes()).await?;
    Ok(())
}

/// Replace an artifact's `valid_until:` frontmatter field — the no-store
/// sibling of [`LanceStore::update_valid_until`] that `forgeplan renew`
/// (ADR-005) needs on a constellation-migrated workspace.
///
/// Same invariants as the other `*_file_first` writers: canonical
/// frontmatter is edited in place, the write is atomic, and it is skipped
/// when the field already holds the requested value (so a no-op renew leaves
/// mtime alone and does not retrigger the watcher).
///
/// Only a non-empty value is accepted — `valid_until` has no "cleared" state
/// in the renew path, and writing an empty `valid_until:` would render as a
/// YAML null that every later parse reads as "present but unparseable".
///
/// # Errors
///
/// Blank value, unparseable frontmatter, or an I/O / atomic-write failure.
/// The file is left untouched when the rendered bytes would be unchanged or
/// the write fails.
pub async fn update_valid_until_file_first(
    artifact_path: &Path,
    valid_until: &str,
) -> anyhow::Result<()> {
    if valid_until.trim().is_empty() {
        anyhow::bail!("renew: valid_until cannot be blank");
    }
    let content = tokio::fs::read_to_string(artifact_path).await?;
    let (mut fm, body) = frontmatter::parse_frontmatter(&content)?;
    if fm
        .get("valid_until")
        .and_then(|v| v.as_str())
        .is_some_and(|existing| existing == valid_until)
    {
        return Ok(());
    }
    fm.insert(
        "valid_until".to_string(),
        serde_yaml::Value::String(valid_until.to_string()),
    );
    let mut rendered = frontmatter::render_frontmatter(&fm, &body)?;
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    if rendered == content {
        return Ok(());
    }
    atomic_markdown_write(artifact_path, rendered.as_bytes()).await?;
    Ok(())
}

/// Set an artifact's `valid_until` with file-first guarantees: sync any user
/// file edits, mutate, then re-render so the frontmatter reflects the new
/// date.
///
/// # Errors
///
/// [`MutationError::InvalidId`] if `id` fails `validate_artifact_id`, or
/// [`MutationError::RowNotFound`] with no legacy sink and no resolvable
/// markdown file for `id`.
pub async fn update_valid_until_with_projection(
    ctx: &MutationContext<'_>,
    id: &str,
    valid_until: &str,
) -> MutationResult<()> {
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    let Some(store) = store else {
        let path = crate::artifact::store::resolve_artifact_path(workspace, id)
            .await
            .ok_or_else(|| MutationError::RowNotFound { id: id.to_string() })?;
        update_valid_until_file_first(&path, valid_until)
            .await
            .map_err(MutationError::from)?;
        return Ok(());
    };
    sync_before_mutation(workspace, store, id).await?;
    store.update_valid_until(id, valid_until).await?;
    render_after_mutation(workspace, store, id).await?;
    Ok(())
}

/// Add and/or remove `tags:` entries in an artifact's projection
/// frontmatter in place — the no-store sibling of
/// [`add_tags_with_projection`] + [`remove_tags_with_projection`].
///
/// Semantics mirror the store path:
/// - reads existing tags through `tags_from_frontmatter`, so a hand-written
///   `tags: [a, b]` inline array or `"a, b"` CSV string normalizes to the
///   same list the index would have held;
/// - `add` appends tags not already present (exact match, same as
///   `store.add_tags`);
/// - `remove` drops exactly the named tags (exact match, same as
///   `store.remove_tags`);
/// - insertion order is preserved and existing order is kept, so a
///   reconcile pass sees a stable diff rather than a reordering;
/// - the write is atomic and skipped when the set is unchanged, so a
///   no-op tag call leaves mtime alone and does not retrigger the watcher.
///
/// An empty `add` list with an empty `remove` list is a no-op.
/// Removing a tag that is not present is not an error — same as the store
/// path, where `remove_tags` deletes nothing and reports success.
///
/// # Errors
///
/// I/O or frontmatter-parse failures. The file is left untouched when the
/// rendered bytes would be unchanged or the write fails.
pub async fn update_tags_file_first(
    artifact_path: &Path,
    add: &[String],
    remove: &[String],
) -> anyhow::Result<()> {
    let content = tokio::fs::read_to_string(artifact_path).await?;
    let (mut fm, body) = frontmatter::parse_frontmatter(&content)?;

    let mut tags = frontmatter::tags_from_frontmatter(&fm);
    let before = tags.clone();
    for tag in add {
        if !tags.contains(tag) {
            tags.push(tag.clone());
        }
    }
    tags.retain(|t| !remove.contains(t));

    if tags == before {
        return Ok(());
    }

    // Keep the empty override: the unchanged body can contain an older tag
    // set, which must not become current again after removing the last tag.
    fm.insert(
        "tags".to_string(),
        serde_yaml::Value::Sequence(tags.into_iter().map(serde_yaml::Value::String).collect()),
    );

    let mut rendered = frontmatter::render_frontmatter(&fm, &body)?;
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    if rendered == content {
        return Ok(());
    }
    atomic_markdown_write(artifact_path, rendered.as_bytes()).await?;
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
///
/// With `ctx.legacy_sink == None` this returns `Ok(())` after validating the
/// id and kind: the caller already holds the canonical file, and there is no
/// legacy index to write it into. It is a no-op, not a refusal — the
/// `FileNotFound` TOCTOU guard below protects a store write that is not
/// happening.
pub async fn sync_artifact_from_file(
    ctx: &MutationContext<'_>,
    artifact: &crate::db::store::NewArtifact,
) -> MutationResult<()> {
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
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
    let Some(store) = store else {
        // File→index direction with no legacy index to write. The caller
        // already holds the canonical file (it just read it), so there is
        // nothing to do and nothing to verify: reconcile owns publishing
        // the derived row. Returning `Ok` — rather than a named refusal —
        // keeps `watch` / `reindex` / `git-sync` reporting the file as
        // handled rather than as a failure they must special-case.
        return Ok(());
    };
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
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
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
    let Some(store) = store else {
        // No legacy index to sync into — see `sync_artifact_from_file`.
        return Ok(());
    };
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
    let store = ctx.legacy_sink;
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
    let Some(store) = store else {
        // No legacy index to sync into — see `sync_artifact_from_file`.
        return Ok(());
    };
    store.update_artifact(id, status, title).await?;
    Ok(())
}

/// Sync a relation from a markdown `links:` block into LanceDB.
/// Used by `reindex` / `git_sync` to restore typed relations after
/// rebuilding the DB from files.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `source` fails `validate_artifact_id`
///   or `target` fails `validate_link_target` (plain id or
///   `KEY@qualifier` — a `links:` block may name a foreign artifact).
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   `add_relation` fails.
pub async fn sync_relation_from_file(
    ctx: &MutationContext<'_>,
    source: &str,
    target: &str,
    relation: &str,
) -> MutationResult<()> {
    // Path-blind helper — see `sync_metadata_from_file` note.
    let store = ctx.legacy_sink;
    crate::db::store::validate_artifact_id(source)
        .map_err(|_| MutationError::InvalidId(source.to_string()))?;
    // A `links:` block may declare a qualified cross-source target —
    // reindex / git-sync must restore it verbatim, not reject it.
    crate::db::store::validate_link_target(target)
        .map_err(|_| MutationError::InvalidId(target.to_string()))?;
    let Some(store) = store else {
        // No legacy index to sync into — see `sync_artifact_from_file`.
        return Ok(());
    };
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
/// - [`MutationError::NoLegacySink`] when the workspace has no legacy index.
///   The markdown is already gone, so there is no canonical counterpart to
///   perform; the orphan belongs to the constellation index, and this
///   reports that rather than claiming a prune it did not make.
pub async fn delete_orphan_artifact(ctx: &MutationContext<'_>, id: &str) -> MutationResult<()> {
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    // Derived-index-only step: the markdown is already gone (reindex
    // Phase 2 / git-sync on `'D'`), so there is no canonical mutation to
    // perform and no counterpart to fall back to. With no legacy sink the
    // row cannot exist, and the orphan this call prunes is the
    // constellation index's business — report it as such rather than
    // claiming the prune happened.
    let store = ctx.require_legacy_sink("prune orphan artifact row")?;
    store.delete_artifact(id).await?;
    Ok(())
}

/// Delete an orphan relation whose source or target artifact no longer
/// exists. Used by `reindex` Phase 3 cleanup.
///
/// # Errors
///
/// - [`MutationError::InvalidId`] if `source` fails `validate_artifact_id`
///   or `target` fails `validate_link_target` (plain id or
///   `KEY@qualifier`). Phase 3 calls this on a qualified edge only when
///   its *source* is gone — the qualified target itself counts as
///   existing-elsewhere there.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   `delete_relation` fails.
/// - [`MutationError::NoLegacySink`] when the workspace has no legacy index
///   — see [`delete_orphan_artifact`].
pub async fn delete_orphan_relation(
    ctx: &MutationContext<'_>,
    source: &str,
    target: &str,
    relation: &str,
) -> MutationResult<()> {
    crate::db::store::validate_artifact_id(source)
        .map_err(|_| MutationError::InvalidId(source.to_string()))?;
    // Qualified targets must also be *deletable* — reindex Phase 3 only
    // calls this for them when the source artifact is gone (see the
    // qualified-target exemption there), and refusing would leak the edge.
    crate::db::store::validate_link_target(target)
        .map_err(|_| MutationError::InvalidId(target.to_string()))?;
    // Derived-index-only step — see `delete_orphan_artifact`.
    let store = ctx.require_legacy_sink("prune orphan relation row")?;
    store.delete_relation(source, target, relation).await?;
    Ok(())
}

/// Restore bundle edges without re-rendering unchanged canonical files.
///
/// Complete-file bundles already declare their edges in `links`. Those edges
/// count as restored, not as failed duplicate additions. A legacy index is
/// updated separately; it must never render its cache over the imported bytes.
pub async fn import_links_with_projection(
    ctx: &MutationContext<'_>,
    links: &[(String, String, String)],
) -> MutationResult<usize> {
    let mut sources: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
    for (source, target, relation) in links {
        crate::db::store::validate_artifact_id(source)
            .map_err(|_| MutationError::InvalidId(source.clone()))?;
        crate::db::store::validate_link_target(target)
            .map_err(|_| MutationError::InvalidId(target.clone()))?;
        sources.entry(source).or_default().push((target, relation));
    }

    let mut restored = 0;
    for (source, edges) in sources {
        let path = crate::artifact::store::resolve_artifact_path(ctx.workspace, source)
            .await
            .ok_or_else(|| MutationError::RowNotFound {
                id: source.to_string(),
            })?;
        let original = tokio::fs::read_to_string(&path)
            .await
            .map_err(anyhow::Error::from)?;
        let (mut fm, body) = frontmatter::parse_frontmatter_borrowed(&original)?;
        let mut known: BTreeMap<String, std::collections::BTreeSet<&str>> = BTreeMap::new();
        if let Some(value) = fm.get("links") {
            let seq = value
                .as_sequence()
                .ok_or_else(|| anyhow::anyhow!("'links' field is not a sequence in {source}"))?;
            for entry in seq {
                if let (Some(target), Some(relation)) = (
                    entry
                        .get("target")
                        .and_then(|value| value.as_str())
                        .map(str::trim)
                        .filter(|value| !value.is_empty()),
                    entry
                        .get("relation")
                        .and_then(|value| value.as_str())
                        .map(str::trim)
                        .filter(|value| !value.is_empty()),
                ) {
                    known
                        .entry(target.to_ascii_lowercase())
                        .or_default()
                        .insert(relation);
                }
            }
        }
        let mut accepted = Vec::with_capacity(edges.len());
        let mut additions = Vec::new();
        for (target, relation) in edges {
            if fm
                .get("id")
                .and_then(|value| value.as_str())
                .is_some_and(|id| id.eq_ignore_ascii_case(target))
            {
                tracing::warn!("import links: self-link {source} --{relation}--> {target} refused");
                continue;
            }
            if known
                .entry(target.to_ascii_lowercase())
                .or_default()
                .insert(relation)
            {
                additions.push((target, relation));
            }
            accepted.push((target, relation));
        }
        drop(known);
        if !additions.is_empty() {
            let value = fm
                .entry("links".to_string())
                .or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));
            let seq = value
                .as_sequence_mut()
                .expect("links sequence validated above");
            for (target, relation) in additions {
                seq.push(
                    serde_yaml::to_value(BTreeMap::from([
                        ("target", target),
                        ("relation", relation),
                    ]))
                    .map_err(anyhow::Error::from)?,
                );
            }
            let rendered = frontmatter::render_frontmatter(&fm, body)?;
            atomic_markdown_write(&path, rendered.as_bytes())
                .await
                .map_err(anyhow::Error::from)?;
        }

        let Some(store) = ctx.legacy_sink else {
            restored += accepted.len();
            continue;
        };
        let stored = store.get_relations(source).await?;
        let mut indexed: BTreeMap<String, std::collections::BTreeSet<&str>> = BTreeMap::new();
        for (target, relation) in &stored {
            indexed
                .entry(target.to_ascii_lowercase())
                .or_default()
                .insert(relation);
        }
        for (target, relation) in accepted {
            let target_key = target.to_ascii_lowercase();
            if indexed
                .get(&target_key)
                .is_some_and(|relations| relations.contains(relation))
            {
                restored += 1;
                continue;
            }
            match store.add_relation(source, target, relation).await {
                Ok(_) => {
                    indexed.entry(target_key).or_default().insert(relation);
                    restored += 1;
                }
                Err(error) => tracing::warn!(
                    "import links: indexing {source} --{relation}--> {target} failed: {error}"
                ),
            }
        }
    }
    Ok(restored)
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
/// - [`MutationError::InvalidId`] if any source id in the batch fails
///   `validate_artifact_id` or any target fails `validate_link_target`
///   (plain id or `KEY@qualifier`). Validation runs up front so a bad
///   id rejects the batch before any write lands.
/// - [`MutationError::StoreFatal`] / [`MutationError::StoreTransient`] if
///   any pre-sync call fails (those are fatal because they snapshot
///   user edits — losing them would corrupt the workspace).
/// - [`MutationError::RowNotFound`] with no legacy sink, when a source id in
///   the batch resolves to no markdown file.
/// - [`MutationError::InvalidId`] is raised for the whole batch before any
///   file is touched when any id is malformed, on both layouts.
///
/// Per-link `add_relation` failures are counted and returned in the
/// `Ok(usize)` payload (number of relations actually applied) — they do
/// not abort the batch. See helper body for the full ordering contract.
pub async fn add_links_batch_with_projection(
    ctx: &MutationContext<'_>,
    links: &[(String, String, String)],
) -> MutationResult<usize> {
    let workspace = ctx.workspace;
    let store = ctx.legacy_sink;
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
        crate::db::store::validate_link_target(target)
            .map_err(|_| MutationError::InvalidId(target.clone()))?;
    }

    let Some(store) = store else {
        // Constellation-managed workspace: each edge is written into its
        // SOURCE file's `links:` block, which is the canonical form. The
        // per-source file is resolved once and every edge for it applied
        // in order, so the deduplicated-render property the store path
        // gets from its post-render phase is preserved here in the only
        // form a file write can offer: one file, all of its edges.
        let mut applied = 0usize;
        let mut resolved_sources: Vec<(String, PathBuf)> = Vec::new();
        for (source, target, relation) in links {
            let idx = match resolved_sources.iter().position(|(s, _)| s == source) {
                Some(idx) => idx,
                None => {
                    let path = crate::artifact::store::resolve_artifact_path(workspace, source)
                        .await
                        .ok_or_else(|| MutationError::RowNotFound { id: source.clone() })?;
                    resolved_sources.push((source.clone(), path));
                    resolved_sources.len() - 1
                }
            };
            let path = resolved_sources[idx].1.clone();
            match crate::link::add_link_verbatim(&path, target, relation).await {
                Ok(()) => applied += 1,
                Err(e) => {
                    // Same count-don't-abort policy as the store path: a
                    // duplicate edge or an unresolvable target must not
                    // strand the rest of the bundle.
                    tracing::warn!(
                        "add_links_batch (file-first): {source} --{relation}--> {target} failed: {e}",
                    );
                }
            }
        }
        return Ok(applied);
    };

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
/// - [`MutationError::NoLegacySink`] when the workspace has no legacy index.
///   The canonical half of the soft delete (moving the file to trash)
///   already happened, so this reports the missing row drop instead of
///   pretending it completed.
pub async fn delete_artifact_after_soft_delete(
    ctx: &MutationContext<'_>,
    id: &str,
) -> MutationResult<()> {
    // Path-blind helper — see `sync_metadata_from_file` note. The file
    // has already been moved to trash by `soft_delete_capture` so this
    // helper deliberately does not touch `ctx.workspace`.
    crate::db::store::validate_artifact_id(id)
        .map_err(|_| MutationError::InvalidId(id.to_string()))?;
    // The canonical half of the soft delete already happened (the file is
    // in trash); this call only drops the derived row. With no legacy
    // sink there is no row, so report that rather than claim the drop.
    let store = ctx.require_legacy_sink("drop row after soft delete")?;
    store.delete_artifact(id).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn publishes_a_new_document_under_its_canonical_key() {
        use crate::workspace::lock::acquire_workspace_lock;

        let tmp = TempDir::new().unwrap();
        let ws = crate::workspace::init_workspace(tmp.path(), "canonical-key").unwrap();
        let lock = acquire_workspace_lock(&ws).await.unwrap();
        let ctx = MutationContext::new(&ws, &lock, None);
        let key = "prd-auth-system-k0123456789abcdef0123456789abcdef";
        let artifact = crate::db::store::NewArtifact {
            canonical_key: key.to_string(),
            id: "PRD-001".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Auth system".to_string(),
            body: "## Summary\n\nBody.\n".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: vec![],
        };
        let path = create_artifact_with_canonical_key(&ctx, &artifact).await.unwrap();
        assert_eq!(path, ws.join("prds").join(format!("{key}.md")));

        let text = tokio::fs::read_to_string(&path).await.unwrap();
        let (fm, _body) = frontmatter::parse_frontmatter(&text).unwrap();
        assert_eq!(
            fm.get("slug").and_then(|v| v.as_str()),
            Some(key),
            "the published document declares the key it is filed under"
        );

        // Publishing onto the same path again aborts, and the existing
        // document is not overwritten.
        let err = create_artifact_with_canonical_key(&ctx, &artifact)
            .await
            .expect_err("publication must not clobber an existing document");
        assert!(
            err.to_string().contains("already exists"),
            "unexpected error: {err}"
        );
        let after = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(after, text, "the existing document must stay untouched");
    }

    #[tokio::test]
    async fn an_update_cannot_change_the_canonical_key() {
        let tmp = TempDir::new().unwrap();
        let path = tmp
            .path()
            .join("prds")
            .join("prd-auth-system-k0123456789abcdef0123456789abcdef.md");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth system\n\
             slug: prd-auth-system-k0123456789abcdef0123456789abcdef\n---\n\n\
             Original body.\n",
        )
        .unwrap();

        let err = update_body_file_first(
            &path,
            "---\nslug: prd-other-system\n---\n\nReplacement body.\n",
        )
        .await
        .expect_err("an ordinary update must not change the canonical key");
        assert!(
            err.to_string().contains("canonical key change refused"),
            "unexpected error: {err:#}"
        );
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(
            after.contains("Original body."),
            "the document must stay untouched"
        );
    }

    #[tokio::test]
    async fn clearing_nested_edges_and_tags_survives_fresh_reads_for_both_owners() {
        use crate::constellation::capture::CaptureLimits;
        use crate::constellation::facade::open_workspace_read;
        use crate::db::store::{LanceStore, NewArtifact};
        use crate::workspace::{init_workspace, lock::acquire_workspace_lock};
        use std::time::Duration;

        let stale = "---\nid: NOTE-001\nkind: note\nstatus: draft\ntitle: Source\ndepth: tactical\nslug: note-source\nauthor: old-author\nparent_epic: EPIC-007\nvalid_until: 2099-01-01\ntags: [obsolete]\nlinks:\n  - target: NOTE-002\n    relation: informs\n---\n\nKeep the original body.\n";
        for legacy in [true, false] {
            let tmp = TempDir::new().unwrap();
            let ws = init_workspace(tmp.path(), "metadata-clears").unwrap();
            let lock = acquire_workspace_lock(&ws).await.unwrap();
            let store = if legacy {
                Some(LanceStore::init(&ws).await.unwrap())
            } else {
                None
            };
            let ctx = MutationContext::new(&ws, &lock, store.as_ref());
            let mut source_path: Option<PathBuf> = None;
            for (id, title, body, tags) in [
                ("NOTE-001", "Source", stale, vec!["obsolete".to_string()]),
                ("NOTE-002", "Target", "Target body.", Vec::new()),
            ] {
                let path = create_artifact_with_projection(
                    &ctx,
                    &NewArtifact {
                        canonical_key: id.to_string(),
                        id: id.to_string(),
                        kind: "note".to_string(),
                        status: "draft".to_string(),
                        title: title.to_string(),
                        body: body.to_string(),
                        depth: "tactical".to_string(),
                        author: None,
                        parent_epic: None,
                        valid_until: None,
                        tags,
                    },
                )
                .await
                .unwrap();
                if id == "NOTE-001" {
                    source_path = Some(path);
                }
            }
            add_link_with_projection(&ctx, "NOTE-001", "NOTE-002", "informs")
                .await
                .unwrap();
            delete_link_with_projection(&ctx, "NOTE-001", "NOTE-002", "informs")
                .await
                .unwrap();
            remove_tags_with_projection(&ctx, "NOTE-001", &["obsolete".to_string()])
                .await
                .unwrap();

            let source = tokio::fs::read_to_string(source_path.unwrap())
                .await
                .unwrap();
            let (fm, body) = frontmatter::parse_frontmatter(&source).unwrap();
            assert!(frontmatter::links_from_frontmatter(&fm).is_empty());
            assert!(frontmatter::tags_from_frontmatter(&fm).is_empty());
            assert!(fm["author"].is_null());
            assert!(fm["parent_epic"].is_null());
            assert!(fm["valid_until"].is_null());
            assert_eq!(body.trim_end(), stale.trim_end());

            let fresh = open_workspace_read(
                &ws,
                None,
                CaptureLimits {
                    budget: Duration::from_secs(120),
                    ..CaptureLimits::default()
                },
            )
            .await
            .unwrap();
            let record = fresh.get_record("NOTE-001").unwrap();
            assert!(fresh.get_relations("NOTE-001").is_empty());
            assert!(record.tags.is_empty());
            assert_eq!(record.author, None);
            assert_eq!(record.parent_epic, None);
            assert_eq!(record.valid_until, None);
            if let Some(store) = &store {
                assert!(store.get_relations("NOTE-001").await.unwrap().is_empty());
                assert!(
                    store
                        .get_record("NOTE-001")
                        .await
                        .unwrap()
                        .unwrap()
                        .tags
                        .is_empty()
                );
            }
        }
    }

    #[tokio::test]
    async fn incoming_link_changes_preserve_target_bytes_and_source_only_metadata() {
        use crate::db::store::LanceStore;
        use crate::workspace::{init_workspace, lock::acquire_workspace_lock};

        for legacy in [true, false] {
            let tmp = TempDir::new().unwrap();
            let ws = init_workspace(tmp.path(), "incoming-link-owner").unwrap();
            let lock = acquire_workspace_lock(&ws).await.unwrap();
            let store = if legacy {
                Some(LanceStore::init(&ws).await.unwrap())
            } else {
                None
            };
            let ctx = MutationContext::new(&ws, &lock, store.as_ref());
            let source = create_artifact_with_projection(&ctx, &art("NOTE-801", "note"))
                .await
                .unwrap();
            let target_artifact = art("NOTE-802", "note");
            let target = create_artifact_with_projection(&ctx, &target_artifact)
                .await
                .unwrap();
            let target_bytes = format!(
                "---\nid: NOTE-802\nkind: note\ntitle: {}\nstatus: draft\ndepth: tactical\nauthor: Source-only author\ncustom: exact\n---\n\nKeep this target untouched.\n",
                target_artifact.title,
            );
            tokio::fs::write(&target, &target_bytes).await.unwrap();

            add_link_with_projection(&ctx, "NOTE-801", "NOTE-802", "informs")
                .await
                .unwrap();
            assert_eq!(
                tokio::fs::read_to_string(&target).await.unwrap(),
                target_bytes
            );
            let raw = tokio::fs::read_to_string(&source).await.unwrap();
            let (fm, _) = frontmatter::parse_frontmatter(&raw).unwrap();
            assert_eq!(
                frontmatter::links_from_frontmatter(&fm),
                [("NOTE-802".to_string(), "informs".to_string())]
            );

            assert_eq!(
                replace_link_with_projection(&ctx, "NOTE-801", "NOTE-802", "based_on")
                    .await
                    .unwrap(),
                LinkUpsertOutcome::Replaced {
                    old_relation: "informs".to_string()
                }
            );
            assert_eq!(
                tokio::fs::read_to_string(&target).await.unwrap(),
                target_bytes
            );
            let raw = tokio::fs::read_to_string(&source).await.unwrap();
            let (fm, _) = frontmatter::parse_frontmatter(&raw).unwrap();
            assert_eq!(
                frontmatter::links_from_frontmatter(&fm),
                [("NOTE-802".to_string(), "based_on".to_string())]
            );

            delete_link_with_projection(&ctx, "NOTE-801", "NOTE-802", "based_on")
                .await
                .unwrap();
            assert_eq!(
                tokio::fs::read_to_string(&target).await.unwrap(),
                target_bytes
            );
            let raw = tokio::fs::read_to_string(&source).await.unwrap();
            let (fm, _) = frontmatter::parse_frontmatter(&raw).unwrap();
            assert!(frontmatter::links_from_frontmatter(&fm).is_empty());
            if let Some(store) = &store {
                assert!(store.get_relations("NOTE-801").await.unwrap().is_empty());
            }
        }
    }

    #[tokio::test]
    async fn bundle_import_keeps_full_source_and_creation_keeps_literal_template_body() {
        use crate::db::store::{LanceStore, NewArtifact};
        use crate::workspace::{init_workspace, lock::acquire_workspace_lock};

        let source = "---\nid: NOTE-001\nkind: note\nstatus: draft\ntitle: Snapshot\nslug: note-snapshot\ncreated: 2026-10-24\ntags: source=fixture, raw, source=fixture\nlinks:\n  - target: \" NOTE-002 \"\n    relation: \" informs \"\n---\n\n## Decision\n\nKeep the literal snapshot.\n";
        let template = "---\nid: NOTE-003\nkind: note\nstatus: draft\ntitle: Template title\ndepth: tactical\nslug: note-template\n---\n\n## Decision\n\nKeep this complete template body.\n";
        for legacy in [true, false] {
            let tmp = TempDir::new().unwrap();
            let ws = init_workspace(tmp.path(), "snapshot-import").unwrap();
            let lock = acquire_workspace_lock(&ws).await.unwrap();
            let store = if legacy {
                Some(LanceStore::init(&ws).await.unwrap())
            } else {
                None
            };
            let ctx = MutationContext::new(&ws, &lock, store.as_ref());
            let imported = import_artifact_with_projection(
                &ctx,
                &NewArtifact {
                    canonical_key: "NOTE-001".to_string(),
                    id: "NOTE-001".to_string(),
                    kind: "note".to_string(),
                    status: "draft".to_string(),
                    title: "Snapshot".to_string(),
                    body: source.to_string(),
                    depth: String::new(),
                    author: None,
                    parent_epic: None,
                    valid_until: None,
                    tags: vec!["raw".to_string(), "source=fixture".to_string()],
                },
            )
            .await
            .unwrap();
            assert_eq!(tokio::fs::read_to_string(&imported).await.unwrap(), source);
            let target = create_artifact_with_projection(&ctx, &art("NOTE-002", "note"))
                .await
                .unwrap();
            let target_before = tokio::fs::read(&target).await.unwrap();

            let created = create_artifact_with_projection(
                &ctx,
                &NewArtifact {
                    canonical_key: "NOTE-003".to_string(),
                    id: "NOTE-003".to_string(),
                    kind: "note".to_string(),
                    status: "draft".to_string(),
                    title: "Selected display title".to_string(),
                    body: template.to_string(),
                    depth: "tactical".to_string(),
                    author: None,
                    parent_epic: None,
                    valid_until: None,
                    tags: Vec::new(),
                },
            )
            .await
            .unwrap();
            let created_before = tokio::fs::read(&created).await.unwrap();
            let invalid = [
                (
                    "NOTE-001".to_string(),
                    "NOTE-002".to_string(),
                    "refines".to_string(),
                ),
                (
                    "NOTE-003".to_string(),
                    "../../escape".to_string(),
                    "informs".to_string(),
                ),
            ];
            assert!(matches!(
                import_links_with_projection(&ctx, &invalid).await,
                Err(MutationError::InvalidId(_))
            ));
            assert_eq!(tokio::fs::read_to_string(&imported).await.unwrap(), source);
            assert_eq!(tokio::fs::read(&created).await.unwrap(), created_before);

            let edges = [
                (
                    "NOTE-001".to_string(),
                    "NOTE-002".to_string(),
                    "informs".to_string(),
                ),
                (
                    "NOTE-003".to_string(),
                    "NOTE-002".to_string(),
                    "informs".to_string(),
                ),
            ];
            assert_eq!(import_links_with_projection(&ctx, &edges).await.unwrap(), 2);
            assert_eq!(tokio::fs::read_to_string(&imported).await.unwrap(), source);
            assert_eq!(tokio::fs::read(&target).await.unwrap(), target_before);
            let restored_once = tokio::fs::read(&created).await.unwrap();
            assert_eq!(import_links_with_projection(&ctx, &edges).await.unwrap(), 2);
            assert_eq!(tokio::fs::read(&created).await.unwrap(), restored_once);
            let restored = tokio::fs::read_to_string(&created).await.unwrap();
            let (fm, body) = frontmatter::parse_frontmatter(&restored).unwrap();
            assert_eq!(fm["title"].as_str(), Some("Selected display title"));
            assert_eq!(body.trim_end(), template.trim_end());
            assert_eq!(
                frontmatter::links_from_frontmatter(&fm),
                [("NOTE-002".to_string(), "informs".to_string())]
            );
            if let Some(store) = &store {
                for id in ["NOTE-001", "NOTE-003"] {
                    assert_eq!(
                        store.get_relations(id).await.unwrap(),
                        [("NOTE-002".to_string(), "informs".to_string())]
                    );
                }
                assert_eq!(
                    store.get_record("NOTE-001").await.unwrap().unwrap().body,
                    source
                );
            }
        }
    }

    #[test]
    fn a_complete_exported_artifact_round_trips_without_another_wrapper() {
        let source = "---\nid: NOTE-001\nkind: note\nstatus: draft\ntitle: Snapshot\ndepth: tactical\nslug: note-snapshot\nassigned_number: 1\ncreated: 2026-10-24\ntags: [source=fixture]\nlinks:\n  - target: NOTE-002\n    relation: informs\n---\n\n## Decision\n\nKeep the literal source.\n";
        let restored = render_markdown_with_extras(
            "NOTE-001",
            "note",
            "Snapshot",
            "draft",
            "tactical",
            None,
            None,
            None,
            source,
            &[],
            &["source=fixture".to_string()],
            None,
            ProjectionBodyMode::CanonicalFile,
        )
        .unwrap();

        assert_eq!(restored, source);
        let (fm, body) = frontmatter::parse_frontmatter(&restored).unwrap();
        assert_eq!(fm["slug"].as_str(), Some("note-snapshot"));
        assert_eq!(
            frontmatter::links_from_frontmatter(&fm),
            [("NOTE-002".to_string(), "informs".to_string())]
        );
        assert_eq!(body, "## Decision\n\nKeep the literal source.\n");
    }

    #[test]
    fn complete_source_metadata_is_preserved_when_explicit_fields_change() {
        let source = "---\nid: NOTE-001\nkind: note\nstatus: draft\ntitle: Original\ndepth: tactical\nslug: note-original\ncreated: 2026-10-24\ncustom_field: keep-me\n---\n\n## Decision\n\nKeep this prose.\n";
        let links = [("NOTE-002".to_string(), "informs".to_string())];
        let changed = render_markdown_with_extras(
            "NOTE-001",
            "note",
            "Renamed",
            "active",
            "deep",
            None,
            None,
            None,
            source,
            &links,
            &[],
            None,
            ProjectionBodyMode::CanonicalFile,
        )
        .unwrap();
        let (fm, body) = frontmatter::parse_frontmatter(&changed).unwrap();

        assert_eq!(fm["title"].as_str(), Some("Renamed"));
        assert_eq!(fm["status"].as_str(), Some("active"));
        assert_eq!(fm["depth"].as_str(), Some("deep"));
        assert_eq!(fm["slug"].as_str(), Some("note-original"));
        assert_eq!(fm["created"].as_str(), Some("2026-10-24"));
        assert_eq!(fm["custom_field"].as_str(), Some("keep-me"));
        assert_eq!(frontmatter::links_from_frontmatter(&fm), links);
        assert_eq!(body.trim_end(), "## Decision\n\nKeep this prose.");
    }

    #[test]
    fn quoted_foreign_artifact_is_body_text_not_the_restored_source() {
        let foreign = "---\nid: NOTE-002\nkind: note\nslug: note-foreign\n---\n\nQuoted source.\n";
        let rendered = render_markdown_with_extras(
            "NOTE-001",
            "note",
            "Own artifact",
            "draft",
            "tactical",
            None,
            None,
            None,
            foreign,
            &[],
            &[],
            None,
            ProjectionBodyMode::CanonicalFile,
        )
        .unwrap();
        let (fm, body) = frontmatter::parse_frontmatter(&rendered).unwrap();

        assert_eq!(fm["id"].as_str(), Some("NOTE-001"));
        assert_eq!(fm["title"].as_str(), Some("Own artifact"));
        assert!(!fm.contains_key("slug"));
        assert_eq!(body.trim_end(), foreign.trim_end());
    }

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
            ProjectionBodyMode::Body,
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
            ProjectionBodyMode::Body,
        )
        .unwrap();
        assert!(content.contains("links:"));
        assert!(content.contains("target: RFC-001"));
        assert!(content.contains("relation: informs"));
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

        let body = "## Summary\n\nSame content.\n\n";

        // Create initial projection
        let initial_path = render_projection(
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
        let initial = tokio::fs::read_to_string(&initial_path).await.unwrap();
        let (_, initial_body) = frontmatter::parse_frontmatter(&initial).unwrap();

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
        let (fm, after_status) = frontmatter::parse_frontmatter(&result).unwrap();
        assert_eq!(fm["status"].as_str(), Some("active"));
        assert_eq!(after_status, initial_body);

        let record = crate::db::store::ArtifactRecord {
            canonical_key: "PRD-001".to_string(),
            id: "PRD-001".to_string(),
            kind: "prd".to_string(),
            title: "Test".to_string(),
            status: "superseded".to_string(),
            depth: "standard".to_string(),
            body: "A stale cache must not replace the canonical body.".to_string(),
            ..Default::default()
        };
        let path = render_projection_record(&ws, &record, &[]).await.unwrap();
        let result = tokio::fs::read_to_string(path).await.unwrap();
        let (fm, after_record) = frontmatter::parse_frontmatter(&result).unwrap();
        assert_eq!(fm["status"].as_str(), Some("superseded"));
        assert_eq!(after_record, initial_body);
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
    async fn sync_detects_a_user_edited_body_and_writes_it_to_the_row() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

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
        store
            .create_artifact(&crate::db::store::NewArtifact {
                canonical_key: "PRD-001".to_string(),
                id: "PRD-001".to_string(),
                kind: "prd".to_string(),
                status: "draft".to_string(),
                title: "Test".to_string(),
                body: db_body.to_string(),
                depth: "standard".to_string(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();

        // Simulate user editing the file.
        let filepath = ws.join("prds").join("PRD-001-test.md");
        let edited = "---\nid: PRD-001\nkind: prd\nstatus: draft\ndepth: standard\ntitle: Test\n---\n\n## Real Content\n\nUser wrote this.\n";
        tokio::fs::write(&filepath, edited).await.unwrap();

        let row = store.get_record("PRD-001").await.unwrap().unwrap();
        assert!(
            sync_file_to_store(&store, &ws, &row).await.unwrap(),
            "the edited body must be detected"
        );
        let after = store.get_record("PRD-001").await.unwrap().unwrap();
        assert!(
            after.body.contains("User wrote this"),
            "the row must carry the user's text, got {:?}",
            after.body
        );
    }

    #[tokio::test]
    async fn sync_is_a_no_op_when_the_file_matches_the_row() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

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
        store
            .create_artifact(&crate::db::store::NewArtifact {
                canonical_key: "PRD-001".to_string(),
                id: "PRD-001".to_string(),
                kind: "prd".to_string(),
                status: "draft".to_string(),
                title: "Test".to_string(),
                body: body.to_string(),
                depth: "standard".to_string(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();

        let row = store.get_record("PRD-001").await.unwrap().unwrap();
        assert!(
            !sync_file_to_store(&store, &ws, &row).await.unwrap(),
            "an unchanged file must not churn the row"
        );
        let after = store.get_record("PRD-001").await.unwrap().unwrap();
        assert_eq!(after.body, body);
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

    // ── #418: body replacement must not delete the ADR-012 identity ─────

    const OLD_WITH_IDENTITY: &str = "---\nid: PRD-001\nslug: prd-auth-system\npredicted_number: 1\nassigned_number: 1\nstatus: Draft\n---\n\n## Problem\n\nold prose\n";

    #[test]
    fn carry_identity_forward_reattaches_triple_to_a_bare_body() {
        // The exact shape CLAUDE.md prescribes right after `forgeplan new`:
        // the caller sends prose only.
        let out = carry_identity_forward(OLD_WITH_IDENTITY, "## Problem\n\nnew prose\n");
        assert!(
            out.contains("slug: prd-auth-system"),
            "slug must survive:\n{out}"
        );
        assert!(out.contains("predicted_number: 1"));
        assert!(out.contains("assigned_number: 1"));
        assert!(
            out.contains("new prose"),
            "the new prose must be what lands"
        );
        assert!(!out.contains("old prose"), "old prose must not resurrect");
        // Only identity is carried — not the old block's stale scaffolding.
        assert!(!out.contains("status: Draft"));
    }

    #[test]
    fn carry_identity_forward_is_noop_for_legacy_artifacts() {
        // Pre-Phase-1.5 artifacts never had a slug; nothing to carry, and the
        // body must come through byte-for-byte.
        let old = "---\nid: PRD-001\nstatus: Draft\n---\n\nold\n";
        let new = "## Problem\n\nplain\n";
        assert_eq!(carry_identity_forward(old, new), new);
    }

    #[test]
    fn carry_identity_forward_respects_caller_supplied_identity() {
        // Round-tripping a full document: the caller's own values win.
        let new =
            "---\nslug: caller-chosen\npredicted_number: 9\nassigned_number: 9\n---\n\nprose\n";
        assert_eq!(carry_identity_forward(OLD_WITH_IDENTITY, new), new);
    }

    #[test]
    fn carry_identity_forward_survives_unparseable_old_body() {
        // A body with no frontmatter at all must not panic or corrupt.
        let new = "## Problem\n\nplain\n";
        assert_eq!(carry_identity_forward("no frontmatter here\n", new), new);
    }

    /// The identity triple was three instances of a class, and the rest of the
    /// class kept being deleted in silence.
    ///
    /// Measured on a real workspace before this was fixed: reading RFC-022's
    /// body back and handing it to `update --body` UNCHANGED — a no-op — cost
    /// eight frontmatter lines. Five (`author`, `depth`, `id`, `status`,
    /// `title`) are in `KNOWN_FM_KEYS` and are regenerated into the projection
    /// block above, so losing the in-body copy is correct. Three were pure
    /// loss, and this pins them.
    #[test]
    fn carry_identity_forward_keeps_every_field_the_record_does_not_own() {
        // The second block of a generated artifact, as `forgeplan new` writes
        // it. `created`/`updated`/`prd` exist ONLY here — no projection field
        // regenerates them.
        let old = "---\nassigned_number: 22\nauthor: null\ncreated: 2026-09-03\n\
                   depth: standard\nid: RFC-022\nprd: null\npredicted_number: 22\n\
                   slug: rfc-production-program\nstatus: Draft\ntitle: 'Production program'\n\
                   updated: 2026-09-03\n---\n\n## Summary\n\nold prose\n";

        let out = carry_identity_forward(old, "## Summary\n\nnew prose\n");

        // Population before the claim: the call produced frontmatter at all.
        // Without this a `!out.contains(...)` pair below would hold for an
        // empty string.
        assert!(
            out.starts_with("---\n"),
            "expected a frontmatter block, got:\n{out}"
        );

        for expected in [
            "created: 2026-09-03",
            "updated: 2026-09-03",
            "prd: null",
            "slug: rfc-production-program",
            "predicted_number: 22",
            "assigned_number: 22",
        ] {
            assert!(
                out.contains(expected),
                "`{expected}` was dropped — it is not a record-owned field:\n{out}"
            );
        }

        // The other direction, and it is the half that keeps this from being
        // "carry everything": a record-owned field must NOT be carried. Its
        // authoritative copy is regenerated into the projection block, and
        // `update_body_with_projection` parses `status` back out of the new
        // body to sync LanceDB — a stale in-body copy could resurrect a
        // superseded status.
        for owned in ["status:", "title:", "depth:", "id:", "author:"] {
            assert!(
                !out.contains(owned),
                "`{owned}` is in KNOWN_FM_KEYS and must not be carried:\n{out}"
            );
        }

        assert!(
            out.contains("new prose"),
            "the new prose must be what lands"
        );
        assert!(!out.contains("old prose"), "old prose must not resurrect");
    }

    // ── #419 BLOCKER: same-slug title change must not delete the file ────

    #[tokio::test]
    async fn rename_helper_removes_old_file_on_a_real_rename() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path();
        let dir = ws.join("prds");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        // Both files exist (new already written by the mutation path).
        tokio::fs::write(dir.join("PRD-001-old-title.md"), "x")
            .await
            .unwrap();
        tokio::fs::write(dir.join("PRD-001-new-title.md"), "x")
            .await
            .unwrap();

        let removed =
            remove_stale_projection_after_rename(ws, "PRD-001", "prd", "Old title", "New title")
                .await
                .unwrap();
        assert!(removed, "a genuine rename must remove the stale file");
        assert!(!dir.join("PRD-001-old-title.md").exists());
        assert!(
            dir.join("PRD-001-new-title.md").exists(),
            "the new file must survive"
        );
    }

    #[tokio::test]
    async fn rename_helper_is_a_noop_when_slug_is_unchanged() {
        // The #419 BLOCKER: "Probe" -> "probe" (or "Probe!") slugifies to the
        // same filename. The helper must NOT delete the just-written file.
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path();
        let dir = ws.join("prds");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let f = dir.join("PRD-001-probe.md");
        tokio::fs::write(&f, "the only copy").await.unwrap();

        for (old, new) in [("Probe", "probe"), ("Probe", "Probe!"), ("Probe", "Probe")] {
            let removed = remove_stale_projection_after_rename(ws, "PRD-001", "prd", old, new)
                .await
                .unwrap();
            assert!(
                !removed,
                "same-slug edit {old:?}->{new:?} must be a no-op, not a delete"
            );
            assert!(
                f.exists(),
                "the artifact file must survive a same-slug edit {old:?}->{new:?}"
            );
        }
    }

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
            ProjectionBodyMode::Body,
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
            ProjectionBodyMode::Body,
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
            canonical_key: "PRD-901".to_string(),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let evil = crate::db::store::NewArtifact {
            canonical_key: "../../etc/evil".to_string(),
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
            create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &evil)
                .await;
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        // Create two memory artifacts where one ID is a strict prefix of the other.
        for (id, title) in [("mem-foo", "foo"), ("mem-foo-bar", "foo bar")] {
            let art = crate::db::store::NewArtifact {
                canonical_key: id.to_string(),
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
            create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &art)
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

        delete_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), "mem-foo")
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let art = crate::db::store::NewArtifact {
            canonical_key: "PRD-901".to_string(),
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
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &art)
            .await
            .unwrap();

        let empty_status = update_metadata_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-901",
            Some(""),
            None,
        )
        .await;
        assert!(empty_status.is_err(), "must reject empty status");
        let empty_title = update_metadata_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let art = crate::db::store::NewArtifact {
            canonical_key: "PRD-902".to_string(),
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
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &art)
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

        update_metadata_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-902",
            None,
            None,
        )
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
            canonical_key: id.to_string(),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let evil = art("../../etc/evil", "note");
        assert!(
            sync_artifact_from_file(&MutationContext::new(&ws, &lock, Some(&store)), &evil)
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let result = sync_body_from_file(
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        assert!(
            sync_relation_from_file(
                &MutationContext::new(&ws, &lock, Some(&store)),
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
                &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        // Place an artifact on disk + in DB
        let a = art("PRD-905", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
            .await
            .unwrap();
        let file = ws.join("prds").join("PRD-905-test-prd-905.md");
        assert!(file.exists(), "setup: file should exist");

        delete_orphan_artifact(&MutationContext::new(&ws, &lock, Some(&store)), "PRD-905")
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-906", "prd");
        let b = art("EVID-906", "evidence");
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
            .await
            .unwrap();
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &b)
            .await
            .unwrap();
        add_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "EVID-906",
            "PRD-906",
            "informs",
        )
        .await
        .unwrap();

        delete_orphan_relation(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "EVID-906",
            "PRD-906",
            "informs",
        )
        .await
        .unwrap();

        let rels = store.get_relations("EVID-906").await.unwrap();
        assert!(rels.is_empty(), "edge must be gone");
    }

    // ─── Constellation stage 4 — qualified cross-source link targets ───

    /// Read the single `.md` projection under `ws/<dir>/` starting with `id-`.
    async fn projection_file_content(ws: &Path, dir: &str, id: &str) -> String {
        let mut rd = tokio::fs::read_dir(ws.join(dir)).await.unwrap();
        while let Some(entry) = rd.next_entry().await.unwrap() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&format!("{id}-")) && name.ends_with(".md") {
                return tokio::fs::read_to_string(entry.path()).await.unwrap();
            }
        }
        panic!("no projection file for {id} under {dir}");
    }

    /// A qualified target lands in the *source* file's `links:` block
    /// verbatim, resolves to no local row, and produces no foreign file.
    #[tokio::test]
    async fn add_link_with_projection_stores_a_qualified_target_in_the_source_only() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art("PRD-940", "prd"),
        )
        .await
        .unwrap();

        add_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-940",
            "ADR-001@src:b7d6f0e1a2c3d4e5",
            "based_on",
        )
        .await
        .unwrap();

        // The relation row stores the ref verbatim.
        let rels = store.get_relations("PRD-940").await.unwrap();
        assert_eq!(
            rels,
            vec![(
                "ADR-001@src:b7d6f0e1a2c3d4e5".to_string(),
                "based_on".to_string()
            )]
        );

        // The source file's frontmatter carries it — ADR-003 truth lives
        // in markdown, not in LanceDB.
        let content = projection_file_content(&ws, "prds", "PRD-940").await;
        assert!(
            content.contains("target: ADR-001@src:b7d6f0e1a2c3d4e5"),
            "source file must carry the qualified target verbatim:\n{content}"
        );

        // No artifact was created for the qualified ref — the write is
        // source-only; `ADR-001` here would be a foreign artifact's ghost.
        assert!(store.get_record("ADR-001").await.unwrap().is_none());
        assert!(
            store
                .get_record("ADR-001@src:b7d6f0e1a2c3d4e5")
                .await
                .unwrap()
                .is_none()
        );
        let mut found_link_file = false;
        for dir_name in crate::workspace::ARTIFACT_DIRS {
            let dir = ws.join(dir_name);
            if !dir.exists() {
                continue;
            }
            let mut rd = tokio::fs::read_dir(&dir).await.unwrap();
            while let Some(entry) = rd.next_entry().await.unwrap() {
                let name = entry.file_name().to_string_lossy().to_string();
                assert!(
                    !name.contains('@'),
                    "no artifact file may be rendered for a qualified target: {name}"
                );
                if name.starts_with("ADR-001") {
                    found_link_file = true;
                }
            }
        }
        assert!(!found_link_file, "no local ADR-001 projection may appear");
    }

    /// `reindex`/`git_sync` restore edges from `links:` blocks — a
    /// qualified target must survive that path verbatim too.
    #[tokio::test]
    async fn sync_relation_from_file_restores_a_qualified_target() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art("PRD-941", "prd"),
        )
        .await
        .unwrap();

        sync_relation_from_file(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-941",
            "ADR-001@ws-beta",
            "informs",
        )
        .await
        .unwrap();

        let rels = store.get_relations("PRD-941").await.unwrap();
        assert_eq!(
            rels,
            vec![("ADR-001@ws-beta".to_string(), "informs".to_string())]
        );
    }

    /// `unlink` accepts what `link` stored — the delete path validates the
    /// same qualified shape and removes by verbatim string equality.
    #[tokio::test]
    async fn delete_link_with_projection_removes_a_qualified_edge() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art("PRD-942", "prd"),
        )
        .await
        .unwrap();
        add_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-942",
            "ADR-001@ws-beta",
            "informs",
        )
        .await
        .unwrap();

        delete_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-942",
            "ADR-001@ws-beta",
            "informs",
        )
        .await
        .unwrap();

        assert!(store.get_relations("PRD-942").await.unwrap().is_empty());
        let content = projection_file_content(&ws, "prds", "PRD-942").await;
        assert!(
            !content.contains("ADR-001@ws-beta"),
            "the re-rendered source file must drop the qualified edge:\n{content}"
        );
    }

    /// The qualifier charset is a filter-injection boundary: `delete_relation`
    /// interpolates the stored string into `target_id = '…'`, so quotes and
    /// separators stay rejected on the qualified form too.
    #[tokio::test]
    async fn add_link_with_projection_rejects_an_injectable_qualified_target() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        for bad in ["ADR-001@x' OR '1'='1", "ADR-001@a/b", "ADR-001@a b"] {
            let err = add_link_with_projection(
                &MutationContext::new(&ws, &lock, Some(&store)),
                "PRD-001",
                bad,
                "informs",
            )
            .await
            .expect_err("injectable qualified target must be rejected");
            assert!(
                matches!(err, MutationError::InvalidId(ref s) if s == bad),
                "expected InvalidId({bad}), got {err:?}",
            );
        }
    }

    /// Orphan cleanup must be able to *remove* a qualified edge — Phase 3
    /// only reaches one when its source artifact is gone.
    #[tokio::test]
    async fn delete_orphan_relation_drops_a_qualified_edge() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art("PRD-943", "prd"),
        )
        .await
        .unwrap();
        add_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-943",
            "ADR-001@src:b7d6f0e1a2c3d4e5",
            "informs",
        )
        .await
        .unwrap();

        delete_orphan_relation(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-943",
            "ADR-001@src:b7d6f0e1a2c3d4e5",
            "informs",
        )
        .await
        .unwrap();

        assert!(store.get_relations("PRD-943").await.unwrap().is_empty());
    }

    // ---------------------------------------------------------------------
    // Issue #286 — `replace_link_with_projection` upsert semantics
    // ---------------------------------------------------------------------

    /// Create source + target artifacts in a fresh temp workspace. Shared
    /// setup for the three upsert-outcome tests below.
    ///
    /// The lock is returned rather than dropped: every caller mutates through
    /// a `MutationContext` that borrows it, so the caller needs the same guard
    /// the setup wrote with. It is bound last so it drops first, before the
    /// store it protects.
    async fn upsert_test_workspace(
        src_id: &str,
        tgt_id: &str,
        src_kind: &str,
        tgt_kind: &str,
    ) -> (
        TempDir,
        std::path::PathBuf,
        crate::db::store::LanceStore,
        crate::workspace::WorkspaceLock,
    ) {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art(src_id, src_kind),
        )
        .await
        .unwrap();
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art(tgt_id, tgt_kind),
        )
        .await
        .unwrap();
        (tmp, ws, store, lock)
    }

    /// On a fresh `(source, target)` pair `replace_link_with_projection`
    /// produces `Created` and the edge appears in the store.
    #[tokio::test]
    async fn replace_link_returns_created_on_fresh_pair() {
        let (_tmp, ws, store, lock) =
            upsert_test_workspace("EVID-920", "PRD-920", "evidence", "prd").await;

        let outcome = replace_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "EVID-920",
            "PRD-920",
            "informs",
        )
        .await
        .unwrap();

        assert_eq!(outcome, LinkUpsertOutcome::Created);
        let rels = store.get_relations("EVID-920").await.unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].1, "informs");
    }

    /// When the same `(source, target, relation)` already exists,
    /// `replace_link_with_projection` returns `Unchanged` and the edge
    /// count stays at one (no parallel duplicate).
    #[tokio::test]
    async fn replace_link_returns_unchanged_on_identical_relation() {
        let (_tmp, ws, store, lock) =
            upsert_test_workspace("EVID-921", "PRD-921", "evidence", "prd").await;

        add_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "EVID-921",
            "PRD-921",
            "informs",
        )
        .await
        .unwrap();

        let outcome = replace_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "EVID-921",
            "PRD-921",
            "informs",
        )
        .await
        .unwrap();

        assert_eq!(outcome, LinkUpsertOutcome::Unchanged);
        let rels = store.get_relations("EVID-921").await.unwrap();
        assert_eq!(
            rels.len(),
            1,
            "identical upsert must not create a parallel edge: {rels:?}"
        );
    }

    /// CORE issue #286 scenario — pre-existing `based_on` edge that should
    /// have been `informs`. The upsert helper deletes the old edge and
    /// adds the new one, returns `Replaced { old_relation: "based_on" }`,
    /// and the final state contains exactly one `informs` edge.
    #[tokio::test]
    async fn replace_link_swaps_mistyped_based_on_to_informs() {
        let (_tmp, ws, store, lock) =
            upsert_test_workspace("EVID-922", "PRD-922", "evidence", "prd").await;

        // Initial mistake: link with based_on.
        add_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "EVID-922",
            "PRD-922",
            "based_on",
        )
        .await
        .unwrap();

        // Fix via upsert.
        let outcome = replace_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "EVID-922",
            "PRD-922",
            "informs",
        )
        .await
        .unwrap();

        match outcome {
            LinkUpsertOutcome::Replaced { old_relation } => {
                assert_eq!(old_relation, "based_on");
            }
            other => panic!("expected Replaced {{ old_relation: based_on }}, got {other:?}"),
        }
        let rels = store.get_relations("EVID-922").await.unwrap();
        assert_eq!(rels.len(), 1, "exactly one edge must remain: {rels:?}");
        assert_eq!(rels[0].0, "PRD-922");
        assert_eq!(rels[0].1, "informs");
    }

    /// Audit-r2 coverage closure: reverse direction. The canonical #286
    /// scenario is `based_on → informs`. The mirror case (`informs →
    /// based_on`) should also work — useful when an evidence pack is
    /// later promoted into a derivation chain.
    #[tokio::test]
    async fn replace_link_swaps_informs_to_based_on_reverse() {
        let (_tmp, ws, store, lock) =
            upsert_test_workspace("PRD-923", "PRD-924", "prd", "prd").await;

        // Initial: PRD-923 informs PRD-924.
        add_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-923",
            "PRD-924",
            "informs",
        )
        .await
        .unwrap();

        let outcome = replace_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-923",
            "PRD-924",
            "based_on",
        )
        .await
        .unwrap();

        match outcome {
            LinkUpsertOutcome::Replaced { old_relation } => {
                assert_eq!(old_relation, "informs");
            }
            other => panic!("expected Replaced {{ old_relation: informs }}, got {other:?}"),
        }
        let rels = store.get_relations("PRD-923").await.unwrap();
        assert_eq!(
            rels.len(),
            1,
            "exactly one edge after reverse swap: {rels:?}"
        );
        assert_eq!(rels[0].1, "based_on");
    }

    /// Audit-r2 coverage closure: replace with `refines` relation. All
    /// existing tests hard-code based_on/informs; verifying refines works
    /// catches dispatch bugs in normalize_relation or in match arms on
    /// the new code path.
    #[tokio::test]
    async fn replace_link_swaps_to_refines_relation() {
        let (_tmp, ws, store, lock) =
            upsert_test_workspace("RFC-925", "ADR-925", "rfc", "adr").await;

        add_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "RFC-925",
            "ADR-925",
            "informs",
        )
        .await
        .unwrap();

        let outcome = replace_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "RFC-925",
            "ADR-925",
            "refines",
        )
        .await
        .unwrap();

        assert!(
            matches!(outcome, LinkUpsertOutcome::Replaced { ref old_relation } if old_relation == "informs"),
            "expected Replaced{{informs}}, got {outcome:?}"
        );
        let rels = store.get_relations("RFC-925").await.unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].1, "refines");
    }

    /// Audit-r2 coverage closure: replace with `supersedes` relation.
    /// Same rationale as refines — exercises the full relation set.
    #[tokio::test]
    async fn replace_link_swaps_to_supersedes_relation() {
        let (_tmp, ws, store, lock) =
            upsert_test_workspace("PRD-926", "PRD-927", "prd", "prd").await;

        add_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-926",
            "PRD-927",
            "informs",
        )
        .await
        .unwrap();

        let outcome = replace_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-926",
            "PRD-927",
            "supersedes",
        )
        .await
        .unwrap();

        assert!(
            matches!(outcome, LinkUpsertOutcome::Replaced { .. }),
            "supersedes replacement must succeed: {outcome:?}"
        );
    }

    /// A1.6 — add_links_batch validates all ids up front, no partial state.
    #[tokio::test]
    async fn add_links_batch_validates_up_front_and_no_partial_state() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-907", "prd");
        let b = art("EVID-907", "evidence");
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
            .await
            .unwrap();
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &b)
            .await
            .unwrap();

        // Mix: first link is good, second is path-traversal payload.
        let links = vec![
            ("EVID-907".into(), "PRD-907".into(), "informs".into()),
            ("../../etc/evil".into(), "PRD-907".into(), "informs".into()),
        ];
        let result = add_links_batch_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &links,
        )
        .await;
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        // 1 source, 3 distinct targets → 4 unique participants, 3 relations.
        for id in ["EVID-908", "PRD-908", "PRD-909", "PRD-910"] {
            let kind = if id.starts_with("EVID") {
                "evidence"
            } else {
                "prd"
            };
            create_artifact_with_projection(
                &MutationContext::new(&ws, &lock, Some(&store)),
                &art(id, kind),
            )
            .await
            .unwrap();
        }
        let links = vec![
            ("EVID-908".into(), "PRD-908".into(), "informs".into()),
            ("EVID-908".into(), "PRD-909".into(), "informs".into()),
            ("EVID-908".into(), "PRD-910".into(), "informs".into()),
        ];

        let applied = add_links_batch_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &links,
        )
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-911", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
            .await
            .unwrap();
        let file = ws.join("prds").join("PRD-911-test-prd-911.md");
        assert!(file.exists());

        delete_artifact_after_soft_delete(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-911",
        )
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-912", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
            .await
            .unwrap();

        // Calling with (None, None) should NOT error and should NOT mutate.
        // (Audit code-reviewer #3: catches the inconsistency vs update_metadata_with_projection.)
        let result = sync_metadata_from_file(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-912",
            None,
            None,
        )
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-913", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
            .await
            .unwrap();

        let empty_status = sync_metadata_from_file(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-913",
            Some(""),
            None,
        )
        .await;
        assert!(empty_status.is_err(), "must reject empty status");
        let empty_title = sync_metadata_from_file(
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = sync_metadata_from_file(
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-921", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
            .await
            .unwrap();

        let err = sync_metadata_from_file(
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = sync_relation_from_file(
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = delete_orphan_artifact(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "../../boom",
        )
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = delete_orphan_relation(
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        for id in ["PRD-922", "EVID-922"] {
            let kind = if id.starts_with("EVID") {
                "evidence"
            } else {
                "prd"
            };
            create_artifact_with_projection(
                &MutationContext::new(&ws, &lock, Some(&store)),
                &art(id, kind),
            )
            .await
            .unwrap();
        }

        let links = vec![
            ("EVID-922".into(), "PRD-922".into(), "informs".into()),
            ("../../evil".into(), "PRD-922".into(), "informs".into()),
        ];
        let err = add_links_batch_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &links,
        )
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = delete_artifact_after_soft_delete(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "..\\evil",
        )
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = remove_tags_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = add_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
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
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = delete_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "../bad",
            "PRD-001",
            "informs",
        )
        .await
        .expect_err("bad source must be rejected");
        assert!(matches!(err, MutationError::InvalidId(ref s) if s == "../bad"));

        let err = delete_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        // Construct a NewArtifact with a valid id+kind+title but DON'T
        // write the projection on disk. Sync must refuse.
        let ghost = art("PRD-930", "prd");
        let err = sync_artifact_from_file(&MutationContext::new(&ws, &lock, Some(&store)), &ghost)
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        // Create the artifact (so DB row exists) and then nuke the file
        // to simulate the TOCTOU race.
        let a = art("PRD-931", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
            .await
            .unwrap();
        let file = ws.join("prds").join("PRD-931-test-prd-931.md");
        assert!(file.exists(), "setup precondition: file must exist");
        tokio::fs::remove_file(&file).await.unwrap();

        let err = sync_body_from_file(
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
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
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
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
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
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
        let result =
            sync_artifact_from_file(&MutationContext::new(&ws, &lock, Some(&store)), &ghost).await;

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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let ghost = art("PRD-941", "prd");
        let err = sync_artifact_from_file(&MutationContext::new(&ws, &lock, Some(&store)), &ghost)
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let mut a = art("PRD-001", "prd");
        a.id = "../../etc/evil".to_string();

        let err =
            create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let mut a = art("PRD-700", "prd");
        a.kind = "bogus_kind".to_string();

        let err =
            create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let mut a = art("PRD-701", "prd");
        a.title = "Задача".to_string();

        let path =
            create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = delete_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "../escape",
        )
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = update_body_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "1bad-start",
            "body",
        )
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = update_body_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-9999",
            "body",
        )
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

    /// PROB-093, on the path the CLI actually takes.
    ///
    /// A store-level test of `update_body` passed while the real `forgeplan
    /// update --body` still served the old vector — the unit test called the
    /// store directly and missed whatever the projection layer does around it.
    /// This one drives `update_body_with_projection`, which is what
    /// `commands/update.rs:151` calls.
    ///
    /// The vector is now seeded through `replace_record`, the same way the
    /// sibling `add_tags_preserves_embedding` test does it. The previous
    /// version called `LanceStore::update_embedding`, which no longer exists:
    /// an unguarded vector write is precisely what the cutover removed, and a
    /// test that needs one to set up state is a test keeping it alive.
    #[tokio::test]
    async fn update_body_with_projection_retires_the_previous_vector() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-943", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
            .await
            .unwrap();

        let dim = crate::db::schema::EMBEDDING_DIM as usize;
        let mut seeded = store.get_record("PRD-943").await.unwrap().unwrap();
        seeded.embedding = Some(vec![0.25f32; dim]);
        store.replace_record(&seeded).await.unwrap();
        assert!(
            store
                .get_record("PRD-943")
                .await
                .unwrap()
                .unwrap()
                .embedding
                .is_some(),
            "precondition: a vector must exist before the rewrite"
        );

        update_body_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-943",
            "## Different\n\nEntirely unrelated subject matter.",
        )
        .await
        .unwrap();

        let rec = store.get_record("PRD-943").await.unwrap().unwrap();
        assert!(
            rec.embedding.is_none(),
            "the vector of the previous body must not survive the CLI's own \
             update path — it made the artifact match text it no longer contains"
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let a = art("PRD-942", "prd");
        create_artifact_with_projection(&MutationContext::new(&ws, &lock, Some(&store)), &a)
            .await
            .unwrap();

        let new_body = "## Updated body\n\nfresh content for happy-path";
        update_body_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            "PRD-942",
            new_body,
        )
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = sync_metadata_from_file(
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = update_depth_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
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
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();

        let err = add_tags_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
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

/// Property-based tests for `replace_link_with_projection` invariants.
///
/// Audit-r2 closure (#286): original unit tests covered three outcomes
/// (Created / Unchanged / Replaced{based_on→informs}) + SEC HIGH-1
/// ordering fix. Follow-up tests added reverse-direction + refines +
/// supersedes. This module shotguns the remaining input space — every
/// legal relation × every legal starting state × idempotency-on-second-
/// call invariant.
///
/// **Invariants pinned**:
/// 1. After successful replace, exactly ONE edge between `(source, target)`.
/// 2. That edge has the requested `relation`.
/// 3. Second call with same args produces `Unchanged`.
/// 4. State-machine outcome (Created / Replaced / Unchanged) deterministic
///    given starting state.
#[cfg(test)]
mod replace_link_property_tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::TempDir;

    const RELATIONS: &[&str] = &[
        "informs",
        "based_on",
        "supersedes",
        "refines",
        "contradicts",
    ];

    fn art(id: &str, kind: &str) -> crate::db::store::NewArtifact {
        crate::db::store::NewArtifact {
            canonical_key: id.to_string(),
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

    async fn run_replace_scenario(
        initial_relation: Option<&str>,
        new_relation: &str,
        source_id: &str,
        target_id: &str,
    ) {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art(source_id, "prd"),
        )
        .await
        .unwrap();
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art(target_id, "prd"),
        )
        .await
        .unwrap();

        if let Some(rel) = initial_relation {
            add_link_with_projection(
                &MutationContext::new(&ws, &lock, Some(&store)),
                source_id,
                target_id,
                rel,
            )
            .await
            .unwrap();
        }

        let outcome = replace_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            source_id,
            target_id,
            new_relation,
        )
        .await
        .unwrap();

        // Invariant 4: deterministic outcome.
        let outcome_ok = match (initial_relation, &outcome) {
            (None, LinkUpsertOutcome::Created) => true,
            (Some(old), LinkUpsertOutcome::Replaced { old_relation }) => {
                old == old_relation.as_str() && old != new_relation
            }
            (Some(old), LinkUpsertOutcome::Unchanged) => old == new_relation,
            _ => false,
        };
        assert!(
            outcome_ok,
            "outcome doesn't match starting state: initial={initial_relation:?} \
             new={new_relation} got={outcome:?}"
        );

        // Invariant 1+2: exactly one edge with requested relation.
        let rels = store.get_relations(source_id).await.unwrap();
        let same_target: Vec<_> = rels
            .iter()
            .filter(|(t, _)| t.eq_ignore_ascii_case(target_id))
            .collect();
        assert_eq!(same_target.len(), 1, "exactly one edge: {same_target:?}");
        assert_eq!(same_target[0].1, new_relation);

        // Invariant 3: idempotency.
        let outcome2 = replace_link_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            source_id,
            target_id,
            new_relation,
        )
        .await
        .unwrap();
        assert_eq!(outcome2, LinkUpsertOutcome::Unchanged);

        let rels2 = store.get_relations(source_id).await.unwrap();
        let same_target2: Vec<_> = rels2
            .iter()
            .filter(|(t, _)| t.eq_ignore_ascii_case(target_id))
            .collect();
        assert_eq!(
            same_target2.len(),
            1,
            "idempotent replace must not add parallel edges"
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 12,
            max_shrink_iters: 8,
            .. ProptestConfig::default()
        })]

        /// Fuzz: empty workspace → replace with any relation = Created.
        #[test]
        fn replace_from_empty_produces_created(
            new_rel_idx in 0usize..RELATIONS.len(),
        ) {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(run_replace_scenario(
                None,
                RELATIONS[new_rel_idx],
                "PRD-A001",
                "PRD-B001",
            ));
        }

        /// Fuzz: any initial relation × any new relation. Same → Unchanged,
        /// different → Replaced{old}; in both cases ends with one edge.
        #[test]
        fn replace_swaps_relation_correctly(
            initial_idx in 0usize..RELATIONS.len(),
            new_idx in 0usize..RELATIONS.len(),
        ) {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(run_replace_scenario(
                Some(RELATIONS[initial_idx]),
                RELATIONS[new_idx],
                "PRD-A002",
                "PRD-B002",
            ));
        }
    }
}

// =============================================================================
// Constellation cutover — behaviour of `legacy_sink: None`
// -----------------------------------------------------------------------------
// These tests pin the two halves of the cutover contract that a signature
// change alone cannot prove:
//
//   * `None` is NOT a refusal. Every helper whose job includes a canonical
//     Markdown mutation still performs it, byte-for-byte, and returns Ok.
//   * `None` is NOT a faked database. Nothing opens `artifacts.lance`, and
//     the derived-index-ONLY helpers report `NoLegacySink` rather than
//     claiming a row write they did not make.
//
// The `Some` halves of the same helpers are covered by the existing suites
// above; each test here names the sink it is asserting about so a regression
// that silently flips one branch is visible.
// =============================================================================

#[cfg(test)]
mod file_first_sink_tests {
    use super::*;
    use crate::workspace::lock::WorkspaceLock;
    use tempfile::TempDir;

    fn art(id: &str, kind: &str, title: &str) -> crate::db::store::NewArtifact {
        crate::db::store::NewArtifact {
            canonical_key: id.to_string(),
            id: id.to_string(),
            kind: kind.to_string(),
            status: "draft".to_string(),
            title: title.to_string(),
            body: "Original body.".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        }
    }

    /// A constellation-managed workspace: markdown exists, there is NO
    /// `artifacts.lance`, and `LanceStore::init` is deliberately never
    /// called. That is the real shape of a migrated workspace — if a helper
    /// tried to open a store here it would fail loudly rather than silently
    /// resurrect a retired table.
    async fn migrated_fixture() -> (TempDir, PathBuf, WorkspaceLock) {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let lock = crate::workspace::acquire_workspace_lock(&ws).await.unwrap();
        (tmp, ws, lock)
    }

    /// Read an artifact file's projection frontmatter + body.
    async fn read_doc(ws: &Path, id: &str) -> (crate::artifact::frontmatter::Frontmatter, String) {
        let path = crate::artifact::store::resolve_artifact_path(ws, id)
            .await
            .unwrap_or_else(|| panic!("{id} must resolve to a markdown file"));
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        crate::artifact::frontmatter::parse_frontmatter(&content).unwrap()
    }

    fn fm_string(fm: &crate::artifact::frontmatter::Frontmatter, key: &str) -> Option<String> {
        fm.get(key).and_then(|v| v.as_str()).map(String::from)
    }

    // ── create ─────────────────────────────────────────────────────────────

    /// `Some` writes markdown AND takes the derived row; `None` writes the
    /// same markdown and opens no database. Same canonical bytes either way.
    #[tokio::test]
    async fn create_writes_the_same_markdown_with_and_without_a_sink() {
        let (_tmp_a, ws_a, lock_a) = migrated_fixture().await;
        let _tmp_b = TempDir::new().unwrap();
        let ws_b = _tmp_b.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws_b).await.unwrap();
        let lock_b = crate::workspace::acquire_workspace_lock(&ws_b)
            .await
            .unwrap();
        let store_b = crate::db::store::LanceStore::init(&ws_b).await.unwrap();

        let path_none = create_artifact_with_projection(
            &MutationContext::new(&ws_a, &lock_a, None),
            &art("PRD-100", "prd", "Auth"),
        )
        .await
        .unwrap();

        let path_some = create_artifact_with_projection(
            &MutationContext::new(&ws_b, &lock_b, Some(&store_b)),
            &art("PRD-100", "prd", "Auth"),
        )
        .await
        .unwrap();

        let bytes_none = tokio::fs::read(&path_none).await.unwrap();
        let bytes_some = tokio::fs::read(&path_some).await.unwrap();
        assert_eq!(
            bytes_none, bytes_some,
            "the canonical Markdown must be identical across both layouts"
        );

        // And the `None` workspace really has no legacy tables — the helper
        // did not create one behind our back.
        assert!(
            !ws_a.join("lance/artifacts.lance").exists(),
            "the file-first path must not materialise a legacy table"
        );
    }

    // ── update metadata / body / depth ─────────────────────────────────────

    #[tokio::test]
    async fn update_metadata_without_a_sink_writes_the_canonical_file() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, None),
            &art("PRD-101", "prd", "Auth"),
        )
        .await
        .unwrap();

        update_metadata_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-101",
            Some("active"),
            None,
        )
        .await
        .unwrap();

        let (fm, _) = read_doc(&ws, "PRD-101").await;
        assert_eq!(fm_string(&fm, "status").as_deref(), Some("active"));
    }

    /// A title change must rename to the canonical slug with no sink, and the
    /// following body update must land on the NEW file — the sequence
    /// `forgeplan update --title … --body …` performs.
    #[tokio::test]
    async fn a_title_change_renames_and_the_next_update_uses_the_new_file() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, None),
            &art("PRD-102", "prd", "Old Title"),
        )
        .await
        .unwrap();

        update_metadata_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-102",
            None,
            Some("New Title"),
        )
        .await
        .unwrap();

        let path = crate::artifact::store::resolve_artifact_path(&ws, "PRD-102")
            .await
            .expect("artifact must still resolve after rename");
        assert_eq!(
            path.file_name().unwrap().to_string_lossy(),
            "PRD-102-new-title.md",
            "the canonical filename follows the new slug"
        );
        assert!(
            !ws.join("prds/PRD-102-old-title.md").exists(),
            "the old-slug file must be gone, not left as a phantom source"
        );

        // The body update that follows must write to the renamed file.
        update_body_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-102",
            "## Fresh\n\nrewritten.",
        )
        .await
        .unwrap();
        let (_, body) = read_doc(&ws, "PRD-102").await;
        assert!(
            body.contains("rewritten"),
            "the body update must land on the renamed file, got: {body}"
        );
    }

    /// The new body's own `status:` still propagates into the projection
    /// block with no sink — the audit-r7 F1 rule the store path enforces.
    #[tokio::test]
    async fn a_status_declared_in_a_new_body_reaches_the_projection_block() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, None),
            &art("PRD-103", "prd", "Auth"),
        )
        .await
        .unwrap();

        update_body_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-103",
            "---\nstatus: deprecated\n---\n\n## Body\n\ntext.",
        )
        .await
        .unwrap();

        let (fm, _) = read_doc(&ws, "PRD-103").await;
        assert_eq!(fm_string(&fm, "status").as_deref(), Some("deprecated"));
    }

    /// A body replacement must not silently drop body-frontmatter fields the
    /// projection does not own (`created`, `slug`, …) — `carry_identity_forward`.
    #[tokio::test]
    async fn a_body_replacement_carries_identity_fields_forward_without_a_sink() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        let path = render_projection_with_body(
            &ws,
            "PRD-104",
            "prd",
            "Auth",
            "draft",
            "standard",
            None,
            None,
            None,
            "---\ncreated: 2026-01-01\nslug: keep-me\n---\n\n## Original\n\nbody.",
            &[],
        )
        .await
        .unwrap();

        update_body_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-104",
            "## Replacement\n\nnew body.",
        )
        .await
        .unwrap();

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(
            content.contains("keep-me"),
            "body-frontmatter the projection does not own must survive: {content}"
        );
    }

    #[tokio::test]
    async fn update_depth_without_a_sink_writes_the_canonical_file() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, None),
            &art("PRD-105", "prd", "Auth"),
        )
        .await
        .unwrap();

        update_depth_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-105",
            "critical",
        )
        .await
        .unwrap();

        let (fm, _) = read_doc(&ws, "PRD-105").await;
        assert_eq!(fm_string(&fm, "depth").as_deref(), Some("critical"));
    }

    // ── tags ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn tag_helpers_edit_the_canonical_frontmatter_without_a_sink() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, None),
            &art("PRD-106", "prd", "Auth"),
        )
        .await
        .unwrap();

        add_tags_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-106",
            &["source=code".to_string(), "layer=domain".to_string()],
        )
        .await
        .unwrap();

        let (fm, _) = read_doc(&ws, "PRD-106").await;
        let mut tags = crate::artifact::frontmatter::tags_from_frontmatter(&fm);
        tags.sort();
        assert_eq!(
            tags,
            vec!["layer=domain".to_string(), "source=code".to_string()]
        );

        // Re-adding an existing tag must not duplicate it.
        add_tags_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-106",
            &["source=code".to_string()],
        )
        .await
        .unwrap();
        let (fm, _) = read_doc(&ws, "PRD-106").await;
        assert_eq!(
            crate::artifact::frontmatter::tags_from_frontmatter(&fm).len(),
            2,
            "adding an existing tag must be a no-op"
        );

        remove_tags_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-106",
            &["layer=domain".to_string()],
        )
        .await
        .unwrap();
        let (fm, _) = read_doc(&ws, "PRD-106").await;
        assert_eq!(
            crate::artifact::frontmatter::tags_from_frontmatter(&fm),
            vec!["source=code".to_string()],
            "remove must drop exactly the named tag"
        );
    }

    /// Removing a tag that is not there is success, matching the store path —
    /// not an error the caller has to special-case.
    #[tokio::test]
    async fn removing_an_absent_tag_without_a_sink_is_success() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, None),
            &art("PRD-108", "prd", "Auth"),
        )
        .await
        .unwrap();

        assert!(
            remove_tags_with_projection(
                &MutationContext::new(&ws, &lock, None),
                "PRD-108",
                &["never-added".to_string()],
            )
            .await
            .is_ok(),
            "removing an absent tag must mirror store.remove_tags"
        );
    }

    // ── links ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn a_link_lands_in_the_source_file_without_a_sink() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        for id in ["PRD-109", "RFC-109"] {
            create_artifact_with_projection(
                &MutationContext::new(&ws, &lock, None),
                &art(id, if id.starts_with("RFC") { "rfc" } else { "prd" }, id),
            )
            .await
            .unwrap();
        }

        add_link_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-109",
            "RFC-109",
            "informs",
        )
        .await
        .unwrap();

        let (fm, _) = read_doc(&ws, "PRD-109").await;
        let links = crate::link::list_links(&fm);
        assert_eq!(links, vec![("RFC-109".to_string(), "informs".to_string())]);

        delete_link_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-109",
            "RFC-109",
            "informs",
        )
        .await
        .unwrap();
        let (fm, _) = read_doc(&ws, "PRD-109").await;
        assert!(
            crate::link::list_links(&fm).is_empty(),
            "the edge must be gone from the canonical file"
        );
    }

    /// A qualified cross-source target must survive verbatim with no sink —
    /// it is never resolved, opened, or rewritten.
    #[tokio::test]
    async fn a_qualified_link_target_is_stored_verbatim_without_a_sink() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, None),
            &art("PRD-110", "prd", "Auth"),
        )
        .await
        .unwrap();

        add_link_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-110",
            "ADR-001@src:b7d6f0e1a2c3d4e5",
            "based_on",
        )
        .await
        .unwrap();

        let (fm, _) = read_doc(&ws, "PRD-110").await;
        assert_eq!(
            crate::link::list_links(&fm),
            vec![(
                "ADR-001@src:b7d6f0e1a2c3d4e5".to_string(),
                "based_on".to_string()
            )],
            "the qualified target must be stored exactly as given"
        );
    }

    /// `replace_link` returns the SAME outcome taxonomy on both layouts, so a
    /// CLI/MCP caller's printed result does not depend on which index owns
    /// the workspace.
    #[tokio::test]
    async fn replace_link_outcomes_match_the_store_path_without_a_sink() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        for id in ["PRD-111", "RFC-111"] {
            create_artifact_with_projection(
                &MutationContext::new(&ws, &lock, None),
                &art(id, if id.starts_with("RFC") { "rfc" } else { "prd" }, id),
            )
            .await
            .unwrap();
        }

        let outcome = replace_link_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-111",
            "RFC-111",
            "informs",
        )
        .await
        .unwrap();
        assert_eq!(outcome, LinkUpsertOutcome::Created);

        let outcome = replace_link_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-111",
            "RFC-111",
            "based_on",
        )
        .await
        .unwrap();
        assert_eq!(
            outcome,
            LinkUpsertOutcome::Replaced {
                old_relation: "informs".to_string()
            }
        );

        let outcome = replace_link_with_projection(
            &MutationContext::new(&ws, &lock, None),
            "PRD-111",
            "RFC-111",
            "based_on",
        )
        .await
        .unwrap();
        assert_eq!(outcome, LinkUpsertOutcome::Unchanged);
    }

    /// The batch helper returns the count of edges that actually landed and
    /// never aborts the whole batch on one bad edge — same as the store path.
    #[tokio::test]
    async fn the_link_batch_reports_applied_edges_without_a_sink() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        for id in ["PRD-112", "RFC-112", "ADR-112"] {
            create_artifact_with_projection(
                &MutationContext::new(&ws, &lock, None),
                &art(id, if id.starts_with("RFC") { "rfc" } else { "prd" }, id),
            )
            .await
            .unwrap();
        }

        let applied = add_links_batch_with_projection(
            &MutationContext::new(&ws, &lock, None),
            &[
                ("PRD-112".into(), "RFC-112".into(), "informs".into()),
                ("PRD-112".into(), "ADR-112".into(), "based_on".into()),
            ],
        )
        .await
        .unwrap();
        assert_eq!(applied, 2);

        let (fm, _) = read_doc(&ws, "PRD-112").await;
        assert_eq!(crate::link::list_links(&fm).len(), 2);

        // A duplicate (target, relation) in the same batch is refused by the
        // file helper and not counted; a same-target with a different relation
        // IS counted — `add_link_verbatim` is additive (a multi-edge to one
        // target is legal), it does not collapse the older `based_on` away.
        // The batch therefore lands exactly one edge here.
        let applied = add_links_batch_with_projection(
            &MutationContext::new(&ws, &lock, None),
            &[
                ("PRD-112".into(), "RFC-112".into(), "informs".into()),
                ("PRD-112".into(), "ADR-112".into(), "informs".into()),
            ],
        )
        .await
        .unwrap();
        assert_eq!(
            applied, 1,
            "RFC-112 informs is an exact duplicate (not counted); \
             ADR-112 informs is a new entry alongside the existing based_on"
        );

        let (fm, _) = read_doc(&ws, "PRD-112").await;
        assert_eq!(
            crate::link::list_links(&fm).len(),
            3,
            "RFC-112 informs + ADR-112 based_on + ADR-112 informs all survive"
        );
    }

    /// Validation happens before any file is touched — a malformed target in
    /// the batch rejects the whole call rather than half-applying it.
    #[tokio::test]
    async fn the_link_batch_rejects_a_traversal_target_before_writing() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, None),
            &art("PRD-113", "prd", "Auth"),
        )
        .await
        .unwrap();

        let err = add_links_batch_with_projection(
            &MutationContext::new(&ws, &lock, None),
            &[
                ("PRD-113".into(), "RFC-113".into(), "informs".into()),
                ("../../evil".into(), "PRD-113".into(), "informs".into()),
            ],
        )
        .await
        .expect_err("a traversal source must reject the batch");
        assert!(matches!(err, MutationError::InvalidId(_)));

        let (fm, _) = read_doc(&ws, "PRD-113").await;
        assert!(
            crate::link::list_links(&fm).is_empty(),
            "nothing may be written when validation rejects the batch"
        );
    }

    // ── delete ─────────────────────────────────────────────────────────────

    /// With no sink, delete resolves through the file-first resolver and
    /// removes the canonical file. Missing artifact stays an idempotent
    /// no-op, mirroring the missing-row policy of the store path.
    #[tokio::test]
    async fn delete_without_a_sink_removes_the_canonical_file() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, None),
            &art("PRD-114", "prd", "Auth"),
        )
        .await
        .unwrap();
        assert!(
            crate::artifact::store::resolve_artifact_path(&ws, "PRD-114")
                .await
                .is_some()
        );

        delete_artifact_with_projection(&MutationContext::new(&ws, &lock, None), "PRD-114")
            .await
            .unwrap();
        assert!(
            crate::artifact::store::resolve_artifact_path(&ws, "PRD-114")
                .await
                .is_none(),
            "the markdown source of truth must be gone"
        );

        // Idempotent second delete.
        assert!(
            delete_artifact_with_projection(&MutationContext::new(&ws, &lock, None), "PRD-114")
                .await
                .is_ok(),
            "delete must stay idempotent without a sink"
        );
    }

    /// An unresolvable id with no sink is `RowNotFound`, NOT a silent success
    /// — the caller must learn the mutation did not happen.
    #[tokio::test]
    async fn update_without_a_sink_reports_row_not_found_for_an_unknown_id() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        for result in [
            update_body_with_projection(&MutationContext::new(&ws, &lock, None), "PRD-999", "x")
                .await,
            update_metadata_with_projection(
                &MutationContext::new(&ws, &lock, None),
                "PRD-999",
                Some("draft"),
                None,
            )
            .await,
            add_tags_with_projection(
                &MutationContext::new(&ws, &lock, None),
                "PRD-999",
                &["x".to_string()],
            )
            .await,
        ] {
            assert!(
                matches!(result, Err(MutationError::RowNotFound { .. })),
                "an unknown id must surface RowNotFound, not a faked success"
            );
        }
    }

    /// The blank-field gates run before the file is opened, so an invalid
    /// field is rejected identically on both layouts.
    #[tokio::test]
    async fn blank_field_gates_still_apply_without_a_sink() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, None),
            &art("PRD-115", "prd", "Auth"),
        )
        .await
        .unwrap();

        assert!(matches!(
            update_metadata_with_projection(
                &MutationContext::new(&ws, &lock, None),
                "PRD-115",
                Some("   "),
                None,
            )
            .await,
            Err(MutationError::EmptyField { field: "status" })
        ));
        assert!(matches!(
            update_metadata_with_projection(
                &MutationContext::new(&ws, &lock, None),
                "PRD-115",
                None,
                Some(""),
            )
            .await,
            Err(MutationError::EmptyField { field: "title" })
        ));
        let (fm, _) = read_doc(&ws, "PRD-115").await;
        assert_eq!(fm_string(&fm, "status").as_deref(), Some("draft"));
    }

    // ── derived-index-only helpers report NoLegacySink ─────────────────────

    /// These helpers have no canonical counterpart, so with no sink they must
    /// say so rather than claim a row write they did not make.
    #[tokio::test]
    async fn derived_index_only_helpers_report_no_legacy_sink() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        let ctx = MutationContext::new(&ws, &lock, None);

        for err in [
            delete_orphan_artifact(&ctx, "PRD-116").await,
            delete_orphan_relation(&ctx, "PRD-116", "RFC-116", "informs").await,
            delete_artifact_after_soft_delete(&ctx, "PRD-116").await,
        ] {
            let err = err.expect_err("a derived-only step must not report success");
            assert!(
                matches!(err, MutationError::NoLegacySink { .. }),
                "expected NoLegacySink, got {err:?}"
            );
            assert!(
                !err.is_recoverable(),
                "retrying cannot conjure a legacy table"
            );
        }
    }

    /// Their input validation still runs first — a traversal id is rejected as
    /// `InvalidId`, not as "no sink", so the security boundary is unchanged.
    #[tokio::test]
    async fn derived_index_only_helpers_validate_before_reporting_no_sink() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        let ctx = MutationContext::new(&ws, &lock, None);

        assert!(matches!(
            delete_orphan_artifact(&ctx, "../../boom").await,
            Err(MutationError::InvalidId(_))
        ));
        assert!(matches!(
            delete_orphan_relation(&ctx, "PRD-001", "../bad", "informs").await,
            Err(MutationError::InvalidId(_))
        ));
        assert!(matches!(
            delete_artifact_after_soft_delete(&ctx, "..\\evil").await,
            Err(MutationError::InvalidId(_))
        ));
    }

    // ── file→index direction is a no-op, not a refusal ─────────────────────

    /// `reindex` / `git-sync` / `watch` call these with no sink on a migrated
    /// workspace. They must report the file as handled (Ok) rather than
    /// failing a pipeline that has nothing to write.
    #[tokio::test]
    async fn sync_from_file_helpers_are_no_ops_without_a_sink() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        let ctx = MutationContext::new(&ws, &lock, None);

        assert!(
            sync_artifact_from_file(&ctx, &art("PRD-117", "prd", "Auth"))
                .await
                .is_ok()
        );
        assert!(
            sync_body_from_file(&ctx, "PRD-117", "prd", "Auth", "body")
                .await
                .is_ok()
        );
        assert!(
            sync_metadata_from_file(&ctx, "PRD-117", Some("draft"), None)
                .await
                .is_ok()
        );
        assert!(
            sync_relation_from_file(&ctx, "PRD-117", "RFC-117", "informs")
                .await
                .is_ok()
        );
    }

    /// Their validation gates are unchanged — an invalid id is still refused
    /// even when there is no sink to write to.
    #[tokio::test]
    async fn sync_from_file_helpers_keep_validating_without_a_sink() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        let ctx = MutationContext::new(&ws, &lock, None);

        let mut evil = art("PRD-118", "prd", "Auth");
        evil.id = "../../etc/evil".to_string();
        assert!(matches!(
            sync_artifact_from_file(&ctx, &evil).await,
            Err(MutationError::InvalidId(_))
        ));
        assert!(matches!(
            sync_metadata_from_file(&ctx, "../../etc/passwd", Some("draft"), None).await,
            Err(MutationError::InvalidId(_))
        ));
        assert!(matches!(
            sync_relation_from_file(&ctx, "PRD-001", "../bad", "informs").await,
            Err(MutationError::InvalidId(_))
        ));
    }

    /// The `(None, None)` short-circuit and blank-field gates survive the
    /// cutover, so `git_sync`'s "only call sync when something changed"
    /// optimisation still behaves.
    #[tokio::test]
    async fn sync_metadata_keeps_its_gates_without_a_sink() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        let ctx = MutationContext::new(&ws, &lock, None);

        assert!(
            sync_metadata_from_file(&ctx, "PRD-119", None, None)
                .await
                .is_ok()
        );
        assert!(matches!(
            sync_metadata_from_file(&ctx, "PRD-119", Some(""), None).await,
            Err(MutationError::EmptyField { field: "status" })
        ));
    }

    // ── L02: a cleared body stays cleared ──────────────────────────────────

    /// The pre-sync resurrection L02 forbids.
    ///
    /// A user replaces a non-empty body with an empty one; the very next
    /// metadata-only mutation re-renders the file. Before this was fixed the
    /// renderer read the empty file body as "no body yet" and wrote the row's
    /// old text back — so the clear was undone by an unrelated command, and a
    /// search kept answering with content that no longer existed on disk.
    ///
    /// Asserted on the **file the user reads**, not on a helper's return value.
    #[tokio::test]
    async fn a_cleared_body_is_not_restored_by_the_next_mutation() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        let ctx = MutationContext::new(&ws, &lock, None);
        create_artifact_with_projection(&ctx, &art("PRD-120", "prd", "Auth"))
            .await
            .unwrap();

        let (_, before) = read_doc(&ws, "PRD-120").await;
        assert!(
            before.contains("Original body."),
            "the fixture must start with a non-empty body"
        );

        // The clear: frontmatter survives, body is gone.
        let path = crate::artifact::store::resolve_artifact_path(&ws, "PRD-120")
            .await
            .unwrap();
        let cleared =
            "---\nid: PRD-120\nkind: prd\nstatus: draft\ndepth: standard\ntitle: Auth\n---\n";
        tokio::fs::write(&path, cleared).await.unwrap();

        // An unrelated mutation on the same artifact.
        update_metadata_with_projection(&ctx, "PRD-120", Some("active"), None)
            .await
            .unwrap();

        let (fm, after) = read_doc(&ws, "PRD-120").await;
        assert_eq!(
            fm_string(&fm, "status").as_deref(),
            Some("active"),
            "the mutation's own effect must still land"
        );
        assert!(
            !after.contains("Original body."),
            "the pre-sync must not resurrect the cleared body, got {after:?}"
        );
        assert!(
            after.trim().is_empty(),
            "the body must still be empty after the mutation, got {after:?}"
        );
    }

    /// The same rule on the row-driven renderer, which is what `tag`, `link`
    /// and `activate` use after they reload the record. Both renderers must
    /// agree about an empty body, or a caller that switched between them
    /// would see the text come back.
    #[tokio::test]
    async fn the_record_renderer_also_treats_a_cleared_body_as_canonical() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art("PRD-121", "prd", "Auth"),
        )
        .await
        .unwrap();

        let path = crate::artifact::store::resolve_artifact_path(&ws, "PRD-121")
            .await
            .unwrap();
        tokio::fs::write(
            &path,
            "---\nid: PRD-121\nkind: prd\nstatus: draft\ndepth: standard\ntitle: Auth\n---\n",
        )
        .await
        .unwrap();

        // A reload that still holds the pre-clear body — exactly the stale row
        // a `link` handler passes after the user emptied the file.
        let mut record = store.get_record("PRD-121").await.unwrap().unwrap();
        record.body = "Original body.".to_string();

        render_projection_record(&ws, &record, &[]).await.unwrap();

        let (_, after) = read_doc(&ws, "PRD-121").await;
        assert!(
            !after.contains("Original body."),
            "the stale row body must not be written over an empty file body, got {after:?}"
        );
    }

    /// The forcing variants keep working: a caller that *does* mean to write a
    /// body — `update --body`, and the lifecycle sections appended by
    /// `deprecate` / `renew` / `reopen` — must still land it. Without this the
    /// L02 fix would silently break #478.
    #[tokio::test]
    async fn the_forcing_renderers_still_write_the_row_body_over_an_empty_file() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art("PRD-122", "prd", "Auth"),
        )
        .await
        .unwrap();

        let path = crate::artifact::store::resolve_artifact_path(&ws, "PRD-122")
            .await
            .unwrap();
        tokio::fs::write(
            &path,
            "---\nid: PRD-122\nkind: prd\nstatus: draft\ndepth: standard\ntitle: Auth\n---\n",
        )
        .await
        .unwrap();

        let mut record = store.get_record("PRD-122").await.unwrap().unwrap();
        record.body = "## Deprecation\n\nsuperseded by PRD-001".to_string();

        render_projection_record_with_body(&ws, &record, &[])
            .await
            .unwrap();
        let (_, after) = read_doc(&ws, "PRD-122").await;
        assert!(
            after.contains("superseded by PRD-001"),
            "the appended lifecycle section must still reach the file, got {after:?}"
        );
    }

    /// The counterpart direction: a cleared body is a real change, so the
    /// row follows the file rather than keeping the text. This is what makes
    /// the two renderers consistent instead of merely both permissive.
    #[tokio::test]
    async fn a_cleared_body_is_synced_into_the_row_as_an_empty_one() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        let store = crate::db::store::LanceStore::init(&ws).await.unwrap();
        create_artifact_with_projection(
            &MutationContext::new(&ws, &lock, Some(&store)),
            &art("PRD-123", "prd", "Auth"),
        )
        .await
        .unwrap();

        let path = crate::artifact::store::resolve_artifact_path(&ws, "PRD-123")
            .await
            .unwrap();
        tokio::fs::write(
            &path,
            "---\nid: PRD-123\nkind: prd\nstatus: draft\ndepth: standard\ntitle: Auth\n---\n",
        )
        .await
        .unwrap();

        let row = store.get_record("PRD-123").await.unwrap().unwrap();
        assert!(
            sync_file_to_store(&store, &ws, &row).await.unwrap(),
            "clearing the body is a change the row must observe"
        );
        let after = store.get_record("PRD-123").await.unwrap().unwrap();
        assert!(
            after.body.trim().is_empty(),
            "the row must carry the empty body, got {:?}",
            after.body
        );
    }

    #[tokio::test]
    async fn a_legacy_title_rename_preserves_edits_and_unrelated_siblings() {
        for body in ["Edited canonical body.", ""] {
            let (_tmp, ws, lock) = migrated_fixture().await;
            let store = crate::db::store::LanceStore::init(&ws).await.unwrap();
            let ctx = MutationContext::new(&ws, &lock, Some(&store));
            let original =
                create_artifact_with_projection(&ctx, &art("mem-foo", "memory", "Old title"))
                    .await
                    .unwrap();
            tokio::fs::write(
                &original,
                format!(
                    "---\nid: mem-foo\nkind: memory\nstatus: draft\ntitle: Old title\ndepth: standard\nassigned_number: 42\ncustom_owner: preserved\n---\n\n{body}"
                ),
            )
            .await
            .unwrap();
            let siblings = [
                (
                    "mem-foo-bar-foo-bar.md",
                    "---\nid: mem-foo-bar\n---\n\nOther artifact.",
                ),
                (
                    "mem-foo-manual.md",
                    "---\nid: mem-other\n---\n\nManual document.",
                ),
                (
                    "mem-foo-invalid.md",
                    "Unparseable source must remain untouched.",
                ),
            ];
            let dir = original.parent().unwrap();
            for (name, contents) in siblings {
                tokio::fs::write(dir.join(name), contents).await.unwrap();
            }

            update_metadata_with_projection(&ctx, "mem-foo", None, Some("New title"))
                .await
                .unwrap();

            assert!(!original.exists());
            let renamed = dir.join("mem-foo-new-title.md");
            let text = tokio::fs::read_to_string(renamed).await.unwrap();
            let (fm, actual_body) = frontmatter::parse_frontmatter(&text).unwrap();
            assert_eq!(actual_body.trim_end(), body);
            assert_eq!(fm["assigned_number"].as_u64(), Some(42));
            assert_eq!(fm["custom_owner"].as_str(), Some("preserved"));
            for (name, contents) in siblings {
                assert_eq!(
                    tokio::fs::read_to_string(dir.join(name)).await.unwrap(),
                    contents
                );
            }
            let row = store.get_record("mem-foo").await.unwrap().unwrap();
            assert_eq!(row.title, "New title");
            assert_eq!(row.body.trim_end(), body);
        }
    }

    #[tokio::test]
    async fn a_title_rename_refuses_an_existing_destination_without_overwriting_it() {
        let (_tmp, ws, lock) = migrated_fixture().await;
        let ctx = MutationContext::new(&ws, &lock, None);
        let original = create_artifact_with_projection(&ctx, &art("PRD-124", "prd", "Old title"))
            .await
            .unwrap();
        let destination = original.parent().unwrap().join("PRD-124-new-title.md");
        let other = "---\nid: PRD-999\n---\n\nDo not overwrite this artifact.";
        tokio::fs::write(&destination, other).await.unwrap();
        let before = tokio::fs::read(&original).await.unwrap();

        assert!(
            update_metadata_with_projection(&ctx, "PRD-124", None, Some("New title"))
                .await
                .is_err()
        );
        assert_eq!(tokio::fs::read(&original).await.unwrap(), before);
        assert_eq!(tokio::fs::read_to_string(destination).await.unwrap(), other);
    }
}
