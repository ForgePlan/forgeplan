//! Local compatibility of the ordinary CLI surfaces AFTER `forgeplan index`
//! hands `.forgeplan/lance/` to the Constellation protocol (S1-R1).
//!
//! Before the cutover, `health` / `journal` / `decay` / `drift` / `anomalies`
//! / `release-notes` / `blindspots` / `status` all went through
//! `common::open_store`, which refuses outright once the index is migrated:
//! the legacy `artifacts.lance` is retired on purpose, and opening it would
//! recreate it. These tests pin the behaviour that replaced the refusal —
//! the same concrete values a legacy workspace produces, no retired table,
//! and an honest error when the capture itself cannot be read.
//!
//! Every case drives the real binary in a temp workspace. No mock, no fixture
//! index, no pre-baked JSON.

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// `init -y` — a legacy-layout workspace (no `CURRENT.json`).
fn init_legacy() -> TempDir {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    tmp
}

/// `init -y` + `index` — the constellation-managed layout, with the same
/// artifact bytes on disk.
fn init_migrated() -> TempDir {
    let tmp = init_legacy();
    forgeplan()
        .args(["index"])
        .current_dir(tmp.path())
        .assert()
        .success();
    assert!(
        tmp.path().join(".forgeplan/lance/CURRENT.json").exists(),
        "forgeplan index must publish CURRENT.json — otherwise this is not \
         the migrated layout these tests claim to cover"
    );
    tmp
}

fn ws_of(root: &std::path::Path) -> std::path::PathBuf {
    root.join(".forgeplan")
}

fn artifact(
    root: &std::path::Path,
    rel: &str,
    body: &str,
) -> std::path::PathBuf {
    let path = ws_of(root).join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, body).unwrap();
    path
}

fn run(root: &std::path::Path, args: &[&str]) -> (bool, String, String) {
    let out = forgeplan().args(args).current_dir(root).output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run_ok(root: &std::path::Path, args: &[&str]) -> String {
    let (ok, stdout, stderr) = run(root, args);
    assert!(ok, "`forgeplan {}` failed: {stderr}", args.join(" "));
    stdout
}

fn run_json(root: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let stdout = run_ok(root, args);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("`forgeplan {}` did not emit JSON ({e}): {stdout}", args.join(" ")))
}

/// A migrated workspace must not have grown a legacy artifact table.
fn assert_no_retired_tables(root: &std::path::Path) {
    let lance = ws_of(root).join("lance");
    for legacy in ["artifacts.lance", "evidence.lance", "relations.lance"] {
        assert!(
            !lance.join(legacy).exists(),
            "{legacy} was recreated on a constellation-managed workspace: {:?}",
            inventory(&lance)
        );
    }
}

/// A markdown inventory, so the "nothing changed" assertions compare a real
/// list instead of an empty string.
fn inventory(dir: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in rd.flatten() {
        out.push(entry.file_name().to_string_lossy().to_string());
    }
    out.sort();
    out
}

/// Run a git command in `root`, failing the test on a non-zero exit.
///
/// Release notes are built from a real `git log` walk: without committed
/// history and an explicit ref range, the walker has nothing to classify and
/// the command's success says nothing about the migrated read path.
fn git(root: &std::path::Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        // Identity is set per invocation so the test never depends on, or
        // mutates, the developer's global git config.
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .unwrap_or_else(|e| panic!("git {args:?} could not run: {e}"));
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Turn the tempdir into a git repository with one commit tagged `v0.1.0`, so
/// `--since v0.1.0` is a real range with real artifacts in it.
fn init_git_history(root: &std::path::Path) {
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "initial"]);
    git(root, &["tag", "v0.1.0"]);
}

