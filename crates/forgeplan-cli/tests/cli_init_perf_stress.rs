//! PROB-068 perf stress — `forgeplan init --force` auto-backup at scale.
//!
//! Functional correctness is covered by `cli_init_safety.rs`. This file
//! pins the **runtime + content-integrity contract** at a workspace size
//! a real user could plausibly hit (1000 artifacts).
//!
//! Threshold rationale:
//! - **< 30s** — production-acceptable. `init --force` is interactive
//!   one-shot maintenance, not a hot path.
//! - **30–60s** — degraded but tolerable; logged in EVID, no PROB filed.
//! - **> 60s** — file a follow-up PROB and consider tightening
//!   `create_force_backup` (parallel copy, hard-link strategy).
//!
//! Tests bypass the CLI subprocess overhead on the *populate* path —
//! writing 1000 markdown files via `forgeplan new` would take many
//! minutes and dominate measurement noise. The `init --force` step
//! itself still runs through the real binary so the measurement
//! reflects shipping behaviour.

use std::fs;
use std::path::Path;
use std::time::Instant;

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Bootstrap a `.forgeplan/` workspace and then materialise `total`
/// realistic artifact markdown files directly on disk. We round-robin
/// across the artifact kinds so the backup walks every `ARTIFACT_DIRS`
/// subdir, not just `prds/`.
fn populate_workspace_at_scale(root: &Path, total: usize) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(root)
        .assert()
        .success();

    // Subset of ARTIFACT_DIRS that map to real `kind:` values produced
    // by `forgeplan new`. We avoid `memory/`, `discovery/`, and
    // `refresh/` here so the populated tree mirrors what a normal user
    // workspace looks like (PRDs / RFCs / ADRs / specs / problems /
    // solutions / evidence / notes / epics).
    let layout: &[(&str, &str, &str)] = &[
        ("prds", "prd", "PRD"),
        ("rfcs", "rfc", "RFC"),
        ("adrs", "adr", "ADR"),
        ("specs", "spec", "SPEC"),
        ("epics", "epic", "EPIC"),
        ("problems", "problem", "PROB"),
        ("solutions", "solution", "SOL"),
        ("evidence", "evidence", "EVID"),
        ("notes", "note", "NOTE"),
    ];

    for i in 0..total {
        let (dir, kind, prefix) = layout[i % layout.len()];
        // Per-kind sequence number so filenames stay realistic.
        let seq = (i / layout.len()) + 1;
        let id = format!("{prefix}-{:04}", seq);
        let slug = format!("perf-stress-canary-{:04}", i);
        let filename = format!("{}-{}.md", id, slug);

        let body = format!(
            "---\n\
             id: {id}\n\
             kind: {kind}\n\
             title: Perf Stress Canary {i:04}\n\
             status: draft\n\
             depth: standard\n\
             author: perf-stress\n\
             tags:\n  - source=perf-test\n  - cohort=stress\n\
             links:\n  - target: PROB-068\n    relation: informs\n\
             custom_index: {i}\n\
             ---\n\
             # Perf Stress Canary {i:04}\n\
             \n\
             ## Problem\n\
             \n\
             Artifact {i} of {total} in the PROB-068 1000-artifact stress\n\
             corpus. The backup path must copy this entire body byte-equal\n\
             so the post-backup integrity sample passes.\n\
             \n\
             ## Goals\n\
             \n\
             - exercise the backup walker over every ARTIFACT_DIRS entry\n\
             - keep file sizes representative of real PRDs (~500 bytes)\n\
             - never collide IDs across the cohort\n\
             \n\
             ## Notes\n\
             \n\
             cohort index = {i}; kind = {kind}; seq = {seq}.\n",
            i = i,
            total = total,
            id = id,
            kind = kind,
            seq = seq,
        );

        let path = root.join(".forgeplan").join(dir).join(&filename);
        fs::write(&path, body).expect("write artifact body");
    }
}

/// Walk all `.forgeplan-backup-*` directories under `root` and return the
/// newest one (lexicographic order matches chronological because the
/// timestamp suffix is `YYYYMMDD-HHMMSS`).
fn newest_backup_dir(root: &Path) -> Option<std::path::PathBuf> {
    let mut best: Option<std::path::PathBuf> = None;
    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(".forgeplan-backup-") {
            continue;
        }
        match &best {
            None => best = Some(entry.path()),
            Some(prev) if entry.path() > *prev => best = Some(entry.path()),
            _ => {}
        }
    }
    best
}

/// Sum the on-disk size of every regular file under `root`. Used purely
/// for the perf log line; never asserted on.
fn dir_total_bytes(root: &Path) -> u64 {
    let mut total: u64 = 0;
    let mut stack: Vec<std::path::PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file()
                && let Ok(meta) = entry.metadata()
            {
                total += meta.len();
            }
        }
    }
    total
}

