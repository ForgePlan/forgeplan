//! PROB-051 L-M3 — boundary tests at exact `VerdictThresholds` values.
//!
//! Each critical threshold class (`orphans`, `blind_spots`,
//! `active_stubs`, `duplicates`, `at_risk`) is exercised at three
//! boundary points:
//!
//! - Just BELOW threshold (`t - 1`) → `NeedsAttention`
//!   (warning present но никакая critical-class promoted).
//! - Exactly AT threshold (`t`) → still `NeedsAttention`
//!   (`compute_verdict_from_signals` uses strict `>` — threshold value
//!   is the MAXIMUM tolerated count, NOT the trip point).
//! - Just ABOVE threshold (`t + 1`) → `Unhealthy`
//!   (critical class fires).
//!
//! Rationale: pre-PROB-051 the boundary semantics were under-tested.
//! `compute_verdict` reviewers сложно verifies whether a `t`-count
//! workspace is "at limit, still tolerable" или "promoted". This file
//! pins down the contract so a future refactor accidentally swapping
//! `>` ↔ `>=` would break a focused, named test.
//!
//! These tests do NOT cover phase mismatches (intentionally excluded
//! from verdict promotion per PROB-063) или `stale` (advisory only —
//! floats verdict to `NeedsAttention`, never `Unhealthy`).
//!
//! Construction pattern: build a minimal `HealthReport` directly via
//! struct literal so each test stays self-contained (no shared
//! fixture state, no async setup, no LanceDB).

use forgeplan_core::health::{
    ActiveStub, AtRiskArtifact, BlindSpot, DuplicatePair, HealthReport, Verdict, VerdictThresholds,
};

/// Minimal `HealthReport` with the given total artifact count and all
/// signal vectors empty. Caller mutates only the field under test —
/// keeps each boundary case visually atomic.
///
/// `total > 0` so the empty-workspace short-circuit (`Verdict::Empty`)
/// does NOT fire. Pick 50 — comfortably above any conceivable
/// threshold so we never accidentally trip the empty path.
fn baseline_report() -> HealthReport {
    HealthReport {
        total: 50,
        by_kind: Vec::new(),
        by_status: Vec::new(),
        at_risk: Vec::new(),
        blind_spots: Vec::new(),
        stale_count: 0,
        orphans: Vec::new(),
        by_derived_status: Vec::new(),
        next_actions: Vec::new(),
        possible_duplicates: Vec::new(),
        active_stubs: Vec::new(),
        gitignore_drift: Vec::new(),
        // Placeholder verdicts; tests recompute via `compute_verdict_with`.
        verdict: Verdict::Healthy,
        partial_verdict: Verdict::Healthy,
    }
}

fn stub(id: &str) -> ActiveStub {
    ActiveStub {
        id: id.into(),
        kind: "prd".into(),
        title: "Boundary stub".into(),
        markers_found: 3,
        message: "boundary fixture".into(),
    }
}

fn dup(id_a: &str, id_b: &str) -> DuplicatePair {
    DuplicatePair {
        id_a: id_a.into(),
        id_b: id_b.into(),
        similarity: 1.0,
        title_a: "X".into(),
        title_b: "X".into(),
        kind: "evidence".into(),
    }
}

fn spot(id: &str) -> BlindSpot {
    BlindSpot {
        id: id.into(),
        title: "Boundary blind spot".into(),
        issue: "no evidence".into(),
    }
}

fn at_risk(id: &str) -> AtRiskArtifact {
    AtRiskArtifact {
        id: id.into(),
        title: "Boundary at-risk artifact".into(),
        reason: "evidence stale".into(),
    }
}

// ── orphans ──────────────────────────────────────────────────────────

/// `orphans = t - 1` (default threshold 5 → 4). One short of critical:
/// still a warning, but verdict stays at `NeedsAttention`.
#[test]
fn verdict_boundary_orphans_just_below_threshold_is_needs_attention() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.orphans = (0..t.orphans - 1).map(|i| format!("NOTE-{i:03}")).collect();
    assert_eq!(r.orphans.len(), t.orphans - 1);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::NeedsAttention,
        "orphans = t-1 ({}) must stay NeedsAttention, got {v:?}",
        t.orphans - 1
    );
}