/// Seed a workspace with a PRD linked to an evidence pack plus one orphan
/// RFC.
///
/// Both layouts must hold the SAME population, not merely the same bytes:
/// the legacy layout enumerates LanceDB rows while the migrated one captures
/// the files. Writing markdown and stopping would leave the legacy side empty,
/// so every parity assertion would be comparing two different sets.
///
/// `forgeplan reindex` is the supported files-first command that fills the
/// legacy rows from the markdown — exactly the ordinary mutation an operator
/// would run.
fn seed_artifacts(root: &std::path::Path) {
    artifact(
        root,
        "prds/PRD-001-auth.md",
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth rewrite\n\
         slug: prd-auth-rewrite\ndepth: standard\n\
         links:\n  - target: EVID-001\n    relation: informs\n---\n\n\
         ## Summary\n\nRewrite authentication.\n",
    );
    artifact(
        root,
        "evidence/EVID-001-stress.md",
        "---\nid: EVID-001\nkind: evidence\nstatus: active\ntitle: Stress run\n\
         depth: standard\n---\n\nverdict: supports\ncongruence_level: 3\n\
         evidence_type: measurement\n",
    );
    artifact(
        root,
        "rfcs/RFC-002-wire.md",
        "---\nid: RFC-002\nkind: rfc\nstatus: draft\ntitle: Wire format\ndepth: standard\n---\n\n## Summary\n\nUnrelated.\n",
    );
    populate_legacy_index(root);
}

/// Project the markdown into the legacy rows so a legacy-layout workspace
/// answers from the same population the migrated one captures.
///
/// Skipped on a migrated workspace: there the capture IS the file set, and
/// `reindex` is deliberately refused there rather than resurrecting the
/// retired tables.
fn populate_legacy_index(root: &std::path::Path) {
    if ws_of(root).join("lance/CURRENT.json").exists() {
        return;
    }
    let (ok, _out, stderr) = run(root, &["reindex"]);
    assert!(ok, "reindex failed while seeding: {stderr}");
}

/// The same fixture plus one relation whose target does not exist — an
/// `orphan_link`, severity High, independent of any clock so the comparison
/// between layouts is deterministic.
fn seed_artifacts_with_dangling_link(root: &std::path::Path) {
    artifact(
        root,
        "prds/PRD-001-auth.md",
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth rewrite\n\
         slug: prd-auth-rewrite\ndepth: standard\n\
         links:\n  - target: EVID-001\n    relation: informs\n\
         ---\n\n## Summary\n\nRewrite authentication.\n",
    );
    artifact(
        root,
        "evidence/EVID-001-stress.md",
        "---\nid: EVID-001\nkind: evidence\nstatus: active\ntitle: Stress run\n\
         depth: standard\n---\n\nverdict: supports\ncongruence_level: 3\n\
         evidence_type: measurement\n",
    );
    artifact(
        root,
        "rfcs/RFC-002-wire.md",
        "---\nid: RFC-002\nkind: rfc\nstatus: draft\ntitle: Wire format\ndepth: standard\n\
         links:\n  - target: PRD-404\n    relation: refines\n---\n\n## Summary\n\nPoints at an artifact that is not there.\n",
    );
    populate_legacy_index(root);
}

/// A linked ADR plus a note with no evidence — the two shapes the decision
/// journal actually contains (`DECISION_KINDS_JOURNAL` is adr/note/problem/
/// solution, not prd/rfc).
fn seed_decision_artifacts(root: &std::path::Path) {
    artifact(
        root,
        "adrs/ADR-030-storage.md",
        "---\nid: ADR-030\nkind: adr\nstatus: active\ntitle: Use Postgres\ndepth: standard\n\
         links:\n  - target: EVID-030\n    relation: based_on\n---\n\n## Decision\n\nPostgres.\n",
    );
    artifact(
        root,
        "evidence/EVID-030-measure.md",
        "---\nid: EVID-030\nkind: evidence\nstatus: active\ntitle: Load run\ndepth: standard\n\
         ---\n\nverdict: supports\ncongruence_level: 3\nevidence_type: measurement\n",
    );
    artifact(
        root,
        "notes/NOTE-031-followup.md",
        "---\nid: NOTE-031\nkind: note\nstatus: draft\ntitle: Follow up\ndepth: standard\n---\n\nLater.\n",
    );
    populate_legacy_index(root);
}

