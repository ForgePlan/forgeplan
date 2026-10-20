//! The full-MCP write path on a workspace the Constellation protocol owns.
//!
//! The read side of this cutover is covered by `constellation_migrated_read.rs`.
//! This file covers the write side, which has one property that reads do not:
//! a migrated write must commit **somewhere**. So each test asserts both
//! halves, and neither alone is enough:
//!
//! 1. **The canonical Markdown changed.** That is the whole mutation on a
//!    migrated workspace, so a handler that quietly no-ops fails here.
//! 2. **No legacy table came into existence.** `LanceStore::open` runs schema
//!    migrations and `init` creates `artifacts.lance` / `relations.lance` /
//!    `change_log.lance`. Resolution therefore must not open a store at all;
//!    the check is on disk, so a handler that wrote correctly *and* migrated a
//!    retired table still fails.
//!
//! Plus the two contracts that surround the cutover:
//!
//! 3. **Explicit workspace precedence on writes.** `--workspace Y` reaches the
//!    MCP server as its default root; a call with no `workspace` parameter must
//!    therefore land in Y, while an explicit `workspace=Z` must land in Z.
//!    Both are proven by the bytes each `.forgeplan/` ends up holding.
//! 4. **Post-unlock reconciliation.** A migrated write releases the workspace
//!    lock before it touches the index, and reports the outcome honestly in
//!    `index_pending`. A `false` here must mean the index really was published,
//!    not that the signal was skipped.

use std::path::{Path, PathBuf};
use std::time::Duration;

use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade::MaintainCapability;
use serde_json::json;
use tempfile::TempDir;

mod common;
use common::McpFixture;

/// The legacy tables a migrated write must never bring into existence.
///
/// Named exactly as `LanceStore::init` names them.
fn legacy_tables(ws: &Path) -> Vec<String> {
    let lance = ws.join("lance");
    ["artifacts.lance", "relations.lance", "change_log.lance"]
        .into_iter()
        .filter(|name| lance.join(name).exists())
        .map(str::to_string)
        .collect()
}

fn write_artifact(ws: &Path, dir: &str, name: &str, contents: &str) {
    let path = ws.join(dir).join(name);
    std::fs::create_dir_all(path.parent().expect("artifact dir")).expect("create dir");
    std::fs::write(path, contents).expect("write artifact");
}

/// A workspace published through `maintain_index` — genuinely migrated.
///
/// `marker` appears in one artifact's body and nowhere else, so a write that
/// landed in the wrong workspace is visible in the bytes rather than in a path
/// assertion.
async fn publish_workspace(root: &Path, marker: &str) -> PathBuf {
    let ws =
        forgeplan_core::workspace::init_workspace(root, "migrated-write").expect("init workspace");
    write_artifact(
        &ws,
        "prds",
        "PRD-001-auth.md",
        &format!(
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth rewrite\ndepth: standard\n---\n\n## Summary\n\n{marker}\n"
        ),
    );

    let capability = MaintainCapability::for_strict_maintenance(
        ws.canonicalize()
            .expect("workspace directory canonicalizes"),
    );
    forgeplan_core::constellation::facade::maintain_index(
        &capability,
        CaptureLimits::default(),
        std::time::Instant::now() + Duration::from_secs(120),
    )
    .await
    .expect("bootstrap the Constellation index");

    assert!(
        forgeplan_core::constellation::facade::workspace_is_migrated(&ws),
        "fixture precondition: a published workspace must report as migrated, \
         otherwise this test would silently exercise the legacy branch"
    );
    assert!(
        legacy_tables(&ws).is_empty(),
        "fixture precondition: a published workspace holds no legacy tables, found {:?}",
        legacy_tables(&ws)
    );
    ws
}

/// Republish a workspace that already exists, so artifacts written after the
/// first `publish_workspace` call are in the captured generation the handlers
/// resolve against.
///
/// A separate entry point because `publish_workspace` also seeds PRD-001 and
/// asserts the fixture preconditions; re-running it would rewrite that artifact
/// and hide whatever the test under it actually wrote.
async fn publish_only(ws: &Path) {
    let capability = MaintainCapability::for_strict_maintenance(
        ws.canonicalize()
            .expect("workspace directory canonicalizes"),
    );
    forgeplan_core::constellation::facade::maintain_index(
        &capability,
        CaptureLimits::default(),
        std::time::Instant::now() + Duration::from_secs(120),
    )
    .await
    .expect("republish the Constellation index");
}

