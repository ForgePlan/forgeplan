---
depth: tactical
id: PROB-073
kind: problem
links:
- target: PROB-072
  relation: informs
- target: ADR-003
  relation: informs
- target: PROB-068
  relation: informs
status: draft
title: MCP per-call latency too high for agentic pipelines — filesystem-write + sync workaround faster than direct
---

# PROB-073: MCP per-call latency too high for agentic pipelines — filesystem-write + sync workaround faster than direct

## Signal

User dogfood feedback (2026-05-21, same session as PROB-072):

> «прям радует результат
> только сука медленно((
>
> например я заметил что если с forgeplan юзать mcp — то агент прям медленно
> делает работу
>
> а если сделать на файловой системе а потом только синкать и линковать через
> mcp — прям летает все
>
> ну и впринципе почему-то агент работает долго»

Context: user just shipped an 11-agent pipeline (`adstall-agentic-dev`) that
runs full Shape → Build → Verification → Tier 2 → Release on real tasks.
Pipeline works correctly (validates the methodology), but per-call MCP
latency is visible enough to be a daily pain.

## Observed pattern

Two paths to the same end state:

| Path | Speed | Used by |
|------|-------|---------|
| **Direct MCP**: `forgeplan_new("PRD", "X")` → server projects markdown + writes LanceDB row | SLOW (visible delay per call) | Methodology-pure agents |
| **File-first + sync**: agent writes `.forgeplan/prds/PRD-XXX-*.md` directly → `forgeplan scan-import` or `forgeplan_update body=...` to register | FAST | User's workaround |

The file-first workaround is **the ADR-003 invariant** itself — markdown
as source of truth. So the workaround is methodologically correct.
What's slow is the per-call MCP roundtrip + the work it does (LanceDB
open/write/commit, possibly embedding, possibly full state-machine
projection).

## Constraints

- MUST NOT break ADR-003 file-first invariant.
- MUST keep MCP transport as the canonical agent surface — plugins,
  pipelines, and skills are wired to MCP tools.
- MUST preserve atomicity (RED-LINE #11) — direct file edits without
  going through MCP/CLI desync LanceDB / state machine / canonical
  body.

## Optimization Targets

- **Per-call latency**: `forgeplan_new` + `forgeplan_link` should be
  fast enough that an 11-agent pipeline does not feel "slow" — concrete
  target TBD after profiling (P95 < 200ms? < 500ms?).
- **Batch mode**: should an agent that creates 5 artifacts pay 5× the
  per-call cost, or is there a batch surface (`forgeplan_new_many`,
  transaction-bracketed writes)?
- **LanceDB open cost**: does each tool call re-open the LanceDB
  connection? Connection pooling / persistent handle?
- **Embedding cost**: when does embedding fire (on create? on commit?
  on demand?). Can we defer / batch?
- **Projection cost**: state machine update + journal write + change-
  log append per call — bounded?

## Observation Indicators

- Wall-clock for `forgeplan_new` ≤ target on cold + warm cache.
- 11-agent pipeline end-to-end time matches expected work, not
  bottlenecked by forgeplan overhead.
- Workaround (file-write + sync) no longer dramatically faster than
  canonical path.

## Hypotheses (to verify with profiling)

1. **LanceDB connection re-open per call** — every MCP tool opens the
   table, writes, closes. Connection pooling would amortize this.
2. **Embedding on every body update** — even if `feature = fastembed`
   is off, the embed-path may still allocate / compute null vectors.
3. **Full reindex on link** — `forgeplan_link` may walk the whole
   graph to recompute R_eff / blocked / order.
4. **State machine file IO** — `.forgeplan/state/<ID>.yaml` written
   on every status change, no batching.
5. **MCP transport stdio overhead** — JSON-RPC framing + cold-start
   per `claude --print` agent dispatch.

## Acceptance Criteria

- [ ] Microbenchmark suite under `crates/forgeplan-core/benches/`
      measures `new`, `update body`, `link`, `validate`, `score` cold
      + warm latency.
- [ ] Top 3 hot paths identified (likely candidates: LanceDB write
      lock, embedding, R_eff cascade).
- [ ] Optimize top hot path; measure improvement.
- [ ] User dogfood verifies "feels fast" on 11-agent pipeline.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-072 | informs (same MCP surface, different aspect — worktree projection) |
| ADR-003 | informs (file-first invariant is the workaround the user discovered) |
| PROB-068 | informs (init/scan-import lossy round-trip — same MCP write path) |











