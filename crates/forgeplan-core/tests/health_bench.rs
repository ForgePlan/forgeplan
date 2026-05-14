//! PROB-051 + PRD-077 FR-026 performance bench — `health_report_with_phase`
//! parameterised over multiple workspace sizes to catch O(N²) regressions.
//!
//! Why parameterise
//! ----------------
//! Round-1 audit (TST-003) observed that a single-point bench at N=1000
//! cannot distinguish linear scaling from O(N log N) or O(N²) drift —
//! you need at least two data points whose ratio tightens or loosens.
//! This bench runs at N ∈ {1000, 5000, 10000} and checks that the
//! per-artifact warm latency does not balloon by more than 2× as N
//! grows 10×.
//!
//! Why not `criterion`
//! -------------------
//! Adding `criterion` is a separate change with its own approval surface
//! (build-time, CI cache size, baseline files committed to repo). This
//! bench uses `std::time::Instant`, prints a human-readable table, and
//! asserts a single ratio invariant — sufficient for the FR-026 gate.
//!
//! Marked `#[ignore]` so `cargo test` does not pay the seed cost on every
//! invocation. Run manually:
//!
//!   cargo test -p forgeplan-core --features test-helpers \
//!     --test health_bench -- --ignored --nocapture
//!
//! The CI surface is the new `.github/workflows/perf.yml` which fires
//! on `workflow_dispatch` + weekly cron and saves stdout as an artifact
//! for trend tracking.
//!
//! Cannot run under `--release` — `LanceStore::create_artifact_for_test`
//! is gated on `debug_assertions`. Numbers are dev-profile relative.

#![cfg(feature = "test-helpers")]

use std::time::{Duration, Instant};

use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::health::health_report_with_phase;
use forgeplan_core::phase::{Phase, store as phase_store};
use tempfile::TempDir;

const KINDS: &[&str] = &[
    "prd", "rfc", "adr", "epic", "spec", "problem", "solution", "note", "evidence", "refresh",
];

