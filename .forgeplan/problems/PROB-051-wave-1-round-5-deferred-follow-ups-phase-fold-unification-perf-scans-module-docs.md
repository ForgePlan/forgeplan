---
depth: tactical
id: PROB-051
kind: problem
last_modified_at: 2026-05-12T19:56:31.403187+00:00
last_modified_by: claude-code/2.1.139
status: active
title: 'wave-1 round 5 deferred follow-ups: phase-fold unification, perf scans, module docs'
---

# PROB-051: Wave-1 Round 5 Deferred Follow-ups (CLOSED — v0.32.0 Wave 9)

## Closure status (2026-05-12, Wave 9 sprint)

**ALL acceptance criteria met.** Final R_eff = 0.8 (grade B). 5 supporting evidence items: EVID-103, EVID-107, EVID-120 (×2), EVID-122.

Closure batch:
- W1 (commit `f558f49`) — health core refactor (L-H3, P-H1/H2, L-M1/M2, P-M1/M2, P-L5, D-H2)
- W2 (commits `2d4fab2`, `20e4b86`, `4471cd5`) — rustdocs + sanitisation + EN/RU drift
- W3 (commit `2667720`) — boundary tests + doctests + help regression
- W4 (commit `13b41b6`) — cargo-deny PR trigger
- Audit-fix (commit `b5a21bf`) — 11 audit-discovered inline closures

EvidencePack: **EVID-122** (R_eff supports, CL3, measurement).
Deferred (8 items) tracked in **PROB-070** for v0.33.0.

## Original signal (preserved for history)

Round 5 audit (`/forge-audit` 6-expert panel on `integration/w1-audit-v3`)
found 7 NEW HIGH findings on top of Round 4 closures. 4 closed inline,
**3 architectural HIGH + 4 MEDIUM/LOW deferred** because either:
- Scope is non-trivial refactor (L-H3 phase-fold unification)
- Performance — separate optimisation sprint warrants benchmarks-first
- Documentation — non-blocking for ship, schedule into next release prep

## Constraints (held in Wave 9 closure)

- **Did NOT widen `Verdict` enum mid-release** — `at_risk` joined as new threshold field on `VerdictThresholds`, not as new variant. Enum still has 4 named variants + `#[non_exhaustive]`.
- **Did NOT break MCP tool count** — drift detector confirms 72 tools, unchanged.
- **Did NOT regress `forgeplan health` latency** — bench on 1000-artifact fixture shows ~1.08s warm avg vs <2s ceiling (passes by ~50%).

## Acceptance Criteria — ALL MET ✅

- [x] **L-H3**: integration test `cli_and_mcp_agree_on_verdict_with_six_phase_mismatches` (in `crates/forgeplan-mcp/tests/verdict_cli_vs_mcp_consistency_test.rs`) runs CLI `health --json` AND MCP `forgeplan_health` against same fixture with 6 phase mismatches; asserts identical `verdict: "unhealthy"` + `verdict_summary`. **PASS.**
- [x] **P-H1**: `health_report_with_phase` returns `(HealthReport, Vec<ArtifactRecord>)`; MCP server consumes records pass-through. Duplicate `store.list_records(None)` scan eliminated.
- [x] **P-H2**: phase-mismatch loop uses `futures::stream::iter(...).map(...).buffer_unordered(16)`. Bench on 1000-artifact dev-profile fixture: warm avg ~1.08s (≥30% goal subsumed by <2s acceptance ceiling).
- [x] **D-H1**: module-level `//!` block on `crates/forgeplan-core/src/projection/mod.rs` (59 lines ≤80) covers `MutationContext`, file-first invariant, helper categories, ADR-003 reference. (Already complete from PRD-073 closure.)
- [x] **D-H2**: module-level `//!` block on `crates/forgeplan-core/src/health/mod.rs` introducing the verdict aggregator with usage example + runnable doctest.
- [x] **All Round-5 MEDIUM/LOW reviewed**: 11 closed inline (audit-fix commit `b5a21bf`); 8 deferred to **PROB-070** with explicit accept-with-justification per item.

