//! Reindex must reject duplicate canonical keys without choosing a file.
//! A shared display id alone is not a collision.

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

/// The document under `dir` that declares `<id>`.
///
/// The file is named after the document's canonical key (ADR-030), so the
/// display id is read from the frontmatter the document declares.
fn find_md(dir: &std::path::Path, id: &str) -> std::path::PathBuf {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let declared = text
            .lines()
            .take(12)
            .find_map(|line| line.strip_prefix("id: "))
            .map(|value| value.trim().trim_matches('"').trim_matches('\''));
        if declared == Some(id) {
            return path;
        }
    }
    panic!("no .md file for {id} in {}", dir.display());
}

/// A copied document claims the same canonical key and must be refused
/// before either claimant can overwrite the index.
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

    assert_eq!(out.status.code(), Some(4), "reindex must refuse a duplicate key: {}", String::from_utf8_lossy(&out.stderr));

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // Both physical claimants must be identified in the refusal.
    let orig_name = original.file_name().unwrap().to_string_lossy().to_string();
    assert!(
        combined.contains(&orig_name),
        "collision report must list the original file {orig_name}; got:\n{combined}"
    );
    assert!(
        combined.contains("onboarding-shadow-copy.md"),
        "collision report must list the shadow file; got:\n{combined}"
    );
    assert_eq!(std::fs::read_to_string(&original).unwrap(), content);
    assert!(std::fs::read_to_string(&dup).unwrap().contains("Shadow copy body"));
}

/// A clean workspace reindexes without a collision refusal.
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
    assert!(combined.contains("Reindex complete"), "{combined}");
    assert!(
        !combined.contains("duplicate-id collision"),
        "clean workspace must NOT print a collision block; got:\n{combined}"
    );
}
