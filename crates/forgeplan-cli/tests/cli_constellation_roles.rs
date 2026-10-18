//! E2E for the required/optional source-role contract on
//! `forgeplan constellation` reads.
//!
//! The contract under test: a `required` source that cannot answer aborts
//! a strict query with `source_unavailable` — no envelope that would read
//! as complete — while an `optional` source, or any source under
//! `--allow-partial`, lands in `failures` and the answer prints `partial`.
//!
//! HOME is pinned to a tempdir so the per-user registry
//! (`~/.forgeplan/constellations.yaml`) is hermetic, matching
//! `cli_constellation_rebind.rs`.

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

/// Every live-read surface shares the role contract: a dead required
/// source is a strict error on each of them, and `--allow-partial`
/// turns each into a partial answer instead.
#[test]
fn a_dead_required_source_fails_strict_reads_and_allow_partial_recovers() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_a = tmp.path().join("ws-a");
    let ws_b = tmp.path().join("ws-b");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_a).unwrap();
    std::fs::create_dir_all(&ws_b).unwrap();

    init_workspace(&ws_a, &home);
    new_prd(&ws_a, &home, "Auth");
    init_workspace(&ws_b, &home);

    let forgeplan_b = ws_b.join(".forgeplan");
    constellation(&home, &["create", "crew"]).success();
    constellation(&home, &["add", &forgeplan_b.display().to_string()]).success();
    constellation(
        &home,
        &["add", &ws_a.join(".forgeplan").display().to_string()],
    )
    .success();

    // The second source dies after registration — the whole point of the
    // role contract is that the registry does not know yet.
    std::fs::remove_dir_all(&ws_b).unwrap();

    // Strict reads refuse outright on every surface — a partial envelope
    // over a dead required source would read as a complete answer.
    for args in [
        vec!["search", "auth"],
        vec!["get", "PRD-001"],
        vec!["pack", "auth storage"],
        vec!["graph"],
        vec!["impact", "PRD-001"],
        vec!["export"],
    ] {
        constellation(&home, &args)
            .failure()
            .stderr(predicate::str::contains("source_unavailable"));
    }

    // `--allow-partial` is the caller opting into a partial answer: the
    // live source still responds and the dead one is a named failure.
    constellation(&home, &["search", "auth", "--allow-partial"])
        .success()
        .stdout(predicate::str::contains("Auth"))
        .stdout(predicate::str::contains("partial"))
        .stderr(predicate::str::contains("unread"));
    constellation(&home, &["get", "PRD-001", "--allow-partial"])
        .success()
        .stdout(predicate::str::contains("Auth"));
    constellation(&home, &["graph", "--allow-partial"])
        .success()
        .stdout(predicate::str::contains("partial"));
    constellation(&home, &["pack", "auth storage", "--allow-partial"])
        .success()
        .stdout(predicate::str::contains("receipt:"));
    constellation(&home, &["impact", "PRD-001", "--allow-partial"])
        .success()
        .stderr(predicate::str::contains("unread"));
    constellation(&home, &["export", "--allow-partial"])
        .success()
        .stdout(predicate::str::contains("\"failures\""));
}

/// An optional source's outage degrades to `partial` — the strict query
/// still answers from the sources that responded.
#[test]
fn an_optional_source_outage_is_partial_not_fatal() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_a = tmp.path().join("ws-a");
    let ws_b = tmp.path().join("ws-b");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_a).unwrap();
    std::fs::create_dir_all(&ws_b).unwrap();

    init_workspace(&ws_a, &home);
    new_prd(&ws_a, &home, "Auth");
    init_workspace(&ws_b, &home);

    let forgeplan_b = ws_b.join(".forgeplan");
    constellation(&home, &["create", "crew"]).success();
    constellation(
        &home,
        &["add", &ws_a.join(".forgeplan").display().to_string()],
    )
    .success();
    constellation(
        &home,
        &["add", &forgeplan_b.display().to_string(), "--optional"],
    )
    .success()
    .stdout(predicate::str::contains("optional"));

    // The role persists in the registry file — back to YAML on disk.
    let registry_yaml = home.join(".forgeplan").join("constellations.yaml");
    let body = std::fs::read_to_string(&registry_yaml).unwrap();
    assert!(body.contains("role: optional"), "{body}");

    std::fs::remove_dir_all(&ws_b).unwrap();

    constellation(&home, &["search", "auth"])
        .success()
        .stdout(predicate::str::contains("Auth"))
        .stdout(predicate::str::contains("partial"))
        .stderr(predicate::str::contains("unread"));
    constellation(&home, &["get", "PRD-001"])
        .success()
        .stdout(predicate::str::contains("Auth"));
    constellation(&home, &["graph"])
        .success()
        .stdout(predicate::str::contains("partial"));
}

/// `sources` and `doctor` print the registered role — a dead required
/// source reads as the outage it is, not a bearable degradation.
#[test]
fn sources_and_doctor_show_the_role() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_a = tmp.path().join("ws-a");
    let ws_b = tmp.path().join("ws-b");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_a).unwrap();
    std::fs::create_dir_all(&ws_b).unwrap();

    init_workspace(&ws_a, &home);
    init_workspace(&ws_b, &home);

    constellation(&home, &["create", "crew"]).success();
    constellation(
        &home,
        &["add", &ws_a.join(".forgeplan").display().to_string()],
    )
    .success()
    .stdout(predicate::str::contains("required"));
    constellation(
        &home,
        &[
            "add",
            &ws_b.join(".forgeplan").display().to_string(),
            "--optional",
        ],
    )
    .success();

    constellation(&home, &["sources"])
        .success()
        .stdout(predicate::str::contains("required"))
        .stdout(predicate::str::contains("optional"))
        .stdout(predicate::str::contains("ws-a"))
        .stdout(predicate::str::contains("ws-b"));

    std::fs::remove_dir_all(&ws_b).unwrap();
    constellation(&home, &["doctor"])
        .success()
        .stdout(predicate::str::contains("broken"))
        .stdout(predicate::str::contains("optional"))
        .stdout(predicate::str::contains("required"))
        .stdout(predicate::str::contains("1 source(s) need attention"));
}
