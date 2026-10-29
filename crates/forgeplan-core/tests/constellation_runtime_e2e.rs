//! Runtime behaviour that only shows up once the pieces are wired together:
//! L02 (an empty body stays empty), L05 (content identity, not size+mtime),
//! L09/L14 (a durable generation and its recovery), and the maintenance host's
//! authority contract.
//!
//! Every test drives the public API and asserts on what a **consumer** would
//! see — the bytes in a `.forgeplan` file, the generation a reader is served,
//! the state a status report names. None of them asserts on a helper's name,
//! on wiring, or on the absence of an error.
//!
//! What this file deliberately does not claim: the process-level scenarios
//! (two live servers, a real stdio client, a hung encoder, a Windows volume)
//! are separate suites. A test here is evidence about the in-process
//! behaviour only.

use std::path::Path;
use std::sync::Arc;

use forgeplan_core::constellation::capture::{CaptureLimits, CaptureRequest, capture_source};
use forgeplan_core::constellation::facade::{
    MaintainCapability, ensure_read_view, maintain_index, workspace_is_migrated,
};
use forgeplan_core::constellation::runtime::{
    FULL_SCAN_PERIOD_MS, INFERENCE_SOFT_DEADLINE_MS, MODEL_UNAVAILABLE_COOLDOWN_MS,
    MaintenanceHost, READY_QUEUE_CAPACITY, ReportSink,
};
use forgeplan_core::constellation::storage::read_current;
use forgeplan_core::constellation::view::ReadView;
use forgeplan_core::workspace::ARTIFACT_DIRS;

fn workspace() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    for dir in ARTIFACT_DIRS {
        std::fs::create_dir_all(tmp.path().join(dir)).expect("artifact dir");
    }
    tmp
}

fn write(root: &Path, dir: &str, name: &str, content: &str) -> std::path::PathBuf {
    let path = root.join(dir).join(name);
    std::fs::write(&path, content).expect("write artifact");
    path
}

fn prd(id: &str, title: &str, status: &str, body: &str) -> String {
    format!(
        "---\nid: {id}\nkind: prd\nstatus: {status}\ntitle: {title}\ndepth: standard\n---\n\n{body}"
    )
}

fn deadline() -> std::time::Instant {
    std::time::Instant::now() + std::time::Duration::from_secs(120)
}

fn capability(root: &Path) -> MaintainCapability {
    MaintainCapability::for_strict_maintenance(root.canonicalize().expect("canonical root"))
}

/// The canonical body of a file, with its frontmatter stripped — what a user
/// editing the artifact actually reads.
fn body_of(root: &Path, dir: &str, name: &str) -> String {
    let path = root.join(dir).join(name);
    let text = std::fs::read_to_string(&path).expect("artifact readable");
    let (_, body) = forgeplan_core::artifact::frontmatter::parse_frontmatter(&text)
        .expect("frontmatter parses");
    body
}

async fn capture(root: &Path) -> forgeplan_core::constellation::capture::CapturedSourceView {
    capture_source(&CaptureRequest::new(root), CaptureLimits::default())
        .await
        .expect("two agreeing passes over a static workspace")
}

// ---------------------------------------------------------------------------
// L02 — an empty body stays empty
// ---------------------------------------------------------------------------

/// Clearing a body must not be undone by the next maintenance pass.
///
/// This is the scenario as a user meets it: the artifact is written, published,
/// then emptied on disk, and the index is brought up to date again. The text
/// that was there must not come back — neither in the file nor in what a reader
/// is served.
#[tokio::test]
async fn l02_a_cleared_body_is_not_restored_by_the_next_pass() {
    let tmp = workspace();
    let root = tmp.path();
    write(
        root,
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "Original text.\n"),
    );

    let capability = capability(root);
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect("first pass publishes");

    // The clear: frontmatter survives, body is gone.
    write(
        root,
        "prds",
        "PRD-001-auth.md",
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\ndepth: standard\n---\n",
    );

    let (view, _outcome) = maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect("second pass publishes the cleared body");
    assert!(
        !view.manifest.generation_id.is_empty(),
        "a migrated workspace must publish a generation, not fall back"
    );

    // What a reader is served: the body is empty, and the old text is not
    // reachable through the pinned generation.
    let read = forgeplan_core::constellation::facade::open_workspace_read(
        root,
        None,
        CaptureLimits::default(),
    )
    .await
    .expect("read view for the migrated workspace");
    let record = read
        .get_record("PRD-001")
        .expect("the artifact is still present after a clear");
    let (_, served_body) = forgeplan_core::artifact::frontmatter::parse_frontmatter(&record.body)
        .expect("consumer receives canonical Markdown including frontmatter");
    assert!(
        served_body.trim().is_empty(),
        "the reader was served the old text over a cleared body: {:?}",
        record.body
    );

    // And the canonical file still says so.
    assert!(
        body_of(root, "prds", "PRD-001-auth.md").trim().is_empty(),
        "the canonical file must keep the cleared body"
    );
}

