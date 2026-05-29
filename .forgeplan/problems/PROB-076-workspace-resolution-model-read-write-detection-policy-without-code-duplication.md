---
depth: tactical
id: PROB-076
kind: problem
status: draft
title: Workspace resolution model — read/write detection policy without code duplication
---

# PROB-076: Workspace resolution model — read/write detection policy without code duplication

## Signal

During PRD-078 audit-fix (HIGH-2, read-tool worktree routing), a sub-agent implemented `resolve_workspace_read` by **duplicating ~90 lines** of `resolve_workspace`. The only difference between the two methods is one behaviour: the read variant skips the multi-worktree detection error-gate (soft fallback), while the write variant errors (-32602) when detection fires (strict).

This is code duplication of the entire resolution chain (param → env → legacy workspace_path → cwd → canonicalize). Any future change to the chain (new step, canonicalize tweak, bug fix) must be applied to BOTH methods in lockstep — a guaranteed drift source. The architect audit (EVID-139) independently flagged a related "split-brain: two parallel store-handle systems" (legacy `workspace_path`/`store` fields for reads + `workspace_store_cache` for writes).

## Context

PRD-078 made MCP mutating tools worktree-aware via `resolve_workspace(param) → ResolvedWorkspace { workspace_dir, resolved_via, store }`. The resolution chain:
1. explicit `workspace=` param (absolute, ~-expanded, reject relative)
2. `FORGEPLAN_WORKSPACE` env var (lazy per-call)
3. legacy `self.workspace_path` (set by `forgeplan_init`) + `self.store`
4. cwd → `find_workspace` walk-up
+ multi-worktree detection gate (ADR-015 Option E) fires on the cold-start cwd branch only
+ `canonical_ws_dir` applied at all return points (HIGH-1 fix — one canonical path for cache key AND lock)
+ `FORGEPLAN_DISABLE_WORKTREE_DETECT` escape hatch (LOW-7)

**The read/write asymmetry is real and intentional**: a write landing in the wrong tree corrupts (phantom artifact); a read hitting the wrong tree is non-destructive (not-found/stale). So reads should soft-fall-back, writes should hard-error. The QUESTION is how to express this asymmetry WITHOUT duplicating the chain.

## Current state (what exists on feat/prd-078-integration)

- `resolve_workspace(param)` — write path, has detection gate. COMMITTED + HIGH-1/LOW-7/MED-5/LOW-8 fixes inside it.
- `resolve_workspace_read(param)` — read path, ~90-line duplicate minus the gate. UNCOMMITTED (working tree).
- 9 read param structs got `workspace: Option<String>` (uncommitted). 21 read handlers still call `require_workspace()` — NOT migrated yet.
- Two store-handle systems coexist: legacy `store: Arc<RwLock<Option<Arc<LanceStore>>>>` + `workspace_path` (read tools + forgeplan_init) AND `workspace_store_cache: HashMap<PathBuf, Arc<LanceStore>>` (write tools via resolve_workspace).

## Constraints

- MUST NOT break single-worktree backward compat (AC-2). forgeplan_init + existing tests rely on legacy `workspace_path`/`store`.
- MUST preserve ADR-003 file-first invariant + RED-LINE #8 (no direct LanceStore::*_artifact from server.rs — go through projection).
- MUST keep the HIGH-1 guarantee: one canonical path for cache key AND lock (no regression of the concurrency fix).
- MUST keep MCP stdio transport.
- ~21 read handlers + ~26 already-migrated write handlers — the chosen model must not force a churn so large it can't be audited.

## Optimization Targets

- **No chain duplication**: param/env/legacy/cwd/canonicalize logic lives in ONE place. Read/write differ only by detection policy.
- **Clear call-site API**: a handler author can tell at a glance whether they get strict (write) or soft (read) resolution.
- **Single source of truth for the store handle**: ideally collapse the split-brain (legacy fields vs cache) OR document why both must persist with a non-stale rationale.
- **Auditable blast radius**: the refactor should be reviewable in one pass.

## Candidate approaches (for ADI to weigh)

- **A — policy enum param**: one `resolve_workspace_with(param, DetectionPolicy::{Strict,SoftFallback})`, two thin wrappers `resolve_workspace`/`resolve_workspace_read`. Kills chain duplication. Leaves split-brain store fields as-is.
- **B — A + collapse split-brain**: also retire legacy `workspace_path`/`store` fields, route everything (incl. forgeplan_init + reads) through the cache + resolution. Architecturally cleanest, largest blast radius.
- **C — extract a WorkspaceResolver type**: move the whole chain into a dedicated `struct WorkspaceResolver` in forgeplan-core (or mcp), owning the cache + policy, server holds one. Separation of concerns; bigger move; testable in isolation.
- **D — keep two methods (status quo / duplication)**: rejected on its face — the very problem.

## Acceptance Criteria

- [ ] Resolution chain (param/env/legacy/cwd/canonicalize) exists in exactly ONE place.
- [ ] Read vs write detection policy is a parameter/type, not a duplicated method body.
- [ ] HIGH-1 canonical-path guarantee preserved (regression test stays green).
- [ ] All read handlers route worktree-aware (Journey-1 read-back closes).
- [ ] Backward compat (AC-2) preserved; forgeplan_init path intact.
- [ ] Decision recorded as ADR (supersedes/extends ADR-015 resolution section).
- [ ] Blast radius reviewable in one adversarial audit pass.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-078 | informs (this refines the resolution implementation) |
| ADR-015 | extends (resolution model section) |
| EVID-139 | informs (audit that surfaced HIGH-2 + split-brain) |
| PROB-072 | informs (root multi-worktree signal) |
| PROB-067 | informs (lock race — HIGH-1 surface) |