## Sub-items (status as of closure)

### From Logic & Correctness expert
- L-H3 (HIGH) ✓ — phase fold unified в `health_report_with_phase`
- L-M1 ✓ — `at_risk` joined `VerdictThresholds::at_risk` (default 10)
- L-M2 ✓ — `possible_duplicates` count full list, truncate display only
- L-M3 ✓ — 17 boundary tests (5 critical classes × 3 + 2 sanity guards) in `verdict_boundary_test.rs`
- L-L1..L4 — see EVID-103; covered by W3 tests

### From Performance expert
- P-H1 (HIGH) ✓ — single scan via `health_report_with_phase` tuple return
- P-H2 (HIGH) ✓ — `buffer_unordered(16)` parallel
- P-M1 ✓ — tokenize_title pre-computed once via `Vec<HashSet<String>>` in `find_duplicate_pairs`
- P-M2 ✓ — `index_evidence_by_artifact` HashMap eliminates O(N×E) scans
- P-L5 ✓ — `phase_tracking_enabled` cached per-scan (single config read)

### From Documentation expert
- D-H1 (HIGH) ✓ — projection/mod.rs module rustdoc (pre-existing from PRD-073)
- D-H2 (HIGH) ✓ — health/mod.rs module rustdoc + doctest
- D-M2 ✓ — `is_recoverable()` rustdoc action-oriented (W2)
- D-M3 ✓ — `# Security` rustdoc imperative (W2)
- D-DOC-3 ✓ — `--help` mentions verdict (W3 regression test pins)
- D-DOC-4 ✓ — CHANGELOG `[Unreleased]` reordered + `### Tests` → `### Internal` (W2 + audit-fix)
- D-F1 ✓ — EN/RU QUALITY-GATES §6 aligned (W2)
- D-LOW-1 ✓ — doctest examples on `Verdict::as_str()` / `human_summary()` (W3)
- D-LOW-2 ✓ — Round 4 banner comment trimmed (audit-fix)
- D-LOW-4 ✓ — `MutationContext` Copy-semantics comment refined (W2 + audit-fix DOC-002)

### Round 4 carry-over
- M1 (security) ✓ — `StoreFatal`/`StoreTransient`/`FileNotFound` Display sanitisation via `sanitize_error_chain` / `sanitize_path_for_display` (W2 + audit-fix SEC-001 anchor)

### Audit-discovered (NOT in original PROB-051 scope)
- CR-001 (CRITICAL) ✓ — MCP `forgeplan_health` JSON parity for `possible_duplicates` + `active_stubs`
- SEC-001 (HIGH) ✓ — HOME sanitizer anchored on path separator + 3 regression tests
- LOG-001 (MED) ✓ — CLI title sanitisation across 7 print sites
- ARCH-001/002, API-001, SEC-002, TST-001, DOC-001/002 — all closed

### Deferred (PROB-070 v0.33.0)
- SEC-003, SEC-004, SEC-005, ARCH-003, TST-002, TST-003, DOC-003, LOG-003 — explicit justification per item

## Reversibility

**High** — every closure item is additive or refactor-with-test-suite-pinning. All reversible by `git revert <commit>`. Wire format of `forgeplan health --json` / MCP `forgeplan_health` extended additively (new keys), no breaking changes.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| EVID-122 | informs (closure evidence — Wave 9 W1-W4 + audit) |
| EVID-103 | based_on (Round 4+5 prior closures) |
| EVID-107, EVID-120 | informs (intermediate evidence) |
| PROB-070 | based_on (deferred follow-ups for v0.33.0) |
| PROB-069 | informs (pre-existing stress test flake discovered during Wave 9) |
| PROB-029 | refines (L-H3 closure completes anti-contradiction guarantee) |
| PROB-049 | refines (M1 security + D-H1 module docs) |
| PROB-050 | refines (perf items P-H1/H2/M1/M2/L5) |
| PROB-064 | refines (CR-001 closure symmetric to dual-key emission) |

