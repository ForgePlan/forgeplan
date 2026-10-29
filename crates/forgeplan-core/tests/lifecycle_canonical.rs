//! Canonical (Constellation) lifecycle transitions — behavioural coverage.
//!
//! Every test here drives the REAL publisher (`maintain_index`) on a real
//! temporary workspace, then drives the real file-first lifecycle operations
//! against the capture it published. Nothing is stubbed: the migration claim
//! under test is "these transitions work on a workspace whose derived index the
//! constellation protocol owns", and a fixture that hand-writes
//! `PROTOCOL.json` would test the branch's shape rather than the state it reads.
//!
//! # What is actually asserted
//!
//! Each test names the observable behaviour it pins and why the naive
//! implementation would fail it:
//!
//! * **Gates read real edges.** The evidence gate is satisfied by an
//!   `informs` edge in the canonical `links:` block and by nothing else. A
//!   migration that fed the gate an empty edge list would pass every
//!   "refuses without evidence" test while failing the "activates once
//!   evidence exists" one — so both halves are asserted.
//! * **Status transitions reach the bytes.** On this layout the Markdown IS
//!   the record; a status flip that is not in the file did not happen.
//! * **Terminal guards still refuse.** `supersede` onto a retired replacement
//!   and `reopen` on a terminal artifact read a *second* artifact's status,
//!   which is what makes them a real read rather than a self-consistency check.
//! * **Ordering is observable.** `reopen` must create and link the successor
//!   BEFORE retiring the old artifact (audit C3), so a mid-flow failure never
//!   leaves something retired with no successor.
//!
//! # Related tests
//!
//! Recursive `score <id>` is covered through the CLI in
//! `forgeplan-cli/tests/cli_migrated_write.rs` and through MCP in
//! `forgeplan-mcp/tests/constellation_migrated_read.rs`. Lifecycle transport
//! coverage lives in `forgeplan-cli/tests/cli_migrated_write.rs` and
//! `forgeplan-mcp/tests/constellation_migrated_write.rs`.

use std::path::{Path, PathBuf};

use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade::{MaintainCapability, maintain_index};
use forgeplan_core::constellation::workspace_read::WorkspaceRead;
use forgeplan_core::projection::MutationContext;
use forgeplan_core::workspace::lock::{WorkspaceLock, acquire_workspace_lock};
use tempfile::TempDir;

/// A body that clears every gate except the EVIDENCE one, so the evidence
/// gate is the only thing that can block in these tests.
///
/// "Every gate except evidence" means every MUST validation rule too, not just
/// the length and stub checks: a PRD at `standard` depth must also carry
/// Non-Goals, Related Artifacts and a target audience. The previous version of
/// this body omitted all three, so `activate_accepts_a_prd_…` failed on a
/// MUST error before it ever reached its assertion about the evidence edge,
/// and `activate_force_overrides_…` was forced through errors the test itself
/// claimed did not exist.
///
/// Written as prose rather than filler so a reader can see what it is for: any
/// template marker (`TODO`, `...`, an unfilled section) would trip
/// `check_stub` and make the evidence assertions meaningless.
fn gated_prd_body() -> String {
    format!(
        "## Summary\n\n{}\n\n## Problem\n\n{}\n\n## Goals\n\n{}\n\n\
         ## Non-Goals\n\n{}\n\n## Functional Requirements\n\n{}\n\n\
         ## Related Artifacts\n\n{}\n\n## Target Audience\n\n{}\n",
        "ForgePlan needs one lifecycle implementation that behaves identically whether the \
         derived index is owned by the legacy LanceDB projection or by the Constellation \
         protocol, so that an operator never has to know which one their workspace uses.",
        "Today the activation gates read the relation graph out of a table the Constellation \
         protocol retires, which forces either a loud refusal on every migrated workspace or, \
         worse, a resurrection of the retired dataset to satisfy a warning.",
        "The canonical Markdown already carries every edge the gates need in its links block, so \
         the gates can read the source of truth directly and the transition can be written \
         file-first without weakening a single rule.",
        "Rewriting the legacy LanceDB tables is out of scope here, and so is any change to the \
         gate rules themselves. This work moves where the edges are read from, not which \
         rules apply, and a gate that got quieter in the move would be a regression.",
        "FR-1 review and activate must produce the same verdict on both workspace layouts. \
         FR-2 the evidence gate must be satisfied by a canonical informs edge and by nothing \
         else. FR-3 no migrated lifecycle operation may open a retired Lance table.",
        "The canonical `links:` block is the only relationship record these operations consult, \
         and it names the evidence pack directly.",
        "Authors and reviewers working in a workspace that has been migrated to the Constellation \
         protocol, who need activation to behave the same as it did before the cutover.",
    )
}

