//! Phase 6 Wave 4 — broad end-to-end coverage of the playbook surface
//! (PRD-072 / ADR-009 / ADR-010).
//!
//! Where `integration_phase6_greenfield.rs` focuses exclusively on the
//! canonical greenfield playbook, this file exercises *cross-cutting*
//! Phase 6 invariants: the `--yes` security gate, init's recommendation
//! engine on a legacy repo, and the dispatcher behaviours that depend
//! on production wiring (real subprocess, Forgeplan_core in-process
//! call, on_error: abort propagation, journal durability).
//!
//! ## Status today vs. once Wave 4 CLI wiring lands
//!
//! On this branch the CLI's `forgeplan playbook run` still uses
//! `MockDispatcher::AlwaysOk` (see
//! `crates/forgeplan-cli/src/commands/playbook.rs::run_execute`). Wave 1
//! shipped the production dispatchers as a library, but the CLI swap is
//! the *next* deliverable. Tests that are observable through that swap
//! (real subprocess invocation, real ForgeplanCore artifact creation,
//! real `on_error: abort` propagation through a non-mock dispatcher) are
//! marked `#[ignore]` with a precise reason — the moment the CLI wires
//! the real dispatchers these tests light up, providing a regression
//! safety net for that swap.
//!
//! Tests that exercise paths *outside* the dispatcher boundary (the
//! `--yes` gate, init's recommendation hints, journal structure on a
//! mock-driven run) run by default and protect those surfaces today.
//!
//! ## AC traceability matrix (PRD-072)
//!
//! | Test                                                       | AC        | Notes                                                |
//! |------------------------------------------------------------|-----------|------------------------------------------------------|
//! | e2e_command_dispatcher_refuses_without_yes                 | AC-1, FR-10 | --yes security gate (ADR-009), runs today          |
//! | e2e_init_recommends_brownfield_code_on_legacy_repo         | AC-5      | PRD-067 AC-5 / FR-6 wired in Wave 2, runs today    |
//! | e2e_dispatch_journal_durability_after_step_end             | AC-10     | Phase 5 NEW-S-H2 contract — no regression today    |
//! | e2e_real_subprocess_dispatch_via_command_delegate          | AC-1, FR-4 | #[ignore] — Wave 4 CLI wiring                      |
//! | e2e_command_dispatcher_propagates_exit_failure             | AC-1, FR-9 | #[ignore] — Wave 4 CLI wiring                      |
//! | e2e_forgeplan_core_dispatcher_creates_artifact_via_new_op  | AC-8, FR-5 | #[ignore] — Wave 4 CLI wiring                      |
//!
//! ## Setup conventions
//!
//! * `TempDir` per test — every test is fully self-contained.
//! * `assert_cmd::Command::cargo_bin("forgeplan")` builds the dev binary
//!   on first request and re-uses it; no `cargo build --release` needed.
//! * `FORGEPLAN_DISABLE_PLUGIN_DISCOVERY=1` keeps host-installed Claude
//!   plugins out of the assertion surface.
//! * `FORGEPLAN_HINTS=1` overrides the TTY guard so init's
//!   recommendation hints reach captured stderr.

use std::path::Path;
use std::process::Command as StdCommand;

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

/// Drop a playbook YAML into `<tmp>/.forgeplan/playbooks/`. `name`
/// becomes both the file stem and (assuming the YAML's `name:` matches)
/// the discovery key for `forgeplan playbook run <name>`.
fn write_workspace_playbook(tmp: &TempDir, file_stem: &str, yaml: &str) -> std::path::PathBuf {
    let dir = tmp.path().join(".forgeplan").join("playbooks");
    std::fs::create_dir_all(&dir).expect("test fixture: create .forgeplan/playbooks");
    let path = dir.join(format!("{file_stem}.yaml"));
    std::fs::write(&path, yaml).expect("test fixture: write playbook");
    path
}

/// Initialise an empty git repo with a usable identity (so `git commit`
/// succeeds in CI sandboxes).
fn git_init_with_identity(dir: &Path) {
    let s = StdCommand::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .status()
        .expect("test fixture: git init");
    assert!(s.success(), "git init failed");
    for (k, v) in [
        ("user.email", "test@forgeplan.dev"),
        ("user.name", "Forgeplan Test"),
    ] {
        let s = StdCommand::new("git")
            .args(["config", k, v])
            .current_dir(dir)
            .status()
            .expect("test fixture: git config");
        assert!(s.success(), "git config {k} failed");
    }
}