/// PROB-068 perf threshold: `init --force` with auto-backup must finish
/// under 30s on a 1000-artifact workspace. The cohort spans every
/// ARTIFACT_DIRS subdir we ship today.
///
/// We use a generous-but-bounded threshold so transient CI noise doesn't
/// flake the test; a sustained regression past 30s is real.
#[test]
fn init_force_backup_1000_artifacts_under_30s() {
    let tmp = TempDir::new().unwrap();
    let n: usize = 1000;
    populate_workspace_at_scale(tmp.path(), n);

    // Sanity check the populate step before timing the actual subject.
    let prds_count = fs::read_dir(tmp.path().join(".forgeplan/prds"))
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .map(|e| e.file_name().to_string_lossy().ends_with(".md"))
                .unwrap_or(false)
        })
        .count();
    assert!(
        prds_count > 0,
        "populate_workspace_at_scale produced no PRDs — populator regression"
    );

    let start = Instant::now();
    forgeplan()
        .args(["init", "-y", "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();
    let elapsed = start.elapsed();

    let backup_dir = newest_backup_dir(tmp.path())
        .expect("--force with default backup created no .forgeplan-backup-* directory");
    let backup_bytes = dir_total_bytes(&backup_dir);

    // Stderr-style perf log so the runner output captures the number.
    // `eprintln!` is OK in tests — the CI smoke harness only redirects
    // forgeplan binary stdout, not cargo test stderr.
    eprintln!(
        "PROB-068 stress: n={} runtime={:?} backup={} backup_size={} bytes",
        n,
        elapsed,
        backup_dir.display(),
        backup_bytes,
    );

    assert!(
        elapsed.as_secs() < 30,
        "PROB-068 perf regression: init --force took {:?} on {n} artifacts, expected < 30s",
        elapsed,
    );
}

/// PROB-068 integrity: regardless of runtime, the backup MUST contain a
/// byte-equal copy of every artifact body. Sampling 50 deterministic
/// indices keeps the assertion fast while still catching partial-copy
/// regressions.
#[test]
fn init_force_backup_preserves_all_bodies() {
    let tmp = TempDir::new().unwrap();
    let n: usize = 1000;
    populate_workspace_at_scale(tmp.path(), n);

    // Snapshot each sample BEFORE the backup so we compare against the
    // exact pre-force bytes, not the live post-force file (which is
    // also preserved, but we want the backup itself to match).
    //
    // Sample 50 indices spread across the cohort: a stride of ~20
    // exercises every kind in `layout` thanks to round-robin assignment.
    let stride = n / 50;
    let sampled_indices: Vec<usize> = (0..50).map(|k| k * stride).collect();
    let layout: &[(&str, &str)] = &[
        ("prds", "PRD"),
        ("rfcs", "RFC"),
        ("adrs", "ADR"),
        ("specs", "SPEC"),
        ("epics", "EPIC"),
        ("problems", "PROB"),
        ("solutions", "SOL"),
        ("evidence", "EVID"),
        ("notes", "NOTE"),
    ];

    let mut expected: Vec<(std::path::PathBuf, String)> = Vec::with_capacity(50);
    for &i in &sampled_indices {
        let (dir, prefix) = layout[i % layout.len()];
        let seq = (i / layout.len()) + 1;
        let filename = format!("{}-{:04}-perf-stress-canary-{:04}.md", prefix, seq, i);
        let live = tmp.path().join(".forgeplan").join(dir).join(&filename);
        let body = fs::read_to_string(&live).unwrap_or_else(|e| {
            panic!(
                "populate step produced no file at {} (sample i={}): {e}",
                live.display(),
                i
            )
        });
        expected.push((std::path::PathBuf::from(dir).join(&filename), body));
    }

    forgeplan()
        .args(["init", "-y", "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let backup_dir = newest_backup_dir(tmp.path())
        .expect("--force with default backup created no .forgeplan-backup-* directory");

    let mut mismatches: Vec<String> = Vec::new();
    for (rel_path, expected_body) in &expected {
        let backed_up = backup_dir.join(rel_path);
        match fs::read_to_string(&backed_up) {
            Ok(actual) if actual == *expected_body => {}
            Ok(actual) => mismatches.push(format!(
                "{}: body diverged ({} vs {} bytes)",
                rel_path.display(),
                actual.len(),
                expected_body.len(),
            )),
            Err(e) => mismatches.push(format!("{}: missing in backup — {e}", rel_path.display())),
        }
    }

    assert!(
        mismatches.is_empty(),
        "PROB-068 integrity regression — {} mismatch(es):\n{}",
        mismatches.len(),
        mismatches.join("\n"),
    );
}
