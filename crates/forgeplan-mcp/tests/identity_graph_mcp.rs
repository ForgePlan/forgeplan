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
fn mint_fixture_key(kind: &str, title: &str) -> String {
    let kind = kind
        .parse::<forgeplan_core::artifact::types::ArtifactKind>()
        .expect("valid artifact kind");
    forgeplan_core::artifact::canonical_key::mint_artifact_key(&kind, title)
        .expect("mint canonical key")
}

async fn seed_two_prds_sharing_one_display_id(
    store: Arc<LanceStore>,
    seed_guard: Arc<workspace::lock::WorkspaceLock>,
) {
    let mut keys = Vec::new();
    for title in ["Alpha", "Beta"] {
        let key = mint_fixture_key("prd", title);
        let body = format!(
            "---\nid: PRD-001\nkind: prd\ntitle: {title}\nslug: {key}\n---\n\n{title} body."
        );
        store
            .create_artifact_for_test(
                &seed_guard,
                &NewArtifact {
                    canonical_key: key.clone(),
                    id: "PRD-001".to_string(),
                    kind: "prd".to_string(),
                    status: "draft".to_string(),
                    title: title.to_string(),
                    body,
                    depth: "standard".to_string(),
                    author: None,
                    parent_epic: None,
                    valid_until: None,
                    tags: Vec::new(),
                },
            )
            .await
            .expect("seed artifact");
        keys.push(key);
    }
    store
        .add_relation_for_test(&seed_guard, &keys[0], &keys[1], "based_on")
        .await
        .expect("seed relation");
}

