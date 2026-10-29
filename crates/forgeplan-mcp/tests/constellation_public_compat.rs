//! Public MCP compatibility on real legacy and canonical-file workspaces.
//! Provider-independent context preparation is tested in server.rs; these cases
//! drive the registered tools over the existing JSON-RPC transport.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade::{MaintainCapability, maintain_index};
use forgeplan_core::constellation::storage::CURRENT_FILE_NAME;
use forgeplan_core::db::auxiliary::AuxiliaryStore;
use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::projection::{self, MutationContext};
use forgeplan_core::workspace::{self, WorkspaceLock};
use serde_json::{Value, json};
use tempfile::TempDir;

mod common;
use common::McpFixture;

const NOTE_BODY: &str = "## Summary\n\nKeep credential rotation compatible with the recorded product requirement.\n\n## Related Artifacts\n\n| Artifact | Relation | Purpose |\n| --- | --- | --- |\n| PRD-002 | based_on | Product requirement |\n";
const PRD_BODY: &str = "## Problem Statement\n\nCredentials currently remain valid after rotation.\n\n## Goals\n\nInvalidate the previous credential only after the replacement is confirmed.\n\n## Functional Requirements\n\n- FR-001: Confirm replacement before retiring the previous credential.\n\n## Acceptance Criteria\n\n- A confirmed replacement invalidates the previous credential.\n";
const EVIDENCE_BODY: &str = "## Structured Fields\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n\n## Result\n\nThe real credential rotation checks passed against the product requirement.\n";

struct Corpus {
    _tempdir: TempDir,
    root: PathBuf,
    ws: PathBuf,
    legacy: Option<LanceStore>,
    paths: BTreeMap<String, PathBuf>,
}

fn artifact(id: &str, kind: &str, title: &str, body: &str) -> NewArtifact {
    NewArtifact {
        id: id.into(),
        kind: kind.into(),
        status: "draft".into(),
        title: title.into(),
        body: body.into(),
        depth: "standard".into(),
        author: None,
        parent_epic: None,
        valid_until: None,
        tags: Vec::new(),
    }
}

async fn add_expired_evidence(ctx: &MutationContext<'_>, id: &str) -> PathBuf {
    let mut evidence = artifact(id, "evidence", id, EVIDENCE_BODY);
    evidence.valid_until = Some("2020-01-01".into());
    let path = projection::create_artifact_with_projection(ctx, &evidence)
        .await
        .expect("create evidence in the canonical workspace");
    projection::add_link_with_projection(ctx, id, "PRD-002", "based_on")
        .await
        .expect("link evidence with the deliberately wrong relation");
    path
}

impl Corpus {
    async fn new(migrated: bool) -> Self {
        let tempdir = TempDir::new().expect("temporary workspace");
        let root = tempdir.path().to_path_buf();
        let ws = workspace::init_workspace(&root, "mcp-public-compat").expect("init workspace");
        let legacy = if migrated {
            None
        } else {
            Some(
                LanceStore::init(&ws)
                    .await
                    .expect("init the legacy projection"),
            )
        };
        let guard = workspace::acquire_workspace_lock(&ws)
            .await
            .expect("fixture lock");
        let ctx = MutationContext::new(&ws, &guard, legacy.as_ref());
        let mut paths = BTreeMap::new();
        for artifact in [
            artifact("NOTE-001", "note", "Rotation rationale", NOTE_BODY),
            artifact("PRD-002", "prd", "Rotation requirement", PRD_BODY),
        ] {
            let path = projection::create_artifact_with_projection(&ctx, &artifact)
                .await
                .expect("create the canonical artifact");
            paths.insert(artifact.id, path);
        }
        projection::add_link_with_projection(&ctx, "NOTE-001", "PRD-002", "based_on")
            .await
            .expect("create the relation described in the body table");
        paths.insert(
            "EVID-003".into(),
            add_expired_evidence(&ctx, "EVID-003").await,
        );
        drop(guard);
        if migrated {
            let capability = MaintainCapability::for_strict_maintenance(
                ws.canonicalize().expect("canonical workspace"),
            );
            maintain_index(
                &capability,
                CaptureLimits::default(),
                Instant::now() + Duration::from_secs(120),
            )
            .await
            .expect("publish the actual Constellation generation");
        }
        Self {
            _tempdir: tempdir,
            root,
            ws,
            legacy,
            paths,
        }
    }

