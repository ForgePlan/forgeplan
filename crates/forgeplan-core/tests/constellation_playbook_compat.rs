//! Native playbook operations on both index owners, without provider calls.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use forgeplan_core::artifact::frontmatter::parse_frontmatter;
use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade::{
    MaintainCapability, maintain_index, open_workspace_read, workspace_is_migrated,
};
use forgeplan_core::constellation::manifest::LEGACY_TABLES;
use forgeplan_core::db::store::{ArtifactRecord, LanceStore, NewArtifact};
use forgeplan_core::playbook::dispatch::forgeplan_core_dispatcher::ForgeplanCoreDispatcher;
use forgeplan_core::playbook::dispatch::{
    DispatchError, DispatchOutcome, Dispatcher, RoutingDispatcher,
};
use forgeplan_core::playbook::executor::{Executor, ExecutorConfig, StepStatus};
use forgeplan_core::playbook::journal::Journal;
use forgeplan_core::playbook::loader::load_playbook;
use forgeplan_core::playbook::types::{Delegation, ForgeplanOp, OnError, Step};
use forgeplan_core::projection::{self, MutationContext};
use forgeplan_core::workspace::{init_workspace, lock::acquire_workspace_lock};
use serde_json::{Value, json};
use tempfile::TempDir;

const NOTE_BODY: &str = "## Context\n\nThe needleq72 marker belongs to the original note.\n";
const ADR_BODY: &str = "## Context\n\nNative playbooks must use the workspace's owning read source so an index migration cannot change the activation decision.\n\n## Decision\n\nKeep the canonical Markdown and its evidence edge together, and run the existing lifecycle rules on that complete state.\n\n## Consequences\n\nA transition must update the real file while preserving its body and outgoing links. The derived index can then publish the new status.\n";

struct Fixture {
    project: TempDir,
    ws: PathBuf,
    migrated: bool,
}

impl Fixture {
    async fn new(migrated: bool) -> Self {
        let project = TempDir::new().expect("temporary project");
        let ws = init_workspace(project.path(), "native-playbook-compat").expect("workspace");
        let lock = acquire_workspace_lock(&ws).await.expect("fixture lock");
        let store = LanceStore::init_new(&ws, &lock).await.expect("legacy fixture tables");
        let ctx = MutationContext::new(&ws, &lock, Some(&store));
        seed(&ctx, "NOTE-007", "note", "Original note", NOTE_BODY, &[]).await;
        seed(&ctx, "ADR-003", "adr", "Native decision", ADR_BODY, &[]).await;
        seed(
            &ctx,
            "PRD-004",
            "prd",
            "Incomplete proposal",
            "Incomplete.\n",
            &[],
        )
        .await;
        seed(
            &ctx,
            "EVID-001",
            "evidence",
            "Native verification",
            "## Verdict\n\nThe native operations must preserve their canonical bytes.\n",
            &[("ADR-003", "informs")],
        )
        .await;
        drop(store);
        drop(lock);
        if migrated {
            publish(&ws).await;
        }
        let fixture = Self {
            project,
            ws,
            migrated,
        };
        fixture.assert_owner();
        fixture
    }

    fn router(&self) -> RoutingDispatcher {
        RoutingDispatcher::new(self.project.path().to_path_buf())
    }

    fn assert_owner(&self) {
        assert_eq!(workspace_is_migrated(&self.ws), self.migrated);
        if self.migrated {
            for table in LEGACY_TABLES {
                assert!(
                    !self
                        .ws
                        .join("lance")
                        .join(format!("{table}.lance"))
                        .exists(),
                    "native operation resurrected {table}"
                );
            }
        }
        assert!(!self.project.path().join("lance").exists());
        assert!(!self.project.path().join("notes").exists());
        assert!(!self.project.path().join(".lock").exists());
    }

