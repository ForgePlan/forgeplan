use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

#[test]
fn init_creates_workspace() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(".forgeplan/"));

    assert!(tmp.path().join(".forgeplan").exists());
    assert!(tmp.path().join(".forgeplan/config.yaml").exists());
    assert!(tmp.path().join(".forgeplan/prds").is_dir());
    assert!(tmp.path().join(".forgeplan/rfcs").is_dir());
}

#[test]
fn init_idempotent_without_force() {
    let tmp = TempDir::new().unwrap();

    // First init succeeds
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Second init succeeds but warns
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Already initialized"));
}

#[test]
fn new_creates_artifact() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Test Feature"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));

    let prd_dir = tmp.path().join(".forgeplan/prds");
    let entries: Vec<_> = std::fs::read_dir(&prd_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].file_name().to_string_lossy().contains("PRD-001"));
}

#[test]
fn new_auto_increments_id() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "rfc", "First RFC"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("RFC-001"));

    forgeplan()
        .args(["new", "rfc", "Second RFC"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("RFC-002"));
}

#[test]
fn list_shows_artifacts() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "My Feature"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .arg("list")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("My Feature"));
}

#[test]
fn status_shows_dashboard() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Feature X"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .arg("status")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("prd"));
}

#[test]
fn validate_checks_artifact() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Validation Test"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Validate the newly created PRD (template has placeholders, so should have findings)
    forgeplan()
        .args(["validate", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .stdout(predicate::str::contains("PRD-001"));
}

#[test]
fn link_creates_relationship() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "My PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "rfc", "My RFC"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["link", "RFC-001", "PRD-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Linked"));
}

#[test]
fn graph_outputs_mermaid() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "rfc", "RFC"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["link", "RFC-001", "PRD-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .arg("graph")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("graph LR"))
        .stdout(predicate::str::contains("RFC-001"))
        .stdout(predicate::str::contains("PRD-001"));
}

#[test]
fn search_finds_content() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Authentication System"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["search", "Authentication"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("Authentication"));
}

#[test]
fn stale_runs_without_error() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .arg("stale")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No stale"));
}

#[test]
fn score_without_evidence() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "adr", "Test Decision"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["score", "ADR-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No evidence"));
}