/// Pile up `n` empty commits — fast single-shell loop, ~50× faster than
/// spawning a child per commit.
fn git_empty_commits(dir: &Path, n: usize) {
    let cmd = format!("for i in $(seq 1 {n}); do git commit --allow-empty -m c$i -q; done");
    let s = StdCommand::new("sh")
        .arg("-c")
        .arg(&cmd)
        .current_dir(dir)
        .status()
        .expect("test fixture: git commit loop");
    assert!(s.success(), "git commit loop failed");
}

// =====================================================================
// AC-1 / FR-10 — `--yes` security gate (ADR-009)
// =====================================================================
//
// The gate is enforced at the top of `commands::playbook::run_execute`
// independent of which dispatcher is wired downstream, so this test is
// stable today even though the rest of the surface is mock-backed.

#[test]
fn e2e_command_dispatcher_refuses_without_yes() {
    let tmp = init_workspace();
    // Single-step `Command` playbook — minimal valid SPEC-003 schema.
    let yaml = r#"
schema_version: "1.0"
name: refuse-no-yes
title: Refuse without --yes
steps:
  - id: only
    delegate_to:
      type: command
      command: ["/bin/echo", "should-never-run"]
"#;
    write_workspace_playbook(&tmp, "refuse-no-yes", yaml);

    let assertion = forgeplan()
        .args(["playbook", "run", "refuse-no-yes"])
        .current_dir(tmp.path())
        .assert()
        .failure();
    let out = assertion.get_output();
    assert_eq!(
        out.status.code().unwrap_or(-1),
        2,
        "missing --yes must exit 2 (security refusal)"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--yes"),
        "stderr should mention --yes: {stderr}"
    );
    assert!(
        stderr.contains("Fix: forgeplan playbook run refuse-no-yes --yes"),
        "stderr should carry a Fix: hint per PRD-071: {stderr}"
    );
}

// =====================================================================
// AC-5 — init recommends brownfield-code on a legacy repo (PRD-067 / FR-6)
// =====================================================================
//
// 100+ empty commits + at least one tracked file = `commit_count_min: 100`
// trigger from `bundled_known_playbooks()`. `FORGEPLAN_HINTS=1` bypasses
// the TTY guard so stderr captures the hint.

#[test]
fn e2e_init_recommends_brownfield_code_on_legacy_repo() {
    let tmp = TempDir::new().expect("test fixture: TempDir");
    git_init_with_identity(tmp.path());
    git_empty_commits(tmp.path(), 110);
    // Push past the empty_repo threshold (≥6 tracked files) so the
    // greenfield trigger doesn't outrank brownfield-code.
    for i in 0..6 {
        std::fs::write(tmp.path().join(format!("legacy_{i}.txt")), b"x")
            .expect("test fixture: write legacy file");
    }

    forgeplan()
        .env("FORGEPLAN_HINTS", "1")
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("brownfield-code"));
}

// =====================================================================
// AC-10 — journal records every StepEnd (Phase 5 NEW-S-H2 contract)
// =====================================================================
//
// PRD-065 FR-6 + ADR-010 promise that `StepEnd` is flushed to disk
// **before** the next step starts. We can't observe a real crash from a
// test, but we can assert the *post-conditions* — for a clean run every
// `StepStart` has a matching `StepEnd` adjacently in the journal — which
// is the property a resumer relies on.

#[test]
fn e2e_dispatch_journal_durability_after_step_end() {
    if !std::path::Path::new("/bin/echo").is_file() {
        eprintln!("skip: /bin/echo missing");
        return;
    }
    let tmp = init_workspace();
    // Three sequential `command` steps — exercises the "≥1 step pair"
    // path that Phase 5's per-step flush guarantees. Phase 6 Wave 4
    // wires the production CommandDispatcher, so we use real /bin/echo
    // here to keep all three steps successful and adjacent in the
    // journal.
    let yaml = r#"
schema_version: "1.0"
name: durable-trio
title: Three sequential steps
steps:
  - id: alpha
    delegate_to: { type: command, command: ["/bin/echo", "alpha"] }
  - id: beta
    delegate_to: { type: command, command: ["/bin/echo", "beta"] }
    requires: [alpha]
  - id: gamma
    delegate_to: { type: command, command: ["/bin/echo", "gamma"] }
    requires: [beta]
"#;
    write_workspace_playbook(&tmp, "durable-trio", yaml);

    forgeplan()
        .args(["playbook", "run", "durable-trio", "--yes"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let journal = tmp
        .path()
        .join(".forgeplan")
        .join("journal")
        .join("playbook-runs.jsonl");
    let body = std::fs::read_to_string(&journal).expect("test fixture: read journal");
    let kinds: Vec<String> = body
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let v: Value =
                serde_json::from_str(l).unwrap_or_else(|e| panic!("invalid journal line: {e}"));
            v["kind"].as_str().unwrap_or("").to_string()
        })
        .collect();

    // Exact ordering for 3 sequential steps: RunStart, then for each step
    // StepStart immediately followed by StepEnd, then RunEnd.
    let expected = [
        "run_start",
        "step_start",
        "step_end",
        "step_start",
        "step_end",
        "step_start",
        "step_end",
        "run_end",
    ];
    assert_eq!(
        kinds, expected,
        "journal must record StepEnd immediately after each StepStart (per-step flush)"
    );
}

