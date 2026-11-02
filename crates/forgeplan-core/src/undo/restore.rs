//! Restore from a soft-delete receipt (PRD-055, increment 3).
//!
//! Given a receipt, reconstruct the artifact state it captured:
//! - restore canonical Markdown (move a Delete back, reset lifecycle metadata)
//! - synchronize the optional legacy row and replay captured relations
//! - mark the receipt consumed so undo-last doesn't re-apply it
//!
//! # Transactional semantics (PRD-055 FR-011)
//!
//! The receipt is not marked consumed until canonical restoration, optional
//! legacy synchronization, and relation replay have completed. An error can
//! leave some writes applied, but the receipt remains unconsumed; restore
//! never hides a failed write behind a successful recovery report.
//!
//! # Collision handling (PRD-055 R-3)
//!
//! If the canonical artifact or its legacy row already exists after Delete,
//! restore refuses so the operator can resolve the collision manually.

use super::{DestructiveOp, Receipt, receipt_path};
use crate::artifact::canonical_key::canonical_key_from_document;
use crate::db::store::{LanceStore, NewArtifact};
use crate::workspace::lock::{WorkspaceLock, WorkspaceReadPermit};
use anyhow::Context;
use std::path::{Path, PathBuf};

/// Outcome of a successful restore. Surfaces warnings about partial
/// issues (e.g. orphaned relation targets) so the caller can report them
/// back to the agent.
#[derive(Debug, Clone)]
pub struct RestoreReport {
    pub artifact_id: String,
    pub op: DestructiveOp,
    pub relations_restored: usize,
    pub relations_skipped: Vec<String>, // target IDs no longer in store
    pub projection_restored: bool,
    pub warnings: Vec<String>,
    /// Issue #291: the status value the artifact was restored to.
    /// Mirrors `receipt.snapshot.status` (or the explicit override from
    /// [`apply_restore_with_target`]). Surfaced in tool responses so the
    /// operator can confirm which state was applied without having to
    /// re-read the artifact.
    pub restored_status: String,
}

/// Restore the artifact captured in `receipt`. Returns an error if the
/// store still has a row with the same ID (collision), or if reading
/// any receipt field fails at apply time.
/// Issue #291: restore an artifact AND override the restored status.
///
/// Same as [`apply_restore`] but accepts a `target_status` override.
/// When `target_status` is `Some(s)`, the artifact is restored to `s`.
/// Without an override, Delete preserves the exact trashed Markdown bytes;
/// lifecycle reversals apply the captured `snapshot.status`.
///
/// Valid status values: `"draft"`, `"active"`, `"deprecated"`. Any other
/// value is rejected with an error so the caller can fix the request.
///
/// The receipt's snapshot is NOT mutated — only the in-flight restore
/// writes the override. Subsequent `forgeplan_activity` on this receipt
/// will still show the original captured status.
pub async fn apply_restore_with_target(
    workspace: &Path,
    guard: &WorkspaceLock,
    store: Option<&LanceStore>,
    receipt: &Receipt,
    target_status: Option<&str>,
) -> anyhow::Result<RestoreReport> {
    let normalized_target = normalize_target(target_status)?;
    apply_restore_with_sink(workspace, guard, store, receipt, normalized_target.as_deref()).await
}

fn normalize_target(target_status: Option<&str>) -> anyhow::Result<Option<String>> {
    let normalized = target_status.map(|value| value.trim().to_lowercase());
    if let Some(value) = &normalized
        && !matches!(value.as_str(), "draft" | "active" | "deprecated")
    {
        anyhow::bail!(
            "Invalid target_status `{}` — valid values: `draft`, `active`, `deprecated`.",
            target_status.unwrap_or("")
        );
    }
    Ok(normalized)
}

pub async fn apply_restore(
    workspace: &Path,
    guard: &WorkspaceLock,
    store: &LanceStore,
    receipt: &Receipt,
) -> anyhow::Result<RestoreReport> {
    apply_restore_with_sink(workspace, guard, Some(store), receipt, None).await
}

/// Restore canonical Markdown without a legacy derived sink.
///
/// This uses the same canonical writes, collision checks, relation replay, and
/// receipt-consumption barrier as [`apply_restore`]. It never opens a retired
/// legacy store.
pub async fn apply_restore_file_first(
    workspace: &Path,
    guard: &WorkspaceLock,
    receipt: &Receipt,
) -> anyhow::Result<RestoreReport> {
    apply_restore_with_sink(workspace, guard, None, receipt, None).await
}

/// Read-only restore validation before opening a legacy index for mutation.
///
/// The caller holds the workspace mutation lock across this check, index
/// migration and restore. Apply repeats these checks; the lock coordinates
/// participating writers, not arbitrary filesystem changes.
pub async fn preflight_restore_with_target(
    workspace: &Path,
    guard: &WorkspaceLock,
    receipt: &Receipt,
    target_status: Option<&str>,
) -> anyhow::Result<()> {
    guard.matches_workspace(workspace)?;
    let target_status = normalize_target(target_status)?;
    let context = super::safe_restore::Context::open(workspace, &receipt.receipt_id)?;
    // Check the receipt before opening even the read-only derived index.
    validate_receipt_header(workspace, receipt, &context)?;
    context.ensure_ready()?;
    let store = open_preflight_store(workspace).await?;
    let record = record_for_receipt(workspace, guard, &context, store.as_ref(), receipt).await?;
    preflight_checked(workspace, guard, &context, store.as_ref(), receipt, target_status.as_deref(), record.as_ref()).await
}

async fn record_for_receipt(
    workspace: &Path,
    guard: &WorkspaceLock,
    context: &super::safe_restore::Context,
    store: Option<&LanceStore>,
    receipt: &Receipt,
) -> anyhow::Result<Option<crate::db::store::ArtifactRecord>> {
    let Some(store) = store else { return Ok(None); };
    let canonical_key = receipt_canonical_key(receipt);
    if store.uses_identity_v5() {
        return store.get_record(guard, &canonical_key).await;
    }
    let record = store.legacy_record_by_display_id(guard, &receipt.snapshot.id).await?;
    if let Some(row) = record.as_ref() {
        let row_key = canonical_key_from_document(&row.id, &row.body);
        if row_key != canonical_key {
            if row_key != row.id {
                anyhow::bail!("legacy index row and receipt claim different canonical keys");
            }
            let path = super::safe_restore::resolve(context, workspace, &receipt.snapshot.id)?
                .ok_or_else(|| anyhow::anyhow!("legacy index row cannot be bound unambiguously to the receipt's canonical key"))?;
            let (directory, name) = document_parts(workspace, &path)?;
            let document = context.read_document(directory, name)?;
            let text = std::str::from_utf8(&document.bytes)?;
            if canonical_key_from_document(&receipt.snapshot.id, text) != canonical_key {
                anyhow::bail!("legacy index row maps to a different canonical document");
            }
        }
    }
    Ok(record)
}

