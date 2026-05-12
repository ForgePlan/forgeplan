//! Soft-delete receipt infrastructure (PRD-055, increment 1).
//!
//! Receipt format + trash I/O + TTL purge. Does NOT wire into destructive
//! tool handlers yet — that is increment 2. Does NOT add restore /
//! undo-last tools — that is increment 3.
//!
//! # Design
//!
//! When a destructive op runs we write a JSON receipt to
//! `.forgeplan/trash/<kind>-<id>-<timestamp>-<rand>.json` capturing the
//! full pre-operation state of the artifact (frontmatter + body +
//! relations). The markdown projection is MOVED into the trash
//! directory alongside the receipt, rather than hard-deleted.
//!
//! ## Crash invariant
//!
//! Order of operations is: write receipt → remove from store.
//! A crash after write_receipt leaves a harmless orphan receipt which
//! TTL purge eventually collects. A crash after remove_from_store but
//! before write_receipt would be fatal data loss, so we avoid that
//! ordering entirely.
//!
//! ## TTL purge
//!
//! Receipts older than `undo.ttl_days` (default 30) are removed on
//! demand — specifically on entry to any `forgeplan_restore` call or
//! when wrapping the first destructive op of a session. No background
//! daemon.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

pub mod restore;

/// Default TTL in days if `undo.ttl_days` is not configured.
pub const DEFAULT_TTL_DAYS: u32 = 30;

/// Which destructive operation produced this receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DestructiveOp {
    Delete,
    Supersede,
    Deprecate,
}

impl DestructiveOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Delete => "delete",
            Self::Supersede => "supersede",
            Self::Deprecate => "deprecate",
        }
    }
}

/// A relation pair captured at soft-delete time so restore can re-link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedRelation {
    /// Source artifact ID.
    pub from: String,
    /// Target artifact ID.
    pub to: String,
    /// Relation kind (e.g. "informs", "based_on", "supersedes").
    pub relation: String,
    /// Whether this artifact was the source (`Outgoing`) or the target
    /// (`Incoming`) of the relation at delete time. Restore uses this
    /// to know which direction to re-create.
    pub direction: RelationDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationDirection {
    Outgoing,
    Incoming,
}

/// The full pre-operation state of an artifact, captured so `restore`
/// can reproduce it byte-for-byte.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSnapshot {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub title: String,
    pub depth: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_epic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<String>,
    #[serde(default)]
    pub relations: Vec<CapturedRelation>,
    /// Absolute path where the markdown projection lived before the
    /// destructive op. Restore writes the body back to this path.
    pub projection_path: String,
    /// Canonical slug from frontmatter (PROB-060 Phase 2.5 closure).
    /// Captured at soft-delete time so `restore <slug>` can locate the
    /// receipt after the artifact has been removed from the main store.
    /// `None` for legacy pre-Phase-1.5 artifacts that never had a slug
    /// (lookup falls back to display id form only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
}

/// A soft-delete receipt — one JSON file per destructive operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    /// Stable unique ID of this receipt (used for CLI references).
    /// Format: `<kind>-<id>-<utc_ms>-<rand4>`.
    pub receipt_id: String,
    /// When the operation happened (ISO-8601 UTC millis).
    pub ts: String,
    /// Kind of destructive op.
    pub op: DestructiveOp,
    /// Full state of the artifact before the op.
    pub snapshot: ArtifactSnapshot,
    /// Optional reason string supplied to `forgeplan_deprecate`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Optional replacement artifact ID supplied to
    /// `forgeplan_supersede`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
    /// Absolute path (in trash) where the original markdown projection
    /// was moved to.
    pub trashed_projection: String,
    /// Activity-log entry hash this receipt corresponds to (ties the
    /// receipt to PRD-054 log entry for audit correlation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity_log_hash: Option<String>,
    /// Once a restore succeeds, the receipt is marked consumed so
    /// restore cannot double-apply the same recovery.
    #[serde(default)]
    pub consumed: bool,
}

/// Path to the trash directory inside a workspace.
pub fn trash_dir(workspace: &Path) -> PathBuf {
    workspace.join("trash")
}

/// Compose a unique receipt file path inside the trash directory.
pub fn receipt_path(workspace: &Path, receipt_id: &str) -> PathBuf {
    trash_dir(workspace).join(format!("receipt-{receipt_id}.json"))
}

