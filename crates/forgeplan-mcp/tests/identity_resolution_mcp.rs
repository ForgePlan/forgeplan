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
async fn seed_two_prds_sharing_one_display_id(store: Arc<LanceStore>) {
    for (key, title, body) in [
        ("prd-alpha", "Alpha", "Alpha body."),
        ("prd-beta", "Beta", "Beta body."),
    ] {
        store
            .create_artifact_for_test(&NewArtifact {
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