/// A complete evidence pack: `is_evidence_complete` requires the structured
/// fields to be declared explicitly, not defaulted.
const COMPLETE_EVIDENCE_BODY: &str = "## Structured Fields\n\nverdict: supports\n\
     congruence_level: 3\nevidence_type: test\n\n\
     ## Verdict\n\nThe lifecycle transitions were exercised on a migrated workspace and each \
     transition wrote its canonical bytes.\n";

struct ArtifactFixture<'a> {
    id: &'a str,
    kind: &'a str,
    title: &'a str,
    status: &'a str,
    depth: &'a str,
    body: &'a str,
    links: &'a [(&'a str, &'a str)],
}

/// Write one artifact file into `ws`, rendering the same projection
/// frontmatter block `projection::render_markdown_with_extras` produces.
///
/// Hand-written rather than rendered through `create_artifact_with_projection`
/// so the fixture cannot depend on the code under test.
fn write_artifact(ws: &Path, dir: &str, fixture: ArtifactFixture<'_>) -> PathBuf {
    let ArtifactFixture {
        id,
        kind,
        title,
        status,
        depth,
        body,
        links,
    } = fixture;
    let mut fm =
        format!("---\nid: {id}\ntitle: {title}\nkind: {kind}\nstatus: {status}\ndepth: {depth}\n");
    if !links.is_empty() {
        fm.push_str("links:\n");
        for (target, relation) in links {
            fm.push_str(&format!("  - target: {target}\n    relation: {relation}\n"));
        }
    }
    let content = format!("{fm}---\n\n{body}\n");
    let path = ws.join(dir).join(format!("{id}-{}.md", slugify(title)));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    path
}

/// Filename slug for the fixture.
///
/// Delegates to the crate's own `slugify` rather than reimplementing it: the
/// exact filename shape is what the assertions below read back, and a local
/// approximation would diverge from the renderer the first time slug rules
/// change — turning a real regression into a "file not found".
fn slugify(title: &str) -> String {
    forgeplan_core::artifact::types::slugify(title)
}

/// Publish the workspace through the real maintenance pipeline, then capture a
/// read view over the published generation.
async fn published_read(ws: &Path) -> WorkspaceRead {
    let capability = MaintainCapability::for_strict_maintenance(ws.canonicalize().unwrap());
    maintain_index(
        &capability,
        CaptureLimits::default(),
        std::time::Instant::now() + std::time::Duration::from_secs(120),
    )
    .await
    .expect("first pass publishes");
    forgeplan_core::constellation::facade::open_workspace_read(ws, None, CaptureLimits::default())
        .await
        .expect("capture over the published workspace")
}

/// An empty published workspace — the starting point for a fixture that then
/// writes artifacts and re-publishes.
///
/// Publishes BEFORE any lock is taken, deliberately: `maintain_index` acquires
/// the workspace lock itself for the storage commit, so a fixture that held the
/// guard across a publish would deadlock against itself for the full 30 s
/// timeout. The mutation guard is taken later, around the mutation only.
async fn workspace() -> TempDir {
    let tmp = TempDir::new().unwrap();
    for dir in forgeplan_core::workspace::ARTIFACT_DIRS {
        std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
    }
    let capability = MaintainCapability::for_strict_maintenance(tmp.path().canonicalize().unwrap());
    maintain_index(
        &capability,
        CaptureLimits::default(),
        std::time::Instant::now() + std::time::Duration::from_secs(120),
    )
    .await
    .expect("empty workspace publishes");
    tmp
}

