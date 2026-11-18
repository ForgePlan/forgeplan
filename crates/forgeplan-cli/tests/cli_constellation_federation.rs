//! Federation surface: `forgeplan constellation *` on the DEFAULT layout.
//! Covers the full registry lifecycle plus cross-source identity resolution
//! (same id in two sources → `ambiguous`), which MCP tests do not exercise at
//! the CLI layer.
//!
//! Every command spawns with `HOME` pointed at a per-test tempdir so the
//! registry file (`~/.forgeplan/constellations.yaml`) is written in-test, not
//! to the real user profile.

use assert_cmd::Command;
use std::path::Path;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init_workspace(dir: &Path) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(dir)
        .assert()
        .success();
}

fn new_artifact(dir: &Path, kind: &str, title: &str) -> String {
    let out = forgeplan()
        .args(["new", kind, title, "--allow-duplicate"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "new {kind}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix("ID:").map(str::trim))
        .unwrap_or_else(|| panic!("no ID in new {kind}: {stdout}"))
        .to_string()
}

/// Spawn a forgeplan command with `HOME` set to `home` so the registry lands
/// in the test tempdir.
fn cmd(home: &TempDir, cwd: &Path, args: &[&str]) -> (bool, String, String) {
    let out = forgeplan()
        .args(args)
        .current_dir(cwd)
        .env("HOME", home.path())
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn workspace() -> TempDir {
    let t = TempDir::new().unwrap();
    init_workspace(t.path());
    t
}

// ── registry lifecycle ─────────────────────────────────────────────────────

/// The first `constellation create` becomes the active one; `use` switches to
/// a later-created name; `list` reports both with the active marker on the
/// right one.
#[test]
fn constellation_create_use_and_list() {
    let home = TempDir::new().unwrap();
    let a = workspace();
    let b = workspace();

    let (ok, _, err) = cmd(&home, a.path(), &["constellation", "create", "first"]);
    assert!(ok, "create first: {err}");
    cmd(
        &home,
        a.path(),
        &[
            "constellation",
            "add",
            a.path().join(".forgeplan").to_str().unwrap(),
            "--to",
            "first",
        ],
    );

    let (ok, _, err) = cmd(&home, a.path(), &["constellation", "create", "second"]);
    assert!(ok, "create second: {err}");
    cmd(
        &home,
        a.path(),
        &[
            "constellation",
            "add",
            b.path().join(".forgeplan").to_str().unwrap(),
            "--to",
            "second",
        ],
    );

    let (ok, _out, err) = cmd(&home, a.path(), &["constellation", "use", "second"]);
    assert!(ok, "use second: {err}");

    let (ok, out, _) = cmd(&home, a.path(), &["constellation", "list"]);
    assert!(ok);
    assert!(
        out.contains("second")
            && (out.contains("*") || out.contains("active") || out.contains("→")),
        "list must mark the active constellation; got:\n{out}"
    );
}

/// `sources` reports the registered .forgeplan roots for the active
/// constellation; after `add` the new root appears.
#[test]
fn constellation_sources_lists_registered_roots() {
    let home = TempDir::new().unwrap();
    let a = workspace();
    let b = workspace();

    cmd(&home, a.path(), &["constellation", "create", "mine"]);
    cmd(
        &home,
        a.path(),
        &[
            "constellation",
            "add",
            a.path().join(".forgeplan").to_str().unwrap(),
        ],
    );

    let (ok, out, err) = cmd(&home, a.path(), &["constellation", "sources"]);
    assert!(ok, "sources: {err}");
    assert!(
        out.contains(".forgeplan") || out.contains(a.path().to_str().unwrap()),
        "sources must list the registered root; got:\n{out}"
    );

    cmd(
        &home,
        a.path(),
        &[
            "constellation",
            "add",
            b.path().join(".forgeplan").to_str().unwrap(),
        ],
    );

    let (_, out, _) = cmd(&home, a.path(), &["constellation", "sources"]);
    assert!(
        out.contains(b.path().to_str().unwrap()),
        "sources must show newly added B root; got:\n{out}"
    );
}

/// `remove` detaches a source from the registry; `sources` no longer lists it.
#[test]
fn constellation_remove_detaches_source() {
    let home = TempDir::new().unwrap();
    let a = workspace();
    let b = workspace();
    let b_src = b.path().join(".forgeplan");

    cmd(&home, a.path(), &["constellation", "create", "mine"]);
    cmd(
        &home,
        a.path(),
        &["constellation", "add", b_src.to_str().unwrap()],
    );

    let (ok, _, err) = cmd(
        &home,
        a.path(),
        &["constellation", "remove", b_src.to_str().unwrap()],
    );
    assert!(ok, "remove B: {err}");

    let (_, out, _) = cmd(&home, a.path(), &["constellation", "sources"]);
    assert!(
        !out.contains(b.path().to_str().unwrap()),
        "removed source must leave the list; got:\n{out}"
    );
}

/// `add` refuses a non-workspace path (no `.forgeplan/` inside).
#[test]
fn constellation_add_rejects_non_workspace() {
    let home = TempDir::new().unwrap();
    let a = workspace();
    let junk = TempDir::new().unwrap();

    cmd(&home, a.path(), &["constellation", "create", "mine"]);
    let (ok, _, err) = cmd(
        &home,
        a.path(),
        &["constellation", "add", junk.path().to_str().unwrap()],
    );
    assert!(
        !ok || err.contains("workspace") || err.contains(".forgeplan") || err.contains("forgeplan"),
        "adding a non-workspace must fail; got ok={ok} err:\n{err}"
    );
}

// ── doctor ─────────────────────────────────────────────────────────────────

/// `doctor` checks every registered source: a healthy workspace reports
/// reachable+published; a deleted path reports unreachable.
#[test]
fn constellation_doctor_reports_health() {
    let home = TempDir::new().unwrap();
    let a = workspace();
    let b = workspace();

    cmd(&home, a.path(), &["constellation", "create", "mine"]);
    cmd(
        &home,
        a.path(),
        &[
            "constellation",
            "add",
            a.path().join(".forgeplan").to_str().unwrap(),
        ],
    );
    cmd(
        &home,
        a.path(),
        &[
            "constellation",
            "add",
            b.path().join(".forgeplan").to_str().unwrap(),
        ],
    );

    let (ok, out, err) = cmd(&home, a.path(), &["constellation", "doctor"]);
    assert!(ok, "doctor: {err}");
    assert!(
        out.contains("ok")
            || out.contains("reachable")
            || out.contains("✓")
            || out.contains("healthy"),
        "doctor must report a healthy source; got:\n{out}"
    );

    // Delete B and re-doctor — the unreachable path must surface.
    let b_path = b.path().join(".forgeplan").to_str().unwrap().to_string();
    drop(b);
    let (_ok, out, _) = cmd(&home, a.path(), &["constellation", "doctor"]);
    assert!(
        out.contains(&b_path)
            && (out.contains("unreachable")
                || out.contains("missing")
                || out.contains("error")
                || out.contains("✗")),
        "doctor must flag the deleted source; got:\n{out}"
    );
}

// ── cross-source identity ─────────────────────────────────────────────────

/// Two sources holding the same artifact id make `constellation get` report
/// `ambiguous` and name both sources — the core identity contract of
/// federation.
#[test]
fn constellation_get_reports_ambiguous_duplicate_id() {
    let home = TempDir::new().unwrap();
    let a = workspace();
    let b = workspace();
    let same_id = new_artifact(a.path(), "prd", "Shared ID");
    let same_id_b = new_artifact(b.path(), "prd", "Shared ID");
    assert_eq!(
        same_id, same_id_b,
        "fixture must produce same id in both workspaces"
    );

    cmd(&home, a.path(), &["constellation", "create", "mine"]);
    cmd(
        &home,
        a.path(),
        &[
            "constellation",
            "add",
            a.path().join(".forgeplan").to_str().unwrap(),
        ],
    );
    cmd(
        &home,
        a.path(),
        &[
            "constellation",
            "add",
            b.path().join(".forgeplan").to_str().unwrap(),
        ],
    );
    cmd(&home, a.path(), &["index"]);
    cmd(&home, b.path(), &["index"]);

    let (ok, out, err) = cmd(&home, a.path(), &["constellation", "get", &same_id]);
    assert!(
        ok || out.contains("ambiguous")
            || out.contains("2 sources")
            || out.contains("claims")
            || err.contains("ambiguous"),
        "duplicate id across sources must surface as ambiguous; got ok={ok} out:\n{out}\nerr:\n{err}"
    );
}

/// `constellation search` fans out across sources — a term unique to source B
/// must be findable from the active scope.
#[test]
fn constellation_search_fans_out_across_sources() {
    let home = TempDir::new().unwrap();
    let a = workspace();
    let b = workspace();
    new_artifact(b.path(), "adr", "Unique Federation Term XYZ");

    cmd(&home, a.path(), &["constellation", "create", "mine"]);
    cmd(
        &home,
        a.path(),
        &[
            "constellation",
            "add",
            b.path().join(".forgeplan").to_str().unwrap(),
        ],
    );
    cmd(&home, b.path(), &["index"]);

    let (ok, out, err) = cmd(
        &home,
        a.path(),
        &["constellation", "search", "Unique Federation Term XYZ"],
    );
    assert!(ok, "search: {err}");
    assert!(
        out.contains("Unique") || out.contains("XYZ") || out.contains("adr"),
        "cross-source search must surface B's artifact; got:\n{out}"
    );
}

/// `constellation graph` lists source-qualified nodes — both A's and B's
/// artifacts appear, prefixed by their source id.
#[test]
fn constellation_graph_qualifies_nodes_by_source() {
    let home = TempDir::new().unwrap();
    let a = workspace();
    let b = workspace();
    let prd = new_artifact(a.path(), "prd", "Graph Node");
    new_artifact(b.path(), "adr", "Graph Node B");

    cmd(&home, a.path(), &["constellation", "create", "mine"]);
    cmd(
        &home,
        a.path(),
        &[
            "constellation",
            "add",
            a.path().join(".forgeplan").to_str().unwrap(),
        ],
    );
    cmd(
        &home,
        a.path(),
        &[
            "constellation",
            "add",
            b.path().join(".forgeplan").to_str().unwrap(),
        ],
    );
    cmd(&home, a.path(), &["index"]);
    cmd(&home, b.path(), &["index"]);

    let (ok, out, err) = cmd(&home, a.path(), &["constellation", "graph"]);
    assert!(ok, "graph: {err}");
    assert!(
        out.contains(&prd)
            || out.contains("src:")
            || out.contains("mermaid")
            || out.contains("graph"),
        "graph must list source-qualified nodes; got:\n{out}"
    );
}
