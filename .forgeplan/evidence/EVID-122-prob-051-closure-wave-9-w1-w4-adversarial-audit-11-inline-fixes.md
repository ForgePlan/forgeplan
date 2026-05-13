---
depth: standard
id: EVID-122
kind: evidence
last_modified_at: 2026-05-12T19:55:27.715622+00:00
last_modified_by: claude-code/2.1.139
links:
- target: PROB-051
  relation: informs
status: active
title: PROB-051 closure — Wave 9 W1-W4 + adversarial audit + 11 inline fixes
---

# EVID-122: PROB-051 closure — Wave 9 W1-W4 + adversarial audit + 11 inline fixes

## Summary

PROB-051 (Wave-1 Round 5 deferred follow-ups) closed end-to-end via Wave 9 sprint:

**Workers (4 parallel, file-disjoint via worktrees)**:
- W1 (`rust-pro`) — health core refactor: L-H3 phase-fold unification + P-H1/P-H2 perf (single-scan + parallel `buffer_unordered(16)`) + L-M1 at_risk threshold + L-M2 truncation semantics + P-M1 pre-tokenize titles + P-M2 evidence indexing + P-L5 config cache + D-H2 module rustdoc with runnable doctest. Commit `f558f49`.
- W2 (`coder`) — projection rustdoc D-H1 + error.rs M1 Display sanitisation + CHANGELOG D-DOC-4 + EN/RU drift D-F1. Commits `2d4fab2`, `20e4b86`, `4471cd5`.
- W3 (`tester`) — boundary tests L-M3 (14 tests) + doctest examples D-LOW-1 + `--help` regression D-DOC-3 (2 tests). Commit `2667720`.
- W4 (`deployment-engineer`) — cargo-deny PR trigger re-enabled. Commit `13b41b6`.

**Adversarial audit (2 parallel auditors)**:
- Security-expert — 7 findings (1 HIGH SEC-001 prefix-overlap; 4 MED SEC-002/003/004/005 + LOG-001 terminal injection; 2 LOW LOG-002/003)
- Code-reviewer — 12 findings (1 CRITICAL CR-001 MCP parity; 1 HIGH CR-002 boundary docstring; 4 MED API/ARCH/TST; 6 LOW DOC/TST)

**Inline fixes (11 of 19 — commit `b5a21bf`)**:
- CR-001 CRITICAL: MCP `forgeplan_health` JSON now emits `possible_duplicates` + `active_stubs` (CLI parity)
- CR-002 HIGH: at_risk boundary triplet (3 tests) + docstring corrected
- SEC-001 HIGH: HOME sanitizer anchored on path separator + 3 regression tests (sibling-no-clobber, anchor + bare-HOME, HOME=/ guard)
- LOG-001 MED: 7 print sites in CLI health sanitised via `sanitize_for_hint`
- ARCH-002 MED: `Path::parent()` empty-string fallback fixed
- API-001 MED: `tokenize_title` + `jaccard_similarity` demoted to `pub(crate)`
- SEC-002 MED: doc-impl drift rule #4 removed
- ARCH-001 LOW: no-op `let _ = (...)` wrapper deleted
- TST-001 LOW: tautological doctest replaced with meaningful assertion
- DOC-001 LOW: CHANGELOG `### Tests` → `### Internal`
- DOC-002 LOW: Copy-rationale contradiction resolved
- D-LOW-2 (W2 skip carry-over): Round 4 history block trimmed

**Deferred to v0.33.0 (PROB-070)**: 8 items — SEC-003 Windows bypass, SEC-004 CI policy, SEC-005 SHA-pin, ARCH-003 partial_verdict JSON, TST-002 help-text coupling, TST-003 multi-point bench, DOC-003 strict_exit_code, LOG-003 silent error swallow.

## Method

**Pipeline gates** (all green on integration branch `feat/v032-w9-integration`):

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | 0 diff |
| `cargo check --workspace --all-targets` | 0 warnings |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| `cargo test --workspace --lib` | 2034 PASS (251 + 1691 + 92), 0 fail |
| `cargo test --test verdict_boundary_test` | 17 PASS (was 14, +3 at_risk) |
| `cargo test --test verdict_cli_vs_mcp_consistency_test` | 1 PASS (L-H3 parity acceptance) |
| `cargo test --test health_help_test` | 2 PASS (D-DOC-3 regression) |
| `cargo doc --no-deps -p forgeplan-core` | exit 0; 52 pre-existing warnings in unrelated modules; 0 new |
| `bash scripts/smoke-test.sh` | 18+ ops PASS |
| `bash scripts/check-mcp-tool-count.sh` | 72 tools, no drift |

