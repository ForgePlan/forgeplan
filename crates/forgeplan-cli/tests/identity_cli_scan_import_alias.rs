//! A display id may be shared; the canonical key is the identity.
//!
//! SPEC-009 V3/V6 and ADR-030: two documents may answer to one display
//! alias as long as their canonical keys differ. `forgeplan scan-import`
//! therefore keeps BOTH documents, and its report must name the key of each
//! one instead of telling the operator to delete a file. A report that asks
//! for a deletion makes the operator lose a document the model keeps.
//!
//! The test drives the real binary in a temporary workspace.

use assert_cmd::Command;
use serde_json::Value;
use std::path::Path;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// A workspace with two source documents that declare the SAME display id
/// and DIFFERENT canonical keys — the legal alias-sharing shape.
fn shared_alias_workspace() -> (TempDir, String, String) {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let docs = tmp.path().join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    for (name, slug, title) in [
        ("alpha.md", "prd-alpha", "Alpha"),
        ("beta.md", "prd-beta", "Beta"),
    ] {
        std::fs::write(
            docs.join(name),
            format!(
                "---\nid: PRD-001\nslug: {slug}\nkind: prd\nstatus: draft\ntitle: {title}\n---\n\n{title} body.\n"
            ),
        )
        .unwrap();
    }
    (tmp, "prd-alpha".to_string(), "prd-beta".to_string())
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn scan_import_keeps_two_documents_that_share_a_display_id() {
    let (tmp, alpha, beta) = shared_alias_workspace();
    let alpha_source = tmp.path().join("docs/alpha.md");
    let beta_source = tmp.path().join("docs/beta.md");
    let alpha_before = read(&alpha_source);
    let beta_before = read(&beta_source);

    let output = forgeplan()
        .args(["scan-import"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "scan-import failed: {}{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    // Both documents are imported: a shared alias is not a collision.
    assert!(
        stdout.contains("2 imported"),
        "both documents must be imported: {stdout}"
    );

    // The report names the canonical key of every file in the group, so the
    // operator can address each document exactly.
    assert!(
        stdout.contains(&alpha) && stdout.contains(&beta),
        "the report must name both canonical keys: {stdout}"
    );

    // The report must never ask for a deletion: both documents are legal.
    assert!(
        !stdout.contains("keep ONE file") && !stdout.contains("delete the stale duplicate"),
        "a shared alias must not produce destructive advice: {stdout}"
    );

    // Data preservation: neither source file was moved, rewritten or removed.
    assert_eq!(
        read(&alpha_source),
        alpha_before,
        "the alpha source changed"
    );
    assert_eq!(read(&beta_source), beta_before, "the beta source changed");
}

#[test]
fn each_shared_alias_document_is_addressed_by_its_own_canonical_key() {
    let (tmp, alpha, beta) = shared_alias_workspace();
    forgeplan()
        .args(["scan-import"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // The alias itself names NEITHER document — the resolver refuses it and
    // names both candidates.
    let ambiguous = forgeplan()
        .args(["get", "PRD-001"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert_eq!(
        ambiguous.status.code(),
        Some(5),
        "a shared display id must be refused as ambiguous: {}{}",
        String::from_utf8_lossy(&ambiguous.stdout),
        String::from_utf8_lossy(&ambiguous.stderr)
    );
    let stderr = String::from_utf8_lossy(&ambiguous.stderr);
    assert!(
        stderr.contains(&alpha) && stderr.contains(&beta),
        "the refusal must name both candidates: {stderr}"
    );

    // Each key names exactly its own document, under the shared display id.
    for (key, title) in [(&alpha, "Alpha"), (&beta, "Beta")] {
        let output = forgeplan()
            .args(["get", key.as_str(), "--json"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "get {key} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(payload["canonical_key"], key.as_str());
        assert_eq!(payload["id"], "PRD-001");
        assert_eq!(payload["title"], title);
    }
}