// ── report readers answer on the migrated layout ──────────────────────────

#[test]
fn health_reports_the_same_verdict_before_and_after_index() {
    let legacy = init_legacy();
    let migrated = init_migrated();
    for root in [legacy.path(), migrated.path()] {
        seed_artifacts(root);
    }

    let before = run_json(legacy.path(), &["health", "--json"]);
    let after = run_json(migrated.path(), &["health", "--json"]);

    assert_eq!(
        after["verdict"], before["verdict"],
        "the migrated layout must reach the same verdict as the legacy one"
    );
    assert_eq!(after["total"], before["total"], "artifact count differs");
    assert_eq!(
        after["by_status"], before["by_status"],
        "status buckets differ between layouts"
    );
    // The linked PRD must not be reported as a blind spot on either layout.
    assert!(
        after["blind_spots"]
            .as_array()
            .map(|b| b.is_empty())
            .unwrap_or(false),
        "a PRD linked to evidence is not a blind spot, got {}",
        after["blind_spots"]
    );
    // `by_kind` is a real, non-empty dashboard on both, and the PRD count
    // is 1 in each — the counts are the values, not just the presence.
    let prd_count = |v: &serde_json::Value| -> u64 {
        v["by_kind"]
            .as_array()
            .expect("by_kind is a list of {kind,count}")
            .iter()
            .find(|e| e["kind"] == "prd")
            .and_then(|e| e["count"].as_u64())
            .unwrap_or(0)
    };
    assert_eq!(prd_count(&after), 1, "by_kind: {}", after["by_kind"]);
    assert_eq!(prd_count(&before), prd_count(&after));
    assert_no_retired_tables(migrated.path());
}

#[test]
fn health_strict_still_fires_on_a_migrated_workspace() {
    // The verdict contract must survive the cutover, not just the display:
    // an orphan RFC is a critical signal on both layouts.
    let migrated = init_migrated();
    seed_artifacts(migrated.path());

    let out = forgeplan()
        .args(["health", "--strict"])
        .current_dir(migrated.path())
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "--strict must exit 1 on an orphan; stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn journal_lists_the_linked_decision_after_index() {
    let migrated = init_migrated();
    seed_decision_artifacts(migrated.path());

    let stdout = run_ok(&migrated.path(), &["journal"]);
    assert!(
        stdout.contains("ADR-030"),
        "a linked ADR must appear in the journal, got: {stdout}"
    );
    assert!(
        !stdout.contains("RFC-002"),
        "an RFC is not a journal decision kind, got: {stdout}"
    );
    assert_no_retired_tables(&migrated.path());
}

#[test]
fn journal_kind_filter_matches_the_same_records_after_index() {
    let migrated = init_migrated();
    seed_decision_artifacts(migrated.path());

    // The flag is `--type`; an ADR filter returns the ADR and not the note.
    let adr = run_ok(&migrated.path(), &["journal", "--type", "adr"]);
    assert!(adr.contains("ADR-030"), "got: {adr}");
    assert!(!adr.contains("NOTE-031"), "--type adr returned a note: {adr}");
    let note = run_ok(&migrated.path(), &["journal", "--type", "note"]);
    assert!(note.contains("NOTE-031"), "got: {note}");
    assert!(!note.contains("ADR-030"), "--type note returned an ADR: {note}");
}

#[test]
fn blindspots_sees_the_orphan_after_index() {
    let migrated = init_migrated();
    seed_artifacts(migrated.path());

    let stdout = run_ok(&migrated.path(), &["blindspots"]);
    assert!(
        stdout.contains("RFC-002"),
        "the unlinked RFC must be reported as an orphan, got: {stdout}"
    );
    assert_no_retired_tables(&migrated.path());
}

