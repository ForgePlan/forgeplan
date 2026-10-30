//! The identity graph and the shared identity, seen from the MCP side.
//!
//! SPEC-009 asks two things of the graph and one of the identity:
//!
//! * the graph's node id is the document's CANONICAL KEY and every node is
//!   labelled with its display id and its title — two documents that share
//!   a display id stay two vertices;
//! * the capture and the legacy index are DIFFERENT document sets — the
//!   capture knows each document's relative path, the index does not — but
//!   they must agree on IDENTITY: the same document gets the same canonical
//!   key from both routes.

use std::sync::Arc;

use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade;
use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::workspace;

mod common;
use common::McpFixture;

/// Seed two documents that carry the SAME display id, linked to each other.
///
/// Their canonical keys differ, so neither is an identity collision: the
/// display id is the alias they share, and it is the alias that must not
/// pick a winner.
async fn seed_two_prds_sharing_one_display_id(store: Arc<LanceStore>) {
    for (key, title) in [("prd-alpha", "Alpha"), ("prd-beta", "Beta")] {
        store
            .create_artifact_for_test(&NewArtifact {
                canonical_key: key.to_string(),
                id: "PRD-001".to_string(),
                kind: "prd".to_string(),
                status: "draft".to_string(),
                title: title.to_string(),
                body: format!("{title} body."),
                depth: "standard".to_string(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .expect("seed artifact");
    }
    store
        .add_relation_for_test("prd-alpha", "prd-beta", "based_on")
        .await
        .expect("seed relation");
}

/// The graph keeps two documents that share a display id apart.
///
/// The node id is the canonical key — the identity — and every node is
/// labelled with its display id AND its title. Two claimants of one display
/// id are two vertices, each with its own title; merging them would lose a
/// document.
#[tokio::test]
async fn the_graph_keeps_two_documents_that_share_a_display_id_apart() {
    let fixture = McpFixture::new_with_seed(seed_two_prds_sharing_one_display_id).await;

    let envelope = fixture
        .call_tool_json("forgeplan_graph", serde_json::json!({}))
        .await;
    let payload = envelope.assert_ok();
    let mermaid = payload["mermaid"].as_str().expect("the graph payload");

    assert!(
        mermaid.contains(r#"prd-alpha["PRD-001 · Alpha"]"#),
        "the first claimant must be its own vertex, labelled with its own title: {mermaid}"
    );
    assert!(
        mermaid.contains(r#"prd-beta["PRD-001 · Beta"]"#),
        "the second claimant must be its own vertex, labelled with its own title: {mermaid}"
    );
    assert!(
        mermaid.contains("prd-alpha -->|based_on| prd-beta"),
        "the edge must address the canonical keys: {mermaid}"
    );
    assert!(
        !mermaid.contains(r#""PRD-001["#),
        "the shared display id must never be a node id: {mermaid}"
    );
}

/// The capture and the index name one document the same way.
///
/// They are different document sets — the capture knows each document's
/// workspace-relative path, the index records none — and they are read
/// through different routes. What they must agree on is the IDENTITY: the
/// same document gets the same canonical key from both, because both apply
/// the ADR-030 rule to the same frontmatter.
#[tokio::test]
async fn the_capture_and_the_index_name_one_document_the_same_way() {
    let temp = tempfile::TempDir::new().unwrap();
    let workspace_dir = workspace::init_workspace(temp.path(), "identity-routes").expect("init");
    let body = "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Alpha\nslug: prd-alpha\n---\n\nAlpha body.\n";
    std::fs::write(workspace_dir.join("prds").join("PRD-001-alpha.md"), body).unwrap();

    // The capture route: the pinned read view of the workspace's Markdown.
    // It is read BEFORE the index exists, so the capture sees the
    // documents as they are on disk.
    let read = facade::open_workspace_read(&workspace_dir, None, CaptureLimits::default())
        .await
        .expect("the workspace reads");
    let captured = forgeplan_core::artifact::documents::capture_candidates(&read);
    assert_eq!(captured.len(), 1, "the capture holds the document");
    assert_eq!(
        captured[0].relative_path,
        std::path::PathBuf::from("prds/PRD-001-alpha.md"),
        "the capture knows the workspace-relative path"
    );

    // The index route: the legacy index's own rows. The row is keyed by
    // the key the document itself declares — no key handed in — so the
    // index derives it the way the capture does.
    let store = LanceStore::init(&workspace_dir).await.expect("init store");
    store
        .create_artifact_for_test(&NewArtifact {
            canonical_key: String::new(),
            id: "PRD-001".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Alpha".to_string(),
            body: body.to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        })
        .await
        .expect("seed the row");
    let rows = store.list_records(None).await.expect("list the rows");
    let indexed = forgeplan_core::artifact::documents::candidates_from_records(&rows);
    assert_eq!(indexed.len(), 1, "the index holds the row");

    assert_eq!(
        captured[0].canonical_key, indexed[0].canonical_key,
        "the capture and the index must name one document the same way"
    );
    assert_eq!(
        captured[0].canonical_key, "prd-alpha",
        "the canonical key is the slug the document declares"
    );
    assert_eq!(captured[0].display_id, indexed[0].display_id);
    assert_eq!(captured[0].title, indexed[0].title);
}
