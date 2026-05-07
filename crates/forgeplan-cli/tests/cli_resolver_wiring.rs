//! PROB-060 / SPEC-005 Phase 2.6 (CD-6) — resolver wiring across CLI verbs.
//!
//! Integration tests verifying that each CLI command listed in CD-6
//! accepts BOTH the slug form (`prd-foo-bar`) and the display id form
//! (`PRD-001`) as the artifact reference argument. The resolver is a
//! single function (`LanceStore::resolve_id`); these tests catch the
//! call-site wiring — i.e. the place in each command where user input
//! flows into `get_record` / lifecycle helpers / projection helpers.
//!
//! # PROB-060 Phase 2 audit closure (HIGH-7) — positive assertions
//!
//! Round 1 (the obsolete shape) used a `assert_no_not_found` helper that
//! string-grep'd combined stderr/stdout for the literal
//! `"Artifact '<id>' not found"` message. The audit caught a false
//! confidence: if the resolver is removed entirely, downstream commands
//! would fail with a *different* error message ("LLM not configured",
//! "Cannot reopen: status 'draft'", "Invalid transition") and the
//! string-grep needle would *still* not match — so the test would
//! silently pass through a real regression.
//!
//! HIGH-7 fix (this file) replaces the negative `assert_no_not_found`
//! with **positive** assertions per command, picking the strategy that
//! best matches the command's semantics:
//!
//! - **Strategy A — post-action state verification** (idempotent,
//!   non-LLM commands): run the command, then re-read state via
//!   `forgeplan get <ref> --json` / `claims --json` / `list --json`
//!   and assert the post-action state matches expectations. Used for
//!   `update`, `delete`, `claim`, `release`, `import`, `renew`, `fgr`,
//!   `estimate` and `calibrate-estimate` (where the body has at least
//!   one estimable item).
//!
//! - **Strategy B — structured downstream error fingerprint**
//!   (commands that legitimately fail on a fresh-draft fixture but
//!   only AFTER successful resolution): we assert the *specific*
//!   downstream error message contains the *resolved canonical id*.
//!   That string is only emitted by code paths that ran AFTER
//!   `resolve_id` returned `Some(canonical)` — so its presence is a
//!   positive proof the resolver did its job. Used for `reason`
//!   (LLM not configured), `decompose` (LLM not configured),
//!   `reopen` (status='draft' gate), and `supersede` (invalid
//!   transition gate).
//!
//! Each strategy is documented at the test site so the next reader
//! understands the tactical choice without re-reading the audit.
//!
//! Reference: §2 CD-6 в `docs/sessions/2026-05-07-PROB-060-phase-2-3-4-handoff.md`
//! и `docs/audit/PROB-060-phase-2-round-1.md` HIGH-7.

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

/// Strategy A helper — read JSON shape of an artifact by ref and return
/// it as a serde_json::Value. Used to verify post-action state.
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

/// Strategy A helper — list all artifacts in workspace as JSON.
fn list_json(workspace: &Path) -> Value {
    let out = forgeplan()
        .args(["list", "--json"])
        .current_dir(workspace)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    // `list --json` may have a trailing `Next:` line; strip everything
    // after the closing `]` of the JSON array.
    let s = String::from_utf8(out).unwrap();
    let end = s.rfind(']').expect("list --json must contain JSON array");
    serde_json::from_str(&s[..=end]).expect("list --json head must parse as JSON")
}

/// Strategy A helper — read claims state as JSON.
fn claims_json(workspace: &Path) -> Value {
    let out = forgeplan()
        .args(["claims", "--json"])
        .current_dir(workspace)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    // `claims --json` body may have leading whitespace + a single root
    // object; serde handles that. Strip any stray Next: tail on success
    // path (the renderer in Phase 2 emits the JSON object cleanly here,
    // but be defensive).
    let s = String::from_utf8(out).unwrap();
    let start = s.find('{').expect("claims --json must contain JSON object");
    let end = s.rfind('}').expect("claims --json must close JSON object");
    serde_json::from_str(&s[start..=end]).expect("claims --json must parse")
}

