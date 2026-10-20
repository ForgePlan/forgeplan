//! S1 local migrated consumer transitions over the existing JSON-RPC fixture.
//!
//! Complements `constellation_migrated_read` / `constellation_migrated_write`:
//! assertions compare complete canonical bytes, parsed metadata, exact edge
//! multisets and subsequent consumer reads, not renderer wording or reachability.
//! `McpFixture::new_rooted` is intentional: `new` initializes retired tables.
//! All workspaces are test-owned; no provider, model, registry or HOME changes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade::{MaintainCapability, maintain_index};
use serde_json::{Value, json};
use serde_yaml::Value as Yaml;
use tempfile::TempDir;

mod common;
use common::{CallToolEnvelope, McpFixture};

const TOKEN: &str = "s1canonicalroundtriptoken";
const SOURCE_ID: &str = "NOTE-001";
const TARGET_ID: &str = "NOTE-002";
const OBSERVER_ID: &str = "NOTE-003";

type Edges = Vec<(String, String)>;

#[derive(Debug)]
struct Document {
    raw: String,
    metadata: BTreeMap<String, Yaml>,
    prose: String,
}

// Independent, strict parsing of fixture/result data. Malformed entries must
// fail, not disappear through the production parser's best-effort link reader.
fn document(path: &Path) -> Document {
    let raw = std::fs::read_to_string(path).expect("read canonical Markdown");
    let rest = raw
        .strip_prefix("---\n")
        .expect("opening frontmatter delimiter");
    let (yaml, prose) = rest
        .split_once("\n---\n")
        .expect("closing frontmatter delimiter");
    let metadata = serde_yaml::from_str(yaml).expect("valid canonical YAML");
    let prose = prose.trim_start_matches('\n').to_owned();
    Document {
        raw,
        metadata,
        prose,
    }
}

fn artifact_paths(ws: &Path, id: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(ws.join("notes")).expect("list canonical notes") {
        let path = entry.expect("read directory entry").path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md")
            && document(&path).metadata.get("id").and_then(Yaml::as_str) == Some(id)
        {
            paths.push(path);
        }
    }
    paths.sort();
    paths
}

fn only_artifact(ws: &Path, id: &str) -> PathBuf {
    let paths = artifact_paths(ws, id);
    assert_eq!(
        paths.len(),
        1,
        "{id} must have exactly one canonical file: {paths:?}"
    );
    paths.into_iter().next().expect("one canonical file")
}

fn note_bytes(ws: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(ws.join("notes")).expect("list canonical notes") {
        let path = entry.expect("read directory entry").path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            let relative = path.strip_prefix(ws).expect("local note").to_path_buf();
            files.insert(relative, std::fs::read(path).expect("read canonical bytes"));
        }
    }
    files
}

fn assert_no_legacy_tables(ws: &Path) {
    for name in ["artifacts.lance", "relations.lance", "change_log.lance"] {
        assert!(
            !ws.join("lance").join(name).exists(),
            "retired table was created: {name}"
        );
    }
}

fn generation(ws: &Path) -> String {
    forgeplan_core::constellation::storage::read_current(&ws.join("lance"))
        .expect("read CURRENT")
        .expect("published CURRENT exists")
        .generation_id
}

fn current_bytes(ws: &Path) -> Vec<u8> {
    std::fs::read(ws.join("lance/CURRENT.json")).expect("read exact CURRENT bytes")
}