/// Acquire the mutation guard for one canonical write.
///
/// Taken here rather than held from the start of the test: `maintain_index`
/// (which [`published_read`] runs) acquires the same workspace lock for its
/// storage commit, so a guard held across a publish would deadlock against
/// itself.
async fn lock(ws: &Path) -> WorkspaceLock {
    acquire_workspace_lock(ws).await.expect("workspace lock")
}

// ── activate: transition + gates ────────────────────────────────────────────

/// A lightweight kind flips `draft → active` in the canonical frontmatter.
///
/// A NOTE is deliberately chosen: `supports_lifecycle("note") == false`, so the
/// methodology gates do not apply and this test isolates the migration
/// question — does the status flip reach the bytes? — from the gate question,
/// which the next two tests cover.
#[tokio::test]
async fn activate_writes_the_status_into_canonical_frontmatter() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-001",
            kind: "note",
            title: "First note",
            status: "draft",
            depth: "tactical",
            body: "Body.",
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let result =
        forgeplan_core::lifecycle::canonical::activate_captured(&ctx, &read, "NOTE-001", false)
            .await
            .unwrap();
    assert_eq!(result.artifact_id, "NOTE-001");
    assert!(!result.forced);

    let bytes = std::fs::read_to_string(ws.join("notes/NOTE-001-first-note.md")).unwrap();
    assert!(
        bytes.contains("status: active"),
        "the transition must reach the canonical frontmatter:\n{bytes}"
    );
}

/// The evidence gate blocks a PRD that has no linked evidence — and the
/// refusal carries the canonical fix order, not a generic "failed".
#[tokio::test]
async fn activate_refuses_a_prd_with_no_linked_evidence() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "prds",
        ArtifactFixture {
            id: "PRD-001",
            kind: "prd",
            title: "Gated product",
            status: "draft",
            depth: "standard",
            body: &gated_prd_body(),
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let err =
        forgeplan_core::lifecycle::canonical::activate_captured(&ctx, &read, "PRD-001", false)
            .await
            .expect_err("no evidence linked must block activation");
    let msg = err.to_string();
    assert!(
        msg.contains("No evidence linked"),
        "the refusal must name the failing gate, got: {msg}"
    );
    assert!(
        msg.contains("forgeplan_new"),
        "the refusal must carry the canonical fix order, got: {msg}"
    );

    let bytes = std::fs::read_to_string(ws.join("prds/PRD-001-gated-product.md")).unwrap();
    assert!(
        bytes.contains("status: draft"),
        "a refused activation must not have flipped the status:\n{bytes}"
    );
}

/// The evidence gate PASSES once an `informs` edge exists in the canonical
/// `links:` block.
///
/// This is the half that makes the refusal above meaningful. A migration that
/// fed the gate an empty edge list would satisfy the first test and fail this
/// one — which is exactly the failure mode "the gate silently stopped
/// checking anything" produces.
#[tokio::test]
async fn activate_accepts_a_prd_whose_evidence_edge_is_canonical() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "prds",
        ArtifactFixture {
            id: "PRD-001",
            kind: "prd",
            title: "Gated product",
            status: "draft",
            depth: "standard",
            body: &gated_prd_body(),
            links: &[("EVID-001", "informs")],
        },
    );
    write_artifact(
        ws,
        "evidence",
        ArtifactFixture {
            id: "EVID-001",
            kind: "evidence",
            title: "Verification verdict",
            status: "active",
            depth: "tactical",
            body: COMPLETE_EVIDENCE_BODY,
            links: &[("PRD-001", "informs")],
        },
    );

    let read = published_read(ws).await;
    // Assert the capture really did observe the edge before trusting the
    // activation: this is the property the gate depends on.
    assert!(
        read.get_relations("PRD-001")
            .iter()
            .any(|(t, r)| t == "EVID-001" && r == "informs"),
        "fixture precondition: the canonical informs edge must be captured"
    );

    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    forgeplan_core::lifecycle::canonical::activate_captured(&ctx, &read, "PRD-001", false)
        .await
        .expect("a linked evidence pack satisfies the gate");

    let bytes = std::fs::read_to_string(ws.join("prds/PRD-001-gated-product.md")).unwrap();
    assert!(
        bytes.contains("status: active"),
        "activation must reach the canonical frontmatter:\n{bytes}"
    );
}