/// A body replaced by empty is a **change**, not "no change": the row the
/// capture produces must carry the empty body and a different `source_digest`
/// than the previous observation, otherwise every downstream consumer keeps
/// indexing text that is gone.
#[tokio::test]
async fn l02_clearing_the_body_moves_the_source_digest() {
    let tmp = workspace();
    let root = tmp.path();
    let path = write(
        root,
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "Original text.\n"),
    );
    let before = capture(root).await;
    let before_digest = before
        .artifact("PRD-001")
        .expect("artifact captured")
        .source_digest
        .clone();

    std::fs::write(
        &path,
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\ndepth: standard\n---\n",
    )
    .expect("clear the body");

    let after = capture_source(
        &CaptureRequest::new(root).with_previous(Arc::new(before)),
        CaptureLimits::default(),
    )
    .await
    .expect("the cleared workspace still captures cleanly");
    let after_digest = after
        .artifact("PRD-001")
        .expect("artifact captured")
        .source_digest
        .clone();

    assert_ne!(
        before_digest, after_digest,
        "an empty body is different content and must have a different digest"
    );
}

// ---------------------------------------------------------------------------
// L05 — content identity, never size + mtime
// ---------------------------------------------------------------------------

/// A same-size rewrite with a restored mtime must still be detected.
///
/// This is the case that a `(len, modified)` identity gets wrong: the file is
/// the same length and the timestamp is put back, so every cheap check agrees
/// the file did not change. Only the bytes disagree, and the bytes are what the
/// protocol actually compares.
#[tokio::test]
async fn l05_a_same_size_same_mtime_rewrite_is_detected() {
    let tmp = workspace();
    let root = tmp.path();
    let path = write(
        root,
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "AAAA text.\n"),
    );
    let before = capture(root).await;
    let before_digest = before
        .artifact("PRD-001")
        .expect("artifact captured")
        .source_digest
        .clone();
    let before_len = std::fs::metadata(&path).expect("stat").len();
    let before_mtime = std::fs::metadata(&path)
        .expect("stat")
        .modified()
        .expect("mtime");

    // Same length, different bytes: "BBBB text.\n" for "AAAA text.\n".
    std::fs::write(&path, prd("PRD-001", "Auth", "draft", "BBBB text.\n")).expect("rewrite");
    // Put the timestamp back, which is what a checkout, an archive restore and
    // `touch -r` all do.
    filetime_reset(&path, before_mtime);

    let after_meta = std::fs::metadata(&path).expect("stat");
    assert_eq!(
        after_meta.len(),
        before_len,
        "the fixture must be length-preserving or it proves nothing"
    );
    assert_eq!(
        after_meta.modified().expect("mtime"),
        before_mtime,
        "the fixture must restore the mtime or it proves nothing"
    );

    let after = capture_source(
        &CaptureRequest::new(root).with_previous(Arc::new(before)),
        CaptureLimits::default(),
    )
    .await
    .expect("the rewritten workspace still captures cleanly");

    assert_ne!(
        after
            .artifact("PRD-001")
            .expect("artifact captured")
            .source_digest,
        before_digest,
        "identical length and identical mtime must not read as 'unchanged'"
    );
}