async fn workspace(token: &str) -> (TempDir, PathBuf) {
    let root = TempDir::new().expect("temporary workspace");
    let ws = forgeplan_core::workspace::init_workspace(root.path(), "s1-migrated-e2e")
        .expect("initialize local workspace only");
    let source = format!(
        "---\n# Exact-byte recovery must retain this comment.\nid: NOTE-001\nkind: note\nstatus: active\ntitle: Source\ndepth: tactical\nauthor: Eli Rum\nvalid_until: '2099-01-01T00:00:00Z'\ntags: [s1-local, constraint]\nx_s1_contract:\n  labels: [alpha, beta]\n  enabled: true\nlinks:\n  - target: NOTE-002\n    relation: based_on\n---\n\n## Summary\n\n{token}: preserve quotes \"here\", backslash \\ and Unicode λ.\n\n## Constraint\n\nKeep the canonical body and declared links together.\n"
    );
    std::fs::write(ws.join("notes/NOTE-001-source.md"), source).expect("seed source");
    std::fs::write(
        ws.join("notes/NOTE-002-target.md"),
        "---\nid: NOTE-002\nkind: note\nstatus: active\ntitle: Target\ndepth: tactical\n---\n\n## Summary\n\nIndependent target body.\n",
    ).expect("seed target");
    std::fs::write(
        ws.join("notes/NOTE-003-observer.md"),
        "---\nid: NOTE-003\nkind: note\nstatus: active\ntitle: Observer\ndepth: tactical\nlinks:\n  - target: NOTE-001\n    relation: informs\n---\n\n## Summary\n\nIndependent observer body.\n",
    ).expect("seed incoming relation");
    let capability = MaintainCapability::for_strict_maintenance(
        ws.canonicalize().expect("canonical workspace directory"),
    );
    maintain_index(
        &capability,
        CaptureLimits::default(),
        Instant::now() + Duration::from_secs(120),
    )
    .await
    .expect("publish migrated fixture without inference");
    assert!(forgeplan_core::constellation::facade::workspace_is_migrated(&ws));
    assert_no_legacy_tables(&ws);
    (root, ws)
}

async fn call(
    server: &McpFixture,
    ws: &Path,
    tool: &'static str,
    mut args: Value,
) -> CallToolEnvelope {
    args.as_object_mut()
        .expect("tool arguments are an object")
        .insert("workspace".to_owned(), json!(ws.to_string_lossy()));
    let result = server.try_call_tool(tool, args).await;
    // Check even when the server returns a JSON-RPC error rather than tool data.
    assert_no_legacy_tables(ws);
    CallToolEnvelope::from(result.unwrap_or_else(|error| panic!("{tool} JSON-RPC error: {error}")))
}

async fn ok(server: &McpFixture, ws: &Path, tool: &'static str, args: Value) -> Value {
    let response = call(server, ws, tool, args).await;
    response.assert_ok();
    response.json.expect("successful JSON tool payload")
}

// No MaintenanceHost is started by this fixture. A successful changed write
// therefore owes a bounded publication, not a promise of work by another task.
async fn changed(server: &McpFixture, ws: &Path, tool: &'static str, args: Value) -> Value {
    let before = generation(ws);
    let payload = ok(server, ws, tool, args).await;
    assert_eq!(payload["index_pending"], false, "{tool}: {payload}");
    assert!(
        payload.get("index_pending_reason").is_none(),
        "{tool}: {payload}"
    );
    let after = generation(ws);
    assert_ne!(
        after, before,
        "{tool} did not publish its changed canonical source"
    );
    assert_eq!(
        payload["index_generation"].as_str(),
        Some(after.as_str()),
        "{tool}: {payload}"
    );
    payload
}

fn edges(pairs: &[(&str, &str)]) -> Edges {
    let mut result: Edges = pairs
        .iter()
        .map(|(peer, relation)| ((*peer).into(), (*relation).into()))
        .collect();
    result.sort();
    result
}

fn canonical_edges(doc: &Document) -> Edges {
    let mut result = match doc.metadata.get("links") {
        None => Vec::new(),
        Some(value) => value
            .as_sequence()
            .expect("links is a sequence")
            .iter()
            .map(|link| {
                (
                    link.get("target")
                        .and_then(Yaml::as_str)
                        .expect("link target")
                        .to_owned(),
                    link.get("relation")
                        .and_then(Yaml::as_str)
                        .expect("link relation")
                        .to_owned(),
                )
            })
            .collect(),
    };
    result.sort();
    result
}

