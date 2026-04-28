//! Append-only JSONL journal for Playbook runs.
//!
//! Each line is a self-contained [`JournalEntry`] keyed by `run_id` so a
//! resumable run can replay the file and skip already-completed steps
//! (PRD-065 FR-6). Default location:
//!
//! ```text
//! <workspace>/.forgeplan/journal/playbook-runs.jsonl
//! ```
//!
//! The file is opened in append mode and never truncated. Callers should
//! reuse one [`Journal`] handle per run for efficiency, but multiple
//! runs can share the same file safely (each line is independently
//! parseable).
//!
//! # Run identifiers
//!
//! Wave 2 uses a random `u64` (rendered as 16-char lower-hex) for
//! [`RunId`] because `uuid` isn't a workspace dependency yet. Wave 3
//! may upgrade to UUID v7 once `uuid` is added — see the report attached
//! to this sprint.
//!
//! # Async I/O + buffering (CRIT-P2, Audit Round 1)
//!
//! [`Journal::append`] is `async` and writes through a
//! [`tokio::io::BufWriter`] over [`tokio::fs::File`]. This keeps the
//! tokio worker thread non-blocking during playbook runs (the executor is
//! already in an async context) and coalesces JSON-line writes into a
//! single syscall per flush instead of two per entry.
//!
//! # Per-step durability (NEW-S-H2, Audit Round 2)
//!
//! `RunStart` and `StepStart` entries are buffered for performance. Each
//! `StepEnd` is followed by an explicit [`Journal::flush`] call from the
//! executor — this guarantees that, on a process crash mid-run, every
//! step that *finished* is durably recorded. PRD-065 FR-6 (resumable
//! runs) relies on this: recovery treats a missing `StepEnd` (after the
//! corresponding `StepStart`) as "step was in flight when we crashed —
//! retry it", which is the correct fail-safe semantic. A fully-buffered
//! journal would lose the last `StepEnd` on crash, leading the resumer
//! to falsely retry a step that actually completed.
//!
//! Cost: one `fsync`-grade syscall per step. At hundreds of steps per
//! run this is well within budget; the alternative (best-effort journal,
//! no resumability) was rejected.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::io::{self, AsyncWriteExt, BufWriter};

/// 64-bit random run identifier rendered as 16-char lower-hex.
///
/// Stub for Wave 2; Wave 3 may swap for UUID v7 once a workspace `uuid`
/// dep lands. Equality is by raw `u64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunId(pub u64);

impl RunId {
    /// Generate a fresh `RunId` from the OS-provided RNG (the same
    /// `rand` crate already in `forgeplan-core` deps).
    pub fn new() -> Self {
        Self(rand::random())
    }

    /// 16-char lower-hex render — stable across (de)serialization.
    pub fn to_hex(self) -> String {
        format!("{:016x}", self.0)
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Serialize for RunId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for RunId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        u64::from_str_radix(&s, 16)
            .map(RunId)
            .map_err(serde::de::Error::custom)
    }
}

/// Kind of journal entry — emitted at the boundaries of a run and each step.
///
/// `#[non_exhaustive]` so future entry kinds (e.g. `Heartbeat`,
/// `Checkpoint` for resumable parallel runs) can be added without
/// breaking downstream `match` arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum JournalEntryKind {
    /// Emitted once per run, before any step starts.
    RunStart,
    /// Emitted before each step's `Dispatcher::dispatch` call.
    StepStart,
    /// Emitted after each step's dispatch returns (success or failure).
    StepEnd,
    /// Emitted once per run after all steps are processed.
    RunEnd,
}

