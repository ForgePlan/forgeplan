//! The shared resolution contract, seen from the CLI side.
//!
//! SPEC-009 freezes the resolution order and the error taxonomy; this file
//! pins the CLI half of it:
//!
//! * a name two documents answer to names **neither** of them — the
//!   command prints every candidate (canonical key, title and
//!   workspace-relative path) and exits non-zero;
//! * a name no document carries is a miss, and an unusable reference is
//!   not a miss — the two stay apart;
//! * the CLI and the MCP server resolve through the SAME resolver, so the
//!   same selector against the same document set yields the same document
//!   or the same failure code;
//! * the graph's node id is the canonical key and every node is labelled
//!   with its display id and its title — two documents that share a
//!   display id stay two vertices.

use assert_cmd::Command;
use forgeplan_core::artifact::canonical_key::ArtifactCandidate;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// One document of the resolver's document set.
fn document(key: &str, display: &str, title: &str, path: &str) -> ArtifactCandidate {
    ArtifactCandidate {
        canonical_key: key.to_string(),
        display_id: display.to_string(),
        title: title.to_string(),
        relative_path: PathBuf::from(path),
    }
}

/// A workspace with two documents that carry the SAME display id.
///
/// Their canonical keys differ — the slug is the key — so neither document
/// is an identity collision and both survive the capture. The display id is
/// the alias they share, and it is the alias that must not pick a winner.
fn workspace_with_two_prds(root: &Path) {
    std::fs::create_dir_all(root.join(".forgeplan/prds")).unwrap();
    std::fs::write(
        root.join(".forgeplan/prds/PRD-001-alpha.md"),
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Alpha\nslug: prd-alpha\n---\n\nAlpha body.\n",
    )
    .unwrap();
    std::fs::write(
        root.join(".forgeplan/prds/PRD-001-beta.md"),
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Beta\nslug: prd-beta\n---\n\nBeta body.\n",
    )
    .unwrap();
}

/// A workspace whose documents are published to the Constellation index.
///
/// The published generation is what a migrated workspace reads, and it is
/// the only layout whose index can hold two documents under one display id.
fn migrated_workspace_with_two_prds() -> TempDir {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    workspace_with_two_prds(tmp.path());
    forgeplan()
        .arg("index")
        .current_dir(tmp.path())
        .assert()
        .success();
    tmp
}

/// The CLI and the MCP server resolve the same selector over the same
/// document set to the same document — and refuse it with the same code.
///
/// The two surfaces call the same resolver, so this is a comparison of two
/// call sites, not of two algorithms.
#[test]
fn the_cli_and_the_mcp_server_agree_on_the_document_and_on_the_refusal() {
    let documents = vec![
        document("prd-alpha", "PRD-001", "Alpha", "prds/PRD-001-alpha.md"),
        document("prd-beta", "PRD-002", "Beta", "prds/PRD-001-beta.md"),
    ];

    for selector in ["PRD-002", "prd-alpha", "PRD-404", "prd-alpha@other"] {
        let cli = forgeplan::commands::common::resolve_in_document_set(selector, &documents);
        let mcp = forgeplan_mcp::convert::resolve_reference(selector, &documents);
        assert_eq!(
            forgeplan_mcp::convert::resolution_code(&cli),
            forgeplan_mcp::convert::resolution_code(&mcp),
            "the CLI and the MCP server must report the same code for {selector:?}"
        );
        assert_eq!(
            cli.canonical_key(),
            mcp.canonical_key(),
            "the CLI and the MCP server must name the same document for {selector:?}"
        );
    }
}