/// Open only an already existing, complete legacy derived plane. This relies
/// on LanceStore::open being the read-only API; open_for_write is prohibited.
async fn open_preflight_store(workspace: &Path) -> anyhow::Result<Option<LanceStore>> {
    if crate::constellation::facade::workspace_is_migrated(workspace) {
        return Ok(None);
    }
    let lance = workspace.join("lance");
    match tokio::fs::symlink_metadata(&lance).await {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
            anyhow::bail!("legacy index root is not a real directory");
        }
        Ok(_) => {}
    }
    let mut old_tables = 0;
    for name in ["artifacts.lance", "evidence.lance", "relations.lance"] {
        match tokio::fs::symlink_metadata(lance.join(name)).await {
            Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => old_tables += 1,
            Ok(_) => anyhow::bail!("legacy index table {name} is not a real directory"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let marker = match tokio::fs::symlink_metadata(lance.join("IDENTITY_V5.json")).await {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => true,
        Ok(_) => anyhow::bail!("identity-v5 marker is not a regular file"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.into()),
    };
    if marker {
        for name in ["artifacts_identity_v5.lance", "evidence_identity_v5.lance", "relations_identity_v5.lance"] {
            let meta = tokio::fs::symlink_metadata(lance.join(name)).await
                .with_context(|| format!("identity-v5 table {name} is missing or unreadable"))?;
            if !meta.is_dir() || meta.file_type().is_symlink() {
                anyhow::bail!("identity-v5 table {name} is not a real directory");
            }
        }
        return Ok(Some(LanceStore::open(workspace).await?));
    }
    if old_tables == 3 {
        return Ok(Some(LanceStore::open(workspace).await?));
    }
    if old_tables != 0 {
        anyhow::bail!("legacy index is incomplete; refusing restore before migration");
    }
    for name in ["artifacts_identity_v5.lance", "evidence_identity_v5.lance", "relations_identity_v5.lance"] {
        match tokio::fs::symlink_metadata(lance.join(name)).await {
            Ok(_) => anyhow::bail!("identity-v5 staging tables exist without a complete legacy index"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(None)
}

async fn preflight_checked(
    workspace: &Path,
    guard: &WorkspaceLock,
    context: &super::safe_restore::Context,
    sink: Option<&LanceStore>,
    receipt: &Receipt,
    target_status: Option<&str>,
    record: Option<&crate::db::store::ArtifactRecord>,
) -> anyhow::Result<()> {
    validate_receipt_header(workspace, receipt, context)?;
    collision_before_missing_trash(workspace, context, receipt, record.is_some())?;
    let source = validate_receipt(workspace, receipt, context)?;
    context.ensure_ready()?;
    let key = receipt_canonical_key(receipt);
    let existing = super::safe_restore::resolve(context, workspace, &key)?;
    let mut current_status = None;
    let mut current_kind = None;
    let mut current_document = None;
    if let Some(path) = existing.as_deref() {
        let (dir, name) = document_parts(workspace, path)?;
        let document = context.read_document(dir, name)?;
        let text = std::str::from_utf8(&document.bytes)?;
        if receipt.op != DestructiveOp::Delete {
            validate_document_identity(receipt, text)?;
        }
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(text)?;
        current_status = fm.get("status").and_then(|v| v.as_str()).map(str::to_owned);
        current_kind = fm.get("kind").and_then(|v| v.as_str()).map(str::to_owned);
        current_document = Some(document);
    }
    if let Some(target) = target_status
        && let Some(current) = current_status.as_deref().or_else(|| record.map(|r| r.status.as_str()))
        && let Err(error) = crate::lifecycle::transitions::validate_transition(current, target)
    {
        anyhow::bail!(
            "target_status override `{target}` would violate lifecycle FSM from current status `{current}`: {error}. Use the canonical lifecycle command instead."
        );
    }
    for kind in current_kind.as_deref().into_iter().chain(record.map(|r| r.kind.as_str())) {
        if receipt.op != DestructiveOp::Delete && !kind.is_empty() && kind != receipt.snapshot.kind {
            anyhow::bail!("Artifact '{}' has kind '{kind}' but receipt captured '{}' — refusing to overwrite. Resolve manually.", receipt.snapshot.id, receipt.snapshot.kind);
        }
    }
    let target = compute_safe_projection_path(workspace, &receipt.snapshot)?;
    let (kind, name) = document_parts(workspace, &target)?;
    let restored_status = target_status.unwrap_or(&receipt.snapshot.status);
    let bytes = if let Some(bytes) = source {
        if let Some(status) = target_status { rewrite_status(&bytes, status)? } else { bytes }
    } else {
        snapshot_content(receipt, restored_status)?.into_bytes()
    };
    let recovered = context.completed_target(kind, name, &bytes)?;
    if receipt.op == DestructiveOp::Delete && (existing.is_some() || record.is_some()) && !recovered {
        anyhow::bail!("Artifact '{}' already exists — cannot restore over it.", receipt.snapshot.id);
    }
    if receipt.op == DestructiveOp::Delete || existing.is_none() || recovered {
        context.check_create(kind, name, &bytes)?;
    } else if let Some(document) = current_document.as_ref() {
        rewrite_metadata(&document.bytes, restored_status, Some(&receipt.snapshot.title))?;
    }
    for relation in &receipt.snapshot.relations {
        let other = match relation.direction {
            super::RelationDirection::Outgoing => relation.to.as_str(),
            super::RelationDirection::Incoming => relation.from.as_str(),
        };
        if participant_exists(guard, context, sink, workspace, other).await? {
            crate::db::store::validate_link_target(&relation.to)?;
            if !relation.from.eq_ignore_ascii_case(&receipt.snapshot.id)
                && !relation.from.eq_ignore_ascii_case(&key)
                && super::safe_restore::resolve(context, workspace, &relation.from)?.is_none()
            {
                anyhow::bail!("source artifact '{}' not found", relation.from);
            }
        }
    }
    Ok(())
}

/// The one restore implementation both layouts share. Canonical Markdown is
/// restored first; `sink` only adds legacy collision checks and derived writes.
async fn apply_restore_with_sink(
    workspace: &Path,
    guard: &WorkspaceLock,
    sink: Option<&LanceStore>,
    receipt: &Receipt,
    target_status: Option<&str>,
) -> anyhow::Result<RestoreReport> {
    guard.matches_workspace(workspace)?;
    let context = super::safe_restore::Context::open(workspace, &receipt.receipt_id)?;
    if receipt.op == DestructiveOp::Delete && receipt.trashed_projection.is_empty() {
        validate_receipt_header(workspace, receipt, &context)?;
        context.ensure_ready()?;
        let record_exists = record_for_receipt(workspace, guard, &context, sink, receipt).await?.is_some();
        collision_before_missing_trash(workspace, &context, receipt, record_exists)?;
    }
    let source = validate_receipt(workspace, receipt, &context)?;
    context.ensure_ready()?;
    let id = &receipt.snapshot.id;
    let canonical_key = receipt_canonical_key(receipt);
    let restored_status = target_status.unwrap_or(&receipt.snapshot.status);
    let target_path = compute_safe_projection_path(workspace, &receipt.snapshot)?;
    let kind_dir = target_path.parent().and_then(Path::file_name).and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("invalid kind directory"))?;
    let leaf = target_path.file_name().and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("invalid projection filename"))?;
    let new_content = if let Some(bytes) = source {
        if let Some(status) = target_status { rewrite_status(&bytes, status)? } else { bytes }
    } else {
        snapshot_content(receipt, restored_status)?.into_bytes()
    };
    let recovered = context.completed_target(kind_dir, leaf, &new_content)?;
    let existing_path = super::safe_restore::resolve(&context, workspace, &canonical_key)?;
    let existing_document = if receipt.op != DestructiveOp::Delete {
        if let Some(path) = existing_path.as_deref() {
            let (directory, name) = document_parts(workspace, path)?;
            let document = context.read_document(directory, name)?;
            validate_document_identity(receipt, std::str::from_utf8(&document.bytes)?)?;
            Some(document)
        } else { None }
    } else { None };
    let existing_record = record_for_receipt(workspace, guard, &context, sink, receipt).await?;
    preflight_checked(workspace, guard, &context, sink, receipt, target_status, existing_record.as_ref()).await?;

    // A stale or absent derived row must not let Delete overwrite a live file.
    if receipt.op == DestructiveOp::Delete && (existing_path.is_some() || existing_record.is_some()) && !recovered
    {
        anyhow::bail!(
            "Artifact '{id}' already exists — cannot restore over it. \
             Delete or rename the current `{id}` first."
        );
    }
    if matches!(
        receipt.op,
        DestructiveOp::Supersede | DestructiveOp::Deprecate
    ) {
        let canonical_kind = match existing_document.as_ref() {
            Some(document) => {
                let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(std::str::from_utf8(&document.bytes)?)?;
                fm.get("kind").and_then(|v| v.as_str()).map(str::to_owned)
            }
            None => None,
        };
        // Kind is structural; title changes are legitimate and are reversed
        // below. Preserve the legacy kind-collision refusal as well.
        for kind in canonical_kind
            .as_deref()
            .into_iter()
            .chain(existing_record.as_ref().map(|record| record.kind.as_str()))
        {
            if !kind.is_empty() && kind != receipt.snapshot.kind {
                anyhow::bail!(
                    "Artifact '{id}' has kind '{kind}' but receipt captured '{}' — \
                     refusing to overwrite. Resolve manually.",
                    receipt.snapshot.kind
                );
            }
        }
    }

    // Canonical restoration is mandatory regardless of the optional sink.
    let path = match receipt.op {
        DestructiveOp::Delete => {
            context.create(kind_dir, leaf, &new_content)?;
            target_path.clone()
        }
        DestructiveOp::Supersede | DestructiveOp::Deprecate => match existing_path {
            Some(_) if recovered => target_path.clone(),
            Some(path) => {
                let (directory, name) = document_parts(workspace, &path)?;
                let document = existing_document.as_ref().ok_or_else(|| anyhow::anyhow!("missing checked document"))?;
                let updated = rewrite_metadata(&document.bytes, restored_status, Some(&receipt.snapshot.title))?;
                context.replace_document(directory, name, document, &updated)?;
                path
            }
            None => {
                context.create(kind_dir, leaf, &new_content)?;
                target_path.clone()
            }
        },
    };
    if receipt.op == DestructiveOp::Supersede
        && let Some(replacement) = &receipt.replacement
    {
        update_link(&context, workspace, &path, &canonical_key, replacement, "supersedes", false)
            .with_context(|| format!("could not drop supersede link {id}→{replacement}"))?;
    }

    // No store-to-file rendering here: it would replace the canonical bytes
    // just restored from trash, including fields the receipt does not capture.
    if let Some(store) = sink {
        if existing_record.is_some() {
            store
                .update_artifact(guard, &canonical_key, Some(restored_status), Some(&receipt.snapshot.title))
                .await?;
        } else {
            store
                .create_artifact(guard, &snapshot_artifact(receipt, restored_status))
                .await?;
        }
        if receipt.op == DestructiveOp::Supersede
            && let Some(replacement) = &receipt.replacement
        {
            store
                .delete_relation(guard, &canonical_key, replacement, "supersedes")
                .await
                .with_context(|| {
                    format!("could not drop legacy supersede link {id}→{replacement}")
                })?;
        }
    }

    let mut relations_restored = 0usize;
    let mut relations_skipped = Vec::new();
    for rel in &receipt.snapshot.relations {
        let (source, target) = (rel.from.as_str(), rel.to.as_str());
        let other_id = match rel.direction {
            super::RelationDirection::Outgoing => target,
            super::RelationDirection::Incoming => source,
        };
        // Missing participants are the only explicit skip policy. Parse,
        // permission, and write errors must leave the receipt unconsumed.
        if !participant_exists(guard, &context, sink, workspace, other_id).await? {
            relations_skipped.push(other_id.to_string());
            continue;
        }
        replay_relation(guard, &context, sink, workspace, source, target, &rel.relation)
            .await
            .with_context(|| format!("could not restore relation {source}→{target}"))?;
        relations_restored += 1;
    }

    context.consume().map_err(|error| {
        if error.is::<super::CompletionDurabilityUncertain>() {
            error
        } else {
            error.context(format!("restore for {} was applied but completion failed; preserve the receipt and restore markers", receipt.receipt_id))
        }
    })?;

    Ok(RestoreReport {
        artifact_id: id.clone(),
        op: receipt.op,
        relations_restored,
        relations_skipped,
        projection_restored: true,
        warnings: Vec::new(),
        restored_status: restored_status.to_string(),
    })
}

/// The `NewArtifact` a receipt's snapshot describes.
///
/// Tags are deliberately empty, matching the store path: the snapshot does
/// not capture them (they live in the projection frontmatter the trashed
/// file carries), and inventing them would claim a derivation that never
/// happened.
fn snapshot_artifact(receipt: &Receipt, status: &str) -> NewArtifact {
    NewArtifact {
        canonical_key: receipt_canonical_key(receipt),
        id: receipt.snapshot.id.clone(),
        kind: receipt.snapshot.kind.clone(),
        status: status.to_string(),
        title: receipt.snapshot.title.clone(),
        body: receipt.snapshot.body.clone(),
        depth: receipt.snapshot.depth.clone(),
        author: receipt.snapshot.author.clone(),
        parent_epic: receipt.snapshot.parent_epic.clone(),
        valid_until: receipt.snapshot.valid_until.clone(),
        tags: Vec::new(),
    }
}

fn receipt_canonical_key(receipt: &Receipt) -> String {
    receipt.snapshot.canonical_key.clone()
        .or_else(|| receipt.snapshot.slug.as_ref().filter(|slug| crate::artifact::types::is_canonical_key(slug)).cloned())
        .unwrap_or_else(|| canonical_key_from_document(&receipt.snapshot.id, &receipt.snapshot.body))
}

/// Validate the receipt itself without opening its optional trashed body.
fn validate_receipt_header(
    workspace: &Path,
    receipt: &Receipt,
    context: &super::safe_restore::Context,
) -> anyhow::Result<()> {
    let stored: Receipt = serde_json::from_slice(&context.receipt_bytes)?;
    if stored.receipt_id != receipt.receipt_id
        || serde_json::to_value(&stored)? != serde_json::to_value(receipt)?
    {
        anyhow::bail!("receipt does not match the checked receipt on disk");
    }
    if receipt.consumed || context.consumed()? {
        anyhow::bail!("receipt {} is already consumed", receipt.receipt_id);
    }
    compute_safe_projection_path(workspace, &receipt.snapshot)?;
    if let (Some(key), Some(slug)) = (&receipt.snapshot.canonical_key, &receipt.snapshot.slug)
        && crate::artifact::types::is_canonical_key(slug) && key != slug
    {
        anyhow::bail!("receipt canonical key and historical slug conflict");
    }
    if receipt.snapshot.body.trim_start().starts_with("---") {
        validate_document_identity(receipt, &receipt.snapshot.body)?;
    }
    Ok(())
}

fn collision_before_missing_trash(
    workspace: &Path,
    context: &super::safe_restore::Context,
    receipt: &Receipt,
    record_exists: bool,
) -> anyhow::Result<()> {
    if receipt.op == DestructiveOp::Delete && receipt.trashed_projection.is_empty() {
        let existing = super::safe_restore::resolve(context, workspace, &receipt_canonical_key(receipt))?;
        if existing.is_some() || record_exists {
            anyhow::bail!("Artifact '{}' already exists — cannot restore over it.", receipt.snapshot.id);
        }
    }
    Ok(())
}

/// Load the source through pinned, no-follow directory handles.
fn validate_receipt(
    workspace: &Path,
    receipt: &Receipt,
    context: &super::safe_restore::Context,
) -> anyhow::Result<Option<Vec<u8>>> {
    validate_receipt_header(workspace, receipt, context)?;
    if receipt.op != DestructiveOp::Delete {
        return Ok(None);
    }
    if receipt.trashed_projection.is_empty() {
        anyhow::bail!("No trashed projection exists for '{}'; the receipt remains available.", receipt.snapshot.id);
    }
    let recorded = Path::new(&receipt.trashed_projection);
    let paired = format!("body-{}.md", receipt.receipt_id);
    if recorded.components().any(|part| matches!(part, std::path::Component::ParentDir))
        || recorded.file_name().and_then(|s| s.to_str()) != Some(paired.as_str())
        || recorded.parent().and_then(Path::file_name).and_then(|s| s.to_str()) != Some("trash")
    {
        anyhow::bail!("receipt trash path does not match its paired body");
    }
    let bytes = context.body().context("cannot open paired trashed projection")?;
    let content = std::str::from_utf8(&bytes).context("trashed projection is not UTF-8")?;
    validate_document_identity(receipt, content)?;
    if !content.trim_start().starts_with("---") && content != receipt.snapshot.body {
        anyhow::bail!("raw trashed body differs from the receipt snapshot");
    }
    Ok(Some(bytes))
}

fn validate_document_identity(receipt: &Receipt, content: &str) -> anyhow::Result<()> {
    let snapshot = &receipt.snapshot;
    let derived_key = canonical_key_from_document(&snapshot.id, &snapshot.body);
    let key = snapshot.canonical_key.as_deref()
        .or(snapshot.slug.as_deref().filter(|slug| crate::artifact::types::is_canonical_key(slug)))
        .unwrap_or(&derived_key);
    if content.trim_start().starts_with("---") {
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(content)?;
        if crate::artifact::frontmatter::declared_canonical_keys(content)?.iter()
            .any(|declared| crate::artifact::types::is_canonical_key(declared) && declared != key) {
            anyhow::bail!("document declares conflicting canonical keys");
        }
        if fm.get("id").and_then(|v| v.as_str()).is_some_and(|id| id != snapshot.id)
            || fm.get("kind").and_then(|v| v.as_str()).is_some_and(|kind| kind != snapshot.kind)
            || canonical_key_from_document(&snapshot.id, content) != key
        {
            anyhow::bail!("receipt and document identity do not match");
        }
    } else if key != snapshot.id {
        anyhow::bail!("document does not declare the receipt's canonical key");
    }
    Ok(())
}


fn rewrite_status(bytes: &[u8], status: &str) -> anyhow::Result<Vec<u8>> {
    rewrite_metadata(bytes, status, None)
}

fn rewrite_metadata(bytes: &[u8], status: &str, title: Option<&str>) -> anyhow::Result<Vec<u8>> {
    let text = std::str::from_utf8(bytes)?;
    let (mut fm, _) = crate::artifact::frontmatter::parse_frontmatter(text)?;
    if fm.get("status").and_then(|v| v.as_str()) == Some(status)
        && title.is_none_or(|value| fm.get("title").and_then(|v| v.as_str()) == Some(value))
    {
        return Ok(bytes.to_vec());
    }
    fm.insert("status".into(), serde_yaml::Value::String(status.into()));
    if let Some(title) = title {
        fm.insert("title".into(), serde_yaml::Value::String(title.into()));
    }
    render_with_original_body(text, &fm)
}

fn render_with_original_body(text: &str, fm: &crate::artifact::frontmatter::Frontmatter) -> anyhow::Result<Vec<u8>> {
    let document = text.trim_start();
    let end = document[3..].find("\n---")
        .ok_or_else(|| anyhow::anyhow!("document has no closing frontmatter delimiter"))?;
    let prefix_len = text.len() - document.len();
    Ok(format!("{}---\n{}---{}", &text[..prefix_len], serde_yaml::to_string(fm)?,
        &document[3 + end + 4..]).into_bytes())
}

/// Re-render a receipt's snapshot into its canonical projection file.
///
/// Used when a status-change receipt's artifact is gone from the workspace
/// entirely: there is no trashed body to move back (supersede/deprecate never
/// trash one), so the canonical file is written from the captured fields.
/// Every value comes from the receipt; nothing is inferred.
fn snapshot_content(receipt: &Receipt, status: &str) -> anyhow::Result<String> {
    let snapshot = &receipt.snapshot;
    let content = if snapshot.body.trim_start().starts_with("---") {
        validate_document_identity(receipt, &snapshot.body)?;
        let (mut fm, _) = crate::artifact::frontmatter::parse_frontmatter(&snapshot.body)?;
        if fm.get("status").and_then(|value| value.as_str()) == Some(status) {
            snapshot.body.clone()
        } else {
            fm.insert("status".into(), serde_yaml::Value::String(status.into()));
            let document = snapshot.body.trim_start();
            let end = document[3..].find("\n---")
                .ok_or_else(|| anyhow::anyhow!("snapshot has no closing frontmatter delimiter"))?;
            let prefix_len = snapshot.body.len() - document.len();
            format!("{}---\n{}---{}", &snapshot.body[..prefix_len],
                serde_yaml::to_string(&fm)?, &document[3 + end + 4..])
        }
    } else {
        let mut fm = crate::artifact::frontmatter::Frontmatter::new();
        for (name, value) in [
            ("id", snapshot.id.as_str()),
            ("kind", snapshot.kind.as_str()),
            ("status", status),
            ("title", snapshot.title.as_str()),
            ("depth", snapshot.depth.as_str()),
        ] {
            fm.insert(name.into(), serde_yaml::Value::String(value.into()));
        }
        for (name, value) in [
            ("author", snapshot.author.as_deref()),
            ("parent_epic", snapshot.parent_epic.as_deref()),
            ("valid_until", snapshot.valid_until.as_deref()),
            ("slug", snapshot.canonical_key.as_deref().or(snapshot.slug.as_deref())),
        ] {
            if let Some(value) = value {
                fm.insert(name.into(), serde_yaml::Value::String(value.into()));
            }
        }
        crate::artifact::frontmatter::render_frontmatter(&fm, &snapshot.body)?
    };
    Ok(content)
}

#[cfg(test)]
async fn render_snapshot_projection(workspace: &Path, receipt: &Receipt, status: &str) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;
    let target = compute_safe_projection_path(workspace, &receipt.snapshot)?;
    ensure_restore_target_is_free(receipt, &target).await?;
    let content = snapshot_content(receipt, status)?;
    if let Some(parent) = target.parent() { tokio::fs::create_dir_all(parent).await?; }
    let mut file = tokio::fs::OpenOptions::new().write(true).create_new(true).open(&target).await?;
    file.write_all(content.as_bytes()).await?;
    file.sync_all().await?;
    Ok(())
}

/// A restore is create-only, including when the existing file claims the
/// same key. An occupied path is never proof that overwriting is safe.
async fn ensure_restore_target_is_free(receipt: &Receipt, target: &Path) -> anyhow::Result<()> {
    match tokio::fs::symlink_metadata(target).await {
        Ok(_) => anyhow::bail!(
            "refusing to restore '{}': {} already holds a different canonical document or occupied path",
            receipt.snapshot.id,
            target.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Canonical participants count even if the derived row is stale or absent.
/// A legacy-only row also counts, so a missing SOURCE file becomes a replay
/// error rather than silently discarding a known participant's relation.
async fn participant_exists(
    guard: &WorkspaceLock,
    context: &super::safe_restore::Context,
    sink: Option<&LanceStore>,
    workspace: &Path,
    id: &str,
) -> anyhow::Result<bool> {
    if super::safe_restore::resolve(context, workspace, id)?.is_some()
    {
        return Ok(true);
    }
    match sink {
        Some(store) => Ok(store.get_record(guard, id).await?.is_some()),
        None => Ok(false),
    }
}

fn document_parts<'a>(workspace: &Path, path: &'a Path) -> anyhow::Result<(&'a str, &'a str)> {
    use std::path::Component;
    let relative = path.strip_prefix(workspace).context("document path is outside workspace")?;
    let mut parts = relative.components();
    match (parts.next(), parts.next(), parts.next()) {
        (Some(Component::Normal(dir)), Some(Component::Normal(name)), None) => Ok((
            dir.to_str().ok_or_else(|| anyhow::anyhow!("invalid document directory"))?,
            name.to_str().ok_or_else(|| anyhow::anyhow!("invalid document filename"))?,
        )),
        _ => anyhow::bail!("document path is not a direct kind-directory child"),
    }
}

fn update_link(
    context: &super::safe_restore::Context,
    workspace: &Path,
    path: &Path,
    expected_source: &str,
    target: &str,
    relation: &str,
    add: bool,
) -> anyhow::Result<()> {
    if add { crate::db::store::validate_link_target(target)?; }
    let (dir, leaf) = document_parts(workspace, path)?;
    let document = context.read_document(dir, leaf)?;
    let text = std::str::from_utf8(&document.bytes)?;
    let (mut fm, _) = crate::artifact::frontmatter::parse_frontmatter(text)?;
    let id = fm.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let key = canonical_key_from_document(id, text);
    if !key.eq_ignore_ascii_case(expected_source) && !id.eq_ignore_ascii_case(expected_source) {
        anyhow::bail!("relation source identity changed before update");
    }
    if add && id.eq_ignore_ascii_case(target) {
        anyhow::bail!("Self-link not allowed: {id} cannot link to itself");
    }
    if !add && !fm.contains_key("links") { return Ok(()); }
    let links = fm.entry("links".into()).or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));
    let serde_yaml::Value::Sequence(seq) = links else {
        return Err(crate::error::ForgeplanError::Frontmatter("'links' field is not a sequence".into()).into());
    };
    let matches_link = |entry: &serde_yaml::Value| {
        let serde_yaml::Value::Mapping(map) = entry else { return false; };
        map.get(serde_yaml::Value::String("target".into())).and_then(|v| v.as_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(target))
            && map.get(serde_yaml::Value::String("relation".into())).and_then(|v| v.as_str()) == Some(relation)
    };
    if add {
        if seq.iter().any(matches_link) { return Ok(()); }
        let mut entry = serde_yaml::Mapping::new();
        entry.insert(serde_yaml::Value::String("target".into()), serde_yaml::Value::String(target.into()));
        entry.insert(serde_yaml::Value::String("relation".into()), serde_yaml::Value::String(relation.into()));
        seq.push(serde_yaml::Value::Mapping(entry));
    } else {
        let before = seq.len();
        seq.retain(|entry| !matches_link(entry));
        if before == seq.len() { return Ok(()); }
    }
    let updated = render_with_original_body(text, &fm)?;
    context.replace_document(dir, leaf, &document, &updated)
}

/// Re-add an edge to its canonical SOURCE file, then the optional legacy sink.
///
/// Only the typed `LinkExists` error is a canonical no-op. Legacy duplicates
/// are confirmed by reading the existing triple, not by matching error text.
async fn replay_relation(
    guard: &WorkspaceLock,
    context: &super::safe_restore::Context,
    sink: Option<&LanceStore>,
    workspace: &Path,
    source: &str,
    target: &str,
    relation: &str,
) -> anyhow::Result<()> {
    let path = super::safe_restore::resolve(context, workspace, source)?
        .ok_or_else(|| anyhow::anyhow!("source artifact '{source}' not found"))?;
    update_link(context, workspace, &path, source, target, relation, true)?;
    if let Some(store) = sink {
        let duplicate = store
            .get_relations(guard, source)
            .await?
            .iter()
            .any(|(t, r)| t.eq_ignore_ascii_case(target) && r == relation);
        if !duplicate {
            store.add_relation(guard, source, target, relation).await?;
        }
    }
    Ok(())
}

/// Validate and retain the historical projection path recorded by the receipt.
fn compute_safe_projection_path(
    workspace: &Path,
    snapshot: &super::ArtifactSnapshot,
) -> anyhow::Result<PathBuf> {
    use crate::artifact::types::ArtifactKind;
    let kind: ArtifactKind = snapshot
        .kind
        .parse()
        .map_err(|e| anyhow::anyhow!("receipt has unknown kind '{}': {e}", snapshot.kind))?;
    let filename = if snapshot.projection_path.is_empty() {
        let slug = crate::projection::projection_slug(&snapshot.title);
        let stem = format!("{}-{}", snapshot.id, slug);
        crate::artifact::store::canonical_filename(&stem)?
    } else {
        recorded_file_name(snapshot, &workspace.join(kind.dir_name()))
            .ok_or_else(|| anyhow::anyhow!("receipt projection path is outside its kind directory or invalid"))?
    };
    let candidate = workspace.join(kind.dir_name()).join(&filename);

    // Canonical check: resolved path must live under workspace.
    // Workspace may not yet exist (fresh init); fall back to simple
    // component-walk if canonicalize fails.
    match (candidate.canonicalize(), workspace.canonicalize()) {
        (Ok(resolved), Ok(ws_canon)) => {
            if !resolved.starts_with(&ws_canon) {
                anyhow::bail!(
                    "computed projection path {} escapes workspace {}",
                    resolved.display(),
                    ws_canon.display()
                );
            }
        }
        _ => {
            // Couldn't canonicalize (parent dir may not exist yet).
            // Walk components and reject `..` / absolute root segments
            // beyond workspace prefix.
            for comp in candidate.components() {
                if matches!(comp, std::path::Component::ParentDir) {
                    anyhow::bail!(
                        "candidate path {} contains `..` — refusing",
                        candidate.display()
                    );
                }
            }
        }
    }
    Ok(candidate)
}

/// Retain a safe basename from an old or current workspace's kind directory.
fn recorded_file_name(snapshot: &super::ArtifactSnapshot, expected_dir: &Path) -> Option<String> {
    let recorded = Path::new(&snapshot.projection_path);
    if recorded.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return None;
    }
    let parent = recorded.parent()?;
    if parent.file_name()? != expected_dir.file_name()?
    {
        return None;
    }
    let name = recorded.file_name()?.to_str()?;
    if !name.ends_with(".md") {
        return None;
    }
    Some(name.to_string())
}

/// Read a receipt by ID and apply_restore. Convenience wrapper.
pub async fn apply_restore_by_id(
    workspace: &Path,
    guard: &WorkspaceLock,
    store: &LanceStore,
    receipt_id: &str,
) -> anyhow::Result<RestoreReport> {
    guard.matches_workspace(workspace)?;
    if receipt_id.is_empty()
        || !receipt_id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        anyhow::bail!("invalid receipt id");
    }
    let path = receipt_path(workspace, receipt_id);
    let receipt = super::read_receipt(&path).await?;
    if receipt.receipt_id != receipt_id {
        anyhow::bail!("receipt id does not match its filename");
    }
    if receipt.consumed {
        anyhow::bail!("receipt {receipt_id} is already consumed");
    }
    apply_restore(workspace, guard, store, &receipt).await
}