/// Every markdown file under `.forgeplan/`, concatenated.
///
/// Deliberately coarse: the question each test asks is "did these bytes land
/// HERE", and a substring scan over the whole workspace answers it without
/// having to predict the renderer's exact filename for a given title.
fn workspace_bytes(ws: &Path) -> String {
    let mut out = String::new();
    let mut stack = vec![ws.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(text) = std::fs::read_to_string(&path) {
                out.push_str(&text);
                out.push('\n');
            }
        }
    }
    out
}

async fn server_at(root: &Path) -> McpFixture {
    McpFixture::new_rooted(root.to_path_buf()).await
}

#[tokio::test]
async fn migrated_update_writes_canonical_markdown_and_creates_no_legacy_tables() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = publish_workspace(tmp.path(), "auth-rewrite-marker").await;

    let server = server_at(tmp.path()).await;
    let updated = server
        .call_tool_json(
            "forgeplan_update",
            json!({
                "id": "PRD-001",
                "title": "Auth rewrite v2",
                "body": "## Summary\n\nRewritten through the migrated write path.\n",
            }),
        )
        .await;
    let payload = updated.assert_ok();

    assert_eq!(
        payload["status"], "draft",
        "the update must report the artifact's post-write status, got {}",
        updated.raw_text
    );

    let bytes = workspace_bytes(&ws);
    assert!(
        bytes.contains("Auth rewrite v2"),
        "the canonical markdown must carry the new title, got {bytes}"
    );
    assert!(
        bytes.contains("Rewritten through the migrated write path."),
        "the canonical markdown must carry the new body, got {bytes}"
    );

    assert!(
        legacy_tables(&ws).is_empty(),
        "a migrated write must not create legacy tables, found {:?}",
        legacy_tables(&ws)
    );
}

#[tokio::test]
async fn migrated_link_writes_the_edge_into_canonical_markdown_only() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = publish_workspace(tmp.path(), "auth-rewrite-marker").await;
    write_artifact(
        &ws,
        "notes",
        "NOTE-001-constraint.md",
        "---\nid: NOTE-001\nkind: note\nstatus: draft\ntitle: Constraint\ndepth: tactical\n---\n\n## Body\n\nA constraint.\n",
    );

    let server = server_at(tmp.path()).await;
    let linked = server
        .call_tool_json(
            "forgeplan_link",
            json!({
                "source": "PRD-001",
                "target": "NOTE-001",
                "relation": "based_on",
            }),
        )
        .await;
    let payload = linked.assert_ok();

    assert!(
        payload["message"]
            .as_str()
            .unwrap_or_default()
            .contains("PRD-001"),
        "the link response must name the source, got {}",
        linked.raw_text
    );

    let bytes = workspace_bytes(&ws);
    assert!(
        bytes.contains("NOTE-001"),
        "the edge must be recorded in the canonical source, got {bytes}"
    );
    assert!(
        bytes.contains("based_on"),
        "the recorded edge must carry its relation, got {bytes}"
    );

    assert!(
        legacy_tables(&ws).is_empty(),
        "a migrated link must not create legacy tables, found {:?}",
        legacy_tables(&ws)
    );
}

#[tokio::test]
async fn migrated_delete_moves_the_artifact_out_of_the_canonical_source() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = publish_workspace(tmp.path(), "auth-rewrite-marker").await;

    let server = server_at(tmp.path()).await;
    let deleted = server
        .call_tool_json("forgeplan_delete", json!({ "id": "PRD-001" }))
        .await;
    let payload = deleted.assert_ok();

    let receipt = payload["receipt_id"]
        .as_str()
        .expect("the delete must report the receipt it wrote");
    assert!(
        ws.join("trash")
            .join(format!("receipt-{receipt}.json"))
            .exists(),
        "the soft-delete receipt must be on disk before the artifact is gone"
    );

    // Scoped to the artifact directory on purpose: the receipt under
    // `.forgeplan/trash/` legitimately still carries the body — that is what
    // makes the delete reversible — so a whole-workspace scan would find the
    // marker there and pass for the wrong reason.
    let prds = workspace_bytes(&ws.join("prds"));
    assert!(
        !prds.contains("auth-rewrite-marker"),
        "the deleted artifact must no longer be readable as canonical content, \
         got {prds}"
    );
    assert!(
        ws.join("trash").exists(),
        "the soft-delete receipt must have moved the projection into .forgeplan/trash/"
    );

    assert!(
        legacy_tables(&ws).is_empty(),
        "a migrated delete must not create legacy tables, found {:?}",
        legacy_tables(&ws)
    );
}

