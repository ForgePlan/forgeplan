---
depth: tactical
id: PROB-067
kind: problem
links:
- target: PROB-060
  relation: informs
- target: PROB-066
  relation: informs
status: active
title: forgeplan_new ID counter race condition across parallel worktrees — same EVID number assigned to different artifacts
---

# PROB-067: forgeplan_new ID counter race — parallel worktrees assign same number

## Signal

Discovered during v0.31.0 cleanup sprint (2026-05-11) — multi-agent parallel work на разных git worktrees. Two workers (`w5-prob-065-fix` в `/Users/explosovebit/Work/fpl-w5-p065` и `w5-prob-066-fix` в `/Users/explosovebit/Work/fpl-w5-p066`) каждый вызвали `forgeplan_new(kind=evidence, ...)` примерно одновременно. ОБА получили id `EVID-119` despite вырабатывая completely different content (PROB-065 fix evidence vs PROB-066 fix evidence).

Worker w5-prob-066-fix's evidence file additionally overwrote the body of pre-existing EVID-118 (PROB-064 dual-key) before noticing, recoverable только через `forgeplan_update id=EVID-118 body=<original>`. The PROB-066 EVID-119 file additionally ended up in main repo as untracked while not appearing in worker's commit (race state caused worker's `git add` to miss it).

## Context

- **Workspace state**: 4 parallel worktrees off `chore/v031-dependabot-bump` tip, each с own working tree but **shared `.git/` directory**. LanceDB index at `.forgeplan/lance/` is gitignored — per-worktree но fundamentally same logical workspace.
- **Trigger**: parallel `forgeplan_new` calls reading `max(<kind>-*) + 1` counter. State is shared via filesystem (counter reads scan `.forgeplan/<kind>/*.md` files in shared parent OR per-worktree-instance LanceDB) — race window от scan-to-write.
- **PROB-060 marathon already addressed это для primary artifacts (PRD/RFC/ADR/Epic/Spec/Problem)** через slug-canonical identity + CI-bot assignment. **Evidence/Note/Memory/Refresh kinds did not receive same treatment** — counter-only assignment remains.

## Root cause

`forgeplan_new` для non-slug-augmented kinds:
- Reads `max(<kind>-NNN)` from existing `.forgeplan/<kind>/*.md` files
- Increments → assigns to new file
- No atomic claim/lock — race с другим worker reading same `max` value

ID assignment for evidence/note/memory/refresh is **counter-only**, no slug. Multi-agent parallel use case (sprint dispatch с workers) hits the race deterministically.

## Why now

v0.31.0 sprint scaled parallel agent work via worktrees (PROB-060 Phase 0b lesson — separate worktrees for parallel workers). Worker scale exposed race. Single-agent prior usage rarely hit this.

## Decision / Proposed fix

**Option A** (recommended): extend PROB-060 slug-canonical pattern к remaining kinds (evidence/note/memory/refresh). Auto-derive slug from title at creation time. Display number assignment by CI-bot post-merge. Same flow as PRD/RFC/ADR.

**Option B**: file-based atomic lock around `forgeplan_new` counter increment. Reuse workspace lock helper (`crates/forgeplan-core/src/workspace/lock.rs`).

**Option C**: skip race by giving workers a pre-allocated number range upfront (dispatch contract). Overkill for current scale.

**Option D**: deterministic collision detection — `forgeplan_new` re-checks file existence after write, retries if file already present с different content.

Lean Option A для consistency с PROB-060 pattern, accept Option B as quick mitigation if Option A scope is too large.

## Acceptance criteria

1. 2+ workers calling `forgeplan_new evidence` parallel в different worktrees получают different IDs
2. No silent overwrite of existing artifact (already detected — Error returned)
3. Regression test: stress test с 5 parallel `forgeplan_new evidence` invocations, all 5 unique IDs
4. CHANGELOG entry для assignment-flow change
5. Evidence linked

## Linked artifacts

- informs PROB-060 (slug-canonical identity marathon)
- informs PROB-066 (during fix этой race condition surfaced)

## References

- v0.31.0 sprint, 2026-05-11
- worker reports: w5-prob-065-fix, w5-prob-066-fix
- CWE-362 (Concurrent Execution using Shared Resource with Improper Synchronization)


