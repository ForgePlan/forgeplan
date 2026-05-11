---
depth: tactical
id: PROB-051
kind: problem
status: active
title: 'wave-1 round 5 deferred follow-ups: phase-fold unification, perf scans, module docs'
---

# PROB-051: Wave-1 Round 5 Deferred Follow-ups (v0.30.0 sprint candidate)

## Signal

Round 5 audit (`/forge-audit` 6-expert panel on `integration/w1-audit-v3`)
found 7 NEW HIGH findings on top of Round 4 closures. 4 closed inline,
**3 architectural HIGH + 4 MEDIUM/LOW deferred** because either:
- Scope is non-trivial refactor (L-H3 phase-fold unification)
- Performance — separate optimisation sprint warrants benchmarks-first
- Documentation — non-blocking for ship, schedule into next release prep

Tracking these as a single PROB so they don't get lost in the v0.29.0
release and so the v0.30.0 sprint planner has one anchor.

## Constraints

- **MUST NOT widen `Verdict` enum mid-release** — adding `Verdict::Uninitialized`
  as a 4th `#[non_exhaustive]` variant requires care: external library
  consumers of `forgeplan-core` need migration guidance and the rendering
  paths (CLI banner, MCP description) need updates. Defer the enum widening
  to v0.30.0 even though L-H2 closure recommended it.
- **MUST NOT break MCP tool count** — phase-fold unification (L-H3) refactor
  adds no new MCP tool (still 63), drift detector must remain green.
- **MUST NOT regress `forgeplan health` latency** below current 320ms warm
  baseline on 275-artifact workspace — perf fixes (P-H1, P-H2) are the
  primary acceptance criterion of perf items.

## Optimization Targets (1-3 макс)

1. **L-H3 closure**: same workspace MUST return identical `verdict` from
   CLI `--json` and MCP `forgeplan_health` regardless of phase tracking.
   Move phase-mismatch detection from `forgeplan-mcp/src/server.rs:2582-2606`
   into `forgeplan_core::health` so both surfaces fold identically.
2. **P-H1 + P-H2 closure**: MCP `forgeplan_health` warm latency on
   1000-artifact workspace MUST drop ≥30% vs current implementation.
   Approach: (a) eliminate the second `store.list_records(None)` scan by
   returning records from `health_report`; (b) parallelise `read_phase`
   per active artifact via `futures::stream::buffer_unordered(16)`.
3. **D-H1 + D-H2 closure**: `cargo doc --no-deps -p forgeplan-core` landing
   pages for `projection` and `health` modules MUST carry narrative
   doc-comments introducing `MutationContext` / `Verdict` to first-time
   readers — discoverability fix.

## Observation Indicators (Anti-Goodhart)

- Test count (1974 today) — should NOT drop. New tests welcome but
  refactor must not silently delete coverage.
- Drift detector (currently 0 drift) — must stay 0.
- `cargo bench` (if added) — track regressions, but don't optimise to
  the bench numbers; correctness > speed.

## Acceptance Criteria

- [ ] **L-H3**: integration test `verdict_consistent_cli_vs_mcp_with_phase_mismatches`
      that runs `forgeplan health --json` AND a synthesised
      `forgeplan_health` MCP call on the same fixture workspace with
      6 phase mismatches; asserts both return `verdict: "unhealthy"` and
      identical `verdict_summary`.
- [ ] **P-H1**: `health_report` returns `(HealthReport, Vec<ArtifactRecord>)`
      OR exposes a sibling helper `health_report_with_records`; MCP server
      consumes the records pass-through to drop the duplicate scan.
- [ ] **P-H2**: phase-mismatch loop uses `buffer_unordered(16)` or similar;
      benchmark on 200-active-artifact fixture drops latency ≥30%.
- [ ] **D-H1**: module-level `//!` block on
      `crates/forgeplan-core/src/projection/mod.rs` introducing
      `MutationContext`, the file-first invariant, and the helper categories.