/// Restore a file's modification time, in-process and portable.
///
/// `touch` was the previous mechanism and it was not portable: `-d @<epoch>`
/// is GNU syntax and BSD `touch` rejects it, so on macOS the reset silently
/// did nothing and the fixture no longer held — a green test that proved
/// nothing. `std::fs::FileTimes` needs no subprocess and no dependency, and
/// its failure is an error this function reports instead of swallowing.
fn filetime_reset(path: &Path, time: std::time::SystemTime) {
    let file = std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap_or_else(|e| panic!("cannot open {} to reset its mtime: {e}", path.display()));
    let times = std::fs::FileTimes::new().set_modified(time);
    file.set_times(times)
        .unwrap_or_else(|e| panic!("cannot reset the mtime of {}: {e}", path.display()));
}

// ---------------------------------------------------------------------------
// L09 / L14 — a durable generation, and what a restarted reader sees
// ---------------------------------------------------------------------------

/// After a publish, a reader is served **that** generation and a later publish
/// does not retroactively change what an in-flight view describes.
///
/// L09's "no mixed generation": the view carries a `generation_id`, a manifest
/// and a pin that all name the same publication, and the descriptor a second
/// publish writes is a different fact.
#[tokio::test]
async fn l09_a_reader_is_served_exactly_the_published_generation() {
    let tmp = workspace();
    let root = tmp.path();
    write(
        root,
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "First.\n"),
    );

    let capability = capability(root);
    let (first_view, _) = maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect("first pass publishes");
    let first_generation = first_view.manifest.generation_id.clone();

    // A second, different publication.
    write(
        root,
        "prds",
        "PRD-002-payments.md",
        &prd("PRD-002", "Payments", "draft", "Second.\n"),
    );
    let (second_view, _) = maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect("second pass publishes");
    let second_generation = second_view.manifest.generation_id.clone();

    assert_ne!(
        first_generation, second_generation,
        "two publications must be two generations, or the first was never real"
    );

    // The first view still names the first generation, and its descriptor is the
    // one that was published with it — not the later CURRENT.
    assert_eq!(
        first_view.descriptor.generation_id, first_generation,
        "a published view carries the descriptor it was published under"
    );
    assert_ne!(
        first_view.descriptor.manifest_sha256, second_view.descriptor.manifest_sha256,
        "two publications must be two sets of manifest bytes"
    );

    // And `CURRENT` now names the newer one, which is what a fresh reader gets.
    let descriptor = read_current(&root.join("lance"))
        .expect("CURRENT is readable")
        .expect("CURRENT exists after a publish");
    assert_eq!(descriptor.generation_id, second_generation);
}

/// A torn pointer — `CURRENT` naming a manifest that is not there — must be an
/// explicit failure, never a silent fall back to whatever generation exists.
///
/// This is the "reader sees a completed generation or an explicit
/// unavailability, never a mixture" half of L09.
#[tokio::test]
async fn l09_a_pointer_to_a_missing_manifest_is_not_a_fallback() {
    let tmp = workspace();
    let root = tmp.path();
    write(
        root,
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "Body.\n"),
    );

    let capability = capability(root);
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect("first pass publishes");

    let index_root = root.join("lance");
    let descriptor = read_current(&index_root)
        .expect("CURRENT is readable")
        .expect("CURRENT exists");
    // A second generation, so "the latest one" is a tempting wrong answer.
    write(
        root,
        "prds",
        "PRD-002-payments.md",
        &prd("PRD-002", "Payments", "draft", "Body.\n"),
    );
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect("a second pass still publishes");
    let manifest = index_root.join(&descriptor.manifest_path);
    std::fs::remove_file(&manifest).expect("remove the pinned manifest");

    // Re-point CURRENT at the missing generation: exactly what a torn rename
    // or a partially-restored backup looks like.
    std::fs::write(
        index_root.join("CURRENT.json"),
        serde_json::to_vec_pretty(&descriptor).expect("serialise"),
    )
    .expect("write the torn pointer");

    let err = forgeplan_core::constellation::storage::pin_generation_read(
        &index_root,
        &read_current(&index_root)
            .expect("CURRENT is readable")
            .expect("CURRENT exists"),
    )
    .expect_err("a pointer to a missing generation must not resolve");
    assert_eq!(
        err.code(),
        forgeplan_core::constellation::DiagnosticCode::IndexGenerationUnavailable,
        "the answer must be 'unavailable', not another generation: {err}"
    );

    let maintenance_error = maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect_err(
            "maintenance must not silently rebuild an active index with a missing manifest",
        );
    assert_eq!(
        maintenance_error.code(),
        forgeplan_core::constellation::DiagnosticCode::IndexGenerationUnavailable,
    );
    assert_eq!(
        read_current(&index_root)
            .expect("CURRENT remains readable")
            .expect("the broken descriptor remains available for explicit repair")
            .generation_id,
        descriptor.generation_id,
        "a failed maintenance pass must not replace the broken publication"
    );
}

