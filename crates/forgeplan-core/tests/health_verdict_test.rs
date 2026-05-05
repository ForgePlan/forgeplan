//! Integration tests for PROB-029 health verdict aggregator.
//!
//! These exercise `health_report` end-to-end against a real LanceDB
//! store seeded with a stub PRD, a duplicate evidence pair, and an
//! orphan, then assert the aggregated `verdict` is *not* `Healthy` and
//! that `next_actions` carries concrete remediation commands.
//!
//! Pre-fix the same workspace produced `verdict = Healthy` and a
//! "Project looks healthy" line in `next_actions`, in direct
//! contradiction of the warnings printed above.

use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::health::{Verdict, health_report};
use tempfile::TempDir;

async fn make_store(tmp: &TempDir) -> LanceStore {
    let ws = tmp.path().join(".forgeplan");
    LanceStore::init(&ws).await.unwrap()
}

fn stub_body() -> String {
    // Three-marker stub body that `validation::rules::check_stub`
    // recognises. Mirrors the dogfood workspace's stub PRDs.
    "## Vision\nWhat we are building and why\n\n## Users\n[Actor] can [capability]\n\n## Notes\nUse {placeholder} here\n".into()
}

fn new_prd_stub(id: &str) -> NewArtifact {
    NewArtifact {
        id: id.into(),
        kind: "prd".into(),
        status: "active".into(), // active stub — the PROB-029 surface
        title: format!("Stub {id}"),
        body: stub_body(),
        depth: "standard".into(),
        author: Some("tester".into()),
        parent_epic: None,
        valid_until: None,
        tags: Vec::new(),
    }
}

fn new_prd_clean(id: &str, title: &str) -> NewArtifact {
    NewArtifact {
        id: id.into(),
        kind: "prd".into(),
        status: "draft".into(),
        title: title.into(),
        body: "## Problem\n\nReal text that is not a stub.\n\n## Goals\n\nMore real text.\n".into(),
        depth: "standard".into(),
        author: Some("tester".into()),
        parent_epic: None,
        valid_until: None,
        tags: Vec::new(),
    }
}

// PROB-029 acceptance criterion #5: integration test creates a
// workspace with 1 stub PRD, runs health, asserts verdict ≠ "healthy".
#[tokio::test]
async fn health_verdict_not_healthy_when_one_active_stub_present() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(&tmp).await;

    // Seed exactly one active stub PRD — the minimum reproducer.
    store
        .create_artifact_for_test(&new_prd_stub("PRD-100"))
        .await
        .expect("create_artifact_for_test");

    let report = health_report(&store).await.expect("health_report");

    // Smoke check — the stub detector saw it.
    assert_eq!(
        report.active_stubs.len(),
        1,
        "one active stub should be detected"
    );

    // PROB-029 AC-1: verdict is NOT Healthy.
    assert_ne!(
        report.verdict,
        Verdict::Healthy,
        "verdict must not be Healthy when 1 active stub is present, got {:?}",
        report.verdict,
    );
    assert_eq!(
        report.verdict,
        Verdict::NeedsAttention,
        "verdict should be NeedsAttention (1 stub is below Unhealthy threshold)",
    );

    // PROB-029 AC-3: next_actions is non-empty and includes a
    // concrete remediation command.
    assert!(
        !report.next_actions.is_empty(),
        "next_actions must not be empty"
    );
    assert!(
        report.next_actions.iter().any(|a| a.contains("PRD-100")),
        "next_actions must mention the stub id, got {:?}",
        report.next_actions,
    );
    // PROB-029 AC-1 regression: no "Project looks healthy" line.
    assert!(
        !report
            .next_actions
            .iter()
            .any(|a| a.contains("looks healthy")),
        "next_actions must not say 'looks healthy', got {:?}",
        report.next_actions,
    );
}

// PROB-029: empty workspace stays Healthy. Anti-Goodhart: don't flip
// PR-E Round 6 audit MED fix: empty workspace now reports
// `Verdict::Empty`, NOT `Verdict::Healthy`. The pre-fix behavior broke
// CI gates that auto-promoted on `verdict == "healthy"` for an
// uninitialized project. Test renamed to reflect new semantic.
#[tokio::test]
async fn health_verdict_is_empty_for_empty_workspace() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(&tmp).await;

    let report = health_report(&store).await.expect("health_report");

    assert_eq!(report.total, 0);
    assert_eq!(
        report.verdict,
        Verdict::Empty,
        "empty workspace must be Verdict::Empty, NOT Healthy — \
         CI gates would auto-promote uninitialized projects",
    );
    assert_ne!(
        report.verdict,
        Verdict::Healthy,
        "explicit guard against regression to pre-Round-6 Healthy verdict",
    );
}

// PROB-029: a workspace with a clean draft PRD (no stub markers, but
// also no evidence yet) should be Healthy — drafts don't count toward
// blind spots, and a single orphan draft alone is below all critical
// thresholds. Acts as a guard against "every workspace is unhealthy"
// false positives.
#[tokio::test]
async fn health_verdict_clean_draft_workspace_is_needs_attention_not_unhealthy() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(&tmp).await;

    store
        .create_artifact_for_test(&new_prd_clean("PRD-200", "Clean PRD"))
        .await
        .expect("create_artifact_for_test");

    let report = health_report(&store).await.expect("health_report");
    // The lone draft PRD with no relations is an orphan → at most
    // NeedsAttention with default thresholds.
    assert_ne!(
        report.verdict,
        Verdict::Unhealthy,
        "1 orphan draft must not promote to Unhealthy, got {:?}",
        report.verdict,
    );
}

// PROB-029 dogfood-style snapshot: 6 active stubs (above default
// threshold of 3) → Unhealthy. This is the regression guard for the
// scenario captured in the PROB-029 body.
#[tokio::test]
async fn health_verdict_many_active_stubs_promotes_to_unhealthy() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(&tmp).await;

    for i in 0..6 {
        let id = format!("PRD-{:03}", 300 + i);
        store
            .create_artifact_for_test(&new_prd_stub(&id))
            .await
            .expect("create_artifact_for_test");
    }

    let report = health_report(&store).await.expect("health_report");
    assert_eq!(
        report.active_stubs.len(),
        6,
        "all 6 stubs should be detected"
    );
    assert_eq!(
        report.verdict,
        Verdict::Unhealthy,
        "6 active stubs (> threshold 3) must promote verdict to Unhealthy, got {:?}",
        report.verdict,
    );
    // No "looks healthy" line in any action.
    assert!(
        !report
            .next_actions
            .iter()
            .any(|a| a.contains("looks healthy")),
        "Unhealthy workspace must not emit 'looks healthy' line",
    );
}
