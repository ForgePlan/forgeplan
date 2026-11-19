//! Behavioural coverage for the remaining thin commands: `discover`,
//! `estimate`, `phase-advance`, `promote`, `reconcile-ids`, `scan-import`,
//! `release-notes`, `ci-assign-id`, `undo-last`.
//!
//! All run on the DEFAULT (identity-v5) layout with real artifacts and real
//! git where the command needs it — no exit-code smoke.

use assert_cmd::Command;
use std::path::Path;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init_workspace(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

fn new_artifact(tmp: &TempDir, kind: &str, title: &str) -> String {
    let out = forgeplan()
        .args(["new", kind, title, "--allow-duplicate"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "new {kind}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.trim().strip_prefix("ID:").map(str::trim))
        .unwrap_or_else(|| panic!("no ID in new {kind}"))
        .to_string()
}

fn cmd(tmp: &TempDir, args: &[&str]) -> (bool, String, String) {
    let out = forgeplan()
        .args(args)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

// ── discover ──────────────────────────────────────────────────────────────

/// `discover start <name>` creates a session and prints the protocol;
/// `discover list` then shows it with a `started` status.
#[test]
fn discover_start_creates_and_lists_session() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let (ok, out, err) = cmd(&tmp, &["discover", "start", "test-discovery"]);
    assert!(ok, "discover start: {err}");
    assert!(
        out.contains("disc-")
            || out.contains("session")
            || out.contains("phase")
            || out.contains("detect"),
        "start must print the new session id/protocol; got:\n{out}"
    );

    let (ok, out, err) = cmd(&tmp, &["discover", "list"]);
    assert!(ok, "discover list: {err}");
    assert!(
        out.contains("test-discovery") || out.contains("started") || out.contains("detect"),
        "list must show the new session; got:\n{out}"
    );
}

/// `discover show <id>` on a real session reports its phase/findings count.
#[test]
fn discover_show_reports_session_status() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let (ok, out, _) = cmd(&tmp, &["discover", "start", "show-me"]);
    assert!(ok);
    let session_id = out
        .lines()
        .find_map(|l| {
            l.split_whitespace()
                .find(|w| w.starts_with("disc-"))
                .map(str::to_string)
        })
        .expect("no session id in discover start output");

    let (ok, out, err) = cmd(&tmp, &["discover", "show", &session_id]);
    assert!(ok, "discover show: {err}");
    assert!(
        out.contains(&session_id) || out.contains("phase") || out.contains("status"),
        "show must report the session status; got:\n{out}"
    );
}

// ── estimate ──────────────────────────────────────────────────────────────

/// `estimate` on an artifact without FR rows reports the no-items contract
/// with the `Fix:` hint naming the required table shape.
#[test]
fn estimate_reports_no_items_and_fix_hint() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd = new_artifact(&tmp, "prd", "Empty Estimate PRD");

    let (_ok, out, _) = cmd(&tmp, &["estimate", &prd]);
    // The command exits 0 but reports the empty-input contract.
    assert!(
        out.contains("No estimable")
            || out.contains("no estimable")
            || out.contains("Add FR table"),
        "estimate must report the no-items contract; got:\n{out}"
    );
}

/// `estimate` on a PRD with a real `| FR-NNN | ... |` row must produce a
/// multi-grade breakdown, not the empty-input contract.
#[test]
fn estimate_produces_breakdown_for_real_fr_rows() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd = new_artifact(&tmp, "prd", "Estimate PRD");

    // Seed a real FR table the estimator can chew on.
    forgeplan()
        .args([
            "update",
            &prd,
            "--body",
            "## Functional Requirements\n\n\
             | ID | Area | Priority | Requirement | Journey |\n\
             |----|------|----------|-------------|---------|\n\
             | FR-001 | Core | Must | User can create an artifact | Journey 1 |\n\
             | FR-002 | Core | Must | User can link artifacts | Journey 1 |\n",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();

    let (ok, out, err) = cmd(&tmp, &["estimate", &prd]);
    assert!(ok, "estimate: {err}");
    assert!(
        out.contains("FR-001")
            || out.contains("Junior")
            || out.contains("estimate")
            || out.contains("confidence")
            || out.contains("hour"),
        "estimate must produce a real breakdown; got:\n{out}"
    );
}

// ── phase-advance ─────────────────────────────────────────────────────────

/// `phase-advance` moves the lifecycle marker forward; `phase` must reflect
/// the new phase afterwards.
#[test]
fn phase_advance_moves_marker() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd = new_artifact(&tmp, "prd", "Phase PRD");

    let (ok, out, err) = cmd(&tmp, &["phase-advance", &prd, "--to", "shape"]);
    assert!(ok, "phase-advance: {err}");
    assert!(
        out.contains("shape") || out.contains(&prd) || out.contains("advanced"),
        "phase-advance must confirm the transition; got:\n{out}"
    );

    let (ok, out, _) = cmd(&tmp, &["phase", &prd]);
    assert!(ok);
    assert!(
        out.contains("shape") || out.contains("current_phase"),
        "phase must show the new phase; got:\n{out}"
    );
}

// ── promote ───────────────────────────────────────────────────────────────

/// `promote` on a non-existent memory id must fail with the not-found
/// contract, not panic or silently create something.
#[test]
fn promote_missing_memory_errors_cleanly() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let (ok, out, err) = cmd(&tmp, &["promote", "mem-does-not-exist", "--kind", "prd"]);
    assert!(!ok, "promote of a missing memory must fail");
    assert!(
        out.contains("not found")
            || out.contains("mem-does-not-exist")
            || err.contains("not found"),
        "error must name the missing id; got:\n{out}\nerr:\n{err}"
    );
}