// =====================================================================
// AC-1 / FR-4 — real subprocess invocation via Command delegate
// =====================================================================
//
// Once the CLI is wired to `CommandDispatcher`, running a playbook with
// a `command:` step actually spawns the process. We verify success +
// captured exit code via the journal payload.

#[test]
fn e2e_real_subprocess_dispatch_via_command_delegate() {
    let tmp = init_workspace();
    let yaml = r#"
schema_version: "1.0"
name: echo-subprocess
title: Real /bin/echo subprocess
steps:
  - id: echo-step
    delegate_to:
      type: command
      command: ["/bin/echo", "hello"]
"#;
    write_workspace_playbook(&tmp, "echo-subprocess", yaml);

    forgeplan()
        .args(["playbook", "run", "echo-subprocess", "--yes"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Once wired, the StepEnd payload should carry exit_code=0.
    let journal = tmp
        .path()
        .join(".forgeplan")
        .join("journal")
        .join("playbook-runs.jsonl");
    let body = std::fs::read_to_string(&journal).expect("test fixture: read journal");
    let step_end = body
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|v| v["kind"] == "step_end" && v["step_id"] == "echo-step")
        .expect("step_end entry for echo-step");
    let success = step_end["payload"]["success"].as_bool().unwrap_or(false);
    assert!(
        success,
        "real /bin/echo must produce success=true: {step_end}"
    );
}

// =====================================================================
// AC-1 / FR-9 — non-zero exit propagates and on_error: abort halts the run
// =====================================================================

#[test]
fn e2e_command_dispatcher_propagates_exit_failure() {
    let tmp = init_workspace();
    // Step 1 exits 7 with on_error: abort. Step 2 must therefore be
    // marked Skipped (predecessor failed) per executor contract.
    let yaml = r#"
schema_version: "1.0"
name: abort-on-exit-7
title: Abort propagation
steps:
  - id: fail
    delegate_to:
      type: command
      command: ["/bin/sh", "-c", "exit 7"]
    on_error: abort
  - id: never-runs
    delegate_to: { type: agent, name: a }
    requires: [fail]
"#;
    write_workspace_playbook(&tmp, "abort-on-exit-7", yaml);

    let out = forgeplan()
        .args(["playbook", "run", "abort-on-exit-7", "--yes", "--json"])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: run abort-on-exit-7");
    let v: Value = serde_json::from_slice(&out.stdout).expect("test fixture: run output is JSON");
    assert_eq!(
        v["report"]["failed"], 1,
        "exactly one step (fail) must report failed: {v}"
    );
    assert_eq!(
        v["report"]["skipped"], 1,
        "downstream `never-runs` must be Skipped: {v}"
    );
}

// =====================================================================
// AC-8 / FR-5 — ForgeplanCoreDispatcher creates real artifacts
// =====================================================================

#[test]
fn e2e_forgeplan_core_dispatcher_creates_artifact_via_new_op() {
    let tmp = init_workspace();
    let yaml = r#"
schema_version: "1.0"
name: core-new-note
title: ForgeplanCore new note
steps:
  - id: make-note
    delegate_to:
      type: forgeplan_core
      target: new
    input:
      kind: note
      title: "E2E probe note"
"#;
    write_workspace_playbook(&tmp, "core-new-note", yaml);

    forgeplan()
        .args(["playbook", "run", "core-new-note", "--yes"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["list", "--type", "note", "--json"])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: list note --json");
    assert!(
        out.status.success(),
        "list --type note must succeed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value =
        serde_json::from_slice(&out.stdout).expect("test fixture: list note output is JSON");
    let count = v.as_array().map(|a| a.len()).unwrap_or(0);
    assert!(
        count >= 1,
        "ForgeplanCore::New must create ≥1 note; got {count}: {v}"
    );
}
