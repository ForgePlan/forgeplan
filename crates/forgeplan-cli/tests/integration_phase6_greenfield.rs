//! Phase 6 Wave 4 — end-to-end integration tests for the canonical
//! `greenfield-kickoff` playbook (PRD-072 FR-7 / AC-7 / AC-8).
//!
//! Drives the real `forgeplan` binary against the marketplace fixture
//! [`marketplace/playbooks/greenfield-kickoff.yaml`], asserting:
//!
//! - the playbook validates clean (AC-7)
//! - `show` exposes all 7 step IDs and the correct delegate labels
//! - `--dry-run` lists every step without writing the journal
//! - `--yes` produces a journal with 1 RunStart + 7×(StepStart/StepEnd) +
//!   1 RunEnd = 16 entries
//!
//! On this branch the `--yes` path runs through `MockDispatcher::AlwaysOk`
//! because `commands::playbook::run_execute` has not been swapped to the
//! production dispatchers yet (Wave 4 CLI wiring TODO). The artifact-
//! creation assertion in [`e2e_greenfield_run_creates_artifacts`] is
//! therefore gated `#[ignore]` and lights up the moment the CLI wires
//! the real `ForgeplanCoreDispatcher`.
//!
//! Test setup pattern (mirrors `integration_phase5_playbook.rs`):
//!   1. `TempDir` workspace
//!   2. `forgeplan init -y`
//!   3. Copy fixture into `<tmp>/.forgeplan/playbooks/`
//!   4. Drive CLI via `assert_cmd::Command::cargo_bin("forgeplan")`
//!
//! Plugin discovery is disabled with `FORGEPLAN_DISABLE_PLUGIN_DISCOVERY=1`
//! so the host machine's `~/.claude/plugins/*/playbooks/` cannot leak
//! installed packs into the assertion surface.
//!
//! AC traceability matrix (PRD-072):
//!   AC-7 -> e2e_greenfield_validate_succeeds, e2e_greenfield_validate_json
//!   AC-8 -> e2e_greenfield_show_prints_7_steps,
//!           e2e_greenfield_dry_run_lists_steps,
//!           e2e_greenfield_run_creates_artifacts (#[ignore] — Wave 4 CLI),
//!           e2e_greenfield_journal_records_real_steps

use std::path::{Path, PathBuf};

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

/// Absolute path to the marketplace greenfield-kickoff fixture. We point
/// to the canonical YAML rather than maintaining a copy under
/// `tests/fixtures/` so `validate` exercises the production artifact —
/// any drift in the marketplace YAML surfaces here immediately.
fn greenfield_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("marketplace")
        .join("playbooks")
        .join("greenfield-kickoff.yaml")
}

/// Copy the marketplace greenfield-kickoff fixture into the temp
/// workspace so name-based discovery (`show`/`run <name>`) finds it.
fn install_greenfield(tmp: &TempDir) -> PathBuf {
    let src = greenfield_fixture();
    let dst_dir = tmp.path().join(".forgeplan").join("playbooks");
    std::fs::create_dir_all(&dst_dir).expect("test fixture: create playbooks dir");
    let dst = dst_dir.join("greenfield-kickoff.yaml");
    std::fs::copy(&src, &dst).expect("test fixture: copy greenfield fixture");
    dst
}

// =====================================================================
// AC-7 — validate the canonical fixture
// =====================================================================

