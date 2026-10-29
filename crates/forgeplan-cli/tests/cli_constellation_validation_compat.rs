//! Validation and gap reports must keep their legacy contracts after index migration.
//! Both layouts contain the same complete bodies and edges; only the read source differs.

use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use forgeplan_core::artifact::frontmatter::{links_from_frontmatter, parse_frontmatter};
use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::projection::{self, MutationContext};
use forgeplan_core::session::{Phase, SessionState};
use serde_json::Value;
use tempfile::TempDir;

const VALIDATION_ARTIFACTS: &[(&str, &str)] = &[
    (
        "notes/NOTE-001-linked-note.md",
        "---\nid: NOTE-001\nkind: note\nstatus: active\ntitle: Linked note\n\
         slug: note-linked-check\npredicted_number: 1\nassigned_number: null\n\
         depth: tactical\nlinks:\n  - target: NOTE-002\n    relation: based_on\n---\n\n\
         ## Summary\n\nThe note records the chosen publication format.\n\n\
         ## Related Artifacts\n\n| Artifact | Role |\n| --- | --- |\n\
         | NOTE-002 | Publication decision |\n",
    ),
    (
        "notes/NOTE-002-ordinary-note.md",
        "---\nid: NOTE-002\nkind: note\nstatus: stale\ntitle: Ordinary note\n\
         slug: note-ordinary-check\npredicted_number: 2\nassigned_number: 2\n\
         depth: tactical\n---\n\nThe publication format uses explicit identifiers.\n",
    ),
    (
        "notes/NOTE-003-draft-note.md",
        "---\nid: NOTE-003\nkind: note\nstatus: draft\ntitle: Draft note\ndepth: tactical\n---\n\n\
         TODO: specify the publication policy.\n\n\
         The review discusses publishing and the unresolved format decisions in detail. \
         We need to improve the publication policy.\n",
    ),
    (
        "prds/PRD-004-broken-proposal.md",
        "---\nid: PRD-004\nkind: prd\nstatus: draft\ntitle: Broken proposal\n\
         slug: prd-broken-check\npredicted_number: 4\nassigned_number: null\n\
         depth: deep\n---\n\n## Related Artifacts\n\n\
         | Artifact | Role |\n| --- | --- |\n| NOTE-001 | Input |\n",
    ),
    (
        "notes/NOTE-00005-padded-note.md",
        "---\nid: NOTE-00005\nkind: note\nstatus: draft\ntitle: Padded note\n\
         depth: tactical\n---\n\nThis historical identifier keeps its original padding.\n",
    ),
];

const CI_FAILURE_ARTIFACTS: &[(&str, &str)] = &[
    (
        "rfcs/RFC-006-active-proposal.md",
        "---\nid: RFC-006\nkind: rfc\nstatus: active\ntitle: Active proposal\n\
         depth: standard\n---\n\nAn active proposal with required sections absent.\n",
    ),
    (
        "rfcs/RFC-007-stale-proposal.md",
        "---\nid: RFC-007\nkind: rfc\nstatus: stale\ntitle: Stale proposal\n\
         depth: standard\n---\n\nA stale proposal with required sections absent.\n",
    ),
    (
        "rfcs/RFC-008-retired-proposal.md",
        "---\nid: RFC-008\nkind: rfc\nstatus: deprecated\ntitle: Retired proposal\n\
         depth: standard\n---\n\nA retired proposal is not selected by CI.\n",
    ),
];

