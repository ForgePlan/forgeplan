//! The shared resolution contract, seen from the MCP side.
//!
//! SPEC-009 freezes the resolution order and the error taxonomy; this file
//! pins the MCP half of it:
//!
//! * the refusal kinds stay apart — `not_found`, `ambiguous`,
//!   `conflicting_canonical_key`, `invalid_selector` and
//!   `incomplete_capture` are five different facts, and none of them
//!   collapses into "not found";
//! * an ambiguous reference reaches the agent as a tool error that carries
//!   a machine-readable code, the input and the candidates — and no
//!   absolute path;
//! * a source-qualified get names ONE source and returns that source's
//!   exact body, while an unqualified get over the same scope reports the
//!   ambiguity explicitly (SPEC-009 scenario V4).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use forgeplan_core::artifact::canonical_key::ArtifactCandidate;
use forgeplan_core::constellation::registry;
use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::workspace;
use forgeplan_mcp::convert::{describe_candidates, resolution_code, resolve_reference};
use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, CallToolResult};
use serde_json::Value;
use serial_test::serial;
use tempfile::TempDir;

mod common;
use common::{CallToolEnvelope, McpFixture, value_to_object};

/// One document of the resolver's document set.
fn document(key: &str, display: &str, title: &str, path: &str) -> ArtifactCandidate {
    ArtifactCandidate {
        canonical_key: key.to_string(),
        display_id: display.to_string(),
        title: title.to_string(),
        relative_path: PathBuf::from(path),
    }
}

/// The refusal kinds stay apart.
///
/// An absence, an ambiguity, a contested key and an unusable reference are
/// four different facts. Collapsing them into "not found" is how a write
/// lands on the wrong document.
#[test]
fn the_refusal_kinds_stay_apart() {
    let documents = vec![
        document("prd-alpha", "PRD-001", "Alpha", "prds/PRD-001-alpha.md"),
        document("prd-beta", "PRD-002", "Beta", "prds/PRD-001-beta.md"),
    ];

    // No document carries the name.
    assert_eq!(
        resolution_code(&resolve_reference("PRD-404", &documents)),
        "not_found"
    );
    // The name is not a selector this resolver can honour.
    assert_eq!(
        resolution_code(&resolve_reference("prd-alpha@other-source", &documents)),
        "invalid_selector"
    );
    // Two documents answer to the alias.
    let contested_alias = vec![
        document("prd-alpha", "PRD-001", "Alpha", "prds/PRD-001-alpha.md"),
        document("prd-beta", "PRD-001", "Beta", "prds/PRD-001-beta.md"),
    ];
    assert_eq!(
        resolution_code(&resolve_reference("PRD-001", &contested_alias)),
        "ambiguous"
    );
    // Two documents claim one canonical key.
    let contested_key = vec![
        document("prd-alpha", "PRD-001", "Alpha", "prds/PRD-001-alpha.md"),
        document("prd-alpha", "PRD-002", "Alpha again", "prds/PRD-002-alpha.md"),
    ];
    assert_eq!(
        resolution_code(&resolve_reference("prd-alpha", &contested_key)),
        "conflicting_canonical_key"
    );
}

/// The candidates of an ambiguity are named by their canonical key, their
/// title and their workspace-relative path — never by an absolute path.
#[test]
fn the_candidates_are_named_by_key_title_and_relative_path() {
    let candidates = vec![
        document("prd-alpha", "PRD-001", "Alpha", "prds/PRD-001-alpha.md"),
        document("prd-beta", "PRD-001", "Beta", "prds/PRD-001-beta.md"),
    ];
    let described = describe_candidates(&candidates);

    assert_eq!(described.len(), 2, "every claimant must be named");
    assert!(described[0].contains("prd-alpha"), "{}", described[0]);
    assert!(described[0].contains("Alpha"), "{}", described[0]);
    assert!(
        described[0].contains("prds/PRD-001-alpha.md"),
        "the workspace-relative path must be part of the description: {}",
        described[0]
    );
    for line in &described {
        assert!(
            !line.starts_with('/'),
            "an absolute path must never reach the output: {line}"
        );
    }
}

