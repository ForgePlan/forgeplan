use assert_cmd::Command;
use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::projection;
use std::path::Path;
use tempfile::TempDir;

fn forgeplan(root: &Path, args: &[&str]) -> std::process::Output {
    Command::cargo_bin("forgeplan").unwrap().args(args).current_dir(root).output().unwrap()
}

fn init_workspace() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let out = forgeplan(tmp.path(), &["init", "-y"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    tmp
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args).current_dir(root).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

#[tokio::test]
async fn lifecycle_by_key_preserves_each_shared_alias_document_and_its_links() {
    for (command, initial, expected) in [
        ("activate", "draft", "active"),
        ("deprecate", "active", "deprecated"),
        ("renew", "stale", "active"),
        ("supersede", "active", "superseded"),
    ] {
        let tmp = init_workspace();
        let ws = tmp.path().join(".forgeplan");
        let alpha = ws.join("prds/prd-alpha.md");
        let beta = ws.join("prds/prd-beta.md");
        std::fs::write(&alpha, format!(
            "---\nid: PRD-001\nslug: prd-alpha\nkind: prd\nstatus: {initial}\ntitle: Alpha\n\
             depth: standard\nlinks:\n  - target: NOTE-001\n    relation: based_on\n---\n\nAlpha body.\n"
        )).unwrap();
        std::fs::write(&beta,
            "---\nid: PRD-001\nslug: prd-beta\nkind: prd\nstatus: active\ntitle: Beta\n---\n\nBeta body.\n").unwrap();
        std::fs::write(ws.join("prds/PRD-002.md"),
            "---\nid: PRD-002\nkind: prd\nstatus: active\ntitle: Replacement\n---\n\nReplacement.\n").unwrap();
        std::fs::write(ws.join("notes/NOTE-001.md"),
            "---\nid: NOTE-001\nkind: note\nstatus: active\ntitle: Target\n---\n\nTarget.\n").unwrap();
        let beta_before = std::fs::read(&beta).unwrap();
        {
            let guard = forgeplan_core::workspace::lock::acquire_workspace_lock(&ws).await.unwrap();
            let store = LanceStore::open(&ws).await.unwrap();
            let read = forgeplan_core::constellation::facade::open_workspace_read_with_permit(&ws, &guard).await.unwrap();
            let ctx = projection::MutationContext::new(&ws, &guard, Some(&store));
            for record in read.records() {
                projection::sync_artifact_from_file(&ctx, &NewArtifact {
                    canonical_key: record.canonical_key.clone(), id: record.id.clone(),
                    kind: record.kind.clone(), status: record.status.clone(), title: record.title.clone(),
                    body: record.body.clone(), depth: record.depth.clone(), author: record.author.clone(),
                    parent_epic: record.parent_epic.clone(), valid_until: record.valid_until.clone(),
                    tags: record.tags.clone(),
                }).await.unwrap();
            }
            for (from, to, relation) in read.get_all_relations() {
                projection::sync_relation_from_file(&ctx, &from, &to, &relation).await.unwrap();
            }
        }
        let args: Vec<&str> = match command {
            "activate" => vec!["activate", "prd-alpha", "--force"],
            "deprecate" => vec!["deprecate", "prd-alpha", "--reason", "replaced"],
            "renew" => vec!["renew", "prd-alpha", "--reason", "still valid", "--until", "2099-12-31"],
            "supersede" => vec!["supersede", "prd-alpha", "--by", "PRD-002"],
            _ => unreachable!(),
        };
        let out = forgeplan(tmp.path(), &args);
        assert!(out.status.success(), "{command}: {}", String::from_utf8_lossy(&out.stderr));
        let content = std::fs::read_to_string(&alpha).unwrap();
        let (fm, body) = forgeplan_core::artifact::frontmatter::parse_frontmatter(&content).unwrap();
        assert_eq!(fm["status"].as_str(), Some(expected));
        assert_eq!(fm["slug"].as_str(), Some("prd-alpha"));
        assert!(body.contains("Alpha body"));
        let links = fm["links"].as_sequence().unwrap();
        assert!(links.iter().any(|edge| edge["target"].as_str() == Some("NOTE-001")));
        if command == "supersede" {
            assert!(links.iter().any(|edge| edge["relation"].as_str() == Some("supersedes")));
        }
        assert_eq!(std::fs::read(&beta).unwrap(), beta_before, "{command} changed sibling");
    }
}

fn invalid_relation_document() -> &'static str {
    "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Invalid relation\n\
     links:\n  - target: invalid/path\n    relation: based_on\n---\n\nBody.\n"
}