/// One JSONL line in the playbook journal.
///
/// `payload` is free-form JSON: success boolean, stderr blob, error message,
/// summary counts. Schema-light by design — readers must tolerate added
/// payload fields (forward compat).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    /// UTC timestamp when the entry was emitted.
    pub ts: DateTime<Utc>,
    /// Identifier shared by every entry in one run.
    pub run_id: RunId,
    /// `Playbook::name` (kebab-case identifier, not title).
    pub playbook_name: String,
    /// Step ID for `StepStart`/`StepEnd`; `None` for run boundaries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
    /// Which boundary the entry marks.
    pub kind: JournalEntryKind,
    /// Free-form payload (success flag, stderr, summary, …). Defaults to
    /// `null` when callers don't need extra context.
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// Append-only JSONL writer (async + buffered).
///
/// Holds a [`tokio::io::BufWriter`] over [`tokio::fs::File`] for the
/// lifetime of the run; entries are coalesced and written without
/// blocking the tokio worker thread (CRIT-P2, Audit Round 1).
///
/// `Drop` flushes the OS handle implicitly via `BufWriter`'s `Drop` only
/// for the file descriptor — buffered bytes are NOT auto-flushed on drop.
/// Callers MUST invoke [`Journal::flush`] explicitly at end-of-run to
/// guarantee durability of the final entries.
///
/// Durability contract (NEW-S-H2, Audit Round 2):
/// * `RunStart` / `StepStart` are buffered; on crash they may be lost
///   without affecting correctness (recovery only inspects `StepEnd`).
/// * Every `StepEnd` is flushed immediately by the executor via
///   [`Journal::flush`] so PRD-065 FR-6 (resumable runs) can trust the
///   journal's tail when the process is killed mid-run.
pub struct Journal {
    /// Resolved path to the JSONL file.
    path: PathBuf,
    /// Lazily opened on first append; allows construction in dry-run paths
    /// without touching the filesystem.
    writer: Option<BufWriter<tokio::fs::File>>,
}