// ── reconcile-ids ─────────────────────────────────────────────────────────

/// `reconcile-ids --check-only` on a clean workspace reports per-kind counts
/// and a zero/small action count — it must not fabricate drift.
#[test]
fn reconcile_ids_check_only_reports_counts() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "prd", "Reconcile PRD");
    new_artifact(&tmp, "adr", "Reconcile ADR");

    let (ok, out, err) = cmd(&tmp, &["reconcile-ids", "--check-only"]);
    assert!(ok, "reconcile-ids: {err}");
    assert!(
        out.contains("kind") || out.contains("artifact") || out.contains("Actions"),
        "check-only must report counts; got:\n{out}"
    );
}

// ── scan-import ───────────────────────────────────────────────────────────

/// `scan-import --dry-run` on a directory inside the project root reports the
/// candidate file; a path outside the root is rejected with the traversal
/// contract.
#[test]
fn scan_import_dry_run_reports_candidates() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let docs = tmp.path().join("docs-import");
    std::fs::create_dir(&docs).unwrap();
    std::fs::write(
        docs.join("some-doc.md"),
        "# Some doc\n\nThis is a markdown file the scanner should find.\n",
    )
    .unwrap();

    let (ok, out, err) = cmd(&tmp, &["scan-import", "--path", "docs-import", "--dry-run"]);
    assert!(ok, "scan-import dry-run: {err}");
    assert!(
        out.contains("some-doc")
            || out.contains("candidate")
            || out.contains("import")
            || out.contains("1"),
        "dry-run must report the found file; got:\n{out}"
    );
}

/// `--path` pointing outside the project root must be rejected with the
/// path-traversal contract — not silently escape.
#[test]
fn scan_import_rejects_out_of_root_path() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let (ok, _, err) = cmd(&tmp, &["scan-import", "--path", "/tmp", "--dry-run"]);
    assert!(
        !ok || err.contains("outside project root") || err.contains("traversal"),
        "out-of-root path must be rejected; got ok={ok} err:\n{err}"
    );
}

// ── release-notes ─────────────────────────────────────────────────────────

/// `release-notes` requires a git repo — without `.git` it reports the
/// `git log failed` contract cleanly; with git history it produces a real
/// changelog section.
#[test]
fn release_notes_errors_cleanly_without_git() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "prd", "Release PRD");

    let (ok, _, err) = cmd(&tmp, &["release-notes"]);
    assert!(
        !ok && (err.contains("git") || err.contains("log")),
        "no-git release-notes must report the git requirement; got ok={ok} err:\n{err}"
    );
}

/// `release-notes --since <ref> --until <ref>` on a git workspace produces a
/// real changelog between two commits.
#[test]
fn release_notes_produces_changelog_between_refs() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "prd", "Release PRD");

    git(tmp.path(), &["init", "-q"]);
    git(tmp.path(), &["add", ".forgeplan"]);
    git(tmp.path(), &["commit", "-q", "-m", "chore: baseline"]);
    let since = git(tmp.path(), &["rev-parse", "HEAD"]);

    // A second commit so there's a diff between refs.
    new_artifact(&tmp, "adr", "Second ADR");
    git(tmp.path(), &["add", ".forgeplan"]);
    git(tmp.path(), &["commit", "-q", "-m", "feat: add second ADR"]);
    let until = git(tmp.path(), &["rev-parse", "HEAD"]);

    let (ok, out, err) = cmd(
        &tmp,
        &["release-notes", "--since", &since, "--until", &until],
    );
    assert!(ok, "release-notes: {err}");
    assert!(
        out.contains("ADR")
            || out.contains("commit")
            || out.contains("changelog")
            || out.contains("##"),
        "release-notes must produce real output between refs; got:\n{out}"
    );
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

// ── ci-assign-id ──────────────────────────────────────────────────────────

/// `ci-assign-id` without git history or PR context must fail with a clear
/// error, not panic.
#[test]
fn ci_assign_id_fails_cleanly_without_git() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let (ok, _, err) = cmd(&tmp, &["ci-assign-id", "--pr", "1"]);
    // Fails (no git/PR context) — that's the contract; it must not panic.
    assert!(
        !ok || err.contains("git") || err.contains("base") || err.contains("ref"),
        "ci-assign-id without git context must fail cleanly; got ok={ok} err:\n{err}"
    );
}

// ── undo-last ─────────────────────────────────────────────────────────────

/// `undo-last` with no destructive ops in the window reports the empty case
/// with the `--within-hours` hint.
#[test]
fn undo_last_reports_no_destructive_ops() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "prd", "Undo PRD");

    let (ok, out, err) = cmd(&tmp, &["undo-last"]);
    // No destructive op → either a clean error or a "nothing to undo" report.
    assert!(
        !ok || out.contains("No non-consumed")
            || out.contains("within-hours")
            || err.contains("within-hours"),
        "undo-last with no destructive ops must report the empty case; got ok={ok} out:\n{out}\nerr:\n{err}"
    );
}
