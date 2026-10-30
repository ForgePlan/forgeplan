//! The one document set, seen from the CLI side.
//!
//! SPEC-009 freezes the resolution order and the error taxonomy. This file
//! pins the CLI half of the contract that came with moving the document set
//! into core:
//!
//! * the CLI and the MCP server decide over the SAME document set, so the
//!   same selector against the same workspace yields the same document — or
//!   the same failure code;
//! * a name two documents answer to names NEITHER of them: the refusal
//!   names every candidate by its canonical key, its title and its
//!   workspace-relative path, and no absolute path reaches the output;
//! * an edge endpoint is decided by the same resolver — the source must
//!   name exactly one document, the target may stay a forward reference,
//!   and neither may be an ambiguous pick;
//! * the CLI and the MCP layer read the documents the ONE workspace scan
//!   found, so a document is named the same way on both surfaces.

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

/// Write one document into a workspace.
fn write_document(workspace: &Path, relative: &str, display: &str, slug: &str, title: &str) {
    let path = workspace.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        format!(
            "---\nid: {display}\nkind: prd\nstatus: draft\ntitle: {title}\nslug: {slug}\n---\n\n{title} body.\n"
        ),
    )
    .unwrap();
}

/// A workspace whose documents are published to the index.
///
/// Two of the documents carry the SAME display id — their canonical keys
/// differ, so neither is an identity collision, and the display id is the
/// alias that must not pick a winner. The third document has its own id.
fn workspace_with_two_prds_sharing_one_display_id() -> TempDir {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    let workspace = tmp.path().join(".forgeplan");
    write_document(
        &workspace,
        "prds/PRD-001-alpha.md",
        "PRD-001",
        "prd-alpha",
        "Alpha",
    );
    write_document(
        &workspace,
        "prds/PRD-001-beta.md",
        "PRD-001",
        "prd-beta",
        "Beta",
    );
    write_document(
        &workspace,
        "prds/PRD-002-gamma.md",
        "PRD-002",
        "prd-gamma",
        "Gamma",
    );
    forgeplan()
        .arg("index")
        .current_dir(tmp.path())
        .assert()
        .success();
    tmp
}