const GAP_ARTIFACTS: &[(&str, &str)] = &[
    (
        "prds/PRD-101-missing-pipeline.md",
        "---\nid: PRD-101\nkind: prd\nstatus: active\ntitle: Missing pipeline\n\
         depth: deep\n---\n\nA decision without pipeline artifacts or evidence.\n",
    ),
    (
        "rfcs/RFC-102-refuted-direction.md",
        "---\nid: RFC-102\nkind: rfc\nstatus: active\ntitle: Refuted direction\n\
         depth: standard\n---\n\nA current pack refutes this direction.\n",
    ),
    (
        "evidence/EVID-102-refutation.md",
        "---\nid: EVID-102\nkind: evidence\nstatus: active\ntitle: Refutation\n\
         depth: tactical\nlinks:\n  - target: RFC-102\n    relation: informs\n---\n\n\
         verdict: refutes\ncongruence_level: 3\nevidence_type: test\n",
    ),
    (
        "rfcs/RFC-103-displaced-support.md",
        "---\nid: RFC-103\nkind: rfc\nstatus: active\ntitle: Displaced support\n\
         depth: standard\nlinks:\n  - target: EVID-103\n    relation: based_on\n---\n\n\
         Its sole evidence pack has been superseded.\n",
    ),
    (
        "evidence/EVID-103-displaced-pack.md",
        "---\nid: EVID-103\nkind: evidence\nstatus: superseded\ntitle: Displaced pack\n\
         depth: tactical\n---\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n",
    ),
    (
        "prds/PRD-104-supported-pipeline.md",
        "---\nid: PRD-104\nkind: prd\nstatus: active\ntitle: Supported pipeline\n\
         depth: standard\nlinks:\n  - target: EVID-104\n    relation: based_on\n---\n\n\
         Its incoming RFC and outgoing evidence satisfy the pipeline.\n",
    ),
    (
        "evidence/EVID-104-support.md",
        "---\nid: EVID-104\nkind: evidence\nstatus: active\ntitle: Support\n\
         depth: tactical\n---\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n",
    ),
    (
        "rfcs/RFC-105-implementation.md",
        "---\nid: RFC-105\nkind: rfc\nstatus: draft\ntitle: Implementation\n\
         created_at: 2999-01-01T00:00:00Z\ndepth: standard\n\
         links:\n  - target: PRD-104\n    relation: based_on\n---\n\nThe linked implementation.\n",
    ),
    (
        "prds/PRD-106-retired-pipeline.md",
        "---\nid: PRD-106\nkind: prd\nstatus: deprecated\ntitle: Retired pipeline\n\
         depth: deep\n---\n\nRetired decisions do not require new pipeline artifacts.\n",
    ),
    (
        "notes/NOTE-107-independent-note.md",
        "---\nid: NOTE-107\nkind: note\nstatus: draft\ntitle: Independent note\n\
         created_at: 2999-01-01T00:00:00Z\ndepth: tactical\n---\n\nStandalone notes are exempt.\n",
    ),
];

fn command(root: &Path, args: &[&str]) -> Output {
    Command::cargo_bin("forgeplan")
        .unwrap()
        .args(args)
        .current_dir(root)
        .env("HOME", root.join("home"))
        .env("XDG_CONFIG_HOME", root.join("home/.config"))
        .env("NO_COLOR", "1")
        .env("CLICOLOR", "0")
        .env("HF_HUB_OFFLINE", "1")
        .output()
        .unwrap()
}

