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

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

/// Append-only JSONL writer.
///
/// Holds the underlying file handle open for the lifetime of the run.
/// Drop closes the file via `std::fs::File`'s `Drop`.
pub struct Journal {
    /// Resolved path to the JSONL file.
    path: PathBuf,
    /// Lazily opened on first append; allows construction in dry-run paths
    /// without touching the filesystem.
    file: Option<File>,
}

impl Journal {
    /// Open (or create) the journal under `<workspace_root>/.forgeplan/journal/playbook-runs.jsonl`.
    /// Creates the parent directory if missing.
    ///
    /// # Errors
    /// Any [`io::Error`] propagated from `create_dir_all` / `OpenOptions`.
    pub fn open(workspace_root: &Path) -> io::Result<Self> {
        let dir = workspace_root.join(".forgeplan").join("journal");
        let path = dir.join("playbook-runs.jsonl");
        fs::create_dir_all(&dir)?;
        let file = OpenOptions::new().append(true).create(true).open(&path)?;
        Ok(Self {
            path,
            file: Some(file),
        })
    }

    /// Open at an explicit path (used by tests + custom journal locations).
    /// Creates the parent directory if missing.
    pub fn open_at(path: PathBuf) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().append(true).create(true).open(&path)?;
        Ok(Self {
            path,
            file: Some(file),
        })
    }

    /// Path the journal writes to (useful for diagnostics).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one entry as a single JSON line + `\n`.
    ///
    /// # Errors
    /// `io::Error` on write failure or `serde_json::Error` rendered as
    /// `io::Error` if the entry can't be serialized (in practice never —
    /// every field is `Serialize`).
    pub fn append(&mut self, entry: &JournalEntry) -> io::Result<()> {
        let line = serde_json::to_string(entry)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| io::Error::other("journal file handle not open"))?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        Ok(())
    }

    /// Flush the OS file buffer to disk. Cheap no-op when there's no handle.
    pub fn flush(&mut self) -> io::Result<()> {
        if let Some(file) = self.file.as_mut() {
            file.flush()?;
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
    use std::io::Read;
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

    /// Append round-trip via tempfile: write one entry, read back, parse.
    #[test]
    fn append_round_trip_via_tempfile() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("test.jsonl");
        let mut journal = Journal::open_at(path.clone()).expect("open");
        let run_id = RunId::new();
        journal
            .append(&make_entry(run_id, JournalEntryKind::RunStart, None))
            .expect("append");
        journal.flush().expect("flush");

        let mut contents = String::new();
        File::open(&path)
            .expect("read")
            .read_to_string(&mut contents)
            .expect("read");
        assert_eq!(contents.lines().count(), 1);
        let parsed: JournalEntry =
            serde_json::from_str(contents.lines().next().expect("line")).expect("parse");
        assert_eq!(parsed.run_id, run_id);
        assert_eq!(parsed.kind, JournalEntryKind::RunStart);
        assert_eq!(parsed.playbook_name, "demo");
    }

    /// Multi-entry append preserves order and lines parse independently.
    #[test]
    fn multi_entry_preserves_order() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("multi.jsonl");
        let mut journal = Journal::open_at(path.clone()).expect("open");
        let run_id = RunId::new();
        journal
            .append(&make_entry(run_id, JournalEntryKind::RunStart, None))
            .expect("a");
        journal
            .append(&make_entry(run_id, JournalEntryKind::StepStart, Some("s1")))
            .expect("b");
        journal
            .append(&make_entry(run_id, JournalEntryKind::StepEnd, Some("s1")))
            .expect("c");
        journal
            .append(&make_entry(run_id, JournalEntryKind::RunEnd, None))
            .expect("d");
        journal.flush().expect("flush");

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
    #[test]
    fn open_creates_missing_journal_dir() {
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
            .expect("append");
        journal.flush().expect("flush");
        assert!(expected.exists());
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