#[tokio::test]
async fn a_migrated_write_reports_the_index_it_published_into() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = publish_workspace(tmp.path(), "auth-rewrite-marker").await;

    let index_root = ws.join("lance");
    let generation_before = forgeplan_core::constellation::storage::read_current(&index_root)
        .expect("read CURRENT before the write")
        .map(|d| d.generation_id);

    let server = server_at(tmp.path()).await;
    let written = server
        .call_tool_json(
            "forgeplan_update",
            json!({ "id": "PRD-001", "title": "Auth rewrite reconciled" }),
        )
        .await;
    let payload = written.assert_ok();

    assert_eq!(
        payload["index_pending"],
        json!(false),
        "the write completed, so the index must not be reported pending, got {}",
        written.raw_text
    );
    assert!(
        payload.get("index_pending_reason").is_none(),
        "a published write must not carry a pending reason, got {}",
        written.raw_text
    );

    let generation_after = forgeplan_core::constellation::storage::read_current(&index_root)
        .expect("read CURRENT after the write")
        .map(|d| d.generation_id);

    assert_ne!(
        generation_before, generation_after,
        "`index_pending: false` must mean the pass really published: CURRENT \
         still names {:?}",
        generation_before
    );

    let captured = forgeplan_core::constellation::facade::open_workspace_read(
        &ws,
        None,
        CaptureLimits::default(),
    )
    .await
    .expect("re-capture the workspace");
    let record = captured
        .get_record("PRD-001")
        .expect("the artifact survives the write");
    assert_eq!(
        record.title, "Auth rewrite reconciled",
        "the published index must describe the committed bytes"
    );
}

#[tokio::test]
async fn an_explicit_workspace_param_writes_that_workspace_not_the_default() {
    let default_tmp = TempDir::new().expect("tempdir for default");
    let target_tmp = TempDir::new().expect("tempdir for target");

    // X is the server's own default; Y is what the call names. Distinct
    // titles, so a write that landed in X instead of Y is visible.
    let _x = publish_workspace(default_tmp.path(), "default-workspace-only-marker").await;
    let y = publish_workspace(target_tmp.path(), "target-workspace-only-marker").await;

    let server = server_at(default_tmp.path()).await;
    server
        .call_tool_json(
            "forgeplan_update",
            json!({
                "id": "PRD-001",
                "title": "Scoped to Y",
                "workspace": y.to_string_lossy(),
            }),
        )
        .await
        .assert_ok();

    assert!(
        workspace_bytes(&y).contains("Scoped to Y"),
        "the write must land in the workspace the call named"
    );
    assert!(
        !workspace_bytes(&default_tmp.path().join(".forgeplan")).contains("Scoped to Y"),
        "the write must not touch the server's default workspace X"
    );

    assert!(
        legacy_tables(&y).is_empty(),
        "writing Y through an explicit param must not create legacy tables in Y"
    );
    assert!(
        legacy_tables(&default_tmp.path().join(".forgeplan")).is_empty(),
        "writing Y must not touch X's tables either"
    );
}

// ---------------------------------------------------------------------------
// Lifecycle transitions through the full MCP transport
// ---------------------------------------------------------------------------

