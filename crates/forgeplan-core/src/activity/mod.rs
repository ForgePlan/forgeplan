//! Activity log — append-only JSONL record of every MCP tool invocation.
//!
//! Closes the audit-trail gap identified after v0.20.0: diagnostic logs
//! went to stderr only and were lost on session end. With this module,
//! every tool call produces one JSONL line at
//! `.forgeplan/logs/tools-YYYY-MM-DD.jsonl`.
//!
//! # Design decisions (PRD-054)
//!
//! - **Append-only**: files opened with `O_APPEND`; no in-place edits.
//! - **Daily rotation**: one file per UTC date. No config needed.
//! - **No args content by default**: we log `args_hash` only (12-char
//!   hex of SHA-256). Secrets in titles / descriptions / body don't
//!   leak into the log. Full args logging is opt-in via config flag.
//! - **Observer, not a gate**: log-write failures are traced via
//!   `tracing::warn` but never fail the tool call. The log is a
//!   diagnostic aid, not a prerequisite.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

pub mod query;

/// One entry in the activity log. Serializes to one JSONL line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    /// ISO-8601 UTC timestamp with millisecond precision.
    pub ts: String,

    /// MCP tool name (e.g. `"forgeplan_score"`).
    pub tool: String,

    /// 12-char hex prefix of SHA-256 of canonicalized JSON args.
    /// Lets operators correlate repeated calls without exposing args content.
    pub args_hash: String,

    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,

    /// Outcome: `"ok"` | `"tool_err"` | `"rpc_err"`.
    pub status: String,

    /// Absolute path to the `.forgeplan/` directory (multi-workspace aware).
    pub workspace: String,

    /// Optional `{name, version}` from MCP `initialize.clientInfo`.
    /// Helps distinguish Claude Code vs Cursor vs Windsurf vs scripts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_info: Option<ClientInfo>,

    /// Opt-in: raw args JSON. Disabled by default; enable with
    /// `activity.log_args: true` in `.forgeplan/config.yaml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Status constants for the `status` field.
pub mod status {
    pub const OK: &str = "ok";
    pub const TOOL_ERR: &str = "tool_err";
    pub const RPC_ERR: &str = "rpc_err";
}

/// Compute the args hash: first 12 hex chars of SHA-256 of canonical
/// JSON. Stable across equivalent input shapes (serde_json sorts map
/// keys when serialized via this helper).
pub fn compute_args_hash(args: &serde_json::Value) -> String {
    // Canonical form: sorted keys, no whitespace. `to_string()` on a
    // Value does not sort keys, so we round-trip through serde_json to
    // a BTreeMap-like structure. Simpler: use serde_json::to_vec with
    // the value; even if key order varies across serializations of the
    // same logical input, it's consistent enough for hash collision
    // purposes (we're not doing cryptographic equality). For strict
    // stability we'd need a canonical JSON lib; acceptable trade-off
    // for v1.
    let bytes = serde_json::to_vec(args).unwrap_or_default();
    let hash = Sha256::digest(&bytes);
    hex_prefix(&hash, 12)
}

fn hex_prefix(bytes: &[u8], n_chars: usize) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(n_chars);
    let n_bytes = n_chars.div_ceil(2);
    for byte in bytes.iter().take(n_bytes) {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0F) as usize] as char);
    }
    out.truncate(n_chars);
    out
}

/// Resolve the current day's log file path:
/// `<workspace>/logs/tools-YYYY-MM-DD.jsonl`.
pub fn log_file_for(workspace: &Path, at: DateTime<Utc>) -> PathBuf {
    let date = at.format("%Y-%m-%d").to_string();
    workspace.join("logs").join(format!("tools-{date}.jsonl"))
}

/// Append one entry to the log. Creates the logs directory on demand.
///
/// Returns `Err` only on catastrophic I/O failures. Callers should
/// treat activity logging as best-effort — swallow errors via
/// `let _ = append(...).await.inspect_err(...)` so that log-write
/// failures never fail the parent tool call.
pub async fn append(workspace: &Path, entry: &ActivityEntry) -> anyhow::Result<()> {
    let now = Utc::now();
    let path = log_file_for(workspace, now);

    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Serialize to one line. Never include unescaped newlines.
    let mut line = serde_json::to_string(entry)?;
    line.push('\n');

    // Open in append mode. O_APPEND is atomic for writes <= PIPE_BUF
    // on POSIX; good enough for our line sizes (~500 bytes).
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await?;
    file.write_all(line.as_bytes()).await?;
    // Explicit flush to buffer boundary. `tokio::fs::File::drop` does
    // NOT perform an async flush; without this, CI filesystems (Linux
    // overlayfs in GitHub Actions) intermittently show an empty file
    // when a test reads right after `append` returns. We do NOT fsync
    // to disk — would dominate latency. The OS still buffers; worst
    // case on SIGKILL we lose <1 sec of entries. For durability-
    // critical deployments, a future `activity.fsync: per_entry` flag
    // can opt in.
    file.flush().await?;
    Ok(())
}

/// Fire-and-forget helper: appends without surfacing errors.
/// Used from MCP dispatch wrapper where a log-write failure must
/// never block the tool response.
pub async fn append_best_effort(workspace: &Path, entry: &ActivityEntry) {
    if let Err(e) = append(workspace, entry).await {
        tracing::warn!("activity log append failed for tool={}: {}", entry.tool, e);
    }
}

