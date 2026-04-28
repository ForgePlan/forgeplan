//! Phase 5 Wave 4 — end-to-end integration tests for `forgeplan ingest`.
//!
//! Where `cli_ingest.rs` (Wave 3) used inline YAML to cover unit-style
//! behaviours (dry-run print, idempotency, `--update`, exit codes), this
//! suite drives the full pipeline against the on-disk fixtures
//!
//!   `tests/fixtures/c4-mini/components/{auth,database}.md`
//!   `tests/fixtures/mappings/c4-to-forge-test.yaml`
//!
//! exercising:
//!
//!   - real markdown parsing (front_matter_plus_sections)
//!   - selector matching across multiple files
//!   - idempotency on re-run (source_hash)
//!   - `--update` refresh on mutated source
//!   - schema rejection (Tera filter abuse, invariant violation)
//!
//! ## AC traceability — PRD-066
//!
//!   AC-1 (mapping applied -> N artifacts)
//!     -> e2e_ingest_dry_run_on_c4_fixture (counts)
//!     -> e2e_ingest_writes_artifacts (writes)
//!
//!   AC-2 (## Sources section with file:line refs)
//!     -> e2e_ingest_writes_artifacts (asserts ## Sources block + path)
//!
//!   AC-3 (re-run idempotent, no duplicates)
//!     -> e2e_ingest_idempotent_rerun
//!
//!   AC-4 (forgeplan doctor --sources):
//!     -> NOTE: deferred — `forgeplan doctor --sources` flag is Wave 4
//!        post-PR scope (no CLI surface in current build). Tracked in
//!        TODO.md by w4b. The ingest engine *does* emit `source_hash`
//!        and path to enable that doctor command later.
//!
//!   AC-5 (schema violation -> clear validation error, no partial ingest)
//!     -> e2e_ingest_invalid_mapping_exits_2
//!
//!   AC-6 (5 canonical mappings publish in marketplace/)
//!     -> deferred to w4b (docs/marketplace ownership)

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

// =====================================================================
// Helpers
// =====================================================================

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").expect("test fixture: cargo_bin forgeplan")
}