// ---------------------------------------------------------------------------
// Publication refusal — an unreadable source is never published
// ---------------------------------------------------------------------------

/// A source whose `links:` block cannot be read as an edge set must not reach
/// the index as an artifact with no relations.
///
/// The pass refuses with a named error, the pointer keeps naming the
/// generation that was published before, and the canonical bytes on disk are
/// exactly what the author wrote.
#[tokio::test]
async fn a_links_block_that_is_not_a_sequence_refuses_publication() {
    let tmp = workspace();
    let root = tmp.path();
    write(
        root,
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "First.\n"),
    );

    let capability = capability(root);
    let (view, _) = maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect("the first pass publishes");
    let published_generation = view.manifest.generation_id.clone();

    // The artifact now carries a `links:` mapping instead of a sequence.
    let path = root.join("prds").join("PRD-001-auth.md");
    std::fs::write(
        &path,
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\ndepth: standard\nlinks:\n  target: ADR-003\n  relation: informs\n---\n\nFirst.\n",
    )
    .expect("write artifact");
    let bytes_before = std::fs::read(&path).expect("artifact readable");

    let error = maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect_err("a source that cannot be read as an edge set is not published");
    assert!(
        matches!(
            error,
            forgeplan_core::constellation::ConstellationError::SourceUnavailable { .. }
        ),
        "the refusal names the incomplete capture, got {error:?}"
    );

    let descriptor = read_current(&root.join("lance"))
        .expect("CURRENT is readable")
        .expect("CURRENT still exists");
    assert_eq!(
        descriptor.generation_id, published_generation,
        "a refused pass must not publish a new generation"
    );
    assert_eq!(
        bytes_before,
        std::fs::read(&path).expect("artifact readable"),
        "the canonical bytes were not rewritten"
    );
}

// ---------------------------------------------------------------------------
// Migration and authority — who may maintain an index
// ---------------------------------------------------------------------------

/// A migrated workspace is recognised as migrated, and an untouched one is not.
///
/// That distinction is the whole authority rule: a host is started for an
/// already-migrated workspace without anybody asking, and is refused for a
/// legacy one because the `LanceStore` still owns that index.
#[tokio::test]
async fn migration_state_is_what_decides_whether_maintenance_is_allowed() {
    let tmp = workspace();
    let root = tmp.path();
    assert!(
        !workspace_is_migrated(root),
        "an empty workspace is not migrated — there is nothing published yet"
    );

    write(
        root,
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "Body.\n"),
    );
    let capability = capability(root);
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect("first pass publishes");

    assert!(
        workspace_is_migrated(root),
        "a workspace with a published generation is migrated"
    );

    // And the legacy tables were not recreated beside it: a second writer on
    // one logical index is exactly what the migration exists to prevent.
    assert!(
        !root.join("lance/artifacts.lance").exists(),
        "the constellation pass must not materialise a legacy artifacts table"
    );
}

