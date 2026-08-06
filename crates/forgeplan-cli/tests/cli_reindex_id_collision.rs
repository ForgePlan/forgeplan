//! #394 regression — reindex MUST surface duplicate-id collisions instead of
//! silently overwriting (last-writer-wins). Two `.md` files carrying the same
//! frontmatter `id` both resolve to one LanceDB row; before this fix the file
//! processed second silently shadowed the first with no anomaly reported (and
//! `read_dir` order is not even stable, so *which* one won was arbitrary).
//! This drives the REAL binary against a real workspace with a hand-crafted
//! collision — the exact shape of the PRD-012 / EVID-143 dogfood collisions.

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

fn find_md(dir: &std::path::Path, id: &str) -> std::path::PathBuf {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(id) && name.ends_with(".md") {
            return entry.path();
        }
    }
    panic!("no .md file for {id} in {}", dir.display());
}

/// Main trace: two files, same `id`, different filenames → reindex must report
/// the collision LOUDLY, name both files, count it in the summary, and still
/// exit 0 (non-fatal, so a pre-existing collision does not brick an onboarding
/// `git clone && forgeplan reindex`).
#[test]
fn reindex_reports_duplicate_id_collision() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let prd = new_artifact(&tmp, "prd", "Onboarding init scan");
    let prds_dir = tmp.path().join(".forgeplan").join("prds");

    // Hand-craft the collision: copy the canonical file to a second filename
    // (different slug) that keeps the SAME frontmatter `id`. Append a body line
    // so the content genuinely differs — exercising the silent-overwrite path,
    // not an identical-file no-op.
    let original = find_md(&prds_dir, &prd);
    let content = std::fs::read_to_string(&original).unwrap();
    let dup = prds_dir.join(format!("{prd}-onboarding-shadow-copy.md"));
    std::fs::write(
        &dup,
        format!("{content}\n\nShadow copy body — #394 collision.\n"),
    )
    .unwrap();

    let out = forgeplan()
        .args(["reindex"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    // Non-fatal: a collision must NOT brick reindex (onboarding safety).
    assert!(
        out.status.success(),
        "reindex must stay non-fatal on a collision: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // The collision must be LOUD, name the id, and list BOTH files.
    assert!(
        combined.contains("duplicate-id collision"),
        "reindex must announce the collision; got:\n{combined}"
    );
    assert!(
        combined.contains(&prd),
        "collision report must name the colliding id {prd}; got:\n{combined}"
    );
    let orig_name = original.file_name().unwrap().to_string_lossy().to_string();
    assert!(
        combined.contains(&orig_name),
        "collision report must list the original file {orig_name}; got:\n{combined}"
    );
    assert!(
        combined.contains("onboarding-shadow-copy.md"),
        "collision report must list the shadow file; got:\n{combined}"
    );
    // Summary must COUNT it — a block alone can be scrolled past; the count is
    // the machine-readable signal an agent/CI greps.
    assert!(
        combined.contains("1 id-collisions"),
        "summary must count id-collisions; got:\n{combined}"
    );
}

/// Negative control: a clean workspace reindex reports zero collisions and
/// prints no collision block. Guards against a false-positive that would fire
/// on every ordinary single-file artifact.
#[test]
fn reindex_reports_zero_collisions_on_clean_workspace() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "prd", "Solo artifact");

    let out = forgeplan()
        .args(["reindex"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("0 id-collisions"),
        "clean workspace must report 0 id-collisions; got:\n{combined}"
    );
    assert!(
        !combined.contains("duplicate-id collision"),
        "clean workspace must NOT print a collision block; got:\n{combined}"
    );
}
