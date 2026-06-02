---
depth: tactical
id: PROB-072
kind: problem
links:
- target: ADR-003
  relation: contradicts
- target: PROB-067
  relation: informs
status: deprecated
title: MCP server cwd frozen at startup — projection writes to main repo, not subagent worktree (multi-worktree projection drift)
---

# PROB-072: MCP server cwd frozen at startup — projection writes to main repo, not subagent worktree

## Signal

User reports (2026-05-21, dogfood feedback after PR #299 merge):

> «У меня был агент, у него сабагенты. Весь этот агент работает в worktree, и
> все это работает по методологии forgeplan, и на каждом этапе создается
> артефакты. Архитектор создал артефакты в forgeplan, и они улетели в LanceDB,
> но Guardian после него увидел, что файлов на диске в worktree нет, и
> отправил архитектора переделывать.»

Их investigation cataloged the root cause as: «МСР-сервер forgeplan имеет
фиксированный CWD = root main repo. Когда subagent работает в worktree и
вызывает `forgeplan_new`, projection пишется в main repo's
`.forgeplan/<kind>s/`, не в worktree's.»

Verified locally:

| Site | Behaviour |
|------|-----------|
| `crates/forgeplan-mcp/src/main.rs:11` | `let cwd = std::env::current_dir()?` — **freeze at startup** |
| `crates/forgeplan-core/src/workspace/init.rs:223` | `find_workspace(start)` walks **UP** from cwd, never sideways into a sibling worktree |
| `crates/forgeplan-mcp/src/server.rs:44` | `workspace_root: PathBuf` — stored once in `ForgeplanServer`, used for every tool call |
| `NewParams` / `LinkParams` / `UpdateParams` | **0 tools accept a `workspace` parameter** — no per-call override |
| `FORGEPLAN_WORKSPACE` env var | **does not exist** — no escape hatch |

## Constraints

- MUST NOT break the single-worktree (typical) case — backward compat is
  non-negotiable. Wire-shape changes on tools require ADR + version bump.
- MUST preserve ADR-003 file-first invariant — projection lives next to the
  workspace it belongs to, not in some shared cache.
- MUST keep MCP transport stdio — `claude` CLI integration depends on it.
- Multi-worktree usage is increasingly common (we ourselves use 19 worktrees
  during sprints) — this is not an edge case anymore.

## Optimization Targets

- **Correctness**: file projection MUST land in the subagent's worktree when
  the subagent invoked `forgeplan_new` from that worktree. Guardian's
  on-disk check is the source of truth per ADR-003.
- **Backward compatibility**: existing single-worktree CLI/MCP flows
  unchanged. No required parameter additions on existing tools.
- **Operability**: clear escape hatch (env var or explicit param) for users
  who explicitly want cross-worktree routing.

## Observation Indicators

- Subagent in worktree W calls `forgeplan_new` → file lands in `W/.forgeplan/`,
  not in `main_repo/.forgeplan/`.
- LanceDB index reflects the same path (no projection drift).
- Guardian's verify-projection hook finds the file in the SAME location the
  LanceDB record points at.
- No false-positive rework loops on multi-worktree pipelines.

## Workaround in use (until proper fix)

User shipped a hook-based workaround (commit `a9a825c` on their `dev`):
- `verify-projection.sh` now checks both worktree CWD AND git common dir's
  main repo
- Read-only roles (PM, Architect, Reviewer, Research, Security, System-Dev,
  Guardian) got Write/Edit tools narrowly path-restricted to
  `.forgeplan/<kind>/`
- New `validate-write-paths-forgeplan-projection.sh` hook enforces the
  restriction
- Materialize section added to all 11 agent definitions: explicit
  forgeplan_new → verify-projection-in-both-locations → if-missing-then-write
  → re-verify pattern

This is a **plugin-layer workaround**, not a forgeplan-core fix. The
correct fix is in MCP server — it should know about worktrees.

## Acceptance Criteria

- [ ] MCP server can route a `forgeplan_new` call to a worktree-specific
      `.forgeplan/` based on either:
      (a) per-call `workspace` / `worktree` parameter, OR
      (b) `FORGEPLAN_WORKSPACE` env var checked at tool-call time (not at
      server startup), OR
      (c) MCP server detects `git rev-parse --git-common-dir` and prefers
      `--show-toplevel` when invoked from a worktree.
- [ ] Existing single-worktree callers (no param, no env) see unchanged
      behaviour.
- [ ] Decision recorded as ADR (this is design-level and irreversible
      enough to deserve one).
- [ ] User's workaround can be deprecated — verify-projection.sh's
      dual-location check becomes unnecessary.
- [ ] Cross-worktree concurrent writes guarded by per-workspace lock
      (we already have `acquire_workspace_lock` but it's keyed on the
      static `workspace_root`).

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-003 | violates (file-first invariant breaks when projection lands in wrong tree) |
| PROB-067 | informs (forgeplan_new ID counter race in parallel worktrees — same surface) |












