//! Source-view behaviour: L01, L02, L03, L05, L08.
//!
//! Every test builds real canonical markdown in a temporary `.forgeplan`
//! workspace and asserts on what the capture *observed* — bytes, digests,
//! exact relation sets, coverage counts and diagnostic codes. Nothing here
//! asserts on wording, forwarding or the mere absence of an error.
//!
//! These are pure read-path scenarios: no index, no owner lock, no model.
//! That is deliberate — L01/L02/L03/L05/L08 are all decided by the
//! stable-capture criterion, and a test that needed a database to observe
//! them would be observing the database.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use forgeplan_core::constellation::DiagnosticCode;
use forgeplan_core::constellation::capture::{
    CaptureLimits, CaptureRequest, DEFAULT_CAPTURE_ATTEMPTS, DEFAULT_CAPTURE_BUDGET_MS,
    FilesystemSupport, capture_source,
};
use forgeplan_core::constellation::jobs::SemanticState;
use forgeplan_core::constellation::manifest::GenerationManifest;
use forgeplan_core::constellation::reconciler::{DeltaAction, plan_reconcile};
use forgeplan_core::workspace::ARTIFACT_DIRS;

/// A workspace with every artifact directory present and an empty config.
fn workspace() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    for dir in ARTIFACT_DIRS {
        std::fs::create_dir_all(tmp.path().join(dir)).expect("artifact dir");
    }
    tmp
}

fn write(root: &Path, dir: &str, name: &str, content: &str) -> PathBuf {
    let path = root.join(dir).join(name);
    std::fs::write(&path, content).expect("write artifact");
    path
}

async fn capture(root: &Path) -> forgeplan_core::constellation::capture::CapturedSourceView {
    capture_source(&CaptureRequest::new(root), CaptureLimits::default())
        .await
        .expect("two agreeing passes over a static workspace")
}

async fn capture_after(
    root: &Path,
    previous: forgeplan_core::constellation::capture::CapturedSourceView,
) -> forgeplan_core::constellation::capture::CapturedSourceView {
    capture_source(
        &CaptureRequest::new(root).with_previous(Arc::new(previous)),
        CaptureLimits::default(),
    )
    .await
    .expect("two agreeing passes over a static workspace")
}

/// Artifact markdown with an explicit body and optional extra frontmatter.
fn artifact(id: &str, title: &str, status: &str, extra: &str, body: &str) -> String {
    format!(
        "---\nid: {id}\nkind: prd\nstatus: {status}\ntitle: {title}\ndepth: standard\n{extra}---\n\n{body}"
    )
}

fn edges(
    view: &forgeplan_core::constellation::capture::CapturedSourceView,
    key: &str,
) -> Vec<(String, String)> {
    view.artifact(key)
        .expect("artifact is present")
        .outgoing
        .iter()
        .map(|e| (e.target.clone(), e.relation.clone()))
        .collect()
}

// ---------------------------------------------------------------------------
// L01 — metadata-only change
// ---------------------------------------------------------------------------

#[tokio::test]
async fn l01_metadata_only_change_updates_the_projection_without_touching_the_body() {
    let tmp = workspace();
    let body = "## Summary\n\nUnchanged body text.\n";
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", "tags:\n  - source=code\n", body),
    );
    let before = capture(tmp.path()).await;

    // Only status and tags move in the file; the body is byte-identical.
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact(
            "PRD-001",
            "Auth",
            "active",
            "tags:\n  - source=code\n  - source=spec\n",
            body,
        ),
    );
    let after = capture_after(tmp.path(), before.clone()).await;

    let artifact = after.artifact("PRD-001").expect("artifact survives");
    assert_eq!(artifact.status, "active", "the status change is observed");
    assert_eq!(artifact.tags, vec!["source=code", "source=spec"]);
    assert_eq!(&*artifact.body, body, "the body was not touched");

    // The whole projection moved even though the text did not.
    assert_ne!(
        before.entries[0].source_digest,
        after.entries[0].source_digest
    );
    // Metadata is not a new incarnation of the same file.
    assert_eq!(after.entries[0].incarnation, before.entries[0].incarnation);
    assert_eq!(
        after.entries[0].row_revision,
        before.entries[0].row_revision + 1
    );
}

#[tokio::test]
async fn l01_a_metadata_only_change_is_seen_as_an_update_not_as_unchanged() {
    let tmp = workspace();
    let file = write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", "", "Body.\n"),
    );
    let before = capture(tmp.path()).await;
    let published = GenerationManifest::from_capture(
        &before,
        "gen-1",
        forgeplan_core::constellation::manifest::TableVersions::new(1, 1),
    );

    std::fs::write(&file, artifact("PRD-001", "Auth", "active", "", "Body.\n")).unwrap();
    let after = capture(tmp.path()).await;

    let plan = plan_reconcile(&after, Some(&published));
    let delta = plan.for_path("prds/PRD-001-auth.md").expect("delta");
    assert_eq!(
        delta.action,
        DeltaAction::Update,
        "a frontmatter-only edit is still a byte change and must be staged"
    );
}

