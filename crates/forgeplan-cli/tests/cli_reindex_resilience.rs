//! PROB-028 regression — reindex MUST trim orphan rows even when Phase 1
//! (parse/sync from files) emits per-file errors. Pre-fix `?`-aborted on
//! the first sync_body_from_file FileNotFound (typically caused by
//! title-on-disk diverging от DB-stored title), preventing Phase 2
//! orphan trim from running. Workspace orphans then persisted forever.

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
fn document_declaring(dir: &std::path::Path, id: &str) -> std::path::PathBuf {
    for entry in std::fs::read_dir(dir).expect("directory").flatten() {
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
    panic!("no document declaring {id} in {}", dir.display());
}
fn index_snapshot(root: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    fn visit(
        lance: &std::path::Path,
        current: &std::path::Path,
        entries: &mut Vec<(std::path::PathBuf, Vec<u8>)>,
    ) {
        for entry in std::fs::read_dir(current).expect("published index directory") {
            let entry = entry.unwrap();
            let path = entry.path();
            let relative = path.strip_prefix(lance).unwrap().to_path_buf();
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                entries.push((relative, b"<directory>".to_vec()));
                visit(lance, &path, entries);
            } else if kind.is_file() {
                entries.push((relative, std::fs::read(path).unwrap()));
            } else if kind.is_symlink() {
                let target = std::fs::read_link(path).unwrap();
                entries.push((
                    relative,
                    format!("symlink:{}", target.display()).into_bytes(),
                ));
            }
        }
    }

    let lance = root.join(".forgeplan/lance");
    let mut entries = Vec::new();
    visit(&lance, &lance, &mut entries);
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries
}
fn assert_not_listed(root: &std::path::Path, id: &str) {
    let stdout = forgeplan()
        .args(["list", "--json"])
        .current_dir(root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(stdout).unwrap();
    let json_end = output.rfind(']').expect("list JSON array");
    let records: serde_json::Value = serde_json::from_str(&output[..=json_end]).unwrap();
    assert!(
        !records
            .as_array()
            .unwrap()
            .iter()
            .any(|record| record["id"] == id),
        "{id} must be absent from the published index: {records}"
    );
}

/// A fully admitted row with an immutable before-image may be trimmed after
/// source disappearance has been verified against a complete census.
#[test]
fn reindex_trims_a_proven_source_disappearance() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let prd = new_artifact(&tmp, "prd", "PROB-028 trace");
    let before = index_snapshot(tmp.path());
    std::fs::remove_file(document_declaring(
        &tmp.path().join(".forgeplan/prds"),
        &prd,
    ))
    .expect("the source document is on disk");

    forgeplan()
        .args(["reindex"])
        .current_dir(tmp.path())
        .assert()
        .success();
    assert_ne!(
        index_snapshot(tmp.path()),
        before,
        "a verified deletion must publish an index generation"
    );
    assert_not_listed(tmp.path(), &prd);
}

/// A supported delete command updates durable state; reindex must not resurrect
/// the record.
#[test]
fn supported_delete_survives_reindex() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd = new_artifact(&tmp, "prd", "PROB-028 supported deletion");

    forgeplan()
        .args(["delete", &prd, "--yes"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["reindex"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert_not_listed(tmp.path(), &prd);
}

/// Malformed mixed input cannot authorize trimming an unrelated absent row.
#[test]
fn reindex_mixed_malformed_input_does_not_mutate_the_index() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd_missing = new_artifact(&tmp, "prd", "PROB-028 missing source");
    let prd_malformed = new_artifact(&tmp, "prd", "PROB-028 malformed source");
    let before = index_snapshot(tmp.path());
    let source_dir = tmp.path().join(".forgeplan/prds");
    std::fs::remove_file(document_declaring(&source_dir, &prd_missing))
        .expect("the missing source is on disk");
    std::fs::write(
        document_declaring(&source_dir, &prd_malformed),
        "not valid frontmatter",
    )
    .unwrap();

    let reindex_out = forgeplan()
        .args(["reindex"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        !reindex_out.status.success(),
        "malformed mixed input must not authorize deletion"
    );
    assert_eq!(
        index_snapshot(tmp.path()),
        before,
        "mixed malformed input must leave the published index untouched"
    );
}