/// `--force` is the documented escape hatch and must still work on this layout:
/// otherwise "the gate refused" has no remedy an operator can reach.
#[tokio::test]
async fn activate_force_overrides_the_evidence_gate() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "prds",
        ArtifactFixture {
            id: "PRD-001",
            kind: "prd",
            title: "Gated product",
            status: "draft",
            depth: "standard",
            body: &gated_prd_body(),
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let result =
        forgeplan_core::lifecycle::canonical::activate_captured(&ctx, &read, "PRD-001", true)
            .await
            .expect("--force bypasses the gates");
    assert!(!result.forced, "no MUST errors were forced through");

    let bytes = std::fs::read_to_string(ws.join("prds/PRD-001-gated-product.md")).unwrap();
    assert!(
        bytes.contains("status: active"),
        "force must still write:\n{bytes}"
    );
}

/// A HYP is redirected to `forgeplan_hypothesis_promote` rather than having its
/// `status` desynchronised from `hypothesis_state`.
#[tokio::test]
async fn activate_redirects_a_hypothesis_instead_of_desynchronising_its_state() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "hypotheses",
        ArtifactFixture {
            id: "HYP-001",
            kind: "hypothesis",
            title: "A hypothesis",
            status: "draft",
            depth: "standard",
            body: "Some hypothesis body.",
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let err =
        forgeplan_core::lifecycle::canonical::activate_captured(&ctx, &read, "HYP-001", false)
            .await
            .expect_err("a HYP must not be activated directly");
    assert!(
        err.to_string().contains("hypothesis_promote"),
        "the refusal must name the canonical path, got: {err}"
    );

    let bytes = std::fs::read_to_string(ws.join("hypotheses/HYP-001-a-hypothesis.md")).unwrap();
    assert!(
        bytes.contains("status: draft"),
        "the refused activation must not have written:\n{bytes}"
    );
}

/// `draft → active` is the only inbound transition. `deprecated → active` must
/// refuse, because "deprecated is terminal" is the rule an operator relies on
/// to know a retired artifact cannot silently come back.
#[tokio::test]
async fn activate_refuses_to_revive_a_terminal_artifact() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-001",
            kind: "note",
            title: "Retired note",
            status: "deprecated",
            depth: "tactical",
            body: "Body.",
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let err =
        forgeplan_core::lifecycle::canonical::activate_captured(&ctx, &read, "NOTE-001", false)
            .await
            .expect_err("deprecated is terminal");
    assert!(err.to_string().contains("terminal"), "got: {err}");

    let bytes = std::fs::read_to_string(ws.join("notes/NOTE-001-retired-note.md")).unwrap();
    assert!(
        bytes.contains("status: deprecated"),
        "the refused activation must not have written:\n{bytes}"
    );
}

// ── review ─────────────────────────────────────────────────────────────────

/// `review` reports the evidence gate as blocking, and stops reporting it once
/// the pack is linked — the read-side twin of the activate pair.
#[tokio::test]
async fn review_reads_the_evidence_gate_from_captured_edges() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "prds",
        ArtifactFixture {
            id: "PRD-001",
            kind: "prd",
            title: "Gated product",
            status: "draft",
            depth: "standard",
            body: &gated_prd_body(),
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let blocked = forgeplan_core::lifecycle::canonical::review_captured(&read, "PRD-001").unwrap();
    assert!(
        !blocked.can_activate,
        "no evidence linked means the review cannot pass"
    );
    assert!(
        blocked
            .warnings
            .iter()
            .any(|w| w.contains("No evidence linked")),
        "the gate must be reported, got: {:?}",
        blocked.warnings
    );

    // Re-publish with the evidence edge in place.
    write_artifact(
        ws,
        "prds",
        ArtifactFixture {
            id: "PRD-001",
            kind: "prd",
            title: "Gated product",
            status: "draft",
            depth: "standard",
            body: &gated_prd_body(),
            links: &[("EVID-001", "informs")],
        },
    );
    write_artifact(
        ws,
        "evidence",
        ArtifactFixture {
            id: "EVID-001",
            kind: "evidence",
            title: "Verification verdict",
            status: "active",
            depth: "tactical",
            body: COMPLETE_EVIDENCE_BODY,
            links: &[("PRD-001", "informs")],
        },
    );
    let read = published_read(ws).await;
    let passed = forgeplan_core::lifecycle::canonical::review_captured(&read, "PRD-001").unwrap();
    assert!(
        !passed
            .warnings
            .iter()
            .any(|w| w.contains("No evidence linked")),
        "the gate must clear once the pack is linked, got: {:?}",
        passed.warnings
    );
}

