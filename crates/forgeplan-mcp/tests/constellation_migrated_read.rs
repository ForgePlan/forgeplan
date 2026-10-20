//! C12/C03 — the documented read handlers on a migrated workspace.
//!
//! Two properties are pinned here, and both are about what a read *does not*
//! do:
//!
//! 1. **No legacy tables.** `forgeplan_get` / `list` / `search` on a
//!    workspace that has been cut over to Constellation must not create
//!    `.forgeplan/lance/artifacts.lance`. Opening a `LanceStore` runs schema
//!    migrations and those migrations create tables — so "the read opened a
//!    store" and "the read created a table" are the same event. This test
//!    watches for it on disk instead of trusting the return value: a handler
//!    that answered correctly *and* migrated a legacy table would still fail.
//! 2. **Explicit workspace routing survives the branch.** A query with
//!    `workspace: Y` while the server's own default is X must read only Y. The
//!    source branch is keyed on the *resolved* workspace, so a resolver that
//!    inspected the default's index state would answer from X while claiming Y
//!    — and X's corpus is deliberately different, so that mistake is visible.
//!
//! Every test drives the real handlers over the real JSON-RPC transport. The
//! migrated fixture is built with `maintain_index`, so `CURRENT.json` and
//! `PROTOCOL.json` genuinely exist and the workspace is on the Constellation
//! side of the branch rather than merely declared so.

use std::path::{Path, PathBuf};
use std::time::Duration;

use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade::MaintainCapability;
use serde_json::json;
use tempfile::TempDir;

mod common;
use common::McpFixture;

/// The legacy tables a read must never bring into existence.
///
/// Named exactly as `LanceStore::init` names them, so the check is "did the
/// read create a table the protocol does not own" rather than "did some file
/// appear under `lance/`".
fn legacy_tables(ws: &Path) -> Vec<String> {
    let lance = ws.join("lance");
    ["artifacts.lance", "relations.lance", "change_log.lance"]
        .into_iter()
        .filter(|name| lance.join(name).exists())
        .map(str::to_string)
        .collect()
}

fn write_artifact(ws: &Path, dir: &str, name: &str, contents: &str) {
    let path = ws.join(dir).join(name);
    std::fs::create_dir_all(path.parent().expect("artifact dir")).expect("create dir");
    std::fs::write(path, contents).expect("write artifact");
}

/// A workspace published through `maintain_index` — genuinely migrated.
///
/// `title_token` is a per-workspace string that appears in one artifact's body
/// and nowhere else, so a cross-workspace read is detectable from the response
/// rather than from a path assertion.
async fn publish_workspace(root: &Path, title_token: &str) -> PathBuf {
    let ws =
        forgeplan_core::workspace::init_workspace(root, "migrated-read").expect("init workspace");
    write_artifact(
        &ws,
        "prds",
        "PRD-001-auth.md",
        &format!(
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth rewrite\ndepth: standard\n---\n\n## Summary\n\n{title_token}\n"
        ),
    );
    write_artifact(
        &ws,
        "evidence",
        "EVID-001-stress.md",
        "---\nid: EVID-001\nkind: evidence\nstatus: active\ntitle: Stress run\ndepth: standard\nlinks:\n  - target: PRD-001\n    relation: informs\n---\n\nverdict: supports\ncongruence_level: 3\nevidence_type: measurement\n",
    );

    let capability = MaintainCapability::for_strict_maintenance(
        ws.canonicalize()
            .expect("workspace directory canonicalizes"),
    );
    forgeplan_core::constellation::facade::maintain_index(
        &capability,
        CaptureLimits::default(),
        std::time::Instant::now() + Duration::from_secs(120),
    )
    .await
    .expect("bootstrap the Constellation index");

    assert!(
        forgeplan_core::constellation::facade::workspace_is_migrated(&ws),
        "fixture precondition: a published workspace must report as migrated, \
         otherwise this test would silently exercise the legacy branch"
    );
    ws
}

/// An MCP server rooted at `root`, over the real transport.
async fn server_at(root: &Path) -> McpFixture {
    McpFixture::new_rooted(root.to_path_buf()).await
}