#[cfg(test)]
mod tests {
    use super::super::{
        ArtifactSnapshot, CapturedRelation, DestructiveOp, Receipt, RelationDirection,
        generate_receipt_id, trashed_projection_path, write_receipt,
    };
    use super::*;
    use crate::workspace;
    use tempfile::TempDir;

    async fn fresh_ws() -> (TempDir, std::path::PathBuf, LanceStore, crate::workspace::lock::WorkspaceLock) {
        let tmp = TempDir::new().unwrap();
        let ws = workspace::init_workspace(tmp.path(), "rt-test").unwrap();
        let guard = workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = LanceStore::init_new(&ws, &guard).await.unwrap();
        (tmp, ws, store, guard)
    }

    fn build_receipt(id: &str, op: DestructiveOp, body: &str) -> Receipt {
        let rid = generate_receipt_id("prd", id);
        Receipt {
            receipt_id: rid.clone(),
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            op,
            snapshot: ArtifactSnapshot {
                id: id.into(),
                kind: "prd".into(),
                status: "active".into(),
                title: format!("Title {id}"),
                depth: "standard".into(),
                body: body.into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                relations: vec![],
                projection_path: String::new(),
                slug: None,
                canonical_key: None,
            },
            reason: None,
            replacement: None,
            trashed_projection: String::new(),
            activity_log_hash: None,
            consumed: false,
        }
    }

