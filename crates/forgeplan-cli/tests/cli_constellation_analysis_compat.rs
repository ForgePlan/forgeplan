//! Rule-based analysis commands must answer the same nonempty question from
//! legacy rows or one captured Markdown view. No provider or model is used.

use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use forgeplan_core::artifact::types::ArtifactKind;
use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::projection::{self, MutationContext};
use serde_json::Value;
use tempfile::TempDir;

const PRD_SLUG: &str = "prd-analysis-plan";
const PRD_BODY: &str = "\
## Problem\n\n\
A delivery team needs the estimate and the evidence graph to describe the same \
work before and after an index migration, without opening the retired tables \
or contacting any provider.\n\n\
## Goals\n\n\
Preserve real requirements, dependency links, validation findings and useful next commands.\n\n\
## Functional Requirements\n\n\
| ID | Category | Priority | Requirement | Journey |\n\
|----|----------|----------|-------------|---------|\n\
| FR-001 | Core | Must | Implement request validation | Delivery |\n\
| FR-002 | UI | Should | Display a result table | Delivery |\n\n\
## Implementation Phases\n\n\
- [ ] **1.1** Implement the report renderer\n\
- [x] **1.2** Add unit tests for the parser\n\n\
## Related Artifacts\n\n\
SPEC-001 describes the intended behavior and NOTE-001 records the delivery context.\n";

fn command(root: &Path) -> Command {
    let home = root.join(".test-home");
    let mut cmd = Command::cargo_bin("forgeplan").unwrap();
    cmd.current_dir(root)
        .env("HOME", &home)
        .env("FORGEPLAN_MODEL_CACHE", home.join("models"))
        .env("HF_HUB_OFFLINE", "1")
        .env("NO_COLOR", "1")
        .env("CLICOLOR", "0")
        .env("TERM", "dumb");
    cmd
}

fn run(root: &Path, args: &[&str]) -> Output {
    command(root).args(args).output().unwrap()
}

fn run_ok(root: &Path, args: &[&str]) -> String {
    let out = run(root, args);
    assert!(
        out.status.success(),
        "forgeplan {args:?} failed: {}\n{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn run_json(root: &Path, args: &[&str]) -> Value {
    let stdout = run_ok(root, args);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|error| panic!("forgeplan {args:?}: {error}: {stdout}"))
}

fn artifact(
    id: &str,
    kind: &str,
    status: &str,
    title: &str,
    parent_epic: Option<&str>,
    links: &str,
    body: &str,
) -> NewArtifact {
    let (prefix, number) = id.split_once('-').unwrap();
    let number: u32 = number.parse().unwrap();
    let slug = format!(
        "{}-{}",
        prefix.to_ascii_lowercase(),
        title.to_ascii_lowercase().replace(' ', "-")
    );
    let parent = parent_epic
        .map(|id| format!("parent_epic: {id}\n"))
        .unwrap_or_default();
    NewArtifact {
        // The canonical key is the slug the document declares (ADR-030): it is
        // the document's permanent identity and the value a reference
        // resolves to. Keying the row by the display id instead would give the
        // same document two identities.
        canonical_key: slug.clone(),
        id: id.to_string(),
        kind: kind.to_string(),
        status: status.to_string(),
        title: title.to_string(),
        depth: "standard".to_string(),
        author: Some("Analysis fixture".to_string()),
        parent_epic: parent_epic.map(str::to_string),
        valid_until: Some("2099-12-31".to_string()),
        body: format!(
            "---\nid: {id}\nkind: {kind}\nstatus: {status}\ntitle: {title}\n\
             depth: standard\nauthor: Analysis fixture\nslug: {slug}\n\
             predicted_number: {number}\nassigned_number: null\nvalid_until: '2099-12-31'\n\
             {parent}{links}---\n\n{body}"
        ),
        ..Default::default()
    }
}

