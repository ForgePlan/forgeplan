//! Phase 5 Wave 4 — end-to-end integration tests for `forgeplan playbook`.
//!
//! Unlike `cli_playbook.rs` (Wave 3 unit-style tests with inline YAML),
//! this suite drives the full pipeline against a real on-disk fixture
//! [`brownfield-code-mini.yaml`] that exercises three of the five
//! `delegate_to` variants (plugin / skill / forgeplan_core) plus a
//! `requires:` DAG and per-step `fallback_hint`.
//!
//! Test setup pattern (per w3a):
//!   1. `TempDir` workspace
//!   2. `forgeplan init -y`
//!   3. Copy fixture into `<tmp>/.forgeplan/playbooks/`
//!   4. Drive CLI via `assert_cmd`
//!
//! Each test sets `FORGEPLAN_DISABLE_PLUGIN_DISCOVERY=1` so the host
//! machine's `~/.claude/plugins/*/playbooks/` cannot leak into the
//! fixture playbook count.
//!
//! Smoke tests against `target/release/forgeplan` are gated with
//! `#[ignore]` so they only run on demand (`cargo test --release --
//! --ignored`); the release build is too slow for the default CI
//! pipeline.
//!
//! AC traceability (PRD-065):
//!   AC-1 -> e2e_playbook_validate_fixture
//!   AC-2 -> e2e_playbook_run_dry_run_lists_steps
//!   AC-3 -> e2e_playbook_run_yes_writes_journal
//!   AC-4 -> covered by `cli_playbook::playbook_validate_unknown_step_ref_lists_pairs`
//!           and dispatcher missing-plugin path (Wave 3 unit tests in
//!           `forgeplan-core::playbook::executor::tests`); fixture path
//!           covered indirectly because every step has `fallback_hint`.
//!   AC-5 -> covered by `cli_playbook::playbook_validate_bad_file_*`
//!   AC-6 -> entire suite green = AC-6 satisfied.

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

// =====================================================================
// Helpers
// =====================================================================

fn forgeplan() -> Command {
    let mut cmd = Command::cargo_bin("forgeplan").expect("test fixture: cargo_bin forgeplan");
    cmd.env("FORGEPLAN_DISABLE_PLUGIN_DISCOVERY", "1");
    cmd
}

fn init_workspace() -> TempDir {
    let tmp = TempDir::new().expect("test fixture: TempDir::new");
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    tmp
}

/// Path to the fixture file shipped alongside this test, relative to
/// `CARGO_MANIFEST_DIR` (the `forgeplan-cli` crate root).
fn fixture_path(rel: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

/// Copy the brownfield-code-mini fixture into the temp workspace's
/// `.forgeplan/playbooks/` so discovery (`list`/`show`/`run`) finds it.
fn install_fixture_playbook(tmp: &TempDir) -> std::path::PathBuf {
    let src = fixture_path("playbooks/brownfield-code-mini.yaml");
    let dst_dir = tmp.path().join(".forgeplan").join("playbooks");
    std::fs::create_dir_all(&dst_dir).expect("test fixture: create playbooks dir");
    let dst = dst_dir.join("brownfield-code-mini.yaml");
    std::fs::copy(&src, &dst).expect("test fixture: copy fixture playbook");
    dst
}

// =====================================================================
// AC-1 (validate fixture)
// =====================================================================

#[test]
fn e2e_playbook_validate_fixture() {
    // Fresh workspace — but `validate` takes a *file path* directly, so
    // we don't even need to install into `.forgeplan/playbooks/`.
    let tmp = init_workspace();
    let fixture = fixture_path("playbooks/brownfield-code-mini.yaml");

    // Text mode: assert exit 0 + report mentions name.
    forgeplan()
        .args(["playbook", "validate", fixture.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("OK: brownfield-code-mini"))
        .stdout(predicate::str::contains("Done."));

    // JSON mode: assert structured payload includes step count = 3.
    let out = forgeplan()
        .args(["playbook", "validate", fixture.to_str().unwrap(), "--json"])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: run validate --json");
    assert!(out.status.success(), "validate must succeed on fixture");
    let v: Value =
        serde_json::from_slice(&out.stdout).expect("test fixture: validate output is JSON");
    assert_eq!(v["passed"], true);
    assert_eq!(v["name"], "brownfield-code-mini");
    assert_eq!(
        v["steps_count"], 3,
        "fixture must contain exactly 3 steps; got: {v}"
    );
}

// =====================================================================
// `show` renders all 3 step IDs
// =====================================================================

#[test]
fn e2e_playbook_show_renders_structure() {
    let tmp = init_workspace();
    install_fixture_playbook(&tmp);

    let assertion = forgeplan()
        .args(["playbook", "show", "brownfield-code-mini"])
        .current_dir(tmp.path())
        .assert()
        .success()
        // All 3 step IDs must be present.
        .stdout(predicate::str::contains("mine-history"))
        .stdout(predicate::str::contains("extract-c4"))
        .stdout(predicate::str::contains("validate-graph"))
        // Phase 6 Wave 4: fixture now uses 3× skill delegates (ALL skill —
        // no plugin/forgeplan_core), so we assert presence of skill labels
        // representing each step.
        .stdout(predicate::str::contains("skill:forge-history-miner"))
        .stdout(predicate::str::contains("skill:c4-extractor-stub"))
        // The `Next:` hint must point at the run command.
        .stdout(predicate::str::contains(
            "Next: forgeplan playbook run brownfield-code-mini",
        ));

    // Sanity: `show --json` returns the playbook object with 3 steps.
    let _ = assertion;
    let out = forgeplan()
        .args(["playbook", "show", "brownfield-code-mini", "--json"])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: run show --json");
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).expect("test fixture: show output is JSON");
    let steps = v["playbook"]["steps"]
        .as_array()
        .expect("test fixture: steps array");
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0]["id"], "mine-history");
    assert_eq!(steps[1]["id"], "extract-c4");
    assert_eq!(steps[2]["id"], "validate-graph");
}

