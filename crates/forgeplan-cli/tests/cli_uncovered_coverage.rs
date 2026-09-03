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

/// PROB-088 M2 / PRD-083 FR-006 — the `Fix:` line must be runnable by the
/// audience that reaches it.
///
/// This error is reachable from a **prebuilt binary** (brew, install.sh,
/// GitHub Releases). Such a user has no source tree, so `cargo build` is
/// inert advice — it fails with "could not find Cargo.toml" and leaves them
/// stuck. `cargo install --git` carries its own source and works from an
/// empty directory.
///
/// The assertion is deliberately negative: it pins the property (no
/// checkout-dependent command) rather than the exact wording, so rephrasing
/// the message stays free while a regression to `cargo build` fails loudly.
#[test]
fn embed_fix_hint_is_runnable_without_a_checkout() {
    let tmp = init_workspace();

    let output = forgeplan()
        .args(["embed"])
        .current_dir(tmp.path())
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&output.get_output().stderr).to_string();

    let fix_line = stderr
        .lines()
        .find(|l| l.trim_start().starts_with("Fix:"))
        .unwrap_or_else(|| panic!("no `Fix:` line in embed refusal output:\n{stderr}"));

    assert!(
        !fix_line.contains("cargo build"),
        "`Fix:` must not tell a binary-install user to run `cargo build` — \
         they have no source tree. Got: {fix_line}"
    );
    assert!(
        fix_line.contains("cargo install"),
        "`Fix:` should offer a self-contained install command. Got: {fix_line}"
    );
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

/// Issue #411 end-to-end: the memory list must show WHO captured the fact.
///
/// Exercises the whole chain in one shot — `resolve_author` tier 1 (git
/// config), the frontmatter + LanceDB author write, `resolve_display_author`
/// reading the column back, and `shorten_author` dropping the address.
#[test]
fn remember_list_shows_the_git_author_column() {
    // Mirrors the skip guard in `forgeplan_core::git::author` tests.
    if std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let tmp = init_workspace();

    // LOCAL git config overrides global/system, so tier 1 of resolve_author
    // is deterministic regardless of the developer's own ~/.gitconfig.
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["config", "user.name", "Ada Lovelace"],
        vec!["config", "user.email", "ada@example.org"],
    ] {
        std::process::Command::new("git")
            .args(&args)
            .current_dir(tmp.path())
            .output()
            .unwrap();
    }

    // GIT_AUTHOR_* now OUTRANKS git config (issue #411 env tier). A
    // developer shell that exports them — or a test run from inside
    // `git rebase` / `git am`, both of which export the pair — would
    // otherwise satisfy tier 1 before the local config is consulted and
    // this test would assert the ambient environment, not the repo.
    forgeplan()
        .args(["remember", "LanceDB index is derived, never commit it"])
        .env_remove("GIT_AUTHOR_NAME")
        .env_remove("GIT_AUTHOR_EMAIL")
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["remember", "--list"])
        .env_remove("GIT_AUTHOR_NAME")
        .env_remove("GIT_AUTHOR_EMAIL")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Author"))
        .stdout(predicate::str::contains("Ada Lovelace"))
        // "Ada Lovelace <ada@example.org>" is 30 chars > AUTHOR_COL_MAX:
        // the address is dropped whole, never cut into.
        .stdout(predicate::str::contains("ada@example.org").not())
        // The regression guard: a revert to the hardcoded literal shows up
        // here and nowhere else.
        .stdout(predicate::str::contains("cli").not());
}