fn consumer_edges(payload: &Value, direction: &str, peer_field: &str) -> Edges {
    let mut result: Edges = payload["links"][direction]
        .as_array()
        .expect("explicit edge array")
        .iter()
        .map(|link| {
            (
                link[peer_field].as_str().expect("edge peer").to_owned(),
                link["relation"].as_str().expect("edge relation").to_owned(),
            )
        })
        .collect();
    result.sort();
    result
}

async fn assert_record(
    server: &McpFixture,
    ws: &Path,
    id: &str,
    outgoing: &[(&str, &str)],
) -> Value {
    let doc = document(&only_artifact(ws, id));
    let payload = ok(server, ws, "forgeplan_get", json!({"id": id})).await;
    assert_eq!(payload["id"], id);
    assert_eq!(
        payload["body"].as_str(),
        Some(doc.raw.as_str()),
        "get must return these exact canonical bytes"
    );
    for key in ["kind", "status", "title", "depth"] {
        assert_eq!(
            payload[key].as_str(),
            doc.metadata.get(key).and_then(Yaml::as_str),
            "{id}: {key}"
        );
    }
    for key in ["author", "valid_until"] {
        assert_eq!(
            payload[key].as_str(),
            doc.metadata.get(key).and_then(Yaml::as_str),
            "{id}: {key}"
        );
    }
    let expected = edges(outgoing);
    // Sort but never deduplicate: duplicate edges must fail both assertions.
    assert_eq!(
        canonical_edges(&doc),
        expected,
        "canonical outgoing relations for {id}"
    );
    assert_eq!(
        consumer_edges(&payload, "outbound", "target"),
        expected,
        "consumer outgoing relations for {id}"
    );
    payload
}

fn assert_metadata_preserved(before: &Document, after: &Document, changed_fields: &[&str]) {
    let mut expected = before.metadata.clone();
    let mut actual = after.metadata.clone();
    // The initialized rmcp client may stamp these two documented audit fields.
    // No other unrequested metadata loss or addition is accepted.
    for field in ["last_modified_by", "last_modified_at"]
        .into_iter()
        .chain(changed_fields.iter().copied())
    {
        expected.remove(field);
        actual.remove(field);
    }
    assert_eq!(actual, expected, "unrequested frontmatter changed");
}

fn receipt_file(ws: &Path, response: &Value) -> PathBuf {
    let id = response["receipt_id"]
        .as_str()
        .expect("destructive operation returns receipt_id");
    assert!(!id.is_empty(), "receipt id must not be empty");
    ws.join("trash").join(format!("receipt-{id}.json"))
}

fn receipt(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("persisted receipt")).expect("receipt JSON")
}

fn assert_clean_restore(payload: &Value, id: &str) {
    assert_eq!(payload["restored"], id);
    assert_eq!(payload["projection_restored"], true);
    assert_eq!(
        payload["relations_skipped"],
        json!([]),
        "all fixture participants exist: {payload}"
    );
    assert_eq!(
        payload["warnings"],
        json!([]),
        "existing exact edges are not replay failures: {payload}"
    );
}

async fn assert_discovery(server: &McpFixture, ws: &Path, all_ids: &[&str], matching_ids: &[&str]) {
    let listed = ok(server, ws, "forgeplan_list", json!({})).await;
    let mut ids: Vec<&str> = listed["artifacts"]
        .as_array()
        .expect("artifact array")
        .iter()
        .map(|item| item["id"].as_str().expect("listed artifact id"))
        .collect();
    ids.sort();
    assert_eq!(ids, all_ids);
    assert_eq!(listed["total"].as_u64(), Some(all_ids.len() as u64));
    let searched = ok(
        server,
        ws,
        "forgeplan_search",
        json!({
            "query": TOKEN, "mode": "keyword", "no_expand": true,
        }),
    )
    .await;
    let mut ids: Vec<&str> = searched["results"]
        .as_array()
        .expect("search results")
        .iter()
        .map(|item| item["id"].as_str().expect("matched artifact id"))
        .collect();
    ids.sort();
    assert_eq!(
        ids, matching_ids,
        "search must not retain deleted canonical bytes"
    );
    assert_eq!(searched["total"].as_u64(), Some(matching_ids.len() as u64));
    assert_eq!(searched["coverage"]["lexical"], true);
    assert_eq!(searched["coverage"]["semantic"], false);
}

