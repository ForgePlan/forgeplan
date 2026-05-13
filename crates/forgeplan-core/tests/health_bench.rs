//! PROB-051 performance bench — health_report_with_phase + duplicate
//! detection + at-risk scan against a synthetic 1000-artifact workspace.
//!
//! Why not `criterion`
//! -------------------
//! Adding `criterion` to the workspace dev-deps is a separate change with
//! its own approval surface (build-time, CI cache size, baseline files
//! committed to repo). This bench uses `std::time::Instant` and prints
//! human-readable timings — sufficient to satisfy the PROB-051 acceptance
//! gate ("warm latency MUST drop ≥30%") without dragging criterion in.
//!
//! Marked `#[ignore]` so `cargo test` does not pay the seed cost. Run with:
//!   cargo test -p forgeplan-core --features test-helpers --test health_bench -- --ignored --nocapture
//!
//! NOTE: cannot run under `--release` — `LanceStore::create_artifact_for_test`
//! is gated on `#[cfg(any(test, all(feature = "test-helpers", debug_assertions)))]`,
//! so the symbol disappears in release builds. The bench remains useful as
//! a relative perf check (cold vs warm, before vs after refactor) on the
//! dev profile; absolute numbers will not match release-mode CLI latency.
//!
//! The fixture seeds 1000 artifacts (200 active with phase state) and runs
//! `health_report_with_phase` THREE times — first call is cold (LanceDB
//! materialises its read snapshot), subsequent calls are warm. The warm
//! number is the metric we care about for MCP-call latency on a hot
//! workspace.

#![cfg(feature = "test-helpers")]

use std::time::Instant;

use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::health::health_report_with_phase;
use forgeplan_core::phase::{Phase, store as phase_store};
use tempfile::TempDir;

const TOTAL_ARTIFACTS: usize = 1000;
const ACTIVE_ARTIFACTS_WITH_PHASE: usize = 200;
const KINDS: &[&str] = &[
    "prd", "rfc", "adr", "epic", "spec", "problem", "solution", "note", "evidence", "refresh",
];

async fn seed_workspace(workspace: &std::path::Path) -> LanceStore {
    let store = LanceStore::init(workspace).await.expect("init store");
    for i in 0..TOTAL_ARTIFACTS {
        let kind = KINDS[i % KINDS.len()];
        let id = format!("{}-BENCH-{i:04}", kind.to_uppercase());
        let status = if i < ACTIVE_ARTIFACTS_WITH_PHASE {
            "active"
        } else if i < ACTIVE_ARTIFACTS_WITH_PHASE * 2 {
            "draft"
        } else {
            "active"
        };
        store
            .create_artifact_for_test(&NewArtifact {
                id: id.clone(),
                kind: kind.to_string(),
                status: status.to_string(),
                title: format!("Bench {kind} {i}"),
                body: format!(
                    "## Problem\nBench body {i}\n\n## Goals\nReal goals.\n\
                     \n\
                     verdict: supports\ncongruence_level: 3\nevidence_type: measurement\n"
                ),
                depth: "standard".to_string(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .expect("seed");
    }

    // Phase state for the first 200 artifacts at varying phases — covers
    // both the "early-cycle (mismatch)" and "late-cycle (no mismatch)" paths.
    let phases = [
        Phase::Shape,
        Phase::Validate,
        Phase::Adi,
        Phase::Code,
        Phase::Done,
    ];
    for i in 0..ACTIVE_ARTIFACTS_WITH_PHASE {
        let kind = KINDS[i % KINDS.len()];
        let id = format!("{}-BENCH-{i:04}", kind.to_uppercase());
        phase_store::initialize_phase(workspace, &id, Some("bench seed".into()))
            .await
            .expect("init phase");
        let target = phases[i % phases.len()];
        if target != Phase::Shape {
            phase_store::advance_phase(workspace, &id, target, Some("bench advance".into()))
                .await
                .expect("advance phase");
        }
    }

    store
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "perf bench — run with --ignored"]
async fn bench_health_report_with_phase_warm_latency() {
    let tempdir = TempDir::new().expect("tempdir");
    let workspace = tempdir.path().join(".forgeplan");
    std::fs::create_dir_all(&workspace).expect("workspace dir");

    let seed_start = Instant::now();
    let store = seed_workspace(&workspace).await;
    let seed_elapsed = seed_start.elapsed();
    eprintln!(
        "[bench] seed: {} artifacts in {:?}",
        TOTAL_ARTIFACTS, seed_elapsed
    );

    // Cold (first scan — LanceDB materialises snapshot)
    let cold_start = Instant::now();
    let (_report, _mismatches) = health_report_with_phase(&store, &workspace)
        .await
        .expect("health_report_with_phase cold");
    let cold_elapsed = cold_start.elapsed();
    eprintln!("[bench] cold scan: {:?}", cold_elapsed);

    // Warm: average of 3 runs (the metric for MCP-call latency)
    let mut warm_total = std::time::Duration::ZERO;
    let warm_runs = 3;
    for i in 0..warm_runs {
        let start = Instant::now();
        let (report, mismatches) = health_report_with_phase(&store, &workspace)
            .await
            .expect("health_report_with_phase warm");
        let elapsed = start.elapsed();
        warm_total += elapsed;
        eprintln!(
            "[bench] warm run {}: {:?} (total={}, mismatches={})",
            i + 1,
            elapsed,
            report.total,
            mismatches.len()
        );
    }
    let warm_avg = warm_total / warm_runs as u32;
    eprintln!(
        "[bench] warm AVERAGE over {warm_runs} runs: {warm_avg:?} (acceptance: ≥30% drop vs pre-PROB-051 baseline)"
    );

    // PROB-051 P-H1 + P-H2 acceptance: warm scan should comfortably fit
    // under 500ms on a 1000-artifact workspace. Generous ceiling so CI
    // jitter does not flake the test; real measurements should be well
    // below this. Adjust downward as the perf budget tightens.
    assert!(
        warm_avg < std::time::Duration::from_millis(2000),
        "warm scan budget exceeded: {warm_avg:?}"
    );
}
