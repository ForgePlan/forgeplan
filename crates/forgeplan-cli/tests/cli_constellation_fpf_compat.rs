//! FPF compatibility over real legacy rows, captured source graphs and auxiliary KB tables.
//! No provider, embedding model or network is used by these scenarios.

use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use forgeplan_core::db::auxiliary::AuxiliaryStore;
use forgeplan_core::db::store::FpfChunk;
use forgeplan_core::fpf::core::config::FpfConfig;
use forgeplan_core::fpf::core::model::ActionType;
use forgeplan_core::fpf::ext::rules::{Condition, NumericExpr, Rule, ValueMatch};
use tempfile::TempDir;

struct Fixture {
    root: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self {
            root: TempDir::new().unwrap(),
        };
        std::fs::create_dir_all(fixture.root.path().join("home")).unwrap();
        fixture.ok(&["init", "-y"]);
        fixture
    }

    fn workspace(&self) -> PathBuf {
        self.root.path().join(".forgeplan")
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::cargo_bin("forgeplan")
            .unwrap()
            .current_dir(self.root.path())
            .env("HOME", self.root.path().join("home"))
            .env("NO_COLOR", "1")
            .env("CLICOLOR_FORCE", "0")
            .env("HF_HUB_OFFLINE", "1")
            .env(
                "FORGEPLAN_MODEL_CACHE",
                self.root.path().join("model-cache"),
            )
            .args(args)
            .output()
            .unwrap()
    }

    fn ok(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "forgeplan {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr),
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn json(&self, args: &[&str]) -> serde_json::Value {
        let output = self.ok(args);
        serde_json::from_str(&output).unwrap_or_else(|error| {
            panic!("forgeplan {args:?} did not emit JSON: {error}: {output}")
        })
    }

    fn migrate(&self) {
        self.ok(&["index"]);
        assert!(self.workspace().join("lance/CURRENT.json").is_file());
        assert!(!self.workspace().join("lance/artifacts.lance").exists());
        assert!(!self.workspace().join("lance/relations.lance").exists());
    }

    fn write(&self, relative_path: &str, bytes: &str) {
        let path = self.workspace().join(relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    fn project_legacy_files(&self) {
        let output = self.ok(&["reindex"]);
        assert!(
            !output.contains("create failed"),
            "fixture projection failed: {output}"
        );
        let records = self.json(&["list", "--json"]);
        assert!(
            records
                .as_array()
                .is_some_and(|records| !records.is_empty()),
            "legacy fixture is empty: {records}",
        );
    }

    fn seed_graph(&self) {
        self.write(
            "prds/PRD-014-auth-boundary.md",
            "---\nid: PRD-014\nslug: prd-auth-boundary\nkind: prd\ntitle: Auth boundary\n\
             status: draft\ndepth: standard\ncreated_at: 2026-01-01T00:00:00Z\n\
             links:\n  - target: RFC-008\n    relation: based_on\n---\n\n\
             ## Problem\n\nKeep the authentication boundary explicit.\n",
        );
        self.write(
            "rfcs/RFC-008-session-store.md",
            "---\nid: RFC-008\nslug: rfc-session-store\nkind: rfc\ntitle: Session store\n\
             status: active\ndepth: standard\ncreated_at: 2026-01-01T00:00:00Z\n---\n\n\
             ## Design\n\nPersist sessions behind the boundary.\n",
        );
        self.write(
            "adrs/ADR-009-token-policy.md",
            "---\nid: ADR-009\nslug: adr-token-policy\nkind: adr\ntitle: Token policy\n\
             status: draft\ndepth: standard\ncreated_at: 2026-01-01T00:00:00Z\n---\n\n\
             ## Decision\n\nAn unlinked decision.\n",
        );
        self.project_legacy_files();
    }

    fn configure_rules(&self) {
        let mut config = forgeplan_core::workspace::load_config(&self.workspace()).unwrap();
        config.fpf = Some(FpfConfig {
            rules: vec![
                Rule {
                    name: "linked-draft".to_string(),
                    priority: 7,
                    condition: Condition {
                        kind: Some(ValueMatch::Single("prd".to_string())),
                        status: Some(ValueMatch::Single("draft".to_string())),
                        link_count: Some(NumericExpr::Eq(1.0)),
                        ..Default::default()
                    },
                    action: ActionType::Investigate,
                    message: Some("Inspect the linked draft.".to_string()),
                },
                Rule {
                    name: "active-policy".to_string(),
                    priority: 9,
                    condition: Condition {
                        status: Some(ValueMatch::Single("active".to_string())),
                        ..Default::default()
                    },
                    action: ActionType::Exploit,
                    message: Some("Act on the active policy.".to_string()),
                },
                Rule {
                    name: "unlinked-draft".to_string(),
                    priority: 11,
                    condition: Condition {
                        status: Some(ValueMatch::Single("draft".to_string())),
                        link_count: Some(NumericExpr::Eq(0.0)),
                        ..Default::default()
                    },
                    action: ActionType::Explore,
                    message: Some("Explore the unlinked draft.".to_string()),
                },
            ],
            ..Default::default()
        });
        std::fs::write(
            self.workspace().join("config.yaml"),
            serde_yaml::to_string(&config).unwrap(),
        )
        .unwrap();
    }

    async fn seed_knowledge(&self) {
        let store = AuxiliaryStore::open_or_create(&self.workspace())
            .await
            .unwrap();
        let chunks: Vec<FpfChunk> = [
            (
                "B.3",
                "Trust calculus",
                "Trust requires explicit evidence.",
                12,
            ),
            (
                "C.2.2",
                "Bounded contexts",
                "Contexts follow actual graph links.",
                22,
            ),
        ]
        .into_iter()
        .map(|(section_id, title, body, line_count)| FpfChunk {
            id: format!("fpf-{section_id}"),
            section_id: section_id.to_string(),
            parent_section: None,
            title: title.to_string(),
            body: body.to_string(),
            line_count,
            file_path: format!("{section_id}.md"),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        })
        .collect();
        assert_eq!(store.insert_fpf_chunks(&chunks, None).await.unwrap(), 2);
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ReadState {
    entries: Vec<String>,
    current: Option<Vec<u8>>,
}

fn read_state(workspace: &Path) -> ReadState {
    let lance = workspace.join("lance");
    let mut entries: Vec<String> = std::fs::read_dir(&lance)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_str().unwrap().to_string())
        .collect();
    entries.sort();
    ReadState {
        entries,
        current: std::fs::read(lance.join("CURRENT.json")).ok(),
    }
}

fn failure(output: &Output) -> String {
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    assert!(
        !stderr.trim().is_empty(),
        "a failed command must explain its failure"
    );
    stderr
}

/// The exact bytes of every manifest file in a table's version directory.
fn manifest_bytes(versions: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(versions)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                std::fs::read(entry.path()).unwrap(),
            )
        })
        .collect();
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

/// Every path below `root`, with its kind and size.
fn tree(root: &Path) -> Vec<String> {
    let mut entries = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let metadata = entry.metadata().unwrap();
            if metadata.is_dir() {
                entries.push(format!("{}/", path.display()));
                pending.push(path);
            } else {
                entries.push(format!("{} {}", path.display(), metadata.len()));
            }
        }
    }
    entries.sort();
    entries
}