#[tokio::test]
async fn metadata_omission_and_explicit_empty_body_have_distinct_canonical_results() {
    let (tmp, ws) = workspace(TOKEN).await;
    let server = McpFixture::new_rooted(tmp.path().to_path_buf()).await;
    let old_path = only_artifact(&ws, SOURCE_ID);
    let original = document(&old_path);
    let target = document(&only_artifact(&ws, TARGET_ID)).raw;
    let observer = document(&only_artifact(&ws, OBSERVER_ID)).raw;

    let updated = changed(
        &server,
        &ws,
        "forgeplan_update",
        json!({
            "id": SOURCE_ID, "title": "Renamed source", "status": "draft",
        }),
    )
    .await;
    assert_eq!(updated["title"], "Renamed source");
    assert_eq!(updated["status"], "draft");
    let renamed_path = only_artifact(&ws, SOURCE_ID);
    assert_ne!(renamed_path, old_path);
    assert!(
        !old_path.exists(),
        "title rename must not leave a phantom file"
    );
    let renamed = document(&renamed_path);
    assert_eq!(
        renamed.prose, original.prose,
        "omitted body must remain byte-for-byte unchanged"
    );
    assert_metadata_preserved(&original, &renamed, &["title", "status"]);
    assert_record(&server, &ws, SOURCE_ID, &[(TARGET_ID, "based_on")]).await;

    changed(
        &server,
        &ws,
        "forgeplan_update",
        json!({"id": SOURCE_ID, "body": ""}),
    )
    .await;
    let emptied = document(&only_artifact(&ws, SOURCE_ID));
    assert_eq!(
        emptied.prose, "",
        "an explicit empty string must clear the body"
    );
    assert_metadata_preserved(&renamed, &emptied, &[]);
    let empty_read = assert_record(&server, &ws, SOURCE_ID, &[(TARGET_ID, "based_on")]).await;
    assert_eq!(empty_read["title"], "Renamed source");
    assert_eq!(empty_read["status"], "draft");

    // The same request must resolve the file renamed by its metadata half.
    let body = "## Summary\n\nReplacement \"quotes\", backslash \\ and Unicode λ.\n";
    let replaced = changed(
        &server,
        &ws,
        "forgeplan_update",
        json!({
            "id": SOURCE_ID, "title": "Final source", "body": body,
        }),
    )
    .await;
    assert_eq!(replaced["title"], "Final source");
    assert!(!renamed_path.exists());
    let final_doc = document(&only_artifact(&ws, SOURCE_ID));
    assert_eq!(final_doc.prose, body);
    assert_metadata_preserved(&emptied, &final_doc, &["title"]);
    assert_record(&server, &ws, SOURCE_ID, &[(TARGET_ID, "based_on")]).await;
    assert_eq!(document(&only_artifact(&ws, TARGET_ID)).raw, target);
    assert_eq!(document(&only_artifact(&ws, OBSERVER_ID)).raw, observer);
}