#[test]
fn e2e_greenfield_validate_succeeds() {
    let tmp = init_workspace();
    let fixture = greenfield_fixture();

    forgeplan()
        .args(["playbook", "validate", fixture.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("OK: greenfield-kickoff (7 steps)"))
        .stdout(predicate::str::contains("Done."));
}

#[test]
fn e2e_greenfield_validate_json() {
    let tmp = init_workspace();
    let fixture = greenfield_fixture();

    let out = forgeplan()
        .args(["playbook", "validate", fixture.to_str().unwrap(), "--json"])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: run validate --json");
    assert!(
        out.status.success(),
        "validate --json failed: stderr={}",
        String::from_utf8_lossy(&out.stderr),
    );
    let v: Value =
        serde_json::from_slice(&out.stdout).expect("test fixture: validate output is JSON");
    assert_eq!(v["passed"], true);
    assert_eq!(v["name"], "greenfield-kickoff");
    assert_eq!(
        v["steps_count"], 7,
        "greenfield-kickoff must have 7 steps; got: {v}"
    );
}

// =====================================================================
// AC-8 — show prints all 7 step IDs + delegate labels
// =====================================================================

#[test]
fn e2e_greenfield_show_prints_7_steps() {
    let tmp = init_workspace();
    install_greenfield(&tmp);

    forgeplan()
        .args(["playbook", "show", "greenfield-kickoff"])
        .current_dir(tmp.path())
        .assert()
        .success()
        // All 7 step IDs must be present.
        .stdout(predicate::str::contains("capture-vision"))
        .stdout(predicate::str::contains("stack-decision"))
        .stdout(predicate::str::contains("kickoff-epic"))
        .stdout(predicate::str::contains("prd-feature-1"))
        .stdout(predicate::str::contains("prd-feature-2"))
        .stdout(predicate::str::contains("prd-feature-3"))
        .stdout(predicate::str::contains("scaffold-docs"))
        // Six steps go through `forgeplan_core: new` (FR-5); the 7th is a
        // skill step (forge-scaffolder). Both delegate labels must render.
        .stdout(predicate::str::contains("forgeplan_core:new"))
        .stdout(predicate::str::contains(
            "skill:forge-scaffolder (pack: brownfield-docs-pack)",
        ))
        // Hint contract: clean show points at the dry-run.
        .stdout(predicate::str::contains(
            "Next: forgeplan playbook run greenfield-kickoff --yes --dry-run",
        ));
}

// =====================================================================
// AC-8 — dry-run lists every step and is side-effect-free
// =====================================================================

#[test]
fn e2e_greenfield_dry_run_lists_steps() {
    let tmp = init_workspace();
    install_greenfield(&tmp);

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
            "greenfield-kickoff",
            "--yes",
            "--dry-run",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Dry-run: greenfield-kickoff (7 steps)",
        ))
        .stdout(predicate::str::contains("capture-vision"))
        .stdout(predicate::str::contains("stack-decision"))
        .stdout(predicate::str::contains("kickoff-epic"))
        .stdout(predicate::str::contains("prd-feature-1"))
        .stdout(predicate::str::contains("prd-feature-2"))
        .stdout(predicate::str::contains("prd-feature-3"))
        .stdout(predicate::str::contains("scaffold-docs"))
        .stdout(predicate::str::contains(
            "Next: forgeplan playbook run greenfield-kickoff --yes",
        ));

    // Dry-run MUST be side-effect-free per SPEC-003 §"--dry-run".
    assert!(
        !journal_path.exists(),
        "dry-run must not write journal; found {journal_path:?}"
    );
}

// =====================================================================
// AC-8 (bonus) — actual run records 1 RunStart + 7×StepStart/StepEnd + RunEnd
// =====================================================================
//
// Today this exercises the `MockDispatcher::AlwaysOk` path that the CLI
// still wires (Wave 4 will swap in the real per-delegate dispatchers).
// The journal contract under test is **structural** — count + ordering of
// entries — and survives the dispatcher swap unchanged. The artifact-
// creation assertion is split into a separate `#[ignore]`d test.

