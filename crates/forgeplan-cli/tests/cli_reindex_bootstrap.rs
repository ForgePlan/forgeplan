//! `forgeplan reindex` on a workspace whose `.forgeplan/lance/` is absent must
//! bootstrap the identity-v5 index from canonical Markdown instead of demanding
//! a pre-existing legacy `artifacts.lance` table. (Issue #564)

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

/// Remove the derived index while keeping the canonical Markdown source.
fn drop_lance(tmp: &TempDir) {
    let lance = tmp.path().join(".forgeplan").join("lance");
    if lance.exists() {
        std::fs::remove_dir_all(&lance).unwrap();
    }
    assert!(
        !lance.exists(),
        "lance directory must be absent after drop_lance"
    );
}

/// `reindex` on a markdown-only workspace must succeed by bootstrapping a
/// fresh identity-v5 index — not fail demanding `artifacts.lance`.
#[test]
fn reindex_bootstraps_v5_on_clean_checkout() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd = new_artifact(&tmp, "prd", "Bootstrap check");

    drop_lance(&tmp);
    assert!(
        !tmp.path().join(".forgeplan/lance/artifacts.lance").exists(),
        "legacy artifacts.lance must be absent before reindex"
    );

    let out = forgeplan()
        .args(["reindex"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "reindex must succeed on a markdown-only workspace; got:\n{combined}"
    );
    assert!(
        combined.contains("Identity-v5 index reconciled"),
        "expected the identity-v5 reconcile marker; got:\n{combined}"
    );

    // The bootstrap must publish the v5 marker.
    let marker = tmp.path().join(".forgeplan/lance/IDENTITY_V5.json");
    assert!(
        marker.exists(),
        "IDENTITY_V5.json marker must exist after bootstrap reindex"
    );

    // The artifact must be reachable through the rebuilt index.
    let get = forgeplan()
        .args(["get", &prd])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        get.status.success(),
        "get must find the artifact after bootstrap reindex; got:\n{}",
        String::from_utf8_lossy(&get.stderr)
    );
}

/// `reindex` on a completely empty workspace (no artifacts) must still
/// bootstrap — zero documents is a valid source, not an error.
#[test]
fn reindex_bootstraps_on_empty_workspace() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    drop_lance(&tmp);

    let out = forgeplan()
        .args(["reindex"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "reindex must succeed on an empty workspace; got:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