/// `review` flags a build-on-draft dependency — a warning that requires
/// resolving a SECOND artifact from the same capture. If the migrated path
/// answered it by re-opening a store, this would be the test that noticed.
#[tokio::test]
async fn review_flags_build_on_draft_from_the_same_capture() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "prds",
        ArtifactFixture {
            id: "PRD-001",
            kind: "prd",
            title: "Depending product",
            status: "draft",
            depth: "standard",
            body: &gated_prd_body(),
            links: &[("RFC-001", "based_on")],
        },
    );
    write_artifact(
        ws,
        "rfcs",
        ArtifactFixture {
            id: "RFC-001",
            kind: "rfc",
            title: "Draft dependency",
            status: "draft",
            depth: "standard",
            body: "An RFC that is still a draft.",
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let result = forgeplan_core::lifecycle::canonical::review_captured(&read, "PRD-001").unwrap();
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.contains("build-on-draft") && w.contains("RFC-001")),
        "the dependency's draft status must be reported, got: {:?}",
        result.warnings
    );
}

// ── supersede ──────────────────────────────────────────────────────────────

/// `supersede` writes the successor edge into the source file's `links:` block
/// and flips the displaced artifact's status, and lists the dependents it
/// found — all from one capture.
#[tokio::test]
async fn supersede_writes_edge_and_status_and_reports_dependents() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-001",
            kind: "note",
            title: "Old note",
            status: "active",
            depth: "tactical",
            body: "Body.",
            links: &[],
        },
    );
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-002",
            kind: "note",
            title: "New note",
            status: "active",
            depth: "tactical",
            body: "Body.",
            links: &[],
        },
    );
    write_artifact(
        ws,
        "prds",
        ArtifactFixture {
            id: "PRD-001",
            kind: "prd",
            title: "Depending product",
            status: "draft",
            depth: "standard",
            body: &gated_prd_body(),
            links: &[("NOTE-001", "based_on")],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let result = forgeplan_core::lifecycle::canonical::supersede_captured(
        &ctx, &read, "NOTE-001", "NOTE-002",
    )
    .await
    .unwrap();
    assert_eq!(
        result.dependents,
        vec!["PRD-001".to_string()],
        "the incoming edge must be reported as a dependent"
    );
    assert!(
        result.warnings.is_empty(),
        "an active replacement raises no warning, got: {:?}",
        result.warnings
    );

    let bytes = std::fs::read_to_string(ws.join("notes/NOTE-001-old-note.md")).unwrap();
    assert!(
        bytes.contains("status: superseded"),
        "the displaced artifact must be marked superseded:\n{bytes}"
    );
    assert!(
        bytes.contains("target: NOTE-002") && bytes.contains("relation: supersedes"),
        "the successor edge must land in the canonical links block:\n{bytes}"
    );
}

/// A replacement that is itself retired is refused — the chain-risk guard.
/// It reads a DIFFERENT artifact's status, so it fails if the transition ever
/// starts reading the source artifact twice instead of resolving the target.
#[tokio::test]
async fn supersede_refuses_a_retired_replacement() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-001",
            kind: "note",
            title: "Middle note",
            status: "active",
            depth: "tactical",
            body: "Body.",
            links: &[],
        },
    );
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-002",
            kind: "note",
            title: "Retired note",
            status: "deprecated",
            depth: "tactical",
            body: "Body.",
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let err = forgeplan_core::lifecycle::canonical::supersede_captured(
        &ctx, &read, "NOTE-001", "NOTE-002",
    )
    .await
    .expect_err("a retired replacement is refused");
    assert!(err.to_string().contains("already deprecated"), "got: {err}");

    let bytes = std::fs::read_to_string(ws.join("notes/NOTE-001-middle-note.md")).unwrap();
    assert!(
        !bytes.contains("supersedes"),
        "a refused supersede must not have written the edge:\n{bytes}"
    );
    assert!(
        bytes.contains("status: active"),
        "a refused supersede must leave the source active:\n{bytes}"
    );
}

