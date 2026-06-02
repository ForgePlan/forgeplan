---
depth: tactical
id: EVID-141
kind: evidence
links:
- target: PROB-073
  relation: informs
- target: PRD-078
  relation: informs
- target: ADR-015
  relation: informs
status: draft
title: 'PROB-073/SC-2: detect_multi_worktree latency bench — p95 ~12-13ms steady, WEAKENS but gated to cold-start (not per-call)'
---

## Summary

Latency bench for the PRD-078 multi-worktree detection gate (`detect_multi_worktree` = 2× `git rev-parse` + 2× `canonicalize`), addressing PRD-078 **SC-2 / NFR-001** (`<5ms p95`) and the narrow detection slice of **PROB-073**. Bench: `crates/forgeplan-core/tests/workspace_detection_bench.rs` (health_bench.rs convention — `Instant`, `#[ignore]`, no criterion).

## Structured Fields

verdict: weakens
congruence_level: 3
evidence_type: benchmark

CL3: direct measurement of the exact shipped `detect_multi_worktree`. `verdict: weakens` per ADR-015 §Evidence Requirements (5ms ≤ p95 < 50ms → caching candidate) — the detection cost exceeds the 5ms NFR-001 budget, but the gating analysis below shows it is NOT on the per-call hot path, so SC-2 is satisfied *in practice*.

## Measured numbers

`cargo test -p forgeplan-core --test workspace_detection_bench -- --ignored --nocapture` (dev-profile, macOS, N=100/branch):

| Run | Branch | p50 | p95 | max | Verdict |
|-----|--------|-----|-----|-----|---------|
| 1 (machine under load, disk 99%) | linked (detect=true) | 21.3ms | 35.3ms | 43.6ms | weakens |
| 1 | main (detect=false) | 19.1ms | 60.0ms | 101.5ms | refutes |
| 2 (machine settled) | linked (detect=true) | 12.4ms | 13.4ms | 16.0ms | **weakens** |
| 2 | main (detect=false) | 12.1ms | 13.2ms | 14.0ms | **weakens** |

**Steady-state p95 ≈ 12–13ms** (run 1's 35–60ms was machine-load noise). The floor is fundamental: two `git rev-parse` subprocess spawns cost ~6ms each on macOS; no amount of tuning brings 2 subprocess spawns under 5ms.

## Interpretation — SC-2 satisfied via gating, not via speed

The literal NFR-001 target (`detection <5ms p95`) is **NOT met** — detection costs ~12ms. **But the detection gate is cold-start-only**, verified in `server.rs::resolve_workspace_core`:

1. `params.workspace` present (Step 1) → return, **no detection**.
2. `FORGEPLAN_WORKSPACE` env present (Step 2) → return, **no detection**.
3. `default_workspace` set (server `forgeplan_init`'d) → return, **no detection**.
4. Only when **all three are absent** (cold-start, no init, no param/env) does the gate run `detect_multi_worktree`.

In any normal agentic pipeline the agent either passes `workspace=` (the H1 primary fix) or the server is initialized → detection never runs. It fires only on the cold-start-multi-worktree-no-param path, which is a **one-time error path** (the agent then retries with an explicit `workspace`). So NFR-001's "per tool call <5ms" holds in practice: per-call detection overhead in the hot path is **0ms**; the ~12ms is paid once, on the error path that tells the agent to pass `workspace`.

## Correction to EVID-140

EVID-140 stated detection was "expected well under 5ms, but unmeasured". **That estimate was wrong** — measured p95 is ~12ms, not <5ms. This bench supersedes that assumption. The conclusion (SC-2 acceptable) still holds, but for a *different, now-measured* reason: detection is gated off the hot path, not fast.

## Scope carve-out — broad PROB-073 remains open

PROB-073's actual user complaint ("медленно через MCP, file-first летает") is the **full `forgeplan_new`/`forgeplan_link` MCP roundtrip** — LanceDB open/write/commit, projection, possibly embedding — measured by the user in *seconds*, not the ~12ms detection. Detection is a small, gated component and is **not** the PROB-073 bottleneck. The broad LanceDB-roundtrip profiling is a separate, larger track and is **NOT closed** by this evidence.

## Decision / follow-up

- **SC-2 (PRD-078)**: closed as satisfied-via-gating (per-call hot path pays 0ms detection). No session-cache added — it would optimize only the rare one-time cold-start path (low ROL), and the FS-shortcut alternative risks reintroducing the ARCH-3 submodule false-positive just fixed.
- **PROB-073 (broad)**: stays open; next track = profile the LanceDB roundtrip on `forgeplan_new`/`link`.

## Provenance

- Bench file: `crates/forgeplan-core/tests/workspace_detection_bench.rs`
- Command: `cargo test -p forgeplan-core --test workspace_detection_bench -- --ignored --nocapture`
- Branch: `feat/prob-073-detection-bench` (off `origin/dev` @ PRD-078 merged)
- Gate at commit: clippy 0 (`--all-targets --features test-helpers`), fmt 0, bench green.