// ---------------------------------------------------------------------------
// L02 — empty body
// ---------------------------------------------------------------------------

#[tokio::test]
async fn l02_replacing_a_body_with_an_empty_one_is_a_change() {
    let tmp = workspace();
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", "", "Secret needle phrase.\n"),
    );
    let before = capture(tmp.path()).await;
    assert_eq!(
        &*before.artifact("PRD-001").unwrap().body,
        "Secret needle phrase.\n"
    );

    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", "", ""),
    );
    let after = capture(tmp.path()).await;

    let artifact = after
        .artifact("PRD-001")
        .expect("artifact is still current");
    assert_eq!(&*artifact.body, "", "an empty body is content, not absence");
    assert_ne!(
        before.entries[0].source_digest, after.entries[0].source_digest,
        "the emptied body must not look like 'no update'"
    );
    assert!(
        !after
            .artifacts
            .iter()
            .any(|a| a.body.contains("Secret needle phrase")),
        "the removed text must be gone from every captured body"
    );
    assert!(
        after.entries[0].input_digest != before.entries[0].input_digest,
        "an emptied body changes the embedding input and not just the bytes"
    );
}

// ---------------------------------------------------------------------------
// L03 — exact outgoing set
// ---------------------------------------------------------------------------

#[tokio::test]
async fn l03_outgoing_is_replaced_exactly_without_union_residue() {
    let tmp = workspace();
    let links = "links:\n  - target: RFC-010\n    relation: informs\n  - target: ADR-011\n    relation: based_on\n  - target: EVID-012\n    relation: supports\n";
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", links, "Body.\n"),
    );
    let before = capture(tmp.path()).await;
    assert_eq!(
        edges(&before, "PRD-001"),
        vec![
            ("ADR-011".to_string(), "based_on".to_string()),
            ("EVID-012".to_string(), "supports".to_string()),
            ("RFC-010".to_string(), "informs".to_string()),
        ]
    );

    // B (ADR-011) removed; C (EVID-012) retargeted to a different artifact.
    let updated = "links:\n  - target: RFC-010\n    relation: informs\n  - target: EVID-999\n    relation: supports\n";
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", updated, "Body.\n"),
    );
    let after = capture(tmp.path()).await;

    assert_eq!(
        edges(&after, "PRD-001"),
        vec![
            ("EVID-999".to_string(), "supports".to_string()),
            ("RFC-010".to_string(), "informs".to_string()),
        ],
        "the outgoing set is replaced exactly: A and C' only, no residue of B"
    );
}

#[tokio::test]
async fn l03_a_target_keeps_its_exact_case_and_is_not_required_to_exist() {
    let tmp = workspace();
    let links = "links:\n  - target: rfc-010\n    relation: informs\n  - target: OTHER-WORKSPACE-77\n    relation: references\n";
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", links, "Body.\n"),
    );
    let view = capture(tmp.path()).await;

    assert_eq!(
        edges(&view, "PRD-001"),
        vec![
            ("OTHER-WORKSPACE-77".to_string(), "references".to_string()),
            ("rfc-010".to_string(), "informs".to_string()),
        ],
        "targets are neither lower-cased nor required to resolve locally"
    );
}

#[tokio::test]
async fn l03_a_malformed_edge_is_diagnosed_and_never_silently_dropped() {
    let tmp = workspace();
    let links = "links:\n  - target: RFC-010\n    relation: informs\n  - target: ADR-011\n";
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", links, "Body.\n"),
    );
    let view = capture(tmp.path()).await;

    let artifact = view.artifact("PRD-001").expect("still current");
    assert_eq!(
        edges(&view, "PRD-001"),
        vec![("RFC-010".to_string(), "informs".to_string())],
        "the valid edge survives"
    );
    assert!(
        artifact
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::InvalidLinks),
        "the malformed edge must be reported, not silently discarded"
    );
    assert!(
        view.entries[0]
            .diagnostic_codes
            .contains(&"invalid_links".to_string()),
        "the diagnostic must reach the durable source entry"
    );
}

