//! E2E for `forgeplan link KEY TARGET@qualifier` — stage 4 qualified
//! cross-repo link targets.
//!
//! The contract under test: a `KEY@qualifier` target names an artifact in
//! a *different* workspace. The write lands in the **source** artifact's
//! file only (`links:` frontmatter + the local relations table) through
//! the normal projection flow — the foreign workspace is never opened,
//! never rendered, never mutated. HOME is pinned so the per-user
//! constellation registry (`~/.forgeplan/constellations.yaml`) is
//! hermetic, matching `cli_constellation_rebind.rs`.

use std::path::{Path, PathBuf};

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

fn new_artifact(root: &Path, home: &Path, kind: &str, title: &str) {
    home_env(forgeplan().args(["new", kind, title]), home)
        .current_dir(root)
        .assert()
        .success();
}

fn constellation(home: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut full = vec!["constellation"];
    full.extend_from_slice(args);
    home_env(forgeplan().args(&full), home).assert()
}

fn link(root: &Path, home: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut full = vec!["link"];
    full.extend_from_slice(args);
    home_env(forgeplan().args(&full), home)
        .current_dir(root)
        .assert()
}

/// Every file under `root` as `(relative path, mtime, byte length,
/// contents)` — the complete "did the foreign workspace change" answer:
/// file set, mtimes, and bytes all compared at once. Recursive because
/// `walkdir` is not a CLI dev-dependency and adding one for a 20-line
/// walker is worse than the walker.
fn tree_snapshot(root: &Path) -> Vec<(PathBuf, std::time::SystemTime, u64, Vec<u8>)> {
    fn walk(
        dir: &Path,
        base: &Path,
        out: &mut Vec<(PathBuf, std::time::SystemTime, u64, Vec<u8>)>,
    ) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                walk(&path, base, out);
            } else {
                let meta = entry.metadata().unwrap();
                out.push((
                    path.strip_prefix(base).unwrap().to_path_buf(),
                    meta.modified().unwrap(),
                    meta.len(),
                    std::fs::read(&path).unwrap(),
                ));
            }
        }
    }
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

fn get_json(root: &Path, home: &Path, id: &str) -> serde_json::Value {
    let out = home_env(forgeplan().args(["get", id, "--json"]), home)
        .current_dir(root)
        .output()
        .expect("get --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("get --json not JSON ({e}):\n{stdout}"))
}

/// The one `.md` file under `<root>/.forgeplan/<dir>/` whose name starts
/// with `<id>-`.
fn projection_file(root: &Path, dir: &str, id: &str) -> PathBuf {
    std::fs::read_dir(root.join(".forgeplan").join(dir))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().starts_with(&format!("{id}-")))
                .unwrap_or(false)
        })
        .unwrap_or_else(|| panic!("no projection file for {id} under {dir}"))
}

