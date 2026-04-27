//! Integration tests for `forgeplan dispatch` (PRD-070 CLI parity).

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

/// Append `## Affected Files` section to a freshly-created markdown file
/// so the dispatcher has non-empty file sets to bucket. The MCP-side
/// helper falls back to this section when frontmatter `affected_files`
/// is missing.
fn append_affected_files(tmp: &TempDir, kind_dir: &str, prefix: &str, files: &[&str]) {
    let dir = tmp.path().join(format!(".forgeplan/{kind_dir}"));
    let entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .to_uppercase()
                .starts_with(&prefix.to_uppercase())
        })
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "expected exactly one file matching {prefix} in {dir:?}"
    );
    let path = entries[0].path();
    let mut body = std::fs::read_to_string(&path).unwrap();
    body.push_str("\n\n## Affected Files\n\n");
    for f in files {
        body.push_str(&format!("- `{f}`\n"));
    }
    std::fs::write(&path, body).unwrap();
}

#[test]
fn dispatch_smoke_two_agents_three_prds_text() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    for (i, title) in ["Alpha", "Beta", "Gamma"].iter().enumerate() {
        forgeplan()
            .args(["new", "prd", title])
            .current_dir(tmp.path())
            .assert()
            .success();
        // Disjoint file sets so the dispatcher can split across buckets
        // (the default Jaccard threshold of 0.3 would otherwise serialize
        // overlapping artifacts).
        let file = format!("crates/agent-{i}.rs");
        append_affected_files(&tmp, "prds", &format!("PRD-{:03}", i + 1), &[file.as_str()]);
    }

    forgeplan()
        .args(["dispatch", "--agents", "2"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Dispatch plan"))
        .stdout(predicate::str::contains("Candidates:"));
}

#[test]
fn dispatch_json_output_parses_with_buckets() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    for (i, title) in ["First", "Second", "Third"].iter().enumerate() {
        forgeplan()
            .args(["new", "prd", title])
            .current_dir(tmp.path())
            .assert()
            .success();
        let file = format!("crates/mod-{i}.rs");
        append_affected_files(&tmp, "prds", &format!("PRD-{:03}", i + 1), &[file.as_str()]);
    }

    let output = forgeplan()
        .args(["dispatch", "--agents", "2", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "dispatch --json failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("expected valid JSON: {e}\n\nbody:\n{stdout}"));
    assert_eq!(parsed["agent_count"], 2);
    assert!(parsed["buckets"].is_array(), "buckets must be array");
    let buckets = parsed["buckets"].as_array().unwrap();
    assert_eq!(buckets.len(), 2, "should have one bucket per agent");
    let total_assigned: usize = buckets
        .iter()
        .map(|b| b.as_array().map(Vec::len).unwrap_or(0))
        .sum::<usize>()
        + parsed["serial_queue"].as_array().map(Vec::len).unwrap_or(0);
    assert_eq!(
        total_assigned, 3,
        "expected all 3 PRDs accounted for in buckets+serial: {parsed}"
    );
    assert_eq!(parsed["candidate_count"], 3);
}

#[test]
fn dispatch_rejects_zero_agents() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    forgeplan()
        .args(["dispatch", "--agents", "0"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("agents"));
}

#[test]
fn dispatch_rejects_threshold_out_of_range() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    forgeplan()
        .args(["dispatch", "--agents", "2", "--overlap-threshold", "1.5"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("overlap-threshold"));
}

#[test]
fn dispatch_skips_already_claimed() {
    let tmp = TempDir::new().unwrap();
    init(&tmp);

    forgeplan()
        .args(["new", "prd", "Reserved"])
        .current_dir(tmp.path())
        .assert()
        .success();
    append_affected_files(&tmp, "prds", "PRD-001", &["crates/sole.rs"]);

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "claimant"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["dispatch", "--agents", "2", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["claimed_count"], 1);

    // The artifact was filtered before bucket placement — buckets +
    // serial_queue must therefore be empty (it's the only candidate).
    let bucket_total: usize = parsed["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b.as_array().unwrap().len())
        .sum();
    let serial = parsed["serial_queue"].as_array().unwrap().len();
    assert_eq!(
        bucket_total + serial,
        0,
        "claimed-only candidate should not be planned: {parsed}"
    );
}