#[tokio::test]
async fn migrated_get_returns_the_capture_and_creates_no_legacy_tables() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = publish_workspace(tmp.path(), "auth-rewrite-marker").await;

    assert!(
        legacy_tables(&ws).is_empty(),
        "precondition: a freshly published workspace holds no legacy tables, found {:?}",
        legacy_tables(&ws)
    );

    let server = server_at(tmp.path()).await;
    let got = server
        .call_tool_json("forgeplan_get", json!({ "id": "PRD-001" }))
        .await;
    let payload = got.assert_ok();

    assert_eq!(
        payload["id"], "PRD-001",
        "the read answers from the capture, got {}",
        got.raw_text
    );
    assert!(
        legacy_tables(&ws).is_empty(),
        "a migrated read must not create legacy tables, found {:?}",
        legacy_tables(&ws)
    );
}

#[tokio::test]
async fn migrated_list_and_search_answer_and_create_no_legacy_tables() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = publish_workspace(tmp.path(), "auth-rewrite-marker").await;

    let server = server_at(tmp.path()).await;

    let listed = server.call_tool_json("forgeplan_list", json!({})).await;
    assert_eq!(
        listed.assert_ok()["total"],
        2,
        "both captured artifacts are listed"
    );

    let searched = server
        .call_tool_json("forgeplan_search", json!({ "query": "auth rewrite" }))
        .await;
    let hits = searched.assert_ok()["total"]
        .as_u64()
        .expect("numeric total");
    assert!(
        hits >= 1,
        "the migrated read answers the query from the capture, got {}",
        searched.raw_text
    );
    assert_eq!(searched.assert_ok()["coverage"]["semantic"], false);

    assert!(
        legacy_tables(&ws).is_empty(),
        "a migrated read must not create legacy tables, found {:?}",
        legacy_tables(&ws)
    );
}

#[tokio::test]
async fn migrated_graph_and_validate_answer_and_create_no_legacy_tables() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = publish_workspace(tmp.path(), "auth-rewrite-marker").await;

    let server = server_at(tmp.path()).await;

    let graphed = server.call_tool_json("forgeplan_graph", json!({})).await;
    let mermaid = graphed.assert_ok()["mermaid"]
        .as_str()
        .expect("mermaid render")
        .to_string();
    assert!(
        mermaid.contains("PRD-001") && mermaid.contains("EVID-001"),
        "the migrated graph must render the declared evidence link, got {mermaid:?}"
    );

    let validated = server
        .call_tool_json("forgeplan_validate", json!({ "id": "PRD-001" }))
        .await;
    assert_eq!(validated.assert_ok()["total_artifacts"], 1);

    assert!(
        legacy_tables(&ws).is_empty(),
        "a migrated read must not create legacy tables, found {:?}",
        legacy_tables(&ws)
    );
}

#[tokio::test]
async fn an_explicit_workspace_param_reads_that_workspace_not_the_default() {
    let default_tmp = TempDir::new().expect("tempdir for default");
    let target_tmp = TempDir::new().expect("tempdir for target");

    // X is the server's own default; Y is what the query names. Different
    // corpora, so answering from the default is visible in the body.
    let _x = publish_workspace(default_tmp.path(), "default-workspace-only-marker").await;
    let y = publish_workspace(target_tmp.path(), "target-workspace-only-marker").await;

    let server = server_at(default_tmp.path()).await;
    let got = server
        .call_tool_json(
            "forgeplan_get",
            json!({ "id": "PRD-001", "workspace": y.to_string_lossy() }),
        )
        .await;
    let payload = got.assert_ok();

    assert_eq!(payload["id"], "PRD-001");
    let body = payload["body"].as_str().unwrap_or_default();
    assert!(
        body.contains("target-workspace-only-marker"),
        "the read must come from Y, got body {body:?}"
    );
    assert!(
        !body.contains("default-workspace-only-marker"),
        "the read must not come from the server's default workspace X, got {body:?}"
    );

    assert!(
        legacy_tables(&y).is_empty(),
        "reading Y through an explicit param must not create legacy tables in Y"
    );
    assert!(
        legacy_tables(&default_tmp.path().join(".forgeplan")).is_empty(),
        "reading Y must not touch the default workspace X's tables either"
    );
}

