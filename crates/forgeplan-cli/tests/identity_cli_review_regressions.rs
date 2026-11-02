use assert_cmd::Command;
use serde_json::Value;
use std::path::Path;
use tempfile::TempDir;

fn cli() -> Command { Command::cargo_bin("forgeplan").unwrap() }

fn workspace() -> TempDir {
    let tmp = TempDir::new().unwrap();
    cli().args(["init", "-y"]).current_dir(tmp.path()).assert().success();
    tmp
}

fn document(root: &Path, path: &str, frontmatter: &str) {
    let path = root.join(".forgeplan").join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, format!("---\n{frontmatter}\n---\n\nBody\n")).unwrap();
}

#[test]
fn validate_explicit_selectors_keep_typed_errors_and_canonical_identity() {
    let tmp = workspace();
    for (selector, code) in [("PRD-123", 3), ("bad/selector", 1)] {
        let output = cli().args(["validate", selector, "--json"]).current_dir(tmp.path()).output().unwrap();
        assert_eq!(output.status.code(), Some(code));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("No artifacts found"));
    }
    for key in ["prd-alpha", "prd-beta"] {
        document(tmp.path(), &format!("prds/{key}.md"), &format!("id: PRD-001\nslug: {key}\nkind: prd\nstatus: draft\ntitle: {key}"));
    }
    cli().arg("index").current_dir(tmp.path()).assert().success();
    let ambiguous = cli().args(["validate", "PRD-001", "--json"]).current_dir(tmp.path()).output().unwrap();
    assert_eq!(ambiguous.status.code(), Some(5));
    let stderr = String::from_utf8_lossy(&ambiguous.stderr);
    assert!(stderr.contains("prd-alpha") && stderr.contains("prd-beta"), "{stderr}");
    let output = cli().args(["validate", "prd-alpha", "--json"]).current_dir(tmp.path()).output().unwrap();
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
    document(tmp.path(), "epics/EPIC-001-main.md", "id: EPIC-001\nslug: epic-main\nkind: epic\nstatus: draft\ntitle: Main");
    document(tmp.path(), "rfcs/RFC-001-child.md", "id: RFC-001\nslug: rfc-child\nkind: rfc\nstatus: draft\ntitle: Child\nparent_epic: epic-1");
    cli().arg("index").current_dir(tmp.path()).assert().success();
    let output = cli().args(["tree", "epic-main", "--json"]).current_dir(tmp.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["children"].as_array().unwrap().iter().any(|v| v["canonical_key"] == "rfc-child"));
    document(tmp.path(), "epics/EPIC-001-other.md", "id: EPIC-001\nslug: epic-other\nkind: epic\nstatus: draft\ntitle: Other");
    let output = cli().args(["tree", "--json"]).current_dir(tmp.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(5));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("epic-main") && stderr.contains("epic-other"), "{stderr}");
}

#[test]
fn tree_preserves_valid_unresolved_structural_parent() {
    let tmp = workspace();
    document(tmp.path(), "rfcs/RFC-001-child.md", "id: RFC-001\nslug: rfc-child\nkind: rfc\nstatus: draft\ntitle: Child\nparent_epic: RFC-A01");
    document(tmp.path(), "rfcs/RFC-002-remote.md", "id: RFC-002\nslug: rfc-remote\nkind: rfc\nstatus: draft\ntitle: Remote\nparent_epic: EPIC-001@remote");
    cli().arg("index").current_dir(tmp.path()).assert().success();
    let output = cli().args(["tree", "--json"]).current_dir(tmp.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let roots: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(roots.as_array().unwrap().iter().any(|root| root["canonical_key"] == "rfc-child"));
    assert!(roots.as_array().unwrap().iter().any(|root| root["canonical_key"] == "rfc-remote"));
}

#[test]
fn keyword_search_applies_composite_filters_before_limit() {
    let tmp = workspace();
    document(tmp.path(), "prds/PRD-001-draft.md", "id: PRD-001\nslug: prd-draft\nkind: prd\nstatus: draft\ndepth: deep\ntitle: Needle");
    document(tmp.path(), "prds/PRD-002-active.md", "id: PRD-002\nslug: prd-active\nkind: prd\nstatus: active\ndepth: deep\ntitle: Needle");
    document(tmp.path(), "prds/PRD-003-standard.md", "id: PRD-003\nslug: prd-standard\nkind: prd\nstatus: active\ndepth: standard\ntitle: Needle");
    document(tmp.path(), "prds/PRD-004-active.md", "id: PRD-004\nslug: prd-active-two\nkind: prd\nstatus: active\ndepth: deep\ntitle: Needle");
    cli().arg("index").current_dir(tmp.path()).assert().success();
    let output = cli().args(["search", "Needle", "--keyword", "--status", "active", "--depth", "deep", "--limit", "1", "--json"])
        .current_dir(tmp.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let results = json["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert!(["prd-active", "prd-active-two"].contains(&results[0]["canonical_key"].as_str().unwrap()));
    assert_eq!(json["_next_action"], format!("forgeplan get {}", results[0]["canonical_key"].as_str().unwrap()));
    let output = cli().args(["search", "Needle", "--keyword", "--limit", "4", "--json"])
        .current_dir(tmp.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let unfiltered: Value = serde_json::from_slice(&output.stdout).unwrap();
    let first_status = unfiltered["results"][0]["status"].as_str().unwrap();
    let excluded_first = if first_status == "active" { "draft" } else { "active" };
    let output = cli().args(["search", "Needle", "--keyword", "--status", excluded_first, "--limit", "1", "--json"])
        .current_dir(tmp.path()).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let filtered: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(filtered["results"].as_array().unwrap().len(), 1);
    assert_eq!(filtered["results"][0]["status"], excluded_first);
}

#[cfg(not(feature = "semantic-search"))]
#[test]
fn semantic_search_never_returns_lexical_hits_when_backend_is_unavailable() {
    let tmp = workspace();
    document(tmp.path(), "prds/PRD-001-alpha.md", "id: PRD-001\nslug: prd-alpha\nkind: prd\nstatus: draft\ntitle: Needle");
    cli().arg("index").current_dir(tmp.path()).assert().success();
    let output = cli().args(["search", "Needle", "--semantic", "--json"]).current_dir(tmp.path()).output().unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("prd-alpha"));
}
