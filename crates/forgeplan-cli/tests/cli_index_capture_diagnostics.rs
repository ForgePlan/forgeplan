//! Initial indexing must expose incomplete-source diagnostics to CLI users.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn initial_index_reports_invalid_links_and_identity_collisions() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let root = tmp.path().join("workspace");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&root).unwrap();

    let command = || {
        let mut cmd = Command::cargo_bin("forgeplan").unwrap();
        cmd.current_dir(&root)
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("XDG_DATA_HOME", &home);
        cmd
    };
    command().args(["init", "-y"]).assert().success();

    let workspace = root.join(".forgeplan");
    let duplicate = "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Duplicate\n---\n\nBody.\n";
    std::fs::write(workspace.join("prds/PRD-001-a.md"), duplicate).unwrap();
    std::fs::write(workspace.join("prds/PRD-001-b.md"), duplicate).unwrap();
    std::fs::write(
        workspace.join("prds/PRD-002-invalid.md"),
        "---\nid: PRD-002\nkind: prd\nstatus: draft\ntitle: Invalid\nlinks: invalid\n---\n\nBody.\n",
    )
    .unwrap();

    command()
        .args(["index"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("source_unavailable"))
        .stderr(predicate::str::contains("InvalidLinks"))
        .stderr(predicate::str::contains("PRD-002-invalid.md"))
        .stderr(predicate::str::contains("IdentityCollision"))
        .stderr(predicate::str::contains("PRD-001-a.md"))
        .stderr(predicate::str::contains("PRD-001-b.md"))
        .stderr(predicate::str::contains("source_busy").not());
    assert!(!workspace.join("lance/CURRENT.json").exists());
}