    async fn record(&self, id: &str) -> ArtifactRecord {
        if self.migrated {
            open_workspace_read(&self.ws, None, fixture_capture_limits())
                .await
                .expect("canonical read")
                .get_record(id)
                .expect("captured artifact")
                .clone()
        } else {
            let permit = forgeplan_core::workspace::lock::acquire_workspace_read_lock(&self.ws)
                .await
                .expect("legacy read permit");
            LanceStore::open(&self.ws)
                .await
                .expect("legacy read")
                .get_record(&permit, id)
                .await
                .expect("get record")
                .expect("legacy artifact")
        }
    }
}

fn fixture_capture_limits() -> CaptureLimits {
    // Native-operation assertions are not a latency test of the ambient model.
    // Read-back uses the same bounded setup envelope as fixture publication.
    CaptureLimits {
        budget: Duration::from_secs(120),
        ..CaptureLimits::default()
    }
}

async fn publish(ws: &Path) {
    maintain_index(
        &MaintainCapability::for_strict_maintenance(
            ws.canonicalize().expect("canonical workspace"),
        ),
        fixture_capture_limits(),
        Instant::now() + Duration::from_secs(120),
    )
    .await
    .expect("publish real fixture");
}

async fn seed(
    ctx: &MutationContext<'_>,
    id: &str,
    kind: &str,
    title: &str,
    body: &str,
    links: &[(&str, &str)],
) {
    let depth = if kind == "note" || kind == "evidence" {
        "tactical"
    } else {
        "standard"
    };
    projection::create_artifact_with_projection(
        ctx,
        &NewArtifact {
            canonical_key: id.to_string(),
            id: id.to_string(),
            kind: kind.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            status: "draft".to_string(),
            depth: depth.to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        },
    )
    .await
    .expect("fixture row");
    for (target, relation) in links {
        projection::add_link_with_projection(ctx, id, target, relation)
            .await
            .expect("fixture relation");
    }
}

fn step(op: ForgeplanOp, input: Value) -> Step {
    Step {
        id: "native".to_string(),
        delegate_to: Delegation::ForgeplanCore { target: op },
        input: Some(serde_yaml::to_value(input).expect("YAML input")),
        produces_at: None,
        mapping: None,
        requires: None,
        fallback_hint: None,
        on_error: OnError::Abort,
        timeout_seconds: Some(17),
        budget_usd: None,
        allowed_tools: None,
    }
}

async fn dispatch(fixture: &Fixture, op: ForgeplanOp, input: Value) -> DispatchOutcome {
    fixture
        .router()
        .dispatch(&step(op, input))
        .await
        .expect("native dispatch")
}

fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut result = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(dir).expect("directory") {
            let entry = entry.expect("entry");
            let path = entry.path();
            if entry.file_type().expect("type").is_dir() {
                pending.push(path);
            } else {
                result.insert(
                    path.strip_prefix(root).expect("relative").to_path_buf(),
                    std::fs::read(path).expect("bytes"),
                );
            }
        }
    }
    result
}

fn ingest_input(project: &Path, dry_run: bool, update: bool) -> Value {
    json!({
        "mapping_path": project.join("mapping.yaml"),
        "source_path": project.join("source.md"),
        "dry_run": dry_run,
        "update": update,
    })
}

fn write_ingest_sources(project: &Path) {
    std::fs::write(
        project.join("source.md"),
        "# Imported\n\nImportedneedle first revision.\n",
    )
    .expect("source");
    std::fs::write(
        project.join("mapping.yaml"),
        r#"
schema_version: "1.0"
name: native-import
title: Native import
compat_spec_version: "c4-architecture: ^1.0"
source_kind: c4-documentation
target_kind: forge
sources:
  - pattern: "**/*.md"
    type: markdown
    parser: markdown_only
rules:
  - id: native-note
    when:
      file_glob: "**/*.md"
      heading_path: ["Imported"]
    target: { kind: note }
    fields:
      title: "Imported native note"
      summary: "{{ section.body }}"
    sources_section:
      include: true
      format: "{path}:{line_start}-{line_end}"
      precision: line
      source_hash: true
    links:
      - target_artifact_id: "ADR-003"
        relation: based_on
        if_exists: skip
"#,
    )
    .expect("mapping");
}