#[tokio::test]
async fn list_and_existing_auxiliary_commands_preserve_real_sections_after_migration() {
    let fixture = Fixture::new();
    fixture.seed_graph();
    fixture.seed_knowledge().await;
    let commands: &[&[&str]] = &[
        &["fpf", "list"],
        &["fpf", "search", "trust", "--limit", "5"],
        &["fpf", "section", "B.3"],
        &["fpf", "section", "C.2.2", "--summary"],
        &["fpf", "status"],
    ];
    let legacy: Vec<String> = commands.iter().map(|args| fixture.ok(args)).collect();
    assert!(legacy[0].contains("Trust calculus"));
    assert!(legacy[0].contains("Bounded contexts"));
    assert!(legacy[0].contains("2 sections total"));
    assert!(legacy[0].contains("Next: forgeplan fpf section B.3"));
    assert!(legacy[1].contains("B.3"));
    assert!(legacy[1].contains("Trust requires explicit evidence."));
    assert!(legacy[2].contains("Trust requires explicit evidence."));
    assert!(legacy[2].contains("Done."));
    assert!(legacy[3].contains("Contexts follow actual graph links."));
    assert!(legacy[3].contains("Next: forgeplan fpf section C.2.2"));
    assert!(legacy[4].contains("2 sections, 34 total lines"));

    fixture.migrate();
    let before = read_state(&fixture.workspace());
    for (args, expected) in commands.iter().zip(legacy) {
        assert_eq!(fixture.ok(args), expected, "forgeplan {args:?}");
        assert_eq!(read_state(&fixture.workspace()), before);
    }
}