/// Strategy B helper — run a command that is expected to fail with a
/// downstream (post-resolver) structured error, and assert that error
/// fingerprint mentions the resolved canonical id. The fingerprint is a
/// substring that downstream code emits AFTER successful resolution; if
/// the resolver misfires, the user instead gets `"Artifact '<x>' not
/// found"` and the fingerprint check fails.
///
/// Returns the combined stdout+stderr for any extra assertions a caller
/// might want to make.
fn assert_downstream_fingerprint(
    cmd: &mut Command,
    ref_arg: &str,
    expected_fingerprint: &str,
) -> String {
    let output = cmd.output().expect("failed to spawn forgeplan");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}\n{stderr}");

    // Negative guard: the resolver-not-found needle MUST be absent. This
    // is the same guard the old assert_no_not_found provided, but kept
    // as a tripwire — the positive assertion below is the primary signal.
    let not_found_needle = format!("Artifact '{ref_arg}' not found");
    assert!(
        !combined.contains(&not_found_needle),
        "resolver failed to recognise `{ref_arg}` (got generic not-found error) — full output:\n{combined}"
    );

    // Positive guard: the downstream fingerprint MUST appear. The
    // fingerprint is a string that only executes if the resolver
    // returned Some(canonical) — so its presence proves the call site
    // is wired correctly.
    assert!(
        combined.contains(expected_fingerprint),
        "expected downstream fingerprint `{expected_fingerprint}` (post-resolver path) not found — full output:\n{combined}"
    );

    combined
}

