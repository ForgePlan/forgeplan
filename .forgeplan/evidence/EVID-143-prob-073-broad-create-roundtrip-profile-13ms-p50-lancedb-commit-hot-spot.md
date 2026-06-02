---
depth: tactical
id: EVID-143
kind: evidence
links:
- target: PROB-073
  relation: informs
status: draft
title: 'PROB-073 broad: create-roundtrip profile — 13ms p50, LanceDB commit hot spot'
---

## Summary

Measure-first profile of the `forgeplan_new` create-roundtrip — the path the user reports as slow ("медленно через MCP, file-first летает"). Decomposes the core create path to locate the hot spot BEFORE optimizing. Bench: `crates/forgeplan-core/tests/create_roundtrip_bench.rs`.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: benchmark

CL3: direct component measurement of the exact shipped create path.

## Measured (dev-profile, n=50/component)

| Component | p50 | p95 | Note |
|-----------|-----|-----|------|
| store init | 22ms (one-time) | — | cached per-workspace since PRD-078 → NOT per-call |
| **lance write (insert+commit)** | **7.1ms** | 10.5ms | **dominant** — manifest write / fsync per row |
| projection (render + file I/O) | 6.2ms | — | markdown render + file write |
| full create roundtrip | 13.3ms | 28.6ms | validate + projection + lance |

## Findings

1. **Hot spot = the per-write LanceDB commit (~7ms p50).** Each `add().execute()` does a manifest write + fsync. In an 11-agent pipeline issuing hundreds of `forgeplan_new`/`forgeplan_link` calls, this is what accumulates into "slow".
2. **Projection (~6ms) is the second half** — markdown render + file write, near-equal to the lance cost.
3. **Store-open is NOT the culprit** — 22ms but one-time, cached per-workspace by `workspace_store_cache` (PRD-078). Not paid per call.
4. **Plan risk #1 is CLEARED**: the slowness is a localized per-write commit cost, NOT a "rewrite ~20% of the server" situation. v0.33 can address the symptom; no v0.34 engine rewrite forced.

## Optimization candidates (surfaced, NOT applied)

The fix is a decision with a real trade-off — recorded here, not chosen blind:

- **(A) Batch / defer LanceDB commits** — accumulate N inserts, one commit. Biggest win on the dominant component, but changes durability semantics (a crash mid-batch loses the un-committed tail) and the per-workspace lock interaction. Best fit for pipeline bursts.
- **(B) Async / background commit** — return after the in-memory insert, fsync off the hot path. Faster perceived latency; same durability caveat + needs failure surfacing.
- **(C) Accept ~13ms** — the file-first workaround stays valid (ADR-003); document the per-call cost and the file-first+sync alternative for burst workloads. Zero risk, zero code.

Projection (~6ms) is harder to cut without touching the file-first invariant (ADR-003), so the lance-commit path is the higher-ROI target.

## Scope

The detection slice of PROB-073 was closed separately (EVID-141, SC-2 via gating). This evidence covers the **broad** create-roundtrip cost — the real user complaint.

## Provenance

- Bench: `crates/forgeplan-core/tests/create_roundtrip_bench.rs`
- Command: `cargo test -p forgeplan-core --features test-helpers --test create_roundtrip_bench -- --ignored --nocapture`
- Branch: `chore/prob-073-roundtrip-profile` (off `origin/dev`)