fn artifacts() -> Vec<NewArtifact> {
    vec![
        artifact(
            "PRD-001",
            "prd",
            "draft",
            "Analysis plan",
            Some("EPIC-001"),
            "links:\n  - target: SPEC-001\n    relation: based_on\n  - target: NOTE-001\n    relation: contradicts\n  - target: EVID-002\n    relation: informs\n",
            PRD_BODY,
        ),
        artifact(
            "SPEC-001",
            "spec",
            "draft",
            "Analysis specification",
            None,
            "",
            "## Requirements\n\nThe report uses canonical artifact data.\n",
        ),
        artifact(
            "EVID-001",
            "evidence",
            "active",
            "Current measurement",
            None,
            "links:\n  - target: PRD-001\n    relation: informs\n",
            "## Structured Fields\n\nverdict: supports\ncongruence_level: 3\nevidence_type: measurement\n\n## Observation\n\nA real local measurement supports the plan.\n",
        ),
        artifact(
            "EVID-002",
            "evidence",
            "superseded",
            "Displaced measurement",
            None,
            "",
            "## Structured Fields\n\nverdict: refutes\ncongruence_level: 3\nevidence_type: measurement\n",
        ),
        artifact(
            "RFC-001",
            "rfc",
            "draft",
            "Report implementation",
            None,
            "links:\n  - target: PRD-001\n    relation: refines\n",
            "## Implementation Phases\n\n- [ ] **1.1** Implement the command\n",
        ),
        artifact(
            "NOTE-001",
            "note",
            "draft",
            "Delivery context",
            None,
            "",
            "## Summary\n\nThis note has no estimable work items.\n",
        ),
        artifact(
            "EPIC-001",
            "epic",
            "draft",
            "Analysis delivery",
            None,
            "",
            "## Summary\n\nDeliver the analysis commands together.\n",
        ),
    ]
}

fn artifact_path(root: &Path, artifact: &NewArtifact) -> PathBuf {
    let kind: ArtifactKind = artifact.kind.parse().unwrap();
    root.join(".forgeplan").join(kind.dir_name()).join(format!(
        "{}-{}.md",
        artifact.id,
        artifact.title.to_ascii_lowercase().replace(' ', "-")
    ))
}

fn seed_legacy() -> TempDir {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir(tmp.path().join(".test-home")).unwrap();
    run_ok(tmp.path(), &["init", "-y"]);
    let records = artifacts();
    for record in &records {
        let path = artifact_path(tmp.path(), record);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, &record.body).unwrap();
    }

    // Seed the real projection through its public mutation boundary. Passing
    // the full Markdown preserves slugs; `reindex`'s prose-only caller would
    // otherwise make the legacy fixture lose its reference-form metadata.
    let ws = tmp.path().join(".forgeplan");
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let lock = forgeplan_core::workspace::lock::acquire_workspace_lock(&ws)
            .await
            .unwrap();
        let store = LanceStore::open(&ws).await.unwrap();
        let ctx = MutationContext::new(&ws, &lock, Some(&store));
        for record in &records {
            projection::sync_artifact_from_file(&ctx, record)
                .await
                .unwrap();
        }
        for (source, target, relation) in [
            ("PRD-001", "SPEC-001", "based_on"),
            ("PRD-001", "NOTE-001", "contradicts"),
            ("PRD-001", "EVID-002", "informs"),
            ("EVID-001", "PRD-001", "informs"),
            ("RFC-001", "PRD-001", "refines"),
        ] {
            projection::sync_relation_from_file(&ctx, source, target, relation)
                .await
                .unwrap();
        }
        // Populate the legacy cached score with the actual rule engine, not
        // a fabricated result. The displaced refutation must not poison it.
        let report = forgeplan_core::scoring::sync_score_target(&store, "PRD-001")
            .await
            .unwrap();
        assert!((report.r_eff - 1.0).abs() < 1e-12);
        assert_eq!(store.list_records(None).await.unwrap().len(), records.len());
        assert_eq!(store.get_all_relations().await.unwrap().len(), 5);
        assert_eq!(
            store.get_record("PRD-001").await.unwrap().unwrap().body,
            records[0].body
        );
    });
    assert!(
        forgeplan_core::workspace::load_config(&ws)
            .unwrap()
            .llm
            .is_none()
    );
    tmp
}

