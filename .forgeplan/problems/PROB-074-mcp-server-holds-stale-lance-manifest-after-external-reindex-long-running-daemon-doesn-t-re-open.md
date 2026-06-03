---
depth: tactical
id: PROB-074
kind: problem
links:
- target: PROB-072
  relation: informs
- target: PROB-073
  relation: informs
- target: ADR-003
  relation: informs
status: draft
title: MCP server holds stale lance manifest after external reindex — long-running daemon doesn't re-open
---

# PROB-074: MCP server holds stale lance manifest after external reindex — long-running daemon doesn't re-open

## Signal

During the v0.32.0 surface-validation sprint (2026-05-21), invoking 6
canonical MCP tools in parallel (`forgeplan_health`, `_status`, `_list`,
`_get`, `_search`, `_session`) returned `lance error: Not found` on
5/6, pointing at a specific lance fragment UUID:

```
lance error: Not found:
  Users/explosovebit/Work/ForgePlan/.forgeplan/lance/artifacts.lance/data/110100000011111100101011af7a4f4bc4819b87e84e835bd2.lance
```

The 6th tool (`forgeplan_session`) succeeded — it reads
`.forgeplan/session.yaml` directly, never touches LanceDB.

## Root cause (verified)

Earlier in the session I ran `rm -rf .forgeplan/lance && forgeplan
reindex` (to verify the new CI workflow). That regenerated every
`.lance` fragment with new UUIDs. The MCP server process was started
before that — it cached the old manifest pointing at fragment
`110100…` which no longer exists. Verified locally:

```bash
$ ls .forgeplan/lance/artifacts.lance/data/ | head
0000001000110111101100001f2dea4663b1097da55d6df56d.lance
00000010110001111100011042693a4873b052f3721df82b30.lance
0000001011010000001110102b60114b33b0b2b4c20c081307.lance
00000011011000010000101176d15c48f8956d07611cfb9447.lance
000001000100101111010011728c3b45ffbf7d67c5bf8c4ef0.lance

$ ls .forgeplan/lance/artifacts.lance/data/110100…lance
ls: ... No such file or directory
```

The MCP server was holding open the LanceDB Dataset object whose
manifest was a snapshot at process-start time. External rewrites to
the filesystem invalidate that snapshot but the daemon never reopens.

## Constraints

- MUST NOT break ADR-003 (markdown is source of truth — current state
  ACTUALLY held: CLI continued to work fine throughout the incident,
  because it opens a fresh LanceDB connection per process invocation).
- MUST NOT regress per-call latency (PROB-073 — re-opening on every
  tool call would amplify the perf problem).
- MUST handle both "external reindex" and "explicit refresh" cases.

## Optimization Targets

- **Recovery**: after external mutation (CLI write, manual file edit,
  reindex), the next MCP tool call should succeed or return a clear
  "stale handle — restart server" hint.
- **Cache validity**: Dataset / manifest objects should detect stale
  state on the next read, ideally via lance's built-in versioning,
  and reopen automatically.
- **Operability**: when reopen fails, the error message should point
  the user / agent at the right recovery (`forgeplan reindex` or
  server restart).

## Hypotheses

1. **LanceDB Dataset is opened once in `ForgeplanServer::new`** and
   held for the daemon's lifetime. Need lazy reopen on read error.
2. **No manifest-version watch** — lance datasets have a version
   counter; we could poll-on-read or watch the manifest file.
3. **Filesystem inode watcher missing** — `.forgeplan/lance/` could
   be watched for mtime change → invalidate cached handle.
4. **The `forgeplan watch` CLI command** (real-time file→LanceDB sync)
   exists but only goes one direction — markdown → lance, not
   lance-handle-refresh.

## Observation Indicators

- Run any MCP tool after `forgeplan reindex` in another shell:
  succeeds with new data (today: fails with `Not found`).
- Long-running session that survives a `rm -rf lance/ && reindex`
  recovers transparently.
- Error path emits actionable hint: `Fix: restart MCP server (handle
  stale after external reindex)` instead of raw lance trace.

## Acceptance Criteria

- [ ] Reproducer integration test: spawn MCP server, mutate via
      external `rm -rf lance && reindex`, call MCP tool, expect either
      success or actionable error.
- [ ] Either auto-reopen on first NotFound, or explicit `Stale handle`
      error variant with clear recovery hint.
- [ ] CLI `forgeplan get` continues to work transparently (proves
      ADR-003 file-first path is unaffected).
- [ ] Documented in operations runbook: when MCP gives `lance error:
      Not found`, what to do.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-072 | informs (same MCP daemon, different state-staleness aspect) |
| PROB-073 | informs (MCP per-call latency — re-open-on-read would worsen, need a smarter cache) |
| ADR-003 | informs (file-first invariant validated: CLI worked while MCP failed) |






