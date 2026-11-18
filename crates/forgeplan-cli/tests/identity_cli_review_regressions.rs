use assert_cmd::Command;
use serde_json::Value;
use std::path::Path;
use tempfile::TempDir;

fn cli() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn workspace() -> TempDir {
    let tmp = TempDir::new().unwrap();
    cli()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    tmp
}

fn document(root: &Path, path: &str, frontmatter: &str) {
    let path = root.join(".forgeplan").join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, format!("---\n{frontmatter}\n---\n\nBody\n")).unwrap();
}

#[test]
fn v5_reindex_updates_external_body_without_rewriting_canonical_markdown() {
    let tmp = workspace();
    cli()
        .args(["new", "note", "Reindex canonical identity"])
        .current_dir(tmp.path())
        .assert()
        .success();
    let output = cli()
        .args(["list", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let records: Value = serde_json::from_slice(&output.stdout).unwrap();
    let created = records
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["title"] == "Reindex canonical identity")
        .unwrap();
    let key = created["canonical_key"].as_str().unwrap();
    let path = std::fs::read_dir(tmp.path().join(".forgeplan/notes"))
        .unwrap()
        .find_map(|entry| {
            let path = entry.unwrap().path();
            (path.extension().is_some_and(|extension| extension == "md")).then_some(path)
        })
        .unwrap();
    let original = std::fs::read_to_string(&path).unwrap();
    let edited = format!("{original}\nExternallyEditedCanonicalBody_20261110\n");
    std::fs::write(&path, &edited).unwrap();
    cli()
        .arg("reindex")
        .current_dir(tmp.path())
        .assert()
        .success();
    let output = cli()
        .args(["get", key, "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let fetched: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(fetched["canonical_key"], key);
    assert!(
        fetched["body"]
            .as_str()
            .unwrap()
            .contains("ExternallyEditedCanonicalBody_20261110")
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), edited);
    let history = cli()
        .args(["log", key, "--source", "reindex", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(history.status.success());
    let history: Value = serde_json::from_slice(&history.stdout).unwrap();
    let updates: Vec<_> = history["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["action"] == "update")
        .collect();
    assert_eq!(
        updates.len(),
        1,
        "the indexed body transition needs one audit receipt"
    );
    cli()
        .arg("reindex")
        .current_dir(tmp.path())
        .assert()
        .success();
    let repeated = cli()
        .args(["log", key, "--source", "reindex", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(repeated.status.success());
    let repeated: Value = serde_json::from_slice(&repeated.stdout).unwrap();
    assert_eq!(
        repeated["entries"], history["entries"],
        "unchanged reindex must not fabricate another mutation"
    );
}

#[test]
fn validate_explicit_selectors_keep_typed_errors_and_canonical_identity() {
    let tmp = workspace();
    for (selector, code) in [("PRD-123", 3), ("bad/selector", 1)] {
        let output = cli()
            .args(["validate", selector, "--json"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(code));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("No artifacts found"));
    }
    for key in ["prd-alpha", "prd-beta"] {
        document(
            tmp.path(),
            &format!("prds/{key}.md"),
            &format!("id: PRD-001\nslug: {key}\nkind: prd\nstatus: draft\ntitle: {key}"),
        );
    }
    cli()
        .arg("index")
        .current_dir(tmp.path())
        .assert()
        .success();
    let ambiguous = cli()
        .args(["validate", "PRD-001", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert_eq!(ambiguous.status.code(), Some(5));
    let stderr = String::from_utf8_lossy(&ambiguous.stderr);
    assert!(
        stderr.contains("prd-alpha") && stderr.contains("prd-beta"),
        "{stderr}"
    );
    let output = cli()
        .args(["validate", "prd-alpha", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let results = json["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["artifact_id"], "PRD-001");
    assert_eq!(results[0]["id"], "PRD-001");
    assert_eq!(results[0]["canonical_key"], "prd-alpha");
}

#[test]
fn tree_resolves_normalized_parent_and_rejects_ambiguous_parent() {
    let tmp = workspace();
    document(
        tmp.path(),
        "epics/EPIC-001-main.md",
        "id: EPIC-001\nslug: epic-main\nkind: epic\nstatus: draft\ntitle: Main",
    );
    document(
        tmp.path(),
        "rfcs/RFC-001-child.md",
        "id: RFC-001\nslug: rfc-child\nkind: rfc\nstatus: draft\ntitle: Child\nparent_epic: epic-1",
    );
    cli()
        .arg("index")
        .current_dir(tmp.path())
        .assert()
        .success();
    let output = cli()
        .args(["tree", "epic-main", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        json["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["canonical_key"] == "rfc-child")
    );
    document(
        tmp.path(),
        "epics/EPIC-001-other.md",
        "id: EPIC-001\nslug: epic-other\nkind: epic\nstatus: draft\ntitle: Other",
    );
    let output = cli()
        .args(["tree", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("epic-main") && stderr.contains("epic-other"),
        "{stderr}"
    );
}

#[test]
fn tree_preserves_valid_unresolved_structural_parent() {
    let tmp = workspace();
    document(
        tmp.path(),
        "rfcs/RFC-001-child.md",
        "id: RFC-001\nslug: rfc-child\nkind: rfc\nstatus: draft\ntitle: Child\nparent_epic: RFC-A01",
    );
    document(
        tmp.path(),
        "rfcs/RFC-002-remote.md",
        "id: RFC-002\nslug: rfc-remote\nkind: rfc\nstatus: draft\ntitle: Remote\nparent_epic: EPIC-001@remote",
    );
    cli()
        .arg("index")
        .current_dir(tmp.path())
        .assert()
        .success();
    let output = cli()
        .args(["tree", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let roots: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        roots
            .as_array()
            .unwrap()
            .iter()
            .any(|root| root["canonical_key"] == "rfc-child")
    );
    assert!(
        roots
            .as_array()
            .unwrap()
            .iter()
            .any(|root| root["canonical_key"] == "rfc-remote")
    );
}

#[test]
fn keyword_search_applies_composite_filters_before_limit() {
    let tmp = workspace();
    document(
        tmp.path(),
        "prds/PRD-001-draft.md",
        "id: PRD-001\nslug: prd-draft\nkind: prd\nstatus: draft\ndepth: deep\ntitle: Needle",
    );
    document(
        tmp.path(),
        "prds/PRD-002-active.md",
        "id: PRD-002\nslug: prd-active\nkind: prd\nstatus: active\ndepth: deep\ntitle: Needle",
    );
    document(
        tmp.path(),
        "prds/PRD-003-standard.md",
        "id: PRD-003\nslug: prd-standard\nkind: prd\nstatus: active\ndepth: standard\ntitle: Needle",
    );
    document(
        tmp.path(),
        "prds/PRD-004-active.md",
        "id: PRD-004\nslug: prd-active-two\nkind: prd\nstatus: active\ndepth: deep\ntitle: Needle",
    );
    cli()
        .arg("index")
        .current_dir(tmp.path())
        .assert()
        .success();
    let output = cli()
        .args([
            "search",
            "Needle",
            "--keyword",
            "--status",
            "active",
            "--depth",
            "deep",
            "--limit",
            "1",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let results = json["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert!(
        ["prd-active", "prd-active-two"].contains(&results[0]["canonical_key"].as_str().unwrap())
    );
    assert_eq!(
        json["_next_action"],
        format!(
            "forgeplan get {}",
            results[0]["canonical_key"].as_str().unwrap()
        )
    );
    let output = cli()
        .args(["search", "Needle", "--keyword", "--limit", "4", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let unfiltered: Value = serde_json::from_slice(&output.stdout).unwrap();
    let first_status = unfiltered["results"][0]["status"].as_str().unwrap();
    let excluded_first = if first_status == "active" {
        "draft"
    } else {
        "active"
    };
    let output = cli()
        .args([
            "search",
            "Needle",
            "--keyword",
            "--status",
            excluded_first,
            "--limit",
            "1",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let filtered: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(filtered["results"].as_array().unwrap().len(), 1);
    assert_eq!(filtered["results"][0]["status"], excluded_first);
}

#[cfg(not(feature = "semantic-search"))]
#[test]
fn semantic_search_never_returns_lexical_hits_when_backend_is_unavailable() {
    let tmp = workspace();
    document(
        tmp.path(),
        "prds/PRD-001-alpha.md",
        "id: PRD-001\nslug: prd-alpha\nkind: prd\nstatus: draft\ntitle: Needle",
    );
    cli()
        .arg("index")
        .current_dir(tmp.path())
        .assert()
        .success();
    let output = cli()
        .args(["search", "Needle", "--semantic", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("prd-alpha"));
}

#[test]
fn dispatch_uses_captured_frontmatter_and_resolves_epic_aliases() {
    let tmp = workspace();
    document(
        tmp.path(),
        "epics/epic-work.md",
        "id: EPIC-001\nslug: epic-work\nkind: epic\nstatus: active\ntitle: Work",
    );
    for (key, parent, file) in [
        ("prd-alpha", "EPIC-001", "src/alpha.rs"),
        ("prd-beta", "epic-work", "src/beta.rs"),
    ] {
        document(
            tmp.path(),
            &format!("prds/{key}.md"),
            &format!(
                "id: PRD-001\nslug: {key}\nkind: prd\nstatus: draft\ntitle: Renamed {key}\n\
                 parent_epic: {parent}\naffected_files: [{file}]"
            ),
        );
    }
    cli()
        .arg("index")
        .current_dir(tmp.path())
        .assert()
        .success();
    let output = cli()
        .args(["dispatch", "--agents", "2", "--epic", "EPIC-001", "--json"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let plan: Value = serde_json::from_slice(&output).unwrap();
    let mut keys: Vec<_> = plan["canonical_buckets"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|bucket| bucket.as_array().unwrap().iter())
        .map(|key| key.as_str().unwrap())
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["prd-alpha", "prd-beta"], "{plan}");
    assert_eq!(plan["skipped_parse_errors"], 0);
    assert!(plan["serial_queue"].as_array().unwrap().is_empty());
}

async fn legacy_dispatch_workspace(blocked: bool) -> TempDir {
    let tmp = TempDir::new().unwrap();
    let ws = forgeplan_core::workspace::init_workspace(tmp.path(), "legacy-dispatch").unwrap();
    let guard = forgeplan_core::workspace::acquire_workspace_lock(&ws)
        .await
        .unwrap();
    let store = forgeplan_core::db::store::LanceStore::init_new(&ws, &guard)
        .await
        .unwrap();
    let ctx = forgeplan_core::projection::MutationContext::new(&ws, &guard, Some(&store));
    for (key, id, kind) in [
        ("prd-claimed", "PRD-001", "prd"),
        ("prd-free", "PRD-002", "prd"),
    ]
    .into_iter()
    .chain(blocked.then_some(("rfc-pending", "RFC-001", "rfc")))
    {
        forgeplan_core::projection::create_artifact_with_projection(
            &ctx,
            &forgeplan_core::db::store::NewArtifact {
                canonical_key: key.into(),
                id: id.into(),
                kind: kind.into(),
                status: "draft".into(),
                title: key.into(),
                body: "Dispatch work.".into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            },
        )
        .await
        .unwrap();
    }
    if blocked {
        for key in ["prd-claimed", "prd-free"] {
            forgeplan_core::projection::add_link_with_projection(
                &ctx,
                key,
                "rfc-pending",
                "based_on",
            )
            .await
            .unwrap();
        }
    }
    drop(store);
    drop(guard);
    tmp
}

fn make_legacy_dispatch_claim(root: &Path) -> std::path::PathBuf {
    cli()
        .args(["claim", "prd-claimed", "--agent", "legacy-worker", "--json"])
        .current_dir(root)
        .assert()
        .success();
    let dir = root.join(".forgeplan/claims");
    let keyed = forgeplan_core::artifact::sidecar::sidecar_path(&dir, "prd-claimed", "yaml");
    let mut claim: serde_yaml::Value =
        serde_yaml::from_slice(&std::fs::read(&keyed).unwrap()).unwrap();
    claim.as_mapping_mut().unwrap().remove("canonical_key");
    let legacy = dir.join("PRD-001.yaml");
    std::fs::write(&legacy, serde_yaml::to_string(&claim).unwrap()).unwrap();
    std::fs::remove_file(keyed).unwrap();
    legacy
}

#[tokio::test]
async fn dispatch_keeps_a_legacy_claim_out_of_every_queue_without_migrating_it() {
    for blocked in [false, true] {
        let tmp = legacy_dispatch_workspace(blocked).await;
        let claim_path = make_legacy_dispatch_claim(tmp.path());
        let before = std::fs::read(&claim_path).unwrap();
        let sources = [
            tmp.path().join(".forgeplan/prds/prd-claimed.md"),
            tmp.path().join(".forgeplan/prds/prd-free.md"),
        ];
        let source_before = sources.each_ref().map(|path| std::fs::read(path).unwrap());
        let output = cli()
            .args(["dispatch", "--agents", "2", "--kind", "prd", "--json"])
            .current_dir(tmp.path())
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let plan: Value = serde_json::from_slice(&output).unwrap();
        let planned: Vec<_> = plan["canonical_buckets"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|bucket| bucket.as_array().unwrap().iter())
            .map(|key| key.as_str().unwrap())
            .chain(
                plan["serial_queue"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|entry| entry["canonical_key"].as_str().unwrap()),
            )
            .collect();
        assert_eq!(planned, ["prd-free"], "{plan}");
        if blocked {
            assert!(
                plan["serial_queue"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|entry| entry["canonical_key"] == "prd-free"
                        && entry["reason"].as_str().unwrap().contains("rfc-pending")),
                "unclaimed blocked work must remain visible: {plan}"
            );
        }
        assert_eq!(plan["claimed_count"], 1);
        assert_eq!(std::fs::read(&claim_path).unwrap(), before);
        assert!(
            !tmp.path()
                .join(".forgeplan/claims/prd-claimed.yaml")
                .exists()
        );
        assert_eq!(
            sources.each_ref().map(|path| std::fs::read(path).unwrap()),
            source_before
        );
    }
}

#[tokio::test]
async fn dispatch_refuses_ambiguous_legacy_claims_without_reassigning_them() {
    let tmp = legacy_dispatch_workspace(false).await;
    let claim_path = make_legacy_dispatch_claim(tmp.path());
    let before = std::fs::read(&claim_path).unwrap();
    document(
        tmp.path(),
        "prds/prd-free.md",
        "id: PRD-001\nslug: prd-free\nkind: prd\nstatus: draft\ntitle: Other owner",
    );
    let output = cli()
        .args(["dispatch", "--agents", "2", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("prd-claimed") && error.contains("prd-free"),
        "{error}"
    );
    assert_eq!(std::fs::read(&claim_path).unwrap(), before);
    assert!(
        !tmp.path()
            .join(".forgeplan/claims/prd-claimed.yaml")
            .exists()
    );
}

#[tokio::test]
async fn dispatch_refuses_a_legacy_source_key_collision_without_picking_metadata() {
    let tmp = legacy_dispatch_workspace(false).await;
    document(
        tmp.path(),
        "prds/conflict.md",
        "id: PRD-003\nslug: prd-claimed\nkind: prd\nstatus: draft\ntitle: Other\n\
         affected_files: [src/unrelated.rs]",
    );
    let first = tmp.path().join(".forgeplan/prds/prd-claimed.md");
    let conflict = tmp.path().join(".forgeplan/prds/conflict.md");
    let before = [
        std::fs::read(&first).unwrap(),
        std::fs::read(&conflict).unwrap(),
    ];
    let output = cli()
        .args(["dispatch", "--agents", "2", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("prds/prd-claimed.md") && error.contains("prds/conflict.md"),
        "{error}"
    );
    assert_eq!(std::fs::read(&first).unwrap(), before[0]);
    assert_eq!(std::fs::read(&conflict).unwrap(), before[1]);
}

#[test]
fn legacy_export_import_can_delete_and_restore_one_document_without_changing_the_other() {
    let source = workspace();
    let target = workspace();
    for _ in 0..2 {
        cli()
            .args(["new", "note", "Еженедельный обзор", "--allow-duplicate"])
            .current_dir(source.path())
            .assert()
            .success();
    }
    let output = cli()
        .args(["list", "--json"])
        .current_dir(source.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let documents: Vec<Value> = serde_json::from_slice(&output).unwrap();
    assert_eq!(documents.len(), 2);
    let deleted_key = documents[0]["canonical_key"].as_str().unwrap();
    let survivor_key = documents[1]["canonical_key"].as_str().unwrap();
    assert_ne!(deleted_key, survivor_key);

    let bundle = source.path().join("export.json");
    cli()
        .arg("export")
        .arg("--output")
        .arg(&bundle)
        .current_dir(source.path())
        .assert()
        .success();
    cli()
        .arg("import")
        .arg(&bundle)
        .current_dir(target.path())
        .assert()
        .success();
    let notes = target.path().join(".forgeplan/notes");
    let deleted_path = notes.join(format!("{deleted_key}.md"));
    let survivor_path = notes.join(format!("{survivor_key}.md"));
    let deleted_bytes = std::fs::read(&deleted_path).unwrap();
    let survivor_bytes = std::fs::read(&survivor_path).unwrap();

    cli()
        .args(["delete", deleted_key, "--yes"])
        .current_dir(target.path())
        .assert()
        .success();
    assert!(!deleted_path.exists());
    cli()
        .args(["get", deleted_key, "--json"])
        .current_dir(target.path())
        .assert()
        .code(3);
    assert_eq!(std::fs::read(&survivor_path).unwrap(), survivor_bytes);

    cli()
        .args(["restore", deleted_key])
        .current_dir(target.path())
        .assert()
        .success();
    assert_eq!(std::fs::read(&deleted_path).unwrap(), deleted_bytes);
    assert_eq!(std::fs::read(&survivor_path).unwrap(), survivor_bytes);
    for (key, original) in [(deleted_key, &documents[0]), (survivor_key, &documents[1])] {
        let output = cli()
            .args(["get", key, "--json"])
            .current_dir(target.path())
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let restored: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(restored["canonical_key"], key);
        assert_eq!(restored["id"], original["id"]);
        assert_eq!(restored["title"], original["title"]);
    }
}

#[test]
fn full_source_import_normalizes_author_without_changing_source_or_blocking_delete_restore() {
    let target = workspace();
    for (id, key, author, expected_author) in [
        (
            "NOTE-010",
            "note-spaced-author",
            "\" Eli Rum \"",
            Some("Eli Rum"),
        ),
        ("NOTE-011", "note-empty-author", "\"\"", None),
    ] {
        let source = format!(
            "---\nid: {id}\nkind: note\nslug: {key}\nstatus: draft\ntitle: {key}\n\
             depth: tactical\nauthor: {author}\ncustom_field: retained\n---\n\n\
             Source document retained through deletion and restoration.\n"
        );
        let bundle = target.path().join(format!("{key}.json"));
        std::fs::write(
            &bundle,
            serde_json::to_vec(&serde_json::json!({"artifacts": [{
                "id": id,
                "canonical_key": key,
                "kind": "note",
                "status": "draft",
                "title": "Envelope title",
                "body": source,
            }]}))
            .unwrap(),
        )
        .unwrap();
        cli()
            .arg("import")
            .arg(&bundle)
            .current_dir(target.path())
            .assert()
            .success();
        let path = target
            .path()
            .join(".forgeplan/notes")
            .join(format!("{key}.md"));
        assert_eq!(std::fs::read(&path).unwrap(), source.as_bytes());
        let output = cli()
            .args(["get", key, "--json"])
            .current_dir(target.path())
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let imported: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(imported["title"], key);
        assert_eq!(
            imported["author"],
            expected_author.map(Value::from).unwrap_or(Value::Null)
        );
        cli()
            .args(["delete", key, "--yes"])
            .current_dir(target.path())
            .assert()
            .success();
        assert!(!path.exists());
        cli()
            .args(["restore", key])
            .current_dir(target.path())
            .assert()
            .success();
        assert_eq!(std::fs::read(&path).unwrap(), source.as_bytes());
    }
}

#[tokio::test]
async fn legacy_repair_refuses_to_guess_the_owner_of_an_ambiguous_indexed_relation() {
    let root = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let ws = forgeplan_core::db::migrate::populated_identity_v4_workspace_for_test(root.path())
        .await
        .unwrap();
    // Both documents claim the same outgoing edge. Its legacy row has only
    // display IDs, so publishing either ownership would invent information.
    std::fs::write(
        ws.join("prds/PRD-001-auth-policy.md"),
        "---\nid: PRD-001\nkind: prd\nstatus: active\ntitle: Authentication Policy\nslug: prd-auth-system\nlinks:\n  - target: RFC-001\n    relation: informs\n---\n\nPreserve session identity across credential rotation.\n",
    )
    .unwrap();
    let document_count = forgeplan_core::artifact::identity_repair::census_workspace(&ws)
        .unwrap()
        .documents
        .len();
    let plan_path = root.path().join("reviewed-plan.json");
    let plan_arg = plan_path.to_str().unwrap();
    cli()
        .args(["reconcile-ids", "--plan-out", plan_arg, "--json"])
        .current_dir(root.path())
        .env("HOME", home.path())
        .assert()
        .success();
    let apply = cli()
        .args(["reconcile-ids", "--apply-plan", plan_arg, "--json"])
        .current_dir(root.path())
        .env("HOME", home.path())
        .output()
        .unwrap();
    assert_eq!(apply.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&apply.stdout).unwrap();
    assert_eq!(
        report["observed_status"],
        "canonical_committed/index_pending"
    );
    assert_eq!(report["reconciliation_confirmed"], false);
    assert!(
        report["reconciliation_error"]
            .as_str()
            .unwrap()
            .contains("cannot be attributed uniquely")
    );
    let pending = forgeplan_core::artifact::identity_repair::census_workspace(&ws).unwrap();
    assert_eq!(pending.documents.len(), document_count);
    let refused_write = cli()
        .args(["new", "note", "Must not bypass unresolved repair"])
        .current_dir(root.path())
        .env("HOME", home.path())
        .output()
        .unwrap();
    assert!(!refused_write.status.success());
    let after = forgeplan_core::artifact::identity_repair::census_workspace(&ws).unwrap();
    assert_eq!(after.inventory_digest, pending.inventory_digest);
}

#[test]
fn managed_update_commits_while_another_index_owner_holds_publication() {
    let root = workspace();
    cli()
        .args(["new", "note", "Concurrent publication"])
        .current_dir(root.path())
        .assert()
        .success();
    let listed = cli()
        .args(["list", "--json"])
        .current_dir(root.path())
        .output()
        .unwrap();
    let listed: Value = serde_json::from_slice(&listed.stdout).unwrap();
    let key = listed[0]["canonical_key"].as_str().unwrap();
    cli()
        .arg("index")
        .current_dir(root.path())
        .assert()
        .success();
    let ws = root.path().join(".forgeplan");
    let owner_path = std::fs::read_dir(&ws)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name().unwrap().to_str().is_some_and(|name| {
                name.starts_with(".constellation-owner-") && name.ends_with(".lock")
            })
        })
        .unwrap();
    let owner = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(owner_path)
        .unwrap();
    fs2::FileExt::lock_exclusive(&owner).unwrap();
    let updated_body = "CommittedDuringIndexContention_20261110";
    let updated = cli()
        .args(["update", key, "--body", updated_body])
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(
        updated.status.success(),
        "canonical committed plus index contention must not fail the command: {}",
        String::from_utf8_lossy(&updated.stderr)
    );
    assert!(
        String::from_utf8_lossy(&updated.stderr).contains("index_pending: true"),
        "a successful canonical write must not claim publication while its owner is held"
    );
    let committed = forgeplan_core::artifact::identity_repair::census_workspace(&ws).unwrap();
    assert_eq!(committed.documents.len(), 1);
    assert_eq!(committed.documents[0].canonical_key, key);
    let canonical =
        std::fs::read_to_string(ws.join(&committed.documents[0].relative_path)).unwrap();
    assert!(canonical.contains(updated_body));
    drop(owner);
    cli()
        .arg("index")
        .current_dir(root.path())
        .assert()
        .success();
    let fetched = cli()
        .args(["get", key, "--json"])
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(fetched.status.success());
    let fetched: Value = serde_json::from_slice(&fetched.stdout).unwrap();
    assert_eq!(fetched["canonical_key"], key);
    assert!(fetched["body"].as_str().unwrap().contains(updated_body));
}