/// Compose a stable path for the moved markdown body, derived from
/// receipt_id so we can find them as a pair.
pub fn trashed_projection_path(workspace: &Path, receipt_id: &str) -> PathBuf {
    trash_dir(workspace).join(format!("body-{receipt_id}.md"))
}

/// Generate a fresh receipt ID. Format: `<kind>-<id>-<utc_ms>-<rand8>`.
///
/// Uses 32-bit non-crypto PRNG (`rand::random::<u32>()`) for the suffix
/// rather than a 16-bit-masked slice of nanoseconds. Audit R2 H-1 showed
/// that the nanos-mask gives a 1-in-65_536 collision on two concurrent
/// deletes in the same millisecond — plenty under multi-agent usage
/// where `write_receipt` uses `create_new(true)` and would fail the
/// second caller. 32 bits gives ~1-in-4_294_967_296 at the same rate,
/// i.e. effectively never for realistic workloads.
pub fn generate_receipt_id(kind: &str, artifact_id: &str) -> String {
    let now = Utc::now();
    let ms = now.timestamp_millis();
    let rand: u32 = rand::random();
    format!(
        "{}-{}-{}-{:08x}",
        safe_segment(kind),
        safe_segment(artifact_id),
        ms,
        rand
    )
}

/// Make a string safe to use as a filename segment. Keeps alphanumeric
/// and `-` / `_`; replaces anything else with `_`. Not a security
/// boundary — artifact IDs are already validated upstream.
fn safe_segment(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Write a receipt to disk. Creates the trash directory on demand.
///
/// PRD-055 Architecture Decision 4: crash invariant requires this is
/// called BEFORE the corresponding store mutation. We fsync the
/// receipt file before returning so partial writes don't leave an
/// ambiguous state.
///
/// Refuses to write if the trash directory is a symlink (H-2 security
/// from Round 3 audit) — an attacker who can point `.forgeplan/trash/`
/// at a location outside the workspace could redirect the projection
/// move into arbitrary filesystem paths.
pub async fn write_receipt(workspace: &Path, receipt: &Receipt) -> anyhow::Result<PathBuf> {
    let dir = trash_dir(workspace);
    // If the directory exists but is a symlink, refuse. If it does not
    // exist yet we create it fresh (and subsequent calls will pass the
    // symlink check).
    if dir.exists() {
        let meta = tokio::fs::symlink_metadata(&dir).await?;
        if meta.file_type().is_symlink() {
            anyhow::bail!(
                "trash directory {} is a symlink — refusing to write receipt",
                dir.display()
            );
        }
    }
    tokio::fs::create_dir_all(&dir).await?;

    let path = receipt_path(workspace, &receipt.receipt_id);
    let json = serde_json::to_vec_pretty(receipt)?;

    let mut file = tokio::fs::OpenOptions::new()
        .create_new(true) // fail if collision — should never happen with 32-bit PRNG suffix
        .write(true)
        .open(&path)
        .await?;
    file.write_all(&json).await?;
    file.sync_data().await?;
    drop(file);

    // Fsync the parent directory too — on ext4/xfs a hard crash can
    // lose the create() entry even if the file's data was synced.
    // Best-effort on Windows (where directory fsync has no-op semantics).
    if let Ok(dir_handle) = std::fs::File::open(&dir) {
        let _ = tokio::task::spawn_blocking(move || dir_handle.sync_all()).await;
    }
    Ok(path)
}

/// Read back a receipt from disk.
pub async fn read_receipt(path: &Path) -> anyhow::Result<Receipt> {
    let bytes = tokio::fs::read(path).await?;
    let receipt: Receipt = serde_json::from_slice(&bytes)?;
    Ok(receipt)
}

/// Move markdown projection into trash alongside the receipt. Uses
/// `rename` which is atomic on the same filesystem. If the projection
/// is on a different mount, falls back to copy+remove.
///
/// Refuses if the source path is a symlink — prevents an attacker who
/// placed a symlink where the artifact's markdown was expected from
/// causing us to "move" system files into the trash directory.
pub async fn trash_projection(
    workspace: &Path,
    receipt_id: &str,
    original_path: &Path,
) -> anyhow::Result<PathBuf> {
    // Symlink guard on source (H-2 security from Round 3 audit).
    if let Ok(meta) = tokio::fs::symlink_metadata(original_path).await
        && meta.file_type().is_symlink()
    {
        anyhow::bail!(
            "source projection {} is a symlink — refusing to move",
            original_path.display()
        );
    }

    let dir = trash_dir(workspace);
    tokio::fs::create_dir_all(&dir).await?;
    let target = trashed_projection_path(workspace, receipt_id);

    match tokio::fs::rename(original_path, &target).await {
        Ok(()) => Ok(target),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Nothing to move — record that fact in the receipt.
            // Caller decides whether this is an error (delete on an
            // artifact that had no projection is legitimate).
            anyhow::bail!("projection not found at {}", original_path.display())
        }
        Err(e) if is_cross_device(&e) => {
            // Cross-device rename → copy + remove.
            tokio::fs::copy(original_path, &target).await?;
            tokio::fs::remove_file(original_path).await?;
            Ok(target)
        }
        Err(e) => Err(e.into()),
    }
}