    fn args(&self, mut args: Value) -> Value {
        args.as_object_mut()
            .expect("object args")
            .insert("workspace".into(), json!(self.root));
        args
    }

    fn context<'a>(&'a self, guard: &'a WorkspaceLock) -> MutationContext<'a> {
        MutationContext::new(&self.ws, guard, self.legacy.as_ref())
    }

    fn assert_retired_tables_stay_retired(&self) {
        if self.legacy.is_none() {
            for table in ["artifacts", "relations", "evidence", "change_log"] {
                assert!(
                    !self
                        .ws
                        .join("lance")
                        .join(format!("{table}.lance"))
                        .exists(),
                    "MCP must not recreate the retired {table} table"
                );
            }
        }
    }
}

fn anomaly<'a>(report: &'a Value, kind: &str, affected: &str) -> &'a Value {
    report["anomalies"]
        .as_array()
        .expect("anomalies array")
        .iter()
        .find(|entry| {
            entry["kind"] == kind
                && entry["affected"]
                    .as_array()
                    .expect("affected array")
                    .iter()
                    .any(|id| id == affected)
        })
        .unwrap_or_else(|| panic!("missing {kind} for {affected}: {report}"))
}

fn journal(ws: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(ws.join("anomalies-journal.jsonl"))
        .expect("the MCP scan persists its observations")
        .lines()
        .map(|line| {
            let entry: Value = serde_json::from_str(line).expect("JSONL observation");
            (
                entry["id"].as_str().expect("journal id").to_string(),
                entry["observed_at"]
                    .as_str()
                    .expect("journal timestamp")
                    .to_string(),
            )
        })
        .collect()
}

#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn validation_uses_real_outgoing_links_and_still_detects_a_missing_link() {
    for migrated in [false, true] {
        let corpus = Corpus::new(migrated).await;
        let fx = McpFixture::new_rooted(corpus.root.clone()).await;
        let valid = fx
            .call_tool_json("forgeplan_validate", corpus.args(json!({"id": "NOTE-001"})))
            .await;
        let valid = valid.assert_ok();
        assert_eq!(valid["total_artifacts"], 1);
        assert_eq!(valid["total_errors"], 0, "{valid}");
        assert_eq!(
            valid["total_warnings"], 0,
            "the real edge is not body-link drift: {valid}"
        );

        let removed = fx
            .call_tool_json(
                "forgeplan_unlink",
                corpus.args(json!({
                    "source": "NOTE-001", "target": "PRD-002", "relation": "based_on"
                })),
            )
            .await;
        removed.assert_ok();
        let drift = fx
            .call_tool_json("forgeplan_validate", corpus.args(json!({"id": "NOTE-001"})))
            .await;
        let drift = drift.assert_ok();
        let findings = drift["results"][0]["findings"]
            .as_array()
            .expect("findings");
        let finding = findings
            .iter()
            .find(|finding| finding["rule_id"] == "body-links-drift")
            .expect("removing the actual edge must reveal the still-present body mention");
        assert!(
            finding["message"]
                .as_str()
                .expect("message")
                .contains("PRD-002")
        );
        corpus.assert_retired_tables_stay_retired();
    }
}