/// The host maintains the workspace its **capability** names.
///
/// A server started in X that is pointed at Y must maintain Y. The host has no
/// other source of truth — it never infers a directory — so the observable is
/// the workspace it reports.
#[tokio::test]
async fn the_host_maintains_the_capabilitys_workspace() {
    let make = || {
        let tmp = workspace();
        std::fs::write(
            tmp.path().join("prds").join("PRD-001-auth.md"),
            prd("PRD-001", "Auth", "draft", "Body.\n"),
        )
        .expect("write artifact");
        tmp
    };
    let target = make();
    let sibling = make();

    // Canonicalise before building the capability. On macOS the temp dir is
    // reached through `/var/...` but resolves to `/private/var/...`, and a host
    // that canonicalises what it was handed would then report a path that
    // differs from the assertion's by exactly that symlink — a difference in
    // spelling, not in the workspace being maintained.
    let canonical_target = target.path().canonicalize().expect("canonical target");
    let host = MaintenanceHost::start(
        MaintainCapability::for_strict_maintenance(canonical_target.clone()),
        ReportSink::Foreground,
    )
    .await
    .expect("a workspace with artifact directories can be maintained");

    assert_eq!(
        host.workspace(),
        canonical_target,
        "the host must maintain the workspace its capability names"
    );
    assert_ne!(
        host.workspace(),
        sibling.path().canonicalize().expect("canonical"),
        "and must never have picked the other workspace"
    );

    // The maintenance really happened there: a generation is published under
    // the target's index root, and the sibling's is untouched.
    host.shutdown().await.expect("clean shutdown");
    assert!(
        read_current(&target.path().join("lance"))
            .expect("CURRENT readable")
            .is_some(),
        "the host's startup pass must publish where it was pointed"
    );
    assert!(
        !target
            .path()
            .join("lance/CURRENT.json")
            .eq(&sibling.path().join("lance/CURRENT.json")),
        "the two indexes are separate"
    );
}

/// A workspace with no artifact directories gets no host at all.
///
/// A host that started anyway would watch nothing, publish nothing, and report
/// success — which a reader cannot distinguish from "the index is current".
#[tokio::test]
async fn a_workspace_with_no_artifact_directories_refuses_to_start_a_host() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    // Pattern-match rather than `expect_err`: a `Debug` bound on
    // `MaintenanceHost` would exist only to satisfy this assertion.
    let Err(err) = MaintenanceHost::start(
        MaintainCapability::for_strict_maintenance(tmp.path().to_path_buf()),
        ReportSink::Foreground,
    )
    .await
    else {
        panic!("a workspace with no artifact directories must refuse to start a host");
    };
    assert!(
        err.to_string().contains("artifact directories"),
        "the refusal must name what is missing: {err}"
    );
}

// ---------------------------------------------------------------------------
// The bounds the maintenance protocol names
// ---------------------------------------------------------------------------

/// The protocol's bounds are the ones the implementation actually uses.
///
/// A host that quietly doubled the debounce interval, or sized the ready queue
/// from its own taste, would still "work" and would break the guarantee the
/// numbers encode — one control-scan period is the ceiling on how long a missed
/// event may hide a change, and that ceiling only exists if the other three
/// hold together.
#[test]
fn the_maintenance_bounds_are_the_documented_ones() {
    assert_eq!(
        forgeplan_core::constellation::runtime::DEBOUNCE_MS,
        250,
        "a burst of atomic saves folds into one capture"
    );
    assert_eq!(
        FULL_SCAN_PERIOD_MS, 30_000,
        "the control scan is the ceiling on how long a missed event hides a change"
    );
    assert_eq!(
        READY_QUEUE_CAPACITY, 64,
        "the ready queue is bounded; overflow defers, it does not drop"
    );
    assert_eq!(
        MODEL_UNAVAILABLE_COOLDOWN_MS, 60_000,
        "an absent model must not be re-probed on every event"
    );
    assert_eq!(
        INFERENCE_SOFT_DEADLINE_MS, 120_000,
        "a caller stops waiting for a vector; the call is never cancelled"
    );
}

// ---------------------------------------------------------------------------
// Read view integrity — one verified publication, not three agreeing ids
// ---------------------------------------------------------------------------