/// `orphans = t` (default 5). At limit: NOT yet promoted to Unhealthy
/// because the comparison is strict `>` — threshold value is the max
/// TOLERATED count, not the trip point.
#[test]
fn verdict_boundary_orphans_exactly_at_threshold_is_needs_attention() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.orphans = (0..t.orphans).map(|i| format!("NOTE-{i:03}")).collect();
    assert_eq!(r.orphans.len(), t.orphans);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::NeedsAttention,
        "orphans = t ({}) must stay NeedsAttention (strict `>` semantics), got {v:?}",
        t.orphans
    );
}

/// `orphans = t + 1` (default 6). Just past limit → critical class
/// fires → `Unhealthy`.
#[test]
fn verdict_boundary_orphans_just_above_threshold_is_unhealthy() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.orphans = (0..=t.orphans).map(|i| format!("NOTE-{i:03}")).collect();
    assert_eq!(r.orphans.len(), t.orphans + 1);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::Unhealthy,
        "orphans = t+1 ({}) must promote to Unhealthy, got {v:?}",
        t.orphans + 1
    );
}

// ── blind_spots ──────────────────────────────────────────────────────

#[test]
fn verdict_boundary_blind_spots_just_below_threshold_is_needs_attention() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.blind_spots = (0..t.blind_spots - 1)
        .map(|i| spot(&format!("ADR-{i:03}")))
        .collect();
    assert_eq!(r.blind_spots.len(), t.blind_spots - 1);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::NeedsAttention,
        "blind_spots = t-1 ({}) must stay NeedsAttention, got {v:?}",
        t.blind_spots - 1
    );
}

#[test]
fn verdict_boundary_blind_spots_exactly_at_threshold_is_needs_attention() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.blind_spots = (0..t.blind_spots)
        .map(|i| spot(&format!("ADR-{i:03}")))
        .collect();
    assert_eq!(r.blind_spots.len(), t.blind_spots);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::NeedsAttention,
        "blind_spots = t ({}) must stay NeedsAttention, got {v:?}",
        t.blind_spots
    );
}

#[test]
fn verdict_boundary_blind_spots_just_above_threshold_is_unhealthy() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.blind_spots = (0..=t.blind_spots)
        .map(|i| spot(&format!("ADR-{i:03}")))
        .collect();
    assert_eq!(r.blind_spots.len(), t.blind_spots + 1);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::Unhealthy,
        "blind_spots = t+1 ({}) must promote to Unhealthy, got {v:?}",
        t.blind_spots + 1
    );
}

// ── active_stubs ─────────────────────────────────────────────────────

#[test]
fn verdict_boundary_active_stubs_just_below_threshold_is_needs_attention() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.active_stubs = (0..t.active_stubs - 1)
        .map(|i| stub(&format!("PRD-{i:03}")))
        .collect();
    assert_eq!(r.active_stubs.len(), t.active_stubs - 1);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::NeedsAttention,
        "active_stubs = t-1 ({}) must stay NeedsAttention, got {v:?}",
        t.active_stubs - 1
    );
}

#[test]
fn verdict_boundary_active_stubs_exactly_at_threshold_is_needs_attention() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.active_stubs = (0..t.active_stubs)
        .map(|i| stub(&format!("PRD-{i:03}")))
        .collect();
    assert_eq!(r.active_stubs.len(), t.active_stubs);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::NeedsAttention,
        "active_stubs = t ({}) must stay NeedsAttention, got {v:?}",
        t.active_stubs
    );
}

#[test]
fn verdict_boundary_active_stubs_just_above_threshold_is_unhealthy() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.active_stubs = (0..=t.active_stubs)
        .map(|i| stub(&format!("PRD-{i:03}")))
        .collect();
    assert_eq!(r.active_stubs.len(), t.active_stubs + 1);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::Unhealthy,
        "active_stubs = t+1 ({}) must promote to Unhealthy, got {v:?}",
        t.active_stubs + 1
    );
}

// ── duplicates ───────────────────────────────────────────────────────

#[test]
fn verdict_boundary_duplicates_just_below_threshold_is_needs_attention() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.possible_duplicates = (0..t.duplicates - 1)
        .map(|i| dup(&format!("EVID-{i:03}"), &format!("EVID-{:03}", i + 100)))
        .collect();
    assert_eq!(r.possible_duplicates.len(), t.duplicates - 1);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::NeedsAttention,
        "duplicates = t-1 ({}) must stay NeedsAttention, got {v:?}",
        t.duplicates - 1
    );
}