/// The refusal the CLI renders: the machine-readable code, and every
/// candidate named by its canonical key, its title and its workspace-
/// relative path.
#[test]
fn the_refusal_names_every_candidate_by_key_title_and_relative_path() {
    let documents = vec![
        document("prd-alpha", "PRD-001", "Alpha", "prds/PRD-001-alpha.md"),
        document("prd-beta", "PRD-001", "Beta", "prds/PRD-001-beta.md"),
    ];
    let outcome = forgeplan::commands::common::resolve_in_document_set("PRD-001", &documents);
    let failure =
        forgeplan::commands::common::resolution_failure(&outcome, "PRD-001", &documents);

    assert_eq!(failure.code, "ambiguous");
    let lines = failure.candidate_lines();
    assert_eq!(lines.len(), 2, "every claimant must be named: {lines:?}");
    for (line, key, title) in [
        (&lines[0], "prd-alpha", "Alpha"),
        (&lines[1], "prd-beta", "Beta"),
    ] {
        assert!(line.contains(key), "the canonical key must be named: {line}");
        assert!(line.contains(title), "the title must be named: {line}");
    }
    assert!(
        lines[0].contains("prds/PRD-001-alpha.md")
            && lines[1].contains("prds/PRD-001-beta.md"),
        "the workspace-relative path must be named: {lines:?}"
    );
    for line in &lines {
        assert!(
            !line.starts_with('/'),
            "an absolute path must never reach the output: {line}"
        );
    }
}

/// A name two documents answer to names neither of them.
///
/// The command prints every candidate — canonical key, title and
/// workspace-relative path — and exits non-zero. It never shows the body
/// of one of them.
#[test]
fn a_reference_two_documents_answer_to_names_both_and_exits_non_zero() {
    let tmp = migrated_workspace_with_two_prds();

    let output = forgeplan()
        .args(["get", "PRD-001"])
        .current_dir(tmp.path())
        .output()
        .expect("forgeplan get runs");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !output.status.success(),
        "an ambiguous reference must exit non-zero: {stderr}"
    );
    for expected in [
        "prd-alpha",
        "prd-beta",
        "Alpha",
        "Beta",
        "prds/PRD-001-alpha.md",
        "prds/PRD-001-beta.md",
    ] {
        assert!(
            stderr.contains(expected),
            "the refusal must name {expected:?}: {stderr}"
        );
    }
    assert!(
        !stdout.contains("Alpha body.") && !stdout.contains("Beta body."),
        "neither body may reach the output of a refusal: {stdout}"
    );
    assert!(
        !stderr.contains(tmp.path().to_string_lossy().as_ref()),
        "an absolute path must never reach the output: {stderr}"
    );
}

/// A name no document carries is a miss — and the miss says so.
#[test]
fn a_reference_no_document_carries_is_reported_as_a_miss() {
    let tmp = migrated_workspace_with_two_prds();

    let output = forgeplan()
        .args(["get", "PRD-404"])
        .current_dir(tmp.path())
        .output()
        .expect("forgeplan get runs");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "a miss must exit non-zero: {stderr}");
    assert!(
        stderr.contains("Artifact 'PRD-404' not found"),
        "the miss must say what it is: {stderr}"
    );
}

/// An unusable reference is not a miss.
///
/// A qualified reference names a document in another source; it is not a
/// selector this workspace can honour, and it must not be reported as a
/// document that simply is not there.
#[test]
fn an_unusable_reference_is_refused_as_such_and_not_as_a_miss() {
    let tmp = migrated_workspace_with_two_prds();

    let output = forgeplan()
        .args(["get", "PRD-001@other-source"])
        .current_dir(tmp.path())
        .output()
        .expect("forgeplan get runs");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "a refusal must exit non-zero: {stderr}");
    assert!(
        stderr.contains("is not a valid artifact reference"),
        "the refusal must say the reference is unusable: {stderr}"
    );
    assert!(
        !stderr.contains("not found"),
        "an unusable reference must not be reported as a miss: {stderr}"
    );
}