#[tokio::test]
async fn native_new_writes_the_correct_workspace_and_keeps_the_template_body() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated).await;
        let outcome = dispatch(
            &fixture,
            ForgeplanOp::New,
            json!({"kind": "note", "title": "Native generated note"}),
        )
        .await;
        assert!(outcome.success, "{outcome:?}");
        assert_eq!(outcome.stderr, None);
        let path = outcome.output_path.expect("created projection");
        assert!(path.starts_with(fixture.ws.join("notes")));
        let raw = std::fs::read_to_string(&path).expect("created bytes");
        let (fm, body) = parse_frontmatter(&raw).expect("created frontmatter");
        assert_eq!(fm["id"].as_str(), Some("NOTE-008"));
        assert_eq!(fm["title"].as_str(), Some("Native generated note"));
        assert_eq!(fm["status"].as_str(), Some("draft"));
        assert_eq!(fm["depth"].as_str(), Some("tactical"));
        assert!(!body.trim().is_empty());
        let record = fixture.record("NOTE-008").await;
        assert_eq!(record.title, "Native generated note");
        if migrated {
            assert_eq!(record.body, raw);
            let read = open_workspace_read(&fixture.ws, None, CaptureLimits::default())
                .await
                .expect("read after new");
            assert!(
                read.is_persisted(),
                "reconcile must run after releasing the write lock"
            );
        } else {
            assert!(body.contains(record.body.trim()));
        }
        fixture.assert_owner();
    }
}

#[tokio::test]
async fn native_validation_and_activation_keep_rules_edges_and_real_status() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated).await;
        let valid = dispatch(&fixture, ForgeplanOp::Validate, json!({"id": "ADR-003"})).await;
        assert!(valid.success, "{valid:?}");
        let invalid = dispatch(&fixture, ForgeplanOp::Validate, json!({"id": "PRD-004"})).await;
        assert!(!invalid.success);
        assert!(invalid.stderr.expect("findings").contains("MUST error(s)"));
        let before = files(&fixture.ws.join("prds"));
        let blocked = dispatch(&fixture, ForgeplanOp::Activate, json!({"id": "PRD-004"})).await;
        assert!(!blocked.success);
        let message = blocked.stderr.expect("gate failure");
        assert!(message.contains("No evidence linked"), "{message}");
        assert!(message.contains("--force"), "{message}");
        assert_eq!(files(&fixture.ws.join("prds")), before);
        assert_eq!(fixture.record("PRD-004").await.status, "draft");

        let activated = dispatch(&fixture, ForgeplanOp::Activate, json!({"id": "ADR-003"})).await;
        assert!(activated.success, "{activated:?}");
        assert_eq!(activated.stderr, None);
        assert_eq!(fixture.record("ADR-003").await.status, "active");
        let raw = std::fs::read_to_string(fixture.ws.join("adrs/ADR-003-native-decision.md"))
            .expect("status bytes");
        let (fm, body) = parse_frontmatter(&raw).expect("status frontmatter");
        assert_eq!(fm["status"].as_str(), Some("active"));
        assert_eq!(body.trim(), ADR_BODY.trim());
        let evidence =
            std::fs::read_to_string(fixture.ws.join("evidence/EVID-001-native-verification.md"))
                .expect("evidence bytes");
        assert!(evidence.contains("target: ADR-003"));
        assert!(evidence.contains("relation: informs"));

        let again = dispatch(&fixture, ForgeplanOp::Activate, json!({"id": "ADR-003"})).await;
        assert!(!again.success, "active -> active must still be rejected");
        let forced = dispatch(
            &fixture,
            ForgeplanOp::Activate,
            json!({"id": "PRD-004", "force": true}),
        )
        .await;
        assert!(forced.success, "{forced:?}");
        assert!(
            forced
                .stderr
                .expect("forced findings")
                .contains("forced MUST error(s)")
        );
        assert_eq!(fixture.record("PRD-004").await.status, "active");
        for op in [ForgeplanOp::Validate, ForgeplanOp::Activate] {
            let missing = dispatch(&fixture, op, json!({"id": "NOTE-999"})).await;
            assert!(!missing.success);
            assert!(missing.stderr.expect("not found").contains("not found"));
        }
        fixture.assert_owner();
    }
}