    async fn stage_deleted_projection(workspace: &Path, receipt: &mut Receipt) {
        render_snapshot_projection(workspace, receipt, &receipt.snapshot.status)
            .await
            .unwrap();
        let original = compute_safe_projection_path(workspace, &receipt.snapshot).unwrap();
        let trashed = trashed_projection_path(workspace, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::rename(&original, &trashed).await.unwrap();
        receipt.snapshot.projection_path = original.display().to_string();
        receipt.trashed_projection = trashed.display().to_string();
    }

    #[tokio::test]
    async fn preflight_is_read_only_and_rechecks_destination() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace::init_workspace(tmp.path(), "restore-preflight").unwrap();
        let guard = workspace::acquire_workspace_lock(&ws).await.unwrap();
        let mut receipt = build_receipt("PRD-721", DestructiveOp::Deprecate, "original");
        let target = ws.join("prds/historical.md");
        receipt.snapshot.projection_path = target.display().to_string();
        let receipt_file = write_receipt(&ws, &receipt).await.unwrap();
        let receipt_bytes = tokio::fs::read(&receipt_file).await.unwrap();
        let entries_before: Vec<_> = std::fs::read_dir(super::super::trash_dir(&ws)).unwrap()
            .map(|entry| entry.unwrap().file_name()).collect();
        preflight_restore_with_target(&ws, &guard, &receipt, None).await.unwrap();
        assert!(!target.exists());
        assert_eq!(tokio::fs::read(&receipt_file).await.unwrap(), receipt_bytes);
        let entries_after: Vec<_> = std::fs::read_dir(super::super::trash_dir(&ws)).unwrap()
            .map(|entry| entry.unwrap().file_name()).collect();
        assert_eq!(entries_after, entries_before);
        let neighbor = b"---\nid: PRD-722\nkind: prd\nstatus: active\n---\n\nNeighbor.\n";
        tokio::fs::write(&target, neighbor).await.unwrap();
        let error = preflight_restore_with_target(&ws, &guard, &receipt, None).await.unwrap_err();
        assert!(error.to_string().contains("already exists"));
        assert_eq!(tokio::fs::read(&target).await.unwrap(), neighbor);
        assert_eq!(tokio::fs::read(&receipt_file).await.unwrap(), receipt_bytes);
        let entries_after: Vec<_> = std::fs::read_dir(super::super::trash_dir(&ws)).unwrap()
            .map(|entry| entry.unwrap().file_name()).collect();
        assert_eq!(entries_after, entries_before);
        assert!(!super::super::read_receipt(&receipt_file).await.unwrap().consumed);
    }