#[tokio::test]
async fn an_explicit_workspace_param_searches_that_workspace_not_the_default() {
    let default_tmp = TempDir::new().expect("tempdir for default");
    let target_tmp = TempDir::new().expect("tempdir for target");

    // Distinct tokens: a shared hyphen suffix would legitimately match BM25.
    let _x = publish_workspace(default_tmp.path(), "originworkspacesentinel").await;
    let y = publish_workspace(target_tmp.path(), "targetworkspacesentinel").await;

    let server = server_at(default_tmp.path()).await;
    let scoped = server
        .call_tool_json(
            "forgeplan_search",
            json!({ "query": "targetworkspacesentinel", "workspace": y.to_string_lossy() }),
        )
        .await;
    assert!(
        scoped.assert_ok()["total"].as_u64().expect("numeric total") >= 1,
        "a Y-scoped search must see Y's artifact, got {}",
        scoped.raw_text
    );

    // The same query against X must not see Y's token — the branch reads the
    // resolved workspace's own index state, not the server's.
    let unscoped = server
        .call_tool_json(
            "forgeplan_search",
            json!({ "query": "targetworkspacesentinel" }),
        )
        .await;
    assert_eq!(
        unscoped.assert_ok()["total"],
        0,
        "a default-scoped search must not see the other workspace's artifact"
    );
}

#[tokio::test]
async fn a_non_migrated_workspace_still_answers_from_the_legacy_store() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = forgeplan_core::workspace::init_workspace(tmp.path(), "legacy-read")
        .expect("init workspace");

    // A legacy workspace is one the store owns. Seed it the way the existing
    // fixture does so the branch under test is the source, not the data.
    let store = forgeplan_core::db::store::LanceStore::init(&ws)
        .await
        .expect("init legacy store");
    store
        .create_artifact_for_test(&forgeplan_core::db::store::NewArtifact {
            id: "PRD-001".into(),
            kind: "prd".into(),
            status: "draft".into(),
            title: "Legacy artifact".into(),
            body: "## Summary\n\nLegacy body.\n".into(),
            depth: "standard".into(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        })
        .await
        .expect("seed legacy row");
    drop(store);

    assert!(
        !forgeplan_core::constellation::facade::workspace_is_migrated(&ws),
        "fixture precondition: an unindexed workspace is not migrated"
    );

    let server = server_at(tmp.path()).await;
    let got = server
        .call_tool_json("forgeplan_get", json!({ "id": "PRD-001" }))
        .await;
    assert_eq!(
        got.assert_ok()["id"],
        "PRD-001",
        "the legacy fallback still answers reads, got {}",
        got.raw_text
    );
}

// ---------------------------------------------------------------------------
// forgeplan_score — the full AssuranceReport, computed from the capture
// ---------------------------------------------------------------------------

/// A published workspace whose graph is deep enough to score: two packs
/// inform PRD-001, PRD-001 is based on an ACTIVE ADR-001, and that ADR is
/// itself informed by a weaker pack.
///
/// The dependency is what separates a real recursive report from a scalar
/// recomputed from the artifact's own packs: ADR-001 is the weakest link,
/// and it is weaker than PRD-001's own evidence.
async fn publish_scored_workspace(root: &Path, superseded_pack: bool) -> PathBuf {
    let ws =
        forgeplan_core::workspace::init_workspace(root, "migrated-score").expect("init workspace");
    write_artifact(
        &ws,
        "prds",
        "PRD-001-auth.md",
        "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth rewrite\ndepth: standard\nlinks:\n  - target: ADR-001\n    relation: based_on\n---\n\n## Summary\n\nRework the login flow.\n",
    );
    write_artifact(
        &ws,
        "adrs",
        "ADR-001-session.md",
        "---\nid: ADR-001\nkind: adr\nstatus: active\ntitle: Session strategy\ndepth: standard\n---\n\n## Decision\n\nRotate opaque sessions.\n",
    );
    write_artifact(
        &ws,
        "evidence",
        "EVID-001-loadtest.md",
        "---\nid: EVID-001\nkind: evidence\nstatus: active\ntitle: Load test run\ndepth: standard\nlinks:\n  - target: PRD-001\n    relation: informs\n---\n\n## Structured Fields\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n",
    );
    // EVID-002 sits exactly on the pack-status boundary: the same pack either
    // counts (active) or is displaced from the min (superseded).
    write_artifact(
        &ws,
        "evidence",
        "EVID-002-benchmark.md",
        &format!(
            "---\nid: EVID-002\nkind: evidence\nstatus: {}\ntitle: Benchmark run\ndepth: standard\nlinks:\n  - target: PRD-001\n    relation: informs\n---\n\n## Structured Fields\n\nverdict: weakens\ncongruence_level: 3\nevidence_type: benchmark\n",
            if superseded_pack {
                "superseded"
            } else {
                "active"
            }
        ),
    );
    write_artifact(
        &ws,
        "evidence",
        "EVID-003-audit.md",
        "---\nid: EVID-003\nkind: evidence\nstatus: active\ntitle: External audit\ndepth: standard\nlinks:\n  - target: ADR-001\n    relation: informs\n---\n\n## Structured Fields\n\nverdict: weakens\ncongruence_level: 3\nevidence_type: audit\n",
    );

    let capability = MaintainCapability::for_strict_maintenance(
        ws.canonicalize()
            .expect("workspace directory canonicalizes"),
    );
    forgeplan_core::constellation::facade::maintain_index(
        &capability,
        CaptureLimits::default(),
        std::time::Instant::now() + Duration::from_secs(120),
    )
    .await
    .expect("bootstrap the Constellation index");

    assert!(
        forgeplan_core::constellation::facade::workspace_is_migrated(&ws),
        "fixture precondition: a published workspace must report as migrated, \
         otherwise this test would silently exercise the legacy branch"
    );
    assert!(
        legacy_tables(&ws).is_empty(),
        "fixture precondition: a published workspace holds no legacy tables, found {:?}",
        legacy_tables(&ws)
    );
    ws
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < 1e-9
}

