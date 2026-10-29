//! Paired CLI graph reads before and after Constellation migration.
//! Legacy records retain complete canonical bytes for metadata-aware comparisons.

use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use forgeplan_core::artifact::types::{ArtifactKind, slugify};
use forgeplan_core::db::store::LanceStore;
use forgeplan_core::{projection, workspace};
use serde_json::{Value, json};
use tempfile::TempDir;

fn run(root: &Path, args: &[&str]) -> Output {
    Command::cargo_bin("forgeplan")
        .unwrap()
        .args(args)
        .current_dir(root)
        .env("HOME", root.join("home"))
        .env("HF_HUB_OFFLINE", "1")
        .env("NO_COLOR", "1")
        .env("CLICOLOR_FORCE", "0")
        .output()
        .unwrap()
}

fn success(root: &Path, args: &[&str]) -> Output {
    let out = run(root, args);
    assert!(
        out.status.success(),
        "forgeplan {args:?} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    out
}

fn tables(root: &Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(root.join(".forgeplan/lance"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Every reporting invocation must leave the published pointer and table set alone.
fn read_output(root: &Path, args: &[&str]) -> Output {
    let pointer = root.join(".forgeplan/lance/CURRENT.json");
    let before_pointer = std::fs::read(&pointer).ok();
    let before_tables = tables(root);
    let out = run(root, args);
    assert_eq!(
        std::fs::read(&pointer).ok(),
        before_pointer,
        "{args:?} changed CURRENT.json"
    );
    if before_pointer.is_some() {
        assert_eq!(
            tables(root),
            before_tables,
            "{args:?} changed migrated tables"
        );
    }
    out
}

fn read_ok(root: &Path, args: &[&str]) -> Output {
    let out = read_output(root, args);
    assert!(
        out.status.success(),
        "forgeplan {args:?} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    out
}

fn text(root: &Path, args: &[&str]) -> String {
    String::from_utf8(read_ok(root, args).stdout).unwrap()
}

fn document(root: &Path, args: &[&str]) -> Value {
    let out = read_ok(root, args);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "forgeplan {args:?}: {err}: {}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

fn artifact(
    root: &Path,
    id: &str,
    kind: &str,
    status: &str,
    title: &str,
    extra: &str,
    body: &str,
) -> PathBuf {
    let kind_dir = kind.parse::<ArtifactKind>().unwrap().dir_name();
    let path = root
        .join(".forgeplan")
        .join(kind_dir)
        .join(format!("{id}-{}.md", slugify(title)));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        format!(
            "---\nid: {id}\nkind: {kind}\nstatus: {status}\ntitle: {title}\ndepth: tactical\n\
             created_at: 2026-01-01T00:00:00Z\nupdated_at: 2026-01-01T00:00:00Z\n\
             {extra}---\n\n{body}\n"
        ),
    )
    .unwrap();
    path
}

/// Preserve the full file in legacy rows, rather than testing reindex's historical
/// frontmatter loss against a metadata-complete capture.
fn hydrate_legacy_source_bodies(root: &Path) {
    let ws = root.join(".forgeplan");
    assert!(
        !ws.join("lance/CURRENT.json").exists(),
        "legacy fixture hydration must happen before Constellation migration"
    );
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let guard = workspace::acquire_workspace_lock(&ws).await.unwrap();
        let store = LanceStore::open(&ws).await.unwrap();
        let records = store.list_records(None).await.unwrap();
        assert!(!records.is_empty(), "legacy fixture must contain real rows");
        for record in records {
            let kind = record.kind.parse::<ArtifactKind>().unwrap();
            let path = ws.join(kind.dir_name()).join(format!(
                "{}-{}.md",
                record.id,
                slugify(&record.title)
            ));
            let content = tokio::fs::read_to_string(&path).await.unwrap();
            projection::sync_body_from_file(
                &projection::MutationContext::new(&ws, &guard, Some(&store)),
                &record.id,
                &record.kind,
                &record.title,
                &content,
            )
            .await
            .unwrap();
            assert_eq!(
                store.get_record(&record.id).await.unwrap().unwrap().body,
                content,
                "legacy fixture must preserve source metadata for {}",
                record.id
            );
        }
    });
}

fn paired(seed: fn(&Path)) -> [TempDir; 2] {
    let roots = [TempDir::new().unwrap(), TempDir::new().unwrap()];
    for root in &roots {
        std::fs::create_dir(root.path().join("home")).unwrap();
        success(root.path(), &["init", "-y"]);
        seed(root.path());
        let out = success(root.path(), &["reindex"]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stdout.contains(", 0 errors,"),
            "legacy seed was incomplete: {stdout} {stderr}"
        );
        assert!(!stdout.contains("create failed") && !stderr.contains("create failed"));
        assert!(
            root.path()
                .join(".forgeplan/lance/artifacts.lance")
                .exists()
        );
        hydrate_legacy_source_bodies(root.path());
    }
    success(roots[1].path(), &["index"]);
    assert!(
        roots[1]
            .path()
            .join(".forgeplan/lance/CURRENT.json")
            .exists()
    );
    assert!(
        !roots[1]
            .path()
            .join(".forgeplan/lance/artifacts.lance")
            .exists()
    );
    assert!(
        !roots[1]
            .path()
            .join(".forgeplan/lance/relations.lance")
            .exists()
    );
    roots
}

fn seed_graph(root: &Path) {
    artifact(
        root,
        "EPIC-001",
        "epic",
        "active",
        "Release",
        "",
        "## Summary\nRelease scope.",
    );
    for (id, status, title) in [
        ("PRD-001", "draft", "Pending"),
        ("PRD-002", "active", "Accepted"),
        ("PRD-003", "deprecated", "Retired"),
        ("PRD-004", "superseded", "Replaced"),
        ("PRD-005", "stale", "Stale"),
    ] {
        let extra = if id == "PRD-001" {
            "parent_epic: EPIC-001\n"
        } else {
            ""
        };
        artifact(
            root,
            id,
            "prd",
            status,
            title,
            extra,
            "## Summary\nA dependency.",
        );
    }
    for (id, target, relation, title) in [
        ("RFC-001", "PRD-001", "based_on", "Blocked work"),
        ("RFC-002", "PRD-002", "based_on", "Active dependency"),
        ("RFC-003", "PRD-003", "based_on", "Retired dependency"),
        ("RFC-004", "PRD-004", "based_on", "Replaced dependency"),
        ("RFC-005", "PRD-005", "refines", "Stale dependency"),
        ("RFC-006", "PRD-001", "informs", "Informational dependency"),
        ("RFC-010", "RFC-011", "based_on", "Cycle one"),
        ("RFC-011", "RFC-010", "based_on", "Cycle two"),
    ] {
        artifact(
            root,
            id,
            "rfc",
            "draft",
            title,
            &format!(
                "parent_epic: EPIC-001\nlinks:\n  - target: {target}\n    relation: {relation}\n"
            ),
            "## Summary\nA linked task.",
        );
    }
    artifact(
        root,
        "RFC-007",
        "rfc",
        "draft",
        "Epic member",
        "parent_epic: EPIC-001\n",
        "A member without explicit links.",
    );
    artifact(
        root,
        "UC-001",
        "use_case",
        "active",
        "Sign in",
        "",
        "## Goal\nSign in.",
    );
    artifact(
        root,
        "HYP-001",
        "hypothesis",
        "active",
        "Verified claim",
        "hypothesis_state: verified\nlinks:\n  - target: UC-001\n    relation: triangulates\n",
        "## Hypothesis\nThe sign-in flow works.",
    );
    artifact(
        root,
        "DM-001",
        "domain_model",
        "active",
        "Identity model",
        "",
        "## Composition\n- UC-001\n- HYP-001",
    );
}

fn has_edge(doc: &Value, from: &str, to: &str, relation: &str) -> bool {
    doc["edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|edge| edge["from"] == from && edge["to"] == to && edge["relation"] == relation)
}

#[test]
fn graph_preserves_nonempty_edges_parent_epics_and_mermaid_on_both_layouts() {
    let roots = paired(seed_graph);
    let before = document(roots[0].path(), &["graph", "--json"]);
    let after = document(roots[1].path(), &["graph", "--json"]);
    assert_eq!(before, after);
    assert_eq!(
        text(roots[0].path(), &["graph"]),
        text(roots[1].path(), &["graph"])
    );
    assert!(before["edges"].as_array().unwrap().len() >= 12);
    assert!(has_edge(&after, "RFC-001", "PRD-001", "based_on"));
    assert!(has_edge(&after, "RFC-007", "EPIC-001", "belongs_to"));
    assert!(has_edge(&after, "PRD-001", "EPIC-001", "belongs_to"));
    assert!(has_edge(&after, "HYP-001", "UC-001", "triangulates"));
    assert_eq!(after["_next_action"], "forgeplan graph | mmdc -o graph.png");
    for root in &roots {
        let mermaid = text(root.path(), &["graph"]);
        assert!(mermaid.starts_with("graph LR\n"));
        assert!(mermaid.contains("RFC-001 -->|based_on| PRD-001"));
        assert!(mermaid.contains("RFC-007 --> EPIC-001"));
        assert!(mermaid.contains("subgraph DM-001[\"DM-001\"]"));
        assert!(mermaid.contains("class HYP-001 hypothesisVerifiedStyle"));
        let brownfield = text(root.path(), &["graph", "--brownfield-only"]);
        assert!(brownfield.contains("HYP-001 -->|triangulates| UC-001"));
        assert!(brownfield.contains("subgraph DM-001"));
        assert!(!brownfield.contains("RFC-001"));
        assert!(!brownfield.contains("PRD-001"));
        let filtered = document(root.path(), &["graph", "--brownfield-only", "--json"]);
        assert_eq!(filtered["brownfield_only"], true);
        // JSON has always returned all edges; the flag selects the Mermaid renderer.
        assert_eq!(filtered["edges"], before["edges"]);
    }
}

#[test]
fn blockers_and_order_preserve_resolved_statuses_and_report_cycles() {
    let roots = paired(seed_graph);
    for args in [["blocked", "--json"], ["order", "--json"]] {
        let before = document(roots[0].path(), &args);
        let after = document(roots[1].path(), &args);
        assert_eq!(before, after);
        assert_eq!(after["blocked"].as_array().unwrap().len(), 2);
        assert!(
            after["blocked"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["id"] == "RFC-001" && row["blocked_by"] == json!(["PRD-001"]))
        );
        assert!(
            after["blocked"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["id"] == "RFC-005" && row["blocked_by"] == json!(["PRD-005"]))
        );
        for ready in ["RFC-002", "RFC-003", "RFC-004"] {
            assert!(after["ready"].as_array().unwrap().contains(&json!(ready)));
        }
        assert!(
            after["cycles"]
                .as_array()
                .unwrap()
                .iter()
                .any(|cycle| cycle == &json!(["RFC-010", "RFC-011", "RFC-010"]))
        );
        assert!(
            after["_next_action"]
                .as_str()
                .unwrap()
                .starts_with("forgeplan ")
        );
    }
    for root in &roots {
        for (id, expected) in [
            ("RFC-001", json!(["PRD-001"])),
            ("RFC-002", json!([])),
            ("RFC-003", json!([])),
            ("RFC-004", json!([])),
            ("RFC-005", json!(["PRD-005"])),
            ("RFC-006", json!([])),
            ("RFC-010", json!(["RFC-011"])),
        ] {
            let result = document(root.path(), &["blocked", id, "--json"]);
            assert_eq!(result["id"], id);
            assert_eq!(result["blocked_by"], expected);
            assert_eq!(result["blocked"], !expected.as_array().unwrap().is_empty());
        }
        let blocked = text(root.path(), &["blocked", "RFC-001"]);
        assert!(blocked.contains("RFC-001 is BLOCKED by:"));
        assert!(blocked.contains("PRD-001 (draft)"));
        let resolved = text(root.path(), &["blocked", "RFC-003"]);
        assert!(resolved.contains("NOT blocked") && resolved.contains("Done."));
        let all_blocked = text(root.path(), &["blocked"]);
        assert!(all_blocked.contains("Cycles detected:"));
        assert!(all_blocked.contains("RFC-005 <- blocked by: PRD-005"));
        let order = text(root.path(), &["order"]);
        assert!(order.contains("Cycles detected:"));
        assert!(order.contains("PRD-003 (deprecated, ready)"));
        assert!(order.contains("PRD-004 (superseded, ready)"));
        assert!(order.contains("RFC-001 (draft, blocked by PRD-001)"));
        assert!(order.contains("Next: forgeplan get RFC-010"));
    }
}

fn find_node<'a>(nodes: &'a [Value], id: &str) -> Option<&'a Value> {
    for node in nodes {
        if node["id"] == id {
            return Some(node);
        }
        if let Some(found) = find_node(node["children"].as_array().unwrap(), id) {
            return Some(found);
        }
    }
    None
}

#[test]
fn tree_keeps_bare_array_shape_hierarchy_depth_and_text_errors() {
    let roots = paired(seed_graph);
    let before = document(roots[0].path(), &["tree", "--json"]);
    let after = document(roots[1].path(), &["tree", "--json"]);
    assert_eq!(before, after);
    let nodes = after
        .as_array()
        .expect("tree stdout must remain a bare JSON array");
    assert!(!nodes.is_empty());
    let epic = find_node(nodes, "EPIC-001").unwrap();
    assert_eq!(epic["title"], "Release");
    assert_eq!(epic["kind"], "epic");
    let pending = find_node(epic["children"].as_array().unwrap(), "PRD-001").unwrap();
    assert!(
        pending["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["id"] == "RFC-001")
    );
    assert!(
        epic["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["id"] == "RFC-007")
    );
    assert!(
        !epic["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["id"] == "RFC-001")
    );
    for root in &roots {
        let out = read_ok(root.path(), &["tree", "--json"]);
        assert!(String::from_utf8_lossy(&out.stderr).contains("Next: forgeplan health"));
        let leaf = document(root.path(), &["tree", "EPIC-001", "--depth", "0", "--json"]);
        assert_eq!(leaf["id"], "EPIC-001");
        assert_eq!(leaf["children"], json!([]));
        let shallow = document(root.path(), &["tree", "EPIC-001", "--depth", "1", "--json"]);
        let prd = find_node(shallow["children"].as_array().unwrap(), "PRD-001").unwrap();
        assert_eq!(prd["children"], json!([]));
        let cycle = document(root.path(), &["tree", "RFC-010", "--depth", "2", "--json"]);
        assert_eq!(cycle["children"][0]["id"], "RFC-011");
        assert_eq!(cycle["children"][0]["children"][0]["id"], "RFC-010");
        assert_eq!(cycle["children"][0]["children"][0]["children"], json!([]));
        let tree = text(root.path(), &["tree", "EPIC-001", "--depth", "2"]);
        assert!(tree.contains("EPIC-001") && tree.contains("PRD-001") && tree.contains("RFC-001"));
        let overview = text(root.path(), &["tree"]);
        assert!(overview.contains("Forgeplan Tree") && overview.contains("artifacts |"));
        let missing = read_output(root.path(), &["tree", "RFC-999"]);
        assert!(!missing.status.success());
        let stderr = String::from_utf8_lossy(&missing.stderr);
        assert!(stderr.contains("not found") && stderr.contains("forgeplan list"));
    }
}

fn seed_dispatch(root: &Path) {
    artifact(
        root,
        "EPIC-001",
        "epic",
        "active",
        "Release",
        "",
        "Release scope.",
    );
    artifact(
        root,
        "EPIC-002",
        "epic",
        "active",
        "Other release",
        "",
        "Other scope.",
    );
    artifact(
        root,
        "PRD-001",
        "prd",
        "draft",
        "Pending",
        "",
        "Pending dependency.",
    );
    artifact(
        root,
        "PRD-002",
        "prd",
        "active",
        "Accepted",
        "",
        "Resolved dependency.",
    );
    for (id, title, status, epic, files, links) in [
        (
            "RFC-001",
            "Alpha",
            "draft",
            "EPIC-001",
            "affected_files: [src/shared.rs]\n",
            "",
        ),
        ("RFC-002", "Beta", "draft", "EPIC-001", "", ""),
        (
            "RFC-003",
            "Blocked",
            "draft",
            "EPIC-001",
            "affected_files: [src/blocked.rs]\n",
            "links:\n  - target: PRD-001\n    relation: based_on\n",
        ),
        (
            "RFC-004",
            "Claimed",
            "draft",
            "EPIC-001",
            "affected_files: [src/claimed.rs]\n",
            "",
        ),
        (
            "RFC-005",
            "Already active",
            "active",
            "EPIC-001",
            "affected_files: [src/active.rs]\n",
            "",
        ),
        (
            "RFC-006",
            "Other epic",
            "draft",
            "EPIC-002",
            "affected_files: [src/other.rs]\n",
            "",
        ),
        ("RFC-007", "No files", "draft", "EPIC-001", "", ""),
        (
            "RFC-008",
            "Ready",
            "draft",
            "EPIC-001",
            "affected_files: src/ready.rs\n",
            "links:\n  - target: PRD-002\n    relation: based_on\n",
        ),
        (
            "RFC-009",
            "Informational",
            "draft",
            "EPIC-001",
            "affected_files: [src/info.rs]\n",
            "links:\n  - target: PRD-001\n    relation: informs\n",
        ),
    ] {
        let body = if id == "RFC-002" {
            "## Affected Files\n- `src/shared.rs`"
        } else {
            "## Summary\nDispatch work."
        };
        artifact(
            root,
            id,
            "rfc",
            status,
            title,
            &format!("parent_epic: {epic}\ndomain: backend\n{files}{links}"),
            body,
        );
    }
}

fn dispatch_json(root: &Path, extra: &[&str]) -> Value {
    let mut args = vec![
        "dispatch", "--agents", "1", "--kind", "rfc", "--epic", "EPIC-001", "--json",
    ];
    args.extend_from_slice(extra);
    let mut result = document(root, &args);
    assert!(
        result["generated_at"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
    result.as_object_mut().unwrap().remove("generated_at");
    result
}

#[test]
fn dispatch_preserves_filters_claims_blockers_and_file_hydration() {
    let roots = paired(seed_dispatch);
    for root in &roots {
        success(
            root.path(),
            &["claim", "RFC-004", "--agent", "fixture-owner"],
        );
    }
    let before = dispatch_json(roots[0].path(), &[]);
    let after = dispatch_json(roots[1].path(), &[]);
    assert_eq!(before, after);
    assert_eq!(after["candidate_count"], 6);
    assert_eq!(after["claimed_count"], 1);
    assert_eq!(after["blocked_count"], 1);
    assert_eq!(after["skipped_parse_errors"], 0);
    assert_eq!(after["buckets"], json!([["RFC-001", "RFC-008", "RFC-009"]]));
    let serial = after["serial_queue"].as_array().unwrap();
    assert!(serial.iter().any(|entry| {
        entry["id"] == "RFC-002" && entry["reason"].as_str().unwrap().contains("overlap")
    }));
    assert!(serial.iter().any(|entry| {
        entry["id"] == "RFC-007" && entry["reason"].as_str().unwrap().contains("affected_files")
    }));
    assert!(serial.iter().any(|entry| {
        entry["id"] == "RFC-003" && entry["reason"] == "blocked by dependencies: PRD-001"
    }));
    assert_eq!(
        after["_next_action"],
        "forgeplan claim RFC-001 --agent agent-0"
    );
    for root in &roots {
        let active = dispatch_json(root.path(), &["--status", "active"]);
        assert_eq!(active["candidate_count"], 1);
        assert_eq!(active["buckets"], json!([["RFC-005"]]));
        let any = dispatch_json(root.path(), &["--status", "any"]);
        assert_eq!(any["candidate_count"], 7);
        assert!(
            any["buckets"][0]
                .as_array()
                .unwrap()
                .contains(&json!("RFC-005"))
        );
        let other = document(
            root.path(),
            &[
                "dispatch", "-n", "2", "-t", "rfc", "--epic", "EPIC-002", "--json",
            ],
        );
        assert_eq!(other["candidate_count"], 1);
        assert_eq!(other["buckets"], json!([["RFC-006"], []]));
        let text = text(
            root.path(),
            &[
                "dispatch", "--agents", "1", "--kind", "rfc", "--epic", "EPIC-001",
            ],
        );
        assert!(text.contains("Dispatch plan") && text.contains("Candidates:        6"));
        assert!(text.contains("blocked by dependencies: PRD-001"));
        for invalid in [
            vec!["dispatch", "--agents", "0"],
            vec!["dispatch", "--agents", "65"],
            vec!["dispatch", "--agents", "1", "--overlap-threshold", "1.5"],
        ] {
            let out = read_output(root.path(), &invalid);
            assert!(!out.status.success());
            assert!(!out.stderr.is_empty());
        }
    }
}

#[test]
fn migrated_reads_see_offline_source_changes_without_publishing() {
    let roots = paired(seed_graph);
    let root = roots[1].path();
    let before = document(root, &["blocked", "RFC-001", "--json"]);
    assert_eq!(before["blocked"], true);
    artifact(
        root,
        "PRD-001",
        "prd",
        "active",
        "Pending",
        "parent_epic: EPIC-001\n",
        "Now resolved offline.",
    );
    artifact(
        root,
        "RFC-007",
        "rfc",
        "draft",
        "Epic member",
        "parent_epic: EPIC-001\nlinks:\n  - target: PRD-002\n    relation: based_on\n",
        "A new offline dependency.",
    );
    let blocked = document(root, &["blocked", "RFC-001", "--json"]);
    assert_eq!(blocked["blocked"], false);
    let order = document(root, &["order", "--json"]);
    assert!(
        order["ready"]
            .as_array()
            .unwrap()
            .contains(&json!("RFC-001"))
    );
    let graph = document(root, &["graph", "--json"]);
    assert!(has_edge(&graph, "RFC-007", "PRD-002", "based_on"));
    let tree = document(root, &["tree", "--json"]);
    let nodes = tree.as_array().unwrap();
    assert_eq!(find_node(nodes, "PRD-001").unwrap()["status"], "active");
    let parent = find_node(nodes, "PRD-002").unwrap();
    assert!(
        parent["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["id"] == "RFC-007")
    );
}

#[test]
fn migrated_dispatch_uses_captured_frontmatter_after_source_rename_and_edit() {
    let roots = paired(seed_dispatch);
    let root = roots[1].path();
    let before = dispatch_json(root, &[]);
    assert!(
        !before["buckets"][0]
            .as_array()
            .unwrap()
            .contains(&json!("RFC-002"))
    );
    let path = artifact(
        root,
        "RFC-001",
        "rfc",
        "draft",
        "Alpha",
        "parent_epic: EPIC-001\ndomain: backend\naffected_files: [src/offline.rs]\n",
        "Changed offline.",
    );
    std::fs::rename(&path, path.with_file_name("independently-named-source.md")).unwrap();
    let after = dispatch_json(root, &[]);
    assert_eq!(after["skipped_parse_errors"], 0);
    let bucket = after["buckets"][0].as_array().unwrap();
    assert!(bucket.contains(&json!("RFC-001")) && bucket.contains(&json!("RFC-002")));
    let moved = root.join(".forgeplan/rfcs/independently-named-source.md");
    let original = std::fs::read_to_string(&moved).unwrap();
    std::fs::write(
        &moved,
        original.replace("parent_epic: EPIC-001", "parent_epic: EPIC-002"),
    )
    .unwrap();
    let filtered = dispatch_json(root, &[]);
    assert_eq!(
        filtered["candidate_count"],
        after["candidate_count"].as_u64().unwrap() - 1
    );
    assert!(
        !filtered["buckets"][0]
            .as_array()
            .unwrap()
            .contains(&json!("RFC-001"))
    );
}

#[test]
fn corrupt_capture_is_an_error_for_every_graph_consumer() {
    let roots = paired(seed_graph);
    let root = roots[1].path();
    std::fs::write(
        root.join(".forgeplan/lance/CURRENT.json"),
        "{ broken pointer",
    )
    .unwrap();
    for args in [
        vec!["graph", "--json"],
        vec!["blocked", "--json"],
        vec!["blocked", "RFC-001", "--json"],
        vec!["order", "--json"],
        vec!["tree", "--json"],
        vec!["dispatch", "--agents", "2", "--json"],
    ] {
        let out = read_output(root, &args);
        assert!(
            !out.status.success(),
            "{args:?} reported a false empty success"
        );
        assert!(!out.stderr.is_empty(), "{args:?} failed silently");
        assert!(
            serde_json::from_slice::<Value>(&out.stdout).is_err(),
            "{args:?} emitted a fake report"
        );
    }
}