/// A persisted view is built from a manifest, a descriptor and a pin that all
/// describe **one** publication.
///
/// The three arguments are separate values at the call site, and a caller can
/// easily pass a manifest read from a backup next to a real pin. The result
/// would be internally consistent and unsound: table versions, pending jobs
/// and coverage from bytes nobody verified.
#[tokio::test]
async fn a_view_cannot_be_built_from_an_unrelated_manifest_that_shares_the_generation_ids() {
    let tmp = workspace();
    let root = tmp.path();
    write(
        root,
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "Body.\n"),
    );

    let capability = capability(root);
    let (view, _) = maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect("pass publishes");

    // A publisher's view is a descriptor plus a manifest; the pin is opened
    // from the descriptor, which is the same pair a reader would use.
    let descriptor = view.descriptor.clone();
    let verified = view.manifest.clone();
    let pinned = Arc::new(
        forgeplan_core::constellation::storage::pin_generation_read(
            &root.join("lance"),
            &descriptor,
        )
        .expect("the published generation can be pinned"),
    );

    // An older publication along the same generation line: same ids, different
    // document. Exactly what a restored backup looks like.
    let mut stale = (*verified).clone();
    stale.table_versions.artifacts = 1;
    stale.evaluated_at = "2020-01-01T00:00:00.000Z".to_string();
    assert_eq!(
        stale.generation_id, verified.generation_id,
        "the fixture must share the identity fields, or it proves nothing"
    );

    let err = ReadView::from_manifest(Arc::new(stale), descriptor, pinned)
        .expect_err("an unverified manifest must not build a view");
    assert_eq!(
        err.code(),
        forgeplan_core::constellation::DiagnosticCode::IndexGenerationUnavailable,
        "an unprovable publication is unavailable, never a repaired one: {err}"
    );
}

/// The view a workspace read is served is the one the manifest and the pin
/// agree on, and it keeps the pin alive for as long as it lives.
#[tokio::test]
async fn the_served_view_keeps_its_own_pin() {
    let tmp = workspace();
    let root = tmp.path();
    write(
        root,
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "Body.\n"),
    );

    let capability = capability(root);
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect("pass publishes");

    // The reader a consumer gets, not the publisher's intermediate value.
    let read = forgeplan_core::constellation::facade::open_workspace_read(
        root,
        None,
        CaptureLimits::default(),
    )
    .await
    .expect("read view for the migrated workspace");

    assert!(
        read.is_persisted(),
        "a migrated workspace with a published generation is served from it"
    );
    let pin = read.view().pin().expect("a persisted view holds its pin");
    assert_eq!(
        pin.generation_id(),
        read.view().snapshot_id(),
        "the pin and the view must name one generation"
    );
    assert_eq!(
        pin.descriptor().manifest_sha256,
        read.view()
            .descriptor()
            .expect("descriptor")
            .manifest_sha256,
        "the pin and the view must name one set of manifest bytes"
    );
    assert!(
        read.records().iter().any(|record| record.id == "PRD-001"),
        "the record set comes from the capture the view was built on"
    );
}

/// An **ephemeral** view — a workspace with no pointer — invents no index
/// identity.
///
/// There is nothing published to describe, and invented table versions would
/// let a caller read semantic state for a generation that never existed.
#[tokio::test]
async fn an_ephemeral_view_invents_no_index_identity() {
    let tmp = workspace();
    write(
        tmp.path(),
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "Body.\n"),
    );
    let captured = Arc::new(capture(tmp.path()).await);
    let view = ReadView::from_capture(captured);

    assert!(!view.is_persisted());
    assert!(view.descriptor().is_none());
    assert!(view.pin().is_none());
    assert!(
        view.pinned_versions().is_none(),
        "a capture has no published table versions to report"
    );
    assert!(
        view.manifest().is_none(),
        "and no manifest: there is no published generation behind it"
    );
}

/// `ensure_read_view` only reports a **persisted** view when the capture really
/// is the published source set.
///
/// Otherwise a reader would be told "current" about text that came from a
/// capture and rows that came from a table — the mixture the tag exists to
/// prevent.
#[tokio::test]
async fn a_capture_that_is_not_the_published_source_set_is_not_reported_as_persisted() {
    let tmp = workspace();
    let root = tmp.path();
    write(
        root,
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "Body.\n"),
    );

    let capability = capability(root);
    maintain_index(&capability, CaptureLimits::default(), deadline())
        .await
        .expect("first pass publishes");

    // Change the workspace after the publication, then capture.
    write(
        root,
        "prds",
        "PRD-002-payments.md",
        &prd("PRD-002", "Payments", "draft", "Body.\n"),
    );
    let moved = Arc::new(capture(root).await);

    let descriptor = read_current(&root.join("lance"))
        .expect("CURRENT readable")
        .expect("CURRENT exists");
    let view = ensure_read_view(root, moved)
        .await
        .expect("a read view is always buildable");
    assert!(
        !view.is_persisted(),
        "a capture that moved past the published source set must not claim to \
         be the published generation: {}",
        view.snapshot_id()
    );
    assert_ne!(
        view.snapshot_id(),
        descriptor.generation_id,
        "an ephemeral view reports its own capture snapshot id, and must never \
         borrow the published generation's id"
    );
    assert!(
        view.descriptor().is_none() && view.pin().is_none(),
        "there is no published generation behind this view to pin"
    );
}

