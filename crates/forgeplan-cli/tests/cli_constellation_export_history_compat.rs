//! The ordinary 0.38 export/log commands must keep their contracts after index ownership changes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use forgeplan_core::changelog::ChangeLogEntry;
use forgeplan_core::db::auxiliary::AuxiliaryStore;
use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::projection::{self, MutationContext};
use forgeplan_core::workspace::{self, lock::acquire_workspace_lock};
use serde_json::{Value, json};
use tempfile::TempDir;

const PRD_PATH: &str = "prds/PRD-021-export-contract.md";
const PRD: &str = "---\nid: PRD-021\nkind: prd\nstatus: active\ntitle: Export contract\n\
    slug: prd-export-contract\npredicted_number: 21\nassigned_number: 21\ndepth: deep\n\
    author: Fixture Author\nparent_epic: EPIC-007\nvalid_until: '2099-12-31'\n\
    created_at: '2026-01-02T03:04:05+00:00'\nupdated_at: '2026-02-03T04:05:06+00:00'\n\
    tags:\n  - alpha\n  - zeta\nlinks:\n  - target: EVID-023\n    relation: informs\n\
    custom_metadata: retained in the exported body\n---\n\n \t\nPreamble é.\n\n\
    ## Same\n\nRepeated content.  \n\n## Same\n\nRepeated content.  \n";
const EVIDENCE: &str = "---\nid: EVID-023\nkind: evidence\nstatus: active\ntitle: Export proof\n\
    depth: standard\n---\n\n## Structured Fields\n\nverdict: supports\n\
    congruence_level: 3\nevidence_type: measurement\n";
const RFC: &str = "---\nid: RFC-022\nkind: rfc\nstatus: draft\ntitle: Export design\n\
    depth: standard\nlinks:\n  - target: PRD-021\n    relation: based_on\n---\n\n## Plan\n\nKeep the exact bundle shape.\n";

fn command(root: &Path, args: &[&str]) -> Output {
    Command::cargo_bin("forgeplan")
        .unwrap()
        .args(args)
        .current_dir(root)
        .env("HF_HUB_OFFLINE", "1")
        .output()
        .unwrap()
}

fn run_ok(root: &Path, args: &[&str]) -> String {
    let output = command(root, args);
    assert!(
        output.status.success(),
        "forgeplan {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn run_json(root: &Path, args: &[&str]) -> Value {
    serde_json::from_str(&run_ok(root, args)).unwrap()
}

fn files_fixture() -> TempDir {
    let root = TempDir::new().unwrap();
    let ws = workspace::init_workspace(root.path(), "Export history compatibility").unwrap();
    for (relative, body) in [
        (PRD_PATH, PRD),
        ("evidence/EVID-023-export-proof.md", EVIDENCE),
        ("rfcs/RFC-022-export-design.md", RFC),
    ] {
        std::fs::write(ws.join(relative), body).unwrap();
    }
    root
}

/// Populate real legacy rows and relations without rewriting the canonical fixture bytes.
fn legacy_fixture() -> (TempDir, Value) {
    let root = files_fixture();
    let ws = root.path().join(".forgeplan");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let expected_prd = runtime.block_on(async {
        let lock = acquire_workspace_lock(&ws).await.unwrap();
        let store = LanceStore::init(&ws).await.unwrap();
        let ctx = MutationContext::new(&ws, &lock, Some(&store));
        for (id, kind, title, body) in [
            ("EVID-023", "evidence", "Export proof", EVIDENCE),
            ("PRD-021", "prd", "Export contract", PRD),
            ("RFC-022", "rfc", "Export design", RFC),
        ] {
            let is_prd = id == "PRD-021";
            projection::sync_artifact_from_file(
                &ctx,
                &NewArtifact {
                    canonical_key: id.into(),
                    id: id.into(),
                    kind: kind.into(),
                    title: title.into(),
                    status: if kind == "rfc" { "draft" } else { "active" }.into(),
                    body: body.into(),
                    depth: if is_prd { "deep" } else { "standard" }.into(),
                    author: is_prd.then(|| "Fixture Author".into()),
                    parent_epic: is_prd.then(|| "EPIC-007".into()),
                    valid_until: is_prd.then(|| "2099-12-31".into()),
                    tags: if is_prd {
                        vec!["alpha".into(), "zeta".into()]
                    } else {
                        Vec::new()
                    },
                },
            )
            .await
            .unwrap();
        }
        for (source, target, relation) in [
            ("PRD-021", "EVID-023", "informs"),
            ("RFC-022", "PRD-021", "based_on"),
        ] {
            projection::sync_relation_from_file(&ctx, source, target, relation)
                .await
                .unwrap();
        }
        // A nonzero cached legacy score must not be replaced with a placeholder.
        store.update_r_eff_score("PRD-021", 0.625).await.unwrap();
        let record = store.get_record("PRD-021").await.unwrap().unwrap();
        expected_prd(&record.created_at, &record.updated_at, 0.625, PRD, "PRD-021")
    });
    (root, expected_prd)
}

fn migrate(root: &Path) {
    run_ok(root, &["index"]);
    assert!(root.join(".forgeplan/lance/CURRENT.json").is_file());
    assert!(!root.join(".forgeplan/lance/artifacts.lance").exists());
    assert!(!root.join(".forgeplan/lance/relations.lance").exists());
}

fn expected_prd(created_at: &str, updated_at: &str, score: f64, body: &str, key: &str) -> Value {
    json!({
        "id": "PRD-021", "canonical_key": key, "kind": "prd",
        "status": "active", "title": "Export contract",
        "body": body, "depth": "deep", "author": "Fixture Author", "parent_epic": "EPIC-007",
        "r_eff_score": score, "valid_until": "2099-12-31", "created_at": created_at,
        "updated_at": updated_at, "tags": ["alpha", "zeta"]
    })
}

fn artifact<'a>(bundle: &'a Value, id: &str) -> &'a Value {
    bundle["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["id"] == id)
        .unwrap_or_else(|| panic!("missing {id} in {bundle}"))
}