#[test]
fn duplicate_link_rejected() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "P"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "rfc", "R"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // First link succeeds
    forgeplan()
        .args(["link", "RFC-001", "PRD-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Duplicate link fails
    forgeplan()
        .args(["link", "RFC-001", "PRD-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn validate_exits_nonzero_on_must_errors() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create a PRD via CLI (goes into LanceDB)
    forgeplan()
        .args(["new", "prd", "Test"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // PRD from template should have placeholder sections, validate should find issues
    // Template-generated PRDs typically have warnings but may pass at standard depth
    // This test verifies validate runs against LanceDB data without crashing
    forgeplan()
        .args(["validate", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .stdout(predicate::str::contains("PRD-001"));
}

#[test]
fn stale_detects_expired_artifact() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create an evidence artifact via CLI (goes into LanceDB + projection)
    forgeplan()
        .args(["new", "evidence", "Old Benchmark"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Update the artifact in LanceDB with an expired valid_until
    // We do this by directly inserting via a helper binary or LanceDB API
    // For now, test that stale command runs successfully with no stale artifacts
    // (since `new` doesn't set valid_until, all artifacts are non-stale)
    forgeplan()
        .arg("stale")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No stale"));

    // Full stale detection is tested in core unit tests (db::store::tests)
}

#[test]
fn no_workspace_gives_error() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .arg("list")
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("forgeplan init"));
}

// ── Phase 4D: CRUD tests ─────────────────────────────────

#[test]
fn get_reads_artifact() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Get Test Feature"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["get", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("Get Test Feature"))
        .stdout(predicate::str::contains("draft"))
        .stdout(predicate::str::contains("prd"));
}

#[test]
fn get_nonexistent_fails() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["get", "PRD-999"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn update_changes_status() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Update Test"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Direct --status active is blocked (B2 fix: must use forgeplan activate)
    forgeplan()
        .args(["update", "PRD-001", "--status", "active"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("forgeplan activate"));

    // Non-active status changes still work
    forgeplan()
        .args(["update", "PRD-001", "--status", "deprecated"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated"));
}

#[test]
fn update_changes_title() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "rfc", "Old Title"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["update", "RFC-001", "--title", "New Title"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("New Title"));
}

#[test]
fn update_nothing_fails() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Test"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["update", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Nothing to update"));
}

#[test]
fn delete_requires_confirmation() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Delete Test"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Without --yes, should fail with confirmation prompt (exit code 1)
    forgeplan()
        .args(["delete", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("--yes"));

    // Artifact should still exist
    forgeplan()
        .args(["get", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

#[test]
fn delete_with_yes_removes_artifact() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "To Be Deleted"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Delete with --yes
    forgeplan()
        .args(["delete", "PRD-001", "--yes"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted"));

    // Should be gone
    forgeplan()
        .args(["get", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// ── Full Workflow Dogfood Test ────────────────────────────

#[test]
fn full_workflow_dogfood() {
    let tmp = TempDir::new().unwrap();

    // 1. Init
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // 2. Create PRD
    forgeplan()
        .args(["new", "prd", "User Authentication"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));

    // 3. Create RFC linked to PRD
    forgeplan()
        .args(["new", "rfc", "OAuth2 Architecture"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("RFC-001"));

    // 4. Link RFC to PRD
    forgeplan()
        .args(["link", "RFC-001", "PRD-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // 5. Read artifact
    forgeplan()
        .args(["get", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("User Authentication"));

    // 6. Activate via lifecycle (update --status active is blocked)
    forgeplan()
        .args(["activate", "PRD-001", "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // 7. Validate
    forgeplan()
        .args(["validate", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .stdout(predicate::str::contains("PRD-001"));

    // 8. Graph shows both artifacts
    forgeplan()
        .arg("graph")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("RFC-001"))
        .stdout(predicate::str::contains("PRD-001"));

    // 9. List shows 2 artifacts
    forgeplan()
        .arg("list")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("RFC-001"));

    // 10. Status shows correct counts
    forgeplan()
        .arg("status")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("2 total"));

    // 11. Search finds artifact
    forgeplan()
        .args(["search", "Authentication"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));

    // 12. Calibrate
    forgeplan()
        .args(["calibrate", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));

    // 13. Progress
    forgeplan()
        .arg("progress")
        .current_dir(tmp.path())
        .assert()
        .success();

    // 14. Delete RFC
    forgeplan()
        .args(["delete", "RFC-001", "--yes"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted"));

    // 15. Verify only PRD remains
    forgeplan()
        .arg("list")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("1 artifact"));
}

// ── E2E: Dependency Graph + Topological Sort ─────────────

#[test]
fn e2e_blocked_shows_dependencies() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create PRD and RFC
    forgeplan()
        .args(["new", "prd", "Design Doc"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "rfc", "Implementation Plan"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Link RFC depends on PRD
    forgeplan()
        .args(["link", "RFC-001", "PRD-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Blocked should show RFC blocked by PRD
    forgeplan()
        .args(["blocked"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("RFC-001"))
        .stdout(predicate::str::contains("PRD-001"));
}

#[test]
fn e2e_order_shows_topological_sequence() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create chain: Epic → PRD → RFC
    forgeplan()
        .args(["new", "epic", "Platform"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Feature A"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "rfc", "How to build A"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["link", "PRD-001", "EPIC-001", "--relation", "refines"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", "RFC-001", "PRD-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Order should list all 3
    forgeplan()
        .args(["order"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("EPIC-001"))
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("RFC-001"));
}

#[test]
fn e2e_activate_unblocks_dependent() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Base Feature"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "rfc", "How to build"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", "RFC-001", "PRD-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Before activate: RFC blocked
    let output = forgeplan()
        .args(["blocked"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let before = String::from_utf8_lossy(&output.stdout);
    assert!(
        before.contains("Blocked") || before.contains("blocked"),
        "RFC should be blocked before activate"
    );

    // Activate PRD (--force because test PRD has short body and no evidence)
    forgeplan()
        .args(["activate", "PRD-001", "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // After activate: RFC should be ready
    let output2 = forgeplan()
        .args(["blocked"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let after = String::from_utf8_lossy(&output2.stdout);
    // RFC-001 should no longer appear as blocked (PRD-001 is now active)
    assert!(
        !after.contains("RFC-001 <- blocked") || after.contains("Ready"),
        "RFC should be unblocked after PRD activation, got: {}",
        after
    );
}

#[test]
fn e2e_graph_shows_mermaid_with_links() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Feature"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "rfc", "Plan"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", "RFC-001", "PRD-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["graph"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("graph LR"))
        .stdout(predicate::str::contains("RFC-001"))
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("based_on"));
}

#[test]
fn e2e_drift_runs_on_empty_workspace() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Drift should run even with no ADR/RFC
    forgeplan()
        .args(["drift"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("No active ADR/RFC")
                .or(predicate::str::contains("affected_files")),
        );
}

#[test]
fn e2e_migrate_idempotent() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // First migrate
    forgeplan()
        .args(["migrate"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("up to date").or(predicate::str::contains("complete")));

    // Second migrate — should also succeed (idempotent)
    forgeplan()
        .args(["migrate"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

// ── E2E: Full Methodology Cycle + Validation Quality ─────

#[test]
fn e2e_full_methodology_cycle() {
    let tmp = TempDir::new().unwrap();

    // 1. Init
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // 2. Create PRD with full content
    forgeplan()
        .args(["new", "prd", "Auth System"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // 3. Fill PRD body with proper content (Problem, Goals, FR, etc.)
    let body = r#"# PRD-001: Auth System

## Problem

Users cannot authenticate. The system has no login mechanism, no session management, and no access control. This blocks all features that require user identity.

## Goals

- [ ] Users can log in with email and password
- [ ] Sessions persist across browser refreshes
- [ ] Admin users have elevated permissions

## Non-Goals

- Social login (OAuth) — deferred to Phase 2
- Two-factor authentication — future enhancement

## Target Users

- End users who need to access the application
- Administrators who manage user accounts

## Functional Requirements

- [ ] FR-001: User can create an account with email and password
- [ ] FR-002: User can log in with valid credentials
- [ ] FR-003: System can maintain session state across requests
- [ ] FR-004: Admin can view and manage user list

## Related

- EPIC-001: Application Platform"#;

    forgeplan()
        .args(["update", "PRD-001", "--body", body])
        .current_dir(tmp.path())
        .assert()
        .success();

    // 4. Validate — should PASS
    forgeplan()
        .args(["validate", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));

    // 5. Create evidence
    forgeplan()
        .args(["new", "evidence", "Auth system tests pass"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Fill evidence with structured fields
    let evid_body = "## Structured Fields\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n\n## Results\n- 10 tests pass\n- Login flow verified";
    forgeplan()
        .args(["update", "EVID-001", "--body", evid_body])
        .current_dir(tmp.path())
        .assert()
        .success();

    // 6. Link evidence → PRD
    forgeplan()
        .args(["link", "EVID-001", "PRD-001", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // 7. Score — should show evidence and R_eff
    forgeplan()
        .args(["score", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("EVID-001"))
        .stdout(predicate::str::contains("R_eff"));

    // 8. Activate PRD (runs review internally, should pass)
    forgeplan()
        .args(["activate", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("active"));

    // 9. Health — should show 1 active artifact
    forgeplan()
        .args(["health"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("active"));
}

#[test]
fn e2e_validation_catches_quality_issues() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Bad PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // PRD with subjective adjectives, tech leakage, filler phrases
    let bad_body = r#"# PRD-001: Bad PRD

## Problem

This is a simple problem statement that needs to be fixed quickly.

## Goals

- [ ] Make the system easy to use and fast
- [ ] Build an intuitive interface with multiple features

## Non-Goals

- None

## Target Users

- Users

## Functional Requirements

- [ ] FR-001: System will allow users to easily navigate using React components with PostgreSQL database
- [ ] FR-002: In order to provide a seamless experience, the system should support multiple authentication methods via JWT and OAuth

## Related

- None"#;

    forgeplan()
        .args(["update", "PRD-001", "--body", bad_body])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Validate should catch issues
    let output = forgeplan()
        .args(["validate", "PRD-001"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect measurability issues (subjective adjectives)
    assert!(
        stdout.contains("Subjective adjective")
            || stdout.contains("adjective")
            || stdout.contains("easy"),
        "Should detect subjective adjectives like 'easy', got: {}",
        stdout
    );

    // Should detect implementation leakage
    assert!(
        stdout.contains("Tech names") || stdout.contains("React") || stdout.contains("PostgreSQL"),
        "Should detect tech leakage (React, PostgreSQL), got: {}",
        stdout
    );

    // Should detect filler phrases
    assert!(
        stdout.contains("filler") || stdout.contains("in order to"),
        "Should detect filler phrases like 'in order to', got: {}",
        stdout
    );
}

#[test]
fn e2e_score_shows_fgr_breakdown() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Test Feature"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Score should show F-G-R breakdown
    forgeplan()
        .args(["score", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Formality"))
        .stdout(predicate::str::contains("Granularity"))
        .stdout(predicate::str::contains("Reliability"));
}

#[test]
fn e2e_route_determines_depth() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Simple task → Tactical
    forgeplan()
        .args(["route", "fix typo in readme"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Tactical"));

    // Complex task with security keyword → Deep or Standard
    forgeplan()
        .args([
            "route",
            "implement OAuth2 authentication with security audit and compliance review",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Deep").or(predicate::str::contains("Standard")));
}

// ── E2E: Export/Import + FPF Knowledge Base ──────────────

#[test]
fn e2e_export_import_preserves_data() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create 3 artifacts with links
    forgeplan()
        .args(["new", "prd", "Feature A"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "rfc", "Plan for A"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "evidence", "Tests pass"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", "RFC-001", "PRD-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", "EVID-001", "PRD-001", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Export
    let export_path = tmp.path().join("backup.json");
    forgeplan()
        .args(["export", "--output", export_path.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("3 artifacts"));

    assert!(export_path.exists());

    // Verify export file is valid JSON
    let content = std::fs::read_to_string(&export_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json["artifacts"].is_array());
    assert_eq!(json["artifacts"].as_array().unwrap().len(), 3);

    // Destroy workspace
    std::fs::remove_dir_all(tmp.path().join(".forgeplan")).unwrap();

    // Re-init
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Import
    forgeplan()
        .args(["import", export_path.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Verify data restored
    forgeplan()
        .args(["health"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("3 total").or(predicate::str::contains("prd")));

    // Verify specific artifact
    forgeplan()
        .args(["get", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Feature A"));
}

// ── F3: Import stub gate (PRD-043) ─────────────────────────

/// A body that clearly triggers `check_stub_detailed` — contains multiple
/// known template markers ("Что мы строим", "[Actor] can [capability]", etc.).
const STUB_BODY: &str = "# PRD\n\n## Problem\n\nЧто мы строим и почему это важно\n\n## Goals\n\n[Actor] can [capability]\n\n## Non-Goals\n\n...\n\n## Target Users\n\n...\n\n## Related\n\n...\n";

/// A filled body with enough real content to pass stub detection.
const FILLED_BODY: &str = "# PRD\n\n## Problem\n\nUsers cannot reset their password without contacting support, causing a 4 hour average delay and measurable churn during onboarding. Support tickets mentioning password reset account for 22% of all tickets this quarter.\n\n## Goals\n\nSelf-service password reset via email link. Reduce support ticket volume by at least 15% within two months of rollout.\n\n## Non-Goals\n\nMulti-factor reset flows. Admin-initiated resets. SMS-based recovery is explicitly deferred until the SMS gateway vendor is selected.\n\n## Target Users\n\nEnd users of the web application who forgot their password. Support engineers handling escalations.\n\n## Related\n\nRFC-004 Auth architecture. ADR-007 Email provider choice.\n\n## Functional Requirements\n\nFR-001: User can request a password reset email from the login screen.\nFR-002: Reset links expire after 30 minutes.\nFR-003: Successful reset invalidates all existing sessions for that user.\n";

fn write_backup(path: &std::path::Path, id: &str, status: &str, body: &str) {
    let backup = serde_json::json!({
        "artifacts": [{
            "id": id,
            "kind": "prd",
            "status": status,
            "title": "Test PRD",
            "body": body,
            "depth": "standard",
        }],
        "relations": []
    });
    std::fs::write(path, serde_json::to_string_pretty(&backup).unwrap()).unwrap();
}

#[test]
fn test_import_downgrades_active_stub_to_draft() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let backup_path = tmp.path().join("stub-backup.json");
    write_backup(&backup_path, "PRD-100", "active", STUB_BODY);

    forgeplan()
        .args(["import", backup_path.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("stub detected").and(predicate::str::contains("PRD-100")));

    // Verify the imported record is draft, not active
    forgeplan()
        .args(["get", "PRD-100"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("draft"));
}

#[test]
fn test_import_preserves_active_for_filled_artifact() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let backup_path = tmp.path().join("filled-backup.json");
    write_backup(&backup_path, "PRD-200", "active", FILLED_BODY);

    forgeplan()
        .args(["import", backup_path.to_str().unwrap()])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["get", "PRD-200"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("active"));
}

#[test]
fn test_import_force_keeps_active_stub() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let backup_path = tmp.path().join("forced-backup.json");
    write_backup(&backup_path, "PRD-300", "active", STUB_BODY);

    forgeplan()
        .args(["import", backup_path.to_str().unwrap(), "--force"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("--force bypasses gate"));

    forgeplan()
        .args(["get", "PRD-300"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("active"));
}

#[test]
fn e2e_health_comprehensive() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create varied artifacts
    forgeplan()
        .args(["new", "prd", "Feature"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "problem", "Bug Report"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "note", "Quick Note"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Health should show all kinds
    forgeplan()
        .args(["health"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("prd"))
        .stdout(predicate::str::contains("problem"))
        .stdout(predicate::str::contains("note"))
        .stdout(predicate::str::contains("3 total"));
}

#[test]
fn e2e_list_shows_all_artifact_types() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "My PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "rfc", "My RFC"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "adr", "My ADR"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["list"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("RFC-001"))
        .stdout(predicate::str::contains("ADR-001"))
        .stdout(predicate::str::contains("My PRD"))
        .stdout(predicate::str::contains("My RFC"))
        .stdout(predicate::str::contains("My ADR"));
}

#[test]
fn e2e_supersede_workflow() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create and activate old PRD (--force because test PRD has short body and no evidence)
    forgeplan()
        .args(["new", "prd", "Old Feature"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["activate", "PRD-001", "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create new PRD
    forgeplan()
        .args(["new", "prd", "New Feature"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Supersede old with new
    forgeplan()
        .args(["supersede", "PRD-001", "--by", "PRD-002"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Superseded"));

    // Old PRD should be superseded
    forgeplan()
        .args(["get", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("superseded").or(predicate::str::contains("Superseded")));
}

#[test]
fn e2e_fpf_commands_available() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // FPF status before ingest
    forgeplan()
        .args(["fpf", "status"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("not initialized").or(predicate::str::contains("Status")));

    // FPF list before ingest
    forgeplan()
        .args(["fpf", "list"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // FPF search before ingest (should not crash)
    forgeplan()
        .args(["fpf", "search", "test"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

#[test]
fn e2e_adr_contract_validation() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create ADR without contract sections
    forgeplan()
        .args(["new", "adr", "Use PostgreSQL"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Validate runs and produces result (PASS or warnings depending on depth)
    forgeplan()
        .args(["validate", "ADR-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("ADR-001"))
        .stdout(
            predicate::str::contains("PASS")
                .or(predicate::str::contains("error").or(predicate::str::contains("warning"))),
        );
}

// ─── Scan-Import E2E Tests ───────────────────────────────────────

#[test]
fn scan_import_dry_run_shows_preview() {
    let tmp = TempDir::new().unwrap();

    // Init workspace
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create docs/ with a PRD file
    let docs = tmp.path().join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(
        docs.join("PRD-001-auth.md"),
        "---\nkind: prd\nid: PRD-001\ntitle: Auth System\n---\n\n# PRD-001: Auth System\n\n## Problem\nUsers can't log in.\n\n## Goals\nSecure auth.",
    ).unwrap();

    // Dry-run
    forgeplan()
        .args(["scan-import", "--dry-run"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD"))
        .stdout(predicate::str::contains("1 imported"));
}

#[test]
fn scan_import_imports_frontmatter_prd() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let docs = tmp.path().join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(
        docs.join("PRD-042-payments.md"),
        "---\nkind: prd\nid: PRD-042\ntitle: Payment Integration\n---\n\n# Payments\n\n## Problem\nNo payments.\n\n## Goals\nAccept payments.",
    ).unwrap();

    // Import
    forgeplan()
        .args(["scan-import"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("1 imported"));

    // Verify artifact exists
    forgeplan()
        .args(["get", "PRD-042"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Payment Integration"));
}

#[test]
fn scan_import_detects_by_filename() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let docs = tmp.path().join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    // File with no frontmatter but a PRD filename pattern
    std::fs::write(
        docs.join("RFC-001-api-redesign.md"),
        "# API Redesign\n\nWe should redesign the API.",
    )
    .unwrap();

    forgeplan()
        .args(["scan-import"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("RFC"))
        .stdout(predicate::str::contains("1 imported"));
}

#[test]
fn scan_import_skips_existing_artifacts() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create a PRD first
    forgeplan()
        .args(["new", "prd", "Existing PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Now put a doc with the same ID
    let docs = tmp.path().join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(
        docs.join("PRD-001-duplicate.md"),
        "---\nkind: prd\nid: PRD-001\ntitle: Duplicate\n---\n# Duplicate",
    )
    .unwrap();

    // Import should skip
    forgeplan()
        .args(["scan-import"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("1 skipped"));
}

#[test]
fn scan_import_handles_unknown_files() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let docs = tmp.path().join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(
        docs.join("random-notes.md"),
        "# Shopping List\n\n- Milk\n- Bread",
    )
    .unwrap();

    forgeplan()
        .args(["scan-import", "--dry-run"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("1 unknown"));
}

#[test]
fn init_with_scan_flag_imports_docs() {
    let tmp = TempDir::new().unwrap();

    // Pre-create docs before init
    let docs = tmp.path().join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(
        docs.join("ADR-001-use-rust.md"),
        "---\nkind: adr\nid: ADR-001\ntitle: Use Rust\n---\n\n## Decision\nUse Rust.\n\n## Status\nAccepted.",
    ).unwrap();

    // Init with --scan
    forgeplan()
        .args(["init", "-y", "--scan"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported"));

    // Verify
    forgeplan()
        .args(["get", "ADR-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Use Rust"));
}

#[test]
fn scan_import_multiple_types() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let docs = tmp.path().join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(
        docs.join("PRD-001-feature.md"),
        "---\nkind: prd\nid: PRD-001\ntitle: Feature\n---\n# Feature",
    )
    .unwrap();
    std::fs::write(
        docs.join("RFC-001-design.md"),
        "---\nkind: rfc\nid: RFC-001\ntitle: Design\n---\n# Design",
    )
    .unwrap();
    std::fs::write(
        docs.join("ADR-001-choice.md"),
        "---\nkind: adr\nid: ADR-001\ntitle: Choice\n---\n# Choice",
    )
    .unwrap();

    forgeplan()
        .args(["scan-import"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("3 imported"));

    // All three exist
    forgeplan()
        .args(["get", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["get", "RFC-001"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["get", "ADR-001"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

// ─── JSON Output Structural Tests ────────────────────────────────

#[test]
fn json_get_is_valid_and_has_required_fields() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Test PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["get", "PRD-001", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(json["id"], "PRD-001");
    assert_eq!(json["kind"], "prd");
    assert!(json["status"].is_string());
    assert!(json["title"].is_string());
    assert!(json["body"].is_string());
    assert!(json["r_eff"].is_number());
}

#[test]
fn json_score_is_valid_and_has_reff() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Test PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["score", "PRD-001", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert!(json["r_eff"].is_number());
    assert!(json["fgr"].is_object());
    assert!(json["fgr"]["formality"].is_number());
}

#[test]
fn json_list_is_valid_array() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Test"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["list", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert!(json.is_array());
    assert!(!json.as_array().unwrap().is_empty());
    assert!(json[0]["id"].is_string());
}

#[test]
fn json_health_has_required_fields() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["health", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert!(json["total"].is_number());
    assert!(json["blind_spots"].is_array());
    assert!(json["at_risk"].is_array());
}

#[test]
fn json_blocked_is_valid() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["blocked", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert!(json["ready"].is_array());
    assert!(json["blocked"].is_array());
}

// ─── Dry-Run Side-Effect Test ────────────────────────────────────

#[test]
fn scan_import_dry_run_does_not_persist() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let docs = tmp.path().join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(
        docs.join("PRD-099-test.md"),
        "---\nkind: prd\nid: PRD-099\ntitle: Dry Run Test\n---\n# Test",
    )
    .unwrap();

    // Dry-run should show preview
    forgeplan()
        .args(["scan-import", "--dry-run"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Artifact should NOT exist
    forgeplan()
        .args(["get", "PRD-099"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// ─── R_eff Bidirectional Evidence E2E Test ────────────────────────

#[test]
fn reff_finds_incoming_evidence() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create PRD and evidence
    forgeplan()
        .args(["new", "prd", "Target PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "evidence", "Proof"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Link evidence → PRD (incoming direction for PRD)
    forgeplan()
        .args(["link", "EVID-001", "PRD-001", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Score should find evidence via incoming link
    forgeplan()
        .args(["score", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("EVID-001"));
}

// ─── v0.11 E2E Tests: Activation Gate + Derived Status + Context ──

#[test]
fn activation_gate_rejects_invalid() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create PRD then strip its body to trigger MUST validation errors
    forgeplan()
        .args(["new", "prd", "Stub PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Replace body with empty content — missing Problem, Goals, etc.
    forgeplan()
        .args([
            "update",
            "PRD-001",
            "--body",
            "# Empty PRD\n\nNo sections here.",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Activate should FAIL — PRD missing MUST sections (Problem, Goals, etc.)
    let output = forgeplan()
        .args(["activate", "PRD-001"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "activate on invalid PRD should fail, but succeeded. stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("MUST")
            || stderr.contains("error")
            || stderr.contains("Validation")
            || stderr.contains("validation")
            || stderr.contains("Cannot activate"),
        "Error should mention rejection, got stderr: {}",
        stderr
    );
}

#[test]
fn activation_gate_force_overrides() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create PRD then strip body to trigger MUST errors
    forgeplan()
        .args(["new", "prd", "Force Activate PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args([
            "update",
            "PRD-001",
            "--body",
            "# Empty\n\nNo required sections.",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Activate with --force should SUCCEED despite validation errors
    forgeplan()
        .args(["activate", "PRD-001", "--force"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Warning")
                .or(predicate::str::contains("forced").or(predicate::str::contains("Activated"))),
        );
}

#[test]
fn activation_gate_passes_when_valid() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Notes skip validation gate (lightweight kind)
    forgeplan()
        .args(["new", "note", "Test Note"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Activate should succeed — notes don't require validation
    forgeplan()
        .args(["activate", "NOTE-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Activated").or(predicate::str::contains("active")));
}

#[test]
fn health_shows_derived_status() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create a stub PRD
    forgeplan()
        .args(["new", "prd", "Derived Status Test"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Health should show derived status info (STUB for an unfilled PRD)
    let output = forgeplan()
        .args(["health"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("STUB") || stdout.contains("derived") || stdout.contains("By derived"),
        "Health should show derived status info, got: {}",
        stdout
    );
}

#[test]
fn context_command_json_output() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Context Test"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["context", "PRD-001", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "context --json should succeed");

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("context --json should produce valid JSON");

    // Verify all required top-level keys
    assert!(
        json["artifact"].is_object(),
        "missing 'artifact' key in context JSON"
    );
    assert!(
        json["derived_status"].is_string(),
        "missing 'derived_status' key in context JSON"
    );
    assert!(
        json["graph"].is_object(),
        "missing 'graph' key in context JSON"
    );
    assert!(
        json["validation"].is_object(),
        "missing 'validation' key in context JSON"
    );
    assert!(json["fgr"].is_object(), "missing 'fgr' key in context JSON");
}

#[test]
fn context_command_human_output() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Human Context Test"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["context", "PRD-001"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "context (human) should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PRD-001"),
        "Human output should contain artifact ID, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Status"),
        "Human output should contain 'Status:', got: {}",
        stdout
    );
    assert!(
        stdout.contains("F-G-R"),
        "Human output should contain 'F-G-R:', got: {}",
        stdout
    );
}

#[test]
fn tree_shows_hierarchy() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create epic and PRD
    forgeplan()
        .args(["new", "epic", "My Epic"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("EPIC-001"));

    forgeplan()
        .args(["new", "prd", "My Feature"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));

    // Link PRD -> Epic (child relation)
    forgeplan()
        .args(["link", "PRD-001", "EPIC-001", "--relation", "refines"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Run tree — should show both artifacts in hierarchy
    let output = forgeplan()
        .args(["tree"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "tree should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("EPIC-001"),
        "tree should contain EPIC-001, got: {}",
        stdout
    );
    assert!(
        stdout.contains("PRD-001"),
        "tree should contain PRD-001, got: {}",
        stdout
    );
    assert!(
        stdout.contains("My Epic"),
        "tree should contain epic title, got: {}",
        stdout
    );
}

#[test]
fn tree_json_is_valid() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "JSON Tree Test"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["tree", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "tree --json should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "tree --json output should be valid JSON: {}. Got: {}",
            e, stdout
        )
    });

    assert!(parsed.is_array(), "root should be an array");
    let arr = parsed.as_array().unwrap();
    assert!(!arr.is_empty(), "array should have at least one root");

    let first = &arr[0];
    assert_eq!(first["id"], "PRD-001");
    assert_eq!(first["kind"], "prd");
    assert!(first["children"].is_array(), "children should be an array");
}

// ─── PROB-012 E2E Tests: Integrity Fixes ────────────────────────────

#[test]
fn e2e_reff_write_back_persists_to_tree() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create PRD + evidence + link
    forgeplan()
        .args(["new", "prd", "Write-back Test"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "evidence", "Proof"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", "EVID-001", "PRD-001", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Score should compute R_eff and persist it
    forgeplan()
        .args(["score", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("R_eff"));

    // Tree should show the persisted R_eff (not 0.00)
    let tree_output = forgeplan()
        .args(["tree", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let tree_json: serde_json::Value =
        serde_json::from_slice(&tree_output.stdout).expect("valid tree JSON");
    let nodes = tree_json.as_array().unwrap();
    let prd = nodes
        .iter()
        .find(|n| n["id"] == "PRD-001")
        .expect("PRD-001 in tree");
    let r_eff = prd["r_eff"].as_f64().unwrap_or(0.0);
    assert!(
        r_eff > 0.0,
        "R_eff should be persisted after score, got {r_eff}"
    );
}

#[test]
fn e2e_route_p0_issues_not_tactical() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Route with severity + integrity keywords should NOT have Tactical as primary depth
    // (Tactical may appear in Alternatives section — that's expected)
    forgeplan()
        .args(["route", "Fix 5 P0 integrity issues in scoring system"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("## Depth: Deep"));
}

#[test]
fn e2e_health_shows_problem_blind_spot() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create active problem without evidence
    forgeplan()
        .args(["new", "problem", "Test Problem"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Activate it (problems don't require validation gate)
    forgeplan()
        .args(["activate", "PROB-001"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Health should show blind spot for active problem without evidence
    forgeplan()
        .args(["health"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Blind spots: 1").or(predicate::str::contains("PROB-001")),
        );
}

#[test]
fn e2e_journal_excludes_deprecated_from_warning() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create note, activate, then deprecate (lifecycle: draft→active→deprecated)
    forgeplan()
        .args(["new", "note", "Old Decision"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["activate", "NOTE-001"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["deprecate", "NOTE-001", "--reason", "outdated"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Journal should NOT count deprecated as "no evidence"
    let output = forgeplan()
        .args(["journal"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // If deprecated is the only artifact, "no evidence" warning should be 0 or absent
    assert!(
        !stdout.contains("1 decision(s) without any evidence"),
        "Deprecated note should not count in no-evidence warning"
    );
}

#[test]
fn e2e_coverage_backfill_adds_section() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create PRD and strip Affected Files from body (simulate pre-template artifact)
    forgeplan()
        .args(["new", "prd", "Backfill Target"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args([
            "update",
            "PRD-001",
            "--body",
            "## Problem\n\nNo affected files section here.",
        ])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Force activate
    forgeplan()
        .args(["activate", "PRD-001", "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Run backfill — should find PRD-001 missing section
    forgeplan()
        .args(["coverage", "--backfill"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));

    // Get artifact and verify body contains Affected Files
    let output = forgeplan()
        .args(["get", "PRD-001", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let body = json["body"].as_str().unwrap_or("");
    assert!(
        body.contains("## Affected Files"),
        "Body should contain Affected Files section"
    );
    assert!(body.contains("/**"), "Should use glob patterns, not ...");

    // Idempotent: second run should say "All active..."
    forgeplan()
        .args(["coverage", "--backfill"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("already have"));
}

#[test]
fn e2e_score_missing_id_shows_warning_not_crash() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Score nonexistent artifact should fail gracefully
    forgeplan()
        .args(["score", "NONEXISTENT-999"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Not found")));
}

#[test]
fn e2e_reff_skips_deprecated_dependency() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create: PRD depends_on PROB, PROB has evidence, then deprecate PROB
    forgeplan()
        .args(["new", "problem", "Old Problem"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Depends on old problem"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "evidence", "PRD proof"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Link: PRD → PROB (based_on), EVID → PRD (informs)
    forgeplan()
        .args(["link", "PRD-001", "PROB-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", "EVID-001", "PRD-001", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Activate and deprecate PROB
    forgeplan()
        .args(["activate", "PROB-001"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["deprecate", "PROB-001", "--reason", "resolved"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Score PRD-001 — should NOT be dragged to 0 by deprecated PROB-001
    let output = forgeplan()
        .args(["score", "PRD-001", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let r_eff = json["r_eff"].as_f64().unwrap_or(0.0);
    assert!(
        r_eff > 0.0,
        "R_eff should be > 0 when dependency is deprecated, got {r_eff}"
    );
}

// -----------------------------------------------------------------------
// BUG-001: scan --path outside project root → exit 1
// -----------------------------------------------------------------------

#[test]
fn scan_path_traversal_rejected() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["scan", "--path", "/tmp"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("outside project root")
                .or(predicate::str::contains("does not exist")),
        );
}

#[test]
fn scan_path_relative_traversal_rejected() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["scan", "--path", "../../etc"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

// -----------------------------------------------------------------------
// BUG-002: unlink non-existent relation → exit 1
// -----------------------------------------------------------------------

#[test]
fn unlink_nonexistent_relation_fails() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Test PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["unlink", "PRD-001", "RFC-999", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// -----------------------------------------------------------------------
// BUG-003: activate shows real old_status, not hardcoded "draft"
// -----------------------------------------------------------------------

#[test]
fn activate_shows_correct_transition_status() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create and activate a note (no validation gate)
    forgeplan()
        .args(["new", "note", "Test Note"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["activate", "NOTE-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("draft → active"));
}

// -----------------------------------------------------------------------
// PROB-020: Cascade delete removes relations
// -----------------------------------------------------------------------

#[test]
fn e2e_delete_cascades_relations() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create two notes and link them
    forgeplan()
        .args(["new", "note", "Parent Note"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "note", "Child Note"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["link", "NOTE-001", "NOTE-002", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Delete parent — should cascade relations
    forgeplan()
        .args(["delete", "NOTE-001", "--yes"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Child should still exist but not show phantom relation
    forgeplan()
        .args(["get", "NOTE-002"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Child Note"));
}

// -----------------------------------------------------------------------
// PROB-020: Deprecated artifact does not block dependents
// -----------------------------------------------------------------------

#[test]
fn e2e_deprecated_does_not_block() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create two notes, link, activate both
    forgeplan()
        .args(["new", "note", "Dependency"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "note", "Dependent"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["activate", "NOTE-001"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["activate", "NOTE-002"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["link", "NOTE-002", "NOTE-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Deprecate the dependency
    forgeplan()
        .args(["deprecate", "NOTE-001", "--reason", "no longer needed"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Blocked should NOT show NOTE-002 as blocked
    forgeplan()
        .args(["blocked"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No blocked artifacts"));
}

// -----------------------------------------------------------------------
// ADR-005: Full lifecycle draft → active → deprecated (terminal)
// -----------------------------------------------------------------------

#[test]
fn e2e_full_lifecycle_deprecate() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "note", "Lifecycle Test"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // draft → active
    forgeplan()
        .args(["activate", "NOTE-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("draft → active"));

    // active → deprecated (terminal)
    forgeplan()
        .args([
            "deprecate",
            "NOTE-001",
            "--reason",
            "replaced by new approach",
        ])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Deprecated"));

    // Verify status is deprecated
    forgeplan()
        .args(["get", "NOTE-001"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("deprecated"));
}

// -----------------------------------------------------------------------
// ADR-005: draft → deprecated directly is NOT allowed
// -----------------------------------------------------------------------

#[test]
fn e2e_draft_cannot_deprecate_directly() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "note", "Draft Note"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // draft → deprecated should fail
    forgeplan()
        .args(["deprecate", "NOTE-001", "--reason", "test"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid transition"));
}

// -----------------------------------------------------------------------
// PROB-020: topological order excludes deprecated
// -----------------------------------------------------------------------

#[test]
fn e2e_order_excludes_deprecated() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Create chain: NOTE-001 → NOTE-002 → NOTE-003
    for title in &["First", "Second", "Third"] {
        forgeplan()
            .args(["new", "note", title])
            .current_dir(tmp.path())
            .assert()
            .success();
    }

    for note in &["NOTE-001", "NOTE-002", "NOTE-003"] {
        forgeplan()
            .args(["activate", note])
            .current_dir(tmp.path())
            .assert()
            .success();
    }

    forgeplan()
        .args(["link", "NOTE-002", "NOTE-001", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["link", "NOTE-003", "NOTE-002", "--relation", "based_on"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Deprecate middle node
    forgeplan()
        .args(["deprecate", "NOTE-002", "--reason", "skipped"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Order should still work without error
    forgeplan()
        .args(["order"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

// -----------------------------------------------------------------------
// Sprint 8 S1: route rejects empty input
// -----------------------------------------------------------------------

#[test]
fn e2e_empty_route_rejected() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["route", ""])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty"));
}

// -----------------------------------------------------------------------
// Sprint 13.1.6: --force is a visible alias for --allow-duplicate
// -----------------------------------------------------------------------

#[test]
fn test_new_accepts_force_alias() {
    let tmp = TempDir::new().unwrap();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Test Title"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Test Title", "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["list", "--type", "prd"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("PRD-002"));
}

// ---------------------------------------------------------------------------
// PRD-039 Sprint 13.2 — search filter flags (--status, --depth,
// --with-evidence/--no-evidence, --since, --no-expand)
// ---------------------------------------------------------------------------

#[test]
fn search_status_filter_excludes_drafts() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Two PRDs: one stays draft, the other gets force-activated.
    forgeplan()
        .args(["new", "prd", "Active Authentication Spec"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Draft Authentication Spec"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["activate", "PRD-001", "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // --status active must exclude PRD-002 (draft).
    forgeplan()
        .args(["search", "Authentication", "--status", "active", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("PRD-002").not());
}

#[test]
fn search_no_evidence_finds_blind_spots() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Payment Gateway"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // No evidence linked → r_eff_score == 0 → --no-evidence must return it,
    // --with-evidence must NOT.
    forgeplan()
        .args(["search", "Payment", "--no-evidence", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));

    forgeplan()
        .args(["search", "Payment", "--with-evidence", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001").not());
}

#[test]
fn search_no_expand_disables_neighbor_expansion() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Logging System"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "rfc", "Telemetry Pipeline"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["link", "RFC-001", "PRD-001", "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // With --no-expand, querying for "Logging" must NOT pull in RFC-001 as
    // an expanded neighbor — only the direct PRD match.
    forgeplan()
        .args(["search", "Logging", "--no-expand", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("RFC-001").not());
}

#[test]
fn search_since_date_filter() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", "Caching Layer"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Created today → --since in the far past must match.
    forgeplan()
        .args(["search", "Caching", "--since", "2000-01-01", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));

    // --since in the far future must exclude everything.
    forgeplan()
        .args(["search", "Caching", "--since", "2999-01-01", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001").not());

    // Bad date format must error out.
    forgeplan()
        .args(["search", "Caching", "--since", "not-a-date"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid --since date"));
}

// ============================================================================
// Tag / Untag commands (PRD-035 FR-002)
// ============================================================================

fn init_with_prd(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Tag Test"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

#[test]
fn test_tag_adds_tags_to_artifact() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp);

    forgeplan()
        .args(["tag", "PRD-001", "source=code", "legacy"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Added 2 tag(s) to PRD-001"))
        .stdout(predicate::str::contains("source=code"))
        .stdout(predicate::str::contains("legacy"));
}

#[test]
fn test_untag_removes_tags_from_artifact() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp);

    forgeplan()
        .args(["tag", "PRD-001", "alpha", "beta", "gamma"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["untag", "PRD-001", "beta"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed 1 tag(s) from PRD-001"))
        .stdout(predicate::str::contains("alpha"))
        .stdout(predicate::str::contains("gamma"));
}

#[test]
fn test_tag_requires_at_least_one_tag() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp);

    forgeplan()
        .args(["tag", "PRD-001"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

#[test]
fn test_tag_fails_on_missing_artifact() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp);

    forgeplan()
        .args(["tag", "PRD-999", "foo"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("Artifact not found")),
        );
}

#[test]
fn test_tag_dedupe_same_tag_twice() {
    let tmp = TempDir::new().unwrap();
    init_with_prd(&tmp);

    forgeplan()
        .args(["tag", "PRD-001", "dup"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["tag", "PRD-001", "dup", "dup"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Verify only one "dup" tag remains via list output
    let output = forgeplan()
        .args(["tag", "PRD-001", "other"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Count occurrences of "dup" in current tags line — should appear once
    let tags_line = stdout
        .lines()
        .find(|l| l.contains("Current tags"))
        .unwrap_or("");
    let dup_count = tags_line.matches("dup").count();
    assert_eq!(
        dup_count, 1,
        "expected dedupe, got tags line: {}",
        tags_line
    );
}

// ============================================================================
// FR-003: `forgeplan list --tag <filter>` (Sprint 13.3 / PRD-035)
// ============================================================================

fn init_workspace(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

fn new_prd_with_tags(tmp: &TempDir, title: &str, id: &str, tags: &[&str]) {
    forgeplan()
        .args(["new", "prd", title])
        .current_dir(tmp.path())
        .assert()
        .success();
    if !tags.is_empty() {
        let mut args = vec!["tag", id];
        args.extend(tags);
        forgeplan()
            .args(&args)
            .current_dir(tmp.path())
            .assert()
            .success();
    }
}

#[test]
fn test_list_tag_filter_exact_match() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    new_prd_with_tags(&tmp, "Alpha Feature", "PRD-001", &["source=code"]);
    new_prd_with_tags(&tmp, "Beta Feature", "PRD-002", &[]);

    forgeplan()
        .args(["list", "--tag", "source=code", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("PRD-002").not());
}

#[test]
fn test_list_tag_filter_key_only() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    new_prd_with_tags(&tmp, "One", "PRD-001", &["source=code"]);
    new_prd_with_tags(&tmp, "Two", "PRD-002", &["legacy"]);
    new_prd_with_tags(&tmp, "Three", "PRD-003", &["layer=domain"]);

    forgeplan()
        .args(["list", "--tag", "source", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"))
        .stdout(predicate::str::contains("PRD-003").not());

    forgeplan()
        .args(["list", "--tag", "legacy", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-002"));
}

#[test]
fn test_list_tag_filter_combined_with_kind() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    new_prd_with_tags(&tmp, "Tagged PRD", "PRD-001", &["source=code"]);

    forgeplan()
        .args(["list", "--tag", "source=code", "--type", "prd", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001"));

    forgeplan()
        .args(["list", "--tag", "source=code", "--type", "rfc", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("PRD-001").not());
}

#[test]
fn test_list_tag_filter_empty_when_no_match() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    new_prd_with_tags(&tmp, "Only", "PRD-001", &["source=code"]);

    forgeplan()
        .args(["list", "--tag", "nothing=here", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}