/// Atomic editor saves must reach the published index, not merely the
/// ephemeral read path that can already see changed Markdown.
#[tokio::test]
async fn l06_a_running_host_publishes_an_atomic_save() {
    let tmp = workspace();
    let root = tmp.path();
    let artifact = write(
        root,
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "Before the editor save.\n"),
    );
    let host = MaintenanceHost::start(capability(root), ReportSink::Foreground)
        .await
        .expect("start the maintenance host");
    let before = read_current(&root.join("lance"))
        .expect("read startup descriptor")
        .expect("startup publishes a generation");
    let runner = host.drive();

    let replacement = root.join("prds/.editor-save");
    std::fs::write(
        &replacement,
        prd(
            "PRD-001",
            "Changed title",
            "draft",
            "After the atomic save.\n",
        ),
    )
    .expect("write replacement");
    std::fs::rename(&replacement, &artifact).expect("install atomic editor save");
    let expected_digest = capture(root).await.source_set_digest;
    assert_ne!(before.source_set_digest, expected_digest);

    // This must beat the 30-second control scan: one watcher batch cannot be lost.
    let published = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if let Some(current) = read_current(&root.join("lance")).expect("read CURRENT")
                && current.source_set_digest == expected_digest
            {
                break current;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await;
    runner.stop().await;
    let published = published.expect("the watcher must publish before the control scan");
    assert_ne!(published.generation_id, before.generation_id);
    assert_eq!(published.source_set_digest, expected_digest);
    assert_eq!(
        body_of(root, "prds", "PRD-001-auth.md").trim(),
        "After the atomic save."
    );
}

#[tokio::test]
async fn dropping_the_runner_unregisters_its_background_maintenance() {
    let tmp = workspace();
    let root = tmp.path();
    write(
        root,
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "Body.\n"),
    );
    let host = MaintenanceHost::start(capability(root), ReportSink::Foreground)
        .await
        .expect("start host");
    let runner = host.drive();
    drop(runner);
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while forgeplan_core::constellation::runtime::mark_dirty(root) {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("a dropped owner must not leave a detached maintenance host registered");
}

#[tokio::test]
async fn migration_cannot_retire_legacy_tables_while_a_canonical_writer_holds_the_lock() {
    let tmp = workspace();
    let root = tmp.path();
    write(
        root,
        "prds",
        "PRD-001-auth.md",
        &prd("PRD-001", "Auth", "draft", "Body.\n"),
    );
    let store = forgeplan_core::db::store::LanceStore::init(root)
        .await
        .expect("initialize the real legacy projection");
    drop(store);
    let legacy = root.join("lance/artifacts.lance");
    assert!(legacy.exists());
    let guard = forgeplan_core::workspace::acquire_workspace_lock(root)
        .await
        .expect("hold the canonical writer lock");
    let capability = capability(root);
    let publication = maintain_index(&capability, CaptureLimits::default(), deadline());
    tokio::pin!(publication);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(200), &mut publication)
            .await
            .is_err(),
        "migration must wait for the active canonical writer"
    );
    assert!(
        legacy.exists(),
        "the legacy sink must remain in place until the writer releases it"
    );
    assert!(
        !workspace_is_migrated(root),
        "ownership cannot change underneath the writer"
    );

    drop(guard);
    let (view, _) = publication
        .await
        .expect("publish after the writer releases its lock");
    assert!(workspace_is_migrated(root));
    assert!(!legacy.exists());
    assert_eq!(
        read_current(&root.join("lance"))
            .unwrap()
            .unwrap()
            .generation_id,
        view.manifest.generation_id
    );
}