#[test]
fn dashboard_check_and_rules_preserve_custom_graph_policy_and_real_id_hints() {
    let fixture = Fixture::new();
    fixture.seed_graph();
    fixture.configure_rules();
    let dashboard = fixture.ok(&["fpf", "dashboard"]);
    assert!(dashboard.contains("=== FPF Dashboard (3 artifacts) ==="));
    assert!(dashboard.contains("[Context-1 (RFC)] 2 members, cohesion 100%: PRD-014, RFC-008"));
    assert!(dashboard.contains("[Unlinked] 1 members, cohesion 0%: ADR-009"));
    assert!(dashboard.contains("3 artifacts total: 1 active, 2 draft"));
    assert!(dashboard.contains("Next: forgeplan fpf rules"));
    let check = fixture.json(&["fpf", "check", "PRD-014", "--json"]);
    assert_eq!(check["artifact_id"], "PRD-014");
    assert_eq!(check["kind"], "prd");
    assert_eq!(check["status"], "draft");
    assert_eq!(check["winning"]["name"], "linked-draft");
    assert_eq!(check["winning"]["priority"], 7);
    assert_eq!(check["winning"]["action"], "INVESTIGATE");
    assert_eq!(check["matched"].as_array().unwrap().len(), 1);
    assert_eq!(
        check["unmatched"],
        serde_json::json!(["active-policy", "unlinked-draft"])
    );
    assert_eq!(
        check["summary"],
        "1 matched, 2 unmatched, winning: linked-draft"
    );
    assert_eq!(check["_next_action"], "forgeplan reason PRD-014");
    let active = fixture.json(&["fpf", "check", "RFC-008", "--json"]);
    assert_eq!(active["winning"]["name"], "active-policy");
    assert_eq!(active["_next_action"], "forgeplan activate RFC-008");
    let rules = fixture.json(&["fpf", "rules", "--json"]);
    assert_eq!(rules["source"], "Config");
    assert_eq!(rules["count"], 3);
    assert_eq!(rules["rules"][0]["name"], "linked-draft");
    assert_eq!(rules["_next_action"], "forgeplan fpf check PRD-014");
    let flat = fixture.ok(&["fpf", "rules", "--flat"]);
    let tree = fixture.ok(&["fpf", "rules"]);
    let verbose = fixture.ok(&["fpf", "check", "PRD-014", "--verbose"]);
    assert!(verbose.contains("active-policy"));
    assert!(verbose.contains("unlinked-draft"));

    fixture.migrate();
    let before = read_state(&fixture.workspace());
    assert_eq!(fixture.ok(&["fpf", "dashboard"]), dashboard);
    assert_eq!(fixture.json(&["fpf", "check", "PRD-014", "--json"]), check);
    assert_eq!(fixture.json(&["fpf", "check", "RFC-008", "--json"]), active);
    assert_eq!(fixture.json(&["fpf", "rules", "--json"]), rules);
    assert_eq!(fixture.ok(&["fpf", "rules", "--flat"]), flat);
    assert_eq!(fixture.ok(&["fpf", "rules"]), tree);
    assert_eq!(
        fixture.ok(&["fpf", "check", "PRD-014", "--verbose"]),
        verbose
    );
    let missing = failure(&fixture.run(&["fpf", "check", "PRD-999", "--json"]));
    assert!(missing.contains("Artifact 'PRD-999' not found"));
    assert!(missing.contains("forgeplan list --type prd"));
    assert_eq!(read_state(&fixture.workspace()), before);
}

