//! PROB-051 L-H3 acceptance — CLI vs MCP verdict parity test.
//!
//! Pre-PROB-051 the CLI surface called `health_report` (no phase fold) while
//! the MCP server called `health_report` AND scanned phase state separately,
//! producing a different `verdict` for the same workspace. Operators reading
//! `forgeplan health --json` and an MCP `forgeplan_health` response side by
//! side would see contradictory verdicts when phase mismatches existed.
//!
//! After L-H3 closure both surfaces route through `health_report_with_phase`,
//! which folds `phase_mismatches.len()` into the verdict via
//! `compute_verdict_with`. This test synthesises a workspace containing 6
//! phase mismatches (6 active artifacts whose recorded phase is still in
//! early-cycle Shape/Validate/Adi state), runs BOTH surfaces against the
//! SAME workspace, and asserts `verdict` plus `verdict_summary` are byte-
//! identical.
//!
//! Note on the "CLI" half: invoking the `forgeplan` shell binary from a
//! library integration test would require building it first and tracking
//! its path. The CLI surface itself is a thin wrapper over
//! `health::health_report_with_phase` (see
//! `crates/forgeplan-cli/src/commands/health.rs::run`), so this test calls
//! the same library function the CLI binary calls. Folding the verdict via
//! `compute_verdict_with(..., phase_mismatches.len())` mirrors the exact
//! sequence both the CLI and MCP server perform. Any future drift between
//! `forgeplan-cli::commands::health::run` and this test fixture would be
//! caught by the unit test suite in `forgeplan-core::health::tests`.

use forgeplan_core::db::store::NewArtifact;
use forgeplan_core::health::{Verdict, VerdictThresholds, health_report_with_phase};
use forgeplan_core::phase::{Phase, store as phase_store};

mod common;
use common::McpFixture;

/// Six artifact slugs — chosen across multiple kinds to exercise the
/// per-kind code paths and to cross the default phase-mismatch threshold
/// (5). 6 mismatches > 5 → if the verdict aggregator were folding
/// `phase_mismatches` as critical we'd see `Unhealthy`; today it's
/// advisory-only (PROB-063 contract) so the verdict is whatever the
/// other signals dictate. Either way, the CLI and MCP paths MUST agree.
const ACTIVE_ARTIFACTS: &[(&str, &str, &str, Phase)] = &[
    ("PRD-001", "prd", "Auth system PRD", Phase::Shape),
    ("PRD-002", "prd", "Billing pipeline PRD", Phase::Validate),
    ("RFC-001", "rfc", "Telemetry RFC", Phase::Adi),
    ("ADR-001", "adr", "Lance vs SQLite ADR", Phase::Shape),
    ("PROB-001", "problem", "Slow startup PROB", Phase::Validate),
    ("SOL-001", "solution", "Lazy init solution", Phase::Adi),
];