#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn anomaly_filters_preserve_full_history_repeat_timestamps_since_and_gc() {
    for migrated in [false, true] {
        let corpus = Corpus::new(migrated).await;
        let fx = McpFixture::new_rooted(corpus.root.clone()).await;
        let filtered = fx
            .call_tool_json(
                "forgeplan_anomalies",
                corpus.args(json!({"kind": "mistyped_based_on", "severity": "medium"})),
            )
            .await;
        let filtered = filtered.assert_ok();
        assert_eq!(filtered["total"], 1, "{filtered}");
        let mistyped = anomaly(filtered, "mistyped_based_on", "EVID-003").clone();
        let initial_journal = journal(&corpus.ws);

        let full = fx
            .call_tool_json("forgeplan_anomalies", corpus.args(json!({})))
            .await;
        let full = full.assert_ok();
        let expired = anomaly(full, "expired_evidence", "EVID-003").clone();
        for entry in [
            &mistyped,
            &expired,
            anomaly(full, "mistyped_based_on", "EVID-003"),
        ] {
            assert_eq!(
                initial_journal
                    .get(entry["id"].as_str().expect("anomaly id"))
                    .map(String::as_str),
                entry["observed_at"].as_str(),
                "the first filtered call must persist even excluded observations: {entry}"
            );
        }
        assert_eq!(journal(&corpus.ws), initial_journal);

        let cutoff = initial_journal
            .values()
            .map(|stamp| {
                DateTime::parse_from_rfc3339(stamp)
                    .expect("RFC 3339")
                    .with_timezone(&Utc)
            })
            .max()
            .expect("nonempty journal")
            + chrono::Duration::nanoseconds(1);
        let since = fx
            .call_tool_json(
                "forgeplan_anomalies",
                corpus.args(json!({"since": cutoff.to_rfc3339()})),
            )
            .await;
        assert_eq!(
            since.assert_ok()["total"],
            0,
            "old observations are not new"
        );
        assert_eq!(
            journal(&corpus.ws),
            initial_journal,
            "since must not GC filtered-out entries"
        );
        drop(fx);

        let guard = workspace::acquire_workspace_lock(&corpus.ws)
            .await
            .expect("writer lock");
        add_expired_evidence(&corpus.context(&guard), "EVID-004").await;
        drop(guard);
        let fx = McpFixture::new_rooted(corpus.root.clone()).await;
        let added = fx
            .call_tool_json(
                "forgeplan_anomalies",
                corpus.args(json!({"kind": "expired_evidence", "since": cutoff.to_rfc3339()})),
            )
            .await;
        let added = added.assert_ok();
        assert_eq!(
            added["total"], 1,
            "only the newly observed expired pack is new: {added}"
        );
        anomaly(added, "expired_evidence", "EVID-004");
        drop(fx);

        let guard = workspace::acquire_workspace_lock(&corpus.ws)
            .await
            .expect("writer lock");
        projection::update_valid_until_with_projection(
            &corpus.context(&guard),
            "EVID-003",
            "2099-01-01",
        )
        .await
        .expect("resolve the first expiry");
        drop(guard);
        let fx = McpFixture::new_rooted(corpus.root.clone()).await;
        let narrowed = fx
            .call_tool_json(
                "forgeplan_anomalies",
                corpus.args(json!({"kind": "mistyped_based_on"})),
            )
            .await;
        assert_eq!(narrowed.assert_ok()["total"], 2);
        let after_gc = journal(&corpus.ws);
        assert!(!after_gc.contains_key(expired["id"].as_str().expect("resolved anomaly id")));
        assert_eq!(
            after_gc.get(mistyped["id"].as_str().expect("surviving anomaly id")),
            initial_journal.get(mistyped["id"].as_str().expect("surviving anomaly id")),
            "GC may remove resolved anomalies, not reset surviving observations"
        );
        corpus.assert_retired_tables_stay_retired();
    }
}

#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn anomaly_capture_waits_for_the_writer_and_reads_only_the_completed_edit() {
    let corpus = Corpus::new(true).await;
    let fx = McpFixture::new_rooted(corpus.root.clone()).await;
    let current_path = corpus.ws.join("lance").join(CURRENT_FILE_NAME);
    let published = std::fs::read(&current_path).expect("published descriptor");
    let evidence_path = corpus.paths.get("EVID-003").expect("evidence path");
    let before = std::fs::read_to_string(evidence_path).expect("canonical evidence");
    let guard = workspace::acquire_workspace_lock(&corpus.ws)
        .await
        .expect("writer lock");
    std::fs::write(evidence_path, "---\nid: [\n---\n")
        .expect("writer's incomplete intermediate bytes");
    let mut pending = Box::pin(fx.call_tool_json(
        "forgeplan_anomalies",
        corpus.args(json!({"kind": "expired_evidence"})),
    ));
    assert!(
        tokio::time::timeout(Duration::from_secs(1), pending.as_mut())
            .await
            .is_err(),
        "a scan may not capture the writer's incomplete file before acquiring its lock"
    );
    std::fs::write(evidence_path, before.replace("2020-01-01", "2099-01-01"))
        .expect("finish the existing evidence edit");
    add_expired_evidence(&corpus.context(&guard), "EVID-004").await;
    drop(guard);

    let response = pending.await;
    let report = response.assert_ok();
    assert_eq!(
        report["total"], 1,
        "capture must be fresh after the lock: {report}"
    );
    anomaly(report, "expired_evidence", "EVID-004");
    assert_eq!(
        std::fs::read(&current_path).expect("descriptor after scan"),
        published
    );
    let persisted = journal(&corpus.ws);
    assert!(persisted.contains_key(report["anomalies"][0]["id"].as_str().expect("anomaly id")));
    corpus.assert_retired_tables_stay_retired();
}

