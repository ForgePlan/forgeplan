---
depth: tactical
id: EVID-129
kind: evidence
links:
- target: PROB-069
  relation: informs
status: active
title: PROB-069 flaky stress test budget calibration — fixed via audit-r6 closure in v0.32.0
---

# EVID-129: PROB-069 flaky stress test budget calibration — fixed via audit-r6 closure in v0.32.0

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement

## Observation

PROB-069 was filed during the v0.32 cycle when `cargo test --workspace` started
flaking in CI on the `prob_060_stress_test` integration test. The test runs a
50-worktree stress simulation with timing assertions; pre-fix budgets assumed
unrealistic synchronous behaviour and produced false negatives ~15% of the
time on slower CI runners.

## Evidence

- Audit-r6 (commit `6efde4b`) recalibrated the budget — see `fix(audit-r6):
  close ARCH-H1 consolidation + PROB-069 flaky budget`.
- Specific fix: `crates/forgeplan-cli/tests/prob_060_stress_test.rs` —
  `MAX_RUNTIME_SECS` raised from 30 to 60, `PER_WORKER_SLACK_MS` introduced as
  configurable parameter (defaults preserve historical behaviour for fast
  runners).
- Post-fix test runs: 100 consecutive CI invocations, 0 spurious failures
  observed. Local laptop runs (M1 Max, 16-core) — 50 consecutive, 0 failures.
- Test still surfaces real perf regressions: introduced an artificial 100ms
  delay in `merge_in_order_and_assign` as a smoke check, test correctly failed
  with the expected timing violation diagnostic.

## Result

PROB-069 status: resolved. Stress test reliable at v0.32.0 (audit-r6 commit
landed in feat/issues-286-288-289 branch). Test still catches real regressions
without flaking on infrastructure variance.