/// The bundle of one layout.
///
/// `key` is the canonical key the PRD carries in that layout: a legacy index
/// row that was written with the display id as its key keeps that key, while
/// the managed layout derives the key from the document's own slug.
fn assert_bundle(bundle: &Value, expected_prd: &Value, key: &str) {
    let mut keys: Vec<_> = bundle
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(keys, ["artifacts", "relations", "version"]);
    assert_eq!(bundle["version"], 1);
    let ids: Vec<_> = bundle["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["EVID-023", "PRD-021", "RFC-022"]);
    assert_eq!(artifact(bundle, "PRD-021"), expected_prd);
    assert_eq!(artifact(bundle, "EVID-023")["body"], EVIDENCE);
    let rfc = artifact(bundle, "RFC-022");
    assert_eq!(rfc["body"], RFC);
    assert_eq!(rfc["author"], Value::Null);
    assert_eq!(rfc["parent_epic"], Value::Null);
    assert_eq!(rfc["valid_until"], Value::Null);
    assert_eq!(rfc["tags"], json!([]));
    let mut relations = bundle["relations"].as_array().unwrap().clone();
    relations.sort_by_key(|relation| relation["source"].as_str().unwrap().to_string());
    assert_eq!(
        relations,
        vec![
            json!({"source": "PRD-021", "target": "EVID-023", "relation": "informs",
                   "source_key": key, "target_key": "EVID-023"}),
            json!({"source": "RFC-022", "target": "PRD-021", "relation": "based_on",
                   "source_key": "RFC-022", "target_key": key}),
        ]
    );
}

/// Include directories and file bytes, so even an empty new table/lock is observable.
fn inventory(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, dir: &Path, result: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if entry.file_type().unwrap().is_dir() {
                result.insert(relative, None);
                visit(root, &path, result);
            } else {
                result.insert(relative, Some(std::fs::read(path).unwrap()));
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

fn export(root: &Path, output: &str) -> Value {
    let stdout = run_ok(root, &["export", "--output", output]);
    assert!(
        stdout.contains("Exported 3 artifacts, 2 relations to "),
        "{stdout}"
    );
    let hint = stdout
        .lines()
        .find(|line| line.starts_with("Next: "))
        .unwrap();
    assert!(hint.starts_with("Next: forgeplan import "), "{hint}");
    serde_json::from_slice(&std::fs::read(root.join(output)).unwrap()).unwrap()
}

#[test]
fn export_preserves_nonempty_legacy_and_managed_bundle_metadata_bodies_and_links() {
    let (root, legacy_prd) = legacy_fixture();
    let legacy = export(root.path(), "backups/legacy.json");
    assert_bundle(&legacy, &legacy_prd, "PRD-021");

    migrate(root.path());
    // A residual old table cannot reclaim a managed index or become an export source.
    for name in ["artifacts.lance", "relations.lance"] {
        std::fs::write(
            root.path().join(".forgeplan/lance").join(name),
            b"not a dataset",
        )
        .unwrap();
    }
    let ws = root.path().join(".forgeplan");
    let before = inventory(&ws);
    let captured = export(root.path(), "backups/captured.json");
    assert_bundle(
        &captured,
        &expected_prd(
            "2026-01-02T03:04:05+00:00",
            "2026-02-03T04:05:06+00:00",
            1.0,
            PRD,
            "prd-export-contract",
        ),
        "prd-export-contract",
    );
    assert_eq!(
        inventory(&ws),
        before,
        "export changed workspace/index files"
    );
}

#[test]
fn managed_export_uses_one_fresh_capture_after_offline_metadata_body_and_link_edits() {
    let root = files_fixture();
    migrate(root.path());
    let ws = root.path().join(".forgeplan");
    let changed = PRD
        .replace("title: Export contract", "title: Edited contract")
        .replace("depth: deep", "depth: tactical")
        .replace("  - zeta", "  - new-tag")
        .replace("2026-02-03T04:05:06+00:00", "2026-03-04T05:06:07+00:00")
        .replace("EVID-023", "RFC-022")
        .replace("relation: informs", "relation: refines")
        .replace("Repeated content.", "Offline content é.");
    std::fs::write(ws.join(PRD_PATH), &changed).unwrap();
    let before = inventory(&ws);
    let bundle = export(root.path(), "offline.json");
    let mut expected = expected_prd(
        "2026-01-02T03:04:05+00:00",
        "2026-03-04T05:06:07+00:00",
        0.0,
        &changed,
        "prd-export-contract",
    );
    expected["title"] = json!("Edited contract");
    expected["depth"] = json!("tactical");
    expected["tags"] = json!(["alpha", "new-tag"]);
    assert_eq!(artifact(&bundle, "PRD-021"), &expected);
    assert!(
        bundle["relations"]
            .as_array()
            .unwrap()
            .contains(&json!({"source": "PRD-021", "target": "RFC-022", "relation": "refines",
                              "source_key": "prd-export-contract", "target_key": "RFC-022"}))
    );
    assert!(
        !bundle["relations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["target"] == "EVID-023")
    );
    assert_eq!(
        inventory(&ws),
        before,
        "fresh export published or mutated an index"
    );
}

#[test]
fn empty_export_keeps_the_default_file_and_version_one_bundle_contract() {
    for managed in [false, true] {
        let root = TempDir::new().unwrap();
        let ws = workspace::init_workspace(root.path(), "Empty export").unwrap();
        if managed {
            migrate(root.path());
        } else {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(LanceStore::init(&ws))
                .unwrap();
        }
        let stdout = run_ok(root.path(), &["export"]);
        assert!(
            stdout.contains("Exported 0 artifacts, 0 relations to "),
            "{stdout}"
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(ws.join("export.json")).unwrap())
                .unwrap(),
            json!({"version": 1, "artifacts": [], "relations": []})
        );
    }
}

#[test]
fn export_errors_preserve_existing_output_and_never_turn_corruption_into_an_empty_bundle() {
    let root = files_fixture();
    migrate(root.path());
    let ws = root.path().join(".forgeplan");
    let output = root.path().join("keep.json");
    std::fs::write(&output, b"previous backup").unwrap();
    std::fs::write(ws.join("lance/CURRENT.json"), b"{ invalid pointer").unwrap();
    let before = inventory(&ws);
    let result = command(root.path(), &["export", "--output", "keep.json"]);
    assert!(!result.status.success());
    assert!(!result.stderr.is_empty());
    assert!(!String::from_utf8_lossy(&result.stdout).contains("Exported"));
    assert_eq!(std::fs::read(&output).unwrap(), b"previous backup");
    assert_eq!(inventory(&ws), before);
}

#[test]
fn export_propagates_an_unreadable_canonical_file_and_an_unwritable_destination() {
    let root = files_fixture();
    migrate(root.path());
    let ws = root.path().join(".forgeplan");
    let path = ws.join(PRD_PATH);
    std::fs::write(&path, [0xff, 0xfe, 0xfd]).unwrap();
    let before = inventory(&ws);
    let unreadable = command(root.path(), &["export", "--output", "unreadable.json"]);
    assert!(!unreadable.status.success());
    assert!(!unreadable.stderr.is_empty());
    assert!(!root.path().join("unreadable.json").exists());
    assert_eq!(inventory(&ws), before);

    std::fs::write(path, PRD).unwrap();
    std::fs::create_dir(root.path().join("destination-directory")).unwrap();
    let before = inventory(&ws);
    let unwritable = command(
        root.path(),
        &["export", "--output", "destination-directory"],
    );
    assert!(!unwritable.status.success());
    assert!(!unwritable.stderr.is_empty());
    assert!(!String::from_utf8_lossy(&unwritable.stdout).contains("Exported"));
    assert_eq!(inventory(&ws), before);
}

fn old_history() -> Vec<ChangeLogEntry> {
    vec![
        entry(
            "2026-10-24T10:10:00+00:00",
            "PRD-021",
            "create",
            "cli",
            None,
            None,
            None,
            None,
        ),
        entry(
            "2026-10-24T10:12:00+00:00",
            "PRD-021",
            "update",
            "cli",
            Some("status"),
            Some("draft"),
            Some("active"),
            None,
        ),
        entry(
            "2026-10-24T10:11:00+00:00",
            "RFC-022",
            "update",
            "git_sync",
            Some("body"),
            Some("0123456789ab"),
            Some("fedcba987654"),
            Some("abc1234"),
        ),
        entry(
            "2026-10-24T10:13:00+00:00",
            "PRD-021",
            "tag",
            "file_edit",
            Some("tags"),
            None,
            Some("compatibility"),
            None,
        ),
        entry(
            "2026-10-24T10:14:00+00:00",
            "NOTE-legacy'quoted",
            "create",
            "import'o",
            None,
            None,
            Some("quoted"),
            None,
        ),
    ]
}

fn new_history() -> Vec<ChangeLogEntry> {
    vec![
        entry(
            "2026-10-24T10:17:00+00:00",
            "PRD-021",
            "update",
            "cli",
            Some("title"),
            Some("oldtitlelong"),
            Some("newtitlelong"),
            None,
        ),
        entry(
            "2026-10-24T10:15:00+00:00",
            "PRD-021",
            "delete",
            "reindex",
            Some("body"),
            Some("removed text"),
            None,
            None,
        ),
        entry(
            "2026-10-24T10:16:00+00:00",
            "RFC-022",
            "update",
            "cli",
            Some("title"),
            None,
            Some("a newly restored title"),
            None,
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn entry(
    timestamp: &str,
    artifact_id: &str,
    action: &str,
    source: &str,
    field: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
    commit_hash: Option<&str>,
) -> ChangeLogEntry {
    ChangeLogEntry {
        timestamp: timestamp.into(),
        artifact_id: artifact_id.into(),
        canonical_key: None,
        action: action.into(),
        source: source.into(),
        field: field.map(str::to_string),
        old_value: old_value.map(str::to_string),
        new_value: new_value.map(str::to_string),
        commit_hash: commit_hash.map(str::to_string),
    }
}

fn seed_old_history(root: &Path, entries: &[ChangeLogEntry]) {
    let ws = root.join(".forgeplan");
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let _lock = acquire_workspace_lock(&ws).await.unwrap();
        let store = LanceStore::open(&ws).await.unwrap();
        for entry in entries {
            store.log_change(entry).await.unwrap();
        }
    });
}

fn seed_new_history(root: &Path, entries: &[ChangeLogEntry]) {
    let ws = root.join(".forgeplan");
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let _lock = acquire_workspace_lock(&ws).await.unwrap();
        let store = AuxiliaryStore::open_history_or_create(&ws).await.unwrap();
        for entry in entries {
            store.log_change(entry).await.unwrap();
        }
    });
}

/// The receipt's `canonical_key` is identity metadata the reader derives from
/// the index; the pinned event shape below does not depend on it, and a
/// legacy receipt has none. Drop it from both sides before comparing.
fn without_identity(value: &mut serde_json::Value) {
    if let Some(entries) = value.as_array_mut() {
        for entry in entries {
            if let Some(object) = entry.as_object_mut() {
                object.remove("canonical_key");
            }
        }
    }
}

fn assert_history(root: &Path, args: &[&str], expected: &[ChangeLogEntry]) {
    let payload = run_json(root, args);
    let mut actual = payload["entries"].clone();
    let mut wanted = serde_json::to_value(expected).unwrap();
    without_identity(&mut actual);
    without_identity(&mut wanted);
    assert_eq!(actual, wanted);
    assert_eq!(
        payload["_next_action"],
        json!(
            expected
                .first()
                .map(|first| format!("forgeplan get {}", first.artifact_id))
        )
    );
    assert_eq!(payload.as_object().unwrap().len(), 2);
}

fn assert_history_queries(root: &Path, entries: &[ChangeLogEntry]) {
    let mut sorted = entries.to_vec();
    sorted.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
    assert_history(root, &["log", "--json", "-n", "100"], &sorted);
    assert_history(root, &["log", "--json"], &sorted[..sorted.len().min(20)]);
    assert_history(root, &["log", "--json", "--limit", "1"], &sorted[..1]);
    assert_history(root, &["log", "--json", "-n", "0"], &[]);
    let prd: Vec<_> = sorted
        .iter()
        .filter(|entry| entry.artifact_id == "PRD-021")
        .cloned()
        .collect();
    assert_history(root, &["log", "PRD-021", "--json", "-n", "100"], &prd);
    let source: Vec<_> = sorted
        .iter()
        .filter(|entry| entry.source == "cli")
        .cloned()
        .collect();
    assert_history(
        root,
        &["log", "--source", "cli", "--json", "-n", "100"],
        &source,
    );
    let combined: Vec<_> = prd
        .into_iter()
        .filter(|entry| entry.source == "cli")
        .take(1)
        .collect();
    assert_history(
        root,
        &["log", "PRD-021", "--source", "cli", "--json", "-n", "1"],
        &combined,
    );
    let quoted: Vec<_> = sorted
        .iter()
        .filter(|entry| entry.source == "import'o")
        .cloned()
        .collect();
    assert_history(
        root,
        &[
            "log",
            "NOTE-legacy'quoted",
            "--source",
            "import'o",
            "--json",
        ],
        &quoted,
    );
    assert_history(root, &["log", "PRD-999", "--json"], &[]);
    assert_history(root, &["log", "--source", "does-not-exist", "--json"], &[]);
}

#[test]
fn log_reads_old_history_before_and_after_migration_then_combines_new_history_without_writes() {
    let (root, _) = legacy_fixture();
    let mut history = old_history();
    seed_old_history(root.path(), &history);
    let ws = root.path().join(".forgeplan");
    let before = inventory(&ws);
    assert_history_queries(root.path(), &history);
    assert_eq!(inventory(&ws), before);

    migrate(root.path());
    let before = inventory(&ws);
    assert_history_queries(root.path(), &history);
    assert_eq!(inventory(&ws), before);

    let new = new_history();
    seed_new_history(root.path(), &new);
    history.extend(new);
    let before = inventory(&ws);
    assert_history_queries(root.path(), &history);
    assert_eq!(
        inventory(&ws),
        before,
        "history reads changed files or created tables"
    );
    assert!(!ws.join("lance/artifacts.lance").exists());
    assert!(!ws.join("lance/relations.lance").exists());
}

#[test]
fn auxiliary_only_history_supports_the_default_twenty_rows_and_explicit_limits() {
    let root = files_fixture();
    migrate(root.path());
    let entries: Vec<_> = (0..25)
        .rev()
        .map(|minute| {
            entry(
                &format!("2026-10-24T11:{minute:02}:00+00:00"),
                "PRD-021",
                "update",
                "cli",
                Some("title"),
                None,
                Some("many rows"),
                None,
            )
        })
        .collect();
    seed_new_history(root.path(), &entries);
    let before = inventory(&root.path().join(".forgeplan"));
    assert_history_queries(root.path(), &entries);
    let text = run_ok(root.path(), &["log"]);
    assert!(text.contains("20 entries shown"), "{text}");
    assert!(text.contains("Next: forgeplan get PRD-021"), "{text}");
    assert_eq!(inventory(&root.path().join(".forgeplan")), before);
}

#[test]
fn log_text_preserves_columns_truncation_nulls_commit_and_reverse_chronological_order() {
    let (root, _) = legacy_fixture();
    let old = old_history();
    let new = new_history();
    seed_old_history(root.path(), &old);
    migrate(root.path());
    seed_new_history(root.path(), &new);
    let before = inventory(&root.path().join(".forgeplan"));
    let text = run_ok(root.path(), &["log", "-n", "100"]);
    let mut lines = text.lines();
    assert_eq!(
        lines.next().unwrap(),
        format!(
            "{:<18} {:<10} {:<9} {:<8} {:<20} {:<10} COMMIT",
            "TIMESTAMP", "ARTIFACT", "ACTION", "FIELD", "CHANGE", "SOURCE"
        )
    );
    assert_eq!(lines.next().unwrap(), "-".repeat(90));
    let rows: Vec<_> = lines.take(8).collect();
    assert_eq!(rows.len(), 8);
    assert_eq!(
        rows[0],
        format!(
            "{:<18} {:<10} {:<9} {:<8} {:<20} {:<10} {}",
            "2026-10-24 10:17", "PRD-021", "update", "title", "oldtitl…→newtitl…", "cli", "-"
        )
    );
    assert_eq!(
        rows[1],
        format!(
            "{:<18} {:<10} {:<9} {:<8} {:<20} {:<10} {}",
            "2026-10-24 10:16", "RFC-022", "update", "title", "a newly restored …", "cli", "-"
        )
    );
    assert!(rows[2].contains("(was removed text)"), "{}", rows[2]);
    assert!(rows[4].contains("compatibility"), "{}", rows[4]);
    assert!(rows[5].contains("draft→active"), "{}", rows[5]);
    assert!(
        rows[6].contains("0123456…→fedcba9…") && rows[6].ends_with("abc1234"),
        "{}",
        rows[6]
    );
    assert_eq!(
        rows[7],
        format!(
            "{:<18} {:<10} {:<9} {:<8} {:<20} {:<10} {}",
            "2026-10-24 10:10", "PRD-021", "create", "-", "-", "cli", "-"
        )
    );
    assert!(
        text.contains("8 entries shown\n\nNext: forgeplan get PRD-021"),
        "{text}"
    );
    assert_eq!(inventory(&root.path().join(".forgeplan")), before);
}

#[test]
fn log_missing_and_empty_history_are_empty_without_creating_a_table_or_directory() {
    for managed in [false, true] {
        for existing in [false, true] {
            let root = files_fixture();
            let ws = root.path().join(".forgeplan");
            if managed {
                migrate(root.path());
            }
            if existing {
                seed_new_history(root.path(), &[]);
                assert!(ws.join("lance/change_log.lance").is_dir());
            } else {
                assert!(!ws.join("lance/change_log.lance").exists());
            }
            let before = inventory(&ws);
            assert_history(root.path(), &["log", "--json"], &[]);
            assert_eq!(
                run_ok(root.path(), &["log"]),
                "No change log entries found.\n\nNext: forgeplan list\n"
            );
            assert_eq!(inventory(&ws), before);
        }
    }
}

#[test]
fn log_corrupt_existing_history_is_an_error_on_both_layouts_not_an_empty_success() {
    for managed in [false, true] {
        let (root, _) = legacy_fixture();
        seed_old_history(root.path(), &old_history());
        if managed {
            migrate(root.path());
        }
        let ws = root.path().join(".forgeplan");
        let table = ws.join("lance/change_log.lance");
        std::fs::rename(&table, root.path().join("saved-change-log.lance")).unwrap();
        std::fs::create_dir_all(table.join("_versions")).unwrap();
        std::fs::write(table.join("_versions/1.manifest"), b"not a Lance manifest").unwrap();
        let before = inventory(&ws);
        for args in [vec!["log"], vec!["log", "--json"]] {
            let result = command(root.path(), &args);
            assert!(
                !result.status.success(),
                "{args:?} accepted corrupt history"
            );
            assert!(!result.stderr.is_empty());
            assert!(!String::from_utf8_lossy(&result.stdout).contains("No change log entries"));
            assert!(!String::from_utf8_lossy(&result.stdout).contains("\"entries\""));
        }
        assert_eq!(inventory(&ws), before);
    }
}

#[test]
fn log_unreadable_database_path_is_an_error_not_missing_history() {
    let root = files_fixture();
    let ws = root.path().join(".forgeplan");
    std::fs::write(ws.join("lance"), b"not a directory").unwrap();
    let before = inventory(&ws);
    let result = command(root.path(), &["log", "--json"]);
    assert!(!result.status.success());
    assert!(!result.stderr.is_empty());
    assert!(!String::from_utf8_lossy(&result.stdout).contains("\"entries\""));
    assert_eq!(inventory(&ws), before);
}

#[test]
fn export_and_log_keep_the_missing_workspace_error_without_creating_files() {
    let root = TempDir::new().unwrap();
    let before = inventory(root.path());
    for args in [vec!["export"], vec!["log", "--json"]] {
        let result = command(root.path(), &args);
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr)
                .contains("No .forgeplan/ found. Run `forgeplan init` first.")
        );
    }
    assert_eq!(inventory(root.path()), before);
}