// ---------------------------------------------------------------------------
// L05 — same mtime, same size, different bytes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn l05_same_size_and_mtime_different_bytes_is_detected() {
    let tmp = workspace();
    let path = write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", "", "alpha text\n"),
    );
    let before = capture(tmp.path()).await;

    let mtime = std::fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    let length_before = std::fs::metadata(&path).expect("metadata").len();
    std::fs::write(
        &path,
        artifact("PRD-001", "Auth", "draft", "", "bravo text\n"),
    )
    .unwrap();
    assert_eq!(
        std::fs::metadata(&path).expect("metadata").len(),
        length_before,
        "the fixture must change without changing size"
    );
    std::fs::File::options()
        .write(true)
        .open(&path)
        .expect("open")
        .set_modified(mtime)
        .expect("restore mtime");

    let after = capture(tmp.path()).await;

    assert_ne!(
        before.entries[0].source_digest, after.entries[0].source_digest,
        "content digest, not size or mtime, is the identity"
    );
    assert_ne!(
        before.source_set_digest, after.source_set_digest,
        "the source set digest must move with it"
    );
    assert_eq!(
        &*after.artifact("PRD-001").unwrap().body,
        "bravo text\n",
        "the new bytes are what the reader will see"
    );
}

#[tokio::test]
async fn l05_identical_bytes_capture_to_the_same_source_set_digest() {
    let tmp = workspace();
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", "", "Body.\n"),
    );
    let first = capture(tmp.path()).await;
    let second = capture(tmp.path()).await;

    assert_eq!(
        first.source_set_digest, second.source_set_digest,
        "the digest describes the bytes, not the instant"
    );
    assert_eq!(
        first.snapshot_id.len(),
        second.snapshot_id.len(),
        "snapshot ids are fixed-width digests"
    );
}

#[tokio::test]
async fn l05_the_source_set_digest_separates_paths_of_equal_length() {
    let tmp = workspace();
    write(
        tmp.path(),
        "prds",
        "PRD-001-a.md",
        &artifact("PRD-001", "Auth", "draft", "", "Body.\n"),
    );
    let before = capture(tmp.path()).await;

    // Same directory, same file-name length, different path.
    std::fs::rename(
        tmp.path().join("prds").join("PRD-001-a.md"),
        tmp.path().join("prds").join("PRD-001-b.md"),
    )
    .unwrap();
    let after = capture(tmp.path()).await;

    assert_ne!(
        before.source_set_digest, after.source_set_digest,
        "paths are length-prefixed, so a one-character rename cannot be \
         absorbed into the digest"
    );
    assert_eq!(
        before.entries[0].source_digest, after.entries[0].source_digest,
        "the bytes themselves did not change"
    );
}

// ---------------------------------------------------------------------------
// L08 — unreadable / invalid source is not a delete
// ---------------------------------------------------------------------------

#[tokio::test]
async fn l08_invalid_frontmatter_is_diagnosed_and_is_not_a_delete() {
    let tmp = workspace();
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", "", "Good body.\n"),
    );
    write(
        tmp.path(),
        "prds",
        "PRD-002-broken.md",
        &artifact("PRD-002", "Broken", "draft", "", "Never read.\n"),
    );
    let before = capture(tmp.path()).await;
    let published = GenerationManifest::from_capture(
        &before,
        "gen-1",
        forgeplan_core::constellation::manifest::TableVersions::new(2, 2),
    );
    assert!(published.coverage.is_complete());

    // The file becomes unparseable, but it is still on disk.
    write(
        tmp.path(),
        "prds",
        "PRD-002-broken.md",
        "---\nid: [unclosed\n",
    );

    let after = capture(tmp.path()).await;

    assert_eq!(
        after.coverage.expected, 2,
        "both paths are still enumerated"
    );
    assert_eq!(after.coverage.read, 1, "only the readable one is current");
    assert_eq!(after.coverage.invalid, 1);
    assert!(
        !after.is_complete(),
        "an unreadable source is incomplete coverage"
    );

    let diagnostic = after
        .diagnostics
        .iter()
        .find(|d| d.code == DiagnosticCode::UnreadableSource)
        .expect("a per-source diagnostic naming the file");
    assert_eq!(diagnostic.source.as_deref(), Some("prds/PRD-002-broken.md"));
    assert!(after.is_unusable("prds/PRD-002-broken.md"));
    assert!(
        after.artifact("PRD-002").is_none(),
        "an unreadable source is excluded from current results"
    );
    assert!(
        after.artifact("PRD-001").is_some(),
        "one broken neighbour must not evict a healthy artifact"
    );

    let plan = plan_reconcile(&after, Some(&published));
    assert_eq!(
        plan.count(DeltaAction::Delete),
        0,
        "an unreadable file is not a confirmed deletion"
    );
    let delta = plan
        .for_path("prds/PRD-002-broken.md")
        .expect("the retained row is in the plan");
    assert_eq!(delta.action, DeltaAction::Retain);
    assert!(
        !plan.publishable,
        "a partial capture is never published as complete"
    );
}