#[test]
fn verdict_boundary_duplicates_exactly_at_threshold_is_needs_attention() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.possible_duplicates = (0..t.duplicates)
        .map(|i| dup(&format!("EVID-{i:03}"), &format!("EVID-{:03}", i + 100)))
        .collect();
    assert_eq!(r.possible_duplicates.len(), t.duplicates);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::NeedsAttention,
        "duplicates = t ({}) must stay NeedsAttention, got {v:?}",
        t.duplicates
    );
}

#[test]
fn verdict_boundary_duplicates_just_above_threshold_is_unhealthy() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.possible_duplicates = (0..=t.duplicates)
        .map(|i| dup(&format!("EVID-{i:03}"), &format!("EVID-{:03}", i + 100)))
        .collect();
    assert_eq!(r.possible_duplicates.len(), t.duplicates + 1);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::Unhealthy,
        "duplicates = t+1 ({}) must promote to Unhealthy, got {v:?}",
        t.duplicates + 1
    );
}

// ── at_risk (audit CR-002 — closes missing critical-class triplet) ──
//
// PROB-051 L-M1 promoted `at_risk` to a critical threshold class via
// `VerdictThresholds::at_risk` (default `DEFAULT_UNHEALTHY_AT_RISK =
// 10`). The original W3 test file omitted this triplet (docstring even
// flagged it as future work — audit CR-002 caught the divergence).

#[test]
fn verdict_boundary_at_risk_just_below_threshold_is_needs_attention() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.at_risk = (0..t.at_risk - 1)
        .map(|i| at_risk(&format!("PRD-{i:03}")))
        .collect();
    assert_eq!(r.at_risk.len(), t.at_risk - 1);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::NeedsAttention,
        "at_risk = t-1 ({}) must stay NeedsAttention, got {v:?}",
        t.at_risk - 1
    );
}

#[test]
fn verdict_boundary_at_risk_exactly_at_threshold_is_needs_attention() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.at_risk = (0..t.at_risk)
        .map(|i| at_risk(&format!("PRD-{i:03}")))
        .collect();
    assert_eq!(r.at_risk.len(), t.at_risk);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::NeedsAttention,
        "at_risk = t ({}) must stay NeedsAttention (strict `>` semantics), got {v:?}",
        t.at_risk
    );
}

#[test]
fn verdict_boundary_at_risk_just_above_threshold_is_unhealthy() {
    let mut r = baseline_report();
    let t = VerdictThresholds::default();
    r.at_risk = (0..=t.at_risk)
        .map(|i| at_risk(&format!("PRD-{i:03}")))
        .collect();
    assert_eq!(r.at_risk.len(), t.at_risk + 1);
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::Unhealthy,
        "at_risk = t+1 ({}) must promote to Unhealthy, got {v:?}",
        t.at_risk + 1
    );
}

// ── empty / zero-count sanity ────────────────────────────────────────

/// Sanity guard: zero counts on a populated workspace → `Healthy`.
/// Ensures the boundary test fixture itself is not silently broken
/// (e.g. accidentally setting `total = 0` would make every test pass
/// trivially via the `Verdict::Empty` short-circuit).
#[test]
fn verdict_boundary_zero_counts_on_populated_workspace_is_healthy() {
    let r = baseline_report();
    let v = r.compute_verdict_with(&VerdictThresholds::default(), 0);
    assert_eq!(
        v,
        Verdict::Healthy,
        "baseline report with zero signals must be Healthy, got {v:?}"
    );
}

/// Sanity guard: empty workspace (`total = 0`) short-circuits to
/// `Empty` even with non-zero orphans (which can't actually happen, but
/// asserting it pins the short-circuit ordering).
#[test]
fn verdict_boundary_empty_workspace_short_circuits_before_threshold_checks() {
    let mut r = baseline_report();
    r.total = 0;
    let t = VerdictThresholds::default();
    // Even with `orphans > t.orphans`, the empty-workspace path wins.
    r.orphans = (0..=t.orphans + 5).map(|i| format!("X-{i}")).collect();
    let v = r.compute_verdict_with(&t, 0);
    assert_eq!(
        v,
        Verdict::Empty,
        "total = 0 must short-circuit to Empty regardless of signal counts, got {v:?}"
    );
}