/// The happy path: a qualifier that *does* name a registered source is
/// stored verbatim in the source file — and the foreign workspace is
/// byte-identical afterwards.
#[test]
fn link_stores_a_qualified_target_in_the_source_workspace_only() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_a = tmp.path().join("ws-a");
    let ws_b = tmp.path().join("ws-b");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_a).unwrap();
    std::fs::create_dir_all(&ws_b).unwrap();

    init_workspace(&ws_a, &home);
    init_workspace(&ws_b, &home);
    new_artifact(&ws_a, &home, "prd", "Local plan");
    new_artifact(&ws_b, &home, "adr", "Foreign decision");

    // ws-b joins the active constellation; `ws-b` is itself a valid
    // path-fragment qualifier.
    let forgeplan_b = ws_b.join(".forgeplan");
    constellation(&home, &["create", "crew"]).success();
    constellation(&home, &["add", &forgeplan_b.display().to_string()]).success();

    let snapshot_before = tree_snapshot(&ws_b);

    // The qualified link writes from ws-a — a registered-source qualifier.
    link(
        &ws_a,
        &home,
        &["PRD-001", "ADR-001@ws-b", "--relation", "based_on"],
    )
    .success()
    .stdout(predicate::str::contains(
        "Linked: PRD-001 --based_on--> ADR-001@ws-b",
    ));

    // `get` reports the qualified target verbatim.
    let v = get_json(&ws_a, &home, "PRD-001");
    let outbound = v["links"]["outbound"].as_array().expect("outbound array");
    assert_eq!(outbound.len(), 1, "expected one outbound edge: {v}");
    assert_eq!(outbound[0]["target"], serde_json::json!("ADR-001@ws-b"));
    assert_eq!(outbound[0]["relation"], serde_json::json!("based_on"));

    // The durable record is the source file's `links:` block (ADR-003).
    let content = std::fs::read_to_string(projection_file(&ws_a, "prds", "PRD-001")).unwrap();
    assert!(
        content.contains("target: ADR-001@ws-b"),
        "the source file must carry the qualified ref verbatim:\n{content}"
    );

    // The foreign workspace was never opened for writing: file set,
    // mtimes and bytes all identical.
    assert_eq!(
        snapshot_before,
        tree_snapshot(&ws_b),
        "a qualified link must not touch the foreign workspace"
    );
}

/// A qualifier matching nothing registered still links — forward refs are
/// legal — but the stderr warning says the qualifier named no source, so
/// a typo cannot masquerade as a deliberate cross-repo reference.
#[test]
fn link_to_an_unmatched_qualifier_warns_but_still_writes() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_a = tmp.path().join("ws-a");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_a).unwrap();

    init_workspace(&ws_a, &home);
    new_artifact(&ws_a, &home, "prd", "Local plan");
    constellation(&home, &["create", "crew"]).success();

    link(
        &ws_a,
        &home,
        &["PRD-001", "ADR-001@src:beef", "--relation", "informs"],
    )
    .success()
    .stderr(predicate::str::contains("matches no registered source"));

    let v = get_json(&ws_a, &home, "PRD-001");
    let outbound = v["links"]["outbound"].as_array().expect("outbound array");
    assert_eq!(outbound[0]["target"], serde_json::json!("ADR-001@src:beef"));
}

/// `src:`-style qualifiers match against the live source id: a prefix of
/// the registered source's `src:` id must *not* warn, and the ref stores
/// verbatim.
#[test]
fn link_accepts_a_src_id_prefix_qualifier_without_warning() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_a = tmp.path().join("ws-a");
    let ws_b = tmp.path().join("ws-b");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_a).unwrap();
    std::fs::create_dir_all(&ws_b).unwrap();

    init_workspace(&ws_a, &home);
    init_workspace(&ws_b, &home);
    new_artifact(&ws_a, &home, "prd", "Local plan");
    new_artifact(&ws_b, &home, "adr", "Foreign decision");

    let forgeplan_b = ws_b.join(".forgeplan");
    constellation(&home, &["create", "crew"]).success();
    constellation(&home, &["add", &forgeplan_b.display().to_string()]).success();

    // `constellation get` prints the answering source's `src:` id
    // (`... [<channel>:src:xxxx]`); parse it rather than recomputing the
    // path hash in the test.
    let out = constellation(&home, &["get", "ADR-001"]).success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let start = stdout
        .find("src:")
        .unwrap_or_else(|| panic!("no `src:` id in `constellation get` output:\n{stdout}"));
    let src_id: String = stdout[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == ':')
        .collect();
    assert!(src_id.len() > "src:".len() + 8, "{src_id}");

    // A strict prefix of the live `src:` id still qualifies.
    let qualifier = &src_id[..src_id.len() - 4];
    let target = format!("ADR-001@{qualifier}");
    link(
        &ws_a,
        &home,
        &["PRD-001", &target, "--relation", "based_on"],
    )
    .success()
    .stderr(
        predicate::str::contains("matches no registered source")
            .not()
            .and(predicate::str::contains("no active constellation").not()),
    );

    let v = get_json(&ws_a, &home, "PRD-001");
    let outbound = v["links"]["outbound"].as_array().expect("outbound array");
    assert_eq!(outbound[0]["target"], serde_json::json!(target));
}