/// `forgeplan_activate` flips the status in canonical Markdown on a migrated
/// workspace and creates no legacy table.
///
/// A NOTE is used so the methodology gates do not apply and the assertion is
/// about the migration, not the gates. The gate itself is covered separately by
/// `forgeplan-cli/tests/cli_migrated_write.rs` (which drives the same
/// `lifecycle::canonical` code through the binary) and by
/// `forgeplan-core/tests/lifecycle_canonical.rs`.
#[tokio::test]
async fn migrated_activate_flips_status_in_canonical_markdown() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = publish_workspace(tmp.path(), "auth-rewrite-marker").await;
    write_artifact(
        &ws,
        "notes",
        "NOTE-001-constraint.md",
        "---\nid: NOTE-001\nkind: note\nstatus: draft\ntitle: Constraint\ndepth: tactical\n---\n\n## Body\n\nA constraint.\n",
    );
    // Republish so the new artifact is in the captured generation the handler
    // resolves against.
    publish_only(&ws).await;

    let server = server_at(tmp.path()).await;
    let activated = server
        .call_tool_json("forgeplan_activate", json!({ "id": "NOTE-001" }))
        .await;
    let payload = activated.assert_ok();

    assert_eq!(
        payload["artifact_id"], "NOTE-001",
        "the response must name the canonical id, got {}",
        activated.raw_text
    );
    assert_eq!(
        payload["forced"],
        json!(false),
        "no validation error was forced through, got {}",
        activated.raw_text
    );

    let bytes = workspace_bytes(&ws);
    assert!(
        bytes.contains("status: active"),
        "the transition must reach the canonical frontmatter, got {bytes}"
    );
    assert!(
        legacy_tables(&ws).is_empty(),
        "activate must not create legacy tables, found {:?}",
        legacy_tables(&ws)
    );
}

/// `forgeplan_deprecate` writes BOTH the status and the reason section.
///
/// #478 is the reason the body matters: on the legacy path the plain renderer
/// is files-first, so a status-only write left the reason nowhere at all —
/// `lance/` is gitignored, so the record of why an artifact was retired would
/// simply not exist.
#[tokio::test]
async fn migrated_deprecate_writes_status_and_reason_to_canonical_markdown() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = publish_workspace(tmp.path(), "auth-rewrite-marker").await;
    write_artifact(
        &ws,
        "notes",
        "NOTE-001-constraint.md",
        "---\nid: NOTE-001\nkind: note\nstatus: active\ntitle: Constraint\ndepth: tactical\n---\n\n## Body\n\nOriginal body text.\n",
    );
    publish_only(&ws).await;

    let server = server_at(tmp.path()).await;
    let deprecated = server
        .call_tool_json(
            "forgeplan_deprecate",
            json!({ "id": "NOTE-001", "reason": "superseded by hand" }),
        )
        .await;
    let payload = deprecated.assert_ok();

    assert_eq!(
        payload["deprecated"], "NOTE-001",
        "the response must name the canonical id, got {}",
        deprecated.raw_text
    );
    // PRD-055: the deprecate path writes a soft-delete receipt so
    // `undo_last` / `restore` can bring the artifact back. It must exist on a
    // migrated workspace too, or the recovery path the tool advertises is a
    // lie.
    assert!(
        payload["receipt_id"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "deprecate must capture a soft-delete receipt on a migrated workspace, got {}",
        deprecated.raw_text
    );

    let bytes = workspace_bytes(&ws);
    assert!(
        bytes.contains("status: deprecated"),
        "the status must reach the canonical frontmatter, got {bytes}"
    );
    assert!(
        bytes.contains("## Deprecation") && bytes.contains("superseded by hand"),
        "the reason must reach the canonical body (#478), got {bytes}"
    );
    assert!(
        bytes.contains("Original body text."),
        "the original body must survive the append, got {bytes}"
    );
    assert!(
        legacy_tables(&ws).is_empty(),
        "deprecate must not create legacy tables, found {:?}",
        legacy_tables(&ws)
    );
}

/// `forgeplan_supersede` writes the successor edge into the canonical `links:`
/// block and flips the displaced artifact's status.
#[tokio::test]
async fn migrated_supersede_writes_the_edge_and_status_to_canonical_markdown() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = publish_workspace(tmp.path(), "auth-rewrite-marker").await;
    write_artifact(
        &ws,
        "notes",
        "NOTE-001-old.md",
        "---\nid: NOTE-001\nkind: note\nstatus: active\ntitle: Old\ndepth: tactical\n---\n\n## Body\n\nOld body.\n",
    );
    write_artifact(
        &ws,
        "notes",
        "NOTE-002-new.md",
        "---\nid: NOTE-002\nkind: note\nstatus: active\ntitle: New\ndepth: tactical\n---\n\n## Body\n\nNew body.\n",
    );
    publish_only(&ws).await;

    let server = server_at(tmp.path()).await;
    let superseded = server
        .call_tool_json(
            "forgeplan_supersede",
            json!({ "id": "NOTE-001", "by": "NOTE-002" }),
        )
        .await;
    let payload = superseded.assert_ok();

    assert_eq!(
        payload["superseded"], "NOTE-001",
        "the response must name the displaced artifact, got {}",
        superseded.raw_text
    );
    assert_eq!(
        payload["replacement"], "NOTE-002",
        "the response must name the successor, got {}",
        superseded.raw_text
    );

    let bytes = workspace_bytes(&ws);
    assert!(
        bytes.contains("status: superseded"),
        "the displaced artifact must be marked superseded, got {bytes}"
    );
    assert!(
        bytes.contains("relation: supersedes") && bytes.contains("target: NOTE-002"),
        "the successor edge must land in the canonical links block, got {bytes}"
    );
    assert!(
        legacy_tables(&ws).is_empty(),
        "supersede must not create legacy tables, found {:?}",
        legacy_tables(&ws)
    );
}

