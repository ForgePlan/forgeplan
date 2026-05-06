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

/// PROB-028 main trace — create an artifact, delete its `.md` file
/// directly, run `forgeplan reindex`, и assert the LanceDB row gets
/// trimmed AND `forgeplan get <id>` reports not-found.
#[test]
fn reindex_trims_orphan_after_md_file_deleted() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let prd = new_artifact(&tmp, "prd", "PROB-028 trace");

    // Find the .md file и delete it directly (simulating git pull / manual rm).
    let prds_dir = tmp.path().join(".forgeplan").join("prds");
    let mut deleted_count = 0;
    for entry in std::fs::read_dir(&prds_dir).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&prd) && name.ends_with(".md") {
            std::fs::remove_file(entry.path()).unwrap();
            deleted_count += 1;
        }
    }
    assert_eq!(deleted_count, 1, "exactly one .md should match {prd}");

    // Run reindex.
    let reindex_out = forgeplan()
        .args(["reindex"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        reindex_out.status.success(),
        "reindex must not crash on missing file: {}",
        String::from_utf8_lossy(&reindex_out.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&reindex_out.stdout),
        String::from_utf8_lossy(&reindex_out.stderr)
    );
    assert!(
        combined.contains(&format!("DEL  {prd}")),
        "Phase 2 must trim {prd}; got:\n{combined}"
    );

    // `forgeplan get` MUST now report not-found.
    let get_out = forgeplan()
        .args(["get", &prd])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        !get_out.status.success(),
        "get on trimmed id must fail; got success"
    );
}

/// PROB-028 resilience — when ONE file's sync fails, reindex MUST
/// continue and Phase 2 trim MUST still run for OTHER missing rows.
/// This is the bug shape that kept PRD-001/SPEC-001 orphans alive in
/// the project workspace despite the trim logic existing.
#[test]
fn reindex_continues_after_per_file_error_and_still_trims_orphans() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    // Create two PRDs. Delete one's .md file. Manipulate the other's
    // frontmatter title to diverge от DB → triggers FileNotFound в
    // sync_body_from_file (pre-fix abort point).
    let prd_orphan = new_artifact(&tmp, "prd", "PROB-028 orphan target");
    let prd_divergent = new_artifact(&tmp, "prd", "PROB-028 divergent title");

    let prds_dir = tmp.path().join(".forgeplan").join("prds");

    // Delete orphan's file.
    for entry in std::fs::read_dir(&prds_dir).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&prd_orphan) && name.ends_with(".md") {
            std::fs::remove_file(entry.path()).unwrap();
        }
    }

    // Touch divergent's body to force a Phase 1 sync attempt (otherwise it
    // skips at "body matches"). We use append rather than full rewrite so
    // frontmatter и body shape stay sane.
    for entry in std::fs::read_dir(&prds_dir).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&prd_divergent) && name.ends_with(".md") {
            let path = entry.path();
            let mut content = std::fs::read_to_string(&path).unwrap();
            content.push_str("\n\nAppended body for PROB-028 test.\n");
            std::fs::write(&path, content).unwrap();
        }
    }

    let reindex_out = forgeplan()
        .args(["reindex"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        reindex_out.status.success(),
        "reindex must succeed even with mixed files; stderr={}",
        String::from_utf8_lossy(&reindex_out.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&reindex_out.stdout),
        String::from_utf8_lossy(&reindex_out.stderr)
    );

    // Phase 2 MUST have run и trimmed the orphan despite any Phase 1
    // per-file errors на the divergent record.
    assert!(
        combined.contains(&format!("DEL  {prd_orphan}")),
        "Phase 2 trim must run after per-file errors; expected `DEL  {prd_orphan}` in:\n{combined}"
    );
}