fn tables(root: &Path) -> Vec<String> {
    let mut entries: Vec<String> = std::fs::read_dir(root.join(".forgeplan/lance"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    entries
}

struct ReadSnapshot {
    pointer: Vec<u8>,
    tables: Vec<String>,
    sources: Vec<(PathBuf, Vec<u8>)>,
}

impl ReadSnapshot {
    fn take(root: &Path) -> Self {
        Self {
            pointer: std::fs::read(root.join(".forgeplan/lance/CURRENT.json")).unwrap(),
            tables: tables(root),
            sources: artifacts()
                .iter()
                .map(|artifact| {
                    let path = artifact_path(root, artifact);
                    let bytes = std::fs::read(&path).unwrap();
                    (path, bytes)
                })
                .collect(),
        }
    }

    fn assert_unchanged(&self, root: &Path) {
        assert_eq!(
            std::fs::read(root.join(".forgeplan/lance/CURRENT.json")).unwrap(),
            self.pointer,
            "a read command rewrote CURRENT.json"
        );
        assert_eq!(
            tables(root),
            self.tables,
            "a read created or retired an index table"
        );
        for (path, bytes) in &self.sources {
            assert_eq!(
                std::fs::read(path).unwrap(),
                *bytes,
                "{} changed",
                path.display()
            );
        }
        assert!(!root.join(".forgeplan/lance/artifacts.lance").exists());
        assert!(!root.join(".forgeplan/lance/relations.lance").exists());
    }
}

struct Pair {
    legacy: TempDir,
    migrated: TempDir,
    snapshot: ReadSnapshot,
}

impl Pair {
    fn new() -> Self {
        let legacy = seed_legacy();
        let migrated = seed_legacy();
        run_ok(migrated.path(), &["index"]);
        let snapshot = ReadSnapshot::take(migrated.path());
        Self {
            legacy,
            migrated,
            snapshot,
        }
    }

    fn roots(&self) -> [&Path; 2] {
        [self.legacy.path(), self.migrated.path()]
    }

    fn assert_read_only(&self) {
        self.snapshot.assert_unchanged(self.migrated.path());
    }
}

fn strings(value: &Value) -> Vec<&str> {
    let mut result: Vec<&str> = value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    result.sort_unstable();
    result
}

fn result_for<'a>(payload: &'a Value, id: &str) -> &'a Value {
    payload["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == id)
        .unwrap()
}

#[test]
fn context_preserves_graph_recursive_assurance_validation_and_hints_on_both_layouts() {
    let pair = Pair::new();
    let mut baseline: Option<Value> = None;
    for root in pair.roots() {
        let payload = run_json(root, &["context", PRD_SLUG, "--json"]);
        assert_eq!(payload["artifact"]["id"], "PRD-001");
        assert_eq!(payload["artifact"]["title"], "Analysis plan");
        assert_eq!(payload["artifact"]["kind"], "prd");
        assert_eq!(payload["artifact"]["status"], "draft");
        assert_eq!(payload["artifact"]["depth"], "standard");
        assert!(payload["artifact"]["created_at"].is_string());
        assert!(payload["artifact"]["updated_at"].is_string());
        assert!((payload["artifact"]["r_eff"].as_f64().unwrap() - 1.0).abs() < 1e-12);
        assert_eq!(payload["graph"]["parent"], "EPIC-001");
        assert_eq!(
            strings(&payload["graph"]["evidence"]),
            ["EVID-001", "EVID-002"]
        );
        assert_eq!(strings(&payload["graph"]["depends_on"]), ["SPEC-001"]);
        assert_eq!(strings(&payload["graph"]["dependents"]), ["RFC-001"]);
        assert_eq!(strings(&payload["graph"]["related"]), ["NOTE-001"]);
        assert!(payload["validation"]["passed"].is_boolean());
        assert!(payload["validation"]["must_errors"].is_u64());
        assert!(payload["validation"]["should_warnings"].is_u64());
        assert!(payload["derived_status"].is_string());
        assert!(payload["suggestions"].is_array());
        assert!(payload["fgr"]["formality"].is_number());
        assert!(payload["fgr"]["granularity"].as_f64().unwrap() > 0.0);
        assert!((payload["fgr"]["reliability"].as_f64().unwrap() - 1.0).abs() < 1e-12);
        assert!(payload["fgr"]["grade"].is_string());
        assert!(payload["_next_action"].as_str().unwrap().contains(PRD_SLUG));
        assert!(!payload["hints"].as_array().unwrap().is_empty());
        if let Some(before) = &baseline {
            for key in [
                "validation",
                "derived_status",
                "fgr",
                "suggestions",
                "_next_action",
                "hints",
            ] {
                assert_eq!(
                    payload[key], before[key],
                    "context field {key} changed after migration"
                );
            }
        }
        baseline = Some(payload);
        let normalized = run_json(root, &["context", "prd-1", "--json"]);
        assert_eq!(normalized["artifact"]["id"], "PRD-001");
        let text = run_ok(root, &["context", "PRD-001"]);
        for expected in [
            "PRD-001",
            "Analysis plan",
            "Status",
            "F-G-R",
            "Graph",
            "Validation",
            "EVID-001",
            "EVID-002",
            "SPEC-001",
            "RFC-001",
            "NOTE-001",
            "Next:",
        ] {
            assert!(
                text.contains(expected),
                "context text missing {expected}: {text}"
            );
        }
    }
    pair.assert_read_only();
}

#[test]
fn rule_estimate_preserves_nonempty_items_confidence_grades_overrides_and_json_envelope() {
    let pair = Pair::new();
    let mut baseline: Option<Value> = None;
    for root in pair.roots() {
        let payload = run_json(root, &["estimate", PRD_SLUG, "--json"]);
        let result = &payload["result"];
        assert_eq!(result["artifact_id"], "PRD-001");
        assert_eq!(result["artifact_title"], "Analysis plan");
        let ids: Vec<&str> = result["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["FR-001", "FR-002", "P1.1", "P1.2"]);
        assert!((result["confidence"].as_f64().unwrap() - 1.0).abs() < 1e-12);
        assert!(
            result["confidence_reasons"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "has Spec (+15%)")
        );
        assert!(
            result["confidence_reasons"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "has calibration evidence (+20%)")
        );
        assert!(result["total_score"].as_f64().unwrap() > 0.0);
        for grade in ["junior", "middle", "senior", "principal", "ai"] {
            assert!(result["totals"][grade].as_f64().unwrap() > 0.0);
        }
        assert_eq!(
            payload["_next_action"],
            format!("forgeplan calibrate-estimate {PRD_SLUG} --actual-hours <N>")
        );
        assert!(!payload["hints"].as_array().unwrap().is_empty());
        if let Some(before) = &baseline {
            assert_eq!(&payload, before);
        }
        baseline = Some(payload);

        let overridden = run_json(
            root,
            &[
                "estimate",
                PRD_SLUG,
                "--complexity",
                "FR-001=13,P1.2=1",
                "--grade",
                "junior",
                "--json",
            ],
        );
        let items = overridden["result"]["items"].as_array().unwrap();
        assert_eq!(
            items.iter().find(|v| v["id"] == "FR-001").unwrap()["complexity"],
            "Epic"
        );
        assert_eq!(
            items.iter().find(|v| v["id"] == "P1.2").unwrap()["complexity"],
            "Trivial"
        );
        assert_eq!(
            overridden["result"]["confidence"],
            baseline.as_ref().unwrap()["result"]["confidence"]
        );
        assert_ne!(
            overridden["result"]["total_score"],
            baseline.as_ref().unwrap()["result"]["total_score"]
        );
        let text = run_ok(root, &["estimate", PRD_SLUG, "--grade", "junior"]);
        for expected in [
            "Estimate for PRD-001",
            "FR-001",
            "P1.1",
            "Grade: Junior",
            "Confidence: 100%",
            "TOTAL",
            "Next:",
        ] {
            assert!(
                text.contains(expected),
                "estimate text missing {expected}: {text}"
            );
        }
        let my_grade = run_ok(root, &["estimate", PRD_SLUG, "--my-grade"]);
        assert!(my_grade.contains("Grade: Senior"), "{my_grade}");
    }
    pair.assert_read_only();
}

#[test]
fn calibration_compares_real_estimated_hours_and_requested_grades_on_both_layouts() {
    let pair = Pair::new();
    let before = run_json(pair.legacy.path(), &["estimate", PRD_SLUG, "--json"]);
    let after = run_json(pair.migrated.path(), &["estimate", PRD_SLUG, "--json"]);
    assert_eq!(before, after);
    for (grade, scale) in [(None, 1.0), (Some("junior"), 2.0), (Some("ai"), 0.25)] {
        let estimated = match grade {
            Some(grade) => before["result"]["totals"][grade].as_f64().unwrap(),
            None => before["result"]["total_score"].as_f64().unwrap(),
        };
        let actual = estimated * scale;
        let hours = actual.to_string();
        let mut args = vec![
            "calibrate-estimate",
            PRD_SLUG,
            "--actual-hours",
            hours.as_str(),
        ];
        if let Some(grade) = grade {
            args.extend(["--grade", grade]);
        }
        let legacy = run_ok(pair.legacy.path(), &args);
        let migrated = run_ok(pair.migrated.path(), &args);
        assert_eq!(legacy, migrated);
        assert!(
            migrated.contains(&format!("Estimated:  {estimated:.1}h (confidence 100%)")),
            "{migrated}"
        );
        assert!(
            migrated.contains(&format!("Actual:     {actual:.1}h")),
            "{migrated}"
        );
        assert!(
            migrated.contains(&format!("Ratio:      {scale:.2}x (actual / estimated)")),
            "{migrated}"
        );
        let accuracy = if scale > 1.0 {
            100.0 / scale
        } else {
            scale * 100.0
        };
        assert!(
            migrated.contains(&format!("Accuracy:   {accuracy:.0}%")),
            "{migrated}"
        );
        let next = if scale == 1.0 {
            format!("forgeplan score {PRD_SLUG}")
        } else {
            format!("forgeplan estimate {PRD_SLUG} --my-grade")
        };
        assert!(migrated.contains(&next), "{migrated}");
    }
    pair.assert_read_only();
}

#[test]
fn fgr_workspace_and_single_artifact_preserve_scores_outgoing_links_and_text() {
    let pair = Pair::new();
    let legacy = run_json(pair.legacy.path(), &["fgr", "--json"]);
    let migrated = run_json(pair.migrated.path(), &["fgr", "--json"]);
    assert_eq!(legacy, migrated);
    assert_eq!(migrated["results"].as_array().unwrap().len(), 7);
    let prd = result_for(&migrated, "PRD-001");
    assert_eq!(prd["title"], "Analysis plan");
    for key in ["formality", "granularity", "reliability", "overall"] {
        assert!(prd[key].is_number(), "F-G-R {key} lost its numeric type");
        assert!(prd[key].as_f64().unwrap() > 0.0);
    }
    assert!(prd["grade"].is_string());
    // Three outgoing edges earn 0.3 * 0.67, not the five-edge context bonus.
    assert!((prd["reliability"].as_f64().unwrap() - 0.901).abs() < 1e-12);
    // NOTE has an incoming edge but no outgoing ones and no evidence cache.
    assert!(
        (result_for(&migrated, "NOTE-001")["reliability"]
            .as_f64()
            .unwrap()
            - 0.2)
            .abs()
            < 1e-12
    );
    for root in pair.roots() {
        for id in ["PRD-001", "prd-1", PRD_SLUG] {
            let single = run_json(root, &["fgr", id, "--json"]);
            assert_eq!(single["results"].as_array().unwrap().len(), 1);
            assert_eq!(&single["results"][0], prd);
            assert_eq!(single["_next_action"], format!("forgeplan get {PRD_SLUG}"));
        }
        let text = run_ok(root, &["fgr"]);
        for expected in [
            "ID",
            "Title",
            "Grade",
            "PRD-001",
            "Analysis plan",
            "EVID-001",
            "NOTE-001",
            "Next:",
        ] {
            assert!(
                text.contains(expected),
                "F-G-R text missing {expected}: {text}"
            );
        }
        let single = run_ok(root, &["fgr", PRD_SLUG]);
        assert!(single.contains("PRD-001"));
        assert!(!single.contains("EVID-001"));
        assert!(single.contains(&format!("forgeplan get {PRD_SLUG}")));
    }
    pair.assert_read_only();
}

#[test]
fn analysis_errors_remain_named_and_empty_estimates_remain_actionable_on_both_layouts() {
    let pair = Pair::new();
    for root in pair.roots() {
        // One code per fact: a selector that names no document exits 3, and a
        // command with its own advisory contract keeps the code it always
        // answered. Both say 'not found' and neither invents a document.
        for (args, code) in [
            (vec!["context", "PRD-404", "--json"], 3),
            (vec!["estimate", "PRD-404", "--json"], 1),
            (vec!["fgr", "PRD-404", "--json"], 1),
            (
                vec!["calibrate-estimate", "PRD-404", "--actual-hours", "8"],
                1,
            ),
        ] {
            let output = run(root, &args);
            assert_eq!(output.status.code(), Some(code), "{args:?}");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("PRD-404") && stderr.contains("not found"),
                "{stderr}"
            );
            if args[0] != "context" {
                assert!(stderr.contains("Fix: forgeplan list"), "{stderr}");
            }
        }
        let empty = run_json(root, &["estimate", "NOTE-001", "--json"]);
        assert_eq!(empty["result"]["artifact_id"], "NOTE-001");
        assert_eq!(empty["result"]["items"], serde_json::json!([]));
        assert_eq!(empty["result"]["totals"], serde_json::json!({}));
        assert_eq!(empty["result"]["total_score"], 0.0);
        assert_eq!(empty["_next_action"], "forgeplan get note-delivery-context");
        let no_items = run(
            root,
            &["calibrate-estimate", "NOTE-001", "--actual-hours", "8"],
        );
        assert_eq!(no_items.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&no_items.stderr)
                .contains("No estimable items in NOTE-001. Cannot calibrate.")
        );
        for hours in ["0", "NaN", "inf"] {
            let output = run(
                root,
                &["calibrate-estimate", PRD_SLUG, "--actual-hours", hours],
            );
            assert_eq!(output.status.code(), Some(1));
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("Actual hours must be a positive finite number"),
                "{stderr}"
            );
            assert!(
                stderr.contains("Fix: forgeplan calibrate-estimate"),
                "{stderr}"
            );
        }
        for (args, error) in [
            (
                vec!["estimate", PRD_SLUG, "--complexity", "FR-001=4"],
                "Invalid Fibonacci value",
            ),
            (
                vec!["estimate", PRD_SLUG, "--grade", "unknown"],
                "Unknown grade",
            ),
            // This is only the precondition error. There is no provider config,
            // client invocation, fabricated response or real-provider test.
            (
                vec!["estimate", PRD_SLUG, "--llm-score", "--json"],
                "LLM not configured",
            ),
        ] {
            let output = run(root, &args);
            assert_eq!(output.status.code(), Some(1));
            assert!(String::from_utf8_lossy(&output.stderr).contains(error));
        }
    }
    pair.assert_read_only();
}

