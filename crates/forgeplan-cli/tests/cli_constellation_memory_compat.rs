//! Memory artifacts keep the same CLI contract on both index layouts.
//! Reads exercise the binary and real canonical files, never a legacy-store
//! fallback on a Constellation-owned workspace.

use std::path::PathBuf;
use std::process::Output;

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::TempDir;

const AUTHOR: &str = "Ada Lovelace <ada@example.invalid>";
const ALPHA: &str = "MEM-001";
const BETA: &str = "MEM-002";
const GAMMA: &str = "MEM-003";

struct Workspace {
    _temp: TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Workspace {
    fn new(migrated: bool) -> Self {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("project");
        let home = temp.path().join("home");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        let workspace = Self {
            _temp: temp,
            root,
            home,
        };
        workspace.ok(&["init", "-y"]);
        if migrated {
            workspace.migrate();
        }
        workspace
    }

    fn output(&self, args: &[&str]) -> Output {
        Command::cargo_bin("forgeplan")
            .unwrap()
            .args(args)
            .current_dir(&self.root)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("XDG_DATA_HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.home)
            .env("XDG_CACHE_HOME", &self.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Ada Lovelace")
            .env("GIT_AUTHOR_EMAIL", "ada@example.invalid")
            .env("NO_COLOR", "1")
            .env("CLICOLOR", "0")
            .output()
            .unwrap()
    }

    fn ok(&self, args: &[&str]) -> String {
        let output = self.output(args);
        assert!(
            output.status.success(),
            "forgeplan {args:?} failed: {}\nstdout: {}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn json(&self, args: &[&str]) -> Value {
        let output = self.ok(args);
        serde_json::from_str(&output).unwrap_or_else(|error| {
            panic!("forgeplan {args:?} emitted invalid JSON: {error}: {output}")
        })
    }

    fn ws(&self) -> PathBuf {
        self.root.join(".forgeplan")
    }

    fn migrate(&self) {
        self.ok(&["index"]);
        assert!(self.ws().join("lance/CURRENT.json").is_file());
        assert!(!self.ws().join("lance/artifacts.lance").exists());
        assert!(!self.ws().join("lance/relations.lance").exists());
    }

    fn write(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.ws().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
        path
    }

    fn populated(migrated: bool) -> Self {
        let workspace = Self::new(false);
        workspace.ok(&[
            "remember",
            "Alpha PostgreSQL BodyNeedle",
            "--category",
            "convention",
        ]);
        workspace.ok(&["remember", "Beta SQLite BodyNeedle", "--category", "fact"]);
        workspace.ok(&["remember", "Gamma read-only", "--category", "insight"]);
        // A matching title in another kind must not leak into memory results.
        workspace.ok(&["new", "note", "BodyNeedle note"]);
        if migrated {
            workspace.migrate();
        }
        workspace
    }
}

fn table_inventory(workspace: &Workspace) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(workspace.ws().join("lance"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn read_state(workspace: &Workspace) -> (Option<Vec<u8>>, Vec<String>) {
    let current = workspace.ws().join("lance/CURRENT.json");
    let current = match std::fs::read(current) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => panic!("could not read CURRENT.json: {error}"),
    };
    (current, table_inventory(workspace))
}

fn assert_migrated_reads_unchanged(workspace: &Workspace, before: &(Option<Vec<u8>>, Vec<String>)) {
    if before.0.is_some() {
        assert_eq!(read_state(workspace), *before);
        // Evidence/history may already exist from init; the exact inventory
        // comparison above catches resurrection without rejecting that baseline.
        assert!(!workspace.ws().join("lance/artifacts.lance").exists());
        assert!(!workspace.ws().join("lance/relations.lance").exists());
    }
}

fn assert_empty_recall(payload: &Value) {
    assert_eq!(
        payload,
        &json!({
            "memories": [],
            "_next_action": "forgeplan recall --limit 5",
        })
    );
}

fn assert_memory_fields(memory: &Value) {
    let mut keys: Vec<_> = memory
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["author", "category", "created_at", "id", "text"]);
    assert!(memory["created_at"].is_string());
}

fn without_timestamp(mut payload: Value) -> Value {
    for memory in payload["memories"].as_array_mut().unwrap() {
        memory.as_object_mut().unwrap().remove("created_at");
    }
    payload
}

fn memory_ids(payload: &Value) -> Vec<&str> {
    payload["memories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|memory| memory["id"].as_str().unwrap())
        .collect()
}

#[test]
fn empty_memory_reads_keep_the_same_payload_text_and_hints_on_both_layouts() {
    for migrated in [false, true] {
        let workspace = Workspace::new(migrated);
        let before = read_state(&workspace);
        assert_empty_recall(&workspace.json(&["recall", "--json"]));
        let recall = workspace.ok(&["recall"]);
        assert!(recall.contains("No memories found."));
        assert!(recall.contains("forgeplan recall --limit 5"));
        let listed = workspace.ok(&["remember", "--list"]);
        assert!(listed.contains("No memories found."));
        assert!(listed.contains("forgeplan remember \"<fact to capture>\""));
        assert_migrated_reads_unchanged(&workspace, &before);
    }
}

#[test]
fn recall_preserves_query_category_provenance_and_limit_contracts_on_both_layouts() {
    for migrated in [false, true] {
        let workspace = Workspace::populated(migrated);
        let before = read_state(&workspace);
        let convention =
            workspace.json(&["recall", "pOsTgReSqL", "--category", "CoNvEnTiOn", "--json"]);
        assert_memory_fields(&convention["memories"][0]);
        assert_eq!(
            without_timestamp(convention),
            json!({
                "memories": [{
                    "id": ALPHA,
                    "category": "convention",
                    "author": AUTHOR,
                    "text": "Alpha PostgreSQL BodyNeedle",
                }],
                "_next_action": format!("forgeplan promote {ALPHA} --kind note"),
            })
        );
        let matches = workspace.json(&["recall", "bOdYnEeDlE", "--json"]);
        let mut ids = memory_ids(&matches);
        ids.sort_unstable();
        assert_eq!(ids, [ALPHA, BETA]);
        for memory in matches["memories"].as_array().unwrap() {
            assert_memory_fields(memory);
            assert_eq!(memory["author"], AUTHOR);
        }
        // Filtering precedes truncation, and the default category is not a
        // wildcard. These combinations must still select the same one memory.
        let limited = workspace.json(&[
            "recall",
            "bodyneedle",
            "--category",
            "FACT",
            "--limit",
            "1",
            "--json",
        ]);
        assert_eq!(memory_ids(&limited), [BETA]);
        assert_empty_recall(&workspace.json(&[
            "recall",
            "postgresql",
            "--category",
            "fact",
            "--json",
        ]));
        assert_empty_recall(&workspace.json(&["recall", "--category", "missing", "--json"]));
        assert_empty_recall(&workspace.json(&["recall", "--limit", "0", "--json"]));
        assert_eq!(
            workspace.json(&["recall", "--limit", "1", "--json"])["memories"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        if !migrated {
            let newest = workspace.json(&["recall", "--json"]);
            assert_eq!(memory_ids(&newest), [GAMMA, BETA, ALPHA]);
            for memory in newest["memories"].as_array().unwrap() {
                chrono::DateTime::parse_from_rfc3339(memory["created_at"].as_str().unwrap())
                    .expect("legacy creation timestamps remain available");
            }
        }
        assert_migrated_reads_unchanged(&workspace, &before);
    }
}

#[test]
fn remember_listing_keeps_id_order_columns_counts_and_author_shortening() {
    for migrated in [false, true] {
        let workspace = Workspace::populated(migrated);
        let before = read_state(&workspace);
        let listed = workspace.ok(&["remember", "--list"]);
        for column in ["ID", "Category", "Author", "Created", "Text"] {
            assert!(listed.lines().next().unwrap().contains(column), "{listed}");
        }
        let rows: Vec<_> = listed
            .lines()
            .filter(|line| line.starts_with("MEM-"))
            .collect();
        assert_eq!(rows.len(), 3, "{listed}");
        for (row, (id, category, text)) in rows.iter().zip([
            (ALPHA, "convention", "Alpha PostgreSQL BodyNeedle"),
            (BETA, "fact", "Beta SQLite BodyNeedle"),
            (GAMMA, "insight", "Gamma read-only"),
        ]) {
            assert!(row.starts_with(id), "{row}");
            assert!(row.contains(category), "{row}");
            assert!(row.contains("Ada Lovelace"), "{row}");
            assert!(row.ends_with(text), "{row}");
        }
        assert!(!listed.contains("ada@example.invalid"), "{listed}");
        assert!(!listed.contains("BodyNeedle note"), "{listed}");
        assert!(listed.contains("3 memory(ies) total"), "{listed}");
        assert!(listed.contains("forgeplan recall \"<query>\""), "{listed}");
        let recalled = workspace.ok(&["recall", "postgresql"]);
        assert!(recalled.contains("[convention]"), "{recalled}");
        assert!(recalled.contains("Ada Lovelace"), "{recalled}");
        assert!(!recalled.contains("ada@example.invalid"), "{recalled}");
        assert_migrated_reads_unchanged(&workspace, &before);
    }
}

#[test]
fn remember_recall_forget_round_trip_keeps_literal_body_and_consumer_fields() {
    for migrated in [false, true] {
        let workspace = Workspace::new(migrated);
        let text = "literal body round trip\n\nUnicode café tail  \n";
        let remembered = workspace.ok(&["remember", text, "--category", "procedure"]);
        assert!(remembered.contains("forgeplan remember --list"));
        let id = remembered
            .split_once("Remembered: ")
            .unwrap()
            .1
            .split_once(" — ")
            .unwrap()
            .0;
        assert!(id.starts_with("MEM-"), "{remembered}");
        let files: Vec<_> = std::fs::read_dir(workspace.ws().join("memory"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
            .collect();
        assert_eq!(files.len(), 1);
        let canonical = std::fs::read_to_string(&files[0]).unwrap();
        assert!(canonical.ends_with(&format!("{text}\n")), "{canonical:?}");
        let before = read_state(&workspace);
        let recalled = workspace.json(&["recall", "cAfÉ", "--category", "PROCEDURE", "--json"]);
        assert_memory_fields(&recalled["memories"][0]);
        assert_eq!(
            without_timestamp(recalled),
            json!({
                "memories": [{
                    "id": id,
                    "category": "procedure",
                    "author": AUTHOR,
                    "text": "literal body round trip  Unicode café tail",
                }],
                "_next_action": format!("forgeplan promote {id} --kind note"),
            })
        );
        let listed = workspace.ok(&["remember", "--list"]);
        assert!(listed.contains(id), "{listed}");
        assert!(listed.contains("procedure"), "{listed}");
        let fetched = workspace.json(&["get", id, "--json"]);
        assert_eq!(fetched["id"], id);
        assert_eq!(fetched["kind"], "memory");
        assert_eq!(fetched["author"], AUTHOR);
        if migrated {
            assert_eq!(fetched["body"], canonical);
        } else {
            let body = fetched["body"].as_str().unwrap();
            assert_eq!(body.rsplit_once("\n---\n\n").unwrap().1, text);
        }
        assert_eq!(std::fs::read_to_string(&files[0]).unwrap(), canonical);
        assert_migrated_reads_unchanged(&workspace, &before);

        let forgotten = workspace.ok(&["remember", "--forget", id]);
        assert!(
            forgotten.contains(&format!("Forgotten: {id}")),
            "{forgotten}"
        );
        assert!(forgotten.contains("forgeplan remember --list"));
        assert!(
            !files[0].exists(),
            "forget left the canonical memory behind"
        );
        let after_forget = read_state(&workspace);
        assert_empty_recall(&workspace.json(&["recall", "--json"]));
        assert!(
            workspace
                .ok(&["remember", "--list"])
                .contains("No memories found.")
        );
        assert_migrated_reads_unchanged(&workspace, &after_forget);
    }
}

#[test]
fn absent_category_author_and_legacy_timestamp_are_not_fabricated() {
    for migrated in [false, true] {
        let workspace = Workspace::new(false);
        let source = "---\nid: mem-anonymous\nkind: memory\nstatus: deprecated\n\
                      depth: tactical\ntitle: Anonymous memory\n---\n\nAnonymous   café memory.\n";
        let path = workspace.write("memory/mem-anonymous-anonymous-memory.md", source);
        let reindexed = workspace.ok(&["reindex"]);
        assert!(!reindexed.contains("create failed"), "{reindexed}");
        if migrated {
            workspace.migrate();
        }
        let before = read_state(&workspace);
        let recalled = workspace.json(&["recall", "anonymous", "--json"]);
        assert_eq!(memory_ids(&recalled), ["mem-anonymous"]);
        let memory = &recalled["memories"][0];
        assert_memory_fields(memory);
        assert_eq!(memory["author"], Value::Null);
        assert_eq!(memory["category"], "fact");
        assert_eq!(memory["text"], "Anonymous   café memory.");
        if migrated {
            // The legacy DB's creation time was never recorded in this file.
            // Keep the existing string-shaped field, honestly empty, not now.
            assert_eq!(memory["created_at"], "");
            assert_eq!(
                workspace.json(&["get", "mem-anonymous", "--json"])["body"],
                source
            );
        } else {
            chrono::DateTime::parse_from_rfc3339(memory["created_at"].as_str().unwrap())
                .expect("legacy row retains its actual creation timestamp");
        }
        // Display defaults do not change the existing explicit-category filter.
        assert_empty_recall(&workspace.json(&["recall", "--category", "fact", "--json"]));
        let listed = workspace.ok(&["remember", "--list"]);
        let row = listed
            .lines()
            .find(|line| line.starts_with("mem-anonymous"))
            .unwrap();
        assert!(row.contains("fact"), "{listed}");
        assert!(row.split_whitespace().any(|cell| cell == "-"), "{listed}");
        assert!(row.ends_with("Anonymous   café memory."), "{listed}");
        assert_eq!(std::fs::read_to_string(path).unwrap(), source);
        assert_migrated_reads_unchanged(&workspace, &before);
    }
}

#[test]
fn captured_reads_sort_source_dates_and_observe_live_body_without_republishing() {
    let workspace = Workspace::new(true);
    let source = |id: &str, date: Option<&str>, text: &str| {
        let date = date
            .map(|value| format!("created_at: {value}\n"))
            .unwrap_or_default();
        format!(
            "---\nid: {id}\nkind: memory\nstatus: active\ndepth: tactical\n\
             title: Shared title\ncategory: insight\nauthor: {AUTHOR}\n{date}---\n\n{text}"
        )
    };
    let alpha = source("mem-alpha", Some("2026-10-01T09:00:00Z"), "older body\n");
    let beta = source("mem-beta", Some("2026-10-03T09:00:00Z"), "latest body\n");
    let gamma = source("mem-gamma", Some("2026-10-03T09:00:00Z"), "tied body\n");
    let delta = source("mem-delta", None, "unknown time\n");
    let path = workspace.write("memory/mem-alpha.md", &alpha);
    workspace.write("memory/mem-beta.md", &beta);
    workspace.write("memory/mem-gamma.md", &gamma);
    workspace.write("memory/mem-delta.md", &delta);
    let before = read_state(&workspace);
    let recalled = workspace.json(&["recall", "--json"]);
    assert_eq!(
        memory_ids(&recalled),
        ["mem-beta", "mem-gamma", "mem-alpha", "mem-delta"]
    );
    assert_eq!(
        recalled["memories"][0]["created_at"],
        "2026-10-03T09:00:00Z"
    );
    assert_eq!(recalled["memories"][3]["created_at"], "");
    assert_eq!(
        recalled["_next_action"],
        "forgeplan promote mem-beta --kind note"
    );
    assert_eq!(
        memory_ids(&workspace.json(&["recall", "--limit", "2", "--json"])),
        ["mem-beta", "mem-gamma"]
    );
    let title_match = workspace.json(&["recall", "SHARED TITLE", "--json"]);
    assert_eq!(memory_ids(&title_match), memory_ids(&recalled));
    let listed = workspace.ok(&["remember", "--list"]);
    let rows: Vec<_> = listed
        .lines()
        .filter(|line| line.starts_with("mem-"))
        .collect();
    assert_eq!(
        rows.iter()
            .map(|line| line.split_whitespace().next().unwrap())
            .collect::<Vec<_>>(),
        ["mem-alpha", "mem-beta", "mem-delta", "mem-gamma"]
    );
    assert!(rows[0].contains("2026-10-01"), "{listed}");
    assert!(rows[1].contains("2026-10-03"), "{listed}");
    assert_eq!(
        workspace.json(&["get", "mem-alpha", "--json"])["body"],
        alpha
    );
    assert_migrated_reads_unchanged(&workspace, &before);

    let edited = source(
        "mem-alpha",
        Some("2026-10-01T09:00:00Z"),
        "fresh   café body  \n",
    );
    std::fs::write(&path, &edited).unwrap();
    assert_empty_recall(&workspace.json(&["recall", "older body", "--json"]));
    let live = workspace.json(&["recall", "fresh", "--json"]);
    assert_eq!(memory_ids(&live), ["mem-alpha"]);
    assert_eq!(live["memories"][0]["text"], "fresh   café body");
    assert_eq!(
        workspace.json(&["get", "mem-alpha", "--json"])["body"],
        edited
    );
    assert!(
        workspace
            .ok(&["remember", "--list"])
            .contains("fresh   café body")
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), edited);
    assert_migrated_reads_unchanged(&workspace, &before);
}

#[test]
fn corrupt_current_is_an_error_not_an_empty_memory_result() {
    let workspace = Workspace::populated(true);
    std::fs::write(workspace.ws().join("lance/CURRENT.json"), "{ not json").unwrap();
    let before = read_state(&workspace);
    for args in [&["recall", "--json"][..], &["remember", "--list"][..]] {
        let output = workspace.output(args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(!stdout.contains("No memories found"), "{stdout}");
        assert!(!stdout.contains("\"memories\": []"), "{stdout}");
        assert!(!output.stderr.is_empty(), "{args:?} failed silently");
    }
    assert_migrated_reads_unchanged(&workspace, &before);
}

#[test]
fn remember_preserves_missing_id_and_non_memory_errors_on_both_layouts() {
    for migrated in [false, true] {
        let workspace = Workspace::new(false);
        workspace.ok(&["new", "note", "Not a memory"]);
        if migrated {
            workspace.migrate();
        }
        let before = read_state(&workspace);
        for (args, error) in [
            (
                vec!["remember", "--forget", "mem-not-present"],
                "Memory 'mem-not-present' not found",
            ),
            (
                vec!["remember", "--forget", "NOTE-001"],
                "'NOTE-001' is not a memory (kind: note)",
            ),
            (
                vec!["remember"],
                "Provide text to remember, or use --list / --forget <id>",
            ),
        ] {
            let output = workspace.output(&args);
            assert_eq!(output.status.code(), Some(1), "{args:?}");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(error),
                "{output:?}"
            );
        }
        assert_empty_recall(&workspace.json(&["recall", "--json"]));
        assert_eq!(
            workspace.json(&["get", "NOTE-001", "--json"])["title"],
            "Not a memory"
        );
        assert_migrated_reads_unchanged(&workspace, &before);
    }
}

#[test]
fn memory_read_commands_preserve_the_missing_workspace_error() {
    let temp = TempDir::new().unwrap();
    for args in [&["recall", "--json"][..], &["remember", "--list"][..]] {
        let output = Command::cargo_bin("forgeplan")
            .unwrap()
            .args(args)
            .current_dir(temp.path())
            .env("HOME", temp.path())
            .env("USERPROFILE", temp.path())
            .env("XDG_DATA_HOME", temp.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("No .forgeplan/ found. Run `forgeplan init` first.")
        );
        assert!(!temp.path().join(".forgeplan").exists());
    }
}
