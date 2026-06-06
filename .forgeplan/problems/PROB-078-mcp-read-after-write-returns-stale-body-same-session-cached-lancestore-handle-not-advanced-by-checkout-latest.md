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
status: deprecated
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

## Session-2 investigation refinement (2026-06-03)

Three more probes, all to pin the mechanism — it is subtler than first stated:

1. **In-process direct-store probes PASS** (content-correct):
   - `update_body_changes_content` (existing): `create_artifact` → `update_body` → `get_record` sees new body. ✅
   - `opened_store_sees_same_session_add` (new probe, then reverted): `LanceStore::open()` → `create_artifact` → `get_record` sees the row. ✅
   So the staleness is NOT a simple `open()`-vs-`init()` handle difference, and NOT reproducible by calling the store methods directly in-process.

2. **The McpFixture Journey-1 e2e does NOT actually cover this.** `worktree_read_e2e.rs::p1_journey1_write_then_read_in_worktree` (in CI, green) asserts only `!body.is_empty()` (line ~273) — a STALE template body is non-empty, so it passes. **Test-quality gap**: the read-back assertion must check the body CONTAINS what was written, not just non-empty. Fixing that assertion would have caught this.

3. **MCP-new variants are inconsistent** (vs the solid CLI-new→stale case):
   - CLI `new` (separate process) → MCP `update`(@file or inline) → MCP `get`: deterministic **stale template** 3/3; the file on disk has the new body. (Solid, concrete file-vs-DB mismatch.)
   - MCP `new` → MCP `update`/`get` same session, WITH `workspace=`: update + get both `not found`.
   - MCP `new` → MCP `update`/`get` same session, NO `workspace=`: update OK, get `not found`.
   The inconsistency suggests either a second related issue OR an unreliable stdio-printf harness; a proper MCP client (not piped printf) is needed to disambiguate.

**Refined status**: the read-after-write staleness is REAL and concretely evidenced (the CLI-new→update→get 3/3 file-vs-DB case), but the mechanism is NOT `checkout_latest`/`refresh` (both fix attempts failed) and NOT reproducible via direct in-process store calls. It needs a DEDICATED session with (a) a reliable MCP client harness, (b) a strengthened Journey-1 content assertion, (c) possibly a `lancedb` version bisect, before a fix is attempted. Do NOT claim fixed until the deterministic repro goes green through the real binary.



## Resolution (2026-06-03): REFUTED — not a product bug

A dedicated session built a RELIABLE, deterministic reproduction (replacing the
hand-rolled stdio-printf repro) and the read-after-write staleness **does not
reproduce**. PROB-078 is a **harness artifact**, not a product bug. This section
supersedes the "Refined status" and "Revises PROB-075 F-6" conclusions above.

### Evidence (commit `8aae19a`, EVID-146)

Seven tests, three layers, all green (fmt 0 diffs, clippy -D warnings 0):

- store: `prob078_reopened_handle_sees_own_update_body` — a handle opened at V1
  (row created by a dropped handle) DOES observe its own `update_body` on a
  later `get_record`. **Refutes the "handle not advanced by update()" claim**
  in the Diagnostics section (point 3): LanceDB advances the handle on
  `update().execute()`.
- mcp in-process (McpFixture x3): full handler stack incl. the two-store probe,
  which proves the param-path and default-path resolve the SAME cached
  `Arc<LanceStore>`.
- real binary (x3): the actual `forgeplan-mcp` over real stdio via the rmcp
  child-process transport, incl. a BYTE-EXACT replica of the Reproduction block
  above (real CLI creates a NOTE, real MCP `update -> get` with `workspace=WS`).
  Fresh.

### Why the original repro lied

The "stale" reading came from the piped-printf stdio client (mis-framed /
mis-sequenced JSON-RPC), not the product. The earlier "MCP-new variants are
inconsistent / not found" symptoms (Session-2 point 3) were the same harness
unreliability. The `Arc::as_ptr` "same instance" diagnostic was correct — and is
exactly why there is no staleness: one handle, and it is fresh.

### Corrections to this record

- **Impact**: NOT a release blocker. v0.33 is unblocked on this axis.
- **Revises PROB-075 F-6**: REVERSED. F-6's "deferred, test-only, not a
  correctness bug" closure (EVID-144) is **vindicated** — the read path it would
  exercise is correct. EVID-144 stands.
- **Fix directions (1-3 above)**: do NOT implement. They would add latency for
  no correctness benefit.

### Residual

Verified on macOS (same machine/scenario as the original repro); the 7 tests run
on Linux CI after push. No breaking-change surface — tests only, no product code
changed. See EVID-146.



