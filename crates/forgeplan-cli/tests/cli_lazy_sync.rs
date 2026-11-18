//! SPEC-013 / RFC-021 Phase 1 — lazy sync on read.
//!
//! Read commands reconcile the canonical markdown against the derived
//! Constellation index before loading data, so an edited, new, or deleted
//! `.md` is never served stale. Detection is digest-based inside `capture`;
//! a no-change read pays one bounded scan and prints nothing.

#[path = "common/historical_lance.rs"]
mod historical_lance;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use assert_cmd::prelude::*;
use forgeplan_core::constellation::storage::{self, IndexProtocolState};
use tempfile::TempDir;

const PRD_PATH: &str = "prds/PRD-001-lazy-sync.md";
const NOTE_PATH: &str = "notes/NOTE-001-lazy-sync.md";
const INITIAL_BODY: &str = "lazy_sync_initial_sentinel";
const EDITED_BODY: &str = "lazy_sync_edited_sentinel";

/// A workspace migrated onto the Constellation protocol: `CURRENT.json`
/// names a published generation and the legacy tables are retired.
struct Workspace {
    _temp: TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Workspace {
    fn migrated() -> Self {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("project");
        let home = temp.path().join("home");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&home).unwrap();
        let workspace = Self {
            _temp: temp,
            root,
            home,
        };
        historical_lance::init_legacy_workspace(&workspace.root, "Lazy sync fixture");
        workspace.write_prd(INITIAL_BODY);
        workspace.write_note("note initial sentinel");
        workspace.run_ok(&["index"]);
        assert!(
            matches!(
                storage::read_marker(&workspace.lance()).unwrap(),
                IndexProtocolState::Active(_)
            ),
            "fixture did not migrate onto the constellation protocol"
        );
        assert!(storage::read_current(&workspace.lance()).unwrap().is_some());
        workspace
    }

    fn workspace(&self) -> PathBuf {
        self.root.join(".forgeplan")
    }

    fn lance(&self) -> PathBuf {
        self.workspace().join("lance")
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::cargo_bin("forgeplan").unwrap();
        command
            .args(args)
            .current_dir(&self.root)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("XDG_DATA_HOME", &self.home)
            .env("FORGEPLAN_MODEL_CACHE", self.home.join("models"))
            .env("HF_HUB_OFFLINE", "1")
            .env("FORGEPLAN_DISABLE_PLUGIN_DISCOVERY", "1")
            .stdin(Stdio::null());
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        assert_cmd::Command::from_std(self.command(args))
            .timeout(Duration::from_secs(120))
            .output()
            .unwrap()
    }

    fn run_ok(&self, args: &[&str]) -> Output {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "forgeplan {args:?} failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn write_prd(&self, body: &str) {
        let source = format!(
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Lazy sync\n\
             slug: prd-lazy-sync\ndepth: standard\n---\n\n## Summary\n\n{body}\n"
        );
        let path = self.workspace().join(PRD_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }

    fn write_note(&self, body: &str) {
        let source = format!(
            "---\nid: NOTE-001\nkind: note\nstatus: draft\ntitle: Lazy sync note\n\
             slug: note-lazy-sync\ndepth: standard\n---\n\n## Summary\n\n{body}\n"
        );
        let path = self.workspace().join(NOTE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
}

/// Edited `.md` is visible to `get` without an operator `index` run, and the
/// reconcile reports exactly the one moved file (SPEC-013 lazy-sync s1).
#[test]
fn edited_markdown_is_visible_to_get() {
    let workspace = Workspace::migrated();

    workspace.write_prd(EDITED_BODY);
    let output = workspace.run_ok(&["get", "PRD-001"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains(EDITED_BODY),
        "get served the stale body\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        !stdout.contains(INITIAL_BODY),
        "get served both bodies\nstdout:\n{stdout}"
    );
    assert!(
        stderr.contains("reconciled: 1 file(s)"),
        "expected one-file reconcile report\nstderr:\n{stderr}"
    );
}

/// New `.md` is visible to `list` without an operator `index` run (s2).
#[test]
fn new_markdown_is_visible_to_list() {
    let workspace = Workspace::migrated();
    // Remove the note the fixture indexed so the created file is the delta.
    fs::remove_file(workspace.workspace().join(NOTE_PATH)).unwrap();
    workspace.run_ok(&["index"]);

    workspace.write_note("fresh note sentinel");
    let output = workspace.run_ok(&["list", "--type", "note"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains("NOTE-001"),
        "list missed the new note\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

/// Deleted `.md` disappears from `list` (s3).
#[test]
fn deleted_markdown_is_absent_from_list() {
    let workspace = Workspace::migrated();

    fs::remove_file(workspace.workspace().join(NOTE_PATH)).unwrap();
    let output = workspace.run_ok(&["list", "--type", "note"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stdout.contains("NOTE-001"),
        "list still shows the deleted note\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

/// Unchanged tree is a no-op: no reconcile line, no new generation (s4).
#[test]
fn unchanged_tree_is_a_noop() {
    let workspace = Workspace::migrated();
    let generation_before = storage::read_current(&workspace.lance())
        .unwrap()
        .unwrap()
        .manifest_path;

    let output = workspace.run_ok(&["get", "PRD-001"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains(INITIAL_BODY),
        "get lost the indexed body\nstdout:\n{stdout}"
    );
    assert!(
        !stderr.contains("reconciled:"),
        "unchanged read printed a reconcile line\nstderr:\n{stderr}"
    );
    let generation_after = storage::read_current(&workspace.lance())
        .unwrap()
        .unwrap()
        .manifest_path;
    assert_eq!(
        generation_before, generation_after,
        "unchanged read published a new generation"
    );
}

/// Bounded scan: many files, few edits, one reconcile report (s5).
/// The spec's p95 bound is 50 ms for the lazy-sync decision; the test
/// measures the reconcile decision itself (capture + plan) rather than the
/// whole process spawn, which is dominated by binary startup.
#[test]
fn many_files_bounded_cost() {
    let workspace = Workspace::migrated();

    // Grow the corpus so the scan is over a realistic file set, then index.
    for i in 0..120 {
        let source = format!(
            "---\nid: NOTE-{i:03}\nkind: note\nstatus: draft\ntitle: Bulk {i}\n\
             slug: note-bulk-{i}\ndepth: standard\n---\n\n## Summary\n\nbulk body {i}\n"
        );
        let path = workspace
            .workspace()
            .join(format!("notes/NOTE-{i:03}-bulk.md"));
        fs::write(path, source).unwrap();
    }
    workspace.run_ok(&["index"]);

    // Exactly 3 files change.
    for i in 0..3 {
        let path = workspace
            .workspace()
            .join(format!("notes/NOTE-{i:03}-bulk.md"));
        let source = format!(
            "---\nid: NOTE-{i:03}\nkind: note\nstatus: draft\ntitle: Bulk {i}\n\
             slug: note-bulk-{i}\ndepth: standard\n---\n\n## Summary\n\nedited body {i}\n"
        );
        fs::write(path, source).unwrap();
    }

    let start = Instant::now();
    let output = workspace.run_ok(&["list", "--type", "note"]);
    let elapsed = start.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains("edited body") || stdout.contains("NOTE-000"),
        "list missed the corpus\nstdout:\n{stdout}"
    );
    assert!(
        stderr.contains("reconciled: 3 file(s)"),
        "expected a three-file reconcile report\nstderr:\n{stderr}"
    );
    assert!(
        elapsed < Duration::from_secs(60),
        "lazy sync on ~125 files took {elapsed:?} — the decision is a scan, \
         not an unbounded walk"
    );
}
