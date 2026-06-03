---
depth: tactical
id: EVID-146
kind: evidence
links:
- target: PROB-078
  relation: informs
status: draft
title: 'PROB-078 refuted: MCP read-after-write correct across 7 tests incl real-binary subprocess'
---

## Summary

Reliable, deterministic verification that the PROB-078 report — "MCP
`forgeplan_update` then `forgeplan_get` in the same session returns the stale
(pre-update) body" — is **not a product bug**. Read-after-write is correct at
every layer, including the real `forgeplan-mcp` binary running as a separate OS
process over real stdio. The original "stale" observation was an artifact of the
hand-rolled **stdio-printf** repro client, not the product.

## Structured Fields

verdict: refutes
congruence_level: 3
evidence_type: test

CL3: the tests exercise the exact code paths and the exact real-binary scenario
the report describes — same context, highest congruence.

## Method — three layers, seven tests (all green)

Built on branch `fix/prob-mcp-stale-read` (commit `8aae19a`), run on macOS with
the real debug binaries.

1. **Store layer** (`forgeplan-core` unit test
   `db::store::tests::prob078_reopened_handle_sees_own_update_body`): a
   `LanceStore` handle opened at version V1 — the row created by a *separate*
   handle that is then dropped (simulating a prior CLI process) — observes its
   own in-place `update_body` on a subsequent `get_record` through the **same**
   handle. The test first asserts it reads the pre-update template body, then
   asserts it reads the marker after the update. **Refutes mechanism A**
   (the suspected "`update().execute()` does not advance the handle").

2. **MCP in-process** (`forgeplan-mcp` integration tests
   `prob078_read_after_write_repro.rs`, McpFixture, 3 variants):
   - `repro_seed_then_update_get_no_ws` — full handler stack, artifact seeded
     before the server opens its handle.
   - `repro_seed_then_update_get_with_ws` — same, with explicit `workspace=`.
   - `repro_get_first_then_update_then_get_with_ws` — **two-store probe**: a
     read with `workspace=` caches a Step-1 store handle *before* a write that
     lands via the default-store path; the second read still sees the update.
     This proves the param-path and default-path resolve the **same** cached
     `Arc<LanceStore>` (no split-brain) and that the read reflects the write.

3. **Real binary** (`forgeplan-mcp` integration tests
   `prob078_real_binary_subprocess_e2e.rs`, rmcp child-process transport whose
   wire framing is byte-identical to the server, 3 variants):
   - `real_binary_read_after_write_no_ws_param` — spawn the actual
     `forgeplan-mcp` over real stdio; `update -> get`; fresh.
   - `real_binary_read_after_write_with_ws_param` — same with `workspace=WS`
     on both calls (the shape the original repro used).
   - `real_binary_cli_create_then_mcp_update_get` — **byte-exact replica** of
     the original repro: the real `forgeplan` CLI creates a `NOTE` (separate
     process), then the real `forgeplan-mcp` binary does `update -> get` with
     `workspace=WS`. Fresh.

Gate: `cargo fmt` 0 diffs, `cargo clippy -D warnings` 0 warnings, 7/7 PASS.

## Root cause of the false positive

In LanceDB 0.27 `add()` (create) advances the in-memory `Table` snapshot, so a
row created by `add()` in the same session is visible immediately. The report
hypothesised that `update().execute()` does *not* advance the handle. The
store-layer test shows it **does** — a handle opened at V1 sees its own
committed UPDATE. The "stale" reading came from the unreliable stdio-printf
client (mis-framed JSON-RPC / mis-sequenced handshake / mis-read response), not
from LanceDB or the MCP handlers.

## Consequences

- **PROB-078 is not a release blocker.** v0.33 is unblocked on this axis.
- The proposed fix directions in PROB-078 (evict/re-open store after write;
  read body from the markdown file; lancedb upgrade) **must not** be
  implemented — they would add latency for no correctness benefit.
- **PROB-075 F-6 is vindicated.** PROB-078 claimed F-6's "deferred, test-only,
  not a correctness bug" closure (EVID-144) was wrong. This evidence shows the
  read path F-6 would exercise is correct, so EVID-144's assessment stands.
- The 7 tests are permanent regression guards and also replace the previously
  weak Journey-1 e2e assertion (`!is_empty`) with content-match checks.

## Honest scope note

Verified on macOS (same machine and same scenario as the original repro). The
mechanism (LanceDB handle snapshot semantics) is not OS-specific; the tests run
on Linux CI once the branch is pushed. Residual risk is low and, regardless,
carries no breaking-change surface (tests only; no product code changed).