fn assert_code(out: &Output, expected: i32) {
    assert_eq!(
        out.status.code(),
        Some(expected),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn table_names(workspace: &Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(workspace.join("lance"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".lance"))
        .collect();
    names.sort();
    names
}

struct Fixture {
    root: TempDir,
    current: Option<Vec<u8>>,
    tables: Vec<String>,
    sources: Vec<(PathBuf, String)>,
}

impl Fixture {
    fn run(&self, args: &[&str]) -> Output {
        command(self.root.path(), args)
    }

    fn workspace(&self) -> PathBuf {
        self.root.path().join(".forgeplan")
    }

    fn assert_read_only(&self) {
        assert_eq!(table_names(&self.workspace()), self.tables);
        if let Some(current) = &self.current {
            assert_eq!(
                std::fs::read(self.workspace().join("lance/CURRENT.json")).unwrap(),
                *current,
                "a report must not publish a new index generation"
            );
        }
        for (path, content) in &self.sources {
            assert_eq!(std::fs::read_to_string(path).unwrap(), *content);
        }
    }
}

async fn fixture(migrated: bool, sources: &[(&str, &str)]) -> Fixture {
    let root = TempDir::new().unwrap();
    std::fs::create_dir_all(root.path().join("home/.config")).unwrap();
    assert_code(&command(root.path(), &["init", "-y"]), 0);
    let workspace = root.path().join(".forgeplan");
    let guard = forgeplan_core::workspace::acquire_workspace_lock(&workspace)
        .await
        .unwrap();
    let store = LanceStore::open(&workspace).await.unwrap();
    let mut source_bytes = Vec::new();
    // Seed real legacy records with the complete source bytes, not reindex's
    // post-frontmatter body. Slug resolution and hint selection read that body.
    for (relative, content) in sources {
        let path = workspace.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
        let (fm, _) = parse_frontmatter(content).unwrap();
        let field = |name: &str| fm.get(name).and_then(|value| value.as_str()).unwrap();
        projection::sync_artifact_from_file(
            &MutationContext::new(&workspace, &guard, Some(&store)),
            &NewArtifact {
                id: field("id").into(),
                kind: field("kind").into(),
                status: field("status").into(),
                title: field("title").into(),
                body: (*content).into(),
                depth: field("depth").into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            },
        )
        .await
        .unwrap();
        source_bytes.push((path, (*content).to_string()));
    }
    for (_, content) in sources {
        let (fm, _) = parse_frontmatter(content).unwrap();
        let id = fm["id"].as_str().unwrap();
        for (target, relation) in links_from_frontmatter(&fm) {
            projection::sync_relation_from_file(
                &MutationContext::new(&workspace, &guard, Some(&store)),
                id,
                &target,
                &relation,
            )
            .await
            .unwrap();
        }
    }
    assert_eq!(store.list_records(None).await.unwrap().len(), sources.len());
    drop(store);
    drop(guard);
    let current = if migrated {
        assert_code(&command(root.path(), &["index"]), 0);
        let bytes = std::fs::read(workspace.join("lance/CURRENT.json")).unwrap();
        assert!(!workspace.join("lance/artifacts.lance").exists());
        assert!(!workspace.join("lance/relations.lance").exists());
        Some(bytes)
    } else {
        assert!(!workspace.join("lance/CURRENT.json").exists());
        None
    };
    Fixture {
        tables: table_names(&workspace),
        current,
        sources: source_bytes,
        root,
    }
}

async fn paired(sources: &[(&str, &str)]) -> [Fixture; 2] {
    [fixture(false, sources).await, fixture(true, sources).await]
}

fn payload(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|error| {
        panic!(
            "not validation JSON: {error}: {}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

// CI deliberately writes its existing textual summary after the JSON object.
fn ci_payload(out: &Output) -> (Value, &str) {
    let stdout = std::str::from_utf8(&out.stdout).unwrap();
    let mut stream = serde_json::Deserializer::from_str(stdout).into_iter::<Value>();
    let value = stream.next().expect("CI JSON result").unwrap();
    (value, stdout[stream.byte_offset()..].trim())
}

fn result_ids(value: &Value) -> Vec<&str> {
    value["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|result| result["artifact_id"].as_str().unwrap())
        .collect()
}

fn assert_finding(result: &Value, rule: &str, severity: &str) {
    assert!(
        result["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["rule_id"] == rule
                    && finding["severity"] == severity
                    && finding["message"]
                        .as_str()
                        .is_some_and(|text| !text.is_empty())
                    && finding.get("section").is_some()
            }),
        "missing {rule} ({severity}): {result}"
    );
}

#[tokio::test]
async fn validate_resolves_ids_slugs_and_uppercase_fallback_and_preserves_hints() {
    let fixtures = paired(VALIDATION_ARTIFACTS).await;
    let mut expected = None;
    for fixture in &fixtures {
        for reference in ["NOTE-001", "note-1", "note-linked-check"] {
            let out = fixture.run(&["validate", reference, "--json"]);
            assert_code(&out, 0);
            let value = payload(&out);
            assert_eq!(result_ids(&value), ["NOTE-001"]);
            assert_eq!(value["results"][0]["findings"], serde_json::json!([]));
            assert_eq!(value["results"][0]["errors"], 0);
            assert_eq!(value["results"][0]["passed"], true);
            assert_eq!(
                value["_next_action"],
                "forgeplan activate note-linked-check"
            );
            if let Some(expected) = &expected {
                assert_eq!(&value, expected);
            } else {
                expected = Some(value);
            }
            assert_eq!(
                SessionState::load(&fixture.workspace()).phase,
                Phase::Coding
            );
        }
        // Actual 0.38 accepts lowercase slugs, not uppercase slug spellings.
        // This is separate from the display-id uppercase fallback below.
        let out = fixture.run(&["validate", "NOTE-LINKED-CHECK", "--json"]);
        assert_code(&out, 1);

        // Normalizing the display shape yields NOTE-005, which does not exist.
        // Validation must retain the historical uppercase fallback to NOTE-00005.
        let out = fixture.run(&["validate", "note-00005", "--json"]);
        assert_code(&out, 0);
        let value = payload(&out);
        assert_eq!(result_ids(&value), ["NOTE-00005"]);
        assert_eq!(value["_next_action"], "forgeplan activate NOTE-00005");

        let out = fixture.run(&["validate", "note-ordinary-check"]);
        assert_code(&out, 0);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("NOTE-002") && stdout.contains("PASS"));
        assert!(stdout.contains("Next: forgeplan activate NOTE-002"));
        fixture.assert_read_only();
    }
}

#[tokio::test]
async fn validation_must_findings_fail_with_existing_json_severities_and_fix_hint() {
    let fixtures = paired(VALIDATION_ARTIFACTS).await;
    let mut values = Vec::new();
    for fixture in &fixtures {
        let out = fixture.run(&["validate", "prd-broken-check", "--json"]);
        assert_code(&out, 1);
        let value = payload(&out);
        assert_eq!(result_ids(&value), ["PRD-004"]);
        let result = &value["results"][0];
        assert_eq!(result["passed"], false);
        assert_eq!(result["kind"], "Prd");
        assert_eq!(result["depth"], "Deep");
        assert!(result["errors"].as_u64().unwrap() > 0);
        assert!(result["warnings"].as_u64().unwrap() > 0);
        assert_finding(result, "prd-problem-exists", "Must");
        assert_finding(result, "body-links-drift", "Should");
        assert_eq!(value["_next_action"], "forgeplan validate prd-broken-check");
        values.push(value);

        let out = fixture.run(&["validate", "PRD-004"]);
        assert_code(&out, 1);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("MUST") && stdout.contains("SHOULD") && stdout.contains("FAIL"));
        assert!(
            String::from_utf8_lossy(&out.stderr)
                .contains("Fix: forgeplan validate prd-broken-check")
        );
        assert_eq!(SessionState::load(&fixture.workspace()).phase, Phase::Idle);
        fixture.assert_read_only();
    }
    assert_eq!(values[0], values[1]);
}

#[tokio::test]
async fn validate_all_includes_drafts_and_ci_selects_active_and_stale_only() {
    let fixtures = paired(VALIDATION_ARTIFACTS).await;
    let mut all_values = Vec::new();
    let mut ci_values = Vec::new();
    for fixture in &fixtures {
        let out = fixture.run(&["validate", "--json"]);
        assert_code(&out, 1);
        let value = payload(&out);
        assert_eq!(
            result_ids(&value),
            ["NOTE-00005", "NOTE-001", "NOTE-002", "NOTE-003", "PRD-004"]
        );
        assert_eq!(value["_next_action"], "forgeplan health");
        all_values.push(value);
        let out = fixture.run(&["validate"]);
        assert_code(&out, 1);
        assert!(String::from_utf8_lossy(&out.stdout).contains("Summary: 5 artifact(s)"));
        assert!(String::from_utf8_lossy(&out.stderr).contains("Fix: forgeplan health"));

        let out = fixture.run(&["validate", "--ci", "--json"]);
        assert_code(&out, 0);
        let (value, summary) = ci_payload(&out);
        assert_eq!(result_ids(&value), ["NOTE-001", "NOTE-002"]);
        assert_eq!(value["_next_action"], "forgeplan health");
        assert!(summary.contains("CI PASSED") && summary.contains("2 artifact(s) validated"));
        ci_values.push(value);
        assert_eq!(SessionState::load(&fixture.workspace()).phase, Phase::Idle);
        fixture.assert_read_only();
    }
    assert_eq!(all_values[0], all_values[1]);
    assert_eq!(ci_values[0], ci_values[1]);
}

#[tokio::test]
async fn ci_failures_report_selected_active_and_stale_artifacts_and_exit_one() {
    let fixtures = paired(CI_FAILURE_ARTIFACTS).await;
    let mut values = Vec::new();
    for fixture in &fixtures {
        let out = fixture.run(&["validate", "--ci", "--json"]);
        assert_code(&out, 1);
        let (value, suffix) = ci_payload(&out);
        assert_eq!(result_ids(&value), ["RFC-006", "RFC-007"]);
        assert!(
            value["results"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["passed"] == false)
        );
        assert_finding(&value["results"][0], "rfc-summary", "Must");
        assert_eq!(value["_next_action"], "forgeplan health");
        assert!(suffix.is_empty());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("CI FAILED") && stderr.contains("in 2 artifact(s)"));
        values.push(value);
        assert_eq!(SessionState::load(&fixture.workspace()).phase, Phase::Idle);
        fixture.assert_read_only();
    }
    assert_eq!(values[0], values[1]);
}

#[tokio::test]
async fn an_explicit_ci_target_still_validates_a_draft_and_advances_only_on_pass() {
    let fixtures = paired(VALIDATION_ARTIFACTS).await;
    for fixture in &fixtures {
        let out = fixture.run(&["validate", "PRD-004", "--ci", "--json"]);
        assert_code(&out, 1);
        let (value, suffix) = ci_payload(&out);
        assert_eq!(result_ids(&value), ["PRD-004"]);
        assert_eq!(value["_next_action"], "forgeplan validate prd-broken-check");
        assert!(suffix.is_empty());
        assert!(String::from_utf8_lossy(&out.stderr).contains("in 1 artifact(s)"));
        assert_eq!(SessionState::load(&fixture.workspace()).phase, Phase::Idle);

        let out = fixture.run(&["validate", "NOTE-003", "--ci", "--json"]);
        assert_code(&out, 0);
        let (value, summary) = ci_payload(&out);
        assert_eq!(result_ids(&value), ["NOTE-003"]);
        assert_finding(&value["results"][0], "no-placeholders", "Should");
        assert_eq!(value["results"][0]["passed"], true);
        assert_eq!(value["_next_action"], "forgeplan activate NOTE-003");
        assert!(summary.contains("CI PASSED") && summary.contains("1 artifact(s) validated"));
        assert_eq!(
            SessionState::load(&fixture.workspace()).phase,
            Phase::Coding
        );
        fixture.assert_read_only();
    }
}

#[tokio::test]
async fn adversarial_findings_keep_should_and_could_severities_without_hiding_must_errors() {
    let fixtures = paired(VALIDATION_ARTIFACTS).await;
    for (reference, code, rule, severity) in [
        ("PRD-004", 1, "adversarial-no-risks", "Should"),
        ("NOTE-003", 0, "adversarial-vague", "Should"),
        ("NOTE-001", 0, "adversarial-too-clean", "Could"),
    ] {
        let mut values = Vec::new();
        for fixture in &fixtures {
            let out = fixture.run(&["validate", reference, "--adversarial", "--json"]);
            assert_code(&out, code);
            let value = payload(&out);
            assert_eq!(result_ids(&value), [reference]);
            assert_finding(&value["results"][0], rule, severity);
            if code == 1 {
                assert_finding(&value["results"][0], "prd-problem-exists", "Must");
            }
            values.push(value);
            fixture.assert_read_only();
        }
        assert_eq!(values[0], values[1]);
    }
}

#[tokio::test]
async fn a_missing_validation_target_is_an_error_with_the_existing_list_fix() {
    for fixture in paired(VALIDATION_ARTIFACTS).await {
        let out = fixture.run(&["validate", "note-does-not-exist", "--json"]);
        assert_code(&out, 1);
        assert!(out.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("not found") && stderr.contains("Fix: forgeplan list"));
        assert_eq!(SessionState::load(&fixture.workspace()).phase, Phase::Idle);
        fixture.assert_read_only();
    }
}

#[tokio::test]
async fn empty_workspaces_keep_the_existing_success_messages_including_ci_json() {
    for fixture in paired(&[]).await {
        for args in [
            vec!["validate", "--json"],
            vec!["validate", "--ci", "--json"],
            vec!["validate", "NOTE-999", "--json"],
        ] {
            let out = fixture.run(&args);
            assert_code(&out, 0);
            assert_eq!(
                String::from_utf8_lossy(&out.stdout).trim(),
                "No artifacts found."
            );
        }
        let out = fixture.run(&["gaps"]);
        assert_code(&out, 0);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("No pipeline gaps found.") && stdout.contains("Done."));
        assert_eq!(SessionState::load(&fixture.workspace()).phase, Phase::Idle);
        fixture.assert_read_only();
    }
}

#[tokio::test]
async fn ci_with_only_a_draft_keeps_its_zero_selected_summary_without_advancing_session() {
    for fixture in paired(&VALIDATION_ARTIFACTS[3..4]).await {
        let out = fixture.run(&["validate", "--ci", "--json"]);
        assert_code(&out, 0);
        let (value, summary) = ci_payload(&out);
        assert!(result_ids(&value).is_empty());
        assert_eq!(value["_next_action"], "forgeplan health");
        assert!(summary.contains("CI PASSED") && summary.contains("0 artifact(s) validated"));
        assert_eq!(SessionState::load(&fixture.workspace()).phase, Phase::Idle);
        fixture.assert_read_only();
    }
}

fn gap_entries(stdout: &str) -> Vec<(String, String)> {
    let mut severity = None;
    let mut entries = Vec::new();
    for line in stdout.lines() {
        match line.trim() {
            "MUST:" => severity = Some("MUST"),
            "SHOULD:" => severity = Some("SHOULD"),
            "COULD:" => severity = Some("COULD"),
            _ => {
                let mut words = line.split_whitespace();
                if matches!(words.next(), Some("!" | "~" | "?")) {
                    let id = words.next().expect("gap artifact id");
                    entries.push((
                        severity.expect("gap severity group").to_string(),
                        id.to_string(),
                    ));
                }
            }
        }
    }
    entries
}

#[tokio::test]
async fn gaps_keep_nonempty_depth_evidence_terminal_and_orphan_results_in_severity_order() {
    let fixtures = paired(GAP_ARTIFACTS).await;
    let expected: Vec<_> = [
        ("MUST", "PRD-101"),
        ("MUST", "PRD-101"),
        ("MUST", "RFC-103"),
        ("SHOULD", "PRD-101"),
        ("SHOULD", "PRD-101"),
        ("SHOULD", "RFC-102"),
        ("COULD", "PRD-101"),
        ("COULD", "PRD-106"),
    ]
    .into_iter()
    .map(|(severity, id)| (severity.to_string(), id.to_string()))
    .collect();
    for fixture in &fixtures {
        let out = fixture.run(&["gaps"]);
        assert_code(&out, 0);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(gap_entries(&stdout), expected);
        assert!(
            stdout.contains("3 MUST") && stdout.contains("3 SHOULD") && stdout.contains("2 COULD")
        );
        assert!(
            stdout.contains("no linked RFC")
                && stdout.contains("no linked Spec")
                && stdout.contains("no linked ADR")
        );
        assert!(stdout.contains("RFC-102 active but R_eff=0.00 (weak evidence)"));
        assert!(stdout.contains("RFC-103 active but R_eff=0 (no evidence)"));
        assert!(
            !stdout.contains("PRD-104")
                && !stdout.contains("RFC-105")
                && !stdout.contains("NOTE-107")
        );
        assert!(stdout.contains("Next: forgeplan get PRD-101"));
        fixture.assert_read_only();
    }
}

#[tokio::test]
async fn a_nonempty_compliant_pipeline_keeps_the_no_gaps_terminal_contract() {
    for fixture in paired(&GAP_ARTIFACTS[5..8]).await {
        let out = fixture.run(&["gaps"]);
        assert_code(&out, 0);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("No pipeline gaps found.") && stdout.contains("Done."));
        assert!(!stdout.contains("Next:"));
        fixture.assert_read_only();
    }
}