#[test]
fn absent_knowledge_base_is_explained_without_creating_tables_or_blocking_artifact_policy() {
    let fixture = Fixture::new();
    fixture.seed_graph();
    std::fs::rename(
        fixture.workspace().join("lance/fpf_spec.lance"),
        fixture.root.path().join("saved-fpf-spec.lance"),
    )
    .unwrap();
    let list = fixture.ok(&["fpf", "list"]);
    assert!(list.contains("No FPF sections loaded. Run `forgeplan fpf ingest` first."));
    assert!(list.contains("Next: forgeplan fpf ingest"));
    let status = fixture.ok(&["fpf", "status"]);
    assert!(status.contains("Ingested:  not initialized"));
    assert!(status.contains("Next: forgeplan fpf ingest"));
    let search_error = failure(&fixture.run(&["fpf", "search", "trust"]));
    assert!(search_error.contains("FPF knowledge base not initialized"));
    let section_error = failure(&fixture.run(&["fpf", "section", "B.3"]));
    assert!(section_error.contains("FPF knowledge base not initialized"));
    let dashboard = fixture.ok(&["fpf", "dashboard"]);
    let check = fixture.json(&["fpf", "check", "PRD-014", "--json"]);
    let rules = fixture.json(&["fpf", "rules", "--json"]);
    assert_eq!(rules["source"], "Default");
    assert_eq!(rules["_next_action"], "forgeplan fpf check PRD-014");

    fixture.migrate();
    let before = read_state(&fixture.workspace());
    assert_eq!(fixture.ok(&["fpf", "list"]), list);
    assert_eq!(fixture.ok(&["fpf", "status"]), status);
    assert_eq!(
        failure(&fixture.run(&["fpf", "search", "trust"])),
        search_error
    );
    assert_eq!(
        failure(&fixture.run(&["fpf", "section", "B.3"])),
        section_error
    );
    assert_eq!(fixture.ok(&["fpf", "dashboard"]), dashboard);
    assert_eq!(fixture.json(&["fpf", "check", "PRD-014", "--json"]), check);
    assert_eq!(fixture.json(&["fpf", "rules", "--json"]), rules);
    assert_eq!(read_state(&fixture.workspace()), before);
    assert!(!fixture.workspace().join("lance/fpf_spec.lance").exists());
}

#[test]
fn empty_workspaces_preserve_default_policy_fallback_hints_and_not_found_exit() {
    let fixture = Fixture::new();
    let dashboard = fixture.ok(&["fpf", "dashboard"]);
    assert!(dashboard.contains("FPF Dashboard (0 artifacts)"));
    assert!(dashboard.contains("0 artifacts total: 0 active, 0 draft"));
    let list = fixture.ok(&["fpf", "list"]);
    assert!(list.contains("Next: forgeplan fpf ingest"));
    let rules = fixture.json(&["fpf", "rules", "--json"]);
    assert_eq!(rules["source"], "Default");
    assert!(rules["count"].as_u64().unwrap() > 0);
    assert_eq!(rules["_next_action"], "forgeplan fpf check PRD-NNN");
    let error = failure(&fixture.run(&["fpf", "check", "PRD-404"]));
    assert!(error.contains("Artifact 'PRD-404' not found"));

    fixture.migrate();
    let before = read_state(&fixture.workspace());
    assert_eq!(fixture.ok(&["fpf", "dashboard"]), dashboard);
    assert_eq!(fixture.ok(&["fpf", "list"]), list);
    assert_eq!(fixture.json(&["fpf", "rules", "--json"]), rules);
    assert_eq!(failure(&fixture.run(&["fpf", "check", "PRD-404"])), error);
    assert_eq!(read_state(&fixture.workspace()), before);
}