#[tokio::test]
async fn native_search_returns_real_consumer_results_and_kind_filters() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated).await;
        let pointer = migrated
            .then(|| std::fs::read(fixture.ws.join("lance/CURRENT.json")).expect("pointer"));
        let outcome = dispatch(
            &fixture,
            ForgeplanOp::Search,
            json!({"query": "NEEDLEQ72", "kind": "note"}),
        )
        .await;
        assert!(outcome.success);
        assert_eq!(outcome.output_path, None);
        let payload: Value =
            serde_json::from_str(&outcome.stderr.expect("operation JSON")).expect("JSON");
        assert_eq!(
            payload,
            json!({"results": [{"id": "NOTE-007", "kind": "note", "status": "draft", "title": "Original note"}]})
        );
        for input in [
            json!({"query": "needleq72", "kind": "adr"}),
            json!({"query": "absent-needle"}),
        ] {
            let empty = dispatch(&fixture, ForgeplanOp::Search, input).await;
            let payload: Value = serde_json::from_str(&empty.stderr.expect("JSON")).expect("JSON");
            assert_eq!(payload, json!({"results": []}));
        }
        if let Some(pointer) = pointer {
            assert_eq!(
                std::fs::read(fixture.ws.join("lance/CURRENT.json")).expect("unchanged pointer"),
                pointer
            );
        }
        fixture.assert_owner();
    }
}

#[tokio::test]
async fn native_ingest_dry_run_writes_nothing_and_apply_keeps_body_links_and_idempotency() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated).await;
        write_ingest_sources(fixture.project.path());
        let before = files(fixture.project.path());
        let dry = dispatch(
            &fixture,
            ForgeplanOp::Ingest,
            ingest_input(fixture.project.path(), true, false),
        )
        .await;
        assert!(dry.success, "{dry:?}");
        assert_eq!(dry.output_path, None);
        assert_eq!(dry.stderr.as_deref(), Some("dry-run: 1 draft(s) planned"));
        assert_eq!(files(fixture.project.path()), before);

        let applied = dispatch(
            &fixture,
            ForgeplanOp::Ingest,
            ingest_input(fixture.project.path(), false, false),
        )
        .await;
        assert!(applied.success, "{applied:?}");
        let path = applied.output_path.expect("imported projection");
        assert!(path.starts_with(fixture.ws.join("notes")));
        let first = std::fs::read_to_string(&path).expect("imported bytes");
        let (fm, body) = parse_frontmatter(&first).expect("imported frontmatter");
        assert_eq!(fm["id"].as_str(), Some("NOTE-008"));
        assert!(body.contains("Importedneedle first revision."));
        assert!(body.contains("## Sources"));
        assert!(body.contains("source_hash:"));
        assert!(first.contains("source=ingest:native-note"));
        assert!(first.contains("target: ADR-003"));
        assert!(first.contains("relation: based_on"));
        let read = open_workspace_read(&fixture.ws, None, fixture_capture_limits())
            .await
            .expect("read imported state");
        assert!(read.get_all_relations().contains(&(
            "NOTE-008".to_string(),
            "ADR-003".to_string(),
            "based_on".to_string()
        )));
        drop(read);
        let repeated = dispatch(
            &fixture,
            ForgeplanOp::Ingest,
            ingest_input(fixture.project.path(), false, false),
        )
        .await;
        assert!(repeated.success, "{repeated:?}");
        assert_eq!(repeated.output_path, None);
        assert_eq!(
            std::fs::read_to_string(&path).expect("unchanged import"),
            first
        );

        std::fs::write(
            fixture.project.path().join("source.md"),
            "# Imported\n\nChangedneedle second revision.\n",
        )
        .expect("changed source");
        let no_update = dispatch(
            &fixture,
            ForgeplanOp::Ingest,
            ingest_input(fixture.project.path(), false, false),
        )
        .await;
        assert!(no_update.success);
        assert_eq!(no_update.output_path, None);
        assert_eq!(
            std::fs::read_to_string(&path).expect("update opt-in"),
            first
        );
        let updated = dispatch(
            &fixture,
            ForgeplanOp::Ingest,
            ingest_input(fixture.project.path(), false, true),
        )
        .await;
        assert!(updated.success, "{updated:?}");
        assert_eq!(updated.output_path.as_ref(), Some(&path));
        let changed = std::fs::read_to_string(&path).expect("changed import");
        assert!(changed.contains("Changedneedle second revision."));
        assert!(!changed.contains("Importedneedle first revision."));
        assert_eq!(
            parse_frontmatter(&changed).expect("updated frontmatter").0["id"].as_str(),
            Some("NOTE-008")
        );
        assert!(changed.contains("target: ADR-003"));
        assert!(
            fixture
                .record("NOTE-008")
                .await
                .body
                .contains("Changedneedle")
        );
        fixture.assert_owner();
    }
}

