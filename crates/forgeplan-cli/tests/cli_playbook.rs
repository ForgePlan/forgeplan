//! Integration tests for `forgeplan playbook` CLI surface (PRD-065 / SPEC-003).
//!
//! Each test boots an isolated `TempDir` workspace, writes a fixture YAML, and
//! asserts both behavior (exit codes, side effects) and the PRD-071 hint
//! contract (`Next:` / `Or:` / `Done.` / `Fix:`).

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn forgeplan() -> Command {
    let mut cmd = Command::cargo_bin("forgeplan").unwrap();
    // Isolate from the host machine's installed Claude plugins so the runner's
    // ~/.claude/plugins/*/playbooks/ does not leak into discovery assertions.
    cmd.env("FORGEPLAN_DISABLE_PLUGIN_DISCOVERY", "1");
    cmd
}

fn init_workspace() -> TempDir {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    tmp
}

/// Write a playbook YAML file inside the workspace's `.forgeplan/playbooks/`.
fn write_workspace_playbook(tmp: &TempDir, filename: &str, yaml: &str) -> std::path::PathBuf {
    let dir = tmp.path().join(".forgeplan").join("playbooks");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(filename);
    std::fs::write(&p, yaml).unwrap();
    p
}

/// Minimal valid playbook (1 agent step).
fn good_playbook_yaml(name: &str) -> String {
    format!(
        r#"
schema_version: "1.0"
name: {name}
title: Sample {name}
steps:
  - id: only-step
    delegate_to:
      type: agent
      name: hello
"#
    )
}

/// Bad playbook: empty steps array (SPEC-003 §Errors).
const BAD_PLAYBOOK_YAML: &str = r#"
schema_version: "1.0"
name: broken
title: Broken
steps: []
"#;

// =====================================================================
// list
// =====================================================================

#[test]
fn playbook_list_empty_workspace_json_is_array() {
    let tmp = init_workspace();
    let out = forgeplan()
        .args(["playbook", "list", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "expected success on empty workspace");

    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("expected valid JSON: {e}\noutput:\n{stdout}"));
    assert!(v["playbooks"].is_array(), "playbooks must be an array");
    assert_eq!(v["playbooks"].as_array().unwrap().len(), 0);
    // PRD-071: empty list → terminal, _next_action MUST be null.
    assert!(
        v["_next_action"].is_null(),
        "empty list should be terminal: {v:?}"
    );
}