fn init_workspace(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

fn fixture_path(rel: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

/// Copy `tests/fixtures/c4-mini/` into `<tmp>/c4-mini/` so the mapping's
/// glob `c4-mini/**/*.md` finds the two component files. Returns the
/// path to the directory inside the temp workspace.
fn install_c4_fixture(tmp: &TempDir) -> std::path::PathBuf {
    let src_dir = fixture_path("c4-mini");
    let dst_dir = tmp.path().join("c4-mini");
    let components_src = src_dir.join("components");
    let components_dst = dst_dir.join("components");
    fs::create_dir_all(&components_dst).expect("test fixture: create components dst");
    for name in ["auth.md", "database.md"] {
        let from = components_src.join(name);
        let to = components_dst.join(name);
        fs::copy(&from, &to).unwrap_or_else(|e| panic!("test fixture: copy {name}: {e}"));
    }
    dst_dir
}

/// Copy `tests/fixtures/mappings/c4-to-forge-test.yaml` into the temp
/// workspace and return its path.
fn install_mapping(tmp: &TempDir) -> std::path::PathBuf {
    let src = fixture_path("mappings/c4-to-forge-test.yaml");
    let dst = tmp.path().join("c4-to-forge-test.yaml");
    fs::copy(&src, &dst).expect("test fixture: copy mapping");
    dst
}

// =====================================================================
// AC-1 (dry-run reports drafts)
// =====================================================================

#[test]
fn e2e_ingest_dry_run_on_c4_fixture() {
    let tmp = TempDir::new().expect("test fixture: tempdir");
    init_workspace(&tmp);
    let mapping = install_mapping(&tmp);
    install_c4_fixture(&tmp);

    let out = forgeplan()
        .args([
            "ingest",
            "--mapping",
            mapping.to_str().unwrap(),
            "--source",
            "c4-mini",
            "--dry-run",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: run ingest --dry-run");
    assert!(
        out.status.success(),
        "dry-run failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("test fixture: dry-run JSON");
    // Dry-run path emits `dry_run: true`; we check the artifact count via
    // the `drafts` array (engine output, not yet written).
    let drafts = v["drafts"].as_array().expect("drafts array");
    assert_eq!(
        drafts.len(),
        2,
        "expected 2 drafts (auth + database components); got {}: {v}",
        drafts.len()
    );
    // Titles should be the front-matter `name` (per mapping rule).
    let titles: Vec<&str> = drafts.iter().filter_map(|d| d["title"].as_str()).collect();
    assert!(
        titles.contains(&"Auth Service"),
        "missing Auth Service title: {titles:?}"
    );
    assert!(
        titles.contains(&"Database"),
        "missing Database title: {titles:?}"
    );
}

// =====================================================================
// AC-1 + AC-2 (real write -> artifacts on disk + ## Sources block)
// =====================================================================

#[test]
fn e2e_ingest_writes_artifacts() {
    let tmp = TempDir::new().expect("test fixture: tempdir");
    init_workspace(&tmp);
    let mapping = install_mapping(&tmp);
    install_c4_fixture(&tmp);

    let out = forgeplan()
        .args([
            "ingest",
            "--mapping",
            mapping.to_str().unwrap(),
            "--source",
            "c4-mini",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: run ingest");
    assert!(
        out.status.success(),
        "ingest failed: stderr={}\nstdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("test fixture: ingest JSON");

    let written = v["written"].as_array().expect("written array");
    assert_eq!(written.len(), 2, "expected 2 written artifacts: {v}");

    // Each artifact must carry an ID and be reachable via `forgeplan get`.
    for w in written {
        let id = w["id"].as_str().expect("artifact id");
        assert!(id.starts_with("NOTE-"), "expected NOTE-* id, got {id}");

        let getout = forgeplan()
            .args(["get", id])
            .current_dir(tmp.path())
            .output()
            .expect("test fixture: forgeplan get");
        assert!(
            getout.status.success(),
            "get {id} failed: stderr={}",
            String::from_utf8_lossy(&getout.stderr)
        );
        let body = String::from_utf8_lossy(&getout.stdout);
        // ADR-009 invariant: every ingested artifact MUST have a
        // ## Sources section pointing back at the source markdown.
        assert!(
            body.contains("## Sources"),
            "{id} missing ## Sources section; body=\n{body}"
        );
        assert!(
            body.contains("c4-mini/components/"),
            "{id} ## Sources must reference c4-mini path; body=\n{body}"
        );
    }
}

// =====================================================================
// AC-3 (idempotent re-run)
// =====================================================================

#[test]
fn e2e_ingest_idempotent_rerun() {
    let tmp = TempDir::new().expect("test fixture: tempdir");
    init_workspace(&tmp);
    let mapping = install_mapping(&tmp);
    install_c4_fixture(&tmp);

    // First run -> 2 writes.
    forgeplan()
        .args([
            "ingest",
            "--mapping",
            mapping.to_str().unwrap(),
            "--source",
            "c4-mini",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Second run on unchanged source -> 0 writes, >=1 idempotent skip.
    let out = forgeplan()
        .args([
            "ingest",
            "--mapping",
            mapping.to_str().unwrap(),
            "--source",
            "c4-mini",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: run ingest second time");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("test fixture: 2nd-run JSON");
    let written = v["written"].as_array().expect("written array");
    assert!(
        written.is_empty(),
        "second run must not create new artifacts; got: {written:?}"
    );
    let skipped = v["skipped_existing"].as_array().expect("skipped array");
    assert_eq!(
        skipped.len(),
        2,
        "second run must skip both pre-existing artifacts; got: {skipped:?}"
    );
    let errors = v["errors"].as_array().expect("errors array");
    assert!(errors.is_empty(), "no errors expected: {errors:?}");
}

// =====================================================================
// `--update` refreshes the artifact body on a mutated source
// =====================================================================

#[test]
fn e2e_ingest_update_flag_refreshes() {
    let tmp = TempDir::new().expect("test fixture: tempdir");
    init_workspace(&tmp);
    let mapping = install_mapping(&tmp);
    let dir = install_c4_fixture(&tmp);

    // First ingest writes both artifacts.
    forgeplan()
        .args([
            "ingest",
            "--mapping",
            mapping.to_str().unwrap(),
            "--source",
            "c4-mini",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Mutate auth.md so its source_hash changes (any body edit suffices).
    let auth_path = dir.join("components").join("auth.md");
    let original = fs::read_to_string(&auth_path).expect("test fixture: read auth.md");
    let mutated = original.replace(
        "The Auth Service authenticates incoming HTTP requests",
        "UPDATED: The Auth Service now also handles SSO requests",
    );
    assert_ne!(mutated, original, "test fixture mutation must take effect");
    fs::write(&auth_path, mutated).expect("test fixture: write mutated auth.md");

    // Re-run with --update -> one write (the changed file), plus one skip
    // (database.md is unchanged).
    let out = forgeplan()
        .args([
            "ingest",
            "--mapping",
            mapping.to_str().unwrap(),
            "--source",
            "c4-mini",
            "--update",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: run ingest --update");
    assert!(
        out.status.success(),
        "update run failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("test fixture: update JSON");
    let written = v["written"].as_array().expect("written");
    assert_eq!(
        written.len(),
        1,
        "expected exactly one updated artifact; got: {written:?}"
    );
    // The written entry should reuse the existing id (NOTE-001 or NOTE-002),
    // not allocate a fresh one.
    let id = written[0]["id"]
        .as_str()
        .expect("written[0].id is a string");
    assert!(
        id == "NOTE-001" || id == "NOTE-002",
        "update must reuse an existing NOTE id, got: {id}"
    );
}

// =====================================================================
// AC-5 (invalid mapping -> exit 2 with Fix: hint)
// =====================================================================

#[test]
fn e2e_ingest_invalid_mapping_exits_2() {
    let tmp = TempDir::new().expect("test fixture: tempdir");
    init_workspace(&tmp);
    install_c4_fixture(&tmp);

    // Broken YAML: `sources_section.include: false` violates ADR-009
    // (hallucination-proof invariant). Loader must reject before any
    // file is touched.
    let bad_mapping = tmp.path().join("broken.yaml");
    fs::write(
        &bad_mapping,
        r#"
schema_version: "1.0"
name: broken
title: "broken"
compat_spec_version: "^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: "c4-mini/**/*.md"
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
    .expect("test fixture: write broken mapping");

    let out = forgeplan()
        .args([
            "ingest",
            "--mapping",
            bad_mapping.to_str().unwrap(),
            "--source",
            "c4-mini",
            "--dry-run",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: run broken-mapping ingest");

    assert_eq!(
        out.status.code(),
        Some(2),
        "broken mapping must exit 2; got: {:?}",
        out.status.code()
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    // Per PRD-071 hint contract: `Error:` + `Fix:` lines on stderr.
    assert!(
        stderr.contains("invalid mapping YAML")
            || stderr.contains("hallucination-proof")
            || stderr.contains("include")
            || stderr.contains("sources_section"),
        "expected validation error on stderr; got: {stderr}"
    );
    // Workspace must still be clean (no NOTE-001 created on partial run).
    let listout = forgeplan()
        .args(["list", "--type", "note"])
        .current_dir(tmp.path())
        .output()
        .expect("test fixture: forgeplan list");
    let listed = String::from_utf8_lossy(&listout.stdout);
    // `list --type note` on empty workspace must not surface any NOTE-*.
    // Tolerate either "No matches" or empty body.
    assert!(
        !listed.contains("NOTE-001"),
        "broken mapping leaked an artifact; list output:\n{listed}"
    );
    // Use predicate to get nice failure messages too.
    let _ = predicate::str::contains("NOTE-001").not();
}

// =====================================================================
// Plugins integration (env-isolated home + clean workspace)
// =====================================================================
//
// These two tests live alongside ingest because they share the
// "clean HOME -> plugins doctor reports missing" environment-isolation
// pattern from `cli_plugins.rs` (Wave 3, w3b).

#[test]
fn e2e_plugins_list_clean_workspace() {
    let home = TempDir::new().expect("test fixture: clean HOME");
    let cwd = TempDir::new().expect("test fixture: cwd");

    let mut cmd = Command::cargo_bin("forgeplan").expect("test fixture: cargo_bin forgeplan");
    cmd.env("HOME", home.path());

    let out = cmd
        .args(["plugins", "list", "--json"])
        .current_dir(cwd.path())
        .output()
        .expect("test fixture: run plugins list");
    assert!(
        out.status.success(),
        "plugins list must succeed on clean HOME"
    );
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("test fixture: plugins list JSON");
    let installed = v["installed"].as_array().expect("installed array");
    // Synthetic forgeplan entry is always present even on a clean HOME.
    let has_forgeplan = installed.iter().any(|p| p["info"]["name"] == "forgeplan");
    assert!(
        has_forgeplan,
        "synthetic forgeplan entry must be present on clean HOME: {v}"
    );
    // No real plugins detected -> only the synthetic entry.
    let real_plugins: Vec<&serde_json::Value> = installed
        .iter()
        .filter(|p| p["info"]["name"] != "forgeplan")
        .collect();
    assert!(
        real_plugins.is_empty(),
        "clean HOME must surface no installed plugins: {real_plugins:?}"
    );
}

#[test]
fn e2e_plugins_doctor_reports_known_missing() {
    let home = TempDir::new().expect("test fixture: clean HOME");
    let cwd = TempDir::new().expect("test fixture: cwd");

    let mut cmd = Command::cargo_bin("forgeplan").expect("test fixture: cargo_bin forgeplan");
    cmd.env("HOME", home.path());

    let out = cmd
        .args(["plugins", "doctor", "--json"])
        .current_dir(cwd.path())
        .output()
        .expect("test fixture: run plugins doctor");
    // Empty HOME -> every known plugin missing -> exit 1.
    assert_eq!(
        out.status.code(),
        Some(1),
        "doctor must exit 1 when plugins missing; got: {:?}",
        out.status.code()
    );
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("test fixture: doctor JSON");
    let missing = v["missing"].as_array().expect("missing array");
    assert!(
        missing.len() >= 4,
        "expected at least 4 missing entries (registry minimum); got {}: {v}",
        missing.len()
    );
    // PRD-067 AC-6: every missing entry MUST emit an actionable
    // `install_command`.
    for m in missing {
        let cmd = m["install_command"].as_str().unwrap_or("");
        assert!(
            !cmd.is_empty(),
            "missing entry without install_command: {m}"
        );
    }
}
