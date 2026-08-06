//! #360 / PRD-082 slice 2 — activate-time provenance gate on the MCP surface.
//!
//! The audit (before this PR) flagged that the gate is wired into two surfaces
//! (CLI + MCP) but was E2E-tested on only one (RED LINE #5, the PROB-035/039
//! silent-failure class). This drives the real `forgeplan_activate` MCP handler
//! against a real git repo: under `block` an empty-delta claim returns an
//! agent-visible error, and `force` overrides it.

mod common;
use common::McpFixture;

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap()
            .success(),
        "git {args:?}"
    );
}

/// A real git repo with an inited forgeplan workspace whose gate is `block`.
/// Returns (tempdir, root, HEAD short sha).
async fn setup_block() -> (TempDir, std::path::PathBuf, String) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    git(&root, &["init", "--quiet", "--initial-branch=main"]);
    git(&root, &["config", "user.email", "t@local"]);
    git(&root, &["config", "user.name", "T"]);

    let ws = forgeplan_core::workspace::init_workspace(&root, "mcp-gate-e2e").unwrap();
    // `new_rooted` only OPENS the store; create the LanceDB tables first, the
    // way `McpFixture::new()` does via LanceStore::init.
    forgeplan_core::db::store::LanceStore::init(&ws)
        .await
        .expect("init lance store");

    let cfg = root.join(".forgeplan/config.yaml");
    let text = std::fs::read_to_string(&cfg).unwrap();
    let text = text.replace(
        "evidence_provenance_gate: warn",
        "evidence_provenance_gate: block",
    );
    assert!(text.contains("evidence_provenance_gate: block"), "gate set");
    std::fs::write(&cfg, text).unwrap();

    git(&root, &["add", "-A"]);
    git(&root, &["commit", "--quiet", "-m", "init"]);
    let head = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "--short=7", "HEAD"])
            .current_dir(&root)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    (tmp, root, head)
}

/// Seed an EVID whose claim is an empty delta (base == result) — the #360 case.
async fn seed_empty_delta(fx: &McpFixture, head: &str) -> String {
    let created = fx
        .call_tool_json(
            "forgeplan_new",
            serde_json::json!({"kind": "evidence", "title": "MCP empty-delta probe"}),
        )
        .await;
    let id = created.assert_ok()["id"].as_str().unwrap().to_string();

    let body = format!(
        "## Structured Fields\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n\
         base_sha: {head}\nresult_sha: {head}\nchanged_paths: src/x.rs\n\n## Summary\n\nprobe\n"
    );
    fx.call_tool_json(
        "forgeplan_update",
        serde_json::json!({"id": id, "body": body}),
    )
    .await
    .assert_ok();
    id
}

#[tokio::test]
async fn mcp_block_mode_refuses_activation_on_empty_delta() {
    let (_tmp, root, head) = setup_block().await;
    let fx = McpFixture::new_rooted(root).await;
    let id = seed_empty_delta(&fx, &head).await;

    let env = fx
        .call_tool_json("forgeplan_activate", serde_json::json!({"id": id}))
        .await;
    assert!(
        env.is_error,
        "block mode must return an error result the agent sees, got: {}",
        env.raw_text
    );
    assert!(
        env.raw_text.contains("provenance gate"),
        "the error must name the provenance gate, got: {}",
        env.raw_text
    );

    // And the artifact must remain draft — the refusal is real, not cosmetic.
    let got = fx
        .call_tool_json("forgeplan_get", serde_json::json!({"id": id}))
        .await;
    assert_eq!(
        got.assert_ok()["status"].as_str().unwrap(),
        "draft",
        "a blocked activation must leave the artifact draft"
    );
}

#[tokio::test]
async fn mcp_force_overrides_the_provenance_gate() {
    let (_tmp, root, head) = setup_block().await;
    let fx = McpFixture::new_rooted(root).await;
    let id = seed_empty_delta(&fx, &head).await;

    let env = fx
        .call_tool_json(
            "forgeplan_activate",
            serde_json::json!({"id": id, "force": true}),
        )
        .await;
    assert!(
        !env.is_error,
        "force must bypass the gate and activate, got error: {}",
        env.raw_text
    );
}
