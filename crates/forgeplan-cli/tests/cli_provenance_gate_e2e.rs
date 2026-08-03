//! #360 / PRD-082 slice 2 — activate-time git-delta provenance gate, E2E on the
//! real CLI binary (RED LINE #5: exercise the affected surface end to end, not
//! only the pure gate logic).
//!
//! Proves the load-bearing behaviour: under `block` mode an EvidencePack whose
//! claim does not hold against git (here: base_sha == result_sha, so an empty
//! delta) is REFUSED at `forgeplan activate`, and `--force` overrides it.

use assert_cmd::Command;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn git(dir: &Path, args: &[&str]) {
    let ok = StdCommand::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?}");
}

/// git-init a workspace, `forgeplan init -y`, commit, set the gate to `block`,
/// and return (workspace dir, HEAD sha).
fn setup_block_mode() -> (TempDir, PathBuf, String) {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();

    git(&dir, &["init", "--quiet", "--initial-branch=main"]);
    git(&dir, &["config", "user.email", "t@local"]);
    git(&dir, &["config", "user.name", "T"]);

    forgeplan()
        .args(["init", "-y"])
        .current_dir(&dir)
        .assert()
        .success();

    // Turn the gate to block for this workspace. `forgeplan init` already
    // writes `evidence_provenance_gate: warn` under an `integrity:` block, so
    // replace the value in place — appending a second `integrity:` would be a
    // duplicate YAML key and break config parsing.
    let cfg = dir.join(".forgeplan/config.yaml");
    let text = std::fs::read_to_string(&cfg).unwrap();
    assert!(
        text.contains("evidence_provenance_gate:"),
        "init should seed the gate config; got:\n{text}"
    );
    let text = text.replace(
        "evidence_provenance_gate: warn",
        "evidence_provenance_gate: block",
    );
    std::fs::write(&cfg, text).unwrap();

    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "--quiet", "-m", "init"]);
    let head = String::from_utf8(
        StdCommand::new("git")
            .args(["rev-parse", "--short=7", "HEAD"])
            .current_dir(&dir)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    (tmp, dir, head)
}

/// Create an EvidencePack whose provenance claim is an empty delta
/// (base_sha == result_sha) but claims a changed path — the #360 case.
fn seed_empty_delta_evidence(dir: &Path, head: &str) -> String {
    forgeplan()
        .args(["new", "evidence", "Empty-delta probe"])
        .current_dir(dir)
        .assert()
        .success();

    // Find the created EVID id from the filename.
    let evid_dir = dir.join(".forgeplan/evidence");
    let file = std::fs::read_dir(&evid_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .find(|n| n.starts_with("EVID-"))
        .expect("an EVID file");
    let id = file.split('-').take(2).collect::<Vec<_>>().join("-");

    let body = format!(
        "## Structured Fields\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n\
         base_sha: {head}\nresult_sha: {head}\nchanged_paths: src/thing.rs\n\n## Summary\n\nprobe\n"
    );
    let body_file = dir.join("body.md");
    std::fs::write(&body_file, body).unwrap();
    forgeplan()
        .args([
            "update",
            &id,
            "--body",
            &format!("@{}", body_file.display()),
        ])
        .current_dir(dir)
        .assert()
        .success();

    id
}

#[test]
fn block_mode_refuses_activation_on_an_empty_delta_claim() {
    let (_tmp, dir, head) = setup_block_mode();
    let id = seed_empty_delta_evidence(&dir, &head);

    // The provenance gate fires before the lifecycle transition and must refuse.
    forgeplan()
        .args(["activate", &id])
        .current_dir(&dir)
        .assert()
        .failure()
        .stderr(predicates::str::contains("provenance gate"));
}

#[test]
fn force_overrides_the_provenance_gate() {
    let (_tmp, dir, head) = setup_block_mode();
    let id = seed_empty_delta_evidence(&dir, &head);

    // --force bypasses the gate (and the methodology gates), so activation
    // succeeds even though the claim does not hold.
    forgeplan()
        .args(["activate", &id, "--force"])
        .current_dir(&dir)
        .assert()
        .success();
}