/// The CLI and the MCP server decide over the same document set — so they
/// name the same document, or they report the same failure.
///
/// The two surfaces call the same resolver, so this is a comparison of two
/// call sites over ONE list of documents, not of two algorithms.
#[tokio::test]
async fn the_cli_and_the_mcp_server_agree_over_the_same_workspace_document_set() {
    let tmp = workspace_with_two_prds_sharing_one_display_id();
    let workspace = tmp.path().join(".forgeplan");

    let documents = forgeplan_core::artifact::documents::workspace_candidates(&workspace)
        .await
        .expect("the workspace scan reads every document");
    assert_eq!(
        documents.len(),
        3,
        "the document set holds every document of the workspace: {documents:?}"
    );

    for selector in [
        "PRD-002",
        "prd-gamma",
        "prd-74",
        "PRD-001",
        "PRD-404",
        "prd-gamma@other-source",
        "",
    ] {
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

/// The document set names every document by its canonical key, its display
/// id, its title and its WORKSPACE-RELATIVE path — never by an absolute
/// path.
#[tokio::test]
async fn the_document_set_names_every_document_without_an_absolute_path() {
    let tmp = workspace_with_two_prds_sharing_one_display_id();
    let workspace = tmp.path().join(".forgeplan");

    let documents = forgeplan_core::artifact::documents::workspace_candidates(&workspace)
        .await
        .expect("the workspace scan reads every document");
    let alpha = documents
        .iter()
        .find(|candidate| candidate.canonical_key == "prd-alpha")
        .expect("the alpha document is in the set");
    assert_eq!(alpha.display_id, "PRD-001");
    assert_eq!(alpha.title, "Alpha");
    assert_eq!(
        alpha.relative_path,
        PathBuf::from("prds/PRD-001-alpha.md"),
        "the path is relative to the workspace"
    );
    for candidate in &documents {
        assert!(
            !candidate.relative_path.is_absolute(),
            "an absolute path must never reach a candidate: {:?}",
            candidate.relative_path
        );
    }
}

/// The refusal names every candidate by its canonical key, its title and
/// its workspace-relative path — and no absolute path.
///
/// This is what the command prints when a name two documents answer to is
/// refused.
#[tokio::test]
async fn the_refusal_names_every_candidate_by_key_title_and_relative_path() {
    let tmp = workspace_with_two_prds_sharing_one_display_id();
    let workspace = tmp.path().join(".forgeplan");

    let documents = forgeplan_core::artifact::documents::workspace_candidates(&workspace)
        .await
        .expect("the workspace scan reads every document");
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

/// A source two documents answer to names NEITHER of them.
///
/// The edge is refused before anything is written: the command exits
/// non-zero, names both claimants, and the source document keeps no link.
#[test]
fn an_ambiguous_source_is_refused_and_no_link_is_written() {
    let tmp = workspace_with_two_prds_sharing_one_display_id();

    let output = forgeplan()
        .args(["link", "PRD-001", "PRD-002", "--relation", "based_on"])
        .current_dir(tmp.path())
        .output()
        .expect("forgeplan link runs");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "an ambiguous source must be refused: {stderr}"
    );
    for expected in ["prd-alpha", "prd-beta"] {
        assert!(
            stderr.contains(expected),
            "the refusal must name {expected:?}: {stderr}"
        );
    }
    assert!(
        !stderr.contains("not found"),
        "an ambiguity must not be reported as a miss: {stderr}"
    );

    // Nothing was written: the source document carries no link.
    let source =
        std::fs::read_to_string(tmp.path().join(".forgeplan/prds/PRD-001-alpha.md")).unwrap();
    assert!(
        !source.contains("PRD-002"),
        "the refused edge must not be written: {source}"
    );
}

/// A target two documents answer to names NEITHER of them.
///
/// A forward reference is legal — a document that does not exist yet may be
/// linked to — but an AMBIGUITY is two documents that already exist and
/// both answer to the name. Writing to either of them is the silent pick
/// the resolver exists to prevent.
#[test]
fn an_ambiguous_target_is_refused_and_no_link_is_written() {
    let tmp = workspace_with_two_prds_sharing_one_display_id();

    let output = forgeplan()
        .args(["link", "PRD-002", "PRD-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .output()
        .expect("forgeplan link runs");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "an ambiguous target must be refused: {stderr}"
    );
    for expected in ["prd-alpha", "prd-beta"] {
        assert!(
            stderr.contains(expected),
            "the refusal must name {expected:?}: {stderr}"
        );
    }

    let source =
        std::fs::read_to_string(tmp.path().join(".forgeplan/prds/PRD-002-gamma.md")).unwrap();
    assert!(
        !source.contains("PRD-001"),
        "the refused edge must not be written: {source}"
    );
}

/// A target that names no document is a legal forward reference.
///
/// The edge is written down under the text the operator declared, verbatim:
/// the document may still be created.
#[test]
fn a_forward_reference_target_is_written_down_verbatim() {
    let tmp = workspace_with_two_prds_sharing_one_display_id();

    let output = forgeplan()
        .args(["link", "PRD-002", "PRD-404", "--relation", "based_on"])
        .current_dir(tmp.path())
        .output()
        .expect("forgeplan link runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "a forward reference is a legal edge: {stderr}"
    );
    assert!(
        stdout.contains("PRD-404"),
        "the declared target must be kept verbatim: {stdout}"
    );
}

/// An unlink whose source two documents answer to is refused — an edge
/// belongs to ONE document, and the command must not guess which.
#[test]
fn an_ambiguous_unlink_is_refused() {
    let tmp = workspace_with_two_prds_sharing_one_display_id();

    let output = forgeplan()
        .args(["unlink", "PRD-001", "PRD-002", "--relation", "based_on"])
        .current_dir(tmp.path())
        .output()
        .expect("forgeplan unlink runs");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "an ambiguous unlink must be refused: {stderr}"
    );
    for expected in ["prd-alpha", "prd-beta"] {
        assert!(
            stderr.contains(expected),
            "the refusal must name {expected:?}: {stderr}"
        );
    }
}

/// The candidates the CLI refuses with are the candidates of the ONE
/// document set — the same list the resolver decided over.
#[test]
fn the_refused_candidates_are_the_candidates_of_the_one_document_set() {
    let documents = vec![
        document("prd-alpha", "PRD-001", "Alpha", "prds/PRD-001-alpha.md"),
        document("prd-beta", "PRD-001", "Beta", "prds/PRD-001-beta.md"),
    ];
    let outcome = forgeplan::commands::common::resolve_in_document_set("PRD-001", &documents);
    let failure =
        forgeplan::commands::common::resolution_failure(&outcome, "PRD-001", &documents);

    let named: Vec<String> = failure
        .candidates
        .iter()
        .map(|candidate| candidate.canonical_key.clone())
        .collect();
    assert_eq!(
        named,
        vec!["prd-alpha".to_string(), "prd-beta".to_string()],
        "the refusal names every claimant of the one document set"
    );
    assert_eq!(failure.code, "ambiguous");
    assert_ne!(
        failure.code, "not_found",
        "an ambiguity is not a miss"
    );
}
/// Ordinary CLI creation publishes each artifact under the key minted for
/// that document, even when the display id and title repeat.
#[tokio::test]
async fn cli_new_publishes_distinct_canonical_key_files_for_duplicate_titles() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    for _ in 0..2 {
        let output = forgeplan()
            .args(["new", "prd", "Auth System", "--allow-duplicate"])
            .current_dir(tmp.path())
            .output()
            .expect("forgeplan new runs");
        assert!(
            output.status.success(),
            "ordinary creation must publish the artifact: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let workspace = tmp.path().join(".forgeplan");
    let documents = forgeplan_core::artifact::documents::workspace_candidates(&workspace)
        .await
        .expect("the workspace scan reads the created documents");
    assert_eq!(documents.len(), 2, "both created documents survive");
    assert_ne!(
        documents[0].canonical_key, documents[1].canonical_key,
        "each created document receives its own key"
    );
    let mut display_ids: Vec<_> = documents
        .iter()
        .map(|document| document.display_id.as_str())
        .collect();
    display_ids.sort_unstable();
    assert_eq!(
        display_ids,
        ["PRD-001", "PRD-002"],
        "the display ids remain the allocator's sequential aliases"
    );
    for document in &documents {
        assert_eq!(
            document.relative_path.file_name().unwrap().to_str().unwrap(),
            format!("{}.md", document.canonical_key),
            "ordinary creation writes the document to its key-named file"
        );
        let text = std::fs::read_to_string(workspace.join(&document.relative_path)).unwrap();
        let (frontmatter, _) =
            forgeplan_core::artifact::frontmatter::parse_frontmatter(&text).unwrap();
        assert_eq!(
            frontmatter
                .get("canonical_key")
                .and_then(serde_yaml::Value::as_str),
            Some(document.canonical_key.as_str()),
            "the stored document carries its minted key"
        );
    }
}