    #[tokio::test]
    async fn preflight_rejects_invalid_transition_before_writing() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace::init_workspace(tmp.path(), "restore-preflight-status").unwrap();
        let guard = workspace::acquire_workspace_lock(&ws).await.unwrap();
        let path = ws.join("prds/existing.md");
        let bytes = b"---\nid: PRD-723\nkind: prd\nstatus: deprecated\n---\n\nOriginal.\n";
        tokio::fs::write(&path, bytes).await.unwrap();
        let mut receipt = build_receipt("PRD-723", DestructiveOp::Deprecate, "original");
        receipt.snapshot.projection_path = path.display().to_string();
        let receipt_file = write_receipt(&ws, &receipt).await.unwrap();
        let before = tokio::fs::read(&receipt_file).await.unwrap();
        let error = preflight_restore_with_target(&ws, &guard, &receipt, Some("active")).await.unwrap_err();
        assert!(error.to_string().contains("lifecycle FSM"));
        assert_eq!(tokio::fs::read(&path).await.unwrap(), bytes);
        assert_eq!(tokio::fs::read(&receipt_file).await.unwrap(), before);
        assert!(!super::super::read_receipt(&receipt_file).await.unwrap().consumed);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn pinned_directory_enumeration_survives_path_substitution_and_repeats() {
        use std::os::unix::fs::symlink;
        let (tmp, ws, _, _guard) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-724", DestructiveOp::Delete, "original");
        let body = trashed_projection_path(&ws, &receipt.receipt_id);
        receipt.trashed_projection = body.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();
        tokio::fs::write(&body, "original").await.unwrap();
        let context = super::super::safe_restore::Context::open(&ws, &receipt.receipt_id).unwrap();
        let trash = super::super::trash_dir(&ws);
        tokio::fs::write(trash.join(format!("restore-{}-present.prepared", receipt.receipt_id)), "prepared").await.unwrap();
        assert!(context.has_journal().unwrap());
        assert!(context.has_journal().unwrap());
        let outside = tmp.path().join("outside-trash");
        tokio::fs::create_dir(&outside).await.unwrap();
        tokio::fs::write(outside.join(body.file_name().unwrap()), "foreign").await.unwrap();
        tokio::fs::rename(&trash, ws.join("trash-saved")).await.unwrap();
        symlink(&outside, &trash).unwrap();
        assert!(context.has_journal().unwrap());
        assert_eq!(context.body().unwrap(), b"original");
        assert_eq!(tokio::fs::read(outside.join(body.file_name().unwrap())).await.unwrap(), b"foreign");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn checked_body_is_not_reopened_after_substitution() {
        let (_tmp, ws, _, _guard) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-711", DestructiveOp::Delete, "original");
        let body = trashed_projection_path(&ws, &receipt.receipt_id);
        receipt.trashed_projection = body.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();
        tokio::fs::write(&body, "original").await.unwrap();
        let context = super::super::safe_restore::Context::open(&ws, &receipt.receipt_id).unwrap();
        let checked = context.body().unwrap();
        tokio::fs::write(&body, "replacement").await.unwrap();
        context.create("prds", "checked.md", &checked).unwrap();
        assert_eq!(tokio::fs::read(ws.join("prds/checked.md")).await.unwrap(), b"original");
        assert_eq!(tokio::fs::read(body).await.unwrap(), b"replacement");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn completion_does_not_consume_a_substituted_receipt() {
        let (_tmp, ws, _, _guard) = fresh_ws().await;
        let receipt = build_receipt("PRD-712", DestructiveOp::Deprecate, "original");
        let path = write_receipt(&ws, &receipt).await.unwrap();
        let context = super::super::safe_restore::Context::open(&ws, &receipt.receipt_id).unwrap();
        tokio::fs::rename(&path, path.with_extension("saved")).await.unwrap();
        let mut replacement = receipt.clone();
        replacement.snapshot.body = "different".into();
        tokio::fs::write(&path, serde_json::to_vec(&replacement).unwrap()).await.unwrap();
        context.consume().unwrap();
        let current = super::super::read_receipt(&path).await.unwrap();
        assert_eq!(current.snapshot.body, "different");
        assert!(!current.consumed);
    }

    #[tokio::test]
    async fn supplied_receipt_must_match_the_persisted_receipt() {
        let (_tmp, ws, _, guard) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-720", DestructiveOp::Deprecate, "original");
        receipt.snapshot.projection_path = ws.join("prds/original.md").display().to_string();
        let path = write_receipt(&ws, &receipt).await.unwrap();
        receipt.snapshot.title = "forged title".into();
        let error = apply_restore_file_first(&ws, &guard, &receipt).await.unwrap_err();
        assert!(error.to_string().contains("does not match"));
        assert!(!ws.join("prds/original.md").exists());
        assert!(!super::super::read_receipt(&path).await.unwrap().consumed);
    }

    #[cfg(any(unix, windows))]
    #[tokio::test]
    async fn completion_sync_failure_is_uncertain_and_retryable() {
        let (_tmp, ws, _, _guard) = fresh_ws().await;
        let receipt = build_receipt("PRD-718", DestructiveOp::Deprecate, "original");
        let path = write_receipt(&ws, &receipt).await.unwrap();
        let context = super::super::safe_restore::Context::open(&ws, &receipt.receipt_id).unwrap();
        let error = context.consume_with_sync_failure().unwrap_err();
        let uncertain = error.downcast_ref::<super::super::CompletionDurabilityUncertain>().unwrap();
        assert!(super::super::read_receipt(&path).await.unwrap().consumed);
        assert!(super::super::retry_completion(&ws, &receipt.receipt_id, "incorrect").await.is_err());
        super::super::retry_completion(&ws, &receipt.receipt_id, &uncertain.receipt_digest).await.unwrap();
        assert!(super::super::read_receipt(&path).await.unwrap().consumed);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn claim_retry_refuses_changed_target_without_overwriting() {
        let (_tmp, ws, _, _guard) = fresh_ws().await;
        let receipt = build_receipt("PRD-713", DestructiveOp::Deprecate, "original");
        write_receipt(&ws, &receipt).await.unwrap();
        let context = super::super::safe_restore::Context::open(&ws, &receipt.receipt_id).unwrap();
        context.create("prds", "claimed.md", b"original").unwrap();
        let target = ws.join("prds/claimed.md");
        tokio::fs::write(&target, b"changed").await.unwrap();
        assert!(context.create("prds", "claimed.md", b"original").is_err());
        assert_eq!(tokio::fs::read(target).await.unwrap(), b"changed");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn checked_document_replacement_detects_inode_substitution() {
        let (_tmp, ws, _, _guard) = fresh_ws().await;
        let receipt = build_receipt("PRD-715", DestructiveOp::Deprecate, "body");
        write_receipt(&ws, &receipt).await.unwrap();
        let path = ws.join("prds/swap.md");
        tokio::fs::write(&path, b"before").await.unwrap();
        let context = super::super::safe_restore::Context::open(&ws, &receipt.receipt_id).unwrap();
        let checked = context.read_document("prds", "swap.md").unwrap();
        tokio::fs::rename(&path, ws.join("prds/original-saved.md")).await.unwrap();
        tokio::fs::write(&path, b"before").await.unwrap();
        assert!(context.replace_document("prds", "swap.md", &checked, b"after").is_err());
        assert_eq!(tokio::fs::read(&path).await.unwrap(), b"before");
        assert_eq!(tokio::fs::read(ws.join("prds/original-saved.md")).await.unwrap(), b"before");
    }

    #[tokio::test]
    async fn incomplete_update_journal_blocks_recovery_without_writes() {
        let (_tmp, ws, _, guard) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-716", DestructiveOp::Deprecate, "body");
        receipt.snapshot.projection_path = ws.join("prds/original.md").display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();
        let base = format!("restore-{}-interrupted", receipt.receipt_id);
        let trash = super::super::trash_dir(&ws);
        tokio::fs::write(trash.join(format!("{base}.prepared")), b"prepared").await.unwrap();
        tokio::fs::write(trash.join(format!("{base}.postimage")), b"expected").await.unwrap();
        let error = apply_restore_file_first(&ws, &guard, &receipt).await.unwrap_err();
        assert!(error.to_string().contains("incomplete document update"));
        assert!(!ws.join("prds/original.md").exists());
        assert!(!super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
            .await.unwrap().consumed);
    }

    #[tokio::test]
    async fn moved_workspace_restores_the_old_historical_name() {
        let (_tmp, ws, _, guard) = fresh_ws().await;
        let original = "---\nid: PRD-714\nkind: prd\nstatus: active\n---\n\nOriginal.\n";
        let mut receipt = build_receipt("PRD-714", DestructiveOp::Delete, original);
        receipt.snapshot.projection_path = "/previous/.forgeplan/prds/Old title.md".into();
        receipt.trashed_projection = format!("/previous/.forgeplan/trash/body-{}.md", receipt.receipt_id);
        let body = trashed_projection_path(&ws, &receipt.receipt_id);
        write_receipt(&ws, &receipt).await.unwrap();
        tokio::fs::write(&body, original).await.unwrap();
        apply_restore_file_first(&ws, &guard, &receipt).await.unwrap();
        assert_eq!(tokio::fs::read(ws.join("prds/Old title.md")).await.unwrap(), original.as_bytes());
        assert_eq!(tokio::fs::read(body).await.unwrap(), original.as_bytes());
    }

    #[tokio::test]
    async fn restore_rejects_foreign_trash_path_before_writing() {
        let (tmp, ws, _, guard) = fresh_ws().await;
        let foreign = tmp.path().join("foreign.md");
        let bytes = b"foreign content\n";
        tokio::fs::write(&foreign, bytes).await.unwrap();
        let mut receipt = build_receipt("PRD-701", DestructiveOp::Delete, "body");
        let target = ws.join("prds/PRD-701-original.md");
        receipt.snapshot.projection_path = target.display().to_string();
        receipt.trashed_projection = foreign.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        assert!(apply_restore_file_first(&ws, &guard, &receipt).await.is_err());
        assert_eq!(tokio::fs::read(&foreign).await.unwrap(), bytes);
        assert!(!target.exists());
        assert!(!super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
            .await.unwrap().consumed);
    }

    #[tokio::test]
    async fn restore_rejects_a_different_identity_in_paired_trash() {
        let (_tmp, ws, _, guard) = fresh_ws().await;
        let key = "prd-original-k0123456789abcdef0123456789abcdef";
        let other = "---\nid: PRD-705\nkind: prd\nslug: prd-other-k11111111111111111111111111111111\n---\n\nOther.\n";
        let original = format!("---\nid: PRD-705\nkind: prd\nslug: {key}\n---\n\nOriginal.\n");
        let mut receipt = build_receipt("PRD-705", DestructiveOp::Delete, &original);
        receipt.snapshot.canonical_key = Some(key.into());
        let target = ws.join("prds/historical.md");
        receipt.snapshot.projection_path = target.display().to_string();
        let trashed = trashed_projection_path(&ws, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap()).await.unwrap();
        tokio::fs::write(&trashed, other).await.unwrap();
        receipt.trashed_projection = trashed.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        assert!(apply_restore_file_first(&ws, &guard, &receipt).await.is_err());
        assert!(!target.exists());
        assert_eq!(tokio::fs::read(&trashed).await.unwrap(), other.as_bytes());
        assert!(!super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
            .await.unwrap().consumed);
    }

    #[tokio::test]
    async fn legacy_restore_without_original_path_uses_safe_fallback() {
        let (_tmp, ws, _, guard) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-706", DestructiveOp::Delete, "body");
        let trashed = trashed_projection_path(&ws, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap()).await.unwrap();
        tokio::fs::write(&trashed, "body").await.unwrap();
        receipt.trashed_projection = trashed.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        apply_restore_file_first(&ws, &guard, &receipt).await.unwrap();
        assert_eq!(tokio::fs::read(ws.join("prds/PRD-706-title-prd-706.md")).await.unwrap(), b"body");
        assert!(trashed.exists());
        assert!(super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
            .await.unwrap().consumed);
    }

    #[tokio::test]
    async fn snapshot_restore_preserves_historical_path_and_neighbor() {
        let (_tmp, ws, _, guard) = fresh_ws().await;
        let key = "prd-original-k0123456789abcdef0123456789abcdef";
        let original = format!("---\nid: PRD-702\nkind: prd\nslug: {key}\nstatus: active\ncustom: keep\n---\n\nOriginal bytes.\n");
        let mut receipt = build_receipt("PRD-702", DestructiveOp::Deprecate, &original);
        receipt.snapshot.canonical_key = Some(key.into());
        receipt.snapshot.slug = Some(key.into());
        let target = ws.join("prds/historical-name.md");
        receipt.snapshot.projection_path = target.display().to_string();
        let neighbor = ws.join("prds/PRD-702-title-prd-702.md");
        let neighbor_bytes = b"---\nid: PRD-703\nkind: prd\nslug: prd-neighbor\nstatus: active\n---\n\nNeighbor.\n";
        tokio::fs::write(&neighbor, neighbor_bytes).await.unwrap();
        write_receipt(&ws, &receipt).await.unwrap();

        apply_restore_file_first(&ws, &guard, &receipt).await.unwrap();
        assert_eq!(tokio::fs::read(&target).await.unwrap(), original.as_bytes());
        assert_eq!(tokio::fs::read(&neighbor).await.unwrap(), neighbor_bytes);
        assert!(super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
            .await.unwrap().consumed);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn restore_rejects_symlinked_trash_body() {
        use std::os::unix::fs::symlink;

        let (tmp, ws, _, guard) = fresh_ws().await;
        let foreign = tmp.path().join("foreign.md");
        tokio::fs::write(&foreign, b"private\n").await.unwrap();
        let mut receipt = build_receipt("PRD-704", DestructiveOp::Delete, "body");
        let target = ws.join("prds/PRD-704-original.md");
        receipt.snapshot.projection_path = target.display().to_string();
        let trashed = trashed_projection_path(&ws, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap()).await.unwrap();
        symlink(&foreign, &trashed).unwrap();
        receipt.trashed_projection = trashed.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        assert!(apply_restore_file_first(&ws, &guard, &receipt).await.is_err());
        assert_eq!(tokio::fs::read(&foreign).await.unwrap(), b"private\n");
        assert!(!target.exists());
        assert!(!super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
            .await.unwrap().consumed);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn restore_rejects_hardlinked_trash_body() {
        let (tmp, ws, _, guard) = fresh_ws().await;
        let foreign = tmp.path().join("private.md");
        tokio::fs::write(&foreign, b"private\n").await.unwrap();
        let mut receipt = build_receipt("PRD-719", DestructiveOp::Delete, "private\n");
        receipt.snapshot.projection_path = ws.join("prds/target.md").display().to_string();
        let body = trashed_projection_path(&ws, &receipt.receipt_id);
        receipt.trashed_projection = body.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();
        tokio::fs::hard_link(&foreign, &body).await.unwrap();
        assert!(apply_restore_file_first(&ws, &guard, &receipt).await.is_err());
        assert_eq!(tokio::fs::read(&foreign).await.unwrap(), b"private\n");
        assert!(!ws.join("prds/target.md").exists());
        assert!(!super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
            .await.unwrap().consumed);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn resolver_refuses_symlinked_workspace_document() {
        use std::os::unix::fs::symlink;
        let (tmp, ws, _, guard) = fresh_ws().await;
        let secret = tmp.path().join("secret.md");
        tokio::fs::write(&secret, b"private bytes\n").await.unwrap();
        symlink(&secret, ws.join("prds/linked.md")).unwrap();
        let mut receipt = build_receipt("PRD-717", DestructiveOp::Deprecate, "body");
        receipt.snapshot.projection_path = ws.join("prds/target.md").display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();
        let error = apply_restore_file_first(&ws, &guard, &receipt).await.unwrap_err();
        assert!(matches!(error.downcast_ref::<crate::artifact::canonical_key::ResolveError>(),
            Some(crate::artifact::canonical_key::ResolveError::IncompleteCapture { .. })));
        assert_eq!(tokio::fs::read(&secret).await.unwrap(), b"private bytes\n");
        assert!(!ws.join("prds/target.md").exists());
        assert!(!super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
            .await.unwrap().consumed);
    }

    #[tokio::test]
    async fn restore_preserves_the_key_from_the_receipt() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        let key = "prd-auth-system-k0123456789abcdef0123456789abcdef";
        let body = format!("---\nid: PRD-042\nkind: prd\nslug: {key}\n---\n\n# body\n");
        let mut receipt = build_receipt("PRD-042", DestructiveOp::Delete, &body);
        receipt.snapshot.slug = Some(key.to_string());
        let target = ws.join("prds").join(format!("{key}.md"));
        receipt.snapshot.projection_path = target.display().to_string();

        // The document is in the trash; its canonical path is free.
        let trashed = trashed_projection_path(&ws, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&trashed, &body).await.unwrap();
        receipt.trashed_projection = trashed.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore(&ws, &guard, &store, &receipt).await.unwrap();
        assert_eq!(report.artifact_id, "PRD-042");
        assert!(
            target.exists(),
            "the document must come back at its canonical path: {}",
            target.display()
        );
        let text = tokio::fs::read_to_string(&target).await.unwrap();
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
        assert_eq!(
            fm.get("slug").and_then(|value| value.as_str()),
            Some(key),
            "the restored document keeps the key the receipt captured"
        );
    }

    #[tokio::test]
    async fn restore_refuses_to_overwrite_a_different_canonical_document() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        let key = "prd-auth-system-k0123456789abcdef0123456789abcdef";
        let body = format!("---\nid: PRD-042\nkind: prd\nslug: {key}\n---\n\n# body\n");
        let mut receipt = build_receipt("PRD-042", DestructiveOp::Delete, &body);
        receipt.snapshot.slug = Some(key.to_string());
        let target = ws.join("prds").join(format!("{key}.md"));
        receipt.snapshot.projection_path = target.display().to_string();

        // A DIFFERENT document already holds the canonical path.
        tokio::fs::create_dir_all(target.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(
            &target,
            "---\nid: PRD-099\nkind: prd\nslug: prd-other-k11111111111111111111111111111111\n---\n\n# other\n",
        )
        .await
        .unwrap();

        let trashed = trashed_projection_path(&ws, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&trashed, &body).await.unwrap();
        receipt.trashed_projection = trashed.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        let err = apply_restore(&ws, &guard, &store, &receipt)
            .await
            .expect_err("restore must refuse to overwrite a different document");
        assert!(
            err.to_string().contains("different canonical document"),
            "unexpected error: {err:#}"
        );
        let after = tokio::fs::read_to_string(&target).await.unwrap();
        assert!(
            after.contains("PRD-099"),
            "the other document must stay untouched"
        );
    }

    async fn assert_canonical_metadata(workspace: &Path, id: &str, status: &str, title: &str) {
        let path = crate::artifact::store::resolve_artifact_path(workspace, id)
            .await
            .expect("canonical artifact must exist");
        let text = tokio::fs::read_to_string(path).await.unwrap();
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
        assert_eq!(fm.get("id").and_then(|value| value.as_str()), Some(id));
        assert_eq!(
            fm.get("status").and_then(|value| value.as_str()),
            Some(status)
        );
        assert_eq!(
            fm.get("title").and_then(|value| value.as_str()),
            Some(title)
        );
    }

    #[tokio::test]
    async fn restore_delete_recreates_row() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-001", DestructiveOp::Delete, "# body");
        stage_deleted_projection(&ws, &mut receipt).await;
        write_receipt(&ws, &receipt).await.unwrap();

        // Simulate prior delete: no row in store.
        assert!(store.get_record(&guard, "PRD-001").await.unwrap().is_none());

        let report = apply_restore(&ws, &guard, &store, &receipt).await.unwrap();
        assert_eq!(report.artifact_id, "PRD-001");

        let restored = store
            .get_record(&guard, "PRD-001")
            .await
            .unwrap()
            .expect("row back");
        assert_eq!(restored.body, "# body");
        assert_eq!(restored.status, "active");
        assert!(report.projection_restored);
        assert_canonical_metadata(&ws, "PRD-001", "active", "Title PRD-001").await;
        assert!(
            super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    #[tokio::test]
    async fn restore_refuses_if_id_collision_on_delete() {
        let (_tmp, ws, store, guard) = fresh_ws().await;

        // Create PRD-001 currently in store (someone re-created it).
        let existing = NewArtifact {
            canonical_key: "PRD-001".into(),
            id: "PRD-001".into(),
            kind: "prd".into(),
            status: "active".into(),
            title: "Collider".into(),
            body: "different body".into(),
            depth: "standard".into(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        };
        store.create_artifact(&guard, &existing).await.unwrap();

        // A delete-receipt for the same ID.
        let receipt = build_receipt("PRD-001", DestructiveOp::Delete, "original body");
        write_receipt(&ws, &receipt).await.unwrap();

        let err = apply_restore(&ws, &guard, &store, &receipt).await.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("already exists"),
            "expected collision error, got: {msg}"
        );

        // Existing row untouched.
        let r = store.get_record(&guard, "PRD-001").await.unwrap().unwrap();
        assert_eq!(r.body, "different body");
    }

    #[tokio::test]
    async fn restore_deprecate_resets_status() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        // Create as active in store.
        store
            .create_artifact(&guard, &NewArtifact {
                canonical_key: "PRD-002".into(),
                id: "PRD-002".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "To deprecate".into(),
                body: "b".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        // Change to deprecated in store.
        store
            .update_artifact(&guard, "PRD-002", Some("deprecated"), None)
            .await
            .unwrap();
        {
            crate::projection::render_after_mutation(&guard, &ws, &store, "PRD-002")
                .await
                .unwrap();
        }
        assert_canonical_metadata(&ws, "PRD-002", "deprecated", "To deprecate").await;

        // Receipt captured state BEFORE deprecate (status=active).
        let mut receipt = build_receipt("PRD-002", DestructiveOp::Deprecate, "b");
        receipt.snapshot.title = "To deprecate".into();
        receipt.reason = Some("testing".into());
        write_receipt(&ws, &receipt).await.unwrap();

        apply_restore(&ws, &guard, &store, &receipt).await.unwrap();

        let r = store.get_record(&guard, "PRD-002").await.unwrap().unwrap();
        assert_eq!(r.status, "active");
        assert_canonical_metadata(&ws, "PRD-002", "active", "To deprecate").await;
        assert!(
            super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    #[tokio::test]
    async fn restore_marks_receipt_consumed() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-003", DestructiveOp::Delete, "x");
        stage_deleted_projection(&ws, &mut receipt).await;
        write_receipt(&ws, &receipt).await.unwrap();
        apply_restore(&ws, &guard, &store, &receipt).await.unwrap();

        // Second try via apply_restore_by_id should refuse (consumed).
        let err = apply_restore_by_id(&ws, &guard, &store, &receipt.receipt_id)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("already consumed"));
    }

    #[tokio::test]
    async fn restore_skips_orphaned_relations() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-004", DestructiveOp::Delete, "b");
        // Outgoing relation to EVID-999 which doesn't exist in store.
        receipt.snapshot.relations.push(CapturedRelation {
            from: "PRD-004".into(),
            to: "EVID-999".into(),
            relation: "informs".into(),
            direction: RelationDirection::Outgoing,
        });
        stage_deleted_projection(&ws, &mut receipt).await;
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore(&ws, &guard, &store, &receipt).await.unwrap();
        assert_eq!(report.relations_restored, 0);
        assert_eq!(report.relations_skipped, vec!["EVID-999".to_string()]);
        assert!(report.warnings.is_empty());
        assert_canonical_metadata(&ws, "PRD-004", "active", "Title PRD-004").await;
        assert!(
            super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    #[tokio::test]
    async fn restore_moves_projection_back_for_delete() {
        let (_tmp, ws, store, guard) = fresh_ws().await;

        // Prepare a trashed projection file.
        let receipt_id = generate_receipt_id("prd", "PRD-005");
        let trashed = trashed_projection_path(&ws, &receipt_id);
        let parent = trashed.parent().unwrap();
        tokio::fs::create_dir_all(parent).await.unwrap();
        tokio::fs::write(&trashed, b"# original body\n")
            .await
            .unwrap();

        // Target path for restore.
        let original = ws.join("prds").join("PRD-005-t.md");

        let receipt = Receipt {
            receipt_id: receipt_id.clone(),
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            op: DestructiveOp::Delete,
            snapshot: ArtifactSnapshot {
                id: "PRD-005".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "t".into(),
                depth: "standard".into(),
                body: "# original body\n".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                relations: vec![],
                projection_path: original.display().to_string(),
                slug: None,
                canonical_key: None,
            },
            reason: None,
            replacement: None,
            trashed_projection: trashed.display().to_string(),
            activity_log_hash: None,
            consumed: false,
        };
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore(&ws, &guard, &store, &receipt).await.unwrap();
        assert!(report.projection_restored);
        assert!(
            original.exists(),
            "projection should be back at {}",
            original.display()
        );
        assert_eq!(tokio::fs::read(&trashed).await.unwrap(), b"# original body\n");
        assert_eq!(
            tokio::fs::read(&original).await.unwrap(),
            b"# original body\n"
        );
        assert_eq!(
            store.get_record(&guard, "PRD-005").await.unwrap().unwrap().status,
            "active"
        );
        assert!(
            super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    // ── Audit Round 3 regression tests ────────────────────────

    #[tokio::test]
    async fn restore_rejects_traversal_projection_path() {
        // A forged destination must fail, not redirect to a computed name.
        let (tmp, ws, store, guard) = fresh_ws().await;

        // Seed a trashed body.
        let receipt_id = generate_receipt_id("prd", "PRD-SEC");
        let trashed = trashed_projection_path(&ws, &receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&trashed, b"# pwn payload\n")
            .await
            .unwrap();

        // Craft malicious receipt.
        let evil_target = tmp.path().join("pwn.md");
        let receipt = Receipt {
            receipt_id: receipt_id.clone(),
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            op: DestructiveOp::Delete,
            snapshot: ArtifactSnapshot {
                id: "PRD-SEC".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "security test".into(),
                depth: "standard".into(),
                body: "# pwn payload\n".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                relations: vec![],
                // ATTACKER-CONTROLLED: tries to write outside workspace.
                projection_path: evil_target.display().to_string(),
                slug: None,
                canonical_key: None,
            },
            reason: None,
            replacement: None,
            trashed_projection: trashed.display().to_string(),
            activity_log_hash: None,
            consumed: false,
        };
        write_receipt(&ws, &receipt).await.unwrap();

        let error = apply_restore(&ws, &guard, &store, &receipt).await.unwrap_err();
        assert!(error.to_string().contains("projection path"));
        assert!(!evil_target.exists());
        assert!(!ws.join("prds/PRD-SEC-security-test.md").exists());
        assert!(trashed.exists());
        assert!(!super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
            .await.unwrap().consumed);
    }

    #[tokio::test]
    async fn restore_refuses_missing_receipt_before_writing() {
        // A receipt that disappeared before restore cannot authorize writes.
        let (_tmp, ws, store, guard) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-MC", DestructiveOp::Delete, "body");
        stage_deleted_projection(&ws, &mut receipt).await;
        write_receipt(&ws, &receipt).await.unwrap();

        // Remove the persisted receipt before applying it.
        let rpath = receipt_path(&ws, &receipt.receipt_id);
        tokio::fs::remove_file(&rpath).await.unwrap();

        let result = apply_restore(&ws, &guard, &store, &receipt).await;
        assert!(result.is_err());
        assert!(store.get_record(&guard, "PRD-MC").await.unwrap().is_none());
        assert!(Path::new(&receipt.trashed_projection).exists());
        assert!(!compute_safe_projection_path(&ws, &receipt.snapshot).unwrap().exists());
    }

    #[tokio::test]
    async fn restore_bails_on_kind_mismatch_for_supersede() {
        // Audit Round 3 H-4: supersede/deprecate restore on collision
        // branch must validate kind/title. If row was re-created with
        // a different kind, refuse rather than silently overwrite.
        let (_tmp, ws, store, guard) = fresh_ws().await;

        // Seed a DIFFERENT artifact at PRD-CONFLICT (same ID but RFC kind
        // would conflict — use same ID prefix but different kind field).
        // In practice ID collision across kinds is prevented, but attacker
        // could craft a receipt claiming prd while row is something else.
        // Simulate: row is prd, receipt claims it was rfc.
        store
            .create_artifact(&guard, &NewArtifact {
                canonical_key: "PRD-CONFLICT".into(),
                id: "PRD-CONFLICT".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "Current".into(),
                body: "current body".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();

        let receipt = Receipt {
            receipt_id: generate_receipt_id("rfc", "PRD-CONFLICT"),
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            op: DestructiveOp::Supersede,
            snapshot: ArtifactSnapshot {
                id: "PRD-CONFLICT".into(),
                kind: "rfc".into(), // mismatch with existing row
                status: "active".into(),
                title: "Old RFC".into(),
                depth: "standard".into(),
                body: "old rfc body".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                relations: vec![],
                projection_path: String::new(),
                slug: None,
                canonical_key: None,
            },
            reason: None,
            replacement: Some("PRD-999".into()),
            trashed_projection: String::new(),
            activity_log_hash: None,
            consumed: false,
        };
        write_receipt(&ws, &receipt).await.unwrap();

        let err = apply_restore(&ws, &guard, &store, &receipt).await.unwrap_err();
        assert!(
            err.to_string().contains("refusing to overwrite"),
            "kind mismatch must refuse: {err}"
        );
        // Current row unchanged.
        let current = store.get_record(&guard, "PRD-CONFLICT").await.unwrap().unwrap();
        assert_eq!(current.kind, "prd", "existing row must be untouched");
        assert_eq!(current.title, "Current");
    }

    // ─── Issue #291: target_status override + restored_status surface ─────

    /// Default behaviour (no override): restored_status equals captured
    /// snapshot.status. RestoreReport surfaces it.
    #[tokio::test]
    async fn restore_report_surfaces_captured_status() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        store
            .create_artifact(&guard, &NewArtifact {
                canonical_key: "PRD-291A".into(),
                id: "PRD-291A".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "T".into(),
                body: "b".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        store
            .update_artifact(&guard, "PRD-291A", Some("deprecated"), None)
            .await
            .unwrap();
        {
            crate::projection::render_after_mutation(&guard, &ws, &store, "PRD-291A")
                .await
                .unwrap();
        }
        let mut receipt = build_receipt("PRD-291A", DestructiveOp::Deprecate, "b");
        receipt.snapshot.status = "active".into();
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore(&ws, &guard, &store, &receipt).await.unwrap();
        assert_eq!(report.restored_status, "active");
        assert_canonical_metadata(&ws, "PRD-291A", "active", "Title PRD-291A").await;
    }

    /// `apply_restore_with_target` explicit override wins over snapshot
    /// when the override is FSM-valid from the current state.
    ///
    /// Audit-dogfood SEC-2: override now respects lifecycle FSM. This
    /// test uses `draft → active` (a legal transition); the negative
    /// case (forbidden transition) is pinned in
    /// `restore_with_target_rejects_fsm_invalid_transition` below.
    #[tokio::test]
    async fn restore_with_target_override_applies_explicit_status() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        store
            .create_artifact(&guard, &NewArtifact {
                canonical_key: "PRD-291B".into(),
                id: "PRD-291B".into(),
                kind: "prd".into(),
                status: "draft".into(),
                title: "T".into(),
                body: "b".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        {
            crate::projection::render_after_mutation(&guard, &ws, &store, "PRD-291B")
                .await
                .unwrap();
        }
        // Receipt captured active (the prior_status before some earlier op);
        // current row is draft (a valid `draft → active` transition).
        let mut receipt = build_receipt("PRD-291B", DestructiveOp::Deprecate, "b");
        receipt.snapshot.status = "draft".into();
        write_receipt(&ws, &receipt).await.unwrap();

        // Override to `active` — FSM-valid from `draft`.
        let report = apply_restore_with_target(&ws, &guard, Some(&store), &receipt, Some("active"))
            .await
            .unwrap();
        assert_eq!(report.restored_status, "active", "override wins");

        let r = store.get_record(&guard, "PRD-291B").await.unwrap().unwrap();
        assert_eq!(r.status, "active", "store reflects override");
        assert_canonical_metadata(&ws, "PRD-291B", "active", "Title PRD-291B").await;
        let persisted = super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
            .await
            .unwrap();
        assert_eq!(persisted.snapshot.status, "draft");
        assert!(persisted.consumed);
    }

    /// Audit-dogfood SEC-2: target_status override that violates the
    /// lifecycle FSM is rejected before any mutation. Example: trying
    /// to restore a `deprecated` artifact to `draft` — that's not a
    /// legal forward transition. Operator must use the canonical
    /// lifecycle commands.
    #[tokio::test]
    async fn restore_with_target_rejects_fsm_invalid_transition() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        store
            .create_artifact(&guard, &NewArtifact {
                canonical_key: "PRD-291E".into(),
                id: "PRD-291E".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "T".into(),
                body: "b".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        store
            .update_artifact(&guard, "PRD-291E", Some("deprecated"), None)
            .await
            .unwrap();
        {
            crate::projection::render_after_mutation(&guard, &ws, &store, "PRD-291E")
                .await
                .unwrap();
        }
        let mut receipt = build_receipt("PRD-291E", DestructiveOp::Deprecate, "b");
        receipt.snapshot.status = "active".into();
        write_receipt(&ws, &receipt).await.unwrap();

        // `deprecated → draft` is FSM-invalid — must be rejected.
        let err = apply_restore_with_target(&ws, &guard, Some(&store), &receipt, Some("draft"))
            .await
            .expect_err("FSM should reject deprecated->draft override");
        assert!(
            err.to_string().contains("FSM"),
            "rejection must cite FSM: {err}"
        );
        // Store still deprecated — no partial mutation.
        let r = store.get_record(&guard, "PRD-291E").await.unwrap().unwrap();
        assert_eq!(r.status, "deprecated", "no partial mutation on FSM reject");
        assert_canonical_metadata(&ws, "PRD-291E", "deprecated", "T").await;
        assert!(
            !super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    /// Invalid target_status value is rejected BEFORE mutation.
    #[tokio::test]
    async fn restore_with_target_rejects_invalid_status() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        store
            .create_artifact(&guard, &NewArtifact {
                canonical_key: "PRD-291C".into(),
                id: "PRD-291C".into(),
                kind: "prd".into(),
                status: "deprecated".into(),
                title: "T".into(),
                body: "b".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        {
            crate::projection::render_after_mutation(&guard, &ws, &store, "PRD-291C")
                .await
                .unwrap();
        }
        let mut receipt = build_receipt("PRD-291C", DestructiveOp::Deprecate, "b");
        receipt.snapshot.status = "active".into();
        write_receipt(&ws, &receipt).await.unwrap();

        let err = apply_restore_with_target(&ws, &guard, Some(&store), &receipt, Some("bogus"))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("Invalid target_status"),
            "bad override must error: {err}"
        );
        // Store unchanged — error happened before mutation.
        let r = store.get_record(&guard, "PRD-291C").await.unwrap().unwrap();
        assert_eq!(r.status, "deprecated", "no partial restore on bad input");
        assert_canonical_metadata(&ws, "PRD-291C", "deprecated", "T").await;
        assert!(
            !super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    /// `None` override = legacy behaviour identical to `apply_restore`.
    #[tokio::test]
    async fn restore_with_target_none_matches_legacy() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        store
            .create_artifact(&guard, &NewArtifact {
                canonical_key: "PRD-291D".into(),
                id: "PRD-291D".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "T".into(),
                body: "b".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        store
            .update_artifact(&guard, "PRD-291D", Some("deprecated"), None)
            .await
            .unwrap();
        {
            crate::projection::render_after_mutation(&guard, &ws, &store, "PRD-291D")
                .await
                .unwrap();
        }
        let mut receipt = build_receipt("PRD-291D", DestructiveOp::Deprecate, "b");
        receipt.snapshot.status = "active".into();
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore_with_target(&ws, &guard, Some(&store), &receipt, None)
            .await
            .unwrap();
        assert_eq!(
            report.restored_status, "active",
            "None override uses captured snapshot.status"
        );
        assert_canonical_metadata(&ws, "PRD-291D", "active", "Title PRD-291D").await;
    }

    #[tokio::test]
    async fn legacy_delete_restore_preserves_bytes_and_replays_canonical_relations() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        let target = build_receipt("PRD-412", DestructiveOp::Deprecate, "Target body.\n");
        let incoming = build_receipt("PRD-413", DestructiveOp::Deprecate, "Incoming body.\n");
        for participant in [&target, &incoming] {
            render_snapshot_projection(&ws, participant, "active")
                .await
                .unwrap();
            store
                .create_artifact(&guard, &snapshot_artifact(participant, "active"))
                .await
                .unwrap();
        }
        let target_path = compute_safe_projection_path(&ws, &target.snapshot).unwrap();
        let target_bytes = tokio::fs::read(&target_path).await.unwrap();
        let mut receipt = build_receipt("PRD-411", DestructiveOp::Delete, "Original body.\n");
        receipt.snapshot.relations = vec![
            CapturedRelation {
                from: "PRD-411".into(),
                to: "PRD-412".into(),
                relation: "based_on".into(),
                direction: RelationDirection::Outgoing,
            },
            CapturedRelation {
                from: "PRD-413".into(),
                to: "PRD-411".into(),
                relation: "informs".into(),
                direction: RelationDirection::Incoming,
            },
        ];
        // A surviving derived triple must be an exact duplicate no-op, not a
        // swallowed relation error. Its missing canonical link still needs replay.
        store
            .add_relation(&guard, "PRD-413", "PRD-411", "informs")
            .await
            .unwrap();
        let original = "---\n# Keep operator formatting and fields not captured in the receipt.\nid: PRD-411\nkind: prd\nstatus: active\ntitle: 'Title PRD-411'\ndepth: standard\ntags: [kept]\ncustom: 'unchanged'\nlinks:\n  - target: PRD-412\n    relation: based_on\n---\n\nOriginal body.\n";
        let trashed = trashed_projection_path(&ws, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&trashed, original).await.unwrap();
        receipt.snapshot.projection_path = ws.join("prds/PRD-411-title-prd-411.md").display().to_string();
        receipt.trashed_projection = trashed.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore(&ws, &guard, &store, &receipt).await.unwrap();
        let restored_path = compute_safe_projection_path(&ws, &receipt.snapshot).unwrap();
        assert_eq!(
            tokio::fs::read(&restored_path).await.unwrap(),
            original.as_bytes()
        );
        assert_eq!(tokio::fs::read(&trashed).await.unwrap(), original.as_bytes());
        assert!(report.projection_restored);
        assert_eq!(report.relations_restored, 2);
        assert!(report.relations_skipped.is_empty());
        assert!(report.warnings.is_empty());
        assert_canonical_metadata(&ws, "PRD-411", "active", "Title PRD-411").await;
        let incoming_path = compute_safe_projection_path(&ws, &incoming.snapshot).unwrap();
        let text = tokio::fs::read_to_string(incoming_path).await.unwrap();
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
        assert_eq!(
            crate::link::list_links(&fm),
            vec![("PRD-411".to_string(), "informs".to_string())]
        );
        assert_eq!(
            store.get_relations(&guard, "PRD-411").await.unwrap(),
            vec![("PRD-412".to_string(), "based_on".to_string())]
        );
        assert_eq!(
            store.get_relations(&guard, "PRD-413").await.unwrap(),
            vec![("PRD-411".to_string(), "informs".to_string())]
        );
        assert_eq!(
            store.get_record(&guard, "PRD-411").await.unwrap().unwrap().status,
            "active"
        );
        assert_eq!(tokio::fs::read(target_path).await.unwrap(), target_bytes);
        assert!(
            super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    #[tokio::test]
    async fn legacy_deleted_artifact_restore_applies_explicit_status() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-441", DestructiveOp::Delete, "Original body.\n");
        stage_deleted_projection(&ws, &mut receipt).await;
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore_with_target(&ws, &guard, Some(&store), &receipt, Some(" draft "))
            .await
            .unwrap();

        assert_canonical_metadata(&ws, "PRD-441", "draft", "Title PRD-441").await;
        assert_eq!(
            store.get_record(&guard, "PRD-441").await.unwrap().unwrap().status,
            "draft"
        );
        assert_eq!(report.restored_status, "draft");
        let persisted = super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
            .await
            .unwrap();
        assert_eq!(persisted.snapshot.status, "active");
        assert!(persisted.consumed);
        assert!(Path::new(&receipt.trashed_projection).exists());
    }

    #[tokio::test]
    async fn restore_supersede_reverses_title_rename_and_edge_on_both_layouts() {
        for legacy in [false, true] {
            let tmp = TempDir::new().unwrap();
            let ws = workspace::init_workspace(tmp.path(), "restore-renamed-supersede").unwrap();
            let guard = workspace::acquire_workspace_lock(&ws).await.unwrap();
            let sink = if legacy {
                Some(LanceStore::init_new(&ws, &guard).await.unwrap())
            } else {
                None
            };
            let mut receipt = build_receipt("PRD-421", DestructiveOp::Supersede, "Source body.\n");
            let replacement =
                build_receipt("PRD-422", DestructiveOp::Deprecate, "Replacement body.\n");
            for participant in [&receipt, &replacement] {
                render_snapshot_projection(&ws, participant, "active")
                    .await
                    .unwrap();
                if let Some(store) = sink.as_ref() {
                    store
                        .create_artifact(&guard, &snapshot_artifact(participant, "active"))
                        .await
                        .unwrap();
                }
            }
            let original_path = compute_safe_projection_path(&ws, &receipt.snapshot).unwrap();
            let replacement_path =
                compute_safe_projection_path(&ws, &replacement.snapshot).unwrap();
            receipt.replacement = Some("PRD-422".into());
            write_receipt(&ws, &receipt).await.unwrap();
            let ctx = crate::projection::MutationContext::new(&ws, &guard, sink.as_ref());
            let read = crate::constellation::facade::open_workspace_read(
                &ws,
                None,
                crate::constellation::capture::CaptureLimits::default(),
            )
            .await
            .unwrap();
            crate::lifecycle::canonical::supersede_captured(&ctx, &read, "PRD-421", "PRD-422")
                .await
                .unwrap();
            crate::projection::update_metadata_with_projection(
                &ctx,
                "PRD-421",
                None,
                Some("Renamed after supersede"),
            )
            .await
            .unwrap();
            let renamed_path = crate::artifact::store::resolve_artifact_path(&ws, "PRD-421")
                .await
                .unwrap();
            assert_eq!(renamed_path, original_path);
            assert!(original_path.exists());
            let superseded_text = tokio::fs::read_to_string(&renamed_path).await.unwrap();
            let (fm, _) =
                crate::artifact::frontmatter::parse_frontmatter(&superseded_text).unwrap();
            assert_eq!(
                fm.get("status").and_then(|value| value.as_str()),
                Some("superseded")
            );
            assert!(
                crate::link::list_links(&fm)
                    .contains(&("PRD-422".to_string(), "supersedes".to_string()))
            );
            let replacement_bytes = tokio::fs::read(&replacement_path).await.unwrap();

            let report = match sink.as_ref() {
                Some(store) => apply_restore(&ws, &guard, store, &receipt).await.unwrap(),
                None => apply_restore_file_first(&ws, &guard, &receipt).await.unwrap(),
            };

            let restored_path = crate::artifact::store::resolve_artifact_path(&ws, "PRD-421")
                .await
                .unwrap();
            assert_eq!(restored_path, original_path);
            assert!(renamed_path.exists());
            assert_canonical_metadata(&ws, "PRD-421", "active", "Title PRD-421").await;
            let text = tokio::fs::read_to_string(restored_path).await.unwrap();
            let (fm, body) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
            assert!(crate::link::list_links(&fm).is_empty());
            assert_eq!(body.trim(), "Source body.");
            assert!(report.projection_restored);
            assert!(report.warnings.is_empty());
            assert_eq!(report.restored_status, "active");
            if let Some(store) = sink.as_ref() {
                let record = store.get_record(&guard, "PRD-421").await.unwrap().unwrap();
                assert_eq!(record.status, "active");
                assert_eq!(record.title, "Title PRD-421");
                assert!(store.get_relations(&guard, "PRD-421").await.unwrap().is_empty());
            } else {
                assert!(!ws.join("lance/artifacts.lance").exists());
            }
            assert_eq!(
                tokio::fs::read(replacement_path).await.unwrap(),
                replacement_bytes
            );
            assert!(
                super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                    .await
                    .unwrap()
                    .consumed
            );
        }
    }

    #[tokio::test]
    async fn restore_relation_write_failure_keeps_receipt_retryable_on_both_layouts() {
        for legacy in [false, true] {
            let tmp = TempDir::new().unwrap();
            let ws = workspace::init_workspace(tmp.path(), "restore-relation-failure").unwrap();
            let guard = workspace::acquire_workspace_lock(&ws).await.unwrap();
            let sink = if legacy {
                Some(LanceStore::init_new(&ws, &guard).await.unwrap())
            } else {
                None
            };
            let mut receipt =
                build_receipt("PRD-431", DestructiveOp::Deprecate, "Restored body.\n");
            let incoming = build_receipt("PRD-432", DestructiveOp::Deprecate, "Incoming body.\n");
            render_snapshot_projection(&ws, &receipt, "deprecated")
                .await
                .unwrap();
            let source_path = compute_safe_projection_path(&ws, &incoming.snapshot).unwrap();
            // This participant exists, but the real canonical link writer
            // cannot append to its malformed links field. It must not count
            // as an orphan, duplicate, or successful recovery.
            let malformed = "---\nid: PRD-432\nkind: prd\nstatus: active\ntitle: Title PRD-432\nlinks: not-a-sequence\n---\n\nIncoming body.\n";
            tokio::fs::write(&source_path, malformed).await.unwrap();
            if let Some(store) = sink.as_ref() {
                store
                    .create_artifact(&guard, &snapshot_artifact(&receipt, "deprecated"))
                    .await
                    .unwrap();
                store
                    .create_artifact(&guard, &snapshot_artifact(&incoming, "active"))
                    .await
                    .unwrap();
            }
            receipt.snapshot.relations.push(CapturedRelation {
                from: "PRD-432".into(),
                to: "PRD-431".into(),
                relation: "informs".into(),
                direction: RelationDirection::Incoming,
            });
            write_receipt(&ws, &receipt).await.unwrap();
            let receipt_file = receipt_path(&ws, &receipt.receipt_id);
            let receipt_bytes = tokio::fs::read(&receipt_file).await.unwrap();

            let error = apply_restore_with_target(&ws, &guard, sink.as_ref(), &receipt, None)
                .await
                .expect_err("a failed incoming relation write cannot consume the receipt");

            assert!(matches!(
                error.downcast_ref::<crate::error::ForgeplanError>(),
                Some(crate::error::ForgeplanError::Frontmatter(_))
            ));
            assert_canonical_metadata(&ws, "PRD-431", "active", "Title PRD-431").await;
            assert_eq!(
                tokio::fs::read(&source_path).await.unwrap(),
                malformed.as_bytes()
            );
            assert_eq!(tokio::fs::read(&receipt_file).await.unwrap(), receipt_bytes);
            assert!(
                !super::super::read_receipt(&receipt_file)
                    .await
                    .unwrap()
                    .consumed
            );
            if let Some(store) = sink.as_ref() {
                assert_eq!(
                    store.get_record(&guard, "PRD-431").await.unwrap().unwrap().status,
                    "active"
                );
                assert!(store.get_relations(&guard, "PRD-432").await.unwrap().is_empty());
            }

            tokio::fs::remove_file(&source_path).await.unwrap();
            render_snapshot_projection(&ws, &incoming, "active")
                .await
                .unwrap();
            let report = apply_restore_with_target(&ws, &guard, sink.as_ref(), &receipt, None)
                .await
                .unwrap();
            let text = tokio::fs::read_to_string(&source_path).await.unwrap();
            let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
            assert_eq!(
                crate::link::list_links(&fm),
                vec![("PRD-431".to_string(), "informs".to_string())]
            );
            assert_eq!(report.relations_restored, 1);
            assert!(report.relations_skipped.is_empty());
            assert!(report.warnings.is_empty());
            assert!(
                super::super::read_receipt(&receipt_file)
                    .await
                    .unwrap()
                    .consumed
            );
            if let Some(store) = sink.as_ref() {
                assert_eq!(
                    store.get_relations(&guard, "PRD-432").await.unwrap(),
                    vec![("PRD-431".to_string(), "informs".to_string())]
                );
            } else {
                assert!(!ws.join("lance/artifacts.lance").exists());
            }
        }
    }

    #[tokio::test]
    async fn restore_delete_without_trash_keeps_receipt_unconsumed_on_both_layouts() {
        for legacy in [false, true] {
            let tmp = TempDir::new().unwrap();
            let ws = workspace::init_workspace(tmp.path(), "restore-without-trash").unwrap();
            let guard = workspace::acquire_workspace_lock(&ws).await.unwrap();
            let sink = if legacy {
                Some(LanceStore::init_new(&ws, &guard).await.unwrap())
            } else {
                None
            };
            let receipt = build_receipt("PRD-451", DestructiveOp::Delete, "Body without trash.\n");
            write_receipt(&ws, &receipt).await.unwrap();

            let error = apply_restore_with_target(&ws, &guard, sink.as_ref(), &receipt, None)
                .await
                .unwrap_err();

            assert!(error.to_string().contains("No trashed projection"));
            assert!(
                crate::artifact::store::resolve_artifact_path(&ws, "PRD-451")
                    .await
                    .is_none()
            );
            assert!(
                !super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                    .await
                    .unwrap()
                    .consumed
            );
            if let Some(store) = sink.as_ref() {
                assert!(store.get_record(&guard, "PRD-451").await.unwrap().is_none());
            } else {
                assert!(!ws.join("lance/artifacts.lance").exists());
            }
        }
    }

    #[tokio::test]
    async fn legacy_restore_refuses_canonical_collision_even_without_a_derived_row() {
        let (_tmp, ws, store, guard) = fresh_ws().await;
        let mut receipt = build_receipt("PRD-461", DestructiveOp::Delete, "Original body.\n");
        stage_deleted_projection(&ws, &mut receipt).await;
        let mut collider = build_receipt("PRD-461", DestructiveOp::Deprecate, "Current body.\n");
        collider.snapshot.title = "Current canonical file".into();
        render_snapshot_projection(&ws, &collider, "active")
            .await
            .unwrap();
        let collider_path = compute_safe_projection_path(&ws, &collider.snapshot).unwrap();
        let current_bytes = tokio::fs::read(&collider_path).await.unwrap();
        write_receipt(&ws, &receipt).await.unwrap();
        assert!(store.get_record(&guard, "PRD-461").await.unwrap().is_none());

        let error = apply_restore(&ws, &guard, &store, &receipt).await.unwrap_err();

        assert!(error.to_string().contains("already exists"));
        assert_eq!(tokio::fs::read(collider_path).await.unwrap(), current_bytes);
        assert!(Path::new(&receipt.trashed_projection).exists());
        assert!(store.get_record(&guard, "PRD-461").await.unwrap().is_none());
        assert!(
            !super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
    }

    #[tokio::test]
    async fn canonical_delete_restore_preserves_bytes_and_existing_outgoing_edge() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace::init_workspace(tmp.path(), "canonical-restore").unwrap();
        let guard = workspace::acquire_workspace_lock(&ws).await.unwrap();
        let mut receipt = build_receipt("PRD-401", DestructiveOp::Delete, "Original body.\n");
        let original = "---\nid: PRD-401\nkind: prd\nstatus: active\ntitle: Title PRD-401\ndepth: standard\nlinks:\n  - target: EVID-401\n    relation: based_on\n---\n\nOriginal body.\n";
        tokio::fs::write(
            ws.join("evidence/EVID-401-target.md"),
            "---\nid: EVID-401\nkind: evidence\nstatus: active\ntitle: Target\n---\n\nEvidence.\n",
        )
        .await
        .unwrap();
        receipt.snapshot.relations.push(CapturedRelation {
            from: "PRD-401".into(),
            to: "EVID-401".into(),
            relation: "based_on".into(),
            direction: RelationDirection::Outgoing,
        });
        let trashed = trashed_projection_path(&ws, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&trashed, original).await.unwrap();
        receipt.snapshot.projection_path = ws.join("prds/PRD-401-title-prd-401.md").display().to_string();
        receipt.trashed_projection = trashed.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore_with_target(&ws, &guard, None, &receipt, None)
            .await
            .unwrap();
        let restored = crate::artifact::store::resolve_artifact_path(&ws, "PRD-401")
            .await
            .unwrap();
        assert_eq!(tokio::fs::read_to_string(restored).await.unwrap(), original);
        assert_eq!(report.relations_restored, 1);
        assert!(report.relations_skipped.is_empty());
        assert!(report.warnings.is_empty());
        assert!(
            super::super::read_receipt(&receipt_path(&ws, &receipt.receipt_id))
                .await
                .unwrap()
                .consumed
        );
        assert!(!ws.join("lance/artifacts.lance").exists());
    }

    #[tokio::test]
    async fn canonical_deleted_artifact_restore_applies_explicit_status() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace::init_workspace(tmp.path(), "canonical-override").unwrap();
        let guard = workspace::acquire_workspace_lock(&ws).await.unwrap();
        let mut receipt = build_receipt("PRD-402", DestructiveOp::Delete, "Original body.\n");
        let trashed = trashed_projection_path(&ws, &receipt.receipt_id);
        tokio::fs::create_dir_all(trashed.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(
            &trashed,
            "---\nid: PRD-402\nkind: prd\nstatus: active\ntitle: Title PRD-402\n---\n\nOriginal body.\n",
        )
        .await
        .unwrap();
        receipt.snapshot.projection_path = ws.join("prds/PRD-402-title-prd-402.md").display().to_string();
        receipt.trashed_projection = trashed.display().to_string();
        write_receipt(&ws, &receipt).await.unwrap();

        let report = apply_restore_with_target(&ws, &guard, None, &receipt, Some(" draft "))
            .await
            .unwrap();
        let restored = crate::artifact::store::resolve_artifact_path(&ws, "PRD-402")
            .await
            .unwrap();
        let text = tokio::fs::read_to_string(restored).await.unwrap();
        let (fm, body) = crate::artifact::frontmatter::parse_frontmatter(&text).unwrap();
        assert_eq!(
            fm.get("status").and_then(|value| value.as_str()),
            Some("draft")
        );
        assert_eq!(body.trim(), "Original body.");
        assert_eq!(report.restored_status, "draft");
        assert!(!ws.join("lance/artifacts.lance").exists());
    }
}
