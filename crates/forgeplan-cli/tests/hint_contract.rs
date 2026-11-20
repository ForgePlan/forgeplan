use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn setup_workspace() -> TempDir {
    let dir = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(dir.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Lifecycle Gate Subject"])
        .current_dir(dir.path())
        .assert()
        .success();
    dir
}

fn run(ws: &TempDir, args: &[&str]) -> std::process::Output {
    forgeplan()
        .args(args)
        .current_dir(ws.path())
        .output()
        .expect("forgeplan invocation")
}

#[test]
fn direct_active_status_change_is_rejected_without_mutating_the_artifact() {
    let ws = setup_workspace();
    let out = run(&ws, &["update", "PRD-001", "--status", "active"]);
    assert!(
        !out.status.success(),
        "update --status active must fail (lifecycle gate)"
    );
    let current = run(&ws, &["get", "PRD-001", "--json"]);
    assert!(current.status.success());
    let artifact: Value = serde_json::from_slice(&current.stdout).unwrap();
    assert_eq!(artifact["status"], "draft");
}