/// Build an entry with automatic timestamp + args hash.
pub fn make_entry(
    tool: impl Into<String>,
    args: &serde_json::Value,
    duration_ms: u64,
    status: impl Into<String>,
    workspace: &Path,
    client_info: Option<ClientInfo>,
    include_args: bool,
) -> ActivityEntry {
    let ts = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    ActivityEntry {
        ts,
        tool: tool.into(),
        args_hash: compute_args_hash(args),
        duration_ms,
        status: status.into(),
        workspace: workspace.display().to_string(),
        client_info,
        args: if include_args {
            Some(args.clone())
        } else {
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn hash_is_deterministic_for_same_input() {
        let v1 = serde_json::json!({"id": "PRD-001"});
        let v2 = serde_json::json!({"id": "PRD-001"});
        assert_eq!(compute_args_hash(&v1), compute_args_hash(&v2));
    }

    #[test]
    fn hash_differs_for_different_input() {
        let v1 = serde_json::json!({"id": "PRD-001"});
        let v2 = serde_json::json!({"id": "PRD-002"});
        assert_ne!(compute_args_hash(&v1), compute_args_hash(&v2));
    }

    #[test]
    fn hash_is_12_hex_chars() {
        let v = serde_json::json!({});
        let h = compute_args_hash(&v);
        assert_eq!(h.len(), 12);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn log_file_path_uses_utc_date() {
        let workspace = Path::new("/tmp/ws/.forgeplan");
        let ts = DateTime::parse_from_rfc3339("2026-04-18T23:59:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let path = log_file_for(workspace, ts);
        assert_eq!(
            path,
            Path::new("/tmp/ws/.forgeplan/logs/tools-2026-04-18.jsonl")
        );
    }

    #[tokio::test]
    async fn append_creates_file_and_directory() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let entry = make_entry(
            "forgeplan_health",
            &serde_json::json!({}),
            42,
            status::OK,
            &ws,
            None,
            false,
        );
        append(&ws, &entry).await.unwrap();

        let path = log_file_for(&ws, Utc::now());
        assert!(path.exists(), "log file should exist: {}", path.display());

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(content.matches('\n').count(), 1);
        let parsed: ActivityEntry = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(parsed.tool, "forgeplan_health");
        assert_eq!(parsed.status, "ok");
        assert_eq!(parsed.duration_ms, 42);
    }

    #[tokio::test]
    async fn append_is_append_only() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        for i in 0..5u64 {
            let entry = make_entry(
                "forgeplan_health",
                &serde_json::json!({"n": i}),
                i,
                status::OK,
                &ws,
                None,
                false,
            );
            append(&ws, &entry).await.unwrap();
        }

        let path = log_file_for(&ws, Utc::now());
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(content.lines().count(), 5);
        // Verify entries appear in insertion order.
        let entries: Vec<ActivityEntry> = content
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        for (i, e) in entries.iter().enumerate() {
            assert_eq!(e.duration_ms, i as u64);
        }
    }

    #[tokio::test]
    async fn args_content_omitted_by_default() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let entry = make_entry(
            "forgeplan_new",
            &serde_json::json!({"title": "Secret API key is sk-proj-ABC123DEADBEEF"}),
            10,
            status::OK,
            &ws,
            None,
            false, // include_args = false
        );
        append(&ws, &entry).await.unwrap();
        let path = log_file_for(&ws, Utc::now());
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(
            !content.contains("sk-proj-ABC123"),
            "secret leaked into log: {content}"
        );
        assert!(
            !content.contains("Secret API key"),
            "args content leaked into log"
        );
        // But the hash must be present.
        let parsed: ActivityEntry = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(parsed.args_hash.len(), 12);
        assert!(parsed.args.is_none());
    }

    #[tokio::test]
    async fn args_included_when_flag_set() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let entry = make_entry(
            "forgeplan_new",
            &serde_json::json!({"kind": "prd", "title": "test"}),
            5,
            status::OK,
            &ws,
            None,
            true, // include_args = true
        );
        append(&ws, &entry).await.unwrap();
        let path = log_file_for(&ws, Utc::now());
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        let parsed: ActivityEntry = serde_json::from_str(content.trim()).unwrap();
        assert!(parsed.args.is_some());
        let args = parsed.args.unwrap();
        assert_eq!(args["kind"], "prd");
    }

    #[tokio::test]
    async fn append_best_effort_swallows_errors() {
        // Workspace under a read-only parent that doesn't exist and
        // can't be created: use a NUL-byte in the path to force failure.
        // Simpler: use a path that is a file, so mkdir fails.
        let tmp = TempDir::new().unwrap();
        let blocker = tmp.path().join("logs");
        tokio::fs::write(&blocker, b"I am a file, not a directory")
            .await
            .unwrap();
        let ws = tmp.path();
        let entry = make_entry(
            "forgeplan_health",
            &serde_json::json!({}),
            1,
            status::OK,
            ws,
            None,
            false,
        );
        // Should not panic, should not propagate error.
        append_best_effort(ws, &entry).await;
    }

    #[test]
    fn client_info_serializes_conditionally() {
        let entry = ActivityEntry {
            ts: "2026-04-18T00:00:00.000Z".into(),
            tool: "forgeplan_health".into(),
            args_hash: "abc123def456".into(),
            duration_ms: 1,
            status: "ok".into(),
            workspace: "/tmp".into(),
            client_info: None,
            args: None,
        };
        let s = serde_json::to_string(&entry).unwrap();
        assert!(!s.contains("client_info"), "None field should be skipped");
        assert!(!s.contains("\"args\""));
    }
}