/// `forgeplan_score` returns the whole report on a migrated workspace:
/// weakest link, factor list, per-pack breakdown, F-G-R — and creates no
/// legacy table.
///
/// The tool used to refuse here, because `weakest_link` and the factor list
/// were only reachable through the legacy relation graph. The migrated
/// workspace has the same graph in its canonical Markdown, so the same
/// engine runs over the same capture.
#[tokio::test]
async fn migrated_score_returns_the_full_assurance_report() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = publish_scored_workspace(tmp.path(), false).await;

    let server = server_at(tmp.path()).await;
    let scored = server
        .call_tool_json("forgeplan_score", json!({ "id": "PRD-001" }))
        .await;
    let payload = scored.assert_ok();

    // PRD-001's own packs: EVID-001 supports/test/CL3 → 1.0 and
    // EVID-002 weakens/benchmark/CL3 → 0.4. So self_score is 0.4.
    let self_score = payload["self_score"].as_f64().expect("numeric self_score");
    assert!(
        close(self_score, 0.4),
        "the weakest own pack must set the self score, got {self_score} in {payload}"
    );

    // ADR-001's own pack weakens/audit/CL3 → 0.3, less the CL2 `based_on`
    // penalty → ~0.2. That is BELOW PRD-001's own evidence, so a report that
    // never walked the graph would answer 0.4 here.
    let r_eff = payload["r_eff"].as_f64().expect("numeric r_eff");
    assert!(
        close(r_eff, 0.2),
        "the recursive dependency must constrain the final score, got {r_eff} in {payload}"
    );
    assert_eq!(
        payload["weakest_link"].as_str(),
        Some("ADR-001"),
        "the weakest link must be attributed, got {}",
        scored.raw_text
    );
    assert!(
        payload["factors"]
            .as_array()
            .expect("factors is a list")
            .iter()
            .filter_map(|f| f.as_str())
            .any(|f| f.contains("CL penalty applied for ADR-001 (relation: based_on)")),
        "the relation penalty must be reported, got {}",
        payload["factors"]
    );

    // The flat breakdown an agent reads: only PRD-001's own packs, in id
    // order, each with its own raw score and eligibility flag.
    let evidence = payload["evidence"].as_array().expect("evidence is a list");
    assert_eq!(
        evidence.len(),
        2,
        "ADR-001's own pack belongs to ADR-001's breakdown, got {evidence:?}"
    );
    assert_eq!(evidence[0]["id"], "EVID-001");
    assert_eq!(evidence[1]["id"], "EVID-002");
    assert_eq!(evidence[1]["verdict"], "Weakens");
    assert_eq!(evidence[0]["excluded"], json!(false));

    // The link count that F-G-R weighs is the count the CAPTURE saw, not a
    // store query: three edges touch PRD-001 (two packs in, one dependency
    // out). Holding r_eff and the workspace weights fixed, only the link
    // count can move reliability — so this pins `relation_link_count` on the
    // migrated branch.
    let weights = forgeplan_core::workspace::load_config(&ws)
        .ok()
        .and_then(|c| c.fpf.map(|f| f.weights));
    let expected_reliability =
        forgeplan_core::scoring::fgr::compute_reliability(r_eff, 3, false, weights.as_ref());
    let reliability = payload["reliability"]
        .as_f64()
        .expect("numeric reliability");
    assert!(
        close(reliability, expected_reliability),
        "F-G-R reliability must come from the captured link count (3), got {reliability}, \
         expected {expected_reliability}"
    );

    assert!(
        payload["overall_grade"].is_string(),
        "F-G-R must come from the same captured record, got {}",
        scored.raw_text
    );

    assert!(
        legacy_tables(&ws).is_empty(),
        "a migrated score must not create legacy tables, found {:?}",
        legacy_tables(&ws)
    );
}

