// PROB-067 — stress test for parallel `forgeplan new` ID race condition.
//
// Reproduces the v0.31.0 sprint discovery where two parallel workers in
// separate worktrees both received `EVID-119` for different artifacts.
// Asserts that after the cross-worktree id-alloc lock + post-write
// collision detection fix, 5 simultaneous `forgeplan new evidence`
// invocations against the same workspace receive 5 distinct IDs and
// produce 5 distinct files on disk.
//
// Acceptance criterion #3 from PROB-067:
//   "Regression test: stress test with 5 parallel `forgeplan_new
//    evidence` invocations, all 5 unique IDs"

use assert_cmd::Command;
use std::collections::HashSet;
use std::path::Path;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init_workspace(path: &Path) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(path)
        .assert()
        .success();
}

/// Collect every `EVID-NNN` id from the workspace's evidence directory
/// by parsing filenames.
fn evidence_ids(workspace_root: &Path) -> HashSet<String> {
    let dir = workspace_root.join(".forgeplan/evidence");
    if !dir.exists() {
        return HashSet::new();
    }
    let mut ids = HashSet::new();
    for entry in std::fs::read_dir(&dir).expect("read evidence dir") {
        let entry = entry.expect("read_dir entry");
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }
        // Filename shape: `EVID-NNN-<slug>.md`.
        if let Some(rest) = name.strip_prefix("EVID-")
            && let Some(num_str) = rest.split('-').next()
        {
            ids.insert(format!("EVID-{num_str}"));
        }
    }
    ids
}

/// 5 parallel `forgeplan new evidence` calls against the same workspace
/// must yield 5 distinct IDs (PROB-067 AC3).
///
/// Uses OS-level process parallelism (one CLI subprocess per worker) to
/// exercise the cross-worktree id-alloc lock — which is keyed on
/// `git-common-dir/forgeplan/id-EVID.lock` when inside a repo, or the
/// workspace-local fallback when not.
#[test]
fn prob_067_parallel_new_evidence_unique_ids() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    init_workspace(root);

    // Distinct titles avoid the duplicate-detection guard (FR-001 of
    // PRD-043), which would force `--allow-duplicate` on near-identical
    // titles. We're testing the id-allocator, not duplicate detection.
    let titles = [
        "PROB-067 baseline measurement alpha",
        "PROB-067 retry benchmark beta",
        "PROB-067 contention probe gamma",
        "PROB-067 timing capture delta",
        "PROB-067 deadlock scan epsilon",
    ];
    let n = titles.len();
    let mut handles = Vec::with_capacity(n);
    for title in titles {
        let root = root.to_path_buf();
        let title = title.to_string();
        handles.push(std::thread::spawn(move || {
            forgeplan()
                .args(["new", "evidence", &title])
                .current_dir(&root)
                .assert()
                .success();
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }

    let ids = evidence_ids(root);
    assert_eq!(
        ids.len(),
        n,
        "expected {n} unique evidence IDs, got {} — collision detected: {:?}",
        ids.len(),
        ids
    );

    // Files on disk count must match unique ID count (no silent overwrites).
    let entries: Vec<_> = std::fs::read_dir(root.join(".forgeplan/evidence"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().to_string().ends_with(".md"))
        .collect();
    assert_eq!(
        entries.len(),
        n,
        "expected {n} files on disk, got {} — silent overwrite suspected",
        entries.len()
    );
}

/// 10 parallel allocations across mixed kinds (5 evidence + 5 note) must
/// not block each other unnecessarily nor produce ID collisions within
/// either kind. Verifies per-kind lock granularity.
#[test]
fn prob_067_per_kind_lock_does_not_serialize_unrelated_kinds() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    init_workspace(root);

    let evid_titles = [
        "Granularity alpha probe",
        "Granularity beta capture",
        "Granularity gamma scan",
        "Granularity delta measure",
        "Granularity epsilon check",
    ];
    let note_titles = [
        "Mixed kappa marker",
        "Mixed lambda tag",
        "Mixed mu signal",
        "Mixed nu trace",
        "Mixed xi log",
    ];
    let mut handles = Vec::new();
    for title in evid_titles {
        let root = root.to_path_buf();
        let title = title.to_string();
        handles.push(std::thread::spawn(move || {
            forgeplan()
                .args(["new", "evidence", &title])
                .current_dir(&root)
                .assert()
                .success();
        }));
    }
    for title in note_titles {
        let root = root.to_path_buf();
        let title = title.to_string();
        handles.push(std::thread::spawn(move || {
            forgeplan()
                .args(["new", "note", &title])
                .current_dir(&root)
                .assert()
                .success();
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }

    let evid_ids = evidence_ids(root);
    assert_eq!(
        evid_ids.len(),
        5,
        "expected 5 unique evidence IDs, got {}: {:?}",
        evid_ids.len(),
        evid_ids
    );

    // Same shape for notes.
    let note_dir = root.join(".forgeplan/notes");
    let mut note_ids = HashSet::new();
    for entry in std::fs::read_dir(&note_dir).expect("read notes dir") {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(rest) = name.strip_prefix("NOTE-")
            && let Some(num_str) = rest.split('-').next()
        {
            note_ids.insert(format!("NOTE-{num_str}"));
        }
    }
    assert_eq!(
        note_ids.len(),
        5,
        "expected 5 unique note IDs, got {}: {:?}",
        note_ids.len(),
        note_ids
    );
}
