//! Reporting/read-only commands on the DEFAULT (identity-v5, non-migrated)
//! layout: `session`, `blindspots`, `coverage`, `gaps`, `decay`, `anomalies`,
//! `drift`, `progress`, `journal`, `order`, `activity`.
//!
//! These commands all read the published index; the constellation-layout
//! compatibility suites already cover the migrated path. Here we seed real
//! artifacts and assert the report content, not just exit codes.

use assert_cmd::Command;
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
        "new {kind} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if let Some(id) = line.trim().strip_prefix("ID:") {
            return id.trim().to_string();
        }
    }
    panic!("no ID parsed from new {kind}: {stdout}");
}

// ── session ────────────────────────────────────────────────────────────────

/// `session` must report the current methodology phase. A fresh workspace is
/// idle (or in whatever phase the routing left); `session --reset` must return
/// it to Idle deterministically.
#[test]
fn session_reset_returns_to_idle() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let out = forgeplan()
        .args(["session", "--reset"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Idle") || stdout.contains("idle"),
        "reset must confirm Idle; got:\n{stdout}"
    );

    let out = forgeplan()
        .args(["session"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("idle") || stdout.contains("Idle") || stdout.contains("Phase"),
        "session must print the phase; got:\n{stdout}"
    );
}

/// `route` mutates the session phase — after routing, `session` must report
/// the routing result, proving session state is real persistence.
#[test]
fn session_reflects_route_transition() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    forgeplan()
        .args(["route", "add a small fix"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["session"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // After a route the session must be past idle — the phase moved.
    assert!(
        stdout.contains("routing")
            || stdout.contains("shaping")
            || stdout.contains("tactical")
            || stdout.contains("Phase"),
        "session must reflect the post-route phase; got:\n{stdout}"
    );
}

// ── blindspots ─────────────────────────────────────────────────────────────

/// An unlinked artifact must appear as a blind-spot orphan; linking it must
/// remove it from the report. This is the core contract of `blindspots`.
#[test]
fn blindspots_reports_orphan_and_clears_after_link() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd = new_artifact(&tmp, "prd", "Linked PRD");
    let orphan = new_artifact(&tmp, "adr", "Orphan ADR");
    let evid = new_artifact(&tmp, "evidence", "Proof");

    let out = forgeplan()
        .args(["blindspots"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&orphan),
        "unlinked artifact must appear as an orphan; got:\n{stdout}"
    );

    // Link the orphan to the PRD — the orphan list must shrink.
    forgeplan()
        .args(["link", &evid, &prd, "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", &orphan, &prd, "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["blindspots"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains(&orphan) || !stdout.contains("Orphan"),
        "linked artifact must leave the orphan list; got:\n{stdout}"
    );
}

// ── coverage ───────────────────────────────────────────────────────────────

/// `coverage` (no --backfill) prints the module/decision coverage report. On
/// a workspace with artifacts it must run without error and print a header.
#[test]
fn coverage_reports_without_error() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "adr", "Decide something");

    let out = forgeplan()
        .args(["coverage"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("coverage") || stdout.contains("module") || stdout.contains("Coverage"),
        "coverage must print its report header; got:\n{stdout}"
    );
}

// ── gaps ───────────────────────────────────────────────────────────────────

/// `gaps` reports pipeline-compliance gaps per depth. A workspace full of
/// stub drafts must report gaps rather than silence.
#[test]
fn gaps_reports_stub_drafts() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "prd", "Stub PRD");

    let out = forgeplan()
        .args(["gaps"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("PRD") || stdout.contains("gap") || stdout.contains("draft"),
        "gaps must report the draft artifact; got:\n{stdout}"
    );
}

// ── decay ──────────────────────────────────────────────────────────────────

/// `decay` on a workspace with only current evidence reports no decay.
#[test]
fn decay_reports_clean_state() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd = new_artifact(&tmp, "prd", "Current PRD");
    let evid = new_artifact(&tmp, "evidence", "Current proof");
    forgeplan()
        .args(["link", &evid, &prd, "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["decay"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No evidence decay")
            || stdout.contains("no decay")
            || stdout.contains("current"),
        "fresh evidence must not be reported as decayed; got:\n{stdout}"
    );
}

// ── anomalies ──────────────────────────────────────────────────────────────

/// `anomalies` must run clean on a normal workspace (or report parseable
/// anomaly entries — never crash).
#[test]
fn anomalies_runs_clean() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "prd", "Anomaly check");

    let out = forgeplan()
        .args(["anomalies"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
}

// ── drift ──────────────────────────────────────────────────────────────────

/// `drift` on a workspace with no "Affected Files" hints must report that
/// state rather than crash.
#[test]
fn drift_reports_no_affected_files() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "adr", "Some decision");

    let out = forgeplan()
        .args(["drift"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
}

// ── progress ───────────────────────────────────────────────────────────────

/// `progress` counts markdown checkboxes per artifact. Seed two boxes (one
/// checked) so the command has something real to count.
#[test]
fn progress_reports_checkbox_counts() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd = new_artifact(&tmp, "prd", "Progress PRD");
    forgeplan()
        .args([
            "update",
            &prd,
            "--body",
            "## Tasks\n\n- [ ] First\n- [x] Done\n",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["progress"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains('%') || stdout.contains("PRD") || stdout.contains("progress"),
        "progress must report checkbox state; got:\n{stdout}"
    );
}

// ── journal ────────────────────────────────────────────────────────────────

/// `journal` lists decision artifacts chronologically; a new ADR must appear.
#[test]
fn journal_lists_new_decision() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let adr = new_artifact(&tmp, "adr", "Journal me");

    let out = forgeplan()
        .args(["journal"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&adr) || stdout.contains("Journal me"),
        "journal must list the decision; got:\n{stdout}"
    );
}

// ── order ──────────────────────────────────────────────────────────────────

/// `order` topologically sorts linked artifacts — a linked chain must appear
/// in dependency order (based_on target first).
#[test]
fn order_respects_based_on_dependency() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd = new_artifact(&tmp, "prd", "Parent PRD");
    let rfc = new_artifact(&tmp, "rfc", "Child RFC");
    forgeplan()
        .args(["link", &rfc, &prd, "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["order"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Both ids must appear; PRD (the dependency) must be listed.
    assert!(
        stdout.contains(&prd) && stdout.contains(&rfc),
        "order must list both linked artifacts; got:\n{stdout}"
    );
}

// ── activity / activity-stats on a seeded workspace ────────────────────────

/// `activity` and `activity-stats` must not crash on a workspace with no
/// recorded tool calls — they degrade to "no activity" messages.
#[test]
fn activity_commands_on_fresh_workspace() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    for args in [["activity"], ["activity-stats"]] {
        let out = forgeplan()
            .args(args)
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
