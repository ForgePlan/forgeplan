//! Coverage tests for 17 previously untested CLI commands.
//!
//! Goal: smoke-test happy path для каждой команды, чтобы regression полностью
//! ломающий команду — failed CLI parse, panic при простом invocation, missing
//! workspace handling и т.п. — был пойман CI.
//!
//! Это **не** TDD — мы не специфицируем поведение через тесты. Это coverage
//! верификация: на момент написания все 17 команд работают на простейшем
//! happy path; тесты фиксируют этот baseline.
//!
//! Covered commands (16; `watch` deferred — long-running foreground watcher
//! is untestable via assert_cmd without SIGTERM handling and timing races,
//! see `crates/forgeplan-cli/src/commands/watch.rs` for module-level unit tests):
//!   embed, tree, git_sync, log_cmd, context, promote, reopen,
//!   scan_import, setup_skill, tag, recall, remember, migrate,
//!   migrate_dry_run, reconcile_ids, ci_assign_id
//!
//! Ignored / negative-path rationale documented inline per test.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Build the `forgeplan` binary command.
fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Initialise a fresh `.forgeplan/` workspace in a tempdir and return it.
///
/// LOW-3 (w4-security-audit): the binary's `discover_known_playbooks`
/// enumerates `$HOME/.claude/plugins/` on every `forgeplan init`. Without
/// HOME override that enumeration is non-deterministic between CI (no
/// user plugins) and local dev (e.g. dev-toolkit, forge plugin installed).
/// We override HOME — and USERPROFILE for Windows portability — to the
/// tempdir so plugin discovery is hermetic. XDG_DATA_HOME pinned too,
/// matching the setup-skill test for consistency.
fn init_workspace() -> TempDir {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .env("HOME", tmp.path())
        .env("USERPROFILE", tmp.path())
        .env("XDG_DATA_HOME", tmp.path())
        .current_dir(tmp.path())
        .assert()
        .success();
    tmp
}

/// Create a draft PRD inside `tmp` and return its id (`PRD-001`).
fn new_prd(tmp: &TempDir, title: &str) -> String {
    forgeplan()
        .args(["new", "prd", title])
        .env("HOME", tmp.path())
        .env("USERPROFILE", tmp.path())
        .env("XDG_DATA_HOME", tmp.path())
        .current_dir(tmp.path())
        .assert()
        .success();
    "PRD-001".to_string()
}

// ---------------------------------------------------------------------------
// embed
// ---------------------------------------------------------------------------