/// PROB-051 L-H3: CLI and MCP MUST produce identical `verdict` and
/// `verdict_summary` for the same workspace. Acceptance criterion: 6
/// phase mismatches across 6 kinds, both paths agree byte-for-byte.
#[tokio::test]
async fn cli_and_mcp_agree_on_verdict_with_six_phase_mismatches() {
    let fixture = McpFixture::new_with_seed(|store| async move {
        // Seed 6 active artifacts via the test-helpers create path so the
        // server-side reader sees them on first scan.
        for (id, kind, title, _phase) in ACTIVE_ARTIFACTS {
            store
                .create_artifact_for_test(&NewArtifact {
                    id: (*id).to_string(),
                    kind: (*kind).to_string(),
                    status: "active".to_string(),
                    title: (*title).to_string(),
                    // Filled body so active_stub detector does not fire
                    // (otherwise stubs > 3 critical would shift verdict).
                    body: format!(
                        "## Problem\nReal text describing the {kind} problem.\n\
                         \n\
                         ## Goals\nReal goals for {title}.\n"
                    ),
                    depth: "standard".to_string(),
                    author: None,
                    parent_epic: None,
                    valid_until: None,
                    tags: Vec::new(),
                })
                .await
                .expect("seed artifact");
        }
        // Seed one healthy evidence per artifact so neither the
        // blind-spot detector (kinds in DECISION_KINDS_EVIDENCE need
        // evidence) nor the orphan detector (no in-/out-edges) fires.
        // Evidence body carries explicit `verdict: supports` +
        // `congruence_level: 3` + `evidence_type: measurement` so the
        // R_eff parser does not fail-closed to CL0 and at_risk stays
        // empty too. Result: phase mismatches are the ONLY signal.
        for (i, (id, _kind, _title, _phase)) in ACTIVE_ARTIFACTS.iter().enumerate() {
            let evid = format!("EVID-L-H3-{i:03}");
            store
                .create_artifact_for_test(&NewArtifact {
                    id: evid.clone(),
                    kind: "evidence".to_string(),
                    status: "active".to_string(),
                    title: format!("Evidence for {id}"),
                    body: "## Structured Fields\n\
                           verdict: supports\n\
                           congruence_level: 3\n\
                           evidence_type: measurement\n"
                        .to_string(),
                    depth: "standard".to_string(),
                    author: None,
                    parent_epic: None,
                    valid_until: None,
                    tags: Vec::new(),
                })
                .await
                .expect("seed evidence");
            store
                .add_relation_for_test(&evid, id, "informs")
                .await
                .expect("seed informs relation");
        }
    })
    .await;

    // Write phase state files for each artifact AT an early-cycle phase
    // (Shape / Validate / Adi). `health_report_with_phase` reads these
    // and emits a `PhaseMismatch` for each active artifact stuck early.
    for (id, _kind, _title, phase) in ACTIVE_ARTIFACTS {
        phase_store::initialize_phase(
            &fixture.workspace_path,
            id,
            Some(format!("seed L-H3 test {phase:?}")),
        )
        .await
        .expect("init phase state");
        if *phase != Phase::Shape {
            phase_store::advance_phase(
                &fixture.workspace_path,
                id,
                *phase,
                Some("seed L-H3 test advance".to_string()),
            )
            .await
            .expect("advance phase state");
        }
    }

    // ── CLI-equivalent path: `health_report_with_phase` directly ─────
    // (`forgeplan health --json` in commands/health.rs calls this and
    // emits `verdict` + `verdict_summary` from the returned report.)
    let cli_store = forgeplan_core::db::store::LanceStore::open(&fixture.workspace_path)
        .await
        .expect("open store");
    let (cli_report, cli_mismatches) =
        health_report_with_phase(&cli_store, &fixture.workspace_path)
            .await
            .expect("health_report_with_phase");
    let cli_verdict = cli_report.verdict;
    let cli_summary = cli_verdict.human_summary();
    let cli_mismatch_count = cli_mismatches.len();

    // Sanity guard: fixture really did emit 6 phase mismatches.
    assert_eq!(
        cli_mismatch_count,
        ACTIVE_ARTIFACTS.len(),
        "expected {} phase mismatches, got {}",
        ACTIVE_ARTIFACTS.len(),
        cli_mismatch_count
    );

    // ── MCP path: live `forgeplan_health` over JSON-RPC ───────────────
    let envelope = fixture
        .call_tool_json("forgeplan_health", serde_json::json!({}))
        .await;
    let mcp_payload = envelope.assert_ok();
    let mcp_verdict_str = mcp_payload
        .get("verdict")
        .and_then(|v| v.as_str())
        .expect("MCP `verdict` field present");
    let mcp_summary = mcp_payload
        .get("verdict_summary")
        .and_then(|v| v.as_str())
        .expect("MCP `verdict_summary` field present");
    let mcp_mismatches_array = mcp_payload
        .get("advisory_phase_mismatches")
        .and_then(|v| v.as_array())
        .expect("MCP advisory_phase_mismatches field present");

    // ── Acceptance asserts ────────────────────────────────────────────
    // 1. Phase mismatch counts agree.
    assert_eq!(
        mcp_mismatches_array.len(),
        cli_mismatch_count,
        "MCP and CLI must report the same phase-mismatch count (MCP={}, CLI={})",
        mcp_mismatches_array.len(),
        cli_mismatch_count
    );

    // 2. Verdict strings agree (`verdict` field).
    assert_eq!(
        mcp_verdict_str,
        cli_verdict.as_str(),
        "L-H3: MCP verdict {:?} must equal CLI verdict {:?} for the same workspace",
        mcp_verdict_str,
        cli_verdict.as_str()
    );

    // 3. `verdict_summary` text agrees (driven off the same enum).
    assert_eq!(
        mcp_summary, cli_summary,
        "L-H3: MCP verdict_summary {:?} must equal CLI verdict_summary {:?}",
        mcp_summary, cli_summary
    );

    // 4. Sanity: when only advisory phase mismatches are present and no
    //    other signals fire, PROB-063 demands the verdict stays
    //    `Healthy` (phase mismatches are advisory-only). This pins the
    //    contract end-to-end across both surfaces.
    assert_eq!(
        cli_verdict,
        Verdict::Healthy,
        "PROB-063: advisory phase mismatches alone must not promote verdict (got {:?})",
        cli_verdict
    );

    // 5. Cross-check: the threshold table is still in sync — recomputing
    //    the verdict with default thresholds against the report's count
    //    yields the same value the MCP surface returned. Regression
    //    guard against future drift between `report.verdict` (computed
    //    inside `health_report_with_phase`) and the recompute path
    //    callers use when applying custom thresholds.
    let recomputed =
        cli_report.compute_verdict_with(&VerdictThresholds::default(), cli_mismatch_count);
    assert_eq!(
        recomputed.as_str(),
        cli_verdict.as_str(),
        "recompute drift: default thresholds must reproduce the stored verdict"
    );
}