#[tokio::test]
async fn malformed_current_is_an_error_not_empty_validation_or_no_gaps_success() {
    let fixture = fixture(true, VALIDATION_ARTIFACTS).await;
    let pointer = fixture.workspace().join("lance/CURRENT.json");
    std::fs::write(&pointer, "{ not json").unwrap();
    for args in [
        vec!["validate", "NOTE-001", "--json"],
        vec!["validate", "--json"],
        vec!["validate", "--ci", "--json"],
        vec!["gaps"],
    ] {
        let out = fixture.run(&args);
        assert_code(&out, 1);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains("No artifacts found.") && !stdout.contains("No pipeline gaps found.")
        );
        assert!(!stdout.contains("CI PASSED") && !out.stderr.is_empty());
        assert_eq!(std::fs::read(&pointer).unwrap(), b"{ not json");
        assert_eq!(table_names(&fixture.workspace()), fixture.tables);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn an_unreadable_captured_source_is_not_ignored_by_validation_or_gaps() {
    use std::os::unix::fs::symlink;

    let fixture = fixture(true, VALIDATION_ARTIFACTS).await;
    let path = fixture.workspace().join("notes/NOTE-999-loop.md");
    symlink("NOTE-999-loop.md", &path).unwrap();
    for args in [vec!["validate", "--json"], vec!["gaps"]] {
        let out = fixture.run(&args);
        assert_code(&out, 1);
        assert!(!out.stderr.is_empty());
        assert!(!String::from_utf8_lossy(&out.stdout).contains("No artifacts found."));
        fixture.assert_read_only();
    }
}
