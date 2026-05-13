---
depth: tactical
id: EVID-121
kind: evidence
links:
- target: PROB-067
  relation: informs
status: active
title: PROB-067 fix — atomic cross-worktree lock + collision retry в forgeplan_new
---

# EVID-121: PROB-067 fix — atomic cross-worktree lock + collision retry в forgeplan_new

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Summary

PROB-067 fix landed: parallel `forgeplan_new` invocations across separate
git worktrees of the same repo now serialize on a per-kind lock anchored
at `<git-common-dir>/forgeplan/id-<KIND>.lock`, with post-write collision
detection retrying allocation on the next number when a sibling slips
through. 5 simultaneous `forgeplan new evidence` invocations yield 5
unique `EVID-NNN` ids and 5 distinct files on disk — the v0.31.0 sprint
race condition (two workers each receiving `EVID-119`) is resolved.

## Method

1. Inspected PROB-067 body and existing `WorkspaceLock`
   (`crates/forgeplan-core/src/workspace/lock.rs`) — workspace lock is
   per-worktree because each linked worktree has its own
   `.forgeplan/.lock` file, so it cannot serialize cross-worktree races.
2. Introduced `forgeplan_core::artifact::id_alloc` module implementing
   `allocate_and_create_artifact`:
   - **Option B (atomic lock)** — `acquire_id_alloc_lock` opens an
     exclusive `flock` on `<git-common-dir>/forgeplan/id-<KIND>.lock`
     when inside a git repo (shared across all worktrees), or a
     workspace-local fallback `<workspace>/.id-alloc-<KIND>.lock` when
     not. Reuses the symlink guard + exponential backoff pattern from
     `acquire_workspace_lock`. Per-kind granularity preserved — PRD and
     EVID allocations do not block each other.
   - **Option D (collision retry)** — pre-write file existence check
     plus post-write uniqueness verification of `{ID}-` prefix in
     `<kind-dir>`. On collision the projection file is rolled back and
     the allocator retries with a higher number (cap 5; panic above is
     intentional — indicates a broken lock rather than a recoverable
     race).
3. Wired both `forgeplan new` (CLI, `crates/forgeplan-cli/src/commands/new.rs`)
   and `forgeplan_new` MCP tool (`crates/forgeplan-mcp/src/server.rs`)
   to call `id_alloc::allocate_and_create_artifact` with a build
   closure that re-renders the template + augments frontmatter for the
   final allocated id. The closure runs once per retry, so a bumped
   id never carries stale rendering.
4. Returned `rendered_body` from the allocator (alongside `id` /
   `number` / `filepath`) so MCP response-shape derivation can keep
   reading `assigned_number` / `slug` / `predicted_number` from the
   augmented frontmatter (the on-disk projection re-builds the
   frontmatter without the identity triple — reading the file back
   would lose it).

## Findings

- **Race confirmed in pre-fix code path**: `next_id` reads `max(NNN)+1`
  from the LanceDB row set with no cross-worktree synchronisation; two
  workers each holding their own per-worktree `.forgeplan/.lock` see
  identical state and both write `EVID-119`. Observed live during
  v0.31.0 sprint (PROB-067 body, signal section).
- **Post-fix stress test passes**: `cli_prob_067_id_race::prob_067_parallel_new_evidence_unique_ids`
  (5 parallel `forgeplan new evidence` via process-level threading on
  the same workspace) produces exactly 5 distinct IDs and 5 distinct
  files. Run time ~12s. Re-runs are stable.
- **Per-kind lock granularity verified**:
  `prob_067_per_kind_lock_does_not_serialize_unrelated_kinds` — 10
  parallel allocations (5 evidence + 5 note) succeed with 5 unique IDs
  per kind. Cross-kind lock contention does not block.
- **Unit coverage in core**: 3 tests in
  `forgeplan_core::artifact::id_alloc::tests` exercise lock path
  resolution (per-kind naming), serialization (peak in-flight
  holders = 1), and per-kind independence.
- **No regressions**: 1985 lib tests PASS, 243+ CLI integration tests
  PASS, smoke-test.sh PASS, `cargo fmt --check` clean, `cargo clippy
  --workspace --all-targets -- -D warnings` clean.

## Pipeline gate

```
cargo fmt -- --check          → 0 diffs
cargo check --workspace       → 0 warnings
cargo clippy ... -D warnings  → 0 warnings
cargo test --workspace --lib  → 1985 PASS / 0 FAIL
cargo test -p forgeplan --tests → 243+ PASS (всех existing + 2 new PROB-067 stress)
bash scripts/smoke-test.sh    → PASSED
```

## Acceptance criteria — PROB-067

1. ✅ 2+ workers calling `forgeplan_new evidence` parallel в different
   worktrees получают different IDs — cross-worktree per-kind lock
   serializes the allocation critical section.
2. ✅ No silent overwrite — pre-write existence check refuses to write
   to an already-occupied `{ID}-*.md` path; post-write verification
   catches any racy intrusion and retries.
3. ✅ Stress test с 5 parallel `forgeplan_new evidence` invocations —
   `cli_prob_067_id_race.rs` PASSES, all 5 unique IDs.
4. ✅ CHANGELOG entry для assignment-flow change — Unreleased § Fixed
   section under "PROB-067 — `forgeplan_new` ID-counter race ...".
5. ✅ Evidence linked + activated — this EVID-121 → PROB-067 informs.

## Risks / follow-ups

- Allocator retry cap fires `panic!` on exhaustion to surface a broken
  lock. In a multi-tenant SaaS context this might warrant a recoverable
  error instead — out of scope for current single-user / agent-team
  scale.
- `git rev-parse --git-common-dir` is invoked synchronously per
  allocation (~microseconds). At higher allocation throughput (1000s/s)
  caching the result per-process would reduce overhead — not currently
  a bottleneck (CLI handlers + MCP handlers each spawn a separate
  process / runtime per request).
- PROB-067 follow-up Option A (slug-canonical identity for
  evidence/note/memory/refresh, parity with PRD/RFC/ADR/Epic/Spec) is
  not in scope here. The atomic-lock + retry combo closes the race
  surface; Option A becomes a future ergonomic improvement (display id
  carries deterministic slug from title).

## References

- PROB-067 body, signal & root cause sections
- v0.31.0 sprint marathon, 2026-05-11 (EVID-118/EVID-119 overwrite incident)
- WorkspaceLock parent module: `crates/forgeplan-core/src/workspace/lock.rs`
- New code:
  - `crates/forgeplan-core/src/artifact/id_alloc.rs` (allocator + tests)
  - `crates/forgeplan-cli/tests/cli_prob_067_id_race.rs` (process-level stress)
- Re-wired call sites:
  - `crates/forgeplan-cli/src/commands/new.rs`
  - `crates/forgeplan-mcp/src/server.rs` (forgeplan_new handler)
- CHANGELOG.md § Unreleased § Fixed
- CWE-362 (Concurrent Execution using Shared Resource)