impl Journal {
    /// Open (or create) the journal under `<workspace_root>/.forgeplan/journal/playbook-runs.jsonl`.
    /// Creates the parent directory if missing.
    ///
    /// Synchronous filesystem prep keeps callers (CLI/MCP) free to open
    /// journals from non-async contexts; the actual write path goes
    /// through async [`Self::append`].
    ///
    /// # Errors
    /// Any [`io::Error`] propagated from `create_dir_all` / `OpenOptions`.
    pub fn open(workspace_root: &Path) -> io::Result<Self> {
        let dir = workspace_root.join(".forgeplan").join("journal");
        let path = dir.join("playbook-runs.jsonl");
        fs::create_dir_all(&dir)?;
        // We open the std file, then convert to tokio::fs::File. This
        // keeps Journal::open synchronous (CLI calls it from non-async
        // contexts) while routing all writes through async I/O.
        let std_file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)?;
        let tokio_file = tokio::fs::File::from_std(std_file);
        Ok(Self {
            path,
            writer: Some(BufWriter::new(tokio_file)),
        })
    }

    /// Open at an explicit path (used by tests + custom journal locations).
    /// Creates the parent directory if missing.
    pub fn open_at(path: PathBuf) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let std_file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)?;
        let tokio_file = tokio::fs::File::from_std(std_file);
        Ok(Self {
            path,
            writer: Some(BufWriter::new(tokio_file)),
        })
    }

    /// Path the journal writes to (useful for diagnostics).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one entry as a single JSON line + `\n`.
    ///
    /// Buffered: bytes land in an in-process `BufWriter` and are flushed
    /// by [`Self::flush`] (called by the executor on `RunEnd`).
    ///
    /// # Errors
    /// `io::Error` on write failure or `serde_json::Error` rendered as
    /// `io::Error` if the entry can't be serialized (in practice never —
    /// every field is `Serialize`).
    pub async fn append(&mut self, entry: &JournalEntry) -> io::Result<()> {
        // Build the serialized payload outside the borrow on `self.writer`
        // so we can return early on serde failure.
        let mut line = serde_json::to_string(entry)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        line.push('\n');
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| io::Error::other("journal file handle not open"))?;
        writer.write_all(line.as_bytes()).await
    }

    /// Flush the BufWriter and the underlying OS file buffer to disk.
    ///
    /// Must be called at the end of every run to guarantee durability;
    /// `BufWriter::Drop` does not flush. Cheap no-op when no handle is
    /// open.
    pub async fn flush(&mut self) -> io::Result<()> {
        if let Some(writer) = self.writer.as_mut() {
            writer.flush().await?;
            // `flush` on BufWriter only drains the in-memory buffer to the
            // underlying file. Sync the file's data to disk so committed
            // entries survive a process crash.
            writer.get_mut().sync_data().await?;
        }
        Ok(())
    }
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_entry(run_id: RunId, kind: JournalEntryKind, step_id: Option<&str>) -> JournalEntry {
        JournalEntry {
            ts: Utc::now(),
            run_id,
            playbook_name: "demo".to_string(),
            step_id: step_id.map(String::from),
            kind,
            payload: serde_json::json!({"detail": "ok"}),
        }
    }

    /// Append round-trip via tempfile: write one entry, flush, read back, parse.
    #[tokio::test]
    async fn append_round_trip_via_tempfile() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("test.jsonl");
        let mut journal = Journal::open_at(path.clone()).expect("open");
        let run_id = RunId::new();
        journal
            .append(&make_entry(run_id, JournalEntryKind::RunStart, None))
            .await
            .expect("append");
        journal.flush().await.expect("flush");

        let contents = fs::read_to_string(&path).expect("read");
        assert_eq!(contents.lines().count(), 1);
        let parsed: JournalEntry =
            serde_json::from_str(contents.lines().next().expect("line")).expect("parse");
        assert_eq!(parsed.run_id, run_id);
        assert_eq!(parsed.kind, JournalEntryKind::RunStart);
        assert_eq!(parsed.playbook_name, "demo");
    }

    /// Multi-entry append preserves order and lines parse independently.
    #[tokio::test]
    async fn multi_entry_preserves_order() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("multi.jsonl");
        let mut journal = Journal::open_at(path.clone()).expect("open");
        let run_id = RunId::new();
        journal
            .append(&make_entry(run_id, JournalEntryKind::RunStart, None))
            .await
            .expect("a");
        journal
            .append(&make_entry(run_id, JournalEntryKind::StepStart, Some("s1")))
            .await
            .expect("b");
        journal
            .append(&make_entry(run_id, JournalEntryKind::StepEnd, Some("s1")))
            .await
            .expect("c");
        journal
            .append(&make_entry(run_id, JournalEntryKind::RunEnd, None))
            .await
            .expect("d");
        journal.flush().await.expect("flush");

        let contents = fs::read_to_string(&path).expect("read");
        let lines: Vec<_> = contents.lines().collect();
        assert_eq!(lines.len(), 4);

        let kinds: Vec<JournalEntryKind> = lines
            .iter()
            .map(|l| serde_json::from_str::<JournalEntry>(l).expect("parse").kind)
            .collect();
        assert_eq!(
            kinds,
            vec![
                JournalEntryKind::RunStart,
                JournalEntryKind::StepStart,
                JournalEntryKind::StepEnd,
                JournalEntryKind::RunEnd,
            ]
        );
    }

    /// `Journal::open` creates the journal directory under a fresh workspace.
    #[tokio::test]
    async fn open_creates_missing_journal_dir() {
        let dir = tempdir().expect("tempdir");
        let workspace = dir.path().join("freshworkspace");
        fs::create_dir_all(&workspace).expect("workspace");

        let mut journal = Journal::open(&workspace).expect("open");
        let expected = workspace
            .join(".forgeplan")
            .join("journal")
            .join("playbook-runs.jsonl");
        assert_eq!(journal.path(), expected);
        assert!(expected.parent().expect("parent").exists());

        // Smoke: append works.
        journal
            .append(&make_entry(RunId::new(), JournalEntryKind::RunStart, None))
            .await
            .expect("append");
        journal.flush().await.expect("flush");
        assert!(expected.exists());
    }

    /// CRIT-P2 (Audit Round 1): durability test — write 5 entries, flush,
    /// drop the journal, then re-open the file and confirm all 5 lines are
    /// present in correct order. Guards against `BufWriter::Drop` losing
    /// buffered bytes.
    #[tokio::test]
    async fn flush_persists_all_entries_in_order() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("durable.jsonl");
        let run_id = RunId(0xdead_beef_cafe_f00d);
        {
            let mut journal = Journal::open_at(path.clone()).expect("open");
            for i in 0..5 {
                let entry = JournalEntry {
                    ts: Utc::now(),
                    run_id,
                    playbook_name: "durable".to_string(),
                    step_id: Some(format!("s{i}")),
                    kind: JournalEntryKind::StepEnd,
                    payload: serde_json::json!({"i": i}),
                };
                journal.append(&entry).await.expect("append");
            }
            journal.flush().await.expect("flush");
            // Journal dropped here.
        }
        let contents = fs::read_to_string(&path).expect("read");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 5, "expected 5 lines, got {}", lines.len());
        for (i, line) in lines.iter().enumerate() {
            let parsed: JournalEntry = serde_json::from_str(line).expect("parse");
            assert_eq!(parsed.run_id, run_id);
            assert_eq!(parsed.step_id.as_deref(), Some(format!("s{i}").as_str()));
            assert_eq!(parsed.payload["i"], i);
        }
    }

    /// NEW-S-H2 (Audit Round 2): per-step durability test. We append
    /// `RunStart + StepStart + StepEnd`, call `flush` (mirroring what the
    /// executor does after every `StepEnd`), then drop the journal handle
    /// **without** another flush call to simulate a process crash. All
    /// three lines must already be on disk because the explicit flush
    /// pushed them through the BufWriter.
    #[tokio::test]
    async fn journal_step_end_is_durable() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("durable_step.jsonl");
        let run_id = RunId(0xfeed_face_dead_beef);
        {
            let mut journal = Journal::open_at(path.clone()).expect("open");
            journal
                .append(&JournalEntry {
                    ts: Utc::now(),
                    run_id,
                    playbook_name: "durable".into(),
                    step_id: None,
                    kind: JournalEntryKind::RunStart,
                    payload: serde_json::Value::Null,
                })
                .await
                .expect("RunStart");
            journal
                .append(&JournalEntry {
                    ts: Utc::now(),
                    run_id,
                    playbook_name: "durable".into(),
                    step_id: Some("s1".into()),
                    kind: JournalEntryKind::StepStart,
                    payload: serde_json::Value::Null,
                })
                .await
                .expect("StepStart");
            journal
                .append(&JournalEntry {
                    ts: Utc::now(),
                    run_id,
                    playbook_name: "durable".into(),
                    step_id: Some("s1".into()),
                    kind: JournalEntryKind::StepEnd,
                    payload: serde_json::json!({"success": true}),
                })
                .await
                .expect("StepEnd");
            // Mirrors the executor's per-StepEnd flush. Subsequent crash
            // (drop without final flush) must not lose committed lines.
            journal.flush().await.expect("flush after StepEnd");
            // Journal dropped here — simulates crash before RunEnd.
        }
        let contents = fs::read_to_string(&path).expect("read");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(
            lines.len(),
            3,
            "all 3 lines (RunStart/StepStart/StepEnd) must be durable, got {}: {contents:?}",
            lines.len()
        );
        // StepEnd must be the last entry and parseable.
        let last: JournalEntry = serde_json::from_str(lines[2]).expect("parse");
        assert_eq!(last.kind, JournalEntryKind::StepEnd);
        assert_eq!(last.step_id.as_deref(), Some("s1"));
    }

    /// `RunId::to_hex` produces stable 16-char output and round-trips via serde.
    #[test]
    fn run_id_hex_and_serde_round_trip() {
        let id = RunId(0x0123_4567_89ab_cdef);
        assert_eq!(id.to_hex(), "0123456789abcdef");

        // Serde via JSON.
        let json = serde_json::to_string(&id).expect("ser");
        assert_eq!(json, "\"0123456789abcdef\"");
        let back: RunId = serde_json::from_str(&json).expect("de");
        assert_eq!(back, id);

        // Display matches to_hex.
        assert_eq!(id.to_string(), "0123456789abcdef");
    }
}