#[test]
fn rules_hints_keep_rfc_adr_and_empty_preference_on_both_layouts() {
    for (directory, kind, id, expected) in [
        ("rfcs", "rfc", "RFC-037", "RFC-037"),
        ("adrs", "adr", "ADR-042", "ADR-042"),
        ("notes", "note", "NOTE-001", "PRD-NNN"),
    ] {
        let fixture = Fixture::new();
        fixture.write(
            &format!("{directory}/{id}-hint-fixture.md"),
            &format!(
                "---\nid: {id}\nkind: {kind}\ntitle: Hint fixture\nstatus: draft\ndepth: standard\n---\n\nHint body.\n"
            ),
        );
        fixture.project_legacy_files();
        let legacy = fixture.json(&["fpf", "rules", "--json"]);
        assert_eq!(
            legacy["_next_action"],
            format!("forgeplan fpf check {expected}")
        );
        fixture.migrate();
        let before = read_state(&fixture.workspace());
        assert_eq!(fixture.json(&["fpf", "rules", "--json"]), legacy);
        assert_eq!(read_state(&fixture.workspace()), before);
    }
}

#[test]
fn incomplete_capture_fails_artifact_reports_instead_of_returning_empty_success() {
    let fixture = Fixture::new();
    fixture.seed_graph();
    fixture.migrate();
    fixture.write("prds/broken.md", "---\nid: [unclosed\n\nUnusable source.\n");
    let before = read_state(&fixture.workspace());
    for args in [
        vec!["fpf", "dashboard"],
        vec!["fpf", "check", "PRD-014", "--json"],
        vec!["fpf", "rules", "--json"],
    ] {
        let output = fixture.run(&args);
        let error = failure(&output);
        assert!(!error.contains("Artifact 'PRD-014' not found"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("PRD-NNN"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("0 artifacts total"));
        assert_eq!(read_state(&fixture.workspace()), before);
    }
    // A KB-only list does not read the artifact graph at all.
    assert!(
        fixture
            .ok(&["fpf", "list"])
            .contains("Next: forgeplan fpf ingest")
    );
    assert_eq!(read_state(&fixture.workspace()), before);
}

#[test]
fn corrupt_existing_knowledge_base_is_not_reported_as_missing_or_empty() {
    for migrated in [false, true] {
        let fixture = Fixture::new();
        if migrated {
            fixture.migrate();
        }
        let table = fixture.workspace().join("lance/fpf_spec.lance");
        std::fs::rename(&table, fixture.root.path().join("saved-fpf-spec.lance")).unwrap();
        std::fs::create_dir_all(&table).unwrap();
        std::fs::write(table.join("unreadable-table"), "not a Lance dataset").unwrap();
        let before = read_state(&fixture.workspace());
        for args in [
            vec!["fpf", "list"],
            vec!["fpf", "status"],
            vec!["fpf", "search", "trust"],
            vec!["fpf", "section", "B.3"],
        ] {
            let output = fixture.run(&args);
            let error = failure(&output);
            assert!(
                !error.contains("FPF knowledge base not initialized"),
                "{error}"
            );
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(!stdout.contains("No FPF sections loaded"));
            assert!(!stdout.contains("Ingested:  not initialized"));
            assert_eq!(read_state(&fixture.workspace()), before);
        }
    }
}

/// A damaged knowledge base must be refused as corruption, never abort the
/// process.
///
/// `lance` derives the manifest length by subtracting the metadata offset
/// recorded in the manifest footer from the file size
/// (`lance-4.0.0/src/dataset.rs:612`), so manifest bytes that were
/// overwritten make that subtraction underflow and kill the process with a
/// panic instead of a reportable error.
#[tokio::test]
async fn corrupt_knowledge_base_manifests_are_refused_without_a_panic() {
    for migrated in [false, true] {
        let fixture = Fixture::new();
        if migrated {
            fixture.migrate();
        }
        fixture.seed_knowledge().await;

        // The intact knowledge base still answers.
        let stdout = fixture.ok(&["fpf", "search", "trust", "--limit", "3"]);
        assert!(stdout.contains("Trust calculus"), "{stdout}");

        let table = fixture.workspace().join("lance/fpf_spec.lance");
        let versions = table.join("_versions");
        let mut manifests: Vec<PathBuf> = std::fs::read_dir(&versions)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().and_then(|extension| extension.to_str()) == Some("manifest")
            })
            .collect();
        manifests.sort();
        assert!(
            !manifests.is_empty(),
            "the fixture must produce real Lance manifests"
        );
        for manifest in &manifests {
            std::fs::write(manifest, b"deliberately invalid owned table version").unwrap();
        }

        let before = read_state(&fixture.workspace());
        // The injected fault bytes and the nested table inventory are
        // snapshotted after the injection: a refused command that rewrote or
        // removed the damaged version must not pass unnoticed.
        let injected = manifest_bytes(&versions);
        let inventory = tree(&table);
        for args in [
            vec!["fpf", "list"],
            vec!["fpf", "status"],
            vec!["fpf", "search", "trust", "--limit", "3"],
            vec!["fpf", "section", "B.3"],
        ] {
            let output = fixture.run(&args);
            let error = failure(&output);
            assert!(error.contains("Corrupt Lance manifest"), "{error}");
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(!stdout.contains("Trust calculus"));
            assert!(!stdout.contains("No FPF sections loaded"));
            assert!(!stdout.contains("Ingested:  not initialized"));
            assert!(!stdout.contains("No FPF sections match"));
            assert_eq!(
                manifest_bytes(&versions),
                injected,
                "a refused command changed the injected manifest bytes"
            );
            assert_eq!(
                tree(&table),
                inventory,
                "a refused command changed the table inventory"
            );
            assert_eq!(read_state(&fixture.workspace()), before);
        }
    }
}