#[tokio::test]
async fn link_replace_and_unlink_preserve_the_exact_other_outgoing_edges() {
    let (tmp, ws) = workspace(TOKEN).await;
    let server = McpFixture::new_rooted(tmp.path().to_path_buf()).await;
    let source = document(&only_artifact(&ws, SOURCE_ID));
    let target = document(&only_artifact(&ws, TARGET_ID)).raw;
    let observer = document(&only_artifact(&ws, OBSERVER_ID)).raw;

    changed(
        &server,
        &ws,
        "forgeplan_link",
        json!({
            "source": SOURCE_ID, "target": OBSERVER_ID, "relation": "refines",
        }),
    )
    .await;
    assert_record(
        &server,
        &ws,
        SOURCE_ID,
        &[(TARGET_ID, "based_on"), (OBSERVER_ID, "refines")],
    )
    .await;
    let target_read = assert_record(&server, &ws, TARGET_ID, &[]).await;
    assert_eq!(
        consumer_edges(&target_read, "inbound", "source"),
        edges(&[(SOURCE_ID, "based_on")])
    );

    changed(
        &server,
        &ws,
        "forgeplan_link",
        json!({
            "source": SOURCE_ID, "target": TARGET_ID, "relation": "contradicts", "replace": true,
        }),
    )
    .await;
    assert_record(
        &server,
        &ws,
        SOURCE_ID,
        &[(TARGET_ID, "contradicts"), (OBSERVER_ID, "refines")],
    )
    .await;
    let target_read = assert_record(&server, &ws, TARGET_ID, &[]).await;
    assert_eq!(
        consumer_edges(&target_read, "inbound", "source"),
        edges(&[(SOURCE_ID, "contradicts")])
    );

    let source_before_noop = document(&only_artifact(&ws, SOURCE_ID)).raw;
    ok(
        &server,
        &ws,
        "forgeplan_link",
        json!({
            "source": SOURCE_ID, "target": TARGET_ID, "relation": "contradicts", "replace": true,
        }),
    )
    .await;
    assert_eq!(
        document(&only_artifact(&ws, SOURCE_ID)).raw,
        source_before_noop
    );
    assert_record(
        &server,
        &ws,
        SOURCE_ID,
        &[(TARGET_ID, "contradicts"), (OBSERVER_ID, "refines")],
    )
    .await;

    // Removing the obsolete relation must diagnose the missing exact triple,
    // not remove the replacement relation with the same target.
    let pointer = current_bytes(&ws);
    let refused = call(
        &server,
        &ws,
        "forgeplan_unlink",
        json!({
            "source": SOURCE_ID, "target": TARGET_ID, "relation": "based_on",
        }),
    )
    .await;
    assert!(
        refused.is_error,
        "missing exact relation unexpectedly succeeded: {refused:?}"
    );
    assert!(
        refused.raw_text.contains("not found") && refused.raw_text.contains("based_on"),
        "{refused:?}"
    );
    assert_eq!(
        document(&only_artifact(&ws, SOURCE_ID)).raw,
        source_before_noop
    );
    assert_eq!(current_bytes(&ws), pointer);

    changed(
        &server,
        &ws,
        "forgeplan_unlink",
        json!({
            "source": SOURCE_ID, "target": TARGET_ID, "relation": "contradicts",
        }),
    )
    .await;
    assert_record(&server, &ws, SOURCE_ID, &[(OBSERVER_ID, "refines")]).await;
    let target_read = assert_record(&server, &ws, TARGET_ID, &[]).await;
    assert_eq!(
        consumer_edges(&target_read, "inbound", "source"),
        edges(&[])
    );
    let observer_read = assert_record(&server, &ws, OBSERVER_ID, &[(SOURCE_ID, "informs")]).await;
    assert_eq!(
        consumer_edges(&observer_read, "inbound", "source"),
        edges(&[(SOURCE_ID, "refines")])
    );
    let after = document(&only_artifact(&ws, SOURCE_ID));
    assert_eq!(after.prose, source.prose);
    assert_metadata_preserved(&source, &after, &["links"]);
    assert_eq!(document(&only_artifact(&ws, TARGET_ID)).raw, target);
    assert_eq!(document(&only_artifact(&ws, OBSERVER_ID)).raw, observer);
}