#[test]
fn a_broken_capture_fails_every_analysis_command_instead_of_reporting_empty_success() {
    let pair = Pair::new();
    let root = pair.migrated.path();
    std::fs::write(root.join(".forgeplan/lance/CURRENT.json"), "{ not json").unwrap();
    let broken = ReadSnapshot::take(root);
    for args in [
        vec!["context", PRD_SLUG, "--json"],
        vec!["estimate", PRD_SLUG, "--json"],
        vec!["fgr", "--json"],
        vec!["fgr", PRD_SLUG, "--json"],
        vec!["calibrate-estimate", PRD_SLUG, "--actual-hours", "8"],
    ] {
        let output = run(root, &args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(
            output.stdout.is_empty(),
            "{args:?} emitted a success payload"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("CURRENT")
                || stderr.contains("current")
                || stderr.contains("JSON")
                || stderr.contains("json"),
            "{args:?} failed without naming the corrupt capture: {stderr}"
        );
    }
    broken.assert_unchanged(root);
}

#[test]
fn captured_composites_observe_offline_edits_without_republishing_the_index() {
    let pair = Pair::new();
    let root = pair.migrated.path();
    let source = artifact_path(root, &artifacts()[0]);
    let original = std::fs::read_to_string(&source).unwrap();
    let edited = original
        .replace("title: Analysis plan", "title: Updated analysis plan")
        .replace("  - target: NOTE-001\n    relation: contradicts\n", "")
        .replace(
            "## Implementation Phases",
            "| FR-003 | Core | Must | Export the validation report | Delivery |\n\n## Implementation Phases",
        );
    assert_ne!(original, edited);
    std::fs::write(&source, edited).unwrap();
    let after_edit = ReadSnapshot::take(root);
    let context = run_json(root, &["context", PRD_SLUG, "--json"]);
    assert_eq!(context["artifact"]["title"], "Updated analysis plan");
    assert!(context["graph"]["related"].as_array().unwrap().is_empty());
    let estimate = run_json(root, &["estimate", PRD_SLUG, "--json"]);
    assert_eq!(
        estimate["result"]["artifact_title"],
        "Updated analysis plan"
    );
    assert_eq!(estimate["result"]["items"].as_array().unwrap().len(), 5);
    assert!(
        estimate["result"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == "FR-003")
    );
    let scores = run_json(root, &["fgr", PRD_SLUG, "--json"]);
    assert_eq!(scores["results"][0]["title"], "Updated analysis plan");
    assert_eq!(after_edit.pointer, pair.snapshot.pointer);
    after_edit.assert_unchanged(root);
}

#[test]
fn post_merge_hints_use_the_assigned_display_id_even_when_queried_by_slug() {
    let pair = Pair::new();
    for root in pair.roots() {
        let path = artifact_path(root, &artifacts()[0]);
        let assigned = std::fs::read_to_string(&path)
            .unwrap()
            .replace("assigned_number: null", "assigned_number: 1");
        std::fs::write(&path, &assigned).unwrap();
        if root == pair.legacy.path() {
            let ws = root.join(".forgeplan");
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let lock = forgeplan_core::workspace::lock::acquire_workspace_lock(&ws)
                    .await
                    .unwrap();
                let store = LanceStore::open(&ws).await.unwrap();
                projection::sync_body_from_file(
                    &MutationContext::new(&ws, &lock, Some(&store)),
                    "PRD-001",
                    "prd",
                    "Analysis plan",
                    &assigned,
                )
                .await
                .unwrap();
            });
        }
    }
    let snapshot = ReadSnapshot::take(pair.migrated.path());
    for root in pair.roots() {
        let context = run_json(root, &["context", PRD_SLUG, "--json"]);
        let next = context["_next_action"].as_str().unwrap();
        assert!(
            next.contains("PRD-001") && !next.contains(PRD_SLUG),
            "{next}"
        );
        let estimate = run_json(root, &["estimate", PRD_SLUG, "--json"]);
        assert_eq!(
            estimate["_next_action"],
            "forgeplan calibrate-estimate PRD-001 --actual-hours <N>"
        );
        let fgr = run_json(root, &["fgr", PRD_SLUG, "--json"]);
        assert_eq!(fgr["_next_action"], "forgeplan get PRD-001");
        let actual = estimate["result"]["total_score"]
            .as_f64()
            .unwrap()
            .to_string();
        let calibration = run_ok(
            root,
            &["calibrate-estimate", PRD_SLUG, "--actual-hours", &actual],
        );
        assert!(
            calibration.contains("forgeplan score PRD-001"),
            "{calibration}"
        );
        assert!(!calibration.contains(PRD_SLUG), "{calibration}");
    }
    assert_eq!(snapshot.pointer, pair.snapshot.pointer);
    snapshot.assert_unchanged(pair.migrated.path());
}
