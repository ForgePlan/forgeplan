// PRD-056 (EPIC-005): integration tests for `forgeplan phase-advance`
// CLI command.
//
// Verifies advisory phase advance semantics:
//   * advance writes / updates `.forgeplan/state/<id>.yaml`
//   * out-of-order jumps are allowed (advisory layer per FR)
//   * subsequent reads surface the new current_phase + cumulative history
//   * --reason is recorded
//   * --json emits a parseable payload

use assert_cmd::Command;
use predicates::prelude::*;
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

fn create_prd(tmp: &TempDir) {
    forgeplan()
        .args(["new", "prd", "Phase Advance Test"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));
}

#[test]
fn advance_to_shape_then_code_records_history() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    create_prd(&tmp);

    // First advance: shape.
    forgeplan()
        .args(["phase-advance", "PRD-001", "--to", "shape"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("shape"));

    // Second advance: code (jump over validate/adi -- advisory allows it).
    forgeplan()
        .args([
            "phase-advance",
            "PRD-001",
            "--to",
            "code",
            "--reason",
            "skipping forward",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("code"));

    // Read it back; cumulative history must include both transitions.
    let output = forgeplan()
        .args(["phase", "PRD-001", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(v["current_phase"], "code");
    let history = v["history"].as_array().expect("history is array");
    assert!(
        history.len() >= 2,
        "expected at least 2 history entries, got {}",
        history.len()
    );
    let last = &history[history.len() - 1];
    assert_eq!(last["to"], "code");
    assert_eq!(last["reason"], "skipping forward");
}

#[test]
fn advance_json_emits_suggested_next() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    create_prd(&tmp);

    let output = forgeplan()
        .args(["phase-advance", "PRD-001", "--to", "validate", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(v["current_phase"], "validate");
    assert_eq!(v["suggested_next"], "adi");
    assert_eq!(v["artifact_id"], "PRD-001");
}

#[test]
fn advance_to_done_is_terminal() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    create_prd(&tmp);

    forgeplan()
        .args(["phase-advance", "PRD-001", "--to", "done"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("done"))
        .stdout(predicate::str::contains("terminal"));
}

#[test]
fn advance_rejects_invalid_phase_value() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    create_prd(&tmp);

    // clap value_enum should reject an unknown variant before our code runs.
    forgeplan()
        .args(["phase-advance", "PRD-001", "--to", "bogus"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

#[test]
fn advance_without_workspace_errors_clearly() {
    let tmp = TempDir::new().unwrap();
    // Deliberately do not init -- there is no .forgeplan/.
    forgeplan()
        .args(["phase-advance", "PRD-001", "--to", "shape"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("forgeplan init").or(predicate::str::contains(".forgeplan")),
        );
}

#[test]
fn phase_by_key_refuses_normalized_ambiguous_legacy_state() {
    use forgeplan_core::db::store::{LanceStore, NewArtifact};

    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let workspace = tmp.path().join(".forgeplan");
    let docs = [
        ("prd-alpha-k11111111111111111111111111111111", "PRD-001"),
        ("prd-beta-k22222222222222222222222222222222", "PRD-1"),
    ];
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let guard = forgeplan_core::workspace::acquire_workspace_lock(&workspace).await.unwrap();
        let store = LanceStore::open(&workspace).await.unwrap();
        for (key, id) in docs {
            let record = NewArtifact {
                canonical_key: key.to_string(),
                id: id.to_string(),
                kind: "prd".to_string(),
                status: "draft".to_string(),
                title: key.to_string(),
                body: "Phase alias regression.".to_string(),
                depth: "standard".to_string(),
                ..Default::default()
            };
            store.create_artifact_for_test(&guard, &record).await.unwrap();
            let stored = store.get_record(&guard, key).await.unwrap().unwrap();
            forgeplan_core::projection::render_projection_record_with_body(&workspace, &stored, &[])
                .await.unwrap();
        }
    });
    let state = workspace.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let legacy = state.join("PRD-001.yaml");
    let bytes = b"artifact_id: PRD-001\ncurrent_phase: shape\n";
    std::fs::write(&legacy, bytes).unwrap();
    for (key, _) in docs {
        for args in [
            vec!["phase", key, "--json"],
            vec!["phase-advance", key, "--to", "code"],
        ] {
            let output = forgeplan().args(&args).current_dir(tmp.path()).output().unwrap();
            assert!(!output.status.success(), "{args:?}: {}", String::from_utf8_lossy(&output.stdout));
            assert!(String::from_utf8_lossy(&output.stderr).contains("matches 2 documents"), "{args:?}: {}", String::from_utf8_lossy(&output.stderr));
        }
        assert!(!state.join(format!("{key}.yaml")).exists());
    }
    assert_eq!(std::fs::read(&legacy).unwrap(), bytes);
    let key = docs[0].0;
    let keyed = state.join(format!("{key}.yaml"));
    std::fs::write(&keyed, format!(
        "artifact_id: PRD-001\ncanonical_key: {key}\ncurrent_phase: shape\nadvanced_at: '2025-01-01T00:00:00Z'\nhistory: []\n"
    )).unwrap();
    forgeplan().args(["phase-advance", key, "--to", "code"])
        .current_dir(tmp.path()).assert().success();
    let keyed_after: serde_yaml::Value = serde_yaml::from_slice(&std::fs::read(&keyed).unwrap()).unwrap();
    assert_eq!(keyed_after["current_phase"].as_str(), Some("code"));
    assert_eq!(std::fs::read(&legacy).unwrap(), bytes);
}