#[tokio::test]
async fn delete_then_restore_by_artifact_id_recovers_exact_bytes_and_relations_in_explicit_workspace()
 {
    let (default_tmp, default_ws) = workspace("defaultuntouchedtoken").await;
    let (_target_tmp, ws) = workspace(TOKEN).await;
    let server = McpFixture::new_rooted(default_tmp.path().to_path_buf()).await;
    let default_before = note_bytes(&default_ws);
    let default_generation = generation(&default_ws);
    let original_notes = note_bytes(&ws);
    let original = document(&only_artifact(&ws, SOURCE_ID));

    assert_record(&server, &ws, SOURCE_ID, &[(TARGET_ID, "based_on")]).await;
    assert_discovery(
        &server,
        &ws,
        &[SOURCE_ID, TARGET_ID, OBSERVER_ID],
        &[SOURCE_ID],
    )
    .await;
    let deleted = changed(&server, &ws, "forgeplan_delete", json!({"id": SOURCE_ID})).await;
    let path = receipt_file(&ws, &deleted);
    let captured = receipt(&path);
    assert_eq!(captured["op"], "delete");
    assert_eq!(captured["consumed"], false);
    assert_eq!(
        captured["snapshot"]["body"].as_str(),
        Some(original.raw.as_str())
    );
    let mut captured_edges: Vec<(String, String, String, String)> =
        captured["snapshot"]["relations"]
            .as_array()
            .expect("captured relation array")
            .iter()
            .map(|edge| {
                (
                    edge["from"].as_str().expect("from").to_owned(),
                    edge["to"].as_str().expect("to").to_owned(),
                    edge["relation"].as_str().expect("relation").to_owned(),
                    edge["direction"].as_str().expect("direction").to_owned(),
                )
            })
            .collect();
    captured_edges.sort();
    assert_eq!(
        captured_edges,
        vec![
            (
                SOURCE_ID.into(),
                TARGET_ID.into(),
                "based_on".into(),
                "outgoing".into()
            ),
            (
                OBSERVER_ID.into(),
                SOURCE_ID.into(),
                "informs".into(),
                "incoming".into()
            ),
        ]
    );
    let trash = ws.join("trash").join(format!(
        "body-{}.md",
        deleted["receipt_id"].as_str().expect("receipt id")
    ));
    assert_eq!(
        std::fs::read(&trash).expect("trashed original bytes"),
        original.raw.as_bytes()
    );
    assert!(artifact_paths(&ws, SOURCE_ID).is_empty());
    let missing = call(&server, &ws, "forgeplan_get", json!({"id": SOURCE_ID})).await;
    assert!(
        missing.is_error
            && missing.raw_text.contains(SOURCE_ID)
            && missing.raw_text.contains("not found"),
        "{missing:?}"
    );
    assert_discovery(&server, &ws, &[TARGET_ID, OBSERVER_ID], &[]).await;

    let restored = changed(&server, &ws, "forgeplan_restore", json!({"id": SOURCE_ID})).await;
    assert_clean_restore(&restored, SOURCE_ID);
    assert_eq!(restored["op_reversed"], "delete");
    assert_eq!(restored["restored_status"], "active");
    assert_eq!(restored["restored_status_source"], "receipt_snapshot");
    assert_eq!(restored["relations_restored"], 2);
    assert_eq!(
        note_bytes(&ws),
        original_notes,
        "recovery must not re-render away comments or change neighbours"
    );
    assert!(
        !trash.exists(),
        "the original projection must have moved home"
    );
    let consumed = receipt(&path);
    assert_eq!(consumed["consumed"], true);
    assert_eq!(
        consumed["snapshot"], captured["snapshot"],
        "consuming a receipt must preserve its captured state"
    );
    let read = assert_record(&server, &ws, SOURCE_ID, &[(TARGET_ID, "based_on")]).await;
    assert_eq!(
        consumer_edges(&read, "inbound", "source"),
        edges(&[(OBSERVER_ID, "informs")])
    );
    assert_discovery(
        &server,
        &ws,
        &[SOURCE_ID, TARGET_ID, OBSERVER_ID],
        &[SOURCE_ID],
    )
    .await;

    let pointer = current_bytes(&ws);
    let again = call(&server, &ws, "forgeplan_restore", json!({"id": SOURCE_ID})).await;
    assert!(
        again.is_error && again.raw_text.contains("No non-consumed receipt"),
        "{again:?}"
    );
    assert_eq!(note_bytes(&ws), original_notes);
    assert_eq!(current_bytes(&ws), pointer);
    assert_eq!(
        note_bytes(&default_ws),
        default_before,
        "explicit Y recovery must not write default X"
    );
    assert_eq!(generation(&default_ws), default_generation);
    assert_no_legacy_tables(&default_ws);
}