fn is_cross_device(e: &std::io::Error) -> bool {
    // libc::EXDEV = 18 on Linux/macOS. Windows ERROR_NOT_SAME_DEVICE = 17.
    // raw_os_error() returns the platform-specific code.
    #[cfg(unix)]
    {
        matches!(e.raw_os_error(), Some(18))
    }
    #[cfg(windows)]
    {
        matches!(e.raw_os_error(), Some(17))
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

/// List all non-consumed receipts in the workspace trash, newest first.
pub async fn list_receipts(workspace: &Path) -> anyhow::Result<Vec<Receipt>> {
    let dir = trash_dir(workspace);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = tokio::fs::read_dir(&dir).await?;
    let mut receipts = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("receipt-") && n.ends_with(".json"))
        {
            match read_receipt(&path).await {
                Ok(r) => receipts.push(r),
                Err(e) => {
                    tracing::warn!("skipping unreadable receipt {}: {}", path.display(), e);
                }
            }
        }
    }
    // Newest first. Receipt IDs embed `<ms>` so ascending `ts` sort works.
    receipts.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok(receipts)
}

/// Find the most recent non-consumed receipt for a given artifact ID.
pub async fn find_latest_for(
    workspace: &Path,
    artifact_id: &str,
) -> anyhow::Result<Option<Receipt>> {
    let all = list_receipts(workspace).await?;
    Ok(all
        .into_iter()
        .find(|r| r.snapshot.id == artifact_id && !r.consumed))
}

/// Find the most recent non-consumed receipt by canonical slug (PROB-060
/// Phase 2.5). Receipts written before Phase 2.5 closure have no `slug`
/// stamped — those are skipped here and remain reachable via display id
/// only. Receipts for legacy pre-Phase-1.5 artifacts also have no slug
/// (no slug ever existed) and are handled identically.
///
/// Slug comparison is case-insensitive to match `resolve_id` semantics.
pub async fn find_latest_for_slug(workspace: &Path, slug: &str) -> anyhow::Result<Option<Receipt>> {
    let all = list_receipts(workspace).await?;
    Ok(all.into_iter().find(|r| {
        !r.consumed
            && r.snapshot
                .slug
                .as_deref()
                .is_some_and(|s| s.eq_ignore_ascii_case(slug))
    }))
}

/// Mark a receipt as consumed on disk (rewrite the file in place).
/// Called by restore after a successful recovery so undo-last won't
/// re-apply the same receipt.
pub async fn mark_consumed(workspace: &Path, receipt_id: &str) -> anyhow::Result<()> {
    let path = receipt_path(workspace, receipt_id);
    let mut receipt = read_receipt(&path).await?;
    if receipt.consumed {
        return Ok(());
    }
    receipt.consumed = true;
    let json = serde_json::to_vec_pretty(&receipt)?;
    // Write to a temp file and rename to atomically replace.
    let tmp = path.with_extension("json.tmp");
    tokio::fs::write(&tmp, &json).await?;
    tokio::fs::rename(&tmp, &path).await?;
    Ok(())
}