#[tokio::test]
async fn invalid_native_inputs_keep_failure_classes_without_writing() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated).await;
        let dispatcher = ForgeplanCoreDispatcher::new(fixture.ws.clone());
        let before = files(fixture.project.path());
        for input in [json!({"title": "Missing kind"}), json!("not a mapping")] {
            assert!(matches!(
                dispatcher.dispatch(&step(ForgeplanOp::New, input)).await,
                Err(DispatchError::Transport(_))
            ));
        }
        for input in [
            json!({"kind": "note", "title": "  "}),
            json!({"kind": "unknown", "title": "Rejected"}),
        ] {
            let result = dispatcher
                .dispatch(&step(ForgeplanOp::New, input))
                .await
                .expect("operation failure");
            assert!(!result.success);
        }
        let missing = dispatch(&fixture, ForgeplanOp::Ingest, json!({"mapping_path": fixture.project.path().join("missing.yaml"), "source_path": fixture.project.path().join("missing.md")})).await;
        assert!(!missing.success);
        assert!(
            missing
                .stderr
                .expect("mapping error")
                .contains("mapping file not found")
        );
        assert_eq!(files(fixture.project.path()), before);
        fixture.assert_owner();
    }
}

#[tokio::test]
async fn migrated_native_reads_observe_offline_edits_without_publishing() {
    let fixture = Fixture::new(true).await;
    let path = fixture.ws.join("notes/NOTE-007-original-note.md");
    let raw = std::fs::read_to_string(&path).expect("note");
    std::fs::write(&path, raw.replace("needleq72", "offlineq73")).expect("offline edit");
    let pointer = std::fs::read(fixture.ws.join("lance/CURRENT.json")).expect("pointer");
    let fresh = dispatch(
        &fixture,
        ForgeplanOp::Search,
        json!({"query": "offlineq73"}),
    )
    .await;
    let payload: Value = serde_json::from_str(&fresh.stderr.expect("search JSON")).expect("JSON");
    assert_eq!(payload["results"][0]["id"], "NOTE-007");
    let old = dispatch(&fixture, ForgeplanOp::Search, json!({"query": "needleq72"})).await;
    assert_eq!(
        serde_json::from_str::<Value>(&old.stderr.expect("JSON")).expect("JSON"),
        json!({"results": []})
    );
    assert!(
        dispatch(&fixture, ForgeplanOp::Validate, json!({"id": "note-7"}))
            .await
            .success
    );
    assert_eq!(
        std::fs::read(fixture.ws.join("lance/CURRENT.json")).expect("pointer after reads"),
        pointer
    );
    fixture.assert_owner();
}

#[tokio::test]
async fn incomplete_canonical_capture_refuses_every_native_operation_without_legacy_fallback() {
    let fixture = Fixture::new(true).await;
    write_ingest_sources(fixture.project.path());
    std::fs::write(
        fixture.ws.join("notes/NOTE-007-original-note.md"),
        "---\nid: [unterminated\n---\nInvalid source.\n",
    )
    .expect("damaged source");
    let before = files(fixture.project.path());
    for (op, input) in [
        (
            ForgeplanOp::New,
            json!({"kind": "note", "title": "Must not be created"}),
        ),
        (ForgeplanOp::Validate, json!({"id": "ADR-003"})),
        (ForgeplanOp::Activate, json!({"id": "ADR-003"})),
        (ForgeplanOp::Search, json!({"query": "needle"})),
        (
            ForgeplanOp::Ingest,
            ingest_input(fixture.project.path(), false, false),
        ),
    ] {
        let result = fixture.router().dispatch(&step(op, input)).await;
        assert!(
            matches!(result, Err(DispatchError::Transport(_))),
            "{op:?}: {result:?}"
        );
    }
    assert_eq!(files(fixture.project.path()), before);
    fixture.assert_owner();
}