#[test]
fn decay_reports_no_decay_for_current_evidence_after_index() {
    let migrated = init_migrated();
    seed_artifacts(migrated.path());

    let (ok, stdout, stderr) = run(migrated.path(), &["decay"]);
    assert!(ok, "decay failed: {stderr}");
    assert!(
        stdout.contains("No evidence decay detected"),
        "an unexpired evidence pack must not be reported as decayed, got: {stdout}"
    );
}

#[test]
fn drift_runs_after_index_and_reports_the_decision_it_checked() {
    // Drift only inspects an ACTIVE ADR/RFC that declares Affected Files, so
    // the fixture must be one of those — otherwise the assertion would pass
    // on an empty list and prove nothing.
    let migrated = init_migrated();
    artifact(
        migrated.path(),
        "adrs/ADR-010-storage.md",
        "---\nid: ADR-010\nkind: adr\nstatus: active\ntitle: Use Postgres\ndepth: standard\n\
         ---\n\n## Affected Files\n\n- src/db.rs\n",
    );

    let payload = run_json(&migrated.path(), &["drift", "--json"]);
    let reports = payload["reports"].as_array().expect("reports array");
    let ids: Vec<&str> = reports
        .iter()
        .filter_map(|r| r["id"].as_str())
        .collect();
    assert!(
        ids.contains(&"ADR-010"),
        "an active ADR with an Affected Files section must be drift-checked, got {payload}"
    );
    // Outside a git repository nothing is stale; the check still completes.
    assert!(
        reports
            .iter()
            .find(|r| r["id"] == "ADR-010")
            .and_then(|r| r["is_stale"].as_bool())
            .is_some(),
        "the report must state staleness explicitly, got {payload}"
    );
    assert_no_retired_tables(&migrated.path());
}

#[test]
fn anomalies_run_after_index_and_detect_the_same_thing_as_before_index() {
    // The comparison is against the LEGACY workspace, not a hard-coded count:
    // what matters is that the migrated layout computes the same detection.
    let legacy = init_legacy();
    let migrated = init_migrated();
    for root in [legacy.path(), migrated.path()] {
        seed_artifacts_with_dangling_link(root);
    }

    let before = run_json(&legacy.path(), &["anomalies", "--json"]);
    let after = run_json(&migrated.path(), &["anomalies", "--json"]);

    for field in ["anomalies", "total", "by_severity", "by_tier"] {
        assert!(
            after.get(field).is_some(),
            "anomalies must keep its shape on the migrated layout, missing {field}: {after}"
        );
    }
    let kinds = |v: &serde_json::Value| -> Vec<String> {
        let mut k: Vec<String> = v["anomalies"]
            .as_array()
            .expect("anomalies list")
            .iter()
            .map(|a| a["kind"].as_str().unwrap_or("?").to_string())
            .collect();
        k.sort();
        k
    };
    assert_eq!(
        kinds(&after),
        kinds(&before),
        "the migrated layout must detect the same anomalies as the legacy one"
    );
    assert!(
        kinds(&after).iter().any(|k| k == "orphan_link"),
        "a relation to PRD-404, which does not exist, must be detected, got {after}"
    );
    assert_no_retired_tables(&migrated.path());
}

#[test]
fn status_counts_the_same_artifacts_after_index() {
    let legacy = init_legacy();
    let migrated = init_migrated();
    for root in [legacy.path(), migrated.path()] {
        seed_artifacts(root);
    }

    let before = run_ok(&legacy.path(), &["status"]);
    let after = run_ok(&migrated.path(), &["status"]);

    let count = |s: &str| -> String {
        s.lines()
            .find(|l| l.contains("Artifacts:"))
            .unwrap_or_else(|| panic!("no artifact count in: {s}"))
            .trim()
            .to_string()
    };
    assert_eq!(count(&after), count(&before));
    assert!(after.contains("prd"), "by-kind breakdown missing: {after}");
    assert_no_retired_tables(&migrated.path());
}