#[test]
fn reindex_fails_instead_of_reporting_success_when_relation_sync_fails() {
    let tmp = init_workspace();
    std::fs::write(tmp.path().join(".forgeplan/prds/PRD-001-invalid.md"), invalid_relation_document()).unwrap();
    let out = forgeplan(tmp.path(), &["reindex"]);
    assert!(!out.status.success(), "reindex must not swallow invalid relation");
    assert!(String::from_utf8_lossy(&out.stderr).contains("invalid/path"));
}

#[test]
fn git_sync_fails_instead_of_reporting_success_when_relation_sync_fails() {
    let tmp = init_workspace();
    git(tmp.path(), &["init", "-q"]);
    git(tmp.path(), &["-c", "user.name=Test", "-c", "user.email=test@example.invalid", "commit", "--allow-empty", "-qm", "baseline"]);
    let baseline = git(tmp.path(), &["rev-parse", "HEAD"]);
    let file = ".forgeplan/prds/PRD-001-invalid.md";
    std::fs::write(tmp.path().join(file), invalid_relation_document()).unwrap();
    git(tmp.path(), &["add", "-f", file]);
    git(tmp.path(), &["-c", "user.name=Test", "-c", "user.email=test@example.invalid", "commit", "-qm", "add invalid relation"]);
    let out = forgeplan(tmp.path(), &["git-sync", "--since", &baseline]);
    assert!(!out.status.success(), "git-sync must not swallow invalid relation");
    assert!(String::from_utf8_lossy(&out.stderr).contains("invalid/path"));
}

fn keyed_document(key: &str, title: &str, target: &str, body: &str) -> String {
    format!("---\nid: PRD-001\nslug: {key}\nkind: prd\nstatus: draft\ntitle: {title}\n\
        links:\n  - target: {target}\n    relation: based_on\n---\n\n{body}\n")
}

fn write_targets(ws: &Path) {
    for id in ["NOTE-001", "NOTE-002"] {
        std::fs::write(ws.join("notes").join(format!("{id}.md")), format!(
            "---\nid: {id}\nkind: note\nstatus: draft\ntitle: {id}\n---\n\nTarget.\n"
        )).unwrap();
    }
}

