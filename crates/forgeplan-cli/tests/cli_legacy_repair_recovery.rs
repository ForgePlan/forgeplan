#![cfg(unix)]

use std::path::Path;
use std::process::{Command, Output};

fn run(root: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_forgeplan"))
        .args(args)
        .current_dir(root)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("XDG_DATA_HOME", home)
        .output().unwrap()
}

fn success(output: &Output) {
    assert!(output.status.success(), "stdout: {}\nstderr: {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

#[test]
fn cli_apply_then_reconcile_unblocks_legacy_reads_and_writes() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    success(&run(root.path(), home.path(), &["init", "-y"]));
    let ws = root.path().join(".forgeplan");
    let lance = ws.join("lance");
    // Construct an empty markerless legacy index using the actual empty Lance
    // tables, then introduce two unindexed historical documents.
    std::fs::remove_file(lance.join("IDENTITY_V5.json")).unwrap();
    for name in ["artifacts", "relations", "evidence"] {
        std::fs::rename(lance.join(format!("{name}_identity_v5.lance")), lance.join(format!("{name}.lance"))).unwrap();
    }
    for (name, title) in [("PRD-001-a.md", "First"), ("PRD-001-b.md", "Second")] {
        std::fs::write(ws.join("prds").join(name), format!("---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: {title}\n---\n\n{title} body.\n")).unwrap();
    }
    let plan_path = root.path().join("plan.json");
    let plan_arg = plan_path.to_str().unwrap();
    success(&run(root.path(), home.path(), &["reconcile-ids", "--plan-out", plan_arg, "--json"]));
    let plan: serde_json::Value = serde_json::from_slice(&std::fs::read(&plan_path).unwrap()).unwrap();
    let key = plan["sources"][0]["new_key"].as_str().unwrap().to_owned();
    let missing = lance.join("evidence.lance");
    let parked = lance.join("evidence.saved");
    std::fs::rename(&missing, &parked).unwrap();
    let apply = run(root.path(), home.path(), &["reconcile-ids", "--apply-plan", plan_arg, "--json"]);
    assert!(!apply.status.success());
    let report: serde_json::Value = serde_json::from_slice(&apply.stdout).unwrap();
    let transaction = report["transaction_id"].as_str().unwrap();
    std::fs::rename(&parked, &missing).unwrap();
    assert!(!run(root.path(), home.path(), &["get", &key]).status.success());
    let recovered = run(root.path(), home.path(), &["reconcile-ids", "--recover", transaction, "--reconcile-index", "--json"]);
    success(&recovered);
    let report: serde_json::Value = serde_json::from_slice(&recovered.stdout).unwrap();
    assert_eq!(report["ready"], true);
    assert_eq!(report["reconciliation_confirmed"], true);
    assert!(lance.join("IDENTITY_V5.json").is_file());
    success(&run(root.path(), home.path(), &["get", &key]));
    success(&run(root.path(), home.path(), &["update", &key, "--body", "Updated after recovery."]));
    let read = run(root.path(), home.path(), &["get", &key]);
    success(&read);
    assert!(String::from_utf8_lossy(&read.stdout).contains("Updated after recovery."));
}