#[tokio::test]
async fn command_consent_refusal_prevents_dependent_native_writes() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated).await;
        let playbook = load_playbook(
            r#"
schema_version: "1.0"
name: consent-native
title: Consent native
steps:
  - id: shell
    delegate_to:
      type: command
      command: ["must-not-run-native-compat"]
  - id: native
    requires: [shell]
    delegate_to:
      type: forgeplan_core
      target: new
    input:
      kind: note
      title: Must not be created
"#,
        )
        .expect("playbook");
        let journal_dir = TempDir::new().expect("separate journal");
        let journal = Journal::open_at(journal_dir.path().join("runs.jsonl")).expect("journal");
        let before = files(fixture.project.path());
        let mut executor = Executor::new(
            fixture.router(),
            journal,
            ExecutorConfig {
                yes_flag: true,
                allow_shell: false,
                skip_revalidation: false,
                start_step: None,
            },
        );
        let report = executor
            .run(&playbook)
            .await
            .expect("refused execution report");
        assert_eq!(report.failed, 1);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.success, 0);
        assert_eq!(report.per_step[0].status, StepStatus::Failed);
        assert!(
            report.per_step[0]
                .message
                .as_deref()
                .expect("refusal")
                .contains("--allow-shell")
        );
        assert_eq!(report.per_step[1].status, StepStatus::Skipped);
        assert_eq!(files(fixture.project.path()), before);
        fixture.assert_owner();
    }
}

#[tokio::test]
async fn native_new_waits_for_the_workspace_lock_then_releases_it_before_reconcile() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated).await;
        let lock = acquire_workspace_lock(&fixture.ws)
            .await
            .expect("held lock");
        let before = files(fixture.project.path());
        let router = fixture.router();
        let mut pending = tokio::spawn(async move {
            router
                .dispatch(&step(
                    ForgeplanOp::New,
                    json!({"kind": "note", "title": "Serialized native note"}),
                ))
                .await
        });
        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut pending)
                .await
                .is_err()
        );
        assert_eq!(files(fixture.project.path()), before);
        drop(lock);
        let result = tokio::time::timeout(Duration::from_secs(20), pending)
            .await
            .expect("no reentrant lock timeout")
            .expect("dispatch task")
            .expect("native dispatch");
        assert!(result.success, "{result:?}");
        assert_eq!(
            fixture.record("NOTE-008").await.title,
            "Serialized native note"
        );
        fixture.assert_owner();
    }
}

#[tokio::test]
async fn native_ingest_rejects_invalid_link_before_any_canonical_write() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated).await;
        write_ingest_sources(fixture.project.path());
        let mapping_path = fixture.project.path().join("mapping.yaml");
        let mapping = std::fs::read_to_string(&mapping_path).expect("mapping");
        std::fs::write(
            &mapping_path,
            mapping
                .replace(
                    "target_artifact_id: \"ADR-003\"",
                    "target_artifact_id: \"../escaped\"",
                )
                .replace("if_exists: skip", "if_exists: error"),
        )
        .expect("link error policy");
        let canonical_before: BTreeMap<_, _> = files(&fixture.ws)
            .into_iter()
            .filter(|(path, _)| path.extension().is_some_and(|ext| ext == "md"))
            .collect();
        let index_before = files(&fixture.ws.join("lance"));
        let result = dispatch(
            &fixture,
            ForgeplanOp::Ingest,
            ingest_input(fixture.project.path(), false, false),
        )
        .await;
        assert!(!result.success);
        assert!(result.output_path.is_none(), "a rejected batch must not report a created artifact");
        let canonical_after: BTreeMap<_, _> = files(&fixture.ws)
            .into_iter()
            .filter(|(path, _)| path.extension().is_some_and(|ext| ext == "md"))
            .collect();
        assert_eq!(canonical_after, canonical_before, "preflight must not change canonical files");
        assert_eq!(files(&fixture.ws.join("lance")), index_before, "preflight must not change the index");
        assert!(!fixture.project.path().join("escaped").exists());
        fixture.assert_owner();
    }
}