/// A draft replacement is a WARNING, not a refusal — the distinction the
/// store path draws, and one a migration could easily collapse into a refusal.
#[tokio::test]
async fn supersede_warns_but_proceeds_for_a_draft_replacement() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-001",
            kind: "note",
            title: "Old note",
            status: "active",
            depth: "tactical",
            body: "Body.",
            links: &[],
        },
    );
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-002",
            kind: "note",
            title: "Draft successor",
            status: "draft",
            depth: "tactical",
            body: "Body.",
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let result = forgeplan_core::lifecycle::canonical::supersede_captured(
        &ctx, &read, "NOTE-001", "NOTE-002",
    )
    .await
    .expect("a draft replacement is a warning, not a refusal");
    assert!(
        result.warnings.iter().any(|w| w.contains("still draft")),
        "the draft warning must be surfaced, got: {:?}",
        result.warnings
    );
}

// ── deprecate ──────────────────────────────────────────────────────────────

/// `deprecate` writes BOTH the status and the reason section.
///
/// #478 is why the body is asserted: on the legacy path the plain renderer is
/// files-first, so a status-only write left the reason nowhere — `lance/` is
/// gitignored and the record of why an artifact was retired vanished.
#[tokio::test]
async fn deprecate_writes_status_and_reason_into_the_canonical_body() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-001",
            kind: "note",
            title: "Retirable note",
            status: "active",
            depth: "tactical",
            body: "Original body text.",
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let dependents = forgeplan_core::lifecycle::canonical::deprecate_captured(
        &ctx,
        &read,
        "NOTE-001",
        "superseded by hand",
    )
    .await
    .unwrap();
    assert!(dependents.is_empty());

    let bytes = std::fs::read_to_string(ws.join("notes/NOTE-001-retirable-note.md")).unwrap();

    let (fm, body) = forgeplan_core::artifact::frontmatter::parse_frontmatter(&bytes).unwrap();
    assert_eq!(
        fm.get("id").and_then(|value| value.as_str()),
        Some("NOTE-001")
    );
    assert!(
        !body.starts_with("---") && !body.contains("id: NOTE-001"),
        "the projection frontmatter must not be appended into the artifact body:\n{body}"
    );
    assert!(
        bytes.contains("status: deprecated"),
        "the status must reach the canonical frontmatter:\n{bytes}"
    );
    assert!(
        bytes.contains("## Deprecation") && bytes.contains("superseded by hand"),
        "the reason must reach the canonical body (#478):\n{bytes}"
    );
    assert!(
        bytes.contains("Original body text."),
        "the original body must survive the append:\n{bytes}"
    );
}

/// `deprecate` refuses an artifact that is already terminal — the FSM guard,
/// read from the artifact's own captured status.
#[tokio::test]
async fn deprecate_refuses_an_already_terminal_artifact() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-001",
            kind: "note",
            title: "Finished note",
            status: "deprecated",
            depth: "tactical",
            body: "Body.",
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let err =
        forgeplan_core::lifecycle::canonical::deprecate_captured(&ctx, &read, "NOTE-001", "again")
            .await
            .expect_err("deprecated cannot be deprecated again");
    assert!(err.to_string().contains("terminal"), "got: {err}");
}

// ── renew ──────────────────────────────────────────────────────────────────