/// Seed `total` artifacts into a fresh workspace. The first `active_with_phase`
/// of them get `status=active` plus phase-state metadata so the parallel
/// `read_phase` path is exercised.
async fn seed_workspace(
    workspace: &std::path::Path,
    total: usize,
    active_with_phase: usize,
) -> LanceStore {
    let store = LanceStore::init(workspace).await.expect("init store");
    for i in 0..total {
        let kind = KINDS[i % KINDS.len()];
        let id = format!("{}-BENCH-{i:05}", kind.to_uppercase());
        let status = if i < active_with_phase {
            "active"
        } else if i < active_with_phase * 2 {
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

    let phases = [
        Phase::Shape,
        Phase::Validate,
        Phase::Adi,
        Phase::Code,
        Phase::Done,
    ];
    for i in 0..active_with_phase {
        let kind = KINDS[i % KINDS.len()];
        let id = format!("{}-BENCH-{i:05}", kind.to_uppercase());
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

/// Run cold + 3 warm scans against a workspace of the given size; return
/// (cold, warm_avg).
async fn measure_one(total: usize, active_with_phase: usize) -> (Duration, Duration) {
    let tempdir = TempDir::new().expect("tempdir");
    let workspace = tempdir.path().join(".forgeplan");
    std::fs::create_dir_all(&workspace).expect("workspace dir");

    let seed_start = Instant::now();
    let store = seed_workspace(&workspace, total, active_with_phase).await;
    eprintln!(
        "[bench] N={total} seed: {} artifacts in {:?}",
        total,
        seed_start.elapsed()
    );

    let cold_start = Instant::now();
    let (_report, _mismatches) = health_report_with_phase(&store, &workspace)
        .await
        .expect("health_report_with_phase cold");
    let cold = cold_start.elapsed();

    let mut warm_total = Duration::ZERO;
    let warm_runs = 3;
    for _ in 0..warm_runs {
        let start = Instant::now();
        let (_r, _m) = health_report_with_phase(&store, &workspace)
            .await
            .expect("health_report_with_phase warm");
        warm_total += start.elapsed();
    }
    let warm_avg = warm_total / warm_runs as u32;

    eprintln!("[bench] N={total} cold={cold:?} warm_avg={warm_avg:?}");
    (cold, warm_avg)
}

/// Single-point bench retained for backward compatibility (PROB-051 P-H1+H2
/// regression guard at the original 1000-artifact size).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "perf bench — run with --ignored"]
async fn bench_health_report_with_phase_warm_latency() {
    let (cold, warm) = measure_one(1000, 200).await;
    eprintln!("[bench] single-point 1000: cold={cold:?} warm_avg={warm:?}");
    // Generous CI ceiling — real numbers should be well under 1s.
    assert!(
        warm < Duration::from_millis(2000),
        "warm scan budget exceeded at N=1000: {warm:?}"
    );
}

/// FR-026: multi-point bench across N ∈ {1000, 5000, 10000} verifying
/// per-artifact latency does not balloon by more than 2× as N grows 10×.
///
/// If health_report_with_phase silently regresses to O(N²) — e.g. someone
/// re-introduces a per-artifact LanceDB query inside the duplicate-pair
/// hot loop — the 10× growth in N would produce ~100× growth in time,
/// way over the 2× per-artifact-ratio gate. Linear scaling (the current
/// design) keeps the ratio ≤ 1 modulo measurement noise.
///
/// Why per-artifact ratio rather than absolute time: machine-portable.
/// A slow CI runner inflates all three points equally; only the SHAPE
/// of the curve matters.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "perf bench multi-point — run with --ignored"]
async fn bench_health_scaling_1k_5k_10k() {
    // (size, active_with_phase)
    let points = [(1000_usize, 200_usize), (5000, 1000), (10000, 2000)];

    let mut results: Vec<(usize, Duration, Duration)> = Vec::new();
    for (n, active) in points {
        let (cold, warm) = measure_one(n, active).await;
        results.push((n, cold, warm));
    }

    eprintln!();
    eprintln!("[bench] === scaling table ===");
    eprintln!("[bench] N      | cold       | warm_avg   | warm/N (µs/artifact)");
    eprintln!("[bench] -------|------------|------------|---------------------");
    for (n, cold, warm) in &results {
        let per_artifact_us = warm.as_micros() as f64 / *n as f64;
        eprintln!("[bench] {n:6} | {cold:>10?} | {warm:>10?} | {per_artifact_us:>10.2}",);
    }
    eprintln!();

    // Per-artifact (µs) ratio gate: warm-per-N at 10000 must not exceed
    // 2× warm-per-N at 1000. Linear scaling produces ratio ≈ 1; quadratic
    // would produce ratio ≈ 10. The 2× allowance covers measurement
    // jitter + sub-linear effects (e.g. cache hits at small N inflate
    // the small-N per-artifact number).
    let per_1k = results[0].2.as_micros() as f64 / results[0].0 as f64;
    let per_10k = results[2].2.as_micros() as f64 / results[2].0 as f64;
    let ratio = per_10k / per_1k;

    eprintln!("[bench] per-artifact ratio (10k vs 1k): {ratio:.2}× (gate: ≤ 2.0×)");

    assert!(
        ratio <= 2.0,
        "FR-026 scaling regression: per-artifact warm latency at N=10000 \
         ({per_10k:.2} µs/artifact) is {ratio:.2}× the N=1000 rate ({per_1k:.2} µs/artifact). \
         Suspect O(N²) or O(N log N) drift in health_report_with_phase. \
         Inspect duplicate-pair detection or evidence indexing for newly-introduced \
         per-artifact queries."
    );

    // Also keep an absolute ceiling on the largest workspace — generous
    // so CI runners don't flake on cold cache.
    let warm_10k = results[2].2;
    assert!(
        warm_10k < Duration::from_secs(30),
        "warm scan at N=10000 exceeded 30s: {warm_10k:?}"
    );
}