#[test]
fn release_notes_build_real_classifications_after_index() {
    // Release notes are produced by a real `git log` walk between two refs.
    // Without committed history and an explicit range there is nothing to
    // classify, and "the command exited 0" would prove nothing about the
    // migrated read path.
    let migrated = init_migrated();
    init_git_history(migrated.path());
    // Touch one artifact AFTER the tag, then amend nothing — the range
    // v0.1.0..HEAD now contains a real change to a real artifact.
    artifact(
        migrated.path(),
        "rfcs/RFC-002-wire.md",
        "---\nid: RFC-002\nkind: rfc\nstatus: active\ntitle: Wire format\ndepth: standard\n\
         ---\n\n## Summary\n\nChanged after the tag.\n",
    );
    git(migrated.path(), &["add", "-A"]);
    git(migrated.path(), &["commit", "-q", "-m", "wire format"]);

    let payload = run_json(migrated.path(), &["release-notes", "--since", "v0.1.0", "--json"]);
    assert_eq!(
        payload["since"], "v0.1.0",
        "the explicit --since range must be echoed, got {payload}"
    );
    // An active RFC that changed after the tag classifies into `changed`.
    let changed = payload["changed"]
        .as_array()
        .unwrap_or_else(|| panic!("`changed` must be a list, got {payload}"));
    assert!(
        changed.iter().any(|e| e["id"] == "RFC-002"),
        "the artifact changed since the tag must be classified, got {payload}"
    );
    assert!(
        payload["total"].as_u64().unwrap_or(0) > 0,
        "a non-empty range must produce entries, got {payload}"
    );
    assert!(
        !stderr_mentions_retired_store(migrated.path()),
        "release-notes fell back to the retired store"
    );
    assert_no_retired_tables(migrated.path());
}

/// Run release-notes once and report whether its stderr names the retired
/// dataset — the error the migration exists to remove.
fn stderr_mentions_retired_store(root: &std::path::Path) -> bool {
    let (_, _out, stderr) = run(root, &["release-notes", "--since", "v0.1.0"]);
    stderr.contains("artifacts.lance was not found")
}

#[test]
fn index_prints_the_real_sections_version_and_never_a_placeholder() {
    // The three tables are always pinned, so `sections@v-` is a state the
    // index can no longer be in. The same holds for the verification line:
    // a staged pass must report the version it actually pinned.
    let migrated = init_migrated();
    seed_artifacts(migrated.path());

    let stdout = run_ok(migrated.path(), &["index"]);

    let tables = stdout
        .lines()
        .find(|l| l.contains("artifacts@v"))
        .unwrap_or_else(|| panic!("no tables line in: {stdout}"));
    assert!(
        !tables.contains("sections@v-"),
        "the sections version must be printed, not a `-` placeholder: {tables}"
    );
    // The real version is a positive integer — this pins that the value is
    // read from the outcome, not merely that the placeholder is gone.
    let sections_version = tables
        .split("sections@v")
        .nth(1)
        .unwrap()
        .trim()
        .parse::<u64>()
        .unwrap_or_else(|e| panic!("sections version is not a number ({e}): {tables}"));
    assert!(
        sections_version > 0,
        "the pinned sections version must be positive, got {tables}"
    );

    // The verification line, when present, carries a version too.
    if let Some(verified) = stdout.lines().find(|l| l.trim_start().starts_with("verified:")) {
        assert!(
            !verified.contains("sections v-"),
            "the verification line must not report an absent sections version: {verified}"
        );
    }
}

// ── the failure side: a capture that cannot be read must say so ────────────