/// `renew` writes all three things ADR-005 requires: the `## Renewal` section,
/// the status, and the new `valid_until`.
///
/// The `valid_until` writer is the reason this test exists as its own case: it
/// did not exist before this cutover, and without it `renew` had no
/// file-first form at all.
#[tokio::test]
async fn renew_writes_section_status_and_valid_until() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-001",
            kind: "note",
            title: "Renewable note",
            // ADR-005: renew is `stale → active`. An `active` artifact has no
            // transition to make, and both layouts refuse it — see
            // `lifecycle::tests::renew_active_fails`. The fixture was `active`,
            // which made this test exercise a refusal while claiming to exercise
            // the three writes renew is required to make.
            status: "stale",
            depth: "tactical",
            body: "Original body text.",
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let result = forgeplan_core::lifecycle::canonical::renew_captured(
        &ctx,
        &read,
        "NOTE-001",
        "still current",
        "2099-12-31",
    )
    .await
    .unwrap();
    assert_eq!(result.old_valid_until, None);
    assert_eq!(result.new_valid_until, "2099-12-31");

    let bytes = std::fs::read_to_string(ws.join("notes/NOTE-001-renewable-note.md")).unwrap();

    let (fm, body) = forgeplan_core::artifact::frontmatter::parse_frontmatter(&bytes).unwrap();
    assert_eq!(
        fm.get("id").and_then(|value| value.as_str()),
        Some("NOTE-001")
    );
    assert!(
        !body.starts_with("---") && !body.contains("id: NOTE-001"),
        "the projection frontmatter must not be appended into the artifact body:\n{body}"
    );
    assert!(
        bytes.contains("## Renewal") && bytes.contains("still current"),
        "the renewal section and reason must reach the canonical body:\n{bytes}"
    );
    assert!(
        bytes.contains("Extended until: 2099-12-31"),
        "the extended date must be recorded in the body:\n{bytes}"
    );
    assert!(
        bytes.contains("valid_until: 2099-12-31"),
        "valid_until must reach the canonical frontmatter:\n{bytes}"
    );
    assert!(
        bytes.contains("status: active"),
        "renew leaves the artifact active:\n{bytes}"
    );
    assert!(
        bytes.contains("Original body text."),
        "the original body must survive the append:\n{bytes}"
    );
}

/// A malformed date is refused before anything is written.
///
/// The audit-C1 "validate early" rule only means something if the refusal
/// leaves the file untouched — hence asserting the absence of the section
/// rather than just the error text.
#[tokio::test]
async fn renew_refuses_a_malformed_date_without_writing() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-001",
            kind: "note",
            title: "Renewable note",
            status: "active",
            depth: "tactical",
            body: "Original body text.",
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let err = forgeplan_core::lifecycle::canonical::renew_captured(
        &ctx,
        &read,
        "NOTE-001",
        "nope",
        "31-12-2099",
    )
    .await
    .expect_err("a malformed date is refused");
    assert!(
        err.to_string().contains("Invalid date format"),
        "got: {err}"
    );

    let bytes = std::fs::read_to_string(ws.join("notes/NOTE-001-renewable-note.md")).unwrap();
    assert!(
        !bytes.contains("## Renewal"),
        "a rejected date must not have appended anything:\n{bytes}"
    );
    assert!(
        !bytes.contains("valid_until:"),
        "a rejected date must not have written valid_until:\n{bytes}"
    );
}

// ── reopen ─────────────────────────────────────────────────────────────────

