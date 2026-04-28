//! Integration tests for `forgeplan ingest` (PRD-066 / SPEC-004).
//!
//! Exercises the full pipeline: mapping load → source parse → engine apply
//! → idempotent artifact write. Each test runs in its own [`TempDir`] with
//! a freshly-initialised workspace so collisions across cases are
//! impossible.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
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

/// Write a minimal mapping YAML that ingests `<root>/sources/foo.md` (a
/// single markdown file with one heading) into a `Note` artifact. Every
/// rule fires once.
fn write_minimal_mapping(root: &Path) -> std::path::PathBuf {
    let path = root.join("mapping.yaml");
    let yaml = r#"
schema_version: "1.0"
name: cli-ingest-test
title: "CLI ingest test mapping"
compat_spec_version: "^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: "sources/*.md"
    type: markdown
    parser: front_matter_plus_sections
rules:
  - id: any-doc-to-note
    when:
      file_glob: "**/sources/*.md"
    target:
      kind: note
    fields:
      title: "{{ front_matter.name | default(value=\"Imported note\") }}"
      summary: "Imported via CLI ingest test"
    sources_section:
      include: true
      format: "{path}:{line_start}-{line_end}"
      precision: line
      source_hash: true
"#;
    fs::write(&path, yaml).unwrap();
    path
}

fn write_source_file(root: &Path) -> std::path::PathBuf {
    let dir = root.join("sources");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("foo.md");
    fs::write(
        &path,
        "---\nname: First Imported\nkind: doc\n---\n\n# First Imported\n\nbody text here.\n",
    )
    .unwrap();
    path
}

#[test]
fn ingest_dry_run_prints_drafts() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let mapping = write_minimal_mapping(tmp.path());
    write_source_file(tmp.path());

    forgeplan()
        .args([
            "ingest",
            "--mapping",
            mapping.to_str().unwrap(),
            "--source",
            "sources",
            "--dry-run",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("dry-run"))
        .stdout(predicate::str::contains("First Imported"));
}

#[test]
fn ingest_writes_artifact_to_workspace() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let mapping = write_minimal_mapping(tmp.path());
    write_source_file(tmp.path());

    forgeplan()
        .args([
            "ingest",
            "--mapping",
            mapping.to_str().unwrap(),
            "--source",
            "sources",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Written"))
        .stdout(predicate::str::contains("NOTE-001"));

    // Subsequent `forgeplan get` must succeed.
    forgeplan()
        .args(["get", "NOTE-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("First Imported"));
}

#[test]
fn ingest_rerun_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let mapping = write_minimal_mapping(tmp.path());
    write_source_file(tmp.path());

    // First run creates NOTE-001.
    forgeplan()
        .args([
            "ingest",
            "--mapping",
            mapping.to_str().unwrap(),
            "--source",
            "sources",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Second run on unchanged source: must report idempotent skip and
    // NOT create NOTE-002.
    let output = forgeplan()
        .args([
            "ingest",
            "--mapping",
            mapping.to_str().unwrap(),
            "--source",
            "sources",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("run ingest");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let written = v["written"].as_array().unwrap();
    assert!(
        written.is_empty(),
        "expected no new writes on idempotent re-run, got: {written:?}"
    );
    let skipped = v["skipped_existing"].as_array().unwrap();
    assert!(
        !skipped.is_empty(),
        "expected at least one idempotent skip, got: {skipped:?}"
    );
}

#[test]
fn ingest_with_update_refreshes_changed_source() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let mapping = write_minimal_mapping(tmp.path());
    let src = write_source_file(tmp.path());

    // First ingest.
    forgeplan()
        .args([
            "ingest",
            "--mapping",
            mapping.to_str().unwrap(),
            "--source",
            "sources",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Mutate source content so source_hash changes.
    fs::write(
        &src,
        "---\nname: First Imported\nkind: doc\n---\n\n# First Imported\n\nUPDATED body text.\n",
    )
    .unwrap();

    // Re-run with --update; expect Updated outcome (still NOTE-001).
    let output = forgeplan()
        .args([
            "ingest",
            "--mapping",
            mapping.to_str().unwrap(),
            "--source",
            "sources",
            "--update",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("run update ingest");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let written = v["written"].as_array().unwrap();
    assert_eq!(written.len(), 1);
    assert_eq!(written[0]["id"], "NOTE-001");
}

#[test]
fn ingest_invalid_mapping_yaml_returns_exit_code_2() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    write_source_file(tmp.path());

    // Mapping that violates ADR-009 (sources_section.include must be true).
    let bad_mapping = tmp.path().join("bad-mapping.yaml");
    fs::write(
        &bad_mapping,
        r#"
schema_version: "1.0"
name: bad-mapping
title: "bad"
compat_spec_version: "^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: "sources/*.md"
    type: markdown
    parser: front_matter_plus_sections
rules:
  - id: r1
    when: {}
    target: { kind: note }
    fields:
      title: "{{ x }}"
    sources_section:
      include: false
"#,
    )
    .unwrap();

    let output = forgeplan()
        .args([
            "ingest",
            "--mapping",
            bad_mapping.to_str().unwrap(),
            "--source",
            "sources",
            "--dry-run",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("run ingest with bad mapping");

    // Exit code 2 (input/validation error).
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid mapping YAML")
            || stderr.contains("hallucination-proof")
            || stderr.contains("include"),
        "expected validation error in stderr, got: {stderr}"
    );
}

#[test]
fn ingest_missing_mapping_returns_exit_code_2() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let output = forgeplan()
        .args([
            "ingest",
            "--mapping",
            "nonexistent.yaml",
            "--source",
            ".",
            "--dry-run",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("run ingest with missing mapping");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found"),
        "expected 'not found' in stderr, got: {stderr}"
    );
}