/// The graph's node id is the canonical key, and every node is labelled
/// with its display id and its title.
///
/// Two documents that share a display id stay two vertices — each with its
/// own title — so the graph never merges them into one.
#[test]
fn the_graph_keeps_two_documents_that_share_a_display_id_apart() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    std::fs::create_dir_all(tmp.path().join(".forgeplan/prds")).unwrap();
    std::fs::write(
        tmp.path().join(".forgeplan/prds/PRD-001-alpha.md"),
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Alpha\nslug: prd-alpha\n\
         links:\n  - target: prd-beta\n    relation: based_on\n---\n\nAlpha body.\n",
    )
    .unwrap();
    std::fs::write(
        tmp.path().join(".forgeplan/prds/PRD-001-beta.md"),
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Beta\nslug: prd-beta\n---\n\nBeta body.\n",
    )
    .unwrap();
    forgeplan()
        .arg("index")
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .arg("graph")
        .current_dir(tmp.path())
        .output()
        .expect("forgeplan graph runs");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains(r#"prd-alpha["PRD-001 · Alpha"]"#),
        "the first claimant must be its own vertex, labelled with its own title: {stdout}"
    );
    assert!(
        stdout.contains(r#"prd-beta["PRD-001 · Beta"]"#),
        "the second claimant must be its own vertex, labelled with its own title: {stdout}"
    );
    assert!(
        stdout.contains("prd-alpha -->|based_on| prd-beta"),
        "the edge must address the canonical keys: {stdout}"
    );

    // The JSON surface addresses the same vertices.
    let output = forgeplan()
        .args(["graph", "--json"])
        .current_dir(tmp.path())
        .output()
        .expect("forgeplan graph --json runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let edges = payload["edges"].as_array().expect("edges array");
    assert_eq!(edges.len(), 1, "one edge expected: {payload}");
    assert_eq!(edges[0]["from"], "prd-alpha");
    assert_eq!(edges[0]["to"], "prd-beta");
}

#[test]
fn migrated_link_by_key_keeps_both_identity_and_display_output() {
    let tmp = migrated_workspace_with_two_prds();
    let alpha = tmp.path().join(".forgeplan/prds/PRD-001-alpha.md");
    let beta = tmp.path().join(".forgeplan/prds/PRD-001-beta.md");
    let beta_before = std::fs::read(&beta).unwrap();
    let before = std::fs::read(&alpha).unwrap();
    let denied = forgeplan().args(["link", "PRD-001", "prd-beta", "--relation", "based_on"])
        .current_dir(tmp.path()).output().unwrap();
    assert!(!denied.status.success());
    assert_eq!(std::fs::read(&alpha).unwrap(), before);
    assert_eq!(std::fs::read(&beta).unwrap(), beta_before);
    let output = forgeplan().args(["link", "prd-alpha", "prd-beta", "--relation", "based_on"])
        .current_dir(tmp.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("PRD-001 --based_on--> PRD-001"));
    assert!(std::fs::read_to_string(&alpha).unwrap().contains("prd-beta"));
    assert_eq!(std::fs::read(&beta).unwrap(), beta_before);
}

#[test]
fn migrated_tag_and_claim_reject_ambiguous_alias_without_mutation() {
    let tmp = migrated_workspace_with_two_prds();
    let alpha = tmp.path().join(".forgeplan/prds/PRD-001-alpha.md");
    let beta = tmp.path().join(".forgeplan/prds/PRD-001-beta.md");
    let before_alpha = std::fs::read(&alpha).unwrap();
    let before_beta = std::fs::read(&beta).unwrap();
    for args in [
        vec!["tag", "PRD-001", "marker"],
        vec!["claim", "PRD-001", "--agent", "identity-test"],
        vec!["release", "PRD-001", "--agent", "identity-test"],
    ] {
        let output = forgeplan().args(&args).current_dir(tmp.path()).output().unwrap();
        assert!(!output.status.success(), "{args:?}");
        assert_eq!(std::fs::read(&alpha).unwrap(), before_alpha);
        assert_eq!(std::fs::read(&beta).unwrap(), before_beta);
    }
    let output = forgeplan().args(["tag", "prd-alpha", "marker"])
        .current_dir(tmp.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("PRD-001"));
    assert!(std::fs::read_to_string(&alpha).unwrap().contains("marker"));
    assert_eq!(std::fs::read(&beta).unwrap(), before_beta);
}

#[test]
fn migrated_progress_scopes_shared_alias_by_key() {
    let tmp = TempDir::new().unwrap();
    forgeplan().args(["init", "-y"]).current_dir(tmp.path()).assert().success();
    workspace_with_two_prds(tmp.path());
    std::fs::write(
        tmp.path().join(".forgeplan/prds/PRD-001-alpha.md"),
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Alpha\nslug: prd-alpha\n---\n\n- [x] Done\n",
    ).unwrap();
    std::fs::write(
        tmp.path().join(".forgeplan/prds/PRD-001-beta.md"),
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Beta\nslug: prd-beta\n---\n\n- [ ] Pending\n",
    ).unwrap();
    forgeplan().arg("index").current_dir(tmp.path()).assert().success();
    let output = forgeplan().args(["progress", "prd-beta", "--json"])
        .current_dir(tmp.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let entries = payload["progress"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["id"], "PRD-001");
    assert_eq!(entries[0]["title"], "Beta");
    assert_eq!(entries[0]["completed"], 0);
    let denied = forgeplan().args(["progress", "PRD-001", "--json"])
        .current_dir(tmp.path()).output().unwrap();
    assert!(!denied.status.success());
}

#[test]
fn legacy_claims_resolve_shared_alias_before_touching_claim_files() {
    let tmp = TempDir::new().unwrap();
    forgeplan().args(["init", "-y"]).current_dir(tmp.path()).assert().success();
    workspace_with_two_prds(tmp.path());
    let claim_dir = tmp.path().join(".forgeplan/claims");
    let denied = forgeplan().args(["claim", "PRD-001", "--agent", "identity-test"])
        .current_dir(tmp.path()).output().unwrap();
    assert!(!denied.status.success());
    assert!(!claim_dir.exists() || std::fs::read_dir(&claim_dir).unwrap().next().is_none());
    let claimed = forgeplan().args(["claim", "prd-alpha", "--agent", "identity-test"])
        .current_dir(tmp.path()).output().unwrap();
    assert!(claimed.status.success(), "{}", String::from_utf8_lossy(&claimed.stderr));
    let denied = forgeplan().args(["release", "PRD-001", "--agent", "identity-test"])
        .current_dir(tmp.path()).output().unwrap();
    assert!(!denied.status.success());
    let released = forgeplan().args(["release", "prd-alpha", "--agent", "identity-test"])
        .current_dir(tmp.path()).output().unwrap();
    assert!(released.status.success(), "{}", String::from_utf8_lossy(&released.stderr));
    assert!(String::from_utf8_lossy(&released.stdout).contains("PRD-001"));
}

#[test]
fn legacy_create_reports_committed_when_semantic_model_is_unavailable() {
    let tmp = TempDir::new().unwrap();
    forgeplan().args(["init", "-y"]).current_dir(tmp.path()).assert().success();
    let ws = tmp.path().join(".forgeplan");
    let mut config = forgeplan_core::workspace::load_config(&ws).unwrap();
    config.embedding = Some(forgeplan_core::config::types::EmbeddingConfig {
        model: "not-a-real-model".into(),
        chunk_size: 2000,
    });
    std::fs::write(ws.join("config.yaml"), serde_yaml::to_string(&config).unwrap()).unwrap();
    let result = forgeplan()
        .args(["new", "prd", "Unavailable semantic model"])
        .env_remove("FORGEPLAN_EMBEDDING_MODEL")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr)
        .contains("canonical committed, semantic backlog pending"));
    assert!(!String::from_utf8_lossy(&result.stdout).contains("Created:"));
    assert!(std::fs::read_dir(ws.join("prds")).unwrap().any(|entry| {
        let path = entry.unwrap().path();
        std::fs::read_to_string(path)
            .map(|body| body.contains("Unavailable semantic model"))
            .unwrap_or(false)
    }));
}
