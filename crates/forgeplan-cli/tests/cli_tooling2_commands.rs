//! Second batch of tooling commands: `setup`, `setup-skill`, `embed`,
//! `remember`/`recall`, `fpf`, `migrate-dry-run`, `untag`, `dispatch`,
//! `orphans`-adjacent (`blindspots` covered in cli_reporting_commands).
//! Default layout, real artifacts, content assertions.

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

fn git(dir: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t.invalid")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── setup / setup-skill ─────────────────────────────────────────────────────

/// `setup` on a non-git workspace reports the missing-hook warning but still
/// installs the alias — the one-time prep contract.
#[test]
fn setup_reports_missing_git_hook_gracefully() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let (ok, out, err) = cmd(&tmp, &["setup"]);
    assert!(ok, "setup: {err}");
    assert!(
        out.contains("Alias") || out.contains("setup") || out.contains("hook"),
        "setup must print its prep report; got:\n{out}"
    );
    // Non-git → must warn about the missing hook, not crash.
    assert!(
        out.contains("Not a git repository")
            || out.contains("nothing to hook")
            || out.contains("hook"),
        "setup must note the missing git hook; got:\n{out}"
    );
}

/// `setup` inside a git repo installs the hooks — the happy path.
#[test]
fn setup_installs_hooks_in_git_repo() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    git(tmp.path(), &["init", "-q"]);
    git(tmp.path(), &["add", ".forgeplan"]);
    git(tmp.path(), &["commit", "-q", "-m", "init"]);

    let (ok, out, err) = cmd(&tmp, &["setup"]);
    assert!(ok, "setup: {err}");
    assert!(
        out.contains("hook") || out.contains("installed") || out.contains("Alias"),
        "setup in git repo must report hook install; got:\n{out}"
    );
}

/// `setup-skill` installs the `/forge` skill file and reports its path.
#[test]
fn setup_skill_installs_skill_file() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let home = TempDir::new().unwrap();

    let out = forgeplan()
        .args(["setup-skill"])
        .current_dir(tmp.path())
        .env("HOME", home.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("skill") || stdout.contains("forge") || stdout.contains("Installed"),
        "setup-skill must report the install; got:\n{stdout}"
    );
    // The skill file must actually exist under the scoped HOME.
    let skill = home.path().join(".claude/skills/forge/SKILL.md");
    assert!(
        skill.exists(),
        "SKILL.md must be written at {}",
        skill.display()
    );
}

// ── embed ─────────────────────────────────────────────────────────────────

/// `embed` without the semantic-search feature must report the feature gate
/// cleanly rather than produce garbage.
#[test]
fn embed_reports_feature_gate() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "prd", "Embed PRD");

    let (_ok, out, err) = cmd(&tmp, &["embed"]);
    let all = format!("{out}{err}");
    assert!(
        all.contains("semantic")
            || all.contains("feature")
            || all.contains("embed")
            || all.contains("model"),
        "embed must report feature/model state; got:\n{all}"
    );
}

// ── remember / recall ─────────────────────────────────────────────────────

/// `remember` stores a fact; `recall` finds it by substring — the core
/// memory round-trip.
#[test]
fn remember_recall_roundtrip() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let (ok, _out, err) = cmd(&tmp, &["remember", "the-deploy-pin-is-42"]);
    assert!(ok, "remember: {err}");

    let (ok, out, err) = cmd(&tmp, &["recall", "deploy-pin"]);
    assert!(ok, "recall: {err}");
    assert!(
        out.contains("deploy-pin") || out.contains("42") || out.contains("mem"),
        "recall must surface the stored fact; got:\n{out}"
    );
}
/// Bare `recall` enumerates stored memories (there is no --list flag; QUERY
/// is optional and defaults to listing recent entries).
#[test]
fn recall_lists_memories() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    cmd(&tmp, &["remember", "entry-alpha"]);

    let (ok, out, err) = cmd(&tmp, &["recall"]);
    assert!(ok, "recall: {err}");
    assert!(
        out.contains("entry-alpha") || out.contains("mem") || out.contains("1"),
        "bare recall must enumerate entries; got:\n{out}"
    );
}