#[test]
fn a_corrupt_current_pointer_is_an_error_not_an_empty_report() {
    let migrated = init_migrated();
    seed_artifacts(migrated.path());

    // Corrupt the published pointer, not the workspace.
    std::fs::write(ws_of(migrated.path()).join("lance/CURRENT.json"), "{ not json")
        .unwrap();

    for cmd in [
        vec!["health", "--json"],
        vec!["journal"],
        vec!["decay"],
        vec!["anomalies", "--json"],
    ] {
        let (ok, stdout, stderr) = run(migrated.path(), &cmd);
        assert!(
            !ok,
            "`forgeplan {}` reported success on a corrupt CURRENT pointer: {stdout}",
            cmd.join(" ")
        );
        assert!(
            !stdout.contains("\"total\": 0") && !stdout.contains("No decision artifacts"),
            "`forgeplan {}` degraded a read failure into an empty report: {stdout}",
            cmd.join(" ")
        );
        assert!(
            !stderr.is_empty(),
            "`forgeplan {}` failed silently: {cmd:?}",
            cmd.join(" ")
        );
    }
}

// ── session/discover surfaces touch no artifact index ──────────────────────

#[test]
fn discover_sessions_start_list_and_complete_on_a_migrated_workspace() {
    let migrated = init_migrated();

    run_ok(migrated.path(), &["discover", "start", "demo"]);
    let listed = run_ok(migrated.path(), &["discover", "list"]);
    assert!(
        listed.contains("demo"),
        "the started session must be listed, got: {listed}"
    );
    assert_no_retired_tables(&migrated.path());

    // `show` / `complete` need the session id the start printed.
    let id = listed
        .lines()
        .find_map(|l| {
            let trimmed = l.trim();
            trimmed
                .split_whitespace()
                .next()
                .filter(|tok| tok.starts_with("disc-"))
                .map(str::to_string)
        })
        .unwrap_or_else(|| panic!("no session id in: {listed}"));

    let shown = run_ok(migrated.path(), &["discover", "show", &id]);
    assert!(shown.contains("demo"), "got: {shown}");
    run_ok(migrated.path(), &["discover", "complete", &id]);
    assert_no_retired_tables(&migrated.path());
}

// ── scan-import on the migrated layout ─────────────────────────────────────

#[test]
fn a_dry_run_takes_no_lock_and_opens_no_store_on_a_legacy_workspace() {
    // The strongest form of "dry-run changes nothing": on a LEGACY layout the
    // mutation opener would run `LanceStore::open`, which migrates schema.
    // Removing `.lock` first makes a lock-taking preview observable as a new
    // file.
    let legacy = init_legacy();
    std::fs::create_dir_all(legacy.path().join("docs")).unwrap();
    std::fs::write(
        legacy.path().join("docs/ADR-060-preview.md"),
        "---\nkind: adr\nid: ADR-060\ntitle: Preview\nstatus: accepted\n---\n# Preview\n",
    )
    .unwrap();
    let lock_file = ws_of(legacy.path()).join(".lock");
    let _ = std::fs::remove_file(&lock_file);

    let stdout = run_ok(legacy.path(), &["scan-import", "--dry-run"]);
    assert!(stdout.contains("ADR-060"), "got: {stdout}");
    assert!(
        !lock_file.exists(),
        "the dry run acquired the workspace lock and created .lock"
    );
}