#[test]
fn captured_reports_follow_file_edits_without_republishing_or_opening_shadow_tables() {
    let fixture = Fixture::new();
    fixture.seed_graph();
    fixture.configure_rules();
    fixture.migrate();
    let shadow = fixture.workspace().join("lance/artifacts.lance");
    std::fs::create_dir_all(&shadow).unwrap();
    std::fs::write(shadow.join("must-not-be-opened"), "not a legacy table").unwrap();
    let before = read_state(&fixture.workspace());
    let original = fixture.json(&["fpf", "check", "PRD-014", "--json"]);
    assert_eq!(original["winning"]["name"], "linked-draft");

    fixture.write(
        "prds/PRD-014-auth-boundary.md",
        "---\nid: PRD-014\nslug: prd-auth-boundary\nkind: prd\ntitle: Auth boundary\n\
         status: active\ndepth: standard\ncreated_at: 2026-01-01T00:00:00Z\n---\n\n\
         ## Problem\n\nThe source is now active and no longer linked.\n",
    );
    let changed = fixture.json(&["fpf", "check", "PRD-014", "--json"]);
    assert_eq!(changed["status"], "active");
    assert_eq!(changed["winning"]["name"], "active-policy");
    assert_eq!(changed["_next_action"], "forgeplan activate PRD-014");
    let dashboard = fixture.ok(&["fpf", "dashboard"]);
    assert!(dashboard.contains("[Unlinked] 3 members, cohesion 0%: ADR-009, PRD-014, RFC-008"));
    assert!(dashboard.contains("3 artifacts total: 2 active, 1 draft"));
    let rules = fixture.json(&["fpf", "rules", "--json"]);
    assert_eq!(rules["_next_action"], "forgeplan fpf check PRD-014");
    assert_eq!(read_state(&fixture.workspace()), before);
    assert_eq!(
        std::fs::read_to_string(shadow.join("must-not-be-opened")).unwrap(),
        "not a legacy table"
    );
}

#[test]
fn migrated_get_resolves_collision_inventory_without_listing_it() {
    let fixture = Fixture::new();
    fixture.seed_graph();
    fixture.migrate();
    fixture.write("prds/PRD-020-alpha.md", "---\nid: PRD-020\nkind: prd\nstatus: draft\ntitle: Alpha\nslug: prd-contested\n---\nAlpha body\n");
    fixture.write("prds/PRD-021-beta.md", "---\nid: PRD-021\nkind: prd\nstatus: draft\ntitle: Beta\nslug: prd-contested\n---\nBeta body\n");
    for selector in ["prd-contested", "PRD-020", "PRD-021"] {
        let output = fixture.run(&["get", selector]);
        assert_eq!(output.status.code(), Some(4));
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("Alpha") && error.contains("Beta"), "{error}");
        assert!(error.contains("PRD-020-alpha.md") && error.contains("PRD-021-beta.md"), "{error}");
    }
    let unique = fixture.json(&["get", "rfc-session-store", "--json"]);
    assert!(unique["body"].as_str().unwrap().contains("Persist sessions"));
    assert!(!fixture.run(&["list", "--json"]).status.success());
    assert!(!fixture.run(&["search", "session", "--json"]).status.success());
    fixture.write("prds/broken.md", "---\nid: [unclosed\n");
    let output = fixture.run(&["get", "rfc-session-store"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("incomplete"));
}