#[tokio::test]
async fn unmarked_title_collision_does_not_mask_the_imported_artifact() {
    for migrated in [false, true] {
        let mut fixture = Fixture::new(false).await;
        let lock = acquire_workspace_lock(&fixture.ws)
            .await
            .expect("fixture lock");
        let store = LanceStore::open_for_write(&fixture.ws, &lock).await.expect("fixture store");
        let ctx = MutationContext::new(&fixture.ws, &lock, Some(&store));
        seed(
            &ctx,
            "NOTE-001",
            "note",
            "Imported native note",
            "## Context\n\nThis independently authored note is not an import.\n",
            &[],
        )
        .await;
        drop(store);
        drop(lock);
        if migrated {
            publish(&fixture.ws).await;
            fixture.migrated = true;
        }
        let unrelated = fixture.ws.join("notes/NOTE-001-imported-native-note.md");
        let unrelated_bytes = std::fs::read(&unrelated).expect("unmarked collision");
        write_ingest_sources(fixture.project.path());
        let mut expected_notes = files(&fixture.ws.join("notes"));

        let first = dispatch(
            &fixture,
            ForgeplanOp::Ingest,
            ingest_input(fixture.project.path(), false, false),
        )
        .await;
        assert!(first.success, "{first:?}");
        let imported = first.output_path.expect("one new import");
        let imported_bytes = std::fs::read(&imported).expect("imported bytes");
        let first_notes = files(&fixture.ws.join("notes"));
        let imported_relative = imported
            .strip_prefix(fixture.ws.join("notes"))
            .expect("import stays in the notes directory")
            .to_path_buf();
        assert!(
            expected_notes
                .insert(imported_relative.clone(), imported_bytes.clone())
                .is_none(),
            "the import must add exactly one new file without replacing a baseline file",
        );
        assert_eq!(first_notes, expected_notes);
        assert_eq!(
            parse_frontmatter(std::str::from_utf8(&imported_bytes).expect("UTF-8"))
                .expect("import frontmatter")
                .0["id"]
                .as_str(),
            Some("NOTE-008"),
        );

        for update in [false, true] {
            let repeated = dispatch(
                &fixture,
                ForgeplanOp::Ingest,
                ingest_input(fixture.project.path(), false, update),
            )
            .await;
            assert!(repeated.success, "{repeated:?}");
            assert_eq!(repeated.output_path, None);
            assert_eq!(repeated.stderr, None);
            assert_eq!(files(&fixture.ws.join("notes")), first_notes);
        }

        std::fs::write(
            fixture.project.path().join("source.md"),
            "# Imported\n\nCollisionupdate second revision.\n",
        )
        .expect("changed source");
        let refused_update = dispatch(
            &fixture,
            ForgeplanOp::Ingest,
            ingest_input(fixture.project.path(), false, false),
        )
        .await;
        assert!(refused_update.success);
        assert_eq!(refused_update.output_path, None);
        assert_eq!(files(&fixture.ws.join("notes")), first_notes);

        let updated = dispatch(
            &fixture,
            ForgeplanOp::Ingest,
            ingest_input(fixture.project.path(), false, true),
        )
        .await;
        assert!(updated.success, "{updated:?}");
        assert_eq!(updated.output_path.as_ref(), Some(&imported));
        assert_eq!(
            std::fs::read(&unrelated).expect("unrelated note"),
            unrelated_bytes
        );
        let updated_bytes = std::fs::read_to_string(&imported).expect("updated import");
        assert!(updated_bytes.contains("Collisionupdate second revision."));
        assert!(!updated_bytes.contains("Importedneedle first revision."));
        expected_notes.insert(imported_relative, updated_bytes.as_bytes().to_vec());
        assert_eq!(files(&fixture.ws.join("notes")), expected_notes);
        let search = dispatch(
            &fixture,
            ForgeplanOp::Search,
            json!({"query": "Collisionupdate", "kind": "note"}),
        )
        .await;
        assert!(search.success);
        let payload: Value =
            serde_json::from_str(&search.stderr.expect("search JSON")).expect("JSON");
        assert_eq!(
            payload,
            json!({"results": [{
                "id": "NOTE-008",
                "kind": "note",
                "status": "draft",
                "title": "Imported native note",
            }]}),
        );
        let repeated = dispatch(
            &fixture,
            ForgeplanOp::Ingest,
            ingest_input(fixture.project.path(), false, true),
        )
        .await;
        assert!(repeated.success);
        assert_eq!(repeated.output_path, None);
        assert_eq!(
            std::fs::read_to_string(&imported).expect("no-op update"),
            updated_bytes
        );
        assert_eq!(files(&fixture.ws.join("notes")), expected_notes);
        fixture.assert_owner();
    }
}