#[test]
fn scan_import_dry_run_changes_nothing_on_a_migrated_workspace() {
    let migrated = init_migrated();
    let source = migrated.path().join("docs/external.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let original = "---\nkind: adr\nid: ADR-050\ntitle: External doc\nstatus: accepted\n\
                     author: Ada\nreview_owner: platform-team\n\
                     links:\n  - target: PRD-001\n    relation: informed_by\n\
                     ---\n## Decision\n\nUse the capture.\n";
    std::fs::write(&source, original).unwrap();
    // A dry run must not even create the lock file on this layout.
    let lock_file = ws_of(migrated.path()).join(".lock");
    let _ = std::fs::remove_file(&lock_file);

    let before = inventory(&ws_of(migrated.path()));
    let stdout = run_ok(migrated.path(), &["scan-import", "--dry-run"]);
    assert!(
        stdout.contains("ADR-050"),
        "dry-run must preview the document it would import, got: {stdout}"
    );
    assert_eq!(
        inventory(&ws_of(migrated.path())),
        before,
        "dry-run created something in the workspace"
    );
    assert!(
        !lock_file.exists(),
        "the dry run acquired the workspace lock and created .lock"
    );
    assert_eq!(
        std::fs::read_to_string(&source).unwrap(),
        original,
        "dry-run rewrote its input"
    );
    assert_no_retired_tables(&migrated.path());
}

#[test]
fn scan_import_apply_then_repeat_is_idempotent_and_preserves_the_document() {
    let migrated = init_migrated();
    seed_artifacts(migrated.path());
    let source = migrated.path().join("docs/external.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let original = "---\nkind: adr\nid: ADR-050\ntitle: External doc\nstatus: accepted\n\
                     author: Ada\nreview_owner: platform-team\n\
                     links:\n  - target: PRD-001\n    relation: informed_by\n\
                     ---\n## Decision\n\nUse the capture.\n";
    std::fs::write(&source, original).unwrap();

    let first = run_ok(migrated.path(), &["scan-import"]);
    assert!(
        first.contains("1 imported"),
        "the external document must be imported, got: {first}"
    );

    let imported = ws_of(migrated.path()).join("adrs");
    let file = inventory(&imported)
        .into_iter()
        .find(|n| n.starts_with("ADR-050-"))
        .map(|n| imported.join(n))
        .expect("ADR-050 canonical file");
    let content = std::fs::read_to_string(&file).unwrap();
    assert!(
        content.contains("review_owner: platform-team"),
        "unknown frontmatter must survive the import: {content}"
    );
    assert!(content.contains("author: Ada"), "author lost: {content}");
    assert!(content.contains("status: active"), "status lost: {content}");
    assert!(
        content.contains("informed_by"),
        "typed relation lost: {content}"
    );
    assert!(content.contains("Use the capture."), "body lost: {content}");

    // The imported artifact is visible to the ordinary readers.
    let health = run_json(&migrated.path(), &["health", "--json"]);
    assert_eq!(
        health["total"].as_u64(),
        Some(4),
        "the imported ADR must be counted: {health}"
    );

    // Repeat: nothing changes on either side.
    let second = run_ok(&migrated.path(), &["scan-import"]);
    assert!(
        second.contains("0 imported") && second.contains("1 skipped"),
        "a repeat run must skip, got: {second}"
    );
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        content,
        "the repeat run rewrote the imported artifact"
    );
    assert_eq!(
        std::fs::read_to_string(&source).unwrap(),
        original,
        "the repeat run touched the external source"
    );
    assert_no_retired_tables(&migrated.path());
}

#[test]
fn init_scan_imports_on_a_migrated_workspace_without_retired_tables() {
    let migrated = init_migrated();
    seed_artifacts(migrated.path());
    std::fs::create_dir_all(migrated.path().join("docs")).unwrap();
    std::fs::write(
        migrated.path().join("docs/brownfield.md"),
        "---\nkind: problem\nid: PROB-070\ntitle: Brownfield\nstatus: open\n---\n# Brownfield\n",
    )
    .unwrap();

    let (ok, stdout, stderr) = run(migrated.path(), &["init", "-y", "--force", "--scan"]);
    assert!(ok, "init --force --scan failed on a migrated workspace: {stderr}");
    assert!(
        !stdout.contains("Skipped: scan-import needs the legacy store"),
        "init --scan must not report the retired skip any more, got: {stdout}"
    );
    let problems = ws_of(migrated.path()).join("problems");
    assert!(
        inventory(&problems)
            .iter()
            .any(|n| n.starts_with("PROB-070-")),
        "init --scan must actually import the document, got {problems:?}"
    );
    assert_no_retired_tables(&migrated.path());
}