#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn an_anomaly_journal_write_failure_keeps_the_existing_best_effort_report() {
    for migrated in [false, true] {
        let corpus = Corpus::new(migrated).await;
        std::fs::create_dir(corpus.ws.join("anomalies-journal.jsonl"))
            .expect("make the journal destination unwritable as a file");
        let fx = McpFixture::new_rooted(corpus.root.clone()).await;
        let response = fx
            .call_tool_json("forgeplan_anomalies", corpus.args(json!({})))
            .await;
        let report = response.assert_ok();
        anomaly(report, "expired_evidence", "EVID-003");
        anomaly(report, "mistyped_based_on", "EVID-003");
        assert!(
            corpus.ws.join("anomalies-journal.jsonl").is_dir(),
            "a failed journal write must not overwrite its destination"
        );
        corpus.assert_retired_tables_stay_retired();
    }
}

#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn corrupt_managed_reads_do_not_become_empty_legacy_or_provider_successes() {
    let corpus = Corpus::new(true).await;
    std::fs::write(corpus.ws.join("lance").join(CURRENT_FILE_NAME), b"{broken")
        .expect("corrupt the actual managed descriptor");
    let fx = McpFixture::new_rooted(corpus.root.clone()).await;
    for (tool, args) in [
        ("forgeplan_validate", json!({"id": "NOTE-001"})),
        ("forgeplan_score", json!({"id": "NOTE-001"})),
        ("forgeplan_anomalies", json!({})),
        ("forgeplan_reason", json!({"id": "NOTE-001"})),
        ("forgeplan_export", json!({})),
    ] {
        match fx.try_call_tool(tool, corpus.args(args)).await {
            Ok(result) => {
                let response = common::CallToolEnvelope::from(result);
                assert!(
                    response.is_error,
                    "{tool} must not report success: {response:?}"
                );
                assert!(
                    response.raw_text.contains("Constellation read view"),
                    "{tool}: {response:?}"
                );
            }
            Err(error) => assert!(
                error.to_string().contains("Constellation read view"),
                "expected the managed read error, not a provider/transport failure: {tool}: {error}"
            ),
        }
    }
    assert!(!corpus.ws.join("anomalies-journal.jsonl").exists());
    corpus.assert_retired_tables_stay_retired();
}

#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn corrupt_auxiliary_kb_is_not_an_empty_catalog_or_a_rule_based_route() {
    for migrated in [false, true] {
        let corpus = Corpus::new(migrated).await;
        let auxiliary = AuxiliaryStore::open_or_create(&corpus.ws)
            .await
            .expect("create actual KB table");
        drop(auxiliary);
        let versions = corpus.ws.join("lance/fpf_spec.lance/_versions");
        std::fs::remove_dir_all(&versions).expect("remove the KB version manifests");
        std::fs::write(&versions, b"corrupt table metadata")
            .expect("break the existing table metadata");
        let mut config = workspace::load_config(&corpus.ws).expect("fixture config");
        config.llm = Some(Default::default());
        std::fs::write(
            corpus.ws.join("config.yaml"),
            serde_yaml::to_string(&config).expect("config YAML"),
        )
        .expect("enable the real provider path; KB must fail before a provider is called");
        let fx = McpFixture::new_rooted(corpus.root.clone()).await;
        let catalog = fx.call_tool_json("forgeplan_fpf_list", json!({})).await;
        assert!(
            catalog.is_error,
            "a corrupt KB is not an empty catalog: {catalog:?}"
        );
        assert!(
            !catalog.raw_text.contains("not loaded"),
            "corruption must not be reported as absence"
        );
        let route = fx
            .try_call_tool(
                "forgeplan_route",
                corpus.args(json!({"description": "Rotation invariants"})),
            )
            .await;
        match route {
            Ok(result) => {
                let response = common::CallToolEnvelope::from(result);
                assert!(
                    response.is_error,
                    "corrupt KB cannot yield a route: {response:?}"
                );
                assert!(response.raw_text.contains("fpf_spec"), "{response:?}");
            }
            Err(error) => assert!(
                error.to_string().contains("fpf_spec"),
                "expected the actual KB failure before any provider call: {error}"
            ),
        }
        corpus.assert_retired_tables_stay_retired();
    }
}