#[tokio::test]
async fn invalid_restore_override_is_non_mutating_and_a_valid_override_preserves_the_receipt_snapshot()
 {
    let (tmp, ws) = workspace(TOKEN).await;
    let server = McpFixture::new_rooted(tmp.path().to_path_buf()).await;
    let original = document(&only_artifact(&ws, SOURCE_ID));
    let deleted = changed(&server, &ws, "forgeplan_delete", json!({"id": SOURCE_ID})).await;
    let path = receipt_file(&ws, &deleted);
    let receipt_before = std::fs::read(&path).expect("receipt before refused override");
    let trash = ws.join("trash").join(format!(
        "body-{}.md",
        deleted["receipt_id"].as_str().expect("receipt id")
    ));
    let trash_before = std::fs::read(&trash).expect("trash before refused override");
    let pointer = current_bytes(&ws);

    let invalid = call(
        &server,
        &ws,
        "forgeplan_restore",
        json!({
            "id": SOURCE_ID, "target_status": "not-a-status",
        }),
    )
    .await;
    assert!(
        invalid.is_error && invalid.raw_text.contains("Invalid target_status"),
        "{invalid:?}"
    );
    assert_eq!(
        std::fs::read(&path).expect("receipt after refusal"),
        receipt_before
    );
    assert_eq!(
        std::fs::read(&trash).expect("trash after refusal"),
        trash_before
    );
    assert!(artifact_paths(&ws, SOURCE_ID).is_empty());
    assert_eq!(current_bytes(&ws), pointer);

    let restored = changed(
        &server,
        &ws,
        "forgeplan_restore",
        json!({
            "id": SOURCE_ID, "target_status": "draft",
        }),
    )
    .await;
    assert_clean_restore(&restored, SOURCE_ID);
    assert_eq!(restored["restored_status"], "draft");
    assert_eq!(restored["restored_status_source"], "explicit_override");
    let after = document(&only_artifact(&ws, SOURCE_ID));
    assert_eq!(after.metadata["status"].as_str(), Some("draft"));
    assert_eq!(after.prose, original.prose);
    assert_metadata_preserved(&original, &after, &["status"]);
    assert_record(&server, &ws, SOURCE_ID, &[(TARGET_ID, "based_on")]).await;
    let saved = receipt(&path);
    assert_eq!(saved["consumed"], true);
    assert_eq!(
        saved["snapshot"]["status"], "active",
        "an override must not rewrite history"
    );
    assert_eq!(
        saved["snapshot"]["body"].as_str(),
        Some(original.raw.as_str())
    );
}