// =====================================================================
// AC-2 (dry-run lists steps, no journal)
// =====================================================================

#[test]
fn e2e_playbook_run_dry_run_lists_steps() {
    let tmp = init_workspace();
    install_fixture_playbook(&tmp);

    let journal_path = tmp
        .path()
        .join(".forgeplan")
        .join("journal")
        .join("playbook-runs.jsonl");
    assert!(
        !journal_path.exists(),
        "pre-condition: no journal yet at {journal_path:?}"
    );

    forgeplan()
        .args([
            "playbook",
            "run",
            "brownfield-code-mini",
            "--yes",
            "--dry-run",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry-run: brownfield-code-mini"))
        .stdout(predicate::str::contains("mine-history"))
        .stdout(predicate::str::contains("extract-c4"))
        .stdout(predicate::str::contains("validate-graph"))
        .stdout(predicate::str::contains(
            "Next: forgeplan playbook run brownfield-code-mini --yes",
        ));

    // Dry-run MUST NOT write the journal — that's the whole point.
    assert!(
        !journal_path.exists(),
        "dry-run must not write journal; found {journal_path:?}"
    );
}

// =====================================================================
// AC-3 (real run writes RunStart / 3x StepStart+StepEnd / RunEnd)
// =====================================================================

#[test]
fn e2e_playbook_run_yes_writes_journal() {
    let tmp = init_workspace();
    install_fixture_playbook(&tmp);

    // Real run with --yes (uses MockDispatcher::AlwaysOk per Wave 3 stub).
    let out = forgeplan()
        .args(["playbook", "run", "brownfield-code-mini", "--yes", "--json"])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: run playbook with --yes");
    assert!(
        out.status.success(),
        "run failed: stderr={}\nstdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout),
    );

    // Phase 6 Wave 4: SkillDispatcher emits trace line `[skill-invoke] /...`
    // to stdout BEFORE the JSON payload. Strip leading non-JSON to parse.
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json_start = stdout
        .find('{')
        .expect("test fixture: run output must contain JSON object");
    let v: Value =
        serde_json::from_str(&stdout[json_start..]).expect("test fixture: run output is JSON");
    assert_eq!(v["report"]["success"], 3, "all 3 steps must succeed: {v}");
    assert_eq!(v["report"]["failed"], 0);
    assert_eq!(v["report"]["skipped"], 0);
    // Clean run -> terminal next action (per Wave 3 hint contract).
    assert!(v["_next_action"].is_null(), "clean run must terminate: {v}");

    // Journal must exist + contain RunStart, 3xStepStart+StepEnd, RunEnd.
    let journal_path = tmp
        .path()
        .join(".forgeplan")
        .join("journal")
        .join("playbook-runs.jsonl");
    assert!(
        journal_path.exists(),
        "journal must exist at {journal_path:?}"
    );

    let body = std::fs::read_to_string(&journal_path).expect("test fixture: read journal");
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
    // 1 RunStart + 3 StepStart + 3 StepEnd + 1 RunEnd = 8 entries.
    assert_eq!(
        lines.len(),
        8,
        "expected 8 journal entries (1 RunStart + 3 StepStart + 3 StepEnd + 1 RunEnd), got: {lines:#?}"
    );

    // Parse and tally entry kinds.
    let mut run_start = 0usize;
    let mut step_start = 0usize;
    let mut step_end = 0usize;
    let mut run_end = 0usize;
    for line in &lines {
        let entry: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("test fixture: journal line is JSON ({e}): {line}"));
        // `JournalEntryKind` derives `serde(rename_all = "snake_case")`,
        // so the on-disk kind is `run_start` / `step_start` / `step_end`
        // / `run_end` (not the PascalCase Rust variant).
        match entry["kind"].as_str().unwrap_or("") {
            "run_start" => run_start += 1,
            "step_start" => step_start += 1,
            "step_end" => step_end += 1,
            "run_end" => run_end += 1,
            other => panic!("unexpected journal kind: {other} in line: {line}"),
        }
    }
    assert_eq!(run_start, 1, "exactly one RunStart");
    assert_eq!(step_start, 3, "one StepStart per fixture step");
    assert_eq!(step_end, 3, "one StepEnd per fixture step");
    assert_eq!(run_end, 1, "exactly one RunEnd");
}

// =====================================================================
// Smoke test (release binary) — gated #[ignore]
// =====================================================================
//
// `cargo build --release` is slow (~5-10 min); these tests only run on
// demand via:
//
//     cargo test -p forgeplan --tests integration_phase5_playbook \
//         --release -- --ignored
//
// They prove the cargo-published artifact (which `homebrew-forgeplan` and
// `install.sh` ship) actually starts and emits the contractual hint
// markers. Ideal Wave 4 audit smoke check.

/// Locate the release binary built by `cargo build --release`. Returns
/// `None` if it has not been built yet (the test will then skip).
fn locate_release_binary() -> Option<std::path::PathBuf> {
    // CARGO_MANIFEST_DIR points at crates/forgeplan-cli; release lives at
    // ../../target/release/forgeplan.
    let candidate = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("release")
        .join("forgeplan");
    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

#[test]
#[ignore = "requires release binary (cargo build --release first); run via --ignored"]
fn smoke_release_binary_playbook_list_empty_workspace() {
    let bin = match locate_release_binary() {
        Some(p) => p,
        None => {
            eprintln!(
                "skip: release binary not found at target/release/forgeplan; \
                 build it first with `cargo build --release`"
            );
            return;
        }
    };
    let cwd = TempDir::new().expect("test fixture: TempDir for release smoke");

    let out = std::process::Command::new(&bin)
        .args(["playbook", "list"])
        .env("FORGEPLAN_DISABLE_PLUGIN_DISCOVERY", "1")
        .current_dir(cwd.path())
        .output()
        .expect("test fixture: spawn release binary");
    assert!(
        out.status.success(),
        "release binary failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Empty workspace -> "No playbooks found." + Done. terminal hint.
    assert!(stdout.contains("No playbooks found"), "stdout={stdout}");
    assert!(stdout.contains("Done."), "stdout={stdout}");
}

#[test]
#[ignore = "requires release binary (cargo build --release first); run via --ignored"]
fn smoke_release_binary_plugins_list_json_is_valid() {
    let bin = match locate_release_binary() {
        Some(p) => p,
        None => {
            eprintln!("skip: release binary not found; build with `cargo build --release`");
            return;
        }
    };
    let home = TempDir::new().expect("test fixture: clean HOME");
    let cwd = TempDir::new().expect("test fixture: cwd");

    let out = std::process::Command::new(&bin)
        .args(["plugins", "list", "--json"])
        // Empty HOME so filesystem scanner deterministically reports no
        // installed plugins (only the synthetic forgeplan entry).
        .env("HOME", home.path())
        .current_dir(cwd.path())
        .output()
        .expect("test fixture: spawn release binary");
    assert!(
        out.status.success(),
        "plugins list --json failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("release binary did not emit JSON: {e}\n{stdout}"));
    assert!(v["installed"].is_array());
}