#[test]
fn migrated_get_rejects_corrupt_managed_index_for_unique_and_missing_selectors() {
    use forgeplan_core::constellation::{manifest::TABLE_ARTIFACTS_V1, storage};
    for collision in [false, true] {
        for damage in ["current", "manifest", "table"] {
            let fixture = Fixture::new();
            fixture.seed_graph();
            fixture.migrate();
            if collision {
                fixture.write("prds/PRD-020-alpha.md", "---\nid: PRD-020\nkind: prd\nstatus: draft\ntitle: Alpha\nslug: prd-contested\n---\nAlpha\n");
                fixture.write("prds/PRD-021-beta.md", "---\nid: PRD-021\nkind: prd\nstatus: draft\ntitle: Beta\nslug: prd-contested\n---\nBeta\n");
            }
            let before = fixture.json(&["get", "rfc-session-store", "--json"]);
            assert!(before["body"].as_str().unwrap().contains("Persist sessions"));
            let root = fixture.workspace().join("lance");
            let descriptor = storage::read_current(&root).unwrap().unwrap();
            match damage {
                "current" => std::fs::write(root.join("CURRENT.json"), b"{broken").unwrap(),
                "manifest" => std::fs::write(root.join(descriptor.manifest_path), b"{broken").unwrap(),
                "table" => std::fs::remove_dir_all(root.join(format!("{TABLE_ARTIFACTS_V1}.lance"))).unwrap(),
                _ => unreachable!(),
            }
            for selector in ["rfc-session-store", "PRD-999"] {
                let result = fixture.run(&["get", selector]);
                assert!(!result.status.success(), "{damage}, collision={collision}, {selector}");
                assert_ne!(result.status.code(), Some(3), "{damage}, collision={collision}, {selector}");
            }
            if collision {
                let result = fixture.run(&["get", "PRD-020"]);
                assert_eq!(result.status.code(), Some(4));
                let error = String::from_utf8_lossy(&result.stderr);
                assert!(error.contains("Alpha") && error.contains("Beta"), "{error}");
            }
        }
    }
}

#[test]
fn scoped_read_routes_keep_collision_diagnostics_and_unique_payloads() {
    let fixture = Fixture::new();
    fixture.seed_graph();
    fixture.migrate();
    fixture.write("prds/PRD-020-alpha.md", "---\nid: PRD-020\nkind: prd\nstatus: draft\ntitle: Alpha\nslug: prd-contested\n---\nAlpha\n");
    fixture.write("prds/PRD-021-beta.md", "---\nid: PRD-021\nkind: prd\nstatus: draft\ntitle: Beta\nslug: prd-contested\n---\nBeta\n");
    fixture.write("notes/NOTE-001-unique.md", "---\nid: NOTE-001\nkind: note\nstatus: draft\ntitle: Unique\nslug: note-unique\ndepth: tactical\n---\n\nA complete note.\n");
    let session_path = fixture.workspace().join("session.yaml");
    let session_before = std::fs::read(&session_path).ok();
    fixture.write("prds/PRD-030-one.md", "---\nid: PRD-030\nkind: prd\nstatus: draft\ntitle: One\nslug: prd-one\n---\nOne\n");
    fixture.write("prds/PRD-030-two.md", "---\nid: PRD-030\nkind: prd\nstatus: draft\ntitle: Two\nslug: prd-two\n---\nTwo\n");
    for command in ["phase", "validate"] {
        let result = fixture.run(&[command, "PRD-020"]);
        assert_eq!(result.status.code(), Some(4), "{command}: {}", String::from_utf8_lossy(&result.stderr));
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(error.contains("Alpha") && error.contains("Beta"), "{command}: {error}");
        let ambiguous = fixture.run(&[command, "PRD-030"]);
        assert_eq!(ambiguous.status.code(), Some(5), "{command}: {}", String::from_utf8_lossy(&ambiguous.stderr));
        let stderr = String::from_utf8_lossy(&ambiguous.stderr);
        assert!(stderr.contains("One") && stderr.contains("Two"), "{command}: {stderr}");
    }
    let phase = fixture.json(&["phase", "rfc-session-store", "--json"]);
    assert_eq!(phase["canonical_key"], "rfc-session-store");
    let validate = fixture.run(&["validate", "rfc-session-store", "--json"]);
    let payload: serde_json::Value = serde_json::from_slice(&validate.stdout).unwrap();
    assert_eq!(payload["results"][0]["canonical_key"], "rfc-session-store");
    let valid = fixture.run(&["validate", "note-unique", "--json"]);
    let valid_json: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(valid_json["results"][0]["passed"], true);
    assert_eq!(std::fs::read(session_path).ok(), session_before);
    assert!(!fixture.workspace().join("state/note-unique.yaml").exists());
    assert!(!fixture.run(&["validate", "--json"]).status.success());
}

