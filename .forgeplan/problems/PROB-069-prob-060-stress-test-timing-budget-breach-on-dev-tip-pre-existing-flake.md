---
depth: standard
id: PROB-069
kind: problem
last_modified_at: 2026-05-12T19:54:26.788348+00:00
last_modified_by: claude-code/2.1.139
links:
- target: PROB-067
  relation: informs
status: active
title: prob_060_stress_test timing-budget breach on dev tip — pre-existing flake
---

# PROB-069: prob_060_stress_test timing-budget breach on dev tip

## Signal

`crates/forgeplan-cli/tests/prob_060_stress_test.rs::stress_test_property_loop_seeds` exhausts its 30-second budget on dev tip `6a6dce7`:

- Isolated run (no parallel load): 33.42s
- Full workspace parallel run: 43.26s
- Budget assertion at `prob_060_stress_test.rs:326` — panics with "stress test loop took 43.256687042s, budget is ≤30 s"

Both Wave 9 workers (W1 and W3) reported the flake during their pipelines. Verified independently on dev tip BEFORE any Wave 9 changes landed — confirms pre-existing, NOT introduced by Wave 9 audit closure work.

## Context

- Test ran clean during PROB-060 sprint authoring (2026-04-23..2026-05-08 timeframe)
- ID-allocation pipeline expanded in PROB-067 fix (cross-worktree per-kind lock + post-write collision check) — `forgeplan_core::artifact::id_alloc::allocate_and_create_artifact` is now serialized through `<git-common-dir>/forgeplan/id-<KIND>.lock`
- Each `forgeplan_new` call serially acquires lock → read counter → write file → release lock. With 11 candidate seeds + 10 file ops each, the lock-acquire latency now dominates
- Machine-dependent: PROB-067 author saw <30s on their machine; this hardware sees 33-43s

## Root cause hypothesis

PROB-067 lock granularity is per-kind (`id-evidence.lock`, `id-note.lock`, etc.) — already optimal granularity. The breach is timing-budget side, not logic side. Probable cause: per-test workspace setup includes `git init` + initial `.forgeplan/` scaffolding, costing 500ms-1s each.

## Decision options (acceptance criteria for closure)

**Option A (recommended)**: widen budget to ≤60s. Pure timing tweak. Trade-off: less sensitive to genuine regressions of ID allocation perf.

**Option B**: optimise ID-allocation hot path. Profile the lock-acquire + counter-read latency. Maybe cache common-dir resolution, batch file scans for the counter computation. Trade-off: real eng work, ~1-2h, perf gain may not solve env-dependent variance.

**Option C**: mark `#[ignore]` and run on a dedicated perf-only CI lane. Keeps the assertion but doesn't gate dev workflow. Trade-off: regression hides until someone re-enables.

**Option D**: split the stress test — keep a tight 5-seed run as fast-test, push 11-seed run to `--ignored`. Trade-off: still covers race in fast run; slow run for nightly verification.

Lean Option A for v0.32.0 quick fix, Option D as follow-up if Option A masks a future regression.

## Acceptance criteria

- [ ] One of A/B/C/D applied; test runs deterministically green in both isolated and parallel mode
- [ ] Regression coverage preserved (no `#[ignore]` permanently without Option D split)
- [ ] CHANGELOG entry under `### Internal`

## Reversibility

High — budget tweak or `#[ignore]` are 1-line reverts.

## Linked artifacts

- informs PROB-060 (parent — ID assignment marathon)
- informs PROB-067 (introduced the cross-worktree lock that may dominate latency)