/// `reopen` creates the successor, links it `based_on` the old artifact, and
/// only then deprecates the old one — the audit-C3 ordering, observed
/// end-to-end on disk.
#[tokio::test]
async fn reopen_creates_the_successor_then_links_then_deprecates() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-001",
            kind: "note",
            title: "Reopenable note",
            status: "active",
            depth: "tactical",
            body: "Original body text.",
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let result = forgeplan_core::lifecycle::canonical::reopen_captured(
        &ctx,
        &read,
        "NOTE-001",
        "needs revision",
        "NOTE-002",
    )
    .await
    .unwrap();
    assert_eq!(result.old_id, "NOTE-001");
    assert_eq!(result.new_id, "NOTE-002");
    assert_eq!(result.new_kind, "note");

    // The successor exists, is a draft, and carries its lineage.
    let successor =
        std::fs::read_to_string(ws.join("notes/NOTE-002-reopenable-note-reopened.md")).unwrap();
    assert!(
        successor.contains("status: draft"),
        "the successor must be a draft:\n{successor}"
    );
    assert!(
        successor.contains("## Lineage") && successor.contains("Reopened from: NOTE-001"),
        "the successor must record where it came from:\n{successor}"
    );
    assert!(
        successor.contains("needs revision"),
        "the successor must record why:\n{successor}"
    );
    assert!(
        successor.contains("target: NOTE-001") && successor.contains("relation: based_on"),
        "the successor must be linked back to the old artifact:\n{successor}"
    );

    // The old artifact is retired LAST and says why.
    let old = std::fs::read_to_string(ws.join("notes/NOTE-001-reopenable-note.md")).unwrap();

    let (fm, body) = forgeplan_core::artifact::frontmatter::parse_frontmatter(&old).unwrap();
    assert_eq!(
        fm.get("id").and_then(|value| value.as_str()),
        Some("NOTE-001")
    );
    assert!(
        !body.starts_with("---") && !body.contains("id: NOTE-001"),
        "the projection frontmatter must not be appended into the artifact body:\n{body}"
    );
    assert!(
        old.contains("status: deprecated"),
        "the old artifact must be deprecated:\n{old}"
    );
    assert!(
        old.contains("## Reopened") && old.contains("Replacement: NOTE-002"),
        "the old artifact must name its successor (#478):\n{old}"
    );
    assert!(
        old.contains("Original body text."),
        "the old artifact's body must survive the append:\n{old}"
    );
}

/// A terminal artifact cannot be reopened — ADR-005's "create fresh instead".
///
/// The refusal must also leave NOTHING behind: a successor created before the
/// refusal would be exactly the orphan the ordering rule exists to prevent.
#[tokio::test]
async fn reopen_refuses_a_terminal_artifact_and_creates_nothing() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-001",
            kind: "note",
            title: "Finished note",
            status: "deprecated",
            depth: "tactical",
            body: "Body.",
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let err = forgeplan_core::lifecycle::canonical::reopen_captured(
        &ctx,
        &read,
        "NOTE-001",
        "second thoughts",
        "NOTE-002",
    )
    .await
    .expect_err("a terminal artifact cannot be reopened");
    assert!(
        err.to_string().contains("terminal"),
        "the refusal must say the state is terminal, got: {err}"
    );
    // The successor's title is derived from the SOURCE artifact's title, so
    // the orphan this guards against would be `NOTE-002-finished-note-reopened.md`.
    assert!(
        !ws.join("notes/NOTE-002-finished-note-reopened.md").exists(),
        "a refused reopen must not have created a successor"
    );
}

/// An `new_id` that is already taken is refused before anything is written —
/// the audit-C3 race guard.
#[tokio::test]
async fn reopen_refuses_a_taken_new_id_without_writing() {
    let tmp = workspace().await;
    let ws = tmp.path();
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-001",
            kind: "note",
            title: "Reopenable note",
            status: "active",
            depth: "tactical",
            body: "Body.",
            links: &[],
        },
    );
    write_artifact(
        ws,
        "notes",
        ArtifactFixture {
            id: "NOTE-002",
            kind: "note",
            title: "Already here",
            status: "active",
            depth: "tactical",
            body: "Body.",
            links: &[],
        },
    );

    let read = published_read(ws).await;
    let lock = lock(ws).await;
    let ctx = MutationContext::new(ws, &lock, None);
    let err = forgeplan_core::lifecycle::canonical::reopen_captured(
        &ctx, &read, "NOTE-001", "reason", "NOTE-002",
    )
    .await
    .expect_err("a taken id is refused");
    assert!(err.to_string().contains("already exists"), "got: {err}");

    let bytes = std::fs::read_to_string(ws.join("notes/NOTE-001-reopenable-note.md")).unwrap();
    assert!(
        bytes.contains("status: active"),
        "a refused reopen must leave the source untouched:\n{bytes}"
    );
    assert!(
        !bytes.contains("## Reopened"),
        "a refused reopen must not have appended anything:\n{bytes}"
    );
}