- [ ] **D-H2**: module-level `//!` block on
      `crates/forgeplan-core/src/health/mod.rs` introducing the verdict
      aggregator with usage example.
- [ ] All Round-5 MEDIUM/LOW (8 + 6 items in EVID-103) reviewed and
      either closed or explicitly accepted-with-justification.

## Sub-items (deferred from Round 5)

### From Logic & Correctness expert
- L-H3 (HIGH): Same workspace different verdict CLI vs MCP — phase fold asymmetry.
- L-M1: `at_risk` count not in `VerdictThresholds` — promotion gap.
- L-M2: `possible_duplicates` truncated to 10 BEFORE verdict count.
- L-M3: No boundary tests at exact threshold.
- L-L1..L4: see EVID-103 detail.

### From Performance expert
- P-H1 (HIGH): Twice-per-call artifacts table scan in `forgeplan_health` MCP path.
- P-H2 (HIGH): Sequential `read_phase` per active artifact (no concurrency).
- P-M1: `find_duplicate_pairs` re-tokenises titles per (i,j) pair.
- P-M2: O(N×E) scans in `find_at_risk` and `compute_derived_status_breakdown`.
- P-L5: `phase_tracking_enabled` re-reads config.yaml on every MCP call.

### From Documentation expert
- D-H1 (HIGH): `projection/mod.rs` zero module-level rustdoc.
- D-H2 (HIGH): `health/mod.rs` zero module-level rustdoc.
- D-M2: `is_recoverable()` rustdoc not action-oriented.
- D-M3: `forgeplan health --help` doesn't mention `verdict` field.
- D-M3 (mislabel duplicate; see EVID-103 D-M3 vs DOC-3): `# Security` rustdoc on
  `from_store_err` is descriptive, not imperative — restructure as imperative.
- D-DOC-3: `forgeplan health --help` doesn't mention `--json verdict`.
- D-DOC-4: CHANGELOG `[Unreleased]` ordering doesn't match Keep-a-Changelog
  (Added before Changed before Security).
- D-F1: EN/RU drift in `QUALITY-GATES` section 6 (PR-F's Round 4 rephrase
  landed in RU only — EN still carries concrete numbers vulnerable to
  drift detector).
- D-LOW-1: `Verdict::as_str()` and `human_summary()` lack rustdoc doctest examples.
- D-LOW-2: `health.rs` Round 4 banner-rendering comment block is mostly history.
- D-LOW-4: `MutationContext` `Copy` semantics may surprise external embedders.

### From Round 4 (deferred earlier, still open)
- M1 (security): `StoreFatal/StoreTransient` Display path leak full sanitisation
  (only `# Security` rustdoc shipped in Round 4).

## Blast Radius

- Logic L-H3 fix touches `forgeplan-core/src/health/mod.rs` (export shape) +
  CLI `commands/health.rs` (consumer) + MCP `server.rs::forgeplan_health`
  (consumer). All file-disjoint from PR-D's projection refactor.
- Perf fixes touch the same files as L-H3 — should land in the same PR
  for review coherence.
- Doc fixes are file-disjoint from code fixes; can land in parallel.

## Reversibility

**High** — every item is additive or refactor-with-test-suite-pinning.
None of the deferred items requires changing on-disk artifact format,
LanceDB schema, or breaking the wire format of `forgeplan health --json`
/ MCP `forgeplan_health` (the new `verdict` / `verdict_summary` fields
just become more accurate). All reversible by `git revert <commit>`.

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| EVID-103 | based_on (Wave-1 Round 4+5 audit closures evidence) |
| PROB-029 | refines (L-H3 closure completes PROB-029 cross-surface guarantee) |
| PROB-049 | refines (M1 security follow-up + D-H1 module docs) |
| PROB-050 | refines (perf items — pre-existing under PROB-050 A-31 + new) |







