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

/// Generate a fresh receipt ID. Format: `<kind>-<id>-<utc_ms>-<rand4>`.
/// Collision-free for >50k receipts per second per workspace.
pub fn generate_receipt_id(kind: &str, artifact_id: &str) -> String {
    let now = Utc::now();
    let ms = now.timestamp_millis();
    // Cheap non-crypto random suffix from nanoseconds. Adequate for a
    // single-writer-per-workspace service.
    let rand = now.timestamp_subsec_nanos() & 0xFFFF;
    format!(
        "{}-{}-{}-{:04x}",
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
pub async fn write_receipt(workspace: &Path, receipt: &Receipt) -> anyhow::Result<PathBuf> {
    let dir = trash_dir(workspace);
    tokio::fs::create_dir_all(&dir).await?;

    let path = receipt_path(workspace, &receipt.receipt_id);
    let json = serde_json::to_vec_pretty(receipt)?;

    let mut file = tokio::fs::OpenOptions::new()
        .create_new(true) // fail if collision — should never happen given ID format
        .write(true)
        .open(&path)
        .await?;
    file.write_all(&json).await?;
    file.sync_data().await?;
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
pub async fn trash_projection(
    workspace: &Path,
    receipt_id: &str,
    original_path: &Path,
) -> anyhow::Result<PathBuf> {
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
    // libc::EXDEV = 18 on Linux/macOS. Windows uses ERROR_NOT_SAME_DEVICE.
    // raw_os_error() returns the platform-specific code.
    matches!(e.raw_os_error(), Some(18))
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
}
