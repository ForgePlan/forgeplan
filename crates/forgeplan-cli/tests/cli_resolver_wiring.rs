//! CLI behavior for commands that accept display IDs and canonical keys.
//! Mutations are verified through persisted state, not hint or error wording.

use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Initialise a workspace and create one PRD; return the temp dir.
/// The first PRD is always assigned `PRD-001` post-merge.
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

/// Read the canonical slug for a PRD via `forgeplan get --json`. The slug
/// is the canonical body frontmatter slug (Phase 1+ artifact contract).
fn slug_for(workspace: &Path, id_or_slug: &str) -> String {
    let out = forgeplan()
        .args(["get", id_or_slug, "--json"])
        .current_dir(workspace)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    json["slug"]
        .as_str()
        .expect("forgeplan get --json must surface the slug for Phase 1+ artifacts")
        .to_string()
}

fn get_json(workspace: &Path, id_or_slug: &str) -> Value {
    let out = forgeplan()
        .args(["get", id_or_slug, "--json"])
        .current_dir(workspace)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&out).expect("forgeplan get --json must emit valid JSON")
}

fn list_json(workspace: &Path) -> Value {
    let out = forgeplan()
        .args(["list", "--json"])
        .current_dir(workspace)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&out).expect("forgeplan list --json must emit valid JSON")
}

fn claims_json(workspace: &Path) -> Value {
    let out = forgeplan()
        .args(["claims", "--json"])
        .current_dir(workspace)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&out).expect("forgeplan claims --json must emit valid JSON")
}

// ────────────────────────────────────────────────────────────────────
// Update by display id or canonical key.
// ────────────────────────────────────────────────────────────────────

#[test]
fn update_accepts_slug_form() {
    let dir = workspace_with_one_prd("Update Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    forgeplan()
        .args(["update", &slug, "--title", "Updated Via Slug"])
        .current_dir(dir.path())
        .assert()
        .success();

    // The accepted canonical key selects the existing artifact, and the
    // updated title is observable through that same key.
    let json = get_json(dir.path(), &slug);
    assert_eq!(
        json["title"].as_str(),
        Some("Updated Via Slug"),
        "post-update title must reflect the persisted value"
    );
}

#[test]
fn update_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Update Display Test");

    forgeplan()
        .args(["update", "PRD-001", "--title", "Updated Via Display"])
        .current_dir(dir.path())
        .assert()
        .success();

    let json = get_json(dir.path(), "PRD-001");
    assert_eq!(
        json["title"].as_str(),
        Some("Updated Via Display"),
        "post-update title must reflect new value"
    );
}

// ────────────────────────────────────────────────────────────────────
// ────────────────────────────────────────────────────────────────────
// 4. Delete by either accepted reference.
// ────────────────────────────────────────────────────────────────────

#[test]
fn delete_accepts_slug_form() {
    let dir = workspace_with_one_prd("Delete Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    forgeplan()
        .args(["delete", &slug, "--yes"])
        .current_dir(dir.path())
        .assert()
        .success();

    // The artifact no longer appears after deletion by canonical key.
    let listed = list_json(dir.path());
    let arr = listed.as_array().expect("list --json returns an array");
    assert!(
        arr.iter().all(|a| a["id"].as_str() != Some("PRD-001")),
        "after delete via slug, PRD-001 must not appear in list — got: {listed:?}"
    );
}

#[test]
fn delete_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Delete Display Test");

    forgeplan()
        .args(["delete", "PRD-001", "--yes"])
        .current_dir(dir.path())
        .assert()
        .success();

    let listed = list_json(dir.path());
    let arr = listed.as_array().expect("list --json returns an array");
    assert!(
        arr.iter().all(|a| a["id"].as_str() != Some("PRD-001")),
        "after delete via display id, PRD-001 must not appear in list — got: {listed:?}"
    );
}

// ────────────────────────────────────────────────────────────────────
// 5. Renewal persists the requested lifecycle and expiry values.
// ────────────────────────────────────────────────────────────────────

