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
async fn seed_two_prds_sharing_one_display_id(store: Arc<LanceStore>, seed_guard: Arc<workspace::lock::WorkspaceLock>) {
    for (key, title) in [("prd-alpha", "Alpha"), ("prd-beta", "Beta")] {
        store
            .create_artifact_for_test(&seed_guard, &NewArtifact {
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
        .add_relation_for_test(&seed_guard, "prd-alpha", "prd-beta", "based_on")
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

async fn seed_topology_with_shared_alias(store: Arc<LanceStore>, guard: Arc<workspace::lock::WorkspaceLock>) {
    for (key, id, status) in [
        ("prd-draft", "PRD-001", "draft"),
        ("prd-active", "PRD-001", "active"),
        ("rfc-draft-dependency", "RFC-001", "draft"),
        ("rfc-active-dependency", "RFC-002", "draft"),
    ] {
        store.create_artifact_for_test(&guard, &NewArtifact {
            canonical_key: key.to_string(),
            id: id.to_string(),
            kind: if id.starts_with("PRD") { "prd" } else { "rfc" }.to_string(),
            status: status.to_string(),
            title: key.to_string(),
            body: format!("{key} body."),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        }).await.expect("seed topology artifact");
    }
    for (from, to) in [
        ("rfc-draft-dependency", "prd-draft"),
        ("rfc-active-dependency", "prd-active"),
    ] {
        store.add_relation_for_test(&guard, from, to, "based_on")
            .await.expect("seed dependency");
    }
}

#[tokio::test]
async fn order_and_blocked_use_canonical_keys_for_shared_aliases() {
    let fixture = McpFixture::new_with_seed(seed_topology_with_shared_alias).await;
    let order_envelope = fixture.call_tool_json("forgeplan_order", serde_json::json!({})).await;
    let order = order_envelope.assert_ok();
    let blocked = order["blocked"].as_array().unwrap();
    let entry = blocked.iter().find(|entry| entry["canonical_key"] == "rfc-draft-dependency").unwrap();
    assert_eq!(entry["id"], "RFC-001");
    assert_eq!(entry["blocked_by"], serde_json::json!(["PRD-001"]));
    assert_eq!(entry["blocked_by_canonical_keys"], serde_json::json!(["prd-draft"]));
    assert!(!blocked.iter().any(|entry| entry["canonical_key"] == "rfc-active-dependency"));
    assert!(order["ready_canonical_keys"].as_array().unwrap().iter().any(|key| key == "rfc-active-dependency"));
    assert!(order["order"].as_array().unwrap().iter().any(|id| id == "RFC-001"));

    let all_envelope = fixture.call_tool_json("forgeplan_blocked", serde_json::json!({})).await;
    let all = all_envelope.assert_ok();
    assert!(all["blocked"].as_array().unwrap().iter().any(|entry| entry == &entry_for_draft()));
    let target_envelope = fixture.call_tool_json("forgeplan_blocked", serde_json::json!({"id": "rfc-draft-dependency"})).await;
    let target = target_envelope.assert_ok();
    assert_eq!(target["blocked"], serde_json::json!([entry_for_draft()]));
    let active_envelope = fixture.call_tool_json("forgeplan_blocked", serde_json::json!({"id": "rfc-active-dependency"})).await;
    assert_eq!(active_envelope.assert_ok()["blocked_count"], 0);

    let mut before: Vec<_> = std::fs::read_dir(fixture.workspace_path.join("prds")).unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (path.clone(), std::fs::read(path).unwrap())
        }).collect();
    before.sort_by(|a, b| a.0.cmp(&b.0));
    let ambiguous = fixture.call_tool_json("forgeplan_blocked", serde_json::json!({"id": "PRD-001"})).await;
    assert!(ambiguous.is_error);
    assert!(ambiguous.raw_text.contains("ambiguous"));
    assert!(ambiguous.raw_text.contains("prd-draft"));
    assert!(ambiguous.raw_text.contains("prd-active"));
    for (path, contents) in before {
        assert_eq!(std::fs::read(path).unwrap(), contents);
    }
    let after_envelope = fixture.call_tool_json("forgeplan_blocked", serde_json::json!({})).await;
    assert_eq!(after_envelope.assert_ok()["blocked"], all["blocked"]);
}

fn entry_for_draft() -> serde_json::Value {
    serde_json::json!({
        "id": "RFC-001",
        "canonical_key": "rfc-draft-dependency",
        "blocked_by": ["PRD-001"],
        "blocked_by_canonical_keys": ["prd-draft"]
    })
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
    let index_temp = tempfile::TempDir::new().unwrap();
    let index_ws = workspace::init_workspace(index_temp.path(), "identity-index").expect("init index workspace");
    let guard = workspace::acquire_workspace_lock(&index_ws).await.unwrap();
    let store = LanceStore::init_new(&index_ws, &guard).await.expect("init store");
    store
        .create_artifact_for_test(&guard, &NewArtifact {
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
    let rows = store.list_records(&guard, None).await.expect("list the rows");
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
