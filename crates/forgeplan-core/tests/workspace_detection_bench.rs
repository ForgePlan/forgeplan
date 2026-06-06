//! PRD-078 NFR-001 / SC-2 + PROB-073 — `detect_multi_worktree` per-call latency.
//!
//! What this measures
//! ------------------
//! PRD-078 added a multi-worktree detection gate to every *mutating* MCP tool
//! call (`resolve_workspace` cold-start path). The gate is two synchronous
//! `git rev-parse` subprocess calls (`--git-dir` + `--git-common-dir`) plus two
//! `canonicalize`s. NFR-001 / SC-2 budgets this overhead at **< 5 ms p95** per
//! tool call. This bench measures the real cost on both detection branches:
//!
//!   * **linked worktree** — detection fires (`true`); the hot path a subagent
//!     hits on every mutating call inside a `git worktree`.
//!   * **plain main repo** — detection short-circuits (`false`); the path a
//!     single-worktree user hits (backward-compat — must also be cheap).
//!
//! Verdict thresholds (ADR-015 §Evidence Requirements)
//! ---------------------------------------------------
//! - **supports** — p95 < 5 ms — SC-2 met, no caching needed.
//! - **weakens** — 5 ms ≤ p95 < 50 ms — add a session-level detection cache (ADR-015 R-1).
//! - **refutes** — p95 ≥ 50 ms — per-call detection model wrong; caching mandatory pre-ship.
//!
//! Scope note (terminology precision)
//! ----------------------------------
//! This closes the **narrow** PRD-078 SC-2 question (detection overhead). It does
//! NOT measure the **broad** PROB-073 complaint — the full `forgeplan_new` /
//! `forgeplan_link` MCP roundtrip (LanceDB open/write/commit, projection). That
//! is a separate, larger profiling track; detection is one small component of it.
//!
//! Why not `criterion`
//! -------------------
//! Same rationale as `health_bench.rs`: adding `criterion` is its own approval
//! surface (build-time, CI cache, committed baselines). This uses
//! `std::time::Instant`, prints a human-readable percentile table, and asserts a
//! single generous sanity ceiling — the precise verdict lives in the printout +
//! EVID, not a brittle hard gate (a legitimate "weakens" must NOT fail the test).
//!
//! Marked `#[ignore]` so `cargo test` does not spawn git subprocesses on every
//! run. Run manually:
//!
//!   cargo test -p forgeplan-core --test workspace_detection_bench -- --ignored --nocapture

use std::process::Command;
use std::time::{Duration, Instant};

use forgeplan_core::workspace::detect_multi_worktree;
use tempfile::TempDir;

/// Init a git repo with one empty commit (mirrors the helper in
/// `workspace::init` detect tests).
fn init_git_repo(dir: &std::path::Path) -> bool {
    let ok = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        return false;
    }
    Command::new("git")
        .args(["commit", "--allow-empty", "-m", "root"])
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "t@t.t")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "t@t.t")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// `p`-th percentile of an already-sorted slice (nearest-rank).
fn percentile(sorted: &[Duration], p: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((p / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Measure `iterations` calls of `detect_multi_worktree(cwd)`, returning the
/// per-call durations sorted ascending.
fn measure(cwd: &std::path::Path, iterations: usize) -> Vec<Duration> {
    // Warm-up: prime the OS git binary page cache + filesystem stat cache so
    // the first measured call isn't an outlier.
    for _ in 0..5 {
        let _ = detect_multi_worktree(cwd);
    }
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = detect_multi_worktree(cwd);
        samples.push(start.elapsed());
    }
    samples.sort_unstable();
    samples
}

/// Print a percentile row + return the p95 for the verdict.
fn report(label: &str, expected: bool, cwd: &std::path::Path, iterations: usize) -> Duration {
    // Sanity: confirm the branch under measurement is the one we intend.
    let got = detect_multi_worktree(cwd);
    assert_eq!(
        got, expected,
        "{label}: detection branch mismatch — expected {expected}, got {got}"
    );

    let s = measure(cwd, iterations);
    let p50 = percentile(&s, 50.0);
    let p95 = percentile(&s, 95.0);
    let max = *s.last().unwrap();
    eprintln!(
        "[bench] {label:<22} | p50={p50:>9.3?} | p95={p95:>9.3?} | max={max:>9.3?} | n={iterations}"
    );
    p95
}

fn verdict(p95: Duration) -> &'static str {
    if p95 < Duration::from_millis(5) {
        "supports (p95 < 5ms — SC-2 met)"
    } else if p95 < Duration::from_millis(50) {
        "weakens (5–50ms — add session cache per ADR-015 R-1)"
    } else {
        "refutes (≥ 50ms — per-call detection model wrong)"
    }
}

/// NFR-001 / SC-2: per-call `detect_multi_worktree` overhead on both branches.
///
/// Prints a percentile table + an ADR-015 verdict. Asserts only a generous
/// sanity ceiling (catch a deadlock / runaway, not jitter) — the real
/// supports/weakens/refutes call is recorded in the EVID from the printed p95,
/// because a "weakens" result is a legitimate signal (→ add caching), not a
/// test failure.
#[test]
#[ignore = "perf bench — run with --ignored --nocapture"]
fn bench_detect_multi_worktree_latency() {
    // ── Setup: main repo + one linked worktree ───────────────────────────
    let main_tmp = TempDir::new().unwrap();
    if !init_git_repo(main_tmp.path()) {
        eprintln!("SKIP bench: git init failed — no git in PATH?");
        return;
    }
    let linked_tmp = TempDir::new().unwrap();
    let added = Command::new("git")
        .args([
            "-C",
            &main_tmp.path().to_string_lossy(),
            "worktree",
            "add",
            &linked_tmp.path().to_string_lossy(),
            "-b",
            "wt-bench",
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !added {
        eprintln!("SKIP bench: git worktree add failed — restricted CI tmpfs?");
        return;
    }

    let iterations = 100;
    eprintln!();
    eprintln!("[bench] === detect_multi_worktree latency (NFR-001 / SC-2) ===");
    eprintln!("[bench] each call = 2× `git rev-parse` subprocess + 2× canonicalize");

    // Hot path: subagent inside a linked worktree (detection fires).
    let p95_linked = report("linked (detect=true)", true, linked_tmp.path(), iterations);
    // Backward-compat path: single-worktree user (detection short-circuits).
    let p95_main = report("main (detect=false)", false, main_tmp.path(), iterations);

    eprintln!();
    eprintln!(
        "[bench] verdict (linked, the hot path): {}",
        verdict(p95_linked)
    );
    eprintln!(
        "[bench] verdict (main, single-worktree): {}",
        verdict(p95_main)
    );
    eprintln!();

    // Generous sanity ceiling only — a real "weakens" (5–50ms) must NOT fail
    // here; that outcome legitimately triggers the session-cache follow-up.
    // 500ms would indicate a hang or pathological subprocess spawn, not jitter.
    let ceiling = Duration::from_millis(500);
    assert!(
        p95_linked < ceiling,
        "detect_multi_worktree p95 on linked worktree ({p95_linked:?}) exceeded the \
         sanity ceiling ({ceiling:?}) — suspect a hung/pathological git subprocess, \
         not measurement jitter."
    );
    assert!(
        p95_main < ceiling,
        "detect_multi_worktree p95 on main repo ({p95_main:?}) exceeded the sanity \
         ceiling ({ceiling:?})."
    );
}