**Performance** (W1 bench `health_bench.rs`, 1000-artifact dev-profile fixture):
- Seed phase: 106s (one-time)
- Cold `health_report_with_phase`: ~815ms
- Warm avg over 3 runs: ~1.08s (200 active, 120 phase mismatches)
- Acceptance ceiling: <2s — passes by ~50%

Pre-PROB-051 baseline (double-scan + serial phase reads) cannot be directly measured because the regression was already structurally eliminated in the new `health_report_with_phase` API. Bench serves as forward regression guard.

**Pre-existing flake observed** (NOT introduced by Wave 9): `prob_060_stress_test::stress_test_property_loop_seeds` exceeds 30s budget (33-43s on this hardware). Filed as PROB-069 for separate closure.

## Findings

### Acceptance criteria status (from PROB-051)

- [x] **L-H3**: integration test `cli_and_mcp_agree_on_verdict_with_six_phase_mismatches` runs CLI `health --json` + MCP `forgeplan_health` on same fixture, asserts identical `verdict` + `verdict_summary`.
- [x] **P-H1**: `health_report` now returns `(HealthReport, Vec<ArtifactRecord>)` via `health_report_with_phase` — MCP server consumes records, single scan.
- [x] **P-H2**: phase-mismatch loop uses `buffer_unordered(16)` (W1 commit `f558f49`).
- [x] **D-H1**: `crates/forgeplan-core/src/projection/mod.rs` `//!` block already present from PRD-073 closure — covers MutationContext, file-first invariant, helper categories, ADR-003 reference (59 lines ≤80).
- [x] **D-H2**: `crates/forgeplan-core/src/health/mod.rs` module-level `//!` block landed with verdict aggregator usage example + runnable doctest.
- [x] **L-M3**: 17 boundary tests (5 critical classes × 3 boundary points + 2 sanity guards) — `verdict_boundary_test.rs`.
- [x] **All Round-5 MEDIUM/LOW reviewed**: 11 closed inline (commit `b5a21bf`); 8 deferred to PROB-070 with explicit justification.

### Audit-discovered improvements (NOT in original PROB-051 scope)

- **CR-001 CRITICAL**: MCP/CLI JSON parity violation predated PROB-051 but only surfaced during cross-surface audit — closed
- **SEC-001 HIGH**: M1 sanitisation had a partial-match bug (sibling-user prefix overlap) — closed with 3 regression tests
- **LOG-001 MED**: terminal injection via artifact titles in 7 CLI print sites — closed via `sanitize_for_hint`

### Anti-Goodhart observations

- Test count NOT silently regressed: 1965 → 2034 lib PASS (+69 net = +32 worker + +6 audit-fix + +31 from other W2/W3 work)
- Drift detector: 72 MCP tools — unchanged
- `cargo bench` not added as gate (still informal); criterion-style perf regression guard left for v0.33.0 multi-point bench follow-up
- All inline fixes have associated tests OR rustdoc justification — no silent acceptance

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement

## Pipeline gate summary

| Gate | Pre-Wave-9 baseline (dev `6a6dce7`) | Post-Wave-9 + audit-fix |
|---|---|---|
| Lib tests | 1965 | 2034 (+69) |
| Integration tests (boundary + parity + help) | 0 | 20 |
| MCP tool count | 72 | 72 (unchanged) |
| Pre-existing flakes | 1 (`prob_060_stress_test`) | 1 (same — filed as PROB-069) |
| Audit findings (Round 5 deferred) | 14 (PROB-051) | 8 (deferred to PROB-070) |

## Linked artifacts

- informs PROB-051 (closure evidence)
- based_on EVID-103 (Wave 1 Round 4+5 prior closure context)
- informs PROB-029 (anti-contradiction cross-surface guarantee — L-H3 closure)
- informs PROB-049 (typed-error follow-up — M1 sanitisation)
- informs PROB-050 (perf items P-H1/H2/M1/M2/L5)
- informs PROB-064 (dual-key emission cross-surface — symmetric closure for CR-001)
- informs PROB-070 (deferred audit findings v0.33.0 — exit door for 8 items)
- informs PROB-069 (pre-existing stress test flake — discovered during Wave 9)

## Sprint metrics

- 4 worker worktrees, 0 merge conflicts on integration (file-disjoint partitioning)
- 6 commits total (4 worker + 1 audit fix) + 4 merge commits
- ~1960 LOC delta net
- 2 adversarial auditors, 19 findings → 11 inline / 8 deferred
- Wall time: ~5h workers + ~1h audit + ~1h audit-fix



