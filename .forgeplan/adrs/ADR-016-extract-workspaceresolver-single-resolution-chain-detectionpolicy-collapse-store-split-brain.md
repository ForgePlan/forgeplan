---
depth: standard
id: ADR-016
kind: adr
links:
- target: PROB-076
  relation: based_on
- target: ADR-015
  relation: refines
- target: PRD-078
  relation: informs
- target: EVID-139
  relation: informs
- target: PROB-072
  relation: informs
- target: PROB-067
  relation: informs
status: active
title: Extract WorkspaceResolver — single resolution chain + DetectionPolicy, collapse store split-brain
---

## Context

PRD-078 made MCP mutating tools worktree-aware via `resolve_workspace`. Closing audit finding HIGH-2 (read tools must be worktree-aware too, else the Guardian loop PROB-072 reopens on the read side) a sub-agent implemented `resolve_workspace_read` by **duplicating ~90 lines** of the resolution chain, differing only in one behaviour: reads skip the multi-worktree detection error-gate (soft fallback), writes error (strict). EVID-139 separately flagged a "split-brain": two parallel store-handle systems — legacy `self.store`/`self.workspace_path` (reads + `forgeplan_init`) and `workspace_store_cache` (writes via `resolve_workspace`).

Decided via ADI (`forgeplan reason PROB-076`, gemini-3.1-pro-preview, 2026-05-30) + two evidence probes. Refines ADR-015 (which fixed the *policy*: strict-error vs soft; H1 param primary). This ADR fixes the *structure*.

## Decision

**Selected**: Extract a `WorkspaceResolver` type that owns the resolution chain + store cache, parameterized by `DetectionPolicy::{Strict, SoftFallback}`. Use the extraction to collapse the store split-brain.

- One chain (param → env → default/legacy → cwd → canonicalize) lives in `WorkspaceResolver::resolve(param, policy)`. Read/write differ ONLY by the `policy` arg — zero chain duplication.
- Two call-site wrappers for legibility: `resolve_workspace(p)` = `resolve(p, Strict)` (writes), `resolve_workspace_read(p)` = `resolve(p, SoftFallback)` (reads).
- `WorkspaceResolver` holds the `workspace_store_cache` + a `default_workspace: Option<PathBuf>` (replaces legacy `workspace_path`). `forgeplan_init` calls `resolver.seed(path, store)` instead of setting legacy fields. Legacy `self.store`/`self.workspace_path` retired → single source of truth for the store handle.
- HIGH-1 canonical-path guarantee moves inside the resolver (one canonical path for cache key AND lock) — preserved by construction, unit-testable in isolation.

**Why Selected** (ADI verdict, confidence High): the Acceptance Criteria already mandate touching all 21 read handlers, so the marginal cost of also retiring the legacy fields is low while the gain (single source of truth, isolated testability of the HIGH-1 invariant, no state-drift risk) is high. Pure Approach A (policy enum, keep split-brain) was rejected because it leaves the state-drift vector (read hits stale legacy store while write updates cache → phantom artifacts) — the exact bug class that produced this whole effort.

### Implementation Note — what actually shipped (post-ship reconciliation, audit ARCH-2)

**The shipped code realizes the SUBSTANCE of this decision but NOT the literal `WorkspaceResolver` TYPE extraction.** The Layer-7 adversarial audit (ARCH-2) correctly flagged that no `struct WorkspaceResolver` exists in the codebase (`grep` is empty) — what shipped is the in-place variant (closer to Option B): the single resolution chain and the collapsed store live as **methods + fields on `ForgeplanServer`**, not on a dedicated extracted type.

The decision's substance WAS delivered:
- **Single chain**: `ForgeplanServer::resolve_workspace_core(param, policy)` is the one resolution chain (param → env → default → cwd → canonicalize). `resolve_workspace(p)` = `core(p, Strict)` (writes), `resolve_workspace_read(p)` = `core(p, SoftFallback)` (reads) — thin wrappers, zero chain duplication. ✅
- **DetectionPolicy** enum parameterizes read-vs-write — exactly as decided. ✅
- **Split-brain collapsed**: legacy `self.store` / `self.workspace_path` retired in favour of a single `default_workspace` + `workspace_store_cache`; `forgeplan_init` seeds via `seed_default(path, store)`. ✅
- **HIGH-1 canonical guarantee** lives in `canonical_ws_dir` + `get_or_open_store`, used by both the cache key and the lock. ✅