/// Seed two documents that carry the SAME display id.
///
/// Their canonical keys differ, so neither is an identity collision: the
/// display id is the alias they share, and it is the alias that must not
/// pick a winner.
async fn seed_two_prds_sharing_one_display_id(store: Arc<LanceStore>, seed_guard: Arc<forgeplan_core::workspace::lock::WorkspaceLock>) {
    for (key, title, body) in [
        ("prd-alpha", "Alpha", "Alpha body."),
        ("prd-beta", "Beta", "Beta body."),
    ] {
        store
            .create_artifact_for_test(&seed_guard, &NewArtifact {
                canonical_key: key.to_string(),
                id: "PRD-001".to_string(),
                kind: "prd".to_string(),
                status: "draft".to_string(),
                title: title.to_string(),
                body: body.to_string(),
                depth: "standard".to_string(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .expect("seed artifact");
    }
}

async fn seed_normalized_phase_aliases(store: Arc<LanceStore>, guard: Arc<forgeplan_core::workspace::lock::WorkspaceLock>) {
    for (key, id) in [
        ("prd-alpha-k11111111111111111111111111111111", "PRD-001"),
        ("prd-beta-k22222222222222222222222222222222", "PRD-1"),
    ] {
        store.create_artifact_for_test(&guard, &NewArtifact {
            canonical_key: key.to_string(),
            id: id.to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: key.to_string(),
            body: "Phase alias regression.".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        }).await.unwrap();
    }
}

#[tokio::test]
async fn phase_by_key_refuses_normalized_ambiguous_legacy_state() {
    let fixture = McpFixture::new_with_seed(seed_normalized_phase_aliases).await;
    let state = fixture.workspace_path.join("state");
    tokio::fs::create_dir_all(&state).await.unwrap();
    let legacy = state.join("PRD-001.yaml");
    let bytes = b"artifact_id: PRD-001\ncurrent_phase: shape\n";
    tokio::fs::write(&legacy, bytes).await.unwrap();
    for key in [
        "prd-alpha-k11111111111111111111111111111111",
        "prd-beta-k22222222222222222222222222222222",
    ] {
        for (tool, args) in [
            ("forgeplan_phase", serde_json::json!({"id": key})),
            ("forgeplan_phase_advance", serde_json::json!({"id": key, "to": "code"})),
        ] {
            let response = fixture.call_tool_json(tool, args).await;
            assert!(response.is_error, "{tool}: {}", response.raw_text);
            let payload = response.json.as_ref().expect("structured resolver error");
            assert_eq!(payload["code"], "ambiguous", "{tool}: {}", response.raw_text);
            let candidates = payload["candidates"].as_array().unwrap();
            assert_eq!(candidates.len(), 2);
            for expected in [
                "prd-alpha-k11111111111111111111111111111111",
                "prd-beta-k22222222222222222222222222222222",
            ] {
                assert!(candidates.iter().any(|candidate| candidate["canonical_key"] == expected), "{tool}: {}", response.raw_text);
            }
        }
        assert!(!state.join(format!("{key}.yaml")).exists());
    }
    assert_eq!(tokio::fs::read(&legacy).await.unwrap(), bytes);
}

/// An ambiguous reference reaches the agent as a tool error that carries
/// the code, the input and the candidates.
///
/// The body of neither document may reach the agent, and no absolute path
/// may either.
#[tokio::test]
async fn an_ambiguous_reference_is_a_tool_error_that_names_every_candidate() {
    let fixture = McpFixture::new_with_seed(seed_two_prds_sharing_one_display_id).await;

    let envelope = fixture
        .call_tool_json("forgeplan_get", serde_json::json!({"id": "PRD-001"}))
        .await;

    assert!(
        envelope.is_error,
        "an ambiguous reference must be an error, not a document: {}",
        envelope.raw_text
    );
    // The code is the shared taxonomy's code for the same selector over the
    // same document set — not a second, private vocabulary.
    let expected_code = resolution_code(&resolve_reference(
        "PRD-001",
        &[
            document("prd-alpha", "PRD-001", "Alpha", "prds/PRD-001-alpha.md"),
            document("prd-beta", "PRD-001", "Beta", "prds/PRD-001-beta.md"),
        ],
    ));
    assert_eq!(expected_code, "ambiguous");
    assert!(
        envelope.raw_text.contains(expected_code),
        "the error must carry the machine-readable code ({expected_code}): {}",
        envelope.raw_text
    );
    assert!(
        envelope.raw_text.contains("PRD-001"),
        "the error must carry the input: {}",
        envelope.raw_text
    );
    for candidate in ["prd-alpha", "prd-beta"] {
        assert!(
            envelope.raw_text.contains(candidate),
            "the error must name every candidate ({candidate}): {}",
            envelope.raw_text
        );
    }
    assert!(
        !envelope.raw_text.contains("Alpha body.")
            && !envelope.raw_text.contains("Beta body."),
        "neither body may reach the agent: {}",
        envelope.raw_text
    );
    assert!(
        !envelope
            .raw_text
            .contains(fixture.workspace_path.to_string_lossy().as_ref()),
        "an absolute path must never reach the output: {}",
        envelope.raw_text
    );
}

/// The read-profile server paired with a `()`-handler client over the real
/// JSON-RPC transport.
struct ReadFixture {
    client: rmcp::service::RunningService<rmcp::RoleClient, ()>,
    _server_task: tokio::task::JoinHandle<()>,
}

impl ReadFixture {
    async fn new() -> Self {
        let server = forgeplan_mcp::constellation_read::ConstellationReadServer::new();
        let (server_io, client_io) = tokio::io::duplex(64 * 1024);
        let server_task = tokio::spawn(async move {
            if let Ok(running) = server.serve(server_io).await {
                let _ = running.waiting().await;
            }
        });
        let client = ().serve(client_io).await.expect("client handshake");
        Self {
            client,
            _server_task: server_task,
        }
    }

    async fn call(&self, name: &'static str, args: Value) -> CallToolEnvelope {
        let params = CallToolRequestParams::new(name).with_arguments(value_to_object(args));
        let result: CallToolResult = tokio::time::timeout(
            Duration::from_secs(15),
            self.client.peer().call_tool(params),
        )
        .await
        .unwrap_or_else(|_| panic!("tool `{name}` timed out (15s)"))
        .unwrap_or_else(|e| panic!("tool `{name}` rpc error: {e}"));
        CallToolEnvelope::from(result)
    }
}

/// A `.forgeplan` workspace holding one ADR, through the real init path.
fn seeded_workspace(root: &Path, dir: &str, body: &str) -> PathBuf {
    let fp = workspace::init_workspace(root, "qualified-get-test").expect("init workspace");
    std::fs::write(
        fp.join("adrs").join(format!("ADR-001-{dir}.md")),
        format!("---\nid: ADR-001\nkind: adr\nstatus: accepted\ntitle: Auth storage\n---\n\n{body}\n"),
    )
    .unwrap();
    fp
}

fn pin_home(home: &Path) {
    // SAFETY-free in practice: every test here is `#[serial(env_home)]`.
    unsafe {
        std::env::set_var("HOME", home);
        std::env::set_var("USERPROFILE", home);
        std::env::set_var("XDG_DATA_HOME", home);
    }
}

/// A qualified get names one source; an unqualified get over the same
/// scope says the name is ambiguous (SPEC-009 scenario V4).
///
/// The qualified read returns that source's exact body — and only that
/// source's body: no other source's data reaches the output.
#[tokio::test]
#[serial(env_home)]
async fn a_qualified_get_names_one_source_and_an_unqualified_get_reports_the_ambiguity() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws_alpha = tmp.path().join("ws-alpha");
    let ws_beta = tmp.path().join("ws-beta");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws_alpha).unwrap();
    std::fs::create_dir_all(&ws_beta).unwrap();
    pin_home(&home);

    let alpha = seeded_workspace(&ws_alpha, "alpha", "Alpha answer.");
    let beta = seeded_workspace(&ws_beta, "beta", "Beta answer.");

    let mut reg = registry::ConstellationRegistry::default();
    registry::create(&mut reg, "crew").unwrap();
    registry::add_source(&mut reg, "crew", &alpha, registry::SourceRole::Required).unwrap();
    registry::add_source(&mut reg, "crew", &beta, registry::SourceRole::Required).unwrap();
    registry::save(&registry::registry_path(&home), &reg).unwrap();

    let fixture = ReadFixture::new().await;
    let opened = fixture
        .call("constellation_open", serde_json::json!({}))
        .await;
    let opened = opened.assert_ok();
    let scope = opened["scope_handle"].as_str().unwrap().to_string();

    // Unqualified: both sources claim ADR-001 — the ambiguity is reported
    // explicitly, with every claimer named.
    let envelope = fixture
        .call(
            "constellation_get",
            serde_json::json!({"id": "ADR-001", "scope_handle": scope}),
        )
        .await;
    let payload = envelope.assert_ok();
    assert_eq!(
        payload["result"], "ambiguous",
        "an unqualified get over two claimants must report the ambiguity: {}",
        envelope.raw_text
    );
    assert_eq!(
        payload["claimers"].as_array().unwrap().len(),
        2,
        "every claimer must be named: {}",
        envelope.raw_text
    );

    // Qualified: the named source answers, with its exact body — and the
    // other source's body never appears.
    let envelope = fixture
        .call(
            "constellation_get",
            serde_json::json!({"id": "ADR-001@ws-alpha", "scope_handle": scope}),
        )
        .await;
    let payload = envelope.assert_ok();
    assert_eq!(
        payload["result"], "found",
        "a qualified get must answer: {}",
        envelope.raw_text
    );
    assert!(
        payload["body"].as_str().unwrap().contains("Alpha answer."),
        "the qualified get must return that source's exact body: {}",
        envelope.raw_text
    );
    assert!(
        !payload["body"].as_str().unwrap().contains("Beta answer."),
        "no other source's data may reach the output: {}",
        envelope.raw_text
    );
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn cached_v4_read_then_import_refreshes_same_server_after_migration() {
    let temp = tempfile::TempDir::new().unwrap();
    let workspace = forgeplan_core::db::migrate::populated_identity_v4_workspace_for_test(temp.path())
        .await.expect("create populated schema-4 workspace");
    let old = LanceStore::open(&workspace).await.expect("open v4 store");
    assert!(!old.uses_identity_v5(), "fixture must really be schema 4");
    assert!(forgeplan_core::db::migrate::read_identity_v5_marker(&workspace.join("lance")).unwrap().is_none());
    drop(old);
    let server = McpFixture::new_rooted(temp.path().to_path_buf()).await;
    let root = temp.path().to_str().unwrap();
    let first = server.call_tool_json("forgeplan_get", serde_json::json!({
        "id": "PRD-001", "workspace": root,
    })).await;
    assert_eq!(first.assert_ok()["canonical_key"], "prd-auth-system");
    let bundle = serde_json::json!({
        "artifacts": [{
            "id": "NOTE-002",
            "canonical_key": "note-cache-refresh",
            "kind": "note",
            "title": "Cache refresh",
            "body": "New note after v4 migration."
        }],
        "relations": []
    });
    let imported = server.call_tool_json("forgeplan_import", serde_json::json!({
        "data": bundle.to_string(), "force": false, "workspace": root,
    })).await;
    imported.assert_ok();
    assert!(forgeplan_core::db::migrate::read_identity_v5_marker(&workspace.join("lance")).unwrap().is_some());
    let original = server.call_tool_json("forgeplan_get", serde_json::json!({
        "id": "PRD-001", "workspace": root,
    })).await;
    assert_eq!(original.assert_ok()["canonical_key"], "prd-auth-system");
    let created = server.call_tool_json("forgeplan_get", serde_json::json!({
        "id": "NOTE-002", "workspace": root,
    })).await;
    assert_eq!(created.assert_ok()["canonical_key"], "note-cache-refresh");
}

#[tokio::test]
async fn validate_and_fpf_refuse_ambiguous_alias_and_validate_key() {
    let fixture = McpFixture::new_with_seed(seed_two_prds_sharing_one_display_id).await;
    for tool in ["forgeplan_validate", "forgeplan_fpf_check"] {
        let result = fixture.call_tool_json(tool, serde_json::json!({"id": "PRD-001"})).await;
        assert!(result.is_error, "{tool}: {}", result.raw_text);
        for expected in ["ambiguous", "prd-alpha", "prd-beta"] {
            assert!(result.raw_text.contains(expected), "{tool}: {}", result.raw_text);
        }
    }
    assert!(forgeplan_core::phase::store::read_phase(&fixture.workspace_path, "prd-alpha").await.unwrap().is_none());
    assert!(forgeplan_core::phase::store::read_phase(&fixture.workspace_path, "prd-beta").await.unwrap().is_none());
    let invalid = fixture.call_tool_json("forgeplan_fpf_check", serde_json::json!({"id": "prd-alpha@other"})).await;
    assert!(invalid.is_error && invalid.raw_text.contains("invalid_selector"), "{}", invalid.raw_text);
    let result = fixture.call_tool_json("forgeplan_validate", serde_json::json!({"id": "prd-alpha"})).await;
    let value = result.assert_ok();
    assert_eq!(value["total_artifacts"], 1);
    assert_eq!(value["results"][0]["id"], "PRD-001");
    assert_eq!(value["results"][0]["canonical_key"], "prd-alpha");
}

#[tokio::test]
async fn update_refuses_key_change_before_metadata_and_returns_both_identities() {
    let fixture = McpFixture::new_with_seed(seed_two_prds_sharing_one_display_id).await;
    let before = fixture.call_tool_json("forgeplan_get", serde_json::json!({"id": "prd-alpha"})).await;
    let before = before.assert_ok().clone();
    let rejected = fixture.call_tool_json("forgeplan_update", serde_json::json!({
        "id": "prd-alpha", "title": "Changed",
        "body": "---\nslug: prd-other\n---\n\nReplacement"
    })).await;
    assert!(rejected.is_error, "{}", rejected.raw_text);
    let after = fixture.call_tool_json("forgeplan_get", serde_json::json!({"id": "prd-alpha"})).await;
    let after = after.assert_ok();
    assert_eq!(after["title"], before["title"]);
    assert_eq!(after["body"], before["body"]);
    let updated = fixture.call_tool_json("forgeplan_update", serde_json::json!({
        "id": "prd-alpha", "title": "Changed"
    })).await;
    let updated = updated.assert_ok();
    assert_eq!(updated["id"], "PRD-001");
    assert_eq!(updated["canonical_key"], "prd-alpha");
}

#[tokio::test]
async fn get_surfaces_keyed_claim_and_phase_for_shared_alias() {
    let fixture = McpFixture::new_with_seed(seed_two_prds_sharing_one_display_id).await;
    let claims = forgeplan_core::claim::ClaimStore::new(&fixture.workspace_path);
    claims.claim_for("prd-alpha", "PRD-001", "agent/1", chrono::Duration::minutes(10), None)
        .await.expect("claim canonical document");
    forgeplan_core::phase::store::initialize_phase(&fixture.workspace_path, "prd-alpha", None)
        .await.expect("initialize canonical phase");
    let result = fixture.call_tool_json("forgeplan_get", serde_json::json!({"id": "prd-alpha"})).await;
    assert!(!result.is_error, "{}", result.raw_text);
    assert!(result.raw_text.contains("agent/1"), "{}", result.raw_text);
    assert!(result.raw_text.contains("Phase:"), "{}", result.raw_text);
}

#[tokio::test]
async fn search_rejects_bad_parameters_before_index_preparation() {
    let fixture = McpFixture::new_with_seed(seed_two_prds_sharing_one_display_id).await;
    let lance = fixture.workspace_path.join("lance");
    let before = forgeplan_core::db::migrate::read_identity_v5_marker(&lance).unwrap();
    for args in [
        serde_json::json!({"query": "alpha", "since": "not-a-date"}),
        serde_json::json!({"query": "alpha", "mode": "bogus"}),
    ] {
        let result = fixture.try_call_tool("forgeplan_search", args).await;
        assert!(result.is_err(), "invalid parameters must be rejected");
        assert_eq!(forgeplan_core::db::migrate::read_identity_v5_marker(&lance).unwrap(), before);
    }
}

#[tokio::test]
async fn semantic_mode_never_reports_lexical_results_as_semantic() {
    let fixture = McpFixture::new_with_seed(seed_two_prds_sharing_one_display_id).await;
    let result = fixture.call_tool_json("forgeplan_search", serde_json::json!({
        "query": "Alpha body", "mode": "semantic"
    })).await;
    assert!(result.is_error, "no configured backend must not yield lexical hits: {}", result.raw_text);
    assert!(result.raw_text.contains("Semantic search is unavailable"), "{}", result.raw_text);
}

#[cfg(all(feature = "semantic-search", debug_assertions))]
#[tokio::test]
async fn semantic_search_filters_before_limit_and_returns_more_than_fifty() {
    let fixture = McpFixture::new_with_seed(|store, guard| async move {
        for i in 0..71 {
            let low = i >= 65;
            let kind = if low { "note" } else { "prd" };
            let id = format!("{}-{:03}", kind.to_uppercase(), i + 1);
            store.create_artifact_for_test(&guard, &NewArtifact {
                canonical_key: format!("{kind}-semantic-{i:03}"),
                id,
                kind: kind.into(),
                status: "draft".into(),
                title: if low { format!("lowrank {i}") } else { format!("highrank {i}") },
                body: "semantic test body".into(),
                depth: if low { "deep" } else { "standard" }.into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            }).await.expect("seed semantic candidate");
        }
    }).await;
    let config = fixture.workspace_path.join("config.yaml");
    let original = std::fs::read_to_string(&config).unwrap();
    std::fs::write(&config, format!("{original}\nembedding:\n  model: forgeplan-test\n  chunk_size: 2000\n")).unwrap();
    let backend = forgeplan_core::db::store::install_semantic_backend_for_test(
        &fixture.workspace_path,
        Arc::new(|text| {
            let mut vector = vec![0.0; forgeplan_core::db::schema::EMBEDDING_DIM as usize];
            if text.contains("lowrank") {
                vector[0] = 0.8;
                vector[1] = 0.6;
            } else {
                vector[0] = 1.0;
            }
            Ok(vector)
        }),
    ).expect("install scoped profile-bound test backend");
    let filtered = fixture.call_tool_json("forgeplan_search", serde_json::json!({
        "query": "rank-query", "mode": "semantic", "kind": "note",
        "status": "draft", "depth": "deep", "since": "2020-01-01",
        "no_evidence": true, "limit": 3, "no_expand": true
    })).await;
    let value = filtered.assert_ok();
    assert_eq!(value["total"], 3);
    assert_eq!(value["coverage"]["lexical"], false);
    for hit in value["results"].as_array().unwrap() {
        assert_eq!(hit["kind"], "note");
        assert!(hit["canonical_key"].as_str().unwrap().starts_with("note-semantic-"));
    }
    let large = fixture.call_tool_json("forgeplan_search", serde_json::json!({
        "query": "rank-query", "mode": "semantic", "limit": 60, "no_expand": true
    })).await;
    assert_eq!(large.assert_ok()["total"], 60);
    drop(backend);
    let unavailable = fixture.try_call_tool("forgeplan_search", serde_json::json!({
        "query": "rank-query", "mode": "semantic", "limit": 60
    })).await;
    match unavailable {
        Err(_) => {}
        Ok(result) => assert!(result.is_error.unwrap_or(false), "missing backend must not return lexical hits"),
    }
}
