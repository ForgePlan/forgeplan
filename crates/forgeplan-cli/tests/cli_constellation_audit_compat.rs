//! Existing audit events survive the index cutover and every fresh CLI process.
//! Commands use an isolated HOME and empty model cache; no model/provider runs.

use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    home: PathBuf,
    models: PathBuf,
    migrated: bool,
    table_baseline: Vec<String>,
}

impl Fixture {
    fn new(migrated: bool, notes: &[(&str, &str, &str, &str)]) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        let home = temp.path().join("home");
        let models = temp.path().join("models");
        for path in [&root, &home, &models] {
            std::fs::create_dir_all(path).unwrap();
        }
        let mut fixture = Self {
            _temp: temp,
            root,
            home,
            models,
            migrated,
            table_baseline: Vec::new(),
        };
        fixture.ok(&["init", "-y"]);
        for (id, title, status, body) in notes {
            fixture.write_note(id, title, status, body);
        }
        fixture.ok(&["reindex"]);
        let old_history = fixture.history();
        if migrated {
            fixture.ok(&["index"]);
            assert!(fixture.workspace().join("lance/CURRENT.json").exists());
            assert_eq!(
                fixture.history(),
                old_history,
                "migration must preserve old rows byte-for-value"
            );
        }
        fixture.table_baseline = fixture.tables();
        fixture
    }

    fn workspace(&self) -> PathBuf {
        self.root.join(".forgeplan")
    }

    fn command(&self) -> Command {
        let mut command = Command::cargo_bin("forgeplan").unwrap();
        command
            .current_dir(&self.root)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("XDG_DATA_HOME", &self.home)
            .env("FORGEPLAN_MODEL_CACHE", &self.models)
            .env("HF_HUB_OFFLINE", "1");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }

    fn ok(&self, args: &[&str]) -> Output {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "forgeplan {args:?}: stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn json(&self, args: &[&str]) -> Value {
        let output = self.ok(args);
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "forgeplan {args:?}: invalid JSON {error}: {}",
                String::from_utf8_lossy(&output.stdout)
            )
        })
    }

    fn history(&self) -> Vec<Value> {
        self.json(&["log", "--limit", "1000", "--json"])["entries"]
            .as_array()
            .unwrap()
            .clone()
    }

    fn source_history(&self, source: &str) -> Vec<Value> {
        self.json(&["log", "--source", source, "--limit", "1000", "--json"])["entries"]
            .as_array()
            .unwrap()
            .clone()
    }

    fn note_path(&self, id: &str, title: &str) -> PathBuf {
        self.workspace().join("notes").join(format!(
            "{id}-{}.md",
            forgeplan_core::artifact::types::slugify(title)
        ))
    }

    fn write_note(&self, id: &str, title: &str, status: &str, body: &str) -> PathBuf {
        let path = self.note_path(id, title);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            format!(
                "---\nid: {id}\nkind: note\nstatus: {status}\ntitle: {title}\ndepth: tactical\n---\n\n## Summary\n\n{body}\n"
            ),
        )
        .unwrap();
        path
    }

    fn tables(&self) -> Vec<String> {
        let mut names = std::fs::read_dir(self.workspace().join("lance"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    fn assert_owner_unchanged(&self) {
        if self.migrated {
            for table in ["artifacts.lance", "evidence.lance", "relations.lance"] {
                assert!(
                    !self.workspace().join("lance").join(table).exists()
                        || self.table_baseline.iter().any(|name| name == table),
                    "history mutation created retired table {table}"
                );
            }
            assert!(self.workspace().join("lance/CURRENT.json").exists());
        }
    }

    fn git(&self, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .env("HOME", &self.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Audit test")
            .env("GIT_AUTHOR_EMAIL", "audit@example.invalid")
            .env("GIT_COMMITTER_NAME", "Audit test")
            .env("GIT_COMMITTER_EMAIL", "audit@example.invalid")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }
}

fn delta(before: &[Value], after: &[Value]) -> Vec<Value> {
    for old in before {
        assert!(after.contains(old), "existing audit row disappeared: {old}");
    }
    after
        .iter()
        .filter(|entry| !before.contains(entry))
        .cloned()
        .collect()
}

fn event(
    id: &str,
    action: &str,
    field: Option<&str>,
    old: Option<&str>,
    new: Option<&str>,
    source: &str,
    commit: Option<&str>,
) -> Value {
    json!({
        "artifact_id": id,
        "action": action,
        "field": field,
        "old_value": old,
        "new_value": new,
        "source": source,
        "commit_hash": commit,
    })
}

fn assert_events(actual: &[Value], expected: &[Value]) {
    let mut actual = actual
        .iter()
        .map(|entry| {
            let mut entry = entry.as_object().unwrap().clone();
            assert_eq!(
                entry.len(),
                8,
                "the established history schema has eight columns"
            );
            let timestamp = entry.remove("timestamp").unwrap();
            chrono::DateTime::parse_from_rfc3339(timestamp.as_str().unwrap()).unwrap();
            Value::Object(entry)
        })
        .collect::<Vec<_>>();
    let mut expected = expected.to_vec();
    actual.sort_by_key(Value::to_string);
    expected.sort_by_key(Value::to_string);
    assert_eq!(actual, expected);
}

#[test]
fn create_keeps_old_history_and_appends_once_in_both_layouts() {
    for migrated in [false, true] {
        let fixture = Fixture::new(
            migrated,
            &[("NOTE-001", "Audit seed", "draft", "seed bytes")],
        );
        let before = fixture.history();
        fixture.ok(&["new", "note", "New audit entry", "--allow-duplicate"]);
        let after = fixture.history();
        assert_eq!(after.len(), before.len() + 1);
        assert_events(
            &delta(&before, &after),
            &[event("NOTE-002", "create", None, None, None, "cli", None)],
        );
        assert_eq!(
            fixture.history(),
            after,
            "a new log process must reopen the same rows"
        );
        assert_eq!(
            fixture.json(&["log", "NOTE-002", "--source", "cli", "--json"])["entries"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(
            fixture.json(&["get", "NOTE-002", "--json"])["body"]
                .as_str()
                .unwrap()
                .contains("New audit entry")
        );
        let failed = fixture.run(&["new", "note", ""]);
        assert!(!failed.status.success());
        assert_eq!(
            fixture.history(),
            after,
            "a rejected create has no success event"
        );
        fixture.assert_owner_unchanged();
    }
}

#[test]
fn update_preserves_three_field_events_and_depth_only_silence() {
    for migrated in [false, true] {
        let fixture = Fixture::new(
            migrated,
            &[("NOTE-001", "Audit seed", "draft", "seed bytes")],
        );
        let before = fixture.history();
        fixture.ok(&[
            "update",
            "NOTE-001",
            "--status",
            "stale",
            "--title",
            "Revised audit seed",
            "--body",
            "## Summary\n\nchanged body bytes",
        ]);
        let after = fixture.history();
        assert_eq!(after.len(), before.len() + 3);
        assert_events(
            &delta(&before, &after),
            &[
                event(
                    "NOTE-001",
                    "update",
                    Some("status"),
                    Some("draft"),
                    Some("stale"),
                    "cli",
                    None,
                ),
                event(
                    "NOTE-001",
                    "update",
                    Some("title"),
                    Some("Audit seed"),
                    Some("Revised audit seed"),
                    "cli",
                    None,
                ),
                event("NOTE-001", "update", Some("body"), None, None, "cli", None),
            ],
        );
        let artifact = fixture.json(&["get", "NOTE-001", "--json"]);
        assert_eq!(artifact["status"], "stale");
        assert_eq!(artifact["title"], "Revised audit seed");
        assert!(
            artifact["body"]
                .as_str()
                .unwrap()
                .contains("changed body bytes")
        );
        fixture.ok(&["update", "NOTE-001", "--depth", "standard"]);
        assert_eq!(
            fixture.history(),
            after,
            "depth-only update has no established event"
        );
        for args in [
            vec!["update", "NOTE-001", "--status", "active"],
            vec!["update", "NOTE-001", "--title", ""],
            vec!["update", "NOTE-999", "--body", "missing artifact"],
        ] {
            assert!(!fixture.run(&args).status.success());
            assert_eq!(fixture.history(), after);
        }
        fixture.assert_owner_unchanged();
    }
}

#[test]
fn lifecycle_verbs_keep_their_established_status_events_and_reopen_pair() {
    for migrated in [false, true] {
        let fixture = Fixture::new(
            migrated,
            &[
                ("NOTE-001", "Activation subject", "draft", "activation body"),
                (
                    "NOTE-002",
                    "Replacement subject",
                    "draft",
                    "replacement body",
                ),
                ("NOTE-003", "Renewal subject", "stale", "renewal body"),
                (
                    "NOTE-004",
                    "Deprecation subject",
                    "active",
                    "deprecation body",
                ),
            ],
        );
        let before = fixture.history();
        fixture.ok(&["activate", "NOTE-001", "--force"]);
        fixture.ok(&["supersede", "NOTE-001", "--by", "NOTE-002"]);
        fixture.ok(&["deprecate", "NOTE-004", "--reason", "Replaced in this test"]);
        fixture.ok(&[
            "renew",
            "NOTE-003",
            "--reason",
            "Still valid",
            "--until",
            "2070-01-01",
        ]);
        let statuses = fixture.history();
        assert_events(
            &delta(&before, &statuses),
            &[
                event(
                    "NOTE-001",
                    "update",
                    Some("status"),
                    Some("draft"),
                    Some("active"),
                    "cli",
                    None,
                ),
                event(
                    "NOTE-001",
                    "update",
                    Some("status"),
                    Some("active"),
                    Some("superseded"),
                    "cli",
                    None,
                ),
                event(
                    "NOTE-004",
                    "update",
                    Some("status"),
                    Some("active"),
                    Some("deprecated"),
                    "cli",
                    None,
                ),
                event(
                    "NOTE-003",
                    "update",
                    Some("status"),
                    Some("stale"),
                    Some("active"),
                    "cli",
                    None,
                ),
            ],
        );
        assert!(
            !fixture
                .run(&["reopen", "NOTE-002", "--reason", "Drafts cannot reopen"])
                .status
                .success()
        );
        assert_eq!(fixture.history(), statuses);
        fixture.ok(&["reopen", "NOTE-003", "--reason", "New revision needed"]);
        let reopened = fixture.history();
        assert_eq!(reopened.len(), statuses.len() + 2);
        assert_events(
            &delta(&statuses, &reopened),
            &[
                event(
                    "NOTE-003",
                    "update",
                    Some("status"),
                    Some("active"),
                    Some("deprecated"),
                    "cli",
                    None,
                ),
                event(
                    "NOTE-005",
                    "create",
                    Some("status"),
                    None,
                    Some("draft"),
                    "cli",
                    None,
                ),
            ],
        );
        assert_eq!(
            fixture.json(&["get", "NOTE-003", "--json"])["status"],
            "deprecated"
        );
        assert_eq!(
            fixture.json(&["get", "NOTE-005", "--json"])["status"],
            "draft"
        );
        fixture.assert_owner_unchanged();
    }
}

#[test]
fn link_replace_unchanged_and_unlink_keep_exact_event_cardinality() {
    for migrated in [false, true] {
        let fixture = Fixture::new(
            migrated,
            &[
                ("NOTE-001", "Link source", "draft", "source body"),
                ("NOTE-002", "Link target", "draft", "target body"),
            ],
        );
        let before = fixture.history();
        fixture.ok(&["link", "NOTE-001", "NOTE-002", "--relation", "informs"]);
        let linked = fixture.history();
        assert_events(
            &delta(&before, &linked),
            &[event(
                "NOTE-001",
                "link",
                Some("relation"),
                None,
                Some("NOTE-002:informs"),
                "cli",
                None,
            )],
        );
        assert!(
            !fixture
                .run(&["link", "NOTE-001", "NOTE-002", "--relation", "informs"])
                .status
                .success()
        );
        assert_eq!(fixture.history(), linked);
        fixture.ok(&[
            "link",
            "NOTE-001",
            "NOTE-002",
            "--relation",
            "informs",
            "--replace",
        ]);
        let unchanged = fixture.history();
        assert_eq!(
            unchanged.len(),
            linked.len() + 1,
            "Unchanged replacement still has its historical link event"
        );
        assert_events(
            &delta(&linked, &unchanged),
            &[event(
                "NOTE-001",
                "link",
                Some("relation"),
                None,
                Some("NOTE-002:informs"),
                "cli",
                None,
            )],
        );
        fixture.ok(&[
            "link",
            "NOTE-001",
            "NOTE-002",
            "--relation",
            "based_on",
            "--replace",
        ]);
        let replaced = fixture.history();
        assert_events(
            &delta(&unchanged, &replaced),
            &[event(
                "NOTE-001",
                "link",
                Some("relation"),
                None,
                Some("NOTE-002:based_on"),
                "cli",
                None,
            )],
        );
        fixture.ok(&["unlink", "NOTE-001", "NOTE-002", "--relation", "based_on"]);
        let unlinked = fixture.history();
        assert_events(
            &delta(&replaced, &unlinked),
            &[event(
                "NOTE-001",
                "unlink",
                Some("relation"),
                Some("NOTE-002:based_on"),
                None,
                "cli",
                None,
            )],
        );
        assert!(
            !fixture
                .run(&["unlink", "NOTE-001", "NOTE-002", "--relation", "based_on"])
                .status
                .success()
        );
        assert_eq!(fixture.history(), unlinked);
        fixture.assert_owner_unchanged();
    }
}

#[test]
fn ingest_create_update_skip_and_dry_run_preserve_the_original_events() {
    for migrated in [false, true] {
        let fixture = Fixture::new(
            migrated,
            &[("NOTE-001", "Audit seed", "draft", "seed bytes")],
        );
        let mapping = fixture.root.join("mapping.yaml");
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/mappings/c4-to-forge-test.yaml"),
            &mapping,
        )
        .unwrap();
        let source = fixture.root.join("c4-mini/components/audit.md");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        let input = "---\nkind: component\nname: Imported audit component\n---\n\n# Component\n\n## Purpose\n\nOriginal source purpose.\n";
        std::fs::write(&source, input).unwrap();
        let mapping = mapping.to_str().unwrap();
        let before = fixture.history();
        let draft = fixture.json(&[
            "ingest",
            "--mapping",
            mapping,
            "--source",
            "c4-mini",
            "--dry-run",
            "--json",
        ]);
        assert_eq!(draft["drafts"].as_array().unwrap().len(), 1);
        assert_eq!(fixture.history(), before);
        let created = fixture.json(&[
            "ingest",
            "--mapping",
            mapping,
            "--source",
            "c4-mini",
            "--json",
        ]);
        let id = created["written"][0]["id"].as_str().unwrap();
        let initial = fixture.json(&["get", id, "--json"]);
        assert!(
            initial["body"]
                .as_str()
                .unwrap()
                .contains("Original source purpose.")
        );
        let after_create = fixture.history();
        assert_events(
            &delta(&before, &after_create),
            &[event(id, "ingest_create", None, None, None, "cli", None)],
        );
        fixture.ok(&["ingest", "--mapping", mapping, "--source", "c4-mini"]);
        assert_eq!(
            fixture.history(),
            after_create,
            "hash match skips must not append"
        );
        std::fs::write(
            &source,
            input.replace("Original source purpose.", "Changed source purpose."),
        )
        .unwrap();
        fixture.ok(&["ingest", "--mapping", mapping, "--source", "c4-mini"]);
        assert_eq!(
            fixture.history(),
            after_create,
            "missing --update is an event-free skip"
        );
        fixture.ok(&[
            "ingest",
            "--mapping",
            mapping,
            "--source",
            "c4-mini",
            "--update",
        ]);
        let after_update = fixture.history();
        assert_events(
            &delta(&after_create, &after_update),
            &[event(id, "ingest_update", None, None, None, "cli", None)],
        );
        let updated = fixture.json(&["get", id, "--json"]);
        let updated_body = updated["body"].as_str().unwrap();
        assert!(updated_body.contains("Changed source purpose."));
        assert!(!updated_body.contains("Original source purpose."));
        assert!(!updated_body.contains("(no purpose section)"));
        assert!(
            !fixture
                .run(&["ingest", "--mapping", "missing.yaml", "--source", "c4-mini"])
                .status
                .success()
        );
        assert_eq!(fixture.history(), after_update);
        fixture.assert_owner_unchanged();
    }
}

#[test]
fn git_sync_keeps_identity_when_the_title_and_body_move_to_a_new_filename() {
    for migrated in [false, true] {
        let fixture = Fixture::new(
            migrated,
            &[("NOTE-001", "Old title", "draft", "original body bytes")],
        );
        fixture.git(&["init", "-q"]);
        fixture.git(&["config", "diff.renames", "true"]);
        fixture.git(&["add", "--", ".forgeplan/notes"]);
        fixture.git(&["commit", "-q", "-m", "baseline source"]);
        let baseline = fixture.git(&["rev-parse", "HEAD"]);
        let before = fixture.history();

        let renamed = fixture.write_note("NOTE-001", "New title", "draft", "changed body bytes");
        std::fs::remove_file(fixture.note_path("NOTE-001", "Old title")).unwrap();
        let source_bytes = std::fs::read(&renamed).unwrap();
        fixture.git(&["add", "-A", "--", ".forgeplan/notes"]);
        fixture.git(&["commit", "-q", "-m", "move and edit source"]);
        let commit = fixture.git(&["rev-parse", "--short=7", "HEAD"]);
        fixture.ok(&["git-sync", "--since", &baseline]);

        let after = fixture.history();
        assert_events(
            &delta(&before, &after),
            &[event(
                "NOTE-001",
                "update",
                None,
                None,
                None,
                "git_sync",
                Some(&commit),
            )],
        );
        let fetched = fixture.json(&["get", "NOTE-001", "--json"]);
        assert_eq!(fetched["title"], "New title");
        let body = fetched["body"].as_str().unwrap();
        assert!(body.contains("changed body bytes"));
        assert!(!body.contains("original body bytes"));
        assert_eq!(std::fs::read(&renamed).unwrap(), source_bytes);
        fixture.ok(&["git-sync", "--since", &baseline]);
        assert_eq!(
            fixture.history(),
            after,
            "replaying the move must be a no-op"
        );
        fixture.assert_owner_unchanged();
    }
}

#[test]
fn git_sync_audits_real_old_to_current_changes_and_preserves_commit_hashes() {
    for migrated in [false, true] {
        let fixture = Fixture::new(
            migrated,
            &[
                ("NOTE-001", "Git update", "draft", "old update bytes"),
                ("NOTE-002", "Git delete", "draft", "old deleted bytes"),
                ("NOTE-004", "Metadata only", "draft", "same body bytes"),
            ],
        );
        fixture.git(&["init", "-q"]);
        fixture.git(&["add", "--", ".forgeplan/notes"]);
        fixture.git(&["commit", "-q", "-m", "baseline source"]);
        let baseline = fixture.git(&["rev-parse", "HEAD"]);
        let before = fixture.history();
        fixture.write_note("NOTE-001", "Git update", "draft", "new update bytes");
        std::fs::remove_file(fixture.note_path("NOTE-002", "Git delete")).unwrap();
        fixture.write_note("NOTE-003", "Git create", "draft", "new created bytes");
        fixture.write_note("NOTE-004", "Metadata only", "stale", "same body bytes");
        fixture.git(&["add", "-A", "--", ".forgeplan/notes"]);
        fixture.git(&[
            "commit",
            "-q",
            "-m",
            "edit add delete and metadata-only source",
        ]);
        let commit = fixture.git(&["rev-parse", "--short=7", "HEAD"]);
        fixture.ok(&["git-sync", "--since", &baseline]);
        let after = fixture.history();
        assert_eq!(after.len(), before.len() + 3);
        assert_events(
            &delta(&before, &after),
            &[
                event(
                    "NOTE-001",
                    "update",
                    None,
                    None,
                    None,
                    "git_sync",
                    Some(&commit),
                ),
                event(
                    "NOTE-002",
                    "delete",
                    None,
                    None,
                    None,
                    "git_sync",
                    Some(&commit),
                ),
                event(
                    "NOTE-003",
                    "create",
                    None,
                    None,
                    None,
                    "git_sync",
                    Some(&commit),
                ),
            ],
        );
        assert_eq!(fixture.source_history("git_sync").len(), 3);
        assert!(
            fixture.json(&["get", "NOTE-001", "--json"])["body"]
                .as_str()
                .unwrap()
                .contains("new update bytes")
        );
        fixture.ok(&["git-sync", "--since", &baseline]);
        fixture.ok(&["git-sync", "--since", "HEAD"]);
        assert_eq!(
            fixture.history(),
            after,
            "already-synced body and absent deleted id are no-ops"
        );
        fixture.assert_owner_unchanged();
    }
}

#[test]
fn reindex_audits_published_before_images_not_two_live_captures() {
    for migrated in [false, true] {
        let fixture = Fixture::new(
            migrated,
            &[
                ("NOTE-001", "Reindex update", "draft", "old update bytes"),
                ("NOTE-002", "Reindex delete", "draft", "old deleted bytes"),
                ("NOTE-004", "Whitespace only", "draft", "same body bytes"),
            ],
        );
        let before = fixture.history();
        fixture.write_note("NOTE-001", "Reindex update", "draft", "new update bytes");
        std::fs::remove_file(fixture.note_path("NOTE-002", "Reindex delete")).unwrap();
        fixture.write_note("NOTE-003", "Reindex create", "draft", "new created bytes");
        fixture.write_note(
            "NOTE-004",
            "Whitespace only",
            "draft",
            "same body bytes\n\n  ",
        );
        fixture.ok(&["reindex"]);
        let after = fixture.history();
        assert_eq!(after.len(), before.len() + 3);
        assert_events(
            &delta(&before, &after),
            &[
                event("NOTE-001", "update", None, None, None, "reindex", None),
                event("NOTE-002", "delete", None, None, None, "reindex", None),
                event("NOTE-003", "create", None, None, None, "reindex", None),
            ],
        );
        assert!(
            fixture.json(&["get", "NOTE-001", "--json"])["body"]
                .as_str()
                .unwrap()
                .contains("new update bytes")
        );
        fixture.ok(&["reindex"]);
        assert_eq!(
            fixture.history(),
            after,
            "unchanged or trim-only bodies must not append"
        );
        fixture.assert_owner_unchanged();
    }
}

#[test]
fn history_failure_warns_but_does_not_undo_a_successful_canonical_create() {
    for migrated in [false, true] {
        let fixture = Fixture::new(
            migrated,
            &[("NOTE-001", "Audit seed", "draft", "seed bytes")],
        );
        // Keep the manifest/schema readable so both legacy and auxiliary
        // opens succeed, but make every new data-file append fail.
        let data = fixture.workspace().join("lance/change_log.lance/data");
        assert!(
            data.is_dir(),
            "the seeded audit rows must own real Lance data files"
        );
        std::fs::remove_dir_all(&data).unwrap();
        std::fs::write(&data, b"unwritable history data path").unwrap();
        let output = fixture.ok(&[
            "new",
            "note",
            "Canonical write survives audit failure",
            "--allow-duplicate",
        ]);
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("  Warning: changelog write failed for NOTE-002: ")
        );
        assert!(
            fixture.json(&["get", "NOTE-002", "--json"])["body"]
                .as_str()
                .unwrap()
                .contains("Canonical write survives audit failure")
        );
        assert_eq!(
            std::fs::read(&data).unwrap(),
            b"unwritable history data path"
        );
        assert!(
            !fixture.run(&["log", "--json"]).status.success(),
            "corrupt history is not successful empty history"
        );
        fixture.assert_owner_unchanged();
    }
}

#[test]
fn failed_managed_maintenance_capture_does_not_append_success_events() {
    let fixture = Fixture::new(true, &[("NOTE-001", "Audit seed", "draft", "seed bytes")]);
    let before = fixture.history();
    fixture.write_note(
        "NOTE-002",
        "Unpublished addition",
        "draft",
        "real but not yet indexed",
    );
    let current = fixture.workspace().join("lance/CURRENT.json");
    std::fs::write(&current, b"not a descriptor").unwrap();
    assert!(!fixture.run(&["reindex"]).status.success());
    assert_eq!(fixture.history(), before);
    for table in ["artifacts.lance", "relations.lance"] {
        assert!(!fixture.workspace().join("lance").join(table).exists());
    }
}

#[test]
fn title_only_filename_rename_is_not_a_delete_or_body_change_event() {
    for migrated in [false, true] {
        let fixture = Fixture::new(
            migrated,
            &[("NOTE-001", "Old title", "draft", "unchanged body bytes")],
        );
        fixture.git(&["init", "-q"]);
        fixture.git(&["config", "diff.renames", "true"]);
        fixture.git(&["add", "--", ".forgeplan/notes"]);
        fixture.git(&["commit", "-q", "-m", "baseline source"]);
        let baseline = fixture.git(&["rev-parse", "HEAD"]);
        let before = fixture.history();
        fixture.write_note("NOTE-001", "New title", "draft", "unchanged body bytes");
        std::fs::remove_file(fixture.note_path("NOTE-001", "Old title")).unwrap();
        fixture.git(&["add", "-A", "--", ".forgeplan/notes"]);
        fixture.git(&["commit", "-q", "-m", "move title without body edit"]);
        fixture.ok(&["git-sync", "--since", &baseline]);
        assert_eq!(
            fixture.history(),
            before,
            "a move alone is not delete/create"
        );
        fixture.ok(&["reindex"]);
        assert_eq!(fixture.history(), before);
        assert!(fixture.note_path("NOTE-001", "New title").exists());
        fixture.assert_owner_unchanged();
    }
}

#[test]
fn failed_git_sync_before_image_read_does_not_append_success_events() {
    let fixture = Fixture::new(
        true,
        &[("NOTE-001", "Audit seed", "draft", "old committed bytes")],
    );
    fixture.git(&["init", "-q"]);
    fixture.git(&["add", "--", ".forgeplan/notes"]);
    fixture.git(&["commit", "-q", "-m", "baseline source"]);
    let baseline = fixture.git(&["rev-parse", "HEAD"]);
    let before = fixture.history();
    fixture.write_note("NOTE-001", "Audit seed", "draft", "new committed bytes");
    fixture.git(&["add", "--", ".forgeplan/notes"]);
    fixture.git(&["commit", "-q", "-m", "changed source"]);
    std::fs::write(
        fixture.workspace().join("lance/CURRENT.json"),
        b"not a descriptor",
    )
    .unwrap();
    assert!(
        !fixture
            .run(&["git-sync", "--since", &baseline])
            .status
            .success()
    );
    assert_eq!(fixture.history(), before);
    for table in ["artifacts.lance", "relations.lance"] {
        assert!(!fixture.workspace().join("lance").join(table).exists());
    }
}

#[test]
fn reindex_history_before_image_includes_artifacts_beyond_the_inventory_page_boundary() {
    let owned = (1..=101)
        .map(|number| {
            (
                format!("NOTE-{number:03}"),
                format!("Bulk audit {number:03}"),
                format!("Original body {number:03}."),
            )
        })
        .collect::<Vec<_>>();
    let notes = owned
        .iter()
        .map(|(id, title, body)| (id.as_str(), title.as_str(), "draft", body.as_str()))
        .collect::<Vec<_>>();
    let fixture = Fixture::new(true, &notes);
    let before = fixture.history();
    fixture.write_note(
        "NOTE-101",
        "Bulk audit 101",
        "draft",
        "Changed last published body.",
    );
    std::fs::remove_file(fixture.note_path("NOTE-100", "Bulk audit 100")).unwrap();
    fixture.write_note(
        "NOTE-102",
        "Bulk audit 102",
        "draft",
        "New unpublished body.",
    );

    fixture.ok(&["reindex"]);
    let after = fixture.history();
    assert_eq!(after.len(), before.len() + 3);
    assert_events(
        &delta(&before, &after),
        &[
            event("NOTE-100", "delete", None, None, None, "reindex", None),
            event("NOTE-101", "update", None, None, None, "reindex", None),
            event("NOTE-102", "create", None, None, None, "reindex", None),
        ],
    );
    fixture.ok(&["reindex"]);
    assert_eq!(fixture.history(), after);
    fixture.assert_owner_unchanged();
}
