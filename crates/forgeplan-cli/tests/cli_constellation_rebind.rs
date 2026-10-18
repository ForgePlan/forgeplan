//! E2E for `forgeplan constellation rebind`.
//!
//! The contract under test: a moved checkout is a new source identity, so
//! moving requires an explicit rebind — the registry path is rewritten and,
//! by default, the index's path-bound marker is too. Before the rebind,
//! `forgeplan index` at the new location must refuse with
//! `index_identity_mismatch`; after it, the pass runs and `constellation
//! get` answers again.
//!
//! HOME is pinned to a tempdir so the per-user registry
//! (`~/.forgeplan/constellations.yaml`) is hermetic, matching the pattern
//! in `cli_uncovered_coverage.rs`.

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Pin HOME (and the Windows/XDG twins) so `dirs::home_dir()` — and with
/// it the constellation registry — is the test's own directory.
fn home_env<'a>(cmd: &'a mut Command, home: &Path) -> &'a mut Command {
    cmd.env("HOME", home)
        .env("USERPROFILE", home)
        .env("XDG_DATA_HOME", home)
}

fn init_workspace(root: &Path, home: &Path) {
    home_env(forgeplan().args(["init", "-y"]), home)
        .current_dir(root)
        .assert()
        .success();
}

fn new_prd(root: &Path, home: &Path, title: &str) {
    home_env(forgeplan().args(["new", "prd", title]), home)
        .current_dir(root)
        .assert()
        .success();
}

fn constellation(home: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut full = vec!["constellation"];
    full.extend_from_slice(args);
    home_env(forgeplan().args(&full), home).assert()
}

/// A moved checkout: registry rebound, marker refused → rewritten →
/// `forgeplan index` answers again, `constellation get` keeps working, and
/// the old `src:` qualifier fails loudly.
#[test]
fn a_moved_checkout_must_be_rebound_before_index_answers() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_a = tmp.path().join("ws-a");
    let ws_b = tmp.path().join("ws-b");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_a).unwrap();

    init_workspace(&ws_a, &home);
    new_prd(&ws_a, &home, "Auth");
    // Build a real index so there is a marker to rebind.
    home_env(forgeplan().args(["index"]), &home)
        .current_dir(&ws_a)
        .assert()
        .success();

    let forgeplan_a = ws_a.join(".forgeplan");
    let forgeplan_b = ws_b.join(".forgeplan");
    constellation(&home, &["create", "crew"]).success();
    constellation(&home, &["add", &forgeplan_a.display().to_string()]).success();
    constellation(&home, &["doctor"])
        .success()
        .stdout(predicate::str::contains("indexed"));
    constellation(&home, &["get", "PRD-001"])
        .success()
        .stdout(predicate::str::contains("Auth"));

    // The checkout moves.
    std::fs::rename(&ws_a, &ws_b).unwrap();

    // The registry still points at the old path.
    constellation(&home, &["doctor"])
        .success()
        .stdout(predicate::str::contains("broken"));
    // And the index at the new location refuses: its marker still claims
    // the old path's identity. This is the loud failure rebind exists for.
    home_env(forgeplan().args(["index"]), &home)
        .current_dir(&ws_b)
        .assert()
        .failure()
        .stderr(predicate::str::contains("index_identity_mismatch"));

    // Rebind by a fragment of the old stored path.
    let assert = constellation(
        &home,
        &["rebind", "ws-a", &forgeplan_b.display().to_string()],
    )
    .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains("Rebound in 'crew'"), "{out}");
    assert!(out.contains("source id: src:"), "{out}");
    assert!(out.contains("checkout"), "{out}");

    // The registry reports the new path as a distinct source id.
    let old_id = out
        .split("source id: ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .expect("the rebind output reports the old source id");
    assert!(old_id.starts_with("src:"), "{old_id}");

    // Doctor healthy again, `forgeplan index` works at the new location,
    // and the artifact answers as before.
    constellation(&home, &["doctor"])
        .success()
        .stdout(predicate::str::contains("indexed"));
    home_env(forgeplan().args(["index"]), &home)
        .current_dir(&ws_b)
        .assert()
        .success();
    constellation(&home, &["get", "PRD-001"])
        .success()
        .stdout(predicate::str::contains("Auth"));

    // The old source identity is gone: refs qualified by it fail loudly
    // via the qualifier-miss path, not silently wider.
    constellation(&home, &["get", &format!("PRD-001@{old_id}")])
        .failure()
        .stderr(predicate::str::contains("matches no source"));
}