/// `forgeplan_review` reads the gates from the captured graph and reports the
/// evidence gate as blocking — without opening a retired table.
#[tokio::test]
async fn migrated_review_reads_gates_from_canonical_links() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = publish_workspace(tmp.path(), "auth-rewrite-marker").await;
    publish_only(&ws).await;

    let server = server_at(tmp.path()).await;
    let reviewed = server
        .call_tool_json("forgeplan_review", json!({ "id": "PRD-001" }))
        .await;
    let payload = reviewed.assert_ok();

    assert_eq!(
        payload["can_activate"],
        json!(false),
        "a PRD with no evidence cannot activate, got {}",
        reviewed.raw_text
    );
    let warnings = payload["warnings"].as_array().expect("warnings array");
    assert!(
        warnings.iter().any(|w| w
            .as_str()
            .unwrap_or_default()
            .contains("No evidence linked")),
        "the evidence gate must be reported from the capture, got {}",
        reviewed.raw_text
    );
    assert!(
        legacy_tables(&ws).is_empty(),
        "review must not create legacy tables, found {:?}",
        legacy_tables(&ws)
    );
}

#[tokio::test]
async fn a_legacy_workspace_still_writes_its_projection_row() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = forgeplan_core::workspace::init_workspace(tmp.path(), "legacy-write")
        .expect("init workspace");

    // A legacy workspace is one the store still owns — and, unlike a migrated
    // one, its `artifacts.lance` genuinely exists.
    let store = forgeplan_core::db::store::LanceStore::init(&ws)
        .await
        .expect("init legacy store");
    store
        .create_artifact_for_test(&forgeplan_core::db::store::NewArtifact {
            id: "PRD-001".into(),
            kind: "prd".into(),
            status: "draft".into(),
            title: "Legacy artifact".into(),
            body: "## Summary\n\nLegacy body.\n".into(),
            depth: "standard".into(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        })
        .await
        .expect("seed legacy row");
    std::fs::write(
        ws.join("prds").join("PRD-001-legacy-artifact.md"),
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Legacy artifact\n---\n\n## Summary\n\nLegacy body.\n",
    )
    .expect("seed the canonical file a real legacy workspace would hold");
    drop(store);

    assert!(
        !forgeplan_core::constellation::facade::workspace_is_migrated(&ws),
        "fixture precondition: an unindexed workspace is not migrated"
    );
    assert!(
        !legacy_tables(&ws).is_empty(),
        "fixture precondition: a legacy workspace holds the tables the migrated \
         branch must not create"
    );

    let server = server_at(tmp.path()).await;
    server
        .call_tool_json(
            "forgeplan_update",
            json!({ "id": "PRD-001", "title": "Legacy artifact renamed" }),
        )
        .await
        .assert_ok();

    assert!(
        workspace_bytes(&ws).contains("Legacy artifact renamed"),
        "the file-first write must land on disk on a legacy workspace too"
    );

    // The legacy projection is still updated — that is what distinguishes a
    // legacy workspace from a migrated one, and losing it would silently
    // regress every non-migrated user.
    let reopened = forgeplan_core::db::store::LanceStore::open(&ws)
        .await
        .expect("reopen the legacy store");
    let row = reopened
        .get_record("PRD-001")
        .await
        .expect("read the legacy row")
        .expect("the row must still exist");
    assert_eq!(
        row.title, "Legacy artifact renamed",
        "a legacy write must still take the derived row"
    );
}
