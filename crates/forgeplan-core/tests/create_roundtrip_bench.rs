//! PROB-073 (broad) — `forgeplan_new` create-roundtrip latency breakdown.
//!
//! The user's complaint: creating an artifact through the MCP server feels slow,
//! while writing the markdown file directly + syncing "flies". This bench
//! decomposes the **core** create path into its components to find where the
//! time goes BEFORE anyone optimizes (measure-first; plan risk #1 — the profile
//! decides v0.33-symptom-fix vs v0.34-engine-rewrite).
//!
//! Components measured (per `forgeplan_new`):
//!   * **store init** — `LanceStore::init` (one-time per workspace; cached by the
//!     MCP server's `workspace_store_cache` since PRD-078, so NOT per-call —
//!     reported once for context).
//!   * **lance write** — `create_artifact_for_test` = the `add().execute()`
//!     LanceDB insert + commit (manifest write / fsync).
//!   * **full roundtrip** — `create_artifact_with_projection` = id/kind validate
//!     + markdown render + file write + the lance write above.
//!   * **projection delta** — (full − lance) ≈ the render + file-I/O cost.
//!
//! Embedding is NOT in this path (feature-gated `semantic-search`/fastembed,
//! computed separately), so it is intentionally excluded.
//!
//! Why not `criterion`: same as `health_bench.rs` — `Instant`, `#[ignore]`,
//! human-readable table, no new dep.
//!
//!   cargo test -p forgeplan-core --features test-helpers \
//!     --test create_roundtrip_bench -- --ignored --nocapture

#![cfg(feature = "test-helpers")]

use std::time::{Duration, Instant};

use forgeplan_core::db::store::{LanceStore, NewArtifact};
use forgeplan_core::projection::{MutationContext, create_artifact_with_projection};
use tempfile::TempDir;

fn art(i: usize) -> NewArtifact {
    NewArtifact {
        id: format!("PRD-BENCH-{i:05}"),
        kind: "prd".to_string(),
        status: "draft".to_string(),
        title: format!("Roundtrip bench {i}"),
        body: "## Problem\nBench body.\n\n## Goals\nReal goals.\n".to_string(),
        depth: "standard".to_string(),
        author: None,
        parent_epic: None,
        valid_until: None,
        tags: Vec::new(),
    }
}

fn pct(sorted: &[Duration], p: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((p / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "perf bench — run with --ignored --nocapture"]
async fn bench_create_roundtrip_components() {
    let dir = TempDir::new().unwrap();
    let ws = dir.path().join(".forgeplan");
    std::fs::create_dir_all(&ws).unwrap();

    // ── store init (one-time per workspace; cached per-workspace in prod) ──
    let t = Instant::now();
    let store = LanceStore::init(&ws).await.expect("init store");
    let store_init = t.elapsed();

    // warm-up — prime LanceDB table handles + FS caches.
    for i in 0..3 {
        store
            .create_artifact_for_test(&art(i))
            .await
            .expect("warmup");
    }

    let n = 50usize;

    // ── lance write only (add + execute = insert + commit) ────────────────
    let mut lance = Vec::with_capacity(n);
    for i in 1000..1000 + n {
        let t = Instant::now();
        store
            .create_artifact_for_test(&art(i))
            .await
            .expect("lance write");
        lance.push(t.elapsed());
    }

    // ── full roundtrip (validate + render + file write + lance write) ─────
    let ctx = MutationContext::new(&ws, &store);
    let mut full = Vec::with_capacity(n);
    for i in 2000..2000 + n {
        let t = Instant::now();
        create_artifact_with_projection(&ctx, &art(i))
            .await
            .expect("full roundtrip");
        full.push(t.elapsed());
    }

    lance.sort_unstable();
    full.sort_unstable();

    let lance_p50 = pct(&lance, 50.0);
    let lance_p95 = pct(&lance, 95.0);
    let full_p50 = pct(&full, 50.0);
    let full_p95 = pct(&full, 95.0);
    let proj_p50 = full_p50.saturating_sub(lance_p50);

    eprintln!();
    eprintln!("[bench] === forgeplan_new create-roundtrip breakdown (n={n}) ===");
    eprintln!("[bench] store init (one-time, cached in prod) : {store_init:>10.3?}");
    eprintln!(
        "[bench] lance write (insert+commit)   p50/p95 : {lance_p50:>10.3?} / {lance_p95:.3?}"
    );
    eprintln!("[bench] full roundtrip (render+io+db) p50/p95 : {full_p50:>10.3?} / {full_p95:.3?}");
    eprintln!("[bench] projection delta (render+fileio) p50  : {proj_p50:>10.3?}");
    eprintln!();
    let dom = if proj_p50 > lance_p50 {
        "PROJECTION (render + file I/O)"
    } else {
        "LANCE WRITE (insert + commit/fsync)"
    };
    eprintln!("[bench] dominant component @ p50: {dom}");
    eprintln!();

    // Sanity ceiling only (catch a hang/runaway, not jitter) — the profile
    // itself is the deliverable, not a pass/fail gate.
    assert!(
        full_p95 < Duration::from_secs(5),
        "full create roundtrip p95 ({full_p95:?}) exceeded 5s — suspect a hang, not jitter"
    );
}