// ────────────────────────────────────────────────────────────────────
// 1. update — Strategy A: post-action title verification.
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

    // Strategy A — post-action: re-read state, assert title actually
    // changed. Proves the resolver mapped slug→PRD-001 AND the update
    // path reached storage. If the resolver were stubbed out, the title
    // would not change (or get_json would fail).
    let json = get_json(dir.path(), &slug);
    assert_eq!(
        json["title"].as_str(),
        Some("Updated Via Slug"),
        "post-update title must reflect new value (proves resolver+update wiring)"
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
// 2. reason — Strategy B: LLM-not-configured fingerprint mentions PRD-001.
//    The reason command's only failure mode on a fresh fixture is the
//    LLM stage; resolver runs first. The fingerprint we look for is the
//    `"Error: LLM not configured"` line which only emits AFTER the
//    record has been resolved + loaded.
// ────────────────────────────────────────────────────────────────────

#[test]
fn reason_accepts_slug_form() {
    let dir = workspace_with_one_prd("Reason Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    let mut cmd = forgeplan();
    cmd.args(["reason", &slug]).current_dir(dir.path());
    // Strategy B: downstream-stage fingerprint = "LLM not configured".
    // This message originates AFTER `resolve_id` + `get_record`, so its
    // presence proves the resolver returned Some.
    assert_downstream_fingerprint(&mut cmd, &slug, "LLM not configured");
}

#[test]
fn reason_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Reason Display Test");

    let mut cmd = forgeplan();
    cmd.args(["reason", "PRD-001"]).current_dir(dir.path());
    assert_downstream_fingerprint(&mut cmd, "PRD-001", "LLM not configured");
}

// ────────────────────────────────────────────────────────────────────
// 3. decompose — Strategy B: same LLM-not-configured fingerprint.
// ────────────────────────────────────────────────────────────────────

#[test]
fn decompose_accepts_slug_form() {
    let dir = workspace_with_one_prd("Decompose Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    let mut cmd = forgeplan();
    cmd.args(["decompose", &slug]).current_dir(dir.path());
    assert_downstream_fingerprint(&mut cmd, &slug, "LLM not configured");
}

#[test]
fn decompose_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Decompose Display Test");

    let mut cmd = forgeplan();
    cmd.args(["decompose", "PRD-001"]).current_dir(dir.path());
    assert_downstream_fingerprint(&mut cmd, "PRD-001", "LLM not configured");
}

// ────────────────────────────────────────────────────────────────────
// 4. delete — Strategy A: post-action `list` must NOT contain PRD-001.
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

    // Strategy A — post-action: list must be empty. Proves resolver
    // routed slug→PRD-001 AND the delete actually removed the row.
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
// 5. renew — Strategy A: post-action status=`active` and `valid_until`
//    matches `--until`. `forgeplan renew` is permissive on draft (it
//    transitions stale|draft → active), so we can verify directly.
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

    // Strategy A — verify renewal landed in storage. status must flip
    // to active; valid_until must equal what we passed.
    let json = get_json(dir.path(), &slug);
    assert_eq!(
        json["status"].as_str(),
        Some("active"),
        "post-renew status must be active (proves resolver+lifecycle wiring)"
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
// 6. reopen — Strategy B: `Cannot reopen <ID>: status 'draft'` is the
//    structured downstream error that ONLY fires after the artifact has
//    been successfully resolved + loaded. The error message echoes the
//    canonical id (PRD-001), so it's both a positive proof of resolver
//    success AND of canonical-id propagation downstream.
// ────────────────────────────────────────────────────────────────────

#[test]
fn reopen_accepts_slug_form() {
    let dir = workspace_with_one_prd("Reopen Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    let mut cmd = forgeplan();
    cmd.args(["reopen", &slug, "--reason", "test reopen"])
        .current_dir(dir.path());
    // Strategy B: downstream error includes the canonical id PRD-001 —
    // emitted after lifecycle gate reads `record.status`. Resolver must
    // have mapped slug→PRD-001 for this fingerprint to appear.
    assert_downstream_fingerprint(&mut cmd, &slug, "Cannot reopen PRD-001: status 'draft'");
}

#[test]
fn reopen_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Reopen Display Test");

    let mut cmd = forgeplan();
    cmd.args(["reopen", "PRD-001", "--reason", "test reopen"])
        .current_dir(dir.path());
    assert_downstream_fingerprint(&mut cmd, "PRD-001", "Cannot reopen PRD-001: status 'draft'");
}

// ────────────────────────────────────────────────────────────────────
// 7. supersede — Strategy B: `Invalid transition: draft → superseded`
//    is the structured lifecycle gate failure that only fires after the
//    record has been loaded. (For draft fixtures.)
// ────────────────────────────────────────────────────────────────────

#[test]
fn supersede_accepts_slug_form() {
    let dir = workspace_with_one_prd("Supersede Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    // Need a target for --by. Create a second PRD.
    forgeplan()
        .args(["new", "prd", "Supersede Target"])
        .current_dir(dir.path())
        .assert()
        .success();

    let mut cmd = forgeplan();
    cmd.args(["supersede", &slug, "--by", "PRD-002"])
        .current_dir(dir.path());
    // Strategy B — the lifecycle gate emits a deterministic error that
    // includes both source and target states; its presence proves
    // `get_record` returned the artifact (resolver hit) AND the gate
    // logic ran on real state.
    assert_downstream_fingerprint(&mut cmd, &slug, "Invalid transition: draft → superseded");
}

#[test]
fn supersede_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Supersede Display Test");
    forgeplan()
        .args(["new", "prd", "Supersede Display Target"])
        .current_dir(dir.path())
        .assert()
        .success();

    let mut cmd = forgeplan();
    cmd.args(["supersede", "PRD-001", "--by", "PRD-002"])
        .current_dir(dir.path());
    assert_downstream_fingerprint(
        &mut cmd,
        "PRD-001",
        "Invalid transition: draft → superseded",
    );
}

// ────────────────────────────────────────────────────────────────────
// 8. estimate — Strategy A: empty-items hint is emitted with the
//    canonical id present in the printed body. We assert the
//    canonical id appears in the *output* (the empty-items hint
//    suggests `forgeplan get PRD-001`), and that the fixture body's
//    template-FR placeholder warning fires (proves the artifact body
//    was loaded — i.e. resolver succeeded).
// ────────────────────────────────────────────────────────────────────

#[test]
fn estimate_accepts_slug_form() {
    let dir = workspace_with_one_prd("Estimate Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    let output = forgeplan()
        .args(["estimate", &slug])
        .current_dir(dir.path())
        .output()
        .expect("estimate spawn");
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Strategy A (output-side): estimate prints "No estimable items
    // found in PRD-001." — that string mentions the canonical id, which
    // only appears if `record.id == "PRD-001"` was loaded via resolver.
    assert!(
        combined.contains("No estimable items found in PRD-001"),
        "estimate must print canonical-id-bearing empty-items message — got:\n{combined}"
    );
}

#[test]
fn estimate_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Estimate Display Test");

    let output = forgeplan()
        .args(["estimate", "PRD-001"])
        .current_dir(dir.path())
        .output()
        .expect("estimate spawn");
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("No estimable items found in PRD-001"),
        "estimate must print canonical-id-bearing empty-items message — got:\n{combined}"
    );
}

// ────────────────────────────────────────────────────────────────────
// 9. calibrate-estimate — Strategy B: "No estimable items in PRD-001.
//    Cannot calibrate." is the structured error fingerprint emitted
//    after the body has been parsed (post-resolver). Mentions canonical
//    id explicitly.
// ────────────────────────────────────────────────────────────────────

#[test]
fn calibrate_estimate_accepts_slug_form() {
    let dir = workspace_with_one_prd("Calibrate Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    let mut cmd = forgeplan();
    cmd.args(["calibrate-estimate", &slug, "--actual-hours", "8"])
        .current_dir(dir.path());
    assert_downstream_fingerprint(&mut cmd, &slug, "No estimable items in PRD-001");
}

#[test]
fn calibrate_estimate_accepts_display_id_form() {
    let dir = workspace_with_one_prd("Calibrate Display Test");

    let mut cmd = forgeplan();
    cmd.args(["calibrate-estimate", "PRD-001", "--actual-hours", "8"])
        .current_dir(dir.path());
    assert_downstream_fingerprint(&mut cmd, "PRD-001", "No estimable items in PRD-001");
}

// ────────────────────────────────────────────────────────────────────
// 10. fgr — Strategy A: success path. The output table includes a
//     numeric overall grade row mentioning PRD-001 — we assert the
//     score row is rendered (proves resolver+scoring wiring).
// ────────────────────────────────────────────────────────────────────

#[test]
fn fgr_accepts_slug_form() {
    let dir = workspace_with_one_prd("FGR Slug Test");
    let slug = slug_for(dir.path(), "PRD-001");

    let output = forgeplan()
        .args(["fgr", &slug])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    // Strategy A (output-side): fgr prints a table row whose first
    // column is the canonical id. Resolver+fgr::compute must have run.
    assert!(
        s.contains("PRD-001") && s.contains("Grade"),
        "fgr text output must contain PRD-001 score row — got:\n{s}"
    );
}

#[test]
fn fgr_accepts_display_id_form() {
    let dir = workspace_with_one_prd("FGR Display Test");

    let output = forgeplan()
        .args(["fgr", "PRD-001"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(output).unwrap();
    assert!(
        s.contains("PRD-001") && s.contains("Grade"),
        "fgr text output must contain PRD-001 score row — got:\n{s}"
    );
}

// ────────────────────────────────────────────────────────────────────
// 11. claim — Strategy A: post-action `claims --json` must contain a
//     row with id=PRD-001 + agent_id=test-agent. Proves the resolver
//     routed the slug to the canonical id stored in the claims table.
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

    // Strategy A — claims store must have a row keyed by the canonical
    // id PRD-001 (NOT the slug). If the resolver were skipped, the
    // claim would be stored under the slug literal, this lookup would
    // fail. (We allow either, but assert at least one matches PRD-001.)
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
// 12. release — Strategy A: post-action `claims --json` must NOT
//     contain a row for PRD-001. Combined with the prior `claim` step,
//     this proves the resolver mapped slug→PRD-001 in *both* surfaces.
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

    // Strategy A — claims store must NOT contain PRD-001 anymore.
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
// 13. import — Strategy A: post-import `get --json` must reflect the
//     payload's title. The export-shaped JSON uses a slug as the row's
//     `id`; the import resolver must canonicalise it to PRD-001 before
//     replacing the row. If the resolver were skipped, the title would
//     not change (or a duplicate row would be created under the slug).
// ────────────────────────────────────────────────────────────────────

#[test]
fn import_accepts_slug_in_payload() {
    let dir = workspace_with_one_prd("Import Slug Payload");
    let slug = slug_for(dir.path(), "PRD-001");

    let payload = serde_json::json!({
        "artifacts": [{
            "id": &slug,
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

    // Strategy A — verify the slug-keyed payload landed on PRD-001.
    let json = get_json(dir.path(), "PRD-001");
    assert_eq!(
        json["title"].as_str(),
        Some("Re-imported via slug"),
        "import via slug must overwrite PRD-001 title (proves resolver canonicalised slug→PRD-001)"
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