/// ADR-020 boundary on a migrated workspace: a superseded pack leaves the
/// weakest-link min, stays listed with `excluded: true`, and is named in the
/// factors — the same displacement the legacy path reports.
#[tokio::test]
async fn a_superseded_pack_leaves_the_min_on_a_migrated_workspace() {
    let tmp = TempDir::new().expect("tempdir");
    let ws = publish_scored_workspace(tmp.path(), true).await;

    let server = server_at(tmp.path()).await;

    // ADR-001 first: its own pack is the only one linked to it, so this
    // answers from a single-pack population and proves EVID-002 (linked to
    // PRD-001) did not leak in.
    let scored = server
        .call_tool_json("forgeplan_score", json!({ "id": "ADR-001" }))
        .await;
    let payload = scored.assert_ok();
    let evidence = payload["evidence"].as_array().expect("evidence is a list");
    assert_eq!(
        evidence.len(),
        1,
        "only the packs linked to ADR-001 belong here, got {}",
        scored.raw_text
    );
    assert_eq!(evidence[0]["id"], "EVID-003");
    let r_eff = payload["r_eff"].as_f64().expect("numeric r_eff");
    assert!(
        close(r_eff, 0.3),
        "weakens/audit/CL3 scores 0.5 - 0.2, got {r_eff} in {payload}"
    );

    // Two edges touch ADR-001 (EVID-003 informs in, PRD-001 based_on out) —
    // the count the capture saw. The earlier draft of this test asserted 1
    // (only EVID-003) and failed because `relation_link_count` counts every
    // edge whose source OR target matches, including PRD-001's based_on.
    let weights = forgeplan_core::workspace::load_config(&ws)
        .ok()
        .and_then(|c| c.fpf.map(|f| f.weights));
    let expected_reliability =
        forgeplan_core::scoring::fgr::compute_reliability(r_eff, 2, false, weights.as_ref());
    let reliability = payload["reliability"]
        .as_f64()
        .expect("numeric reliability");
    assert!(
        close(reliability, expected_reliability),
        "F-G-R reliability must come from the captured link count (2), got {reliability}, \
         expected {expected_reliability}"
    );

    // PRD-001, whose weaker pack was displaced: its own score rises to the
    // remaining active pack, and the displaced pack stays listed, marked, and
    // named in the factors.
    let prd = server
        .call_tool_json("forgeplan_score", json!({ "id": "PRD-001" }))
        .await;
    let prd_payload = prd.assert_ok();
    let prd_evidence = prd_payload["evidence"]
        .as_array()
        .expect("evidence is a list");
    assert_eq!(prd_evidence.len(), 2, "the displaced pack stays listed");
    let superseded = prd_evidence
        .iter()
        .find(|e| e["id"] == "EVID-002")
        .expect("EVID-002 is listed");
    assert_eq!(superseded["excluded"], json!(true));
    assert_eq!(superseded["status"], "superseded");
    assert!(
        prd_payload["factors"]
            .as_array()
            .expect("factors is a list")
            .iter()
            .filter_map(|f| f.as_str())
            .any(|f| f.contains("Skipped EVID-002 (status: superseded)")),
        "the displacement must be auditable, not silent, got {}",
        prd_payload["factors"]
    );
    let self_score = prd_payload["self_score"]
        .as_f64()
        .expect("numeric self_score");
    assert!(
        close(self_score, 1.0),
        "with the weakens pack displaced, only the supporting pack counts, got {self_score}"
    );

    assert!(
        legacy_tables(&ws).is_empty(),
        "a migrated score must not create legacy tables, found {:?}",
        legacy_tables(&ws)
    );
}
