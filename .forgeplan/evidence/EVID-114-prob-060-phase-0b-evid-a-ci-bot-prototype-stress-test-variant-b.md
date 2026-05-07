---
depth: standard
id: EVID-114
kind: evidence
links:
- target: PROB-060
  relation: informs
- target: ADR-012
  relation: evidences
- target: PRD-076
  relation: informs
- target: RFC-009
  relation: informs
status: active
title: PROB-060 Phase 0b EVID-A CI bot prototype Variant B stress-test
---

# EVID-114: PROB-060 Phase 0b — EVID-A CI bot prototype + Variant B stress-test

## Summary

Closes the EVID-A reversal-gate from ADR-012 §Evidence Requirements via **Variant B (local in-process simulation)**. `forgeplan ci-assign-id` binary subcommand prototypes the CI ID-assignment logic; deterministic 10-PR stress-test integration test with seeded permutation models GitHub Actions `concurrency: forgeplan-id-assign, cancel-in-progress: false` serialization. Real-runtime concurrency proof (Variant A) is documented as a manual one-time gate before Phase 2 GA — runbook + helper script shipped, not yet executed.

## Structured Fields

verdict: supports
congruence_level: 2
evidence_type: test

## Methodology

**Variant B (covered by this evidence)**: sequential merging of 10 simulated PR branches in randomly permuted order, modeling what the GitHub Actions concurrency group does in production by design (it serializes parallel merges through the `forgeplan-id-assign` group). Test verifies binary's logic is correct under that serialization. In-process invocation of `ci_assign_id::run(...)` (per CD-2 binding contract — unit-of-test is the function, not a binary subprocess).

**Variant A (NOT covered, documented as future gate)**: real GitHub Actions runs of `assign-id.yml` with 10 simultaneous PRs. Captures real-runtime concurrency primitive behavior under multi-runner contention. Runbook in `docs/operations/EVID-A-real-stress-test.md` (Russian, 215 lines, 6 sections); helper script `scripts/stress-test-real-gh.sh` (idempotent, confirmation-gated, NOT executed). When Variant A runs before Phase 2 GA → upgrade this evidence to `congruence_level: 3` and ADR-012 R-1 reversal-gate fully closes.

## CL2 Framing — Honest Acknowledgment

Per L1 risk acceptance from team lead briefing: Variant B sequential-permutation models GH Actions concurrency-group serialization (which is what the production primitive does by spec) but does NOT exercise multi-runner contention. CL2 reflects that evidence is one degree removed from real environment — same kind of test as production behavior, but executed locally rather than in real CI. R_eff weighting honestly reflects this.

## Evidence — Test Results

Branch: `feat/prob-060-phase-0b-integration` @ `8bd66b6`
File: `crates/forgeplan-cli/tests/prob_060_stress_test.rs`
Fixture: `crates/forgeplan-cli/tests/fixtures/prob_060_stress/` (1 base + 10 PR branches × 1 artifact each)

**Test runs (default `cargo test --workspace --test prob_060_stress_test`)**:
- `seeded_permutation_is_deterministic_and_complete` — verifies StdRng seed produces deterministic permutation of 10 PRs, all distinct
- `stress_test_single_seed_zero` — single seed (0) full integration: git init temp repo, import fixture, create 10 branches, merge in seeded permutation, assert all 10 final assignments unique sequential 74..83, no nulls
- `stress_test_property_loop_seeds` — 12 git-backed seeds (0..11), each running full git integration
- `property_loop_in_process` — 100 in-process permutations through `compute_assignment_plan` directly (no git layer), verifies deterministic-ordering invariant

**Wall-time on M1**: 21.53s total for 4 tests (≤ 30s budget per CD-2)
- Single seed: ~1.8s (10 git merges)
- 12-seed git-backed loop: ~21s
- 100-seed in-process loop: <1s

**Asserts (per seed)**:
1. All 10 artifacts end with unique non-null `assigned_number` in dev
2. Per kind, assigned_numbers are 74..83 (sequential after baseline 73), no gaps, no duplicates
3. No file in dev has `assigned_number: null` after run
4. Same `{74..83}` set across all 112 seed combos (12 git + 100 in-process)

**Result**: 4/4 tests pass on integration branch HEAD. 0 race conditions detected in any seed permutation.

## CD-2 Deviation — Split Coverage Rationale (R_eff=0.85 ADI)

CD-2 originally specified «100 git-backed seeds in ≤30s wall-time». Empirical measurement showed 1.8s per git seed on M1 → 100 seeds = 180s, infeasible within budget. Worker 1 split to 12 git-backed (full integration coverage) + 100 in-process (logic invariant coverage). Both layers assert the same `{74..83}` invariant. ADI F-G-R analysis: F=1.0 (already implemented) × G=0.85 (CD-2 spirit met) × R=0.85 (both layers tested) × CL3 = R_eff 0.85. Accepted by parent.

## Variant A Future Upgrade Path

Pre-Phase-2-GA gate for `congruence_level: 3` upgrade:
1. User runs `bash scripts/stress-test-real-gh.sh` against real `ForgePlan/forgeplan` repo (after confirmation prompt)
2. Script creates 10 `prob-060-stress-*` branches simultaneously, labels all 10 PRs with `ready-to-merge` to fan out concurrent workflow triggers
3. Polls `gh run list` until all 10 complete, asserts no two ended with same `assigned_number`
4. Script cleans up branches + closes PRs at end
5. On pass: append real-runtime measurements to this evidence pack, bump `congruence_level: 3`, document in CHANGELOG entry

Reference: `docs/operations/EVID-A-real-stress-test.md` for full runbook.

## Cross-Reference

- ADR-012 §Evidence Requirements → EVID-A (this evidence closes that gate)
- ADR-012 §Risks → R-1 (concurrency primitive serialization claim)
- ADR-012 §Rollback Plan (this evidence does NOT trigger rollback; Variant A would if it fails)
- RFC-009 §Phase 0b (methodological remediation)
- PROB-060 (the original problem this evidence supports)
- CLAUDE.md feedback `feedback_evidence_structured_fields.md` (structured fields requirement)

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-060 | informs |
| ADR-012 | evidences |
| PRD-076 | informs |
| RFC-009 | informs |