fn assert_keyed_result(root: &Path, key: &str, body: &str, target: &str) {
    let out = forgeplan(root, &["get", key, "--json"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(payload["canonical_key"], key);
    assert!(payload["body"].as_str().unwrap().contains(body), "{payload}");
    let edges = payload["links"]["outbound"].as_array().unwrap();
    assert_eq!(edges.len(), 1, "{payload}");
    assert_eq!(edges[0]["target_key"], target);
}

#[test]
fn legacy_reindex_keeps_two_shared_alias_documents_and_edges_separate() {
    let tmp = init_workspace();
    let ws = tmp.path().join(".forgeplan");
    write_targets(&ws);
    for (key, title, target) in [
        ("prd-alpha", "Alpha", "NOTE-001"),
        ("prd-beta", "Beta", "NOTE-002"),
    ] {
        std::fs::write(ws.join("prds").join(format!("{key}.md")),
            keyed_document(key, title, target, &format!("{title} initial"))).unwrap();
    }
    let out = forgeplan(tmp.path(), &["reindex"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(!String::from_utf8_lossy(&out.stderr).contains("last-writer-wins"));
    assert_keyed_result(tmp.path(), "prd-alpha", "Alpha initial", "NOTE-001");
    assert_keyed_result(tmp.path(), "prd-beta", "Beta initial", "NOTE-002");
    std::fs::write(ws.join("prds/prd-alpha.md"),
        keyed_document("prd-alpha", "Alpha", "NOTE-001", "Alpha revised")).unwrap();
    let out = forgeplan(tmp.path(), &["reindex"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_keyed_result(tmp.path(), "prd-alpha", "Alpha revised", "NOTE-001");
    assert_keyed_result(tmp.path(), "prd-beta", "Beta initial", "NOTE-002");
}

#[test]
fn git_sync_uses_canonical_key_for_existing_and_new_shared_alias_documents() {
    for migrated in [false, true] {
        let tmp = init_workspace();
        let ws = tmp.path().join(".forgeplan");
        write_targets(&ws);
        std::fs::write(ws.join("prds/prd-alpha.md"),
            keyed_document("prd-alpha", "Alpha", "NOTE-001", "Alpha initial")).unwrap();
        git(tmp.path(), &["init", "-q"]);
        git(tmp.path(), &["add", "-f", ".forgeplan/prds/prd-alpha.md",
            ".forgeplan/notes/NOTE-001.md", ".forgeplan/notes/NOTE-002.md"]);
        git(tmp.path(), &["-c", "user.name=Test", "-c", "user.email=test@example.invalid",
            "commit", "-qm", "initial documents"]);
        let baseline = git(tmp.path(), &["rev-parse", "HEAD"]);
        let index_command = if migrated { "index" } else { "reindex" };
        let out = forgeplan(tmp.path(), &[index_command]);
        assert!(out.status.success(), "{index_command}: {}", String::from_utf8_lossy(&out.stderr));
        std::fs::write(ws.join("prds/prd-alpha.md"),
            keyed_document("prd-alpha", "Alpha", "NOTE-001", "Alpha revised")).unwrap();
        std::fs::write(ws.join("prds/prd-beta.md"),
            keyed_document("prd-beta", "Beta", "NOTE-002", "Beta added")).unwrap();
        git(tmp.path(), &["add", "-f", ".forgeplan/prds/prd-alpha.md", ".forgeplan/prds/prd-beta.md"]);
        git(tmp.path(), &["-c", "user.name=Test", "-c", "user.email=test@example.invalid",
            "commit", "-qm", "update and add"]);
        let out = forgeplan(tmp.path(), &["git-sync", "--since", &baseline]);
        assert!(out.status.success(), "migrated={migrated}: {}", String::from_utf8_lossy(&out.stderr));
        assert_keyed_result(tmp.path(), "prd-alpha", "Alpha revised", "NOTE-001");
        assert_keyed_result(tmp.path(), "prd-beta", "Beta added", "NOTE-002");
    }
}

#[tokio::test]
async fn reindex_detects_deleted_historical_file_by_key_not_shared_id_prefix() {
    for migrated in [false, true] {
        let tmp = init_workspace();
        let ws = tmp.path().join(".forgeplan");
        write_targets(&ws);
        let alpha = ws.join("prds/PRD-001-alpha.md");
        let beta = ws.join("prds/PRD-001-beta.md");
        std::fs::write(&alpha, keyed_document("prd-alpha", "Alpha", "NOTE-001", "Alpha body")).unwrap();
        std::fs::write(&beta, keyed_document("prd-beta", "Beta", "NOTE-002", "Beta body")).unwrap();
        let before_beta = std::fs::read(&beta).unwrap();
        let index_command = if migrated { "index" } else { "reindex" };
        let out = forgeplan(tmp.path(), &[index_command]);
        assert!(out.status.success(), "{index_command}: {}", String::from_utf8_lossy(&out.stderr));

        std::fs::remove_file(&alpha).unwrap();
        // A partial source scan must never be treated as proof of absence.
        let broken = ws.join("notes/NOTE-900-broken.md");
        std::fs::write(&broken, "---\nid: [unterminated\n---\n").unwrap();
        let refused = forgeplan(tmp.path(), &["reindex"]);
        assert!(!refused.status.success(), "an incomplete capture must refuse orphan cleanup");
        if !migrated {
            let permit = forgeplan_core::workspace::lock::acquire_workspace_read_lock(&ws).await.unwrap();
            let store = LanceStore::open(&ws).await.unwrap();
            let records = store.list_records(&permit, None).await.unwrap();
            assert!(records.iter().any(|record| record.canonical_key == "prd-alpha"));
        }
        std::fs::remove_file(&broken).unwrap();

        let out = forgeplan(tmp.path(), &["reindex"]);
        assert!(out.status.success(), "migrated={migrated}: {}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(std::fs::read(&beta).unwrap(), before_beta);
        assert_keyed_result(tmp.path(), "prd-beta", "Beta body", "NOTE-002");
        if migrated {
            let out = forgeplan(tmp.path(), &["log", "--source", "reindex", "--limit", "1000", "--json"]);
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
            let log: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
            let deletes: Vec<_> = log["entries"].as_array().unwrap().iter()
                .filter(|entry| entry["action"] == "delete").collect();
            assert_eq!(deletes.len(), 1, "{log}");
            assert_eq!(deletes[0]["canonical_key"], "prd-alpha");
        } else {
            let permit = forgeplan_core::workspace::lock::acquire_workspace_read_lock(&ws).await.unwrap();
            let store = LanceStore::open(&ws).await.unwrap();
            let records = store.list_records(&permit, None).await.unwrap();
            assert!(!records.iter().any(|record| record.canonical_key == "prd-alpha"));
            assert!(records.iter().any(|record| record.canonical_key == "prd-beta"));
        }
    }
}
