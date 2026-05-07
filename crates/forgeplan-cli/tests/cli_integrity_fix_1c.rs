//! PROB-060 Phase 2 Round-1 audit fix-1c — CLI/MCP integrity guards.
//!
//! Integration tests that exercise the HIGH-3 / HIGH-6 closures end-to-end
//! through the real `forgeplan` binary. These complement the unit tests in
//! `forgeplan-core` / `forgeplan-mcp` by catching regressions at the
//! command-surface boundary (where unit tests can't observe the actual
//! exit code, error message, and side effects on the LanceDB row).
//!
//! Coverage:
//!  - HIGH-6 — `forgeplan import --force` with a kind-mismatched payload
//!    must refuse to corrupt the existing artifact.
//!  - Closure validation that a clean payload (matching kind) still
//!    succeeds — guards against false positives in the new check.

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn workspace_with_one_prd(title: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(dir.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", title])
        .current_dir(dir.path())
        .assert()
        .success();
    dir
}

// ────────────────────────────────────────────────────────────────────
// HIGH-6: import --force must not silently rewrite kind on an existing
// artifact. Previously, payload `{ "id": "PRD-001", "kind": "rfc" }`
// resolved to the existing PRD-001 row, deleted it + projection, and
// recreated с kind="rfc" — silent data corruption (CWE-639).
// ────────────────────────────────────────────────────────────────────

#[test]
fn import_rejects_kind_mismatch_on_force() {
    let dir = workspace_with_one_prd("Existing PRD");

    // Payload claims `id="PRD-001"` but `kind="rfc"`. Resolver maps
    // PRD-001 → existing PRD row. The new HIGH-6 check must refuse
    // before the destructive delete.
    let payload = serde_json::json!({
        "artifacts": [{
            "id": "PRD-001",
            "kind": "rfc",
            "status": "draft",
            "title": "Pretender",
            "body": "## Goal\n\nKind mismatch.\n",
            "depth": "standard",
            "tags": [],
        }],
        "relations": []
    });
    let payload_path = dir.path().join("payload.json");
    std::fs::write(&payload_path, payload.to_string()).unwrap();

    let output = forgeplan()
        .args(["import", payload_path.to_str().unwrap(), "--force"])
        .current_dir(dir.path())
        .output()
        .expect("forgeplan import");
    assert!(
        !output.status.success(),
        "import must fail when payload kind mismatches existing artifact"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Import would change kind") || stderr.contains("would change kind"),
        "expected HIGH-6 kind-mismatch message in stderr, got:\n{stderr}"
    );

    // Existing artifact must remain a PRD post-failure.
    let get_out = forgeplan()
        .args(["get", "PRD-001", "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&get_out).unwrap();
    assert_eq!(
        json["kind"].as_str(),
        Some("prd"),
        "existing artifact must stay a PRD after rejected import"
    );
    assert_eq!(
        json["title"].as_str(),
        Some("Existing PRD"),
        "existing artifact body/title must not have been overwritten"
    );
}

#[test]
fn import_rejects_kind_mismatch_via_slug_resolution() {
    // Same threat model but the malicious payload uses the slug form
    // `prd-existing-prd`. The resolver maps slug → PRD-001 *before* the
    // HIGH-6 check, so the guard must still fire.
    let dir = workspace_with_one_prd("Existing PRD");

    // Read the slug from the canonical row.
    let get_out = forgeplan()
        .args(["get", "PRD-001", "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&get_out).unwrap();
    let slug = json["slug"]
        .as_str()
        .expect("PRD-001 should have a slug")
        .to_string();

    let payload = serde_json::json!({
        "artifacts": [{
            "id": &slug,
            "kind": "adr",
            "status": "draft",
            "title": "Pretender via slug",
            "body": "## Decision\n\nKind mismatch via slug.\n",
            "depth": "standard",
            "tags": [],
        }],
        "relations": []
    });
    let payload_path = dir.path().join("payload.json");
    std::fs::write(&payload_path, payload.to_string()).unwrap();

    let output = forgeplan()
        .args(["import", payload_path.to_str().unwrap(), "--force"])
        .current_dir(dir.path())
        .output()
        .expect("forgeplan import");
    assert!(
        !output.status.success(),
        "import must fail when slug-resolved id has kind mismatch"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("would change kind"),
        "expected HIGH-6 message, got:\n{stderr}"
    );
}

#[test]
fn import_accepts_matching_kind_on_force() {
    // Closure check: the new guard must NOT block a legitimate
    // re-import that preserves the kind.
    let dir = workspace_with_one_prd("Existing PRD");

    let payload = serde_json::json!({
        "artifacts": [{
            "id": "PRD-001",
            "kind": "prd",
            "status": "draft",
            "title": "Re-imported with same kind",
            "body": "## Goal\n\nLegitimate re-import.\n",
            "depth": "standard",
            "tags": [],
        }],
        "relations": []
    });
    let payload_path = dir.path().join("payload.json");
    std::fs::write(&payload_path, payload.to_string()).unwrap();

    forgeplan()
        .args(["import", payload_path.to_str().unwrap(), "--force"])
        .current_dir(dir.path())
        .assert()
        .success();
}

#[test]
fn import_accepts_novel_id_when_no_existing_artifact() {
    // The HIGH-6 check is gated on `existing.is_some()`; novel ids that
    // don't resolve must still flow through unchanged.
    let dir = workspace_with_one_prd("Existing PRD");

    // Use a completely new id that won't resolve to anything.
    let payload = serde_json::json!({
        "artifacts": [{
            "id": "RFC-999",
            "kind": "rfc",
            "status": "draft",
            "title": "Brand new RFC",
            "body": "## Proposal\n\nNew rfc.\n",
            "depth": "standard",
            "tags": [],
        }],
        "relations": []
    });
    let payload_path = dir.path().join("payload.json");
    std::fs::write(&payload_path, payload.to_string()).unwrap();

    forgeplan()
        .args(["import", payload_path.to_str().unwrap()])
        .current_dir(dir.path())
        .assert()
        .success();
}