#[test]
fn renew_accepts_slug_form() {
    let dir = workspace_with_one_prd("Renew Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    forgeplan()
        .args(["renew", &slug, "--reason", "test", "--until", "2099-12-31"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Confirm the renewed record and expiry are persisted.
    let json = get_json(dir.path(), &slug);
    assert_eq!(
        json["status"].as_str(),
        Some("active"),
        "renewal must set the active status"
    );
    assert_eq!(
        json["valid_until"].as_str(),
        Some("2099-12-31"),
        "post-renew valid_until must reflect the --until argument"
    );
}

#[test]
fn renew_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Renew Display Test");

    forgeplan()
        .args([
            "renew",
            "PRD-001",
            "--reason",
            "test",
            "--until",
            "2099-12-31",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let json = get_json(dir.path(), "PRD-001");
    assert_eq!(json["status"].as_str(), Some("active"));
    assert_eq!(json["valid_until"].as_str(), Some("2099-12-31"));
}

// ────────────────────────────────────────────────────────────────────
// 10. F-G-R lookup returns the selected artifact's structured score.
#[test]
fn fgr_accepts_slug_form() {
    let dir = workspace_with_one_prd("FGR Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");
    let output = forgeplan()
        .args(["fgr", &slug, "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    let results = value["results"].as_array().expect("results array");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["id"], "PRD-001");
    assert!(results[0]["overall"].is_number());
    assert!(results[0]["grade"].is_string());
}

#[test]
fn fgr_accepts_display_id_form() {
    let dir = workspace_with_one_prd("FGR Display Test");
    let output = forgeplan()
        .args(["fgr", "PRD-001", "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    let results = value["results"].as_array().expect("results array");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["id"], "PRD-001");
    assert!(results[0]["overall"].is_number());
    assert!(results[0]["grade"].is_string());
}

// ────────────────────────────────────────────────────────────────────
// Claims persist the display ID and agent identity.
// ────────────────────────────────────────────────────────────────────

#[test]
fn claim_accepts_slug_form() {
    let dir = workspace_with_one_prd("Claim Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    forgeplan()
        .args(["claim", &slug, "--agent", "test-agent"])
        .current_dir(dir.path())
        .assert()
        .success();

    // The claim row retains the artifact's display ID and agent.
    let claims = claims_json(dir.path());
    let arr = claims["claims"]
        .as_array()
        .expect("claims --json must contain a `claims` array");
    let found = arr.iter().any(|c| {
        c["id"].as_str() == Some("PRD-001") && c["agent_id"].as_str() == Some("test-agent")
    });
    assert!(
        found,
        "post-claim claims store must contain canonical id PRD-001 + agent test-agent — got: {claims:?}"
    );
}

#[test]
fn claim_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Claim Display Test");

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "test-agent"])
        .current_dir(dir.path())
        .assert()
        .success();

    let claims = claims_json(dir.path());
    let arr = claims["claims"]
        .as_array()
        .expect("claims --json must contain a `claims` array");
    let found = arr.iter().any(|c| {
        c["id"].as_str() == Some("PRD-001") && c["agent_id"].as_str() == Some("test-agent")
    });
    assert!(
        found,
        "post-claim claims store must contain PRD-001 + test-agent — got: {claims:?}"
    );
}

// ────────────────────────────────────────────────────────────────────
// 12. Release removes the corresponding persisted claim.
// ────────────────────────────────────────────────────────────────────

#[test]
fn release_accepts_slug_form() {
    let dir = workspace_with_one_prd("Release Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    forgeplan()
        .args(["claim", &slug, "--agent", "test-agent"])
        .current_dir(dir.path())
        .assert()
        .success();

    forgeplan()
        .args(["release", &slug, "--agent", "test-agent"])
        .current_dir(dir.path())
        .assert()
        .success();

    // The claim row is absent after release by canonical key.
    let claims = claims_json(dir.path());
    let arr = claims["claims"]
        .as_array()
        .expect("claims --json must contain a `claims` array");
    assert!(
        arr.iter().all(|c| c["id"].as_str() != Some("PRD-001")),
        "post-release claims store must not contain PRD-001 — got: {claims:?}"
    );
}

#[test]
fn release_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Release Display Test");

    forgeplan()
        .args(["claim", "PRD-001", "--agent", "test-agent"])
        .current_dir(dir.path())
        .assert()
        .success();

    forgeplan()
        .args(["release", "PRD-001", "--agent", "test-agent"])
        .current_dir(dir.path())
        .assert()
        .success();

    let claims = claims_json(dir.path());
    let arr = claims["claims"]
        .as_array()
        .expect("claims --json must contain a `claims` array");
    assert!(
        arr.iter().all(|c| c["id"].as_str() != Some("PRD-001")),
        "post-release claims store must not contain PRD-001 — got: {claims:?}"
    );
}

// ────────────────────────────────────────────────────────────────────
// 13. import — preserve the payload's display id while using its slug as
//     canonical identity. The imported title must update the same record.
// ────────────────────────────────────────────────────────────────────

#[test]
fn import_accepts_slug_in_payload() {
    let dir = workspace_with_one_prd("Import Slug Payload");
    let slug = slug_for(dir.path(), "PRD-001");

    let payload = serde_json::json!({
        "artifacts": [{
            "id": "PRD-001",
            "canonical_key": &slug,
            "kind": "prd",
            "status": "draft",
            "title": "Re-imported via slug",
            "body": "## Goal\n\nRe-import smoke test.\n",
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

    // The payload keeps PRD-001 as its display id and supplies the slug as
    // canonical_key; verify the existing record received the update.
    let json = get_json(dir.path(), "PRD-001");
    assert_eq!(
        json["title"].as_str(),
        Some("Re-imported via slug"),
        "import by canonical key must update the existing artifact"
    );
}

#[test]
fn import_accepts_display_id_in_payload() {
    let dir = workspace_with_one_prd("Import Display Payload");

    let payload = serde_json::json!({
        "artifacts": [{
            "id": "PRD-001",
            "kind": "prd",
            "status": "draft",
            "title": "Re-imported via display id",
            "body": "## Goal\n\nRe-import smoke test.\n",
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

    let json = get_json(dir.path(), "PRD-001");
    assert_eq!(
        json["title"].as_str(),
        Some("Re-imported via display id"),
        "import via display id must overwrite PRD-001 title"
    );
}
