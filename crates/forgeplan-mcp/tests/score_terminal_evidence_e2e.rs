//! ADR-020 / #436 — MCP surface parity for terminal-evidence exclusion.
//! The gate was wired into two surfaces before (#429 lesson: test every
//! affected surface, not one of them): this drives the real
//! `forgeplan_score` MCP handler through the PRD-177 chain and asserts the
//! recovered r_eff plus the `excluded`/`status` DTO fields agents read.

mod common;
use common::McpFixture;

const REFUTES_BODY: &str = "## Summary\n\ntypecheck fails\n\n## Structured Fields\n\nverdict: refutes\ncongruence_level: 3\nevidence_type: test\n";
const SUPPORTS_BODY: &str = "## Summary\n\nfixed, re-verified\n\n## Structured Fields\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n";

async fn new_artifact(fx: &McpFixture, kind: &str, title: &str) -> String {
    let created = fx
        .call_tool_json(
            "forgeplan_new",
            serde_json::json!({"kind": kind, "title": title}),
        )
        .await;
    created.assert_ok()["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn mcp_score_recovers_after_supersede_and_flags_excluded() {
    let fx = McpFixture::new().await;

    let prd = new_artifact(&fx, "prd", "PRD-177 repro over MCP").await;
    let refutes = new_artifact(&fx, "evidence", "typecheck BLOCKER found").await;
    fx.call_tool_json(
        "forgeplan_update",
        serde_json::json!({"id": refutes, "body": REFUTES_BODY}),
    )
    .await
    .assert_ok();
    fx.call_tool_json(
        "forgeplan_link",
        serde_json::json!({"source": refutes, "target": prd, "relation": "informs"}),
    )
    .await
    .assert_ok();
    fx.call_tool_json(
        "forgeplan_activate",
        serde_json::json!({"id": refutes, "force": true}),
    )
    .await
    .assert_ok();

    // Active refutes → 0.0 (guardrail unchanged on the MCP surface too).
    let before = fx
        .call_tool_json("forgeplan_score", serde_json::json!({"id": prd}))
        .await;
    assert_eq!(before.assert_ok()["r_eff"].as_f64().unwrap(), 0.0);

    // Displace it with a linked re-verification pack.
    let supports = new_artifact(&fx, "evidence", "re-verification passes").await;
    fx.call_tool_json(
        "forgeplan_update",
        serde_json::json!({"id": supports, "body": SUPPORTS_BODY}),
    )
    .await
    .assert_ok();
    fx.call_tool_json(
        "forgeplan_link",
        serde_json::json!({"source": supports, "target": prd, "relation": "informs"}),
    )
    .await
    .assert_ok();
    fx.call_tool_json(
        "forgeplan_supersede",
        serde_json::json!({"id": refutes, "by": supports}),
    )
    .await
    .assert_ok();

    let after = fx
        .call_tool_json("forgeplan_score", serde_json::json!({"id": prd}))
        .await;
    let payload = after.assert_ok();
    assert_eq!(
        payload["r_eff"].as_f64().unwrap(),
        1.0,
        "MCP r_eff must recover after displacement (ADR-020): {payload}"
    );

    let ev = payload["evidence"].as_array().expect("evidence array");
    let displaced = ev
        .iter()
        .find(|e| e["id"].as_str() == Some(refutes.as_str()))
        .expect("superseded pack still listed for the agent");
    assert_eq!(displaced["excluded"].as_bool(), Some(true));
    assert_eq!(displaced["status"].as_str(), Some("superseded"));
    // Raw own-merit score stays visible (0.0 for a CL3 refutes), clearly
    // separated from the aggregate by the excluded flag.
    assert_eq!(displaced["score"].as_f64(), Some(0.0));

    let live = ev
        .iter()
        .find(|e| e["id"].as_str() == Some(supports.as_str()))
        .expect("successor listed");
    assert_eq!(live["excluded"].as_bool(), Some(false));
}

/// Audit BLOCKER guard on the MCP surface: `forgeplan_update` may not write
/// lifecycle-bearing statuses — `superseded`/`deprecated` (score-laundering
/// bypass: no successor, no edge, no transition validation, no journal) and
/// `active` (would skip the validation/R_eff/provenance gates).
#[tokio::test]
async fn mcp_update_rejects_lifecycle_status_writes() {
    let fx = McpFixture::new().await;

    let prd = new_artifact(&fx, "prd", "gate probe").await;
    let evid = new_artifact(&fx, "evidence", "hostile pack").await;
    fx.call_tool_json(
        "forgeplan_update",
        serde_json::json!({"id": evid, "body": REFUTES_BODY}),
    )
    .await
    .assert_ok();
    fx.call_tool_json(
        "forgeplan_link",
        serde_json::json!({"source": evid, "target": prd, "relation": "informs"}),
    )
    .await
    .assert_ok();

    for status in ["superseded", "deprecated", "active"] {
        let env = fx
            .call_tool_json(
                "forgeplan_update",
                serde_json::json!({"id": evid, "status": status}),
            )
            .await;
        assert!(
            env.is_error,
            "forgeplan_update status={status} must be rejected, got: {}",
            env.raw_text
        );
    }

    // The pack is untouched — still draft.
    let got = fx
        .call_tool_json("forgeplan_get", serde_json::json!({"id": evid}))
        .await;
    assert_eq!(got.assert_ok()["status"].as_str(), Some("draft"));
}