/// `embed` is feature-gated на `semantic-search`. Test binary компилируется
/// без feature по умолчанию, поэтому команда должна вернуть actionable error
/// с инструкцией по rebuild. Это валидный coverage error-path: regression,
/// убивающий graceful fallback (panic, silent exit), будет пойман.
#[test]
fn embed_without_feature_returns_error_with_fix() {
    let tmp = init_workspace();

    forgeplan()
        .args(["embed"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Embedding not available"))
        .stderr(predicate::str::contains("semantic-search"));
}

// ---------------------------------------------------------------------------
// tree
// ---------------------------------------------------------------------------

#[test]
fn tree_empty_workspace_shows_empty_message() {
    let tmp = init_workspace();

    forgeplan()
        .args(["tree"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No artifacts"));
}

#[test]
fn tree_with_artifact_renders_id() {
    let tmp = init_workspace();
    let _ = new_prd(&tmp, "Tree test PRD");

    forgeplan()
        .args(["tree"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));
}

#[test]
fn tree_json_emits_valid_array() {
    let tmp = init_workspace();
    let _ = new_prd(&tmp, "Tree JSON test");

    let output = forgeplan()
        .args(["tree", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    // PRD-071: stdout MUST be a bare JSON array (Next: hint goes to stderr).
    // Strip trailing whitespace; parse strictly.
    let parsed: serde_json::Value = serde_json::from_str(s.trim()).expect("tree --json output");
    assert!(parsed.is_array(), "tree --json root must be array");
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["id"], "PRD-001");
}

// ---------------------------------------------------------------------------
// git_sync
// ---------------------------------------------------------------------------

/// `git-sync` без recent pull/merge возвращает Err с actionable `Fix:` line.
/// Verifies error-path contract (no panic, deterministic message).
#[test]
fn git_sync_without_orig_head_emits_fix() {
    let tmp = init_workspace();
    // Init git repo but no pull/merge → no ORIG_HEAD
    Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["git-sync"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("ORIG_HEAD"))
        .stderr(predicate::str::contains("Fix:"));
}

#[test]
fn git_sync_since_head_no_changes() {
    let tmp = init_workspace();
    // Real git repo with an initial commit so `--since HEAD` succeeds.
    Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(tmp.path())
        .assert()
        .success();
    Command::new("git")
        .args(["add", "."])
        .current_dir(tmp.path())
        .assert()
        .success();
    Command::new("git")
        .args([
            "-c",
            "user.email=t@t.com",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "init",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["git-sync", "--since", "HEAD"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No .forgeplan/ files changed"));
}

// ---------------------------------------------------------------------------
// log_cmd
// ---------------------------------------------------------------------------

#[test]
fn log_empty_workspace_no_entries() {
    let tmp = init_workspace();

    forgeplan()
        .args(["log"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No change log entries"));
}

#[test]
fn log_after_create_shows_entry() {
    let tmp = init_workspace();
    let _ = new_prd(&tmp, "Log test");

    forgeplan()
        .args(["log", "-n", "5"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("create"));
}

#[test]
fn log_json_emits_entries_array() {
    let tmp = init_workspace();
    let _ = new_prd(&tmp, "Log JSON test");

    let output = forgeplan()
        .args(["log", "--json", "-n", "5"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(s.trim()).expect("log --json output");
    assert!(parsed["entries"].is_array());
}

// ---------------------------------------------------------------------------
// context
// ---------------------------------------------------------------------------

#[test]
fn context_existing_artifact_renders_id_and_status() {
    let tmp = init_workspace();
    let id = new_prd(&tmp, "Context test");

    forgeplan()
        .args(["context", &id])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("Status"));
}

#[test]
fn context_json_has_artifact_and_validation_fields() {
    let tmp = init_workspace();
    let id = new_prd(&tmp, "Context JSON test");

    let output = forgeplan()
        .args(["context", &id, "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(s.trim()).expect("context --json output");
    assert_eq!(parsed["artifact"]["id"], "PRD-001");
    assert!(parsed["validation"].is_object());
    assert!(parsed["fgr"].is_object());
}

#[test]
fn context_missing_artifact_errors() {
    let tmp = init_workspace();

    forgeplan()
        .args(["context", "PRD-999"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// promote
// ---------------------------------------------------------------------------

#[test]
fn promote_memory_to_note_creates_new_artifact() {
    let tmp = init_workspace();
    forgeplan()
        .args(["remember", "test memory for promote"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["promote", "mem-test-memory-for-promote", "--kind", "note"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Promoted"))
        .stdout(predicate::str::contains("NOTE-001"));

    // Memory file should be deleted; new note file should exist.
    assert!(
        tmp.path()
            .join(".forgeplan/notes")
            .read_dir()
            .unwrap()
            .count()
            > 0
    );
}

#[test]
fn promote_non_memory_errors() {
    let tmp = init_workspace();
    let _ = new_prd(&tmp, "Not a memory");

    forgeplan()
        .args(["promote", "PRD-001", "--kind", "note"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// reopen
// ---------------------------------------------------------------------------

/// reopen draft → error (lifecycle gate). Covers error path with `Fix:` hint.
/// Note: reopen happy path (active → deprecated + new draft) requires
/// fully-shaped + activated artifact with validation PASS — overkill для
/// CLI smoke coverage. State-transition success is exercised by the
/// `forgeplan-core::lifecycle` unit tests.
#[test]
fn reopen_draft_artifact_errors_with_fix_hint() {
    let tmp = init_workspace();
    let id = new_prd(&tmp, "Reopen test draft");

    forgeplan()
        .args(["reopen", &id, "--reason", "test"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Fix:"));
}

// ---------------------------------------------------------------------------
// scan_import
// ---------------------------------------------------------------------------

#[test]
fn scan_import_dry_run_empty_project() {
    let tmp = init_workspace();

    forgeplan()
        .args(["scan-import", "--dry-run"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry-run mode"));
}

#[test]
fn scan_import_finds_external_markdown() {
    let tmp = init_workspace();
    // Drop a markdown doc adjacent to .forgeplan/.
    let doc = tmp.path().join("RFC-external.md");
    std::fs::write(
        &doc,
        "---\nkind: rfc\ntitle: External Draft\n---\n\n# External Draft\n\nBody.\n",
    )
    .unwrap();

    forgeplan()
        .args(["scan-import", "--dry-run"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("document(s)"));
}

// ---------------------------------------------------------------------------
// setup_skill
// ---------------------------------------------------------------------------

#[test]
fn setup_skill_writes_skill_file_under_fake_home() {
    let tmp = init_workspace();
    let fake_home = TempDir::new().unwrap();

    // LOW-1 (w4-security-audit): override HOME, USERPROFILE (Windows), and
    // XDG_DATA_HOME so `dirs::home_dir()` resolves to the tempdir on every
    // platform — and never falls through to passwd entry when HOME is blank.
    forgeplan()
        .args(["setup-skill"])
        .env("HOME", fake_home.path())
        .env("USERPROFILE", fake_home.path())
        .env("XDG_DATA_HOME", fake_home.path())
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed /forge skill"));

    let skill_path = fake_home
        .path()
        .join(".claude")
        .join("skills")
        .join("forge")
        .join("SKILL.md");
    assert!(
        skill_path.exists(),
        "SKILL.md must be written under fake HOME"
    );
    let content = std::fs::read_to_string(&skill_path).unwrap();
    assert!(!content.is_empty(), "SKILL.md must not be empty");
}

// ---------------------------------------------------------------------------
// tag / untag
// ---------------------------------------------------------------------------

#[test]
fn tag_adds_tag_to_artifact() {
    let tmp = init_workspace();
    let id = new_prd(&tmp, "Tag test");

    forgeplan()
        .args(["tag", &id, "smoke"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Added"))
        .stdout(predicate::str::contains("smoke"));
}

#[test]
fn untag_removes_tag_from_artifact() {
    let tmp = init_workspace();
    let id = new_prd(&tmp, "Untag test");
    forgeplan()
        .args(["tag", &id, "removeme"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["untag", &id, "removeme"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed"));
}

#[test]
fn tag_missing_artifact_errors() {
    let tmp = init_workspace();

    forgeplan()
        .args(["tag", "PRD-999", "ghost"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// recall
// ---------------------------------------------------------------------------

#[test]
fn recall_empty_workspace_shows_no_memories() {
    let tmp = init_workspace();

    forgeplan()
        .args(["recall"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No memories found"));
}

#[test]
fn recall_after_remember_returns_memory() {
    let tmp = init_workspace();
    forgeplan()
        .args(["remember", "recall coverage fact"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["recall", "recall"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("mem-"));
}

#[test]
fn recall_json_emits_memories_array() {
    let tmp = init_workspace();
    forgeplan()
        .args(["remember", "recall json fact"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["recall", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(s.trim()).expect("recall --json output");
    assert!(parsed["memories"].is_array());
    assert_eq!(parsed["memories"].as_array().unwrap().len(), 1);
}

// ---------------------------------------------------------------------------
// remember
// ---------------------------------------------------------------------------

#[test]
fn remember_creates_memory_artifact() {
    let tmp = init_workspace();

    forgeplan()
        .args(["remember", "remember coverage fact"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Remembered"))
        .stdout(predicate::str::contains("mem-remember-coverage-fact"));
}

#[test]
fn remember_list_empty_shows_no_memories() {
    let tmp = init_workspace();

    forgeplan()
        .args(["remember", "--list"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No memories"));
}

#[test]
fn remember_list_after_capture_shows_entry() {
    let tmp = init_workspace();
    forgeplan()
        .args(["remember", "list-coverage entry"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["remember", "--list"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("mem-list-coverage-entry"));
}

#[test]
fn remember_forget_removes_memory() {
    let tmp = init_workspace();
    forgeplan()
        .args(["remember", "to be forgotten"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["remember", "--forget", "mem-to-be-forgotten"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Forgotten"));
}

// ---------------------------------------------------------------------------
// migrate
// ---------------------------------------------------------------------------

#[test]
fn migrate_runs_schema_migrations() {
    let tmp = init_workspace();

    forgeplan()
        .args(["migrate"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("migrations"))
        .stdout(predicate::str::contains("Schema up to date"));
}

// ---------------------------------------------------------------------------
// migrate_dry_run
// ---------------------------------------------------------------------------

#[test]
fn migrate_dry_run_empty_workspace_no_collisions() {
    let tmp = init_workspace();

    forgeplan()
        .args(["migrate-dry-run"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No collisions"))
        .stdout(predicate::str::contains("Greenlight"));
}

#[test]
fn migrate_dry_run_json_schema_v1() {
    let tmp = init_workspace();
    let _ = new_prd(&tmp, "Migrate dry-run JSON");

    let output = forgeplan()
        .args(["migrate-dry-run", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(s.trim()).expect("migrate-dry-run --json");
    assert_eq!(parsed["schema_version"], 1);
    assert_eq!(parsed["total_artifacts"], 1);
    assert_eq!(parsed["summary"]["exit_code"], 0);
}

// ---------------------------------------------------------------------------
// reconcile_ids
// ---------------------------------------------------------------------------

#[test]
fn reconcile_ids_check_only_emits_json() {
    let tmp = init_workspace();
    let _ = new_prd(&tmp, "Reconcile target");

    // LOW-2 (w4-security-audit): fresh-init workspace MUST be clean. The
    // shipped templates no longer carry literal cross-refs (`ADR-001` etc.
    // were replaced with `<id>` placeholders), and `detect_body_links_drift`
    // now treats the outer-frontmatter `id` as a self-ref to cover artifacts
    // with double-wrapped frontmatter. Together these close the bad-first-
    // run UX where a fresh `forgeplan init` + `new prd` would surface
    // body_links_drift. Strict assertion: exit 0, summary.unresolved=false.
    let output = forgeplan()
        .args(["reconcile-ids", "--check-only", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(s.trim()).expect("reconcile-ids --json output");
    assert!(parsed["actions"].is_array());
    assert!(parsed["summary"].is_object());
    assert_eq!(
        parsed["summary"]["unresolved"], false,
        "fresh init must have zero unresolved drift (LOW-2 regression guard)"
    );
}

// ---------------------------------------------------------------------------
// ci_assign_id
// ---------------------------------------------------------------------------

#[test]
fn ci_assign_id_dry_run_no_candidates_emits_json() {
    let tmp = init_workspace();
    // Real git repo required (binary calls `git remote get-url`).
    Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(tmp.path())
        .assert()
        .success();

    // Exit code contract (CD-1): 0 = success, 2 = no candidates. Fresh
    // workspace без PRD/RFC slug-без-assigned_number кандидатов → exit 2.
    // Структура JSON emits в обоих случаях, проверяем shape.
    let output = forgeplan()
        .args(["ci-assign-id", "--dry-run", "--json"])
        .current_dir(tmp.path())
        .assert()
        .code(predicate::in_iter([0i32, 2i32]))
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(s.trim()).expect("ci-assign-id --json output");
    assert_eq!(parsed["schema_version"], 1);
    assert_eq!(parsed["dry_run"], true);
    assert!(parsed["assignments"].is_array());
    assert!(parsed["summary"].is_object());
}