Only the **encapsulation shape** differs: the logic is on `ForgeplanServer` rather than a separate `WorkspaceResolver` struct. This was a deliberate trade-off — Risk **R-2** (extraction churn in a degraded shell that had already failed 3 sub-agent waves) materialized, making a clean mid-PR type extraction too costly relative to its marginal benefit once the single-chain + single-store substance was already in place. A future follow-up MAY extract the type for isolated unit-testing of the resolver without a full `ForgeplanServer` (the one benefit not yet realized); it is not required for correctness. Throughout this ADR, read "`WorkspaceResolver::resolve`" / "`resolver.seed`" as the shipped `ForgeplanServer::resolve_workspace_core` / `seed_default` methods.

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| A — policy enum only, keep split-brain | Rejected | Fixes duplication but leaves two store systems → state-drift risk (stale read vs cached write). Treats symptom, not cause. |
| B — policy enum + collapse split-brain in-place (no extraction) | Rejected as primary | Right outcome but mixes the cache/default state into the already-13K-line server struct; harder to audit + unit-test the HIGH-1 invariant. |
| **C+B — extract WorkspaceResolver + collapse (hybrid)** | **Chosen** | Single chain + single store source + isolated, unit-testable resolver. Blast radius bounded (H2 evidence: 12 prod + ~24 test sites) and reviewable in one pass. |
| D — status quo (resolve_workspace_read duplicate) | Rejected | The problem itself: ~90-line duplication → guaranteed drift. |

## Consequences

### Positive
- Resolution chain in exactly one place; read/write asymmetry is a 1-enum parameter, not a duplicated body.
- Single source of truth for store handles — split-brain state-drift vector eliminated.
- HIGH-1 canonical guarantee isolated + unit-testable inside the resolver.
- `WorkspaceResolver` testable without booting a full `ForgeplanServer`.

### Negative (trade-offs)
- Larger blast radius than the duplicate-method shortcut: 12 prod `workspace_path` sites + ~24 test sites + `forgeplan_init` refactor + 21 read + 26 write handler call-sites re-pointed to the resolver.
- Requires a fresh adversarial audit pass after the move (the cache/lock/init seams all shift).
- Touches `forgeplan_init` — highest-risk surface for AC-2 backward-compat regressions.

### Risks
- **R-1**: `forgeplan_init` refactor breaks single-worktree backward compat (AC-2). Mitigation: keep `seed()` semantically identical to old field-set; full `cargo test --workspace` regression gate; the 24 test sites are the canary.
- **R-2**: extraction churn in a degraded shell environment (cwd-reset + output corruption already failed 3 sub-agent waves). Mitigation: orchestrator does all git via `git -C <abs>` (no `cd`); sub-agents write code only, never git; verify by grep + file-read, never by agent report.

## Invariants
- HIGH-1: ONE canonical path feeds both the store-cache key AND `acquire_workspace_lock` (no regression).
- ADR-003 file-first + RED-LINE #8 (no direct `LanceStore::*_artifact` from server.rs — through projection).
- AC-2: single-worktree / `forgeplan_init` flows byte-identical to v0.32.x post-extraction.
- Reads are lock-free; writes serialize via per-workspace lock.

## Evidence Requirements
- EVID: `cargo test --workspace` regression count holds (≥3113) after extraction — proves AC-2.
- EVID: unit test on `WorkspaceResolver` alone proving canonical lock==cache path (HIGH-1) without a server.
- EVID: fresh 2-agent adversarial audit (security + architecture) on the extracted resolver + collapsed init.

## Valid Until
**Date**: `valid_until: 2026-11-30` (6 months). Refresh triggers: a third resolution policy beyond strict/soft appears; the cache needs cross-process coordination (multi-process MCP); or `forgeplan_init` semantics change.

## Admissibility
- NOT: re-introduce a second copy of the resolution chain for any reason.
- NOT: a read path that can write, or a write path without the per-workspace lock.
- NOT: bypass the resolver to touch a raw store handle from a handler.

## Rollback Plan
- Trigger: extraction destabilizes AC-2 (test regressions that can't be closed in-PR).
- Steps: the extraction lives in its own commits on `feat/prd-078-integration`; `git revert` the extraction range returns to the committed HIGH-1/LOW-7/MED-5/LOW-8 state (write-side solid). The uncommitted `resolve_workspace_read` duplicate is discarded (`git checkout`), HIGH-2 re-scoped as a tracked follow-up.
- Blast radius: MCP server only; CLI untouched.

## Weakest Link
`forgeplan_init` seam: it is the one place legacy fields and the cache must agree during the transition. If `seed()` and the old field-set diverge, AC-2 breaks silently. Mitigation: a dedicated init-parity test before/after.

## AI Guidance
- All workspace resolution goes through `WorkspaceResolver::resolve(param, policy)`. New handlers pick `Strict` (mutating) or `SoftFallback` (read-only). Never duplicate the chain. Never hold a raw store handle.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-076 | based_on (design question this answers) |
| ADR-015 | refines (resolution structure; ADR-015 owns the policy) |
| PRD-078 | informs (implementation of the worktree-aware feature) |
| EVID-139 | informs (audit surfacing HIGH-2 + split-brain) |
| PROB-072 | informs (root signal) |
| PROB-067 | informs (lock race — HIGH-1) |

## Reasoning Trace
ADI on PROB-076 generated H1 (policy enum only), H2 (enum + collapse in-place), H3 (extract WorkspaceResolver). Recommended hybrid H3+H2 (extract + collapse), confidence High — because all read handlers must be touched regardless, the marginal cost of collapsing split-brain is low and removes the state-drift class. Evidence probes: H2 blast radius = 12 prod + ~24 test `workspace_path` sites + 2 `self.store` (bounded, auditable); H3 resolver signature = self-contained (owns cache + default_workspace), no back-ref/lifetime issues.










