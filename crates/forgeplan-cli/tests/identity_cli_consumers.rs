//! Behavioral regressions for CLI consumers of canonical document identity.

use assert_cmd::Command;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init_workspace() -> TempDir {
    let temp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(temp.path())
        .assert()
        .success();
    temp
}

fn write_document(workspace: &Path, relative: &str, key: &str, title: &str, body: &str) {
    let path = workspace.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        path,
        format!(
            "---\nid: PRD-001\nslug: {key}\nkind: prd\nstatus: draft\ntitle: {title}\n---\n\n{body}\n"
        ),
    )
    .unwrap();
}

fn shared_alias_workspace() -> TempDir {
    let temp = init_workspace();
    let workspace = temp.path().join(".forgeplan");
    write_document(
        &workspace,
        "prds/PRD-001-alpha.md",
        "prd-alpha",
        "Shared title",
        "SharedNeedle Alpha body",
    );
    write_document(
        &workspace,
        "prds/PRD-001-beta.md",
        "prd-beta",
        "Shared title",
        "SharedNeedle Beta body",
    );
    forgeplan()
        .arg("index")
        .current_dir(temp.path())
        .assert()
        .success();
    temp
}

#[test]
fn graph_commands_keep_duplicate_alias_dependencies_separate() {
    let temp = shared_alias_workspace();
    let ws = temp.path().join(".forgeplan");
    let alpha = ws.join("prds/PRD-001-alpha.md");
    let text = std::fs::read_to_string(&alpha).unwrap().replace("status: draft", "status: active");
    std::fs::write(alpha, text).unwrap();
    let child = ws.join("rfcs/RFC-001-child.md");
    std::fs::create_dir_all(child.parent().unwrap()).unwrap();
    std::fs::write(&child, "---\nid: RFC-001\nslug: rfc-child\nkind: rfc\nstatus: draft\ntitle: Child\nlinks:\n  - target: prd-beta\n    relation: based_on\n---\n\nChild\n").unwrap();
    forgeplan().arg("index").current_dir(temp.path()).assert().success();
    let output = forgeplan().args(["blocked", "rfc-child", "--json"]).current_dir(temp.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["blocked"], true);
    assert_eq!(payload["blocked_by"], serde_json::json!(["PRD-001"]));
    assert_eq!(payload["canonical_blocked_by"], serde_json::json!(["prd-beta"]));
    let output = forgeplan().args(["tree", "prd-beta", "--json"]).current_dir(temp.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["canonical_key"], "prd-beta");
    assert!(payload["children"].as_array().unwrap().iter().any(|v| v["canonical_key"] == "rfc-child"));
    forgeplan().args(["tree", "PRD-001", "--json"]).current_dir(temp.path()).assert().failure();
    let output = forgeplan().args(["dispatch", "--agents", "1", "--json"]).current_dir(temp.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(payload["canonical_buckets"].as_array().unwrap().iter().all(|bucket| !bucket.as_array().unwrap().iter().any(|id| id == "rfc-child")));
}

#[test]
fn validate_by_canonical_key_checks_only_that_shared_alias_document() {
    let temp = shared_alias_workspace();
    let output = forgeplan()
        .args(["validate", "prd-alpha", "--json"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    // The fixture documents carry no required sections, so validation reports
    // errors and exits non-zero — what this checks is WHICH document it
    // validated, not whether the document is good.
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "validate --json must still answer ({e}); stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    let results = payload["results"].as_array().unwrap();
    assert_eq!(results.len(), 1, "exact-key validation must not include its alias sibling: {payload}");
    assert_eq!(results[0]["artifact_id"], "PRD-001");
}

#[test]
fn keyword_search_returns_each_shared_alias_hit_with_its_canonical_key() {
    let temp = shared_alias_workspace();
    let output = forgeplan()
        .args(["search", "SharedNeedle", "--keyword", "--json"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut keys: Vec<_> = payload["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|result| result["canonical_key"].as_str().unwrap())
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["prd-alpha", "prd-beta"]);
    let output = forgeplan()
        .args(["search", "SharedNeedle", "--no-expand", "--json"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut keys: Vec<_> = payload["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|result| result["canonical_key"].as_str().unwrap())
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["prd-alpha", "prd-beta"]);

}

#[tokio::test]
async fn forced_import_replaces_two_equal_alias_documents_by_key_not_alias_or_title() {
    let temp = shared_alias_workspace();
    let import_path = temp.path().join("identity-bundle.json");
    let bundle = serde_json::json!({
        "artifacts": [
            {"id":"PRD-001","canonical_key":"prd-alpha","kind":"prd","status":"draft","title":"Shared title","body":"Imported Alpha body"},
            {"id":"PRD-001","canonical_key":"prd-beta","kind":"prd","status":"draft","title":"Shared title","body":"Imported Beta body"}
        ],
        "relations": []
    });
    std::fs::write(&import_path, serde_json::to_vec(&bundle).unwrap()).unwrap();
    let output = forgeplan()
        .args(["import", import_path.to_str().unwrap(), "--force"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let workspace = temp.path().join(".forgeplan");
    let docs = forgeplan_core::artifact::documents::workspace_candidates(&workspace)
        .await
        .unwrap();
    let mut actual: Vec<_> = docs
        .iter()
        .filter(|doc| doc.canonical_key.starts_with("prd-"))
        .map(|doc| {
            let content = std::fs::read_to_string(workspace.join(&doc.relative_path)).unwrap();
            (doc.canonical_key.as_str(), content)
        })
        .collect();
    actual.sort_by_key(|(key, _)| *key);
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0].0, "prd-alpha");
    assert!(actual[0].1.contains("Imported Alpha body"));
    assert_eq!(actual[1].0, "prd-beta");
    assert!(actual[1].1.contains("Imported Beta body"));
}

#[test]
fn repeated_and_cyrillic_memories_publish_distinct_keys_without_overwriting() {
    let temp = init_workspace();
    // Count the delta, not the directory: the workspace may already hold a
    // memory document before this test runs.
    let memory_dir = temp.path().join(".forgeplan/memory");
    let before: Vec<PathBuf> = std::fs::read_dir(&memory_dir)
        .map(|entries| entries.map(|entry| entry.unwrap().path()).collect())
        .unwrap_or_default();
    for text in ["same memory text", "same memory text", "Память о важном"] {
        let output = forgeplan()
            .args(["remember", text])
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }

    let workspace = memory_dir.clone();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&workspace)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| !before.contains(path))
        .collect();
    paths.sort();
    assert_eq!(
        paths.len(),
        3,
        "each remember call must create exactly one new document"
    );
    let mut keys = Vec::new();
    let mut bodies = Vec::new();
    for path in paths {
        let content = std::fs::read_to_string(path).unwrap();
        let (frontmatter, body) = forgeplan_core::artifact::frontmatter::parse_frontmatter(&content).unwrap();
        keys.push(frontmatter.get("slug").and_then(serde_yaml::Value::as_str).unwrap().to_string());
        bodies.push(body.to_string());
    }
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 3, "minted memory keys must be unique");
    assert_eq!(bodies.iter().filter(|body| body.contains("same memory text")).count(), 2);
    assert_eq!(bodies.iter().filter(|body| body.contains("Память о важном")).count(), 1);
}

#[test]
fn dispatch_keeps_the_unclaimed_shared_alias_document_independent() {
    let temp = init_workspace();
    let workspace = temp.path().join(".forgeplan");
    write_document(
        &workspace,
        "prds/PRD-001-alpha.md",
        "prd-alpha",
        "Alpha",
        "## Affected Files\n\n- src/alpha.rs",
    );
    write_document(
        &workspace,
        "prds/PRD-001-beta.md",
        "prd-beta",
        "Beta",
        "## Affected Files\n\n- src/beta.rs",
    );
    forgeplan()
        .arg("index")
        .current_dir(temp.path())
        .assert()
        .success();
    forgeplan()
        .args(["claim", "prd-alpha", "--agent", "agent-alpha"])
        .current_dir(temp.path())
        .assert()
        .success();

    let output = forgeplan()
        .args(["dispatch", "--agents", "1", "--json"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["candidate_count"], 2);
    assert_eq!(payload["claimed_count"], 1);
    let bucket_ids: Vec<_> = payload["canonical_buckets"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|bucket| bucket.as_array().unwrap())
        .filter_map(Value::as_str)
        .collect();
    assert!(bucket_ids.contains(&"prd-beta"));
    assert!(!bucket_ids.contains(&"prd-alpha"));
}

#[tokio::test]
async fn import_uses_legacy_body_slug_when_the_export_has_no_canonical_key() {
    let temp = init_workspace();
    let import_path = temp.path().join("legacy-bundle.json");
    let body = "---\nid: PRD-009\nslug: prd-legacy-key\nkind: prd\nstatus: draft\ntitle: Legacy\n---\n\nLegacy body";
    let bundle = serde_json::json!({
        "artifacts": [{
            "id": "PRD-009",
            "kind": "prd",
            "status": "draft",
            "title": "Legacy",
            "body": body
        }],
        "relations": []
    });
    std::fs::write(&import_path, serde_json::to_vec(&bundle).unwrap()).unwrap();
    let output = forgeplan()
        .args(["import", import_path.to_str().unwrap()])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let workspace = temp.path().join(".forgeplan");
    let docs = forgeplan_core::artifact::documents::workspace_candidates(&workspace)
        .await
        .unwrap();
    let imported = docs
        .iter()
        .find(|document| document.display_id == "PRD-009")
        .unwrap();
    assert_eq!(imported.canonical_key, "prd-legacy-key");
    assert!(std::fs::read_to_string(workspace.join(&imported.relative_path))
        .unwrap()
        .contains("Legacy body"));
}

#[test]
fn update_refuses_external_key_substitution_before_title_or_file_changes() {
    let temp = init_workspace();
    let workspace = temp.path().join(".forgeplan");
    write_document(
        &workspace,
        "prds/PRD-001-alpha.md",
        "prd-alpha",
        "Original title",
        "Original body",
    );
    forgeplan()
        .arg("index")
        .current_dir(temp.path())
        .assert()
        .success();

    let path = workspace.join("prds/PRD-001-alpha.md");
    let original = std::fs::read(&path).unwrap();
    let payload = temp.path().join("replacement.md");
    std::fs::write(
        &payload,
        "---\nslug: another-key\n---\n\nReplacement body\n",
    )
    .unwrap();
    let body_arg = format!("@{}", payload.display());
    let output = forgeplan()
        .args([
            "update",
            "prd-alpha",
            "--title",
            "Changed title",
            "--body",
            body_arg.as_str(),
        ])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(!output.status.success(), "a key substitution must be refused");
    assert!(String::from_utf8_lossy(&output.stderr).contains("canonical key change refused"));
    assert_eq!(std::fs::read(&path).unwrap(), original, "refusal must preserve exact file bytes");
}
#[tokio::test]
async fn legacy_store_update_refuses_key_substitution_before_metadata_writes() {
    let temp = init_workspace();
    let created = forgeplan()
        .args(["new", "prd", "Store update target"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(created.status.success(), "{}", String::from_utf8_lossy(&created.stderr));
    let workspace = temp.path().join(".forgeplan");
    let documents = forgeplan_core::artifact::documents::workspace_candidates(&workspace)
        .await
        .unwrap();
    let document = documents
        .iter()
        .find(|document| document.title == "Store update target")
        .unwrap();
    let key = document.canonical_key.clone();
    let path = workspace.join(&document.relative_path);
    let original = std::fs::read(&path).unwrap();
    let payload = temp.path().join("replacement.md");
    std::fs::write(&payload, "---\nslug: different-key\n---\n\nReplacement body\n").unwrap();
    let body_arg = format!("@{}", payload.display());
    let output = forgeplan()
        .args([
            "update",
            &key,
            "--title",
            "Changed store title",
            "--body",
            body_arg.as_str(),
        ])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(!output.status.success(), "a key substitution must be refused");
    assert!(String::from_utf8_lossy(&output.stderr).contains("canonical key change refused"));
    assert_eq!(std::fs::read(&path).unwrap(), original, "refusal must preserve file bytes");
}

#[tokio::test]
async fn get_and_calibrate_keep_shared_alias_relations_separate_on_both_layouts() {
    use forgeplan_core::db::store::{LanceStore, NewArtifact};
    use forgeplan_core::projection;

    for migrated in [false, true] {
        let temp = init_workspace();
        let ws = temp.path().join(".forgeplan");
        let mut links = String::new();
        for n in 1..=6 {
            let id = format!("NOTE-{n:03}");
            links.push_str(&format!("  - target: {id}\n    relation: based_on\n"));
            std::fs::write(ws.join("notes").join(format!("{id}.md")), format!(
                "---\nid: {id}\nkind: note\nstatus: draft\ntitle: {id}\n---\n\nTarget.\n"
            )).unwrap();
        }
        std::fs::write(ws.join("prds/alpha.md"), format!(
            "---\nid: PRD-001\nslug: prd-alpha\nkind: prd\nstatus: draft\n\
             title: Alpha\ndepth: tactical\nlinks:\n{links}---\n\nAlpha body.\n"
        )).unwrap();
        std::fs::write(ws.join("prds/beta.md"),
            "---\nid: PRD-001\nslug: prd-beta\nkind: prd\nstatus: draft\n\
             title: Beta\ndepth: tactical\nlinks:\n  - target: prd-alpha\n\
             \x20   relation: refines\n---\n\nBeta body.\n").unwrap();
        if migrated {
            forgeplan().arg("index").current_dir(temp.path()).assert().success();
        } else {
            let guard = forgeplan_core::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
            let store = LanceStore::open(&ws).await.unwrap();
            let read = forgeplan_core::constellation::facade::open_workspace_read_with_permit(&ws, &guard).await.unwrap();
            let ctx = projection::MutationContext::new(&ws, &guard, Some(&store));
            for record in read.records() {
                projection::sync_artifact_from_file(&ctx, &NewArtifact {
                    canonical_key: record.canonical_key.clone(),
                    id: record.id.clone(),
                    kind: record.kind.clone(),
                    status: record.status.clone(),
                    title: record.title.clone(),
                    body: record.body.clone(),
                    depth: record.depth.clone(),
                    author: record.author.clone(),
                    parent_epic: record.parent_epic.clone(),
                    valid_until: record.valid_until.clone(),
                    tags: record.tags.clone(),
                }).await.unwrap();
            }
            for (from, to, relation) in read.get_all_relations() {
                projection::sync_relation_from_file(&ctx, &from, &to, &relation).await.unwrap();
            }
        }
        for (key, outbound, inbound) in [("prd-alpha", 6, 1), ("prd-beta", 1, 0)] {
            let output = forgeplan().args(["get", key, "--json"])
                .current_dir(temp.path()).output().unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            let json: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(json["canonical_key"], key);
            assert_eq!(json["links"]["outbound"].as_array().unwrap().len(), outbound);
            assert_eq!(json["links"]["inbound"].as_array().unwrap().len(), inbound);
            let calibration = forgeplan().args(["calibrate", key])
                .current_dir(temp.path()).output().unwrap();
            assert!(calibration.status.success(), "{}", String::from_utf8_lossy(&calibration.stderr));
            let text = String::from_utf8(calibration.stdout).unwrap();
            assert_eq!(text.contains("dependency_links (6 links)"), key == "prd-alpha", "{text}");
        }
        let ambiguous = forgeplan().args(["calibrate", "PRD-001"])
            .current_dir(temp.path()).output().unwrap();
        assert_eq!(ambiguous.status.code(), Some(5));
        let error = String::from_utf8_lossy(&ambiguous.stderr);
        assert!(error.contains("prd-alpha") && error.contains("prd-beta"), "{error}");
    }
}

#[test]
fn migrated_coverage_backfill_updates_both_shared_alias_documents() {
    let temp = init_workspace();
    let ws = temp.path().join(".forgeplan");
    for (key, marker) in [("prd-alpha", "Alpha body"), ("prd-beta", "Beta body")] {
        std::fs::write(ws.join("prds").join(format!("{key}.md")), format!(
            "---\nid: PRD-001\nslug: {key}\nkind: prd\nstatus: active\n\
             title: {key}\n---\n\n{marker}\n"
        )).unwrap();
    }
    forgeplan().arg("index").current_dir(temp.path()).assert().success();
    let output = forgeplan().args(["coverage", "--backfill"])
        .current_dir(temp.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Backfilled 2 artifact(s)"));
    for (key, marker) in [("prd-alpha", "Alpha body"), ("prd-beta", "Beta body")] {
        let text = std::fs::read_to_string(ws.join("prds").join(format!("{key}.md"))).unwrap();
        assert!(text.contains(marker) && text.contains("## Affected Files"), "{text}");
    }
}

fn snapshot_files(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, path: &Path, files: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>) {
        if !path.exists() {
            return;
        }
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, files);
            } else if path.is_file() {
                files.insert(path.strip_prefix(root).unwrap().to_owned(), std::fs::read(path).unwrap());
            }
        }
    }
    let mut files = std::collections::BTreeMap::new();
    visit(root, root, &mut files);
    files
}

#[test]
fn reindex_refuses_ambiguous_relation_before_index_or_audit_changes() {
    let temp = shared_alias_workspace();
    let ws = temp.path().join(".forgeplan");
    let source = ws.join("prds/prd-source.md");
    std::fs::write(&source, "---\nid: PRD-002\nslug: prd-source\nkind: prd\nstatus: draft\ntitle: Source\nlinks:\n  - target: PRD-001\n    relation: based_on\n---\n\nSource\n").unwrap();
    let before = snapshot_files(&ws.join("lance"));
    let output = forgeplan().arg("reindex").current_dir(temp.path()).output().unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("PRD-001"), "{error}");
    assert_eq!(before, snapshot_files(&ws.join("lance")));
    assert!(std::fs::read_to_string(source).unwrap().contains("target: PRD-001"));
}

#[test]
fn git_sync_refuses_ambiguous_relation_before_index_or_audit_changes() {
    let temp = shared_alias_workspace();
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git").args(args).current_dir(temp.path()).output().unwrap();
        assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    };
    git(&["init"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.invalid"]);
    git(&["commit", "--allow-empty", "-m", "before"]);
    let ws = temp.path().join(".forgeplan");
    let path = ws.join("prds/prd-source.md");
    std::fs::write(&path, "---\nid: PRD-002\nslug: prd-source\nkind: prd\nstatus: draft\ntitle: Source\nlinks:\n  - target: PRD-001\n    relation: based_on\n---\n\nSource\n").unwrap();
    git(&["add", "-f", ".forgeplan/prds/prd-source.md"]);
    git(&["commit", "-m", "add ambiguous relation"]);
    let before = snapshot_files(&ws.join("lance"));
    let output = forgeplan().args(["git-sync", "--since", "HEAD~1"])
        .current_dir(temp.path()).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("PRD-001"));
    assert_eq!(before, snapshot_files(&ws.join("lance")));
}

#[test]
fn batch_reindex_validates_qualified_targets_before_index_and_audit_writes() {
    for target in ["prd-alpha@", "prd-alpha@bad@qualifier"] {
        let temp = shared_alias_workspace();
        let ws = temp.path().join(".forgeplan");
        std::fs::write(ws.join("prds/prd-source.md"), format!(
            "---\nid: PRD-002\nslug: prd-source\nkind: prd\nstatus: draft\ntitle: Source\nlinks:\n  - target: '{target}'\n    relation: based_on\n---\n\nSource\n"
        )).unwrap();
        let before = snapshot_files(&ws.join("lance"));
        let output = forgeplan().arg("reindex").current_dir(temp.path()).output().unwrap();
        assert!(!output.status.success(), "{target}");
        assert_eq!(before, snapshot_files(&ws.join("lance")), "{target}");
    }
}


#[test]
fn git_sync_rejects_malformed_qualified_target_before_index_and_audit_writes() {
    let temp = shared_alias_workspace();
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git").args(args).current_dir(temp.path()).output().unwrap();
        assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    };
    git(&["init"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.invalid"]);
    git(&["commit", "--allow-empty", "-m", "before"]);
    let ws = temp.path().join(".forgeplan");
    std::fs::write(ws.join("prds/prd-source.md"),
        "---\nid: PRD-002\nslug: prd-source\nkind: prd\nstatus: draft\ntitle: Source\nlinks:\n  - target: 'prd-alpha@'\n    relation: based_on\n---\n\nSource\n").unwrap();
    git(&["add", "-f", ".forgeplan/prds/prd-source.md"]);
    git(&["commit", "-m", "add malformed relation"]);
    let before = snapshot_files(&ws.join("lance"));
    let output = forgeplan().args(["git-sync", "--since", "HEAD~1"]).current_dir(temp.path()).output().unwrap();
    assert!(!output.status.success());
    assert_eq!(before, snapshot_files(&ws.join("lance")));
}

