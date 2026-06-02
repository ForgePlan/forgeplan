---
depth: tactical
id: PROB-078
kind: problem
links:
- target: PROB-074
  relation: informs
- target: PROB-075
  relation: informs
- target: PROB-073
  relation: informs
- target: PRD-078
  relation: informs
status: draft
title: MCP read-after-write returns stale body same session — cached LanceStore handle not advanced by checkout_latest
---

## Signal

Discovered during the v0.33 release E2E pass (RED LINE #5, real-binary dogfood) on 2026-06-02. An MCP client that calls `forgeplan_update` then `forgeplan_get` for the same artifact **in the same MCP session** gets back the **pre-update (stale) body**. The file on disk and a fresh CLI process both show the correct new body — only the long-lived MCP session reads stale.

This is the read-after-write axis of the same staleness family as PROB-074, but PROB-074's `checkout_latest`-based `refresh()` does NOT fix it (proven below).

## Reproduction (deterministic, 3/3)

```
# fresh forgeplan workspace; create NOTE-001 via CLI
# one MCP stdio session:
initialize → forgeplan_update(id=NOTE-001, body="MARKER unique", workspace=WS)
           → forgeplan_get(id=NOTE-001, workspace=WS)
# get.body = the NOTE template body (from create), NOT "MARKER unique"
```

- File on disk (`.forgeplan/notes/NOTE-001-*.md`): **correct** (new body). ✅
- CLI `forgeplan get` (fresh process): **correct** (new body). ✅
- `forgeplan reindex` then get: correct. ✅
- MCP `forgeplan_get` same session as the update: **STALE** (template). ❌ — 3/3 deterministic.
- Not `@filepath`-specific: an **inline** body update is equally stale (so this is NOT #350).

## Diagnostics (measured, not guessed)

1. **Same store instance**: instrumented `Arc::as_ptr(&store)` in both handlers — `forgeplan_update` and `forgeplan_get` get the **identical** `Arc<LanceStore>` (`0x115cd64d0`) and identical `workspace_dir`. So this is NOT a split-store / cache-key mismatch.
2. **`update()` does commit to disk**: a fresh `LanceStore::open` (CLI, new process) reads the new body. So `self.artifacts.update().execute()` persists correctly.
3. **The in-memory handle cannot be advanced**:
   - `LanceStore::update_body` uses `self.artifacts.update().execute()` (in-place UPDATE). Unlike `add()` (which advances the handle — the create's template IS visible), `update()` commits a new on-disk version the handle does not pick up.
   - **Failed fix A** — `self.artifacts.checkout_latest()` after each of the 6 `.update()` methods: still stale 3/3.
   - **Failed fix B** — full `store.refresh()` (checkout_latest on all 5 tables) immediately BEFORE `get_record`: still stale 3/3.
   - `with_retry_on_stale` never fires: the row EXISTS (at the old version), so there is no `Not found` to trigger the retry/refresh path.

**Conclusion**: `checkout_latest()` is insufficient to advance a long-lived `Table` handle to a version committed by `update().execute()` in the same process. Only a **fresh `open_table` / store re-open** observes the write. PROB-074's `refresh()` primitive is therefore **ineffective for same-process read-after-write** (it was validated only against external reindex / `Not found`).

## Impact

**Release-blocker for v0.33.** "Update then read in the same session" is a core agentic-pipeline pattern (the exact workflow PRD-078 + PROB-073 are about). Agents silently get stale data after their own writes — worse than a hard error. Likely contributes to the PROB-073 "file-first feels faster/correct" workaround signal.

## Revises PROB-075 F-6

PROB-075 was closed (EVID-144) with F-6 ("true manifest-version-skew test") marked **deferred as "test-only, not a correctness bug."** This E2E proves that assessment **wrong** — the manifest-version skew F-6 pointed at IS a real correctness bug (this PROB). The deferral reasoning does not hold.

## Fix directions (need design — NOT a quick patch)

1. **Re-open after write**: evict the workspace store from `workspace_store_cache` after a mutating commit (or re-open the affected `Table`), so the next read opens fresh. Cost: a store/table re-open on the next read (store open ≈ 22ms per the PROB-073 profile) — needs a perf-vs-correctness call, possibly table-granular re-open.
2. **Read body from the file (ADR-003)**: markdown is the source of truth and always fresh; `forgeplan_get` could read the `.md` body instead of the DB row. Bigger semantic change; other reads (list/search) still hit the DB.
3. Investigate whether a newer `lancedb` exposes a working "advance handle to latest" that `checkout_latest` does not.

This warrants a focused session (likely an ADR weighing re-open vs file-read vs lancedb-upgrade with the latency tradeoff), not a marathon-tail patch. Two guessed fixes already failed; the next attempt must be verified against the deterministic repro + real-binary dogfood before claiming closure.

## Related

- PROB-074 (stale manifest — external reindex; its `refresh()` is insufficient here)
- PROB-075 F-6 (this is the real bug F-6 pointed at; the "deferred, test-only" closure was wrong)
- PROB-073 (latency; the file-first workaround is also a staleness workaround)
- PRD-078 (worktree routing — same store-handle surface)
- ADR-003 (markdown source of truth — basis for fix direction 2)