/// `unlink` accepts what `link` stored — the qualified edge deletes by
/// verbatim string equality, source-side only.
#[test]
fn unlink_removes_a_qualified_edge() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_a = tmp.path().join("ws-a");
    let ws_b = tmp.path().join("ws-b");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_a).unwrap();
    std::fs::create_dir_all(&ws_b).unwrap();

    init_workspace(&ws_a, &home);
    init_workspace(&ws_b, &home);
    new_artifact(&ws_a, &home, "prd", "Local plan");
    let forgeplan_b = ws_b.join(".forgeplan");
    constellation(&home, &["create", "crew"]).success();
    constellation(&home, &["add", &forgeplan_b.display().to_string()]).success();

    link(
        &ws_a,
        &home,
        &["PRD-001", "ADR-001@ws-b", "--relation", "based_on"],
    )
    .success();
    home_env(
        forgeplan().args([
            "unlink",
            "PRD-001",
            "ADR-001@ws-b",
            "--relation",
            "based_on",
        ]),
        &home,
    )
    .current_dir(&ws_a)
    .assert()
    .success();

    let v = get_json(&ws_a, &home, "PRD-001");
    let outbound = v["links"]["outbound"].as_array().expect("outbound array");
    assert!(outbound.is_empty(), "the qualified edge must be gone: {v}");
}

/// A malformed qualifier is still an invalid id, not a weird link: the
/// qualifier charset is a filter-injection boundary.
#[test]
fn link_rejects_an_injectable_qualified_target() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_a = tmp.path().join("ws-a");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_a).unwrap();

    init_workspace(&ws_a, &home);
    new_artifact(&ws_a, &home, "prd", "Local plan");

    link(
        &ws_a,
        &home,
        &["PRD-001", "ADR-001@a/b", "--relation", "informs"],
    )
    .failure();

    let v = get_json(&ws_a, &home, "PRD-001");
    let outbound = v["links"]["outbound"].as_array().expect("outbound array");
    assert!(outbound.is_empty(), "no edge may be stored: {v}");
}

/// Reindex restores `links:` edges from files — a qualified edge must
/// survive the rebuild instead of being reaped as an orphan relation
/// (the target lives in a foreign workspace by definition, so Phase 3
/// skips it unless the source itself is gone).
#[test]
fn a_qualified_edge_survives_reindex() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_a = tmp.path().join("ws-a");
    let ws_b = tmp.path().join("ws-b");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_a).unwrap();
    std::fs::create_dir_all(&ws_b).unwrap();

    init_workspace(&ws_a, &home);
    init_workspace(&ws_b, &home);
    new_artifact(&ws_a, &home, "prd", "Local plan");
    let forgeplan_b = ws_b.join(".forgeplan");
    constellation(&home, &["create", "crew"]).success();
    constellation(&home, &["add", &forgeplan_b.display().to_string()]).success();

    link(
        &ws_a,
        &home,
        &["PRD-001", "ADR-001@ws-b", "--relation", "based_on"],
    )
    .success();

    // Reindex must neither print the edge as an orphan delete nor warn
    // about it — the target is foreign, not missing.
    home_env(forgeplan().args(["reindex"]), &home)
        .current_dir(&ws_a)
        .assert()
        .success()
        .stdout(predicate::str::contains("ADR-001@ws-b").not())
        .stderr(predicate::str::contains("ADR-001@ws-b").not());

    let v = get_json(&ws_a, &home, "PRD-001");
    let outbound = v["links"]["outbound"].as_array().expect("outbound array");
    assert_eq!(
        outbound[0]["target"],
        serde_json::json!("ADR-001@ws-b"),
        "the qualified edge must survive a full reindex: {v}"
    );
}