/// Purge receipts (and their paired projection bodies) older than
/// `ttl_days`. Returns number of receipts removed.
///
/// Lazily invoked from `forgeplan_restore` and from the wrapper around
/// any destructive tool. No background daemon per PRD-055 Architecture
/// Decision 5.
pub async fn purge_expired(workspace: &Path, ttl_days: u32) -> anyhow::Result<usize> {
    let dir = trash_dir(workspace);
    if !dir.exists() {
        return Ok(0);
    }
    let threshold = Utc::now() - Duration::days(i64::from(ttl_days));
    let all = list_receipts(workspace).await?;
    let mut removed = 0usize;

    for receipt in all {
        let ts = match DateTime::parse_from_rfc3339(&receipt.ts) {
            Ok(t) => t.with_timezone(&Utc),
            Err(_) => continue, // unparseable → skip, don't purge
        };
        if ts < threshold {
            let receipt_file = receipt_path(workspace, &receipt.receipt_id);
            let body_file = trashed_projection_path(workspace, &receipt.receipt_id);
            let _ = tokio::fs::remove_file(&receipt_file).await;
            let _ = tokio::fs::remove_file(&body_file).await;
            removed += 1;
        }
    }
    Ok(removed)
}

/// Capture a soft-delete receipt for an artifact and (for `Delete` ops)
/// move its markdown projection into `.forgeplan/trash/`. Returns the
/// receipt ID so the caller can include it in user-facing output for
/// `forgeplan_undo_last` / `forgeplan_restore`.
///
/// **Crash invariant**: receipt is written FIRST, projection move SECOND.
/// If the process is killed between the two, the next `restore` finds the
/// receipt and uses the snapshot to recreate the artifact (file move
/// failure is non-fatal — receipt has full body and metadata).
///
/// **Helper used by both CLI `forgeplan delete` AND MCP
/// `forgeplan_delete` / `forgeplan_supersede` / `forgeplan_deprecate`** —
/// previously lived as a private function in MCP server.rs which left CLI
/// `delete` permanently destructive (audit 2026-05-01 follow-up).
///
/// # Errors
/// Returns Err if receipt write fails. The caller MUST NOT proceed with
/// the destructive op in that case.
pub async fn soft_delete_capture(
    workspace: &std::path::Path,
    store: &crate::db::store::LanceStore,
    record: &crate::db::store::ArtifactRecord,
    op: DestructiveOp,
    reason: Option<&str>,
    replacement: Option<&str>,
) -> anyhow::Result<String> {
    use crate::artifact::types::ArtifactKind;

    // Gather outgoing + incoming relations so restore can replay both
    // directions (PRD-055 ADR #6).
    let mut relations = Vec::new();
    if let Ok(outgoing) = store.get_relations(&record.id).await {
        for (to, relation) in outgoing {
            relations.push(CapturedRelation {
                from: record.id.clone(),
                to,
                relation,
                direction: RelationDirection::Outgoing,
            });
        }
    }
    if let Ok(incoming) = store.get_incoming_relations(&record.id).await {
        for (from, relation) in incoming {
            relations.push(CapturedRelation {
                from,
                to: record.id.clone(),
                relation,
                direction: RelationDirection::Incoming,
            });
        }
    }

    // Resolve original projection path BEFORE move. Cannot trust
    // `slugify(current_title)` because the title may have been edited
    // after artifact creation, so the projection filename on disk may
    // differ. Scan the kind directory for any file matching `<ID>-*.md`
    // and take the first match. Fall back to slugify only if filesystem
    // scan fails (e.g. missing dir).
    let projection_path = if let Ok(kind) = record.kind.parse::<ArtifactKind>() {
        let kind_dir = workspace.join(kind.dir_name());
        let id_prefix = format!("{}-", record.id);
        let mut found: Option<std::path::PathBuf> = None;
        if let Ok(mut entries) = tokio::fs::read_dir(&kind_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Some(name) = entry.file_name().to_str()
                    && name.starts_with(&id_prefix)
                    && name.ends_with(".md")
                {
                    found = Some(entry.path());
                    break;
                }
            }
        }
        match found {
            Some(p) => p.display().to_string(),
            None => {
                let slug = crate::artifact::types::slugify(&record.title);
                kind_dir
                    .join(format!("{}-{slug}.md", record.id))
                    .display()
                    .to_string()
            }
        }
    } else {
        String::new()
    };

    let receipt_id = generate_receipt_id(&record.kind, &record.id);
    let trashed = trashed_projection_path(workspace, &receipt_id)
        .display()
        .to_string();

    // PROB-060 Phase 2.5: parse slug from body frontmatter so restore
    // can find this receipt by slug after the artifact is removed from
    // the main store. Best-effort: legacy pre-Phase-1.5 artifacts have
    // no slug field and that's a documented compatibility case.
    let slug = crate::artifact::frontmatter::parse_frontmatter(&record.body)
        .ok()
        .and_then(|(fm, _body)| {
            crate::artifact::frontmatter::slug_from_frontmatter(&fm).map(str::to_string)
        });

    let snapshot = ArtifactSnapshot {
        id: record.id.clone(),
        kind: record.kind.clone(),
        status: record.status.clone(),
        title: record.title.clone(),
        depth: record.depth.clone(),
        body: record.body.clone(),
        author: record.author.clone(),
        parent_epic: record.parent_epic.clone(),
        valid_until: record.valid_until.clone(),
        relations,
        projection_path: projection_path.clone(),
        slug,
    };

    let receipt = Receipt {
        receipt_id: receipt_id.clone(),
        ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        op,
        snapshot,
        reason: reason.map(String::from),
        replacement: replacement.map(String::from),
        trashed_projection: trashed,
        activity_log_hash: None,
        consumed: false,
    };

    // Write receipt first (crash invariant).
    write_receipt(workspace, &receipt).await?;

    // Move projection file into trash ONLY for Delete — supersede and
    // deprecate leave the markdown in place.
    let projection_pathbuf = std::path::PathBuf::from(&projection_path);
    if matches!(op, DestructiveOp::Delete)
        && projection_pathbuf.exists()
        && let Err(e) = trash_projection(workspace, &receipt_id, &projection_pathbuf).await
    {
        tracing::warn!(
            "soft_delete: failed to move projection {}: {}. Receipt written, artifact \
             will still be recoverable via store snapshot.",
            projection_pathbuf.display(),
            e
        );
    }

    // Fire-and-forget TTL purge so trash stays bounded.
    let ws_clone = workspace.to_path_buf();
    tokio::spawn(async move {
        if let Err(e) = purge_expired(&ws_clone, DEFAULT_TTL_DAYS).await {
            tracing::warn!("TTL purge failed: {}", e);
        }
    });

    Ok(receipt_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_snapshot(id: &str) -> ArtifactSnapshot {
        ArtifactSnapshot {
            id: id.into(),
            kind: "prd".into(),
            status: "active".into(),
            title: format!("Sample artifact {id}"),
            depth: "standard".into(),
            body: "# Body\n\nHello world.\n".into(),
            author: Some("tester".into()),
            parent_epic: None,
            valid_until: None,
            relations: vec![CapturedRelation {
                from: id.into(),
                to: "EVID-001".into(),
                relation: "informs".into(),
                direction: RelationDirection::Outgoing,
            }],
            projection_path: format!("/ws/.forgeplan/prds/{id}-sample.md"),
            slug: None,
        }
    }

    fn sample_receipt(id: &str) -> Receipt {
        let receipt_id = generate_receipt_id("prd", id);
        Receipt {
            receipt_id: receipt_id.clone(),
            ts: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            op: DestructiveOp::Delete,
            snapshot: sample_snapshot(id),
            reason: None,
            replacement: None,
            trashed_projection: format!("/ws/.forgeplan/trash/body-{receipt_id}.md"),
            activity_log_hash: Some("abc123def456".into()),
            consumed: false,
        }
    }

    #[test]
    fn receipt_id_is_unique() {
        let a = generate_receipt_id("prd", "PRD-001");
        let b = generate_receipt_id("prd", "PRD-001");
        // Best effort — with ms + nanos combined, collisions should be
        // astronomically rare. Accept equality only if clocks are at
        // nanosecond resolution the same, which is practically never.
        assert_ne!(a, b);
        assert!(a.starts_with("prd-PRD-001-"));
    }

    #[test]
    fn safe_segment_drops_path_separators() {
        assert_eq!(safe_segment("a/b"), "a_b");
        assert_eq!(safe_segment("a\\b"), "a_b");
        assert_eq!(safe_segment("PRD-001"), "PRD-001");
        assert_eq!(safe_segment("evil..id"), "evil__id");
    }

    #[tokio::test]
    async fn write_read_receipt_round_trip() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let r = sample_receipt("PRD-001");
        let path = write_receipt(&ws, &r).await.unwrap();
        assert!(path.exists());
        let back = read_receipt(&path).await.unwrap();
        assert_eq!(back.receipt_id, r.receipt_id);
        assert_eq!(back.snapshot.body, r.snapshot.body);
        assert_eq!(back.op, DestructiveOp::Delete);
        assert_eq!(back.snapshot.relations.len(), 1);
        assert_eq!(back.snapshot.relations[0].to, "EVID-001");
    }

    #[tokio::test]
    async fn list_receipts_sorted_newest_first() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        // Seed 3 receipts with increasing timestamps.
        for i in 0..3 {
            let mut r = sample_receipt(&format!("PRD-00{i}"));
            r.ts = (Utc::now() + Duration::seconds(i))
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            // Force unique receipt_id despite same-nanosecond calls
            r.receipt_id = format!("prd-PRD-00{i}-{}-aaaa", i);
            write_receipt(&ws, &r).await.unwrap();
        }
        let listed = list_receipts(&ws).await.unwrap();
        assert_eq!(listed.len(), 3);
        // Newest first.
        assert_eq!(listed[0].snapshot.id, "PRD-002");
        assert_eq!(listed[1].snapshot.id, "PRD-001");
        assert_eq!(listed[2].snapshot.id, "PRD-000");
    }

    #[tokio::test]
    async fn find_latest_for_skips_consumed() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let mut r1 = sample_receipt("PRD-001");
        r1.receipt_id = "prd-PRD-001-111-aaaa".into();
        r1.ts = "2026-04-18T10:00:00.000Z".into();
        r1.consumed = true;
        write_receipt(&ws, &r1).await.unwrap();

        let mut r2 = sample_receipt("PRD-001");
        r2.receipt_id = "prd-PRD-001-222-bbbb".into();
        r2.ts = "2026-04-18T09:00:00.000Z".into();
        // r2 is older BUT not consumed — find_latest_for should return it.
        write_receipt(&ws, &r2).await.unwrap();

        let found = find_latest_for(&ws, "PRD-001").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().receipt_id, "prd-PRD-001-222-bbbb");
    }

    #[tokio::test]
    async fn mark_consumed_persists() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let r = sample_receipt("PRD-001");
        let receipt_id = r.receipt_id.clone();
        write_receipt(&ws, &r).await.unwrap();
        mark_consumed(&ws, &receipt_id).await.unwrap();

        let path = receipt_path(&ws, &receipt_id);
        let back = read_receipt(&path).await.unwrap();
        assert!(back.consumed);
    }

    #[tokio::test]
    async fn mark_consumed_idempotent() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let r = sample_receipt("PRD-001");
        let receipt_id = r.receipt_id.clone();
        write_receipt(&ws, &r).await.unwrap();
        mark_consumed(&ws, &receipt_id).await.unwrap();
        // Second call must not fail.
        mark_consumed(&ws, &receipt_id).await.unwrap();
    }

    #[tokio::test]
    async fn purge_removes_expired_keeps_fresh() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        // Expired receipt — 35 days old.
        let mut old = sample_receipt("PRD-OLD");
        old.receipt_id = "prd-PRD-OLD-111-aaaa".into();
        old.ts =
            (Utc::now() - Duration::days(35)).to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let old_path = write_receipt(&ws, &old).await.unwrap();
        // Pretend there's a paired body file.
        let body_path = trashed_projection_path(&ws, &old.receipt_id);
        tokio::fs::write(&body_path, b"old body").await.unwrap();

        // Fresh receipt — 1 day old.
        let mut fresh = sample_receipt("PRD-FRESH");
        fresh.receipt_id = "prd-PRD-FRESH-222-bbbb".into();
        fresh.ts =
            (Utc::now() - Duration::days(1)).to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let fresh_path = write_receipt(&ws, &fresh).await.unwrap();

        let removed = purge_expired(&ws, 30).await.unwrap();
        assert_eq!(removed, 1);
        assert!(!old_path.exists(), "expired receipt should be removed");
        assert!(!body_path.exists(), "paired body should be removed");
        assert!(fresh_path.exists(), "fresh receipt must survive");
    }

    #[tokio::test]
    async fn purge_on_empty_trash_returns_zero() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let removed = purge_expired(&ws, 30).await.unwrap();
        assert_eq!(removed, 0);
    }

    #[tokio::test]
    async fn trash_projection_moves_file() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let src_dir = tmp.path().join(".forgeplan/prds");
        tokio::fs::create_dir_all(&src_dir).await.unwrap();
        let src = src_dir.join("PRD-001-sample.md");
        tokio::fs::write(&src, b"# PRD-001\n\nHello\n")
            .await
            .unwrap();

        let target = trash_projection(&ws, "prd-PRD-001-111-aaaa", &src)
            .await
            .unwrap();
        assert!(target.exists(), "projection moved into trash");
        assert!(!src.exists(), "original is gone");
        let content = tokio::fs::read_to_string(&target).await.unwrap();
        assert!(content.contains("Hello"));
    }

    #[tokio::test]
    async fn trash_projection_missing_file_errors() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let nowhere = tmp.path().join("ghost.md");
        let err = trash_projection(&ws, "prd-x-111-aaaa", &nowhere).await;
        assert!(err.is_err(), "missing file should error");
    }

    #[test]
    fn destructive_op_round_trips_through_serde() {
        for op in [
            DestructiveOp::Delete,
            DestructiveOp::Supersede,
            DestructiveOp::Deprecate,
        ] {
            let s = serde_json::to_string(&op).unwrap();
            let back: DestructiveOp = serde_json::from_str(&s).unwrap();
            assert_eq!(op, back);
        }
    }

    #[tokio::test]
    async fn corrupted_receipt_file_does_not_break_list() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let dir = trash_dir(&ws);
        tokio::fs::create_dir_all(&dir).await.unwrap();
        // Write a bogus receipt file.
        tokio::fs::write(dir.join("receipt-bogus.json"), b"this is not json")
            .await
            .unwrap();
        // And one good one.
        let good = sample_receipt("PRD-001");
        write_receipt(&ws, &good).await.unwrap();
        let listed = list_receipts(&ws).await.unwrap();
        assert_eq!(listed.len(), 1, "good receipt survives corrupt neighbour");
    }

    // ── Audit Round 3 regression tests ────────────────────────

    #[test]
    fn receipt_id_uses_32bit_prng_suffix() {
        // Audit H-1 logic: collision protection. Previous 16-bit
        // nanos-masked suffix collided at ~1/65_536. Now 32-bit PRNG.
        // Probabilistic — generate N receipts, assert all unique.
        let mut ids = std::collections::HashSet::new();
        for _ in 0..1000 {
            let id = generate_receipt_id("prd", "PRD-001");
            assert!(ids.insert(id.clone()), "duplicate receipt_id: {id}");
        }
        // Verify format: the last segment is 8 hex chars.
        let sample = generate_receipt_id("prd", "PRD-001");
        let last_segment = sample.rsplit('-').next().unwrap();
        assert_eq!(
            last_segment.len(),
            8,
            "suffix should be 8 hex chars (32-bit)"
        );
        assert!(
            last_segment.chars().all(|c| c.is_ascii_hexdigit()),
            "suffix must be hex"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn write_receipt_refuses_symlinked_trash_dir() {
        // Audit H-2 security: if .forgeplan/trash/ is a symlink to
        // somewhere outside the workspace, write_receipt must refuse
        // (otherwise the subsequent projection move redirects to the
        // symlink target).
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();

        // Create a symlink from .forgeplan/trash -> outside.
        let outside = tmp.path().join("outside");
        tokio::fs::create_dir_all(&outside).await.unwrap();
        std::os::unix::fs::symlink(&outside, ws.join("trash")).unwrap();

        let receipt = sample_receipt("PRD-001");
        let err = write_receipt(&ws, &receipt).await.unwrap_err();
        assert!(
            err.to_string().contains("symlink"),
            "symlinked trash dir must be refused: {err}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn trash_projection_refuses_symlinked_source() {
        // Audit H-2 security: source projection being a symlink must
        // be refused — otherwise destructive op moves the symlink
        // target (could be user config files).
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();

        // Target the symlink would point at.
        let sensitive = tmp.path().join("sensitive.txt");
        tokio::fs::write(&sensitive, b"secrets").await.unwrap();

        // Plant a symlink where the projection would be.
        let src_dir = ws.join("prds");
        tokio::fs::create_dir_all(&src_dir).await.unwrap();
        let src = src_dir.join("PRD-001-evil.md");
        std::os::unix::fs::symlink(&sensitive, &src).unwrap();

        let err = trash_projection(&ws, "prd-PRD-001-1-deadbeef", &src)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("symlink"),
            "symlinked source must be refused: {err}"
        );
        // Sensitive file must not have moved.
        assert!(sensitive.exists(), "sensitive file must not be moved");
    }
}
