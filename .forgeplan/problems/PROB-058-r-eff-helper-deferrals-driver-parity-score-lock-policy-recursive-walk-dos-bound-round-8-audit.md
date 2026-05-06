---
depth: standard
id: PROB-058
kind: problem
links:
- target: PROB-057
  relation: refines
status: active
title: R_eff helper deferrals — driver parity + score lock policy + recursive walk DoS bound (Round 8 audit)
---

# PROB-058: R_eff helper Round 8 audit deferrals

## Signal

PROB-057 / PRD-075 closure (Round 8 adversarial audit, 2 parallel agents — security + code-reviewer, 2026-05-06) flagged **3 HIGH-severity findings** that close-cleanly only with architectural changes outside the PRD-075 scope. PRD-075 shipped with the immediate stale-cache leak fixed, but these structural follow-ups stay open as PROB-058 to prevent silent decay:

1. **Driver parity (FR-006 vacuous)** — `sync_score_target(store: &LanceStore, ...)` is hardcoded to the concrete `LanceStore` rather than the `StorageDriver` trait (`crates/forgeplan-core/src/scoring/mod.rs:32`). PRD-075 FR-006 promised "behavior identical в LanceStore (production) и InMemoryStore (test-helpers)" but no `InMemoryStore` test exercises the helper because the trait has no method for it. The FR currently ships untested.
2. **`forgeplan score` / `score-all` operate without workspace lock** (`crates/forgeplan-cli/src/commands/score.rs:88`, `score.rs:21`) — they call `common::store()` (no lock) instead of `common::open_store_locked()`. Concurrent CLI invocations (operator running `score` while a multi-agent `link` runs in parallel — PRD-057 dispatch) can race on `update_r_eff_score` writes; later writer wins, earlier work lost. Pre-PROB-057 audit history (`audit 2026-05-01 H1`) confirmed similar concurrency hazard in `update PRD-001` for `update`; this is the same class.
3. **Recursive walk DoS bound** (`crates/forgeplan-core/src/scoring/reff.rs:227`) — `r_eff_recursive` walks the full transitive dependency closure on every mutator invocation through `sync_score_target`. On a pathological dense graph the workspace lock is held for the entire walk; a malicious or accidentally-recursive artifact graph can starve other CLI/MCP callers past the 30 s lock timeout. PRD-075 FR-005 set a 100 ms budget but the inline recompute path has no cap.

PROB-058 also tracks the **MED side-channel finding** (timing oracle on `forgeplan link` mutation latency — leaks graph topology) and the **LOW gaps** (no negative `hint_contract` test for FR-009 hint divergence; helper docstring overclaims scope). These are smaller but in scope for the same architectural follow-up.

## Constraints

- **MUST** preserve PROB-057 closure invariant — link/unlink/activate continue auto-recomputing the local target's cached `r_eff_score`.
- **MUST** preserve `forgeplan score` / `score-all` semantics — operators rely on these for batch reconciliation; behavior must not degrade.
- **MUST NOT** require a LanceDB schema migration without an explicit backwards-compat path (workspaces ниже current minor продолжают читаться).
- **MUST** preserve ADR-003 file-first invariant — markdown is source of truth, all mutations go through `projection::*` helpers.

## Optimization Targets

- **Driver parity**: `sync_score_target` callable through `StorageDriver` trait so `InMemoryStore` (test-helpers feature) tests cover the same code path as `LanceStore` production.
- **Concurrency safety**: `forgeplan score` / `score-all` acquire workspace lock — same parity as `link` / `activate`.
- **Bounded mutator latency**: split `r_eff_recursive` into `r_eff_local` (own evidence + L1 deps only) used by mutators, and full-recursive variant used by `score-all`. Cap inline recompute to documented depth.

## Observation Indicators (Anti-Goodhart)

- Total `r_eff_recursive` calls per session — measure но не оптимизировать вниз (legitimate batch use).
- Mutator latency p95 — monitor but не sub-50ms — natural variance OK.
- Lock contention events — should be **near zero** на single-operator workflows; spike indicates regression.

## Acceptance Criteria