#[test]
fn playbook_list_text_empty_emits_done() {
    let tmp = init_workspace();
    forgeplan()
        .args(["playbook", "list"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No playbooks found"))
        .stdout(predicate::str::contains("Done."));
}

#[test]
fn playbook_list_finds_workspace_playbook() {
    let tmp = init_workspace();
    write_workspace_playbook(&tmp, "demo.yaml", &good_playbook_yaml("demo-pb"));

    let out = forgeplan()
        .args(["playbook", "list", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: Value = serde_json::from_str(&stdout).expect("json");
    let arr = v["playbooks"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "demo-pb");
    assert_eq!(arr[0]["steps_count"], 1);
    // PRD-071: should suggest `show <first-name>` as Next.
    let next = v["_next_action"].as_str().expect("next");
    assert!(
        next.contains("forgeplan playbook show demo-pb"),
        "_next_action: {next}"
    );
}

// =====================================================================
// validate
// =====================================================================

#[test]
fn playbook_validate_good_file_exits_zero() {
    let tmp = init_workspace();
    let path = write_workspace_playbook(&tmp, "good.yaml", &good_playbook_yaml("good-pb"));

    forgeplan()
        .args(["playbook", "validate", path.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("OK: good-pb"))
        .stdout(predicate::str::contains("Done."));
}

#[test]
fn playbook_validate_good_file_json_passed_true() {
    let tmp = init_workspace();
    let path = write_workspace_playbook(&tmp, "good.yaml", &good_playbook_yaml("good-pb"));

    let out = forgeplan()
        .args(["playbook", "validate", path.to_str().unwrap(), "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());

    let v: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["passed"], true);
    assert_eq!(v["name"], "good-pb");
    assert_eq!(v["steps_count"], 1);
    assert!(v["_next_action"].is_string());
}

#[test]
fn playbook_validate_bad_file_exits_two_with_fix_hint() {
    let tmp = init_workspace();
    let path = write_workspace_playbook(&tmp, "bad.yaml", BAD_PLAYBOOK_YAML);

    let assertion = forgeplan()
        .args(["playbook", "validate", path.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .failure();
    let out = assertion.get_output();
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(code, 2, "expected exit 2 for malformed playbook");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Error:"),
        "missing Error: line in: {stderr}"
    );
    assert!(stderr.contains("Fix:"), "missing Fix: line in: {stderr}");
    assert!(
        stderr.contains("no steps") || stderr.contains("at least one"),
        "expected explanation about empty steps; got: {stderr}"
    );
}

#[test]
fn playbook_validate_unknown_step_ref_lists_pairs() {
    let tmp = init_workspace();
    let yaml = r#"
schema_version: "1.0"
name: typo-pb
title: Typo PB
steps:
  - id: a
    delegate_to: { type: agent, name: x }
  - id: b
    delegate_to: { type: agent, name: y }
    requires: [does-not-exist]
"#;
    let path = write_workspace_playbook(&tmp, "typo.yaml", yaml);
    let assertion = forgeplan()
        .args(["playbook", "validate", path.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).to_string();
    assert!(
        stderr.contains("does-not-exist"),
        "should mention bad ref: {stderr}"
    );
    assert!(stderr.contains("Fix:"));
}

// =====================================================================
// show
// =====================================================================

#[test]
fn playbook_show_by_name_succeeds() {
    let tmp = init_workspace();
    write_workspace_playbook(&tmp, "demo.yaml", &good_playbook_yaml("demo-show"));

    forgeplan()
        .args(["playbook", "show", "demo-show"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Playbook: demo-show"))
        .stdout(predicate::str::contains("only-step"))
        .stdout(predicate::str::contains("agent:hello"))
        .stdout(predicate::str::contains(
            "Next: forgeplan playbook run demo-show",
        ));
}

#[test]
fn playbook_show_unknown_target_exits_two() {
    let tmp = init_workspace();
    let assertion = forgeplan()
        .args(["playbook", "show", "no-such-playbook"])
        .current_dir(tmp.path())
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).to_string();
    assert!(
        stderr.contains("no playbook named") || stderr.contains("no such"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("Fix:"));
}

#[test]
fn playbook_show_json_returns_full_playbook() {
    let tmp = init_workspace();
    write_workspace_playbook(&tmp, "demo.yaml", &good_playbook_yaml("demo-json"));

    let out = forgeplan()
        .args(["playbook", "show", "demo-json", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());

    let v: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["playbook"]["name"], "demo-json");
    assert_eq!(v["playbook"]["steps"].as_array().unwrap().len(), 1);
    assert!(v["_next_action"].is_string());
}

// =====================================================================
// run
// =====================================================================

#[test]
fn playbook_run_without_yes_exits_two_with_fix_hint() {
    let tmp = init_workspace();
    write_workspace_playbook(&tmp, "demo.yaml", &good_playbook_yaml("run-target"));

    let assertion = forgeplan()
        .args(["playbook", "run", "run-target"])
        .current_dir(tmp.path())
        .assert()
        .failure();
    let out = assertion.get_output();
    assert_eq!(out.status.code().unwrap_or(-1), 2);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--yes"));
    assert!(stderr.contains("Fix: forgeplan playbook run run-target --yes"));
}

#[test]
fn playbook_run_dry_run_lists_steps() {
    let tmp = init_workspace();
    write_workspace_playbook(&tmp, "demo.yaml", &good_playbook_yaml("dry-pb"));

    forgeplan()
        .args(["playbook", "run", "dry-pb", "--yes", "--dry-run"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry-run: dry-pb"))
        .stdout(predicate::str::contains("only-step"))
        .stdout(predicate::str::contains(
            "Next: forgeplan playbook run dry-pb --yes",
        ));
}

#[test]
fn playbook_run_real_writes_journal_and_succeeds() {
    let tmp = init_workspace();
    write_workspace_playbook(&tmp, "demo.yaml", &good_playbook_yaml("real-pb"));

    let out = forgeplan()
        .args(["playbook", "run", "real-pb", "--yes", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}\nstdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout),
    );

    let v: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["report"]["success"], 1);
    assert_eq!(v["report"]["failed"], 0);
    assert_eq!(v["report"]["skipped"], 0);
    // Clean run → terminal next-action.
    assert!(v["_next_action"].is_null());

    // Journal file should now exist.
    let journal = tmp
        .path()
        .join(".forgeplan")
        .join("journal")
        .join("playbook-runs.jsonl");
    assert!(journal.exists(), "journal should be created at {journal:?}");
}

#[test]
fn playbook_run_step_out_of_range_exits_two() {
    let tmp = init_workspace();
    write_workspace_playbook(&tmp, "demo.yaml", &good_playbook_yaml("step-pb"));

    let assertion = forgeplan()
        .args([
            "playbook",
            "run",
            "step-pb",
            "--yes",
            "--dry-run",
            "--step",
            "99",
        ])
        .current_dir(tmp.path())
        .assert()
        .failure();
    let out = assertion.get_output();
    assert_eq!(out.status.code().unwrap_or(-1), 2);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("out of range"));
    assert!(stderr.contains("Fix: forgeplan playbook show step-pb"));
}

/// HIGH-S5 (Audit Round 1): `--step N` must reach the executor on a real
/// run so resumable playbooks (PRD-065 FR-6) actually skip earlier steps.
/// Before the fix the flag was parsed but discarded — every step always ran.
///
/// We use three INDEPENDENT steps (no `requires:`) so the only skip in the
/// report can be attributed to `--step 2`. A linear playbook would compound
/// the explicit skip with the executor's predecessor-not-successful rule.
#[test]
fn playbook_run_step_skips_earlier_steps() {
    let tmp = init_workspace();
    let yaml = r#"
schema_version: "1.0"
name: linear-pb
title: Three-step independent
steps:
  - id: s1
    delegate_to:
      type: agent
      name: alpha
  - id: s2
    delegate_to:
      type: agent
      name: alpha
  - id: s3
    delegate_to:
      type: agent
      name: alpha
"#;
    write_workspace_playbook(&tmp, "linear.yaml", yaml);

    let out = forgeplan()
        .args([
            "playbook",
            "run",
            "linear-pb",
            "--yes",
            "--step",
            "2",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}\nstdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout),
    );

    let v: Value = serde_json::from_slice(&out.stdout).expect("json");
    // step=2 → s1 skipped, s2 + s3 succeed.
    assert_eq!(
        v["report"]["skipped"], 1,
        "exactly one step must be skipped"
    );
    assert_eq!(v["report"]["success"], 2, "remaining steps must execute");

    // The skipped step must be the first one (s1), not an arbitrary later one.
    let per_step = v["report"]["per_step"].as_array().expect("array");
    let s1 = per_step
        .iter()
        .find(|e| e["step_id"].as_str() == Some("s1"))
        .expect("s1 reported");
    assert_eq!(s1["status"].as_str(), Some("skipped"));
}
