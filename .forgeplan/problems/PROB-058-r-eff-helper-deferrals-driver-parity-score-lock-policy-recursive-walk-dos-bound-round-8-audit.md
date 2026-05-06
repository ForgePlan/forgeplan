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

- [ ] **AC-1**: `sync_score_target` re-typed against `StorageDriver` trait (or split into trait-bound + LanceStore convenience wrapper). InMemoryStore test exercises the helper end-to-end.
- [ ] **AC-2**: `forgeplan score` (single id) and `score-all` switch from `common::store()` to `common::open_store_locked()`. Regression test: two concurrent `score-all` invocations in parallel via `tokio::join!` — second blocks until first completes; final state matches sequential execution.
- [ ] **AC-3**: New `r_eff_local(id, store)` variant computes own_evidence + immediate deps only (no recursion). `sync_score_target` (mutator path) calls `r_eff_local`. `score-all` continues calling `r_eff_recursive`. Performance test: mutator p95 < 50 ms на synthetic graph depth 50, fanout 200.
- [ ] **AC-4**: Negative `hint_contract` test asserts `forgeplan link <args>` output does NOT contain `Next: forgeplan score <ID>` (FR-009 enforcement against drift).
- [ ] **AC-5**: Side-channel mitigation documented — either constant-time wrapper around mutator recompute (likely infeasible without major refactor) OR explicit `## Out of Scope` paragraph in PRD-075 / PROB-058 acknowledging the threat model.
- [ ] **AC-6**: `sync_score_target` docstring clarifies the actual scope (recursive descent for own assurance report, но no parent ascent) — closes Round 8 LOW-2.

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