async fn seed_topology_with_shared_alias(
    store: Arc<LanceStore>,
    guard: Arc<workspace::lock::WorkspaceLock>,
) {
    for (id, title, kind, status) in [
        ("PRD-001", "draft PRD", "prd", "draft"),
        ("PRD-001", "active PRD", "prd", "active"),
        ("RFC-001", "draft dependency", "rfc", "draft"),
        ("RFC-002", "active dependency", "rfc", "draft"),
    ] {
        let key = mint_fixture_key(kind, title);
        let body = format!(
            "---\nid: {id}\nkind: {kind}\ntitle: {title}\nslug: {key}\n---\n\n{title} body."
        );
        store
            .create_artifact_for_test(
                &guard,
                &NewArtifact {
                    canonical_key: key,
                    id: id.to_string(),
                    kind: kind.to_string(),
                    status: status.to_string(),
                    title: title.to_string(),
                    body,
                    depth: "standard".to_string(),
                    author: None,
                    parent_epic: None,
                    valid_until: None,
                    tags: Vec::new(),
                },
            )
            .await
            .expect("seed topology artifact");
    }
    let records = store
        .list_records(&*guard, None)
        .await
        .expect("list topology seeds");
    let prds: Vec<_> = records
        .iter()
        .filter(|record| record.kind == "prd")
        .collect();
    let rfcs: Vec<_> = records
        .iter()
        .filter(|record| record.kind == "rfc")
        .collect();
    let draft_prd = prds
        .iter()
        .find(|record| record.title == "draft PRD")
        .unwrap();
    let active_prd = prds
        .iter()
        .find(|record| record.title == "active PRD")
        .unwrap();
    let draft_rfc = rfcs.iter().find(|record| record.id == "RFC-001").unwrap();
    let active_rfc = rfcs.iter().find(|record| record.id == "RFC-002").unwrap();
    for (from, to) in [
        (&draft_rfc.canonical_key, &draft_prd.canonical_key),
        (&active_rfc.canonical_key, &active_prd.canonical_key),
    ] {
        store
            .add_relation_for_test(&guard, from, to, "based_on")
            .await
            .expect("seed dependency");
    }
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
    let listed = fixture
        .call_tool_json("forgeplan_list", serde_json::json!({}))
        .await
        .assert_ok()
        .clone();
    let records = listed["artifacts"].as_array().expect("artifact list");
    let key_for = |title: &str| {
        records
            .iter()
            .find(|record| record["title"] == title)
            .unwrap_or_else(|| panic!("missing seeded record {title:?}"))["canonical_key"]
            .as_str()
            .expect("canonical key")
            .to_string()
    };
    let alpha_key = key_for("Alpha");
    let beta_key = key_for("Beta");
    assert_ne!(alpha_key, beta_key);
    let node_id = |label: &str| {
        mermaid
            .lines()
            .find_map(|line| {
                let (node, rendered_label) = line.split_once("[\"")?;
                rendered_label
                    .starts_with(label)
                    .then(|| node.trim().to_string())
            })
            .unwrap_or_else(|| panic!("graph node label {label:?} missing: {mermaid}"))
    };
    let alpha_node = node_id("PRD-001 · Alpha");
    let beta_node = node_id("PRD-001 · Beta");

    assert!(
        mermaid.contains("PRD-001 · Alpha") && mermaid.contains("PRD-001 · Beta"),
        "each claimant must remain a distinct vertex with its title: {mermaid}"
    );
    assert_eq!(
        mermaid.matches("PRD-001 · ").count(),
        2,
        "both documents sharing the display id must remain distinct vertices: {mermaid}"
    );
    assert!(mermaid.contains(&format!("{alpha_node} -->|based_on| {beta_node}")));
    assert!(
        !mermaid.contains(r#""PRD-001["#),
        "the shared display id must never be a node id: {mermaid}"
    );
}

#[tokio::test]
async fn order_and_blocked_use_canonical_keys_for_shared_aliases() {
    let fixture = McpFixture::new_with_seed(seed_topology_with_shared_alias).await;
    let listed = fixture
        .call_tool_json("forgeplan_list", serde_json::json!({}))
        .await
        .assert_ok()
        .clone();
    let records = listed["artifacts"].as_array().expect("artifact list");
    let key_for = |title: &str| {
        records
            .iter()
            .find(|record| record["title"] == title)
            .unwrap_or_else(|| panic!("missing seeded record {title:?}"))["canonical_key"]
            .as_str()
            .expect("canonical key")
            .to_string()
    };
    let draft_key = key_for("draft dependency");
    let active_key = key_for("active dependency");
    let draft_prd_key = key_for("draft PRD");
    let active_prd_key = key_for("active PRD");
    let expected_entry = entry_for_draft(&draft_key, &draft_prd_key);
    let order = fixture
        .call_tool_json("forgeplan_order", serde_json::json!({}))
        .await
        .assert_ok()
        .clone();
    let blocked = order["blocked"].as_array().unwrap();
    let entry = blocked
        .iter()
        .find(|entry| entry["canonical_key"] == draft_key)
        .unwrap();
    assert_eq!(entry["id"], "RFC-001");
    assert_eq!(entry["blocked_by"], serde_json::json!(["PRD-001"]));
    assert_eq!(
        entry["blocked_by_canonical_keys"],
        serde_json::json!([draft_prd_key])
    );
    assert!(
        !blocked
            .iter()
            .any(|entry| entry["canonical_key"] == active_key)
    );
    assert!(
        order["ready_canonical_keys"]
            .as_array()
            .unwrap()
            .iter()
            .any(|key| key == &active_key)
    );
    assert!(
        order["order"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == "RFC-001")
    );

    let all = fixture
        .call_tool_json("forgeplan_blocked", serde_json::json!({}))
        .await
        .assert_ok()
        .clone();
    assert!(
        all["blocked"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry == &expected_entry)
    );
    let target = fixture
        .call_tool_json("forgeplan_blocked", serde_json::json!({"id": draft_key}))
        .await
        .assert_ok()
        .clone();
    assert_eq!(target["blocked"], serde_json::json!([expected_entry]));
    let active = fixture
        .call_tool_json("forgeplan_blocked", serde_json::json!({"id": active_key}))
        .await
        .assert_ok()
        .clone();
    assert_eq!(active["blocked_count"], 0);

    let mut before: Vec<_> = std::fs::read_dir(fixture.workspace_path.join("prds"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (path.clone(), std::fs::read(path).unwrap())
        })
        .collect();
    before.sort_by(|a, b| a.0.cmp(&b.0));
    let ambiguous = fixture
        .call_tool_json("forgeplan_blocked", serde_json::json!({"id": "PRD-001"}))
        .await;
    assert!(ambiguous.is_error);
    assert!(ambiguous.raw_text.contains("ambiguous"));
    assert!(ambiguous.raw_text.contains(&draft_prd_key));
    assert!(ambiguous.raw_text.contains(&active_prd_key));
    for (path, contents) in before {
        assert_eq!(std::fs::read(path).unwrap(), contents);
    }
    let after = fixture
        .call_tool_json("forgeplan_blocked", serde_json::json!({}))
        .await
        .assert_ok()
        .clone();
    assert_eq!(after["blocked"], all["blocked"]);
}

fn entry_for_draft(draft_key: &str, draft_prd_key: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "RFC-001",
        "canonical_key": draft_key,
        "blocked_by": ["PRD-001"],
        "blocked_by_canonical_keys": [draft_prd_key]
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
    let key = mint_fixture_key("prd", "Alpha");
    let body = format!(
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Alpha\nslug: {key}\n---\n\nAlpha body.\n"
    );
    let canonical_path = std::path::PathBuf::from(format!("prds/{key}.md"));
    std::fs::write(workspace_dir.join(&canonical_path), &body).unwrap();

    // The capture route: the pinned read view of the workspace's Markdown.
    // It is read BEFORE the index exists, so the capture sees the
    // documents as they are on disk.
    let read = facade::open_workspace_read(&workspace_dir, None, CaptureLimits::default())
        .await
        .expect("the workspace reads");
    let captured = forgeplan_core::artifact::documents::capture_candidates(&read);
    assert_eq!(captured.len(), 1, "the capture holds the document");
    assert_eq!(
        captured[0].relative_path, canonical_path,
        "the capture knows the workspace-relative path"
    );

    // The index route: seed the same minted canonical key. The row is
    // keyed by the same identity found in the real canonical Markdown file.
    let index_temp = tempfile::TempDir::new().unwrap();
    let index_ws = workspace::init_workspace(index_temp.path(), "identity-index")
        .expect("init index workspace");
    let guard = workspace::acquire_workspace_lock(&index_ws).await.unwrap();
    let store = LanceStore::init_new(&index_ws, &guard)
        .await
        .expect("init store");
    store
        .create_artifact_for_test(
            &guard,
            &NewArtifact {
                canonical_key: key.clone(),
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
            },
        )
        .await
        .expect("seed the row");
    let rows = store
        .list_records(&guard, None)
        .await
        .expect("list the rows");
    let indexed = forgeplan_core::artifact::documents::candidates_from_records(&rows);
    assert_eq!(indexed.len(), 1, "the index holds the row");

    assert_eq!(
        captured[0].canonical_key, indexed[0].canonical_key,
        "the capture and the index must name one document the same way"
    );
    assert_eq!(
        captured[0].canonical_key, key,
        "the capture and index retain the minted canonical key"
    );
    assert_eq!(captured[0].display_id, indexed[0].display_id);
    assert_eq!(captured[0].title, indexed[0].title);
}
