---
depth: tactical
id: EVID-131
kind: evidence
links:
- target: PROB-072
  relation: informs
status: active
title: User dogfood report — Guardian rework loop from missing worktree projection (PROB-072 source)
---

# EVID-131: User dogfood report — Guardian rework loop from missing worktree projection (PROB-072 source)

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-05-21 |
| Valid Until | 2026-08-21 |
| Target | PROB-072 (MCP server cwd frozen at startup — multi-worktree projection drift) |

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit

## Measurement

User-reported dogfood incident (2026-05-21, after PR #299 merge), backed by a
direct read of the relevant source sites in `forgeplan-mcp` / `forgeplan-core`.

Method:
1. Captured user's verbatim report from
   `~/Downloads/forgeplan-feedback.md` (raw signal).
2. Cross-checked the claim against the actual code at
   `crates/forgeplan-mcp/src/main.rs:11`,
   `crates/forgeplan-core/src/workspace/init.rs:223`,
   `crates/forgeplan-mcp/src/server.rs:44`.
3. Surveyed all MCP tool param structs for a `workspace` /
   `worktree` field and grepped for a `FORGEPLAN_WORKSPACE` env var.

## Result

### User report (verbatim, Russian)

> «У меня был агент, у него сабагенты. Весь этот агент работает в worktree,
> и все это работает по методологии forgeplan, и на каждом этапе создается
> артефакты. Архитектор создал артефакты в forgeplan, и они улетели в
> LanceDB, но Guardian после него увидел, что файлов на диске в worktree
> нет, и отправил архитектора переделывать.»
>
> «МСР-сервер forgeplan имеет фиксированный CWD = root main repo. Когда
> subagent работает в worktree и вызывает `forgeplan_new`, projection
> пишется в main repo's `.forgeplan/<kind>s/`, не в worktree's.»

### Code read confirms three independent footguns

| Site | Behaviour |
|------|-----------|
| `crates/forgeplan-mcp/src/main.rs:11` | `let cwd = std::env::current_dir()?` — frozen at startup |
| `crates/forgeplan-core/src/workspace/init.rs:223` | `find_workspace(start)` walks **UP** from cwd; never sideways into a sibling worktree |
| `crates/forgeplan-mcp/src/server.rs:44` | `workspace_root: PathBuf` stored once in `ForgeplanServer`, used for every tool call |
| `NewParams` / `LinkParams` / `UpdateParams` | **No tool accepts a `workspace` parameter** — no per-call override |
| `FORGEPLAN_WORKSPACE` env var | **Not implemented** — no escape hatch |

### User-side workaround in active use

`forgeplan-marketplace` commit `a9a825c` adds a plugin-layer mitigation:
- `verify-projection.sh` checks both worktree CWD *and* `git rev-parse
  --git-common-dir`'s main repo
- Read-only roles get Write/Edit narrowly scoped to `.forgeplan/<kind>/`
- New `validate-write-paths-forgeplan-projection.sh` enforces the
  restriction
- All 11 agent definitions get a Materialize section:
  `forgeplan_new → verify-projection-in-both-locations →
  if-missing-then-write → re-verify`.

This is a workaround, not a fix. It can only succeed by writing the
markdown a second time from outside MCP — the LanceDB projection still
lands in the main repo on first call.

## Interpretation

The bug is real, reproducible at the source level, and not theoretical:

1. **Symptom matches code**: the user's described loop (Architect writes →
   Guardian can't see → Architect re-writes) is exactly what the frozen
   `workspace_root` will produce when the subagent's cwd is a worktree
   but the MCP server's cwd was the main repo.
2. **No in-tree mitigation**: zero MCP tools accept a workspace override,
   zero env vars give one, the workspace resolver never looks sideways
   — there is no path inside the current MCP server to route the write
   to the correct worktree.
3. **ADR-003 invariant violated under multi-worktree**: file projection
   ends up in `main_repo/.forgeplan/`, but the LanceDB record claims it
   lives where the caller called from. The "files are the source of
   truth" guarantee silently breaks.
4. **Plugin workaround addresses the symptom, not the cause**: it makes
   the markdown materialise in the worktree by re-writing from the
   agent layer, but the canonical first-write goes to the wrong place
   and LanceDB still indexes the wrong path.

This evidence supports PROB-072's framing and the case that the fix
needs to live in MCP server (not in plugin agents). Congruence level
3 — same context (forgeplan MCP), same artifact subject (projection
routing), same observation (multi-worktree drift).

## Congruence Level Justification

<!-- Legend: CL3 same-context (penalty 0.0); CL2 related (0.1); CL1 external (0.4); CL0 opposed (0.9). -->

CL3: the evidence and PROB-072 share **subject** (MCP server worktree
projection), **codebase** (this repo's `forgeplan-mcp` + `forgeplan-core`),
and **observation** (Guardian rework loop on multi-worktree). Same
context, no abstraction gap.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-072 | informs |