#[tokio::test]
async fn l08_a_file_that_vanishes_from_a_complete_capture_is_a_delete() {
    let tmp = workspace();
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", "", "Body.\n"),
    );
    write(
        tmp.path(),
        "prds",
        "PRD-002-second.md",
        &artifact("PRD-002", "Second", "draft", "", "Body.\n"),
    );
    let before = capture(tmp.path()).await;
    let published = GenerationManifest::from_capture(
        &before,
        "gen-1",
        forgeplan_core::constellation::manifest::TableVersions::new(2, 2),
    );

    std::fs::remove_file(tmp.path().join("prds").join("PRD-002-second.md")).unwrap();
    let after = capture(tmp.path()).await;

    let plan = plan_reconcile(&after, Some(&published));
    assert_eq!(
        plan.count(DeltaAction::Delete),
        1,
        "a complete capture that cannot see the file is evidence of absence"
    );
    assert_eq!(plan.tombstones.len(), 1);
    assert_eq!(plan.tombstones[0].canonical_key, "PRD-002");
    assert!(plan.publishable);
}

// ---------------------------------------------------------------------------
// Capture envelope
// ---------------------------------------------------------------------------

#[tokio::test]
async fn capture_bounds_are_the_agreed_project_parameters() {
    assert_eq!(DEFAULT_CAPTURE_ATTEMPTS, 2);
    assert_eq!(DEFAULT_CAPTURE_BUDGET_MS, 5000);
    let limits = CaptureLimits::default();
    assert_eq!(limits.max_attempts, 2);
    assert_eq!(limits.budget.as_millis(), 5000);
}

#[tokio::test]
async fn a_confirmed_capture_of_an_empty_workspace_is_complete_and_empty() {
    let tmp = workspace();
    let view = capture(tmp.path()).await;
    assert!(view.confirmed);
    assert!(view.entries.is_empty());
    assert_eq!(view.coverage.expected, 0);
    assert!(view.is_complete());
    assert_eq!(view.filesystem_support, FilesystemSupport::Supported);
    assert_eq!(
        view.attempts, 1,
        "a static workspace confirms on the first attempt"
    );
}

#[tokio::test]
async fn a_capture_with_no_profile_reports_semantics_as_unavailable_not_failed() {
    let tmp = workspace();
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", "", "Body.\n"),
    );
    let view = capture(tmp.path()).await;
    let entry = &view.entries[0];

    assert_eq!(
        entry.semantic_state,
        SemanticState::Unavailable,
        "no model profile is resolved yet, and that is not a failure"
    );
    assert_eq!(
        entry.profile_id, None,
        "an unresolved profile must not be invented"
    );
    assert!(
        entry.input_digest.is_some(),
        "the input digest is always known"
    );
}

#[tokio::test]
async fn missing_timestamps_stay_missing_instead_of_becoming_now() {
    let tmp = workspace();
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact("PRD-001", "Auth", "draft", "", "Body.\n"),
    );
    let first = capture(tmp.path()).await;
    assert_eq!(first.artifact("PRD-001").unwrap().created_at, None);
    assert_eq!(first.artifact("PRD-001").unwrap().updated_at, None);

    // A second scan minutes later must not invent a timestamp; if it did, the
    // row would look edited on every pass.
    let second = capture(tmp.path()).await;
    assert_eq!(second.artifact("PRD-001").unwrap().created_at, None);
    assert_eq!(second.artifact("PRD-001").unwrap().updated_at, None);
    assert_eq!(
        first.entries[0].row_revision,
        second.entries[0].row_revision
    );
}

#[tokio::test]
async fn unknown_frontmatter_keys_survive_the_capture() {
    let tmp = workspace();
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &artifact(
            "PRD-001",
            "Auth",
            "draft",
            "last_modified_by: omp-coding-agent\ndomain:\n  - payments\n",
            "Body.\n",
        ),
    );
    let view = capture(tmp.path()).await;
    let frontmatter = &view.artifact("PRD-001").unwrap().frontmatter;

    assert_eq!(
        frontmatter.get("last_modified_by").and_then(|v| v.as_str()),
        Some("omp-coding-agent"),
        "an agent-owned field nobody models must not be dropped"
    );
    assert!(frontmatter.contains_key("domain"));
}

#[tokio::test]
async fn a_duplicate_identity_excludes_both_files_and_never_picks_a_winner() {
    let tmp = workspace();
    write(
        tmp.path(),
        "prds",
        "PRD-001-first.md",
        &artifact("PRD-001", "Auth", "draft", "", "From the prds directory.\n"),
    );
    write(
        tmp.path(),
        "rfcs",
        "PRD-001-second.md",
        &artifact("PRD-001", "Auth", "draft", "", "From the rfcs directory.\n"),
    );

    let view = capture(tmp.path()).await;

    assert_eq!(view.coverage.expected, 2);
    assert_eq!(view.coverage.read, 0);
    assert_eq!(view.coverage.excluded, 2);
    assert!(
        view.artifacts.is_empty(),
        "choosing one of two files claiming an identity would publish one \
         file's bytes under the other's identity"
    );
    assert!(
        view.diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::IdentityCollision)
    );
}