// The v0.38 full-profile catalog, not an inferred subset of today's router.
const OLD_TOOLS: &[&str] = &[
    "forgeplan_activate",
    "forgeplan_activity",
    "forgeplan_activity_stats",
    "forgeplan_anomalies",
    "forgeplan_blindspots",
    "forgeplan_blocked",
    "forgeplan_calibrate",
    "forgeplan_capture",
    "forgeplan_claim",
    "forgeplan_claims",
    "forgeplan_contradictions",
    "forgeplan_coverage",
    "forgeplan_coverage_business",
    "forgeplan_decay",
    "forgeplan_decompose",
    "forgeplan_delete",
    "forgeplan_deprecate",
    "forgeplan_discover_complete",
    "forgeplan_discover_finding",
    "forgeplan_discover_start",
    "forgeplan_dispatch",
    "forgeplan_drift",
    "forgeplan_estimate",
    "forgeplan_export",
    "forgeplan_fpf_check",
    "forgeplan_fpf_list",
    "forgeplan_fpf_rules",
    "forgeplan_fpf_search",
    "forgeplan_fpf_section",
    "forgeplan_generate",
    "forgeplan_get",
    "forgeplan_graph",
    "forgeplan_guard",
    "forgeplan_health",
    "forgeplan_hypothesis_promote",
    "forgeplan_hypothesis_status",
    "forgeplan_import",
    "forgeplan_ingest",
    "forgeplan_init",
    "forgeplan_interview_packet_draft",
    "forgeplan_interview_packet_ingest",
    "forgeplan_journal",
    "forgeplan_link",
    "forgeplan_list",
    "forgeplan_new",
    "forgeplan_order",
    "forgeplan_orphans",
    "forgeplan_phase",
    "forgeplan_phase_advance",
    "forgeplan_playbook_list",
    "forgeplan_playbook_run",
    "forgeplan_playbook_show",
    "forgeplan_playbook_validate",
    "forgeplan_plugins_doctor",
    "forgeplan_plugins_info",
    "forgeplan_plugins_list",
    "forgeplan_progress",
    "forgeplan_reason",
    "forgeplan_release",
    "forgeplan_release_notes",
    "forgeplan_restore",
    "forgeplan_review",
    "forgeplan_route",
    "forgeplan_score",
    "forgeplan_search",
    "forgeplan_session",
    "forgeplan_stale",
    "forgeplan_status",
    "forgeplan_supersede",
    "forgeplan_undo_last",
    "forgeplan_unlink",
    "forgeplan_update",
    "forgeplan_validate",
];

#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn the_full_profile_keeps_every_old_tool_and_the_existing_parameter_shapes() {
    for migrated in [false, true] {
        let corpus = Corpus::new(migrated).await;
        let fx = McpFixture::new_rooted(corpus.root.clone()).await;
        let tools = fx.peer_list_all_tools().await.expect("real tools/list");
        let names: BTreeSet<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
        assert_eq!(OLD_TOOLS.len(), 73);
        for name in OLD_TOOLS {
            assert!(
                names.contains(name),
                "the full profile lost the old {name} surface"
            );
        }
        for name in [
            "forgeplan_constellation_get",
            "forgeplan_constellation_search",
            "forgeplan_constellation_status",
        ] {
            assert!(
                names.contains(name),
                "the native surface must remain registered"
            );
        }
        for (name, fields, required) in [
            ("forgeplan_validate", vec!["id", "workspace"], vec![]),
            (
                "forgeplan_anomalies",
                vec!["kind", "severity", "since", "workspace"],
                vec![],
            ),
            (
                "forgeplan_route",
                vec!["description", "workspace"],
                vec!["description"],
            ),
            ("forgeplan_reason", vec!["id", "workspace"], vec!["id"]),
        ] {
            let tool = tools
                .iter()
                .find(|tool| tool.name == name)
                .expect("registered tool");
            let serialized = serde_json::to_value(tool).expect("tool schema JSON");
            let schema = &serialized["inputSchema"];
            let actual_fields: Vec<&str> = schema["properties"]
                .as_object()
                .expect("native object input schema")
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(actual_fields, fields, "{name} must not rename or add flags");
            let actual_required: Vec<&str> = schema["required"]
                .as_array()
                .map(|values| {
                    values
                        .iter()
                        .map(|v| v.as_str().expect("required field"))
                        .collect()
                })
                .unwrap_or_default();
            assert_eq!(
                actual_required, required,
                "{name} must keep its optional defaults"
            );
        }
        corpus.assert_retired_tables_stay_retired();
    }
}