#[test]
fn selected_phase_and_validate_reject_invalid_integrity_config() {
    let fixture = Fixture::new();
    fixture.seed_graph();
    fixture.migrate();
    let workspace = fixture.workspace();
    let mut config = forgeplan_core::workspace::load_config(&workspace).unwrap();
    config.integrity.duplicate_threshold = 1.5;
    std::fs::write(workspace.join("config.yaml"), serde_yaml::to_string(&config).unwrap()).unwrap();
    for command in ["phase", "validate"] {
        let output = fixture.run(&[command, "rfc-session-store", "--json"]);
        assert!(!output.status.success(), "{command} accepted invalid integrity config");
        assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate_threshold"),
            "{command}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
fn phase_and_validate_selector_integrity_and_refusals_do_not_write_sidecars() {
    use forgeplan_core::constellation::{manifest::TABLE_ARTIFACTS_V1, storage};
    for damage in ["current", "manifest", "table"] {
        let fixture = Fixture::new();
        fixture.seed_graph();
        fixture.migrate();
        let workspace = fixture.workspace();
        fixture.write("prds/PRD-020-alpha.md", "---\nid: PRD-020\nkind: prd\nstatus: draft\ntitle: Alpha\nslug: prd-contested\n---\nAlpha\n");
        fixture.write("prds/PRD-021-beta.md", "---\nid: PRD-021\nkind: prd\nstatus: draft\ntitle: Beta\nslug: prd-contested\n---\nBeta\n");
        let state_dir = workspace.join("state");
        let session_path = workspace.join("session.yaml");
        let session_before = std::fs::read(&session_path).ok();
        for command in ["phase", "validate"] {
            let missing = fixture.run(&[command, "PRD-999"]);
            assert_eq!(missing.status.code(), Some(3), "{command}: {}", String::from_utf8_lossy(&missing.stderr));
            let collision = fixture.run(&[command, "PRD-020"]);
            assert_eq!(collision.status.code(), Some(4), "{command}: {}", String::from_utf8_lossy(&collision.stderr));
        }
        assert!(!state_dir.join("PRD-020.yaml").exists());
        assert!(!state_dir.join("prd-contested.yaml").exists());
        assert_eq!(std::fs::read(&session_path).ok(), session_before);
        let root = workspace.join("lance");
        let descriptor = storage::read_current(&root).unwrap().unwrap();
        match damage {
            "current" => std::fs::write(root.join("CURRENT.json"), b"{broken").unwrap(),
            "manifest" => std::fs::write(root.join(descriptor.manifest_path), b"{broken").unwrap(),
            "table" => std::fs::remove_dir_all(root.join(format!("{TABLE_ARTIFACTS_V1}.lance"))).unwrap(),
            _ => unreachable!(),
        }
        for command in ["phase", "validate"] {
            for selector in ["rfc-session-store", "PRD-999"] {
                let output = fixture.run(&[command, selector]);
                assert!(!output.status.success() && output.status.code() != Some(3),
                    "{damage}, {command}, {selector}: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        assert!(!state_dir.join("rfc-session-store.yaml").exists());
        assert_eq!(std::fs::read(session_path).ok(), session_before);
    }
}