- [ ] **AC-1**: `sync_score_target` re-typed against `StorageDriver` trait. **Status**: deferred — требует rework `r_eff_recursive` signature too (entire scoring pipeline currently `&LanceStore`-bound), separate sprint. Will be addressed when driver-trait scope is opened (RFC pending).
- [x] **AC-2**: `forgeplan score` (single id) and `score-all` switch from `common::store()` to `common::open_store_locked()`. **Closed** in PROB-057 sprint extension commit (2026-05-06). `score::run` and `score::run_all` now use `common::open_store_locked()`. **Round 9 audit MED-3 closure**: real concurrent-writer regression test added — `parallel_score_all_invocations_serialize_via_workspace_lock` in `cli_reff_cache_invalidation.rs` spawns two `forgeplan score --all` processes against the same workspace via `std::thread::spawn` + `Command::new`, asserts both succeed and the post-condition R_eff matches sequential expectation для 3 PRDs × 1 evidence each. Test exercises the OS-level fs2 advisory lock that `acquire_workspace_lock` wraps.
- [ ] **AC-3**: New `r_eff_local(id, store)` variant computes own_evidence + immediate deps only. **Status**: deferred — performance benchmark scaffold + variant function are non-trivial; current FR-005 budget held in practice (typical workspace ≤ 300 artifacts). Revisit when measurements show p95 > 100 ms. **Round 9 audit MED-1 caveat**: `score-all` now holds the workspace lock for the entire batch; on dense graphs (>200 artifacts × deep dep chains) this can starve concurrent CLI/MCP callers past the 30 s lock timeout. Closing AC-3 reduces the window incidentally; until then, operators are advised to schedule `score-all` outside batch-mutation windows.
- [x] **AC-4**: Negative `hint_contract` test asserts mutators do NOT emit `Next: forgeplan score <ID>` (FR-009 enforcement against drift). **Closed + Round 9 hardening** — `cli_reff_cache_invalidation.rs` now has 3 negative tests (`link_does_not_emit_per_target_score_hint`, `unlink_does_not_emit_per_target_score_hint`, `activate_does_not_emit_per_target_score_hint`) using line-shape match (`assert_reconcile_parents_hint_line` helper) instead of substring containment so concatenated drift `score-all && score <ID>` still trips the negative.
- [x] **AC-5**: Side-channel mitigation documented. **Closed** — PRD-075 §"Threat Model — Mutation Latency Side-Channel" describes mitigation posture + trigger-to-revisit conditions.
- [x] **AC-6**: `sync_score_target` docstring clarifies the actual scope. **Closed + Round 9 HIGH-3 fix** — docstring rewritten to distinguish three concerns: (1) evidence collection (in scope, **bidirectional** — outgoing AND incoming relations, since canonical link direction puts evidence sources pointing INTO their target), (2) dependency recursion (in scope, **descendant-only**), (3) transitive parent rescore (OUT of scope). Pre-Round-9 docstring conflated #1 with #2 by claiming "descendant only" for both, which contradicted the implementation at `reff.rs:252-253`.

## Round 9 Audit — MCP Transport Parity (HIGH-1)

Round 9 adversarial audit (2 parallel agents, 2026-05-06) flagged **HIGH-1 transport asymmetry**: PROB-057 closure landed CLI auto-recompute, but MCP `forgeplan_link` / `forgeplan_activate` / `forgeplan_score` still bypassed both the workspace lock and the `sync_score_target` helper. Multi-agent dispatch (PRD-057) routes through MCP, so the CLI fix alone left the production path exposed. Closed in the same sprint extension commit:

- **MCP `forgeplan_link`** (`crates/forgeplan-mcp/src/server.rs:1683`) now acquires `acquire_workspace_lock` before mutation, calls `sync_score_target` after `add_link_with_projection`, and emits `Next: forgeplan_score_all` instead of the pre-Round-9 `forgeplan_score {target}` per-target hint (FR-009 parity).
- **MCP `forgeplan_activate`** (`server.rs:2219`) now acquires the lock and calls `sync_score_target` BEFORE `render_after_mutation` so the rendered markdown reflects post-activation R_eff (mirroring CLI activate ordering — Round 8 MED-3 lesson).
- **MCP `forgeplan_score`** (`server.rs:1499`) — pre-Round-9 this tool computed `r_eff_recursive` for display but **never called `update_r_eff_score`**, so cached values stayed stale forever through the MCP transport. Now routes through `sync_score_target` to persist (with a graceful fallback if persist fails — display continues with a fresh recompute).

These three changes restore CLI/MCP parity for the PROB-057 closure invariant.

## Blast Radius

| Surface | File:line | Risk if unaddressed |
|---|---|---|
| Test harness driver parity | `crates/forgeplan-core/src/scoring/mod.rs:32` | FR-006 ships unverified; future driver swap regresses silently |
| `forgeplan score` concurrency | `crates/forgeplan-cli/src/commands/score.rs:88` | Multi-agent dispatch (PRD-057) races on R_eff writes; latest-writer-wins data loss |
| `forgeplan score-all` concurrency | `crates/forgeplan-cli/src/commands/score.rs:21` | Same as above при batch reconciliation |
| Mutator DoS via deep graph | `crates/forgeplan-core/src/scoring/reff.rs:227` | Hostile playbook with deep dependency chain blocks all mutations past lock timeout |
| Side-channel | mutator latency | Information disclosure в multi-tenant deployments (Forgeplan не targets currently но PRD-057 nears it) |

## Reversibility

**High** — all changes are non-schema. AC-1 driver parity = trait refactor (purely structural). AC-2 lock acquisition = 1-line change в `score.rs::run` / `run_all`. AC-3 `r_eff_local` = additive helper. Rollback through `git revert <commit>`.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-057 | refines (Round 8 audit findings deferred from PROB-057 closure) |
| PRD-075 | informs (PRD-075 FR-006 ships ничего не обеспечивая до закрытия PROB-058 AC-1) |
| PRD-057 | informs (multi-agent dispatch is the concrete consumer that needs the lock fix in AC-2) |
| ADR-003 | informs (file-first invariant — driver parity refactor must respect projection helpers) |
| PROB-049 | informs (typed errors umbrella — `MutationError` could subsume the new helper too) |

## Notes

This PROB intentionally batches **3 HIGH + 1 MED + 2 LOW** because they share the architectural surface (`sync_score_target` API + scoring entry points). Splitting per-finding would lose the coupling. ETA estimate: 4-6 h for AC-1 + AC-2 (single PR), separate 2-4 h for AC-3 (performance work). AC-4..6 can ride along in either.