/// A workspace that was never indexed has no marker: the registry rebind
/// still applies, the marker step reports there was nothing to rewrite,
/// and reads fall back to markdown as before.
#[test]
fn rebind_without_an_index_leaves_the_marker_alone() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_a = tmp.path().join("ws-a");
    let ws_b = tmp.path().join("ws-b");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_a).unwrap();

    init_workspace(&ws_a, &home);
    new_prd(&ws_a, &home, "Auth");
    let forgeplan_a = ws_a.join(".forgeplan");
    let forgeplan_b = ws_b.join(".forgeplan");
    constellation(&home, &["create", "crew"]).success();
    constellation(&home, &["add", &forgeplan_a.display().to_string()]).success();

    std::fs::rename(&ws_a, &ws_b).unwrap();

    constellation(
        &home,
        &["rebind", "ws-a", &forgeplan_b.display().to_string()],
    )
    .success()
    .stdout(predicate::str::contains("nothing to rewrite"));

    constellation(&home, &["doctor"])
        .success()
        .stdout(predicate::str::contains("markdown"));
    constellation(&home, &["get", "PRD-001"])
        .success()
        .stdout(predicate::str::contains("Auth"));
}

/// An index at the rebind target that claims a *third* checkout is a
/// foreign index: refused without `--force`, claimed with it.
#[test]
fn rebind_refuses_a_foreign_index_unless_forced() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_a = tmp.path().join("ws-a");
    let ws_c = tmp.path().join("ws-c");
    let ws_d = tmp.path().join("ws-d");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_a).unwrap();
    std::fs::create_dir_all(&ws_c).unwrap();

    // ws-a: registered, never indexed. It will be the moved checkout.
    init_workspace(&ws_a, &home);
    new_prd(&ws_a, &home, "Auth");
    // ws-c: a different checkout with its own index — its marker will be
    // the foreign claim at the rebind target.
    init_workspace(&ws_c, &home);
    new_prd(&ws_c, &home, "Other");
    home_env(forgeplan().args(["index"]), &home)
        .current_dir(&ws_c)
        .assert()
        .success();

    let forgeplan_a = ws_a.join(".forgeplan");
    let forgeplan_d = ws_d.join(".forgeplan");
    constellation(&home, &["create", "crew"]).success();
    constellation(&home, &["add", &forgeplan_a.display().to_string()]).success();

    // ws-a moves away (registry still points at it); ws-c's whole checkout
    // — index and all — sits at ws-d.
    std::fs::rename(&ws_a, tmp.path().join("ws-a-moved")).unwrap();
    std::fs::rename(&ws_c, &ws_d).unwrap();

    // The marker at ws-d claims ws-c's checkout: neither the moved one's
    // nor ws-d's own. Refused — and the registry file stays untouched.
    constellation(
        &home,
        &["rebind", "ws-a", &forgeplan_d.display().to_string()],
    )
    .failure()
    .stderr(predicate::str::contains("foreign index"))
    .stderr(predicate::str::contains("--force"));
    constellation(&home, &["sources"])
        .success()
        .stdout(predicate::str::contains(forgeplan_a.display().to_string()));

    // With --force the operator claims the index: marker rewritten to
    // ws-d's bound id.
    constellation(
        &home,
        &[
            "rebind",
            "ws-a",
            "--force",
            &forgeplan_d.display().to_string(),
        ],
    )
    .success()
    .stdout(predicate::str::contains("checkout"));
    constellation(&home, &["doctor"])
        .success()
        .stdout(predicate::str::contains("indexed"));
    home_env(forgeplan().args(["index"]), &home)
        .current_dir(&ws_d)
        .assert()
        .success();
}

/// `--skip-index-marker` rewrites only the registry entry; the index's
/// identity is left for a later pass.
#[test]
fn skip_index_marker_rebinds_the_registry_only() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_a = tmp.path().join("ws-a");
    let ws_b = tmp.path().join("ws-b");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_a).unwrap();

    init_workspace(&ws_a, &home);
    new_prd(&ws_a, &home, "Auth");
    home_env(forgeplan().args(["index"]), &home)
        .current_dir(&ws_a)
        .assert()
        .success();
    let forgeplan_a = ws_a.join(".forgeplan");
    let forgeplan_b = ws_b.join(".forgeplan");
    constellation(&home, &["create", "crew"]).success();
    constellation(&home, &["add", &forgeplan_a.display().to_string()]).success();

    std::fs::rename(&ws_a, &ws_b).unwrap();

    constellation(
        &home,
        &[
            "rebind",
            "ws-a",
            "--skip-index-marker",
            &forgeplan_b.display().to_string(),
        ],
    )
    .success()
    .stdout(predicate::str::contains("left untouched"));

    // Registry rebound — doctor sees the workspace again — but the marker
    // still claims the old path, so maintenance still refuses it.
    constellation(&home, &["doctor"])
        .success()
        .stdout(predicate::str::contains("indexed"));
    home_env(forgeplan().args(["index"]), &home)
        .current_dir(&ws_b)
        .assert()
        .failure()
        .stderr(predicate::str::contains("index_identity_mismatch"));

    // A second rebind finishes the job. The stored path is ws-b now, so the
    // marker still claiming ws-a reads as foreign to this call — the
    // operator asserts the claim with --force, which is exactly what the
    // flag exists for.
    constellation(
        &home,
        &[
            "rebind",
            "ws-b",
            "--force",
            &forgeplan_b.display().to_string(),
        ],
    )
    .success();
    home_env(forgeplan().args(["index"]), &home)
        .current_dir(&ws_b)
        .assert()
        .success();
}