#[tokio::test]
async fn terminal_restore_override_cannot_bypass_the_canonical_lifecycle_fsm() {
    let (tmp, ws) = workspace(TOKEN).await;
    let server = McpFixture::new_rooted(tmp.path().to_path_buf()).await;
    let deprecated = changed(
        &server,
        &ws,
        "forgeplan_deprecate",
        json!({
            "id": SOURCE_ID, "reason": "S1 terminal restore override fixture",
        }),
    )
    .await;
    let path = receipt_file(&ws, &deprecated);
    let receipt_before = std::fs::read(&path).expect("receipt before refused FSM override");
    let terminal = document(&only_artifact(&ws, SOURCE_ID));
    assert_eq!(terminal.metadata["status"].as_str(), Some("deprecated"));
    let notes_before = note_bytes(&ws);
    let pointer = current_bytes(&ws);

    let rejected = call(
        &server,
        &ws,
        "forgeplan_restore",
        json!({
            "id": SOURCE_ID, "target_status": "active",
        }),
    )
    .await;
    assert!(
        rejected.is_error
            && rejected.raw_text.contains("lifecycle FSM")
            && rejected.raw_text.contains("deprecated"),
        "{rejected:?}"
    );
    assert_eq!(
        note_bytes(&ws),
        notes_before,
        "FSM refusal must not mutate canonical data"
    );
    assert_eq!(
        std::fs::read(&path).expect("receipt after FSM refusal"),
        receipt_before
    );
    assert_eq!(current_bytes(&ws), pointer);

    // A receipt-driven reversal remains available; an override is not a way
    // to turn that recovery operation into an arbitrary lifecycle transition.
    let restored = changed(&server, &ws, "forgeplan_restore", json!({"id": SOURCE_ID})).await;
    assert_clean_restore(&restored, SOURCE_ID);
    assert_eq!(restored["restored_status"], "active");
    assert_eq!(restored["restored_status_source"], "receipt_snapshot");
    let after = document(&only_artifact(&ws, SOURCE_ID));
    assert_eq!(
        after.prose, terminal.prose,
        "reversal must preserve the recorded deprecation history"
    );
    assert_record(&server, &ws, SOURCE_ID, &[(TARGET_ID, "based_on")]).await;
    assert_eq!(receipt(&path)["consumed"], true);
}

#[tokio::test]
async fn undo_last_skips_a_consumed_receipt_and_restores_the_remaining_deleted_artifact() {
    let (tmp, ws) = workspace(TOKEN).await;
    let server = McpFixture::new_rooted(tmp.path().to_path_buf()).await;
    let original = note_bytes(&ws);
    let source_deleted = changed(&server, &ws, "forgeplan_delete", json!({"id": SOURCE_ID})).await;
    let source_receipt = receipt_file(&ws, &source_deleted);
    let target_deleted = changed(&server, &ws, "forgeplan_delete", json!({"id": TARGET_ID})).await;
    let target_receipt = receipt_file(&ws, &target_deleted);

    let target_restored =
        changed(&server, &ws, "forgeplan_restore", json!({"id": TARGET_ID})).await;
    assert_clean_restore(&target_restored, TARGET_ID);
    assert_eq!(receipt(&target_receipt)["consumed"], true);
    assert_eq!(receipt(&source_receipt)["consumed"], false);
    assert!(artifact_paths(&ws, SOURCE_ID).is_empty());

    let undone = changed(
        &server,
        &ws,
        "forgeplan_undo_last",
        json!({"within_hours": 24}),
    )
    .await;
    assert_clean_restore(&undone, SOURCE_ID);
    assert_eq!(undone["op_reversed"], "delete");
    assert_eq!(undone["receipt_id"], source_deleted["receipt_id"]);
    assert_eq!(undone["relations_restored"], 2);
    assert_eq!(
        note_bytes(&ws),
        original,
        "undo must recover original bytes without duplicating links"
    );
    assert_eq!(receipt(&source_receipt)["consumed"], true);
    assert_eq!(receipt(&target_receipt)["consumed"], true);
    assert_record(&server, &ws, SOURCE_ID, &[(TARGET_ID, "based_on")]).await;
    let target = assert_record(&server, &ws, TARGET_ID, &[]).await;
    assert_eq!(
        consumer_edges(&target, "inbound", "source"),
        edges(&[(SOURCE_ID, "based_on")])
    );
    assert_record(&server, &ws, OBSERVER_ID, &[(SOURCE_ID, "informs")]).await;
}