#[test]
fn e2e_greenfield_journal_records_real_steps() {
    let tmp = init_workspace();
    install_greenfield(&tmp);

    let out = forgeplan()
        .args(["playbook", "run", "greenfield-kickoff", "--yes", "--json"])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: run greenfield with --yes");
    assert!(
        out.status.success(),
        "greenfield run failed: stderr={}\nstdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout),
    );

    // Wave 4 swap: the production SkillDispatcher emits a `[skill-invoke] /…`
    // trace line on stdout (Claude Code harness contract) before the
    // top-level `--json` payload. Strip any leading non-JSON lines so the
    // structural assertions still apply.
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json_start = stdout
        .find('{')
        .expect("test fixture: run stdout must contain JSON payload");
    let v: Value =
        serde_json::from_str(&stdout[json_start..]).expect("test fixture: run output is JSON");
    assert_eq!(
        v["report"]["success"], 7,
        "all 7 greenfield steps must succeed: {v}"
    );
    assert_eq!(v["report"]["failed"], 0);
    assert_eq!(v["report"]["skipped"], 0);
    assert!(
        v["_next_action"].is_null(),
        "clean greenfield run must terminate (Done.): {v}"
    );

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
    // 1 RunStart + 7 StepStart + 7 StepEnd + 1 RunEnd = 16 entries.
    assert_eq!(
        lines.len(),
        16,
        "expected 16 journal entries (1 RunStart + 7 StepStart + 7 StepEnd + 1 RunEnd), got {}: {lines:#?}",
        lines.len(),
    );

    let mut run_start = 0usize;
    let mut step_start = 0usize;
    let mut step_end = 0usize;
    let mut run_end = 0usize;
    for line in &lines {
        let entry: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("test fixture: journal line is JSON ({e}): {line}"));
        match entry["kind"].as_str().unwrap_or("") {
            "run_start" => run_start += 1,
            "step_start" => step_start += 1,
            "step_end" => step_end += 1,
            "run_end" => run_end += 1,
            other => panic!("unexpected journal kind: {other} in line: {line}"),
        }
    }
    assert_eq!(run_start, 1, "exactly one RunStart");
    assert_eq!(step_start, 7, "one StepStart per greenfield step");
    assert_eq!(step_end, 7, "one StepEnd per greenfield step");
    assert_eq!(run_end, 1, "exactly one RunEnd");
}

// =====================================================================
// AC-8 — actual artifact creation via ForgeplanCoreDispatcher
// =====================================================================
//
// Wave 4 swap: CLI now routes through `RoutingDispatcher`, which fans
// `forgeplan_core` steps to the production `ForgeplanCoreDispatcher`.
// The six `forgeplan_core: new` steps in greenfield-kickoff therefore
// materialise real artifacts on disk, satisfying PRD-072 AC-8
// ("на actual run создаются ADR-001 + EPIC-001 + ≥3 PRD stubs").

#[test]
fn e2e_greenfield_run_creates_artifacts() {
    let tmp = init_workspace();
    install_greenfield(&tmp);

    forgeplan()
        .args(["playbook", "run", "greenfield-kickoff", "--yes"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // ADR-001 — stack-decision step.
    let adrs = forgeplan()
        .args(["list", "--type", "adr", "--json"])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: list adr");
    assert!(adrs.status.success(), "list --type adr must succeed");
    let v: Value = serde_json::from_slice(&adrs.stdout).expect("test fixture: list adr JSON");
    let adr_count = v.as_array().map(|a| a.len()).unwrap_or(0);
    assert!(
        adr_count >= 1,
        "expected ≥1 ADR after greenfield-kickoff; got {adr_count}: {v}"
    );

    // EPIC-001 — kickoff-epic step.
    let epics = forgeplan()
        .args(["list", "--type", "epic", "--json"])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: list epic");
    let v: Value = serde_json::from_slice(&epics.stdout).expect("test fixture: list epic JSON");
    let epic_count = v.as_array().map(|a| a.len()).unwrap_or(0);
    assert!(
        epic_count >= 1,
        "expected ≥1 Epic after greenfield-kickoff; got {epic_count}: {v}"
    );

    // ≥3 PRDs — feature-1/2/3 steps.
    let prds = forgeplan()
        .args(["list", "--type", "prd", "--json"])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: list prd");
    let v: Value = serde_json::from_slice(&prds.stdout).expect("test fixture: list prd JSON");
    let prd_count = v.as_array().map(|a| a.len()).unwrap_or(0);
    assert!(
        prd_count >= 3,
        "expected ≥3 PRDs after greenfield-kickoff; got {prd_count}: {v}"
    );

    // The 7th step (scaffold-docs) is `on_error: continue` with skill
    // `forge-scaffolder` — when the skill is absent we expect the run
    // to surface its `fallback_hint` on stderr but the prior 6 steps
    // still complete (per the playbook's design, FR-7).
}