#[tokio::test]
async fn one_ingest_operation_remembers_created_candidates_and_allocated_ids() {
    for migrated in [false, true] {
        let fixture = Fixture::new(migrated).await;
        write_ingest_sources(fixture.project.path());
        let mapping_path = fixture.project.path().join("mapping.yaml");
        let mapping = std::fs::read_to_string(&mapping_path).expect("mapping");
        std::fs::write(
            &mapping_path,
            mapping
                .replace(
                    "heading_path: [\"Imported\"]",
                    "heading_path: [\"Imported\", \"*\"]",
                )
                .replace(
                    "title: \"Imported native note\"",
                    "title: \"{{ heading_text }}\"",
                ),
        )
        .expect("two-section mapping");
        let sources = fixture.project.path().join("sources");
        std::fs::create_dir(&sources).expect("source directory");
        let source = "# Imported\n\n## Alpha\n\nBatchneedle alpha body.\n\n\
                      ## Beta\n\nBatchneedle beta body.\n";
        std::fs::write(sources.join("first.md"), source).expect("first source");
        std::fs::write(sources.join("same-content.md"), source).expect("same source text");
        let mut input = ingest_input(fixture.project.path(), false, false);
        input["source_path"] = json!(sources);
        let result = dispatch(&fixture, ForgeplanOp::Ingest, input).await;
        assert!(result.success, "{result:?}");
        assert!(result.output_path.expect("created artifact").exists());
        assert_eq!(result.stderr, None);

        let read = open_workspace_read(&fixture.ws, None, CaptureLimits::default())
            .await
            .expect("read batch artifacts");
        let notes: Vec<_> = read
            .records()
            .iter()
            .filter(|record| record.kind == "note")
            .collect();
        assert_eq!(notes.len(), 3);
        let mut ids: Vec<_> = notes.iter().map(|record| record.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, ["NOTE-007", "NOTE-008", "NOTE-009"]);
        let imported: Vec<_> = notes
            .iter()
            .filter(|record| record.id != "NOTE-007")
            .collect();
        let mut titles: Vec<_> = imported
            .iter()
            .map(|record| record.title.as_str())
            .collect();
        titles.sort_unstable();
        assert_eq!(titles, ["Alpha", "Beta"]);
        for record in imported {
            assert!(record.body.contains("Batchneedle"));
            assert!(record.body.contains("source_hash:"));
            assert!(
                read.get_relations(&record.id)
                    .contains(&("ADR-003".to_string(), "based_on".to_string(),))
            );
        }
        let search = dispatch(
            &fixture,
            ForgeplanOp::Search,
            json!({"query": "Batchneedle", "kind": "note"}),
        )
        .await;
        assert!(search.success);
        let payload: Value =
            serde_json::from_str(&search.stderr.expect("batch search JSON")).expect("JSON");
        assert_eq!(payload["results"].as_array().expect("results").len(), 2);
        fixture.assert_owner();
    }
}