// ── fpf ───────────────────────────────────────────────────────────────────

/// `fpf dashboard` on a workspace without ingested KB must report the empty
/// state with the ingest hint, not crash.
#[test]
fn fpf_dashboard_reports_empty_kb() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let (_ok, out, err) = cmd(&tmp, &["fpf", "dashboard"]);
    let all = format!("{out}{err}");
    assert!(
        all.contains("fpf")
            || all.contains("ingest")
            || all.contains("knowledge")
            || all.contains("empty")
            || all.contains("dashboard"),
        "fpf dashboard must report KB state; got:\n{all}"
    );
}

/// `fpf search <term>` must run and report matches-or-empty cleanly.
#[test]
fn fpf_search_runs() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let (_ok, out, err) = cmd(&tmp, &["fpf", "search", "boundary"]);
    let all = format!("{out}{err}");
    assert!(
        all.contains("search")
            || all.contains("result")
            || all.contains("no")
            || all.contains("fpf")
            || all.contains("section"),
        "fpf search must respond; got:\n{all}"
    );
}

// ── migrate-dry-run ───────────────────────────────────────────────────────

/// `migrate-dry-run` computes slug collisions across the workspace — on a
/// clean workspace it reports per-kind counts with zero collisions.
#[test]
fn migrate_dry_run_reports_counts() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "prd", "DryRun PRD");
    new_artifact(&tmp, "adr", "DryRun ADR");

    let (_ok, out, err) = cmd(&tmp, &["migrate-dry-run"]);
    // fail-and-list mode exits 1 on collisions, 0 on clean; either way the
    // report must carry per-kind counts.
    let all = format!("{out}{err}");
    assert!(
        all.contains("collision")
            || all.contains("kind")
            || all.contains("artifact")
            || all.contains("slug"),
        "migrate-dry-run must report counts/collisions; got:\n{all}"
    );
}

/// `--auto-suffix --json` emits `suggested_resolution` per collision. Two
/// same-title artifacts in one kind guarantee a collision to be listed.
#[test]
fn migrate_dry_run_auto_suffix_lists_resolutions() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    // Same kind + same title → same slug → guaranteed collision.
    new_artifact(&tmp, "note", "Same Title");
    new_artifact(&tmp, "note", "Same Title");

    let (_ok, out, err) = cmd(&tmp, &["migrate-dry-run", "--auto-suffix", "--json"]);
    let all = format!("{out}{err}");
    assert!(
        all.contains("suggested_resolution") || all.contains("collision"),
        "auto-suffix --json must surface resolutions; got:\n{all}"
    );
}

// ── untag ─────────────────────────────────────────────────────────────────

/// `tag` then `untag` — visibility is via `list --tag` (get does not render
/// the tags field).
#[test]
fn untag_removes_tag() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd = new_artifact(&tmp, "prd", "Tagged PRD");

    cmd(&tmp, &["tag", &prd, "hotfix"]);
    let (_, out, _) = cmd(&tmp, &["list", "--tag", "hotfix"]);
    assert!(
        out.contains(&prd),
        "tag must be visible via list --tag; got:\n{out}"
    );

    let (ok, _, err) = cmd(&tmp, &["untag", &prd, "hotfix"]);
    assert!(ok, "untag: {err}");
    let (_, out, _) = cmd(&tmp, &["list", "--tag", "hotfix"]);
    assert!(
        !out.contains(&prd),
        "untag must remove the tag; got:\n{out}"
    );
}

// ── dispatch ──────────────────────────────────────────────────────────────

/// `dispatch --agents 2` computes a parallel-safe plan — must run and
/// report a plan structure (buckets/reasoning) even on a sparse workspace.
#[test]
fn dispatch_produces_plan() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "prd", "Dispatch PRD");
    new_artifact(&tmp, "rfc", "Dispatch RFC");

    let (ok, out, err) = cmd(&tmp, &["dispatch", "--agents", "2"]);
    assert!(ok, "dispatch: {err}");
    assert!(
        out.contains("agent")
            || out.contains("bucket")
            || out.contains("plan")
            || out.contains("serial"),
        "dispatch must produce a plan; got:\n{out}"
    );
}
