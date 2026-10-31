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
fn validate_by_canonical_key_checks_only_that_shared_alias_document() {
    let temp = shared_alias_workspace();
    let output = forgeplan()
        .args(["validate", "prd-alpha", "--json"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
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
    for text in ["same memory text", "same memory text", "Память о важном"] {
        let output = forgeplan()
            .args(["remember", text])
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }

    let workspace = temp.path().join(".forgeplan/memory");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&workspace)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 3, "each remember call must create a distinct document");
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