/// Issue #411: the agent-facing surface reported the "when" but not the
/// "who". `recall --json` must carry provenance, and carry it WHOLE.
#[test]
fn recall_json_carries_provenance() {
    let tmp = init_workspace();

    forgeplan()
        .args(["remember", "PostgreSQL for concurrent writes, not SQLite"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["recall", "PostgreSQL", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();
    let payload: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let mem = &payload["memories"][0];

    // The key must exist at all — an agent has no other provenance channel.
    let author = mem.get("author").expect("recall --json must expose author");
    assert!(
        author.as_str().is_some_and(|s| !s.is_empty()),
        "author must be a resolved value, got {author}"
    );
    // Unshortened: the payload is machine-read, no width budget applies.
    assert!(!author.as_str().unwrap().ends_with('…'));
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

// ---------------------------------------------------------------------------
// setup — `fpl` alias + embedding model preparation
// ---------------------------------------------------------------------------

/// The alias step must work on a build with no embedding support, since that
/// is exactly what the prebuilt binaries are (PROB-088 / ADR-022).
#[test]
fn setup_creates_the_fpl_alias_next_to_the_binary() {
    let dir = TempDir::new().unwrap();
    let exe = dir.path().join("forgeplan");
    std::fs::copy(assert_cmd::cargo::cargo_bin("forgeplan"), &exe).unwrap();

    Command::new(&exe)
        .args(["setup", "--skip-model"])
        .assert()
        .success();

    let alias = dir.path().join("fpl");
    assert!(alias.exists(), "expected the alias at {}", alias.display());
    assert_eq!(
        std::fs::read_link(&alias).unwrap(),
        exe,
        "alias must point at the binary that created it, not a guessed path"
    );
}

/// Running setup twice is a normal thing to do; the second run must not fail
/// or report a fresh creation it did not perform.
#[test]
fn setup_is_idempotent() {
    let dir = TempDir::new().unwrap();
    let exe = dir.path().join("forgeplan");
    std::fs::copy(assert_cmd::cargo::cargo_bin("forgeplan"), &exe).unwrap();

    Command::new(&exe)
        .args(["setup", "--skip-model"])
        .assert()
        .success();

    Command::new(&exe)
        .args(["setup", "--skip-model"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already in place"));
}

/// A user may already have their own `fpl` on PATH. Overwriting someone's
/// binary because our command happened to want the name would be indefensible.
#[test]
fn setup_refuses_to_overwrite_an_existing_fpl() {
    let dir = TempDir::new().unwrap();
    let exe = dir.path().join("forgeplan");
    std::fs::copy(assert_cmd::cargo::cargo_bin("forgeplan"), &exe).unwrap();

    let alias = dir.path().join("fpl");
    std::fs::write(&alias, b"someone elses tool").unwrap();

    Command::new(&exe)
        .args(["setup", "--skip-model"])
        .assert()
        .success();

    assert_eq!(
        std::fs::read(&alias).unwrap(),
        b"someone elses tool",
        "a foreign file at the alias path must survive untouched"
    );
}

/// On a build without the feature there is no model to fetch. Saying so beats
/// either a silent no-op or a confident claim that something was prepared.
#[test]
#[cfg(not(feature = "semantic-search"))]
fn setup_explains_itself_when_the_build_cannot_embed() {
    let dir = TempDir::new().unwrap();
    let exe = dir.path().join("forgeplan");
    std::fs::copy(assert_cmd::cargo::cargo_bin("forgeplan"), &exe).unwrap();

    Command::new(&exe)
        .arg("setup")
        .assert()
        .success()
        .stdout(predicate::str::contains("no semantic-search feature"))
        .stdout(predicate::str::contains("cargo install --git"));
}

/// PROB-088 lesson applied to our own flags: `-y` must never pull gigabytes.
/// Agents and CI runners call `init -y` routinely; a 2.1 GB download there
/// would be a denial of service on someone's build.
#[test]
fn init_non_interactive_never_mentions_or_fetches_the_model() {
    let dir = TempDir::new().unwrap();

    let out = forgeplan()
        .args(["init", "-y"])
        .current_dir(dir.path())
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_lowercase();
    assert!(
        !stdout.contains("download"),
        "`init -y` must not start or announce a model download: {stdout}"
    );
}

/// `--with-model` is explicit opt-in and has to be honoured on the
/// non-interactive path too. This regressed once already: the flag parsed
/// fine and did nothing, because `-y` returns early on its own code path and
/// never reached the preparation step. Only an end-to-end run caught it.
#[test]
fn init_with_model_flag_is_accepted_on_the_non_interactive_path() {
    let dir = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y", "--with-model"])
        .current_dir(dir.path())
        .assert()
        .success();

    // On a default build there is nothing to download, so the assertion is
    // about the flag being wired at all — it must not error, and the
    // workspace must still be created.
    assert!(dir.path().join(".forgeplan").is_dir());
}
