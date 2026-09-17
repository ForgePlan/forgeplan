# Issue tracker — ForgePlan

Read by the `task-cycle` skill and the `task-runner` agent before Stage 0, so neither has to probe
for the tracker or guess which board this repository writes to.

| | |
|---|---|
| Tracker | **Orchestra** (desktop app; the MCP server runs inside it) |
| Workspace | `ForgePlan` — `kgYnhWodYw7kNbJMAK5q` |
| Project | **Development** — `ZB8WYUpwNskvAZdRK8Jh` |
| Endpoint | admin: `orch` `http://localhost:28173/mcp` · task: `orchestra-beta-forge-agent` `http://127.0.0.1:28174/mcp` |
| Rules | `docs/methodology/UNIFIED-WORKFLOW.ru.md` — sync rules |
| Verify | `get_current_context` + `list_workspaces` before the first write; confirm the workspace is `ForgePlan` |

## Endpoint and identity — two MCP roles

The program uses two Orchestra MCP servers with distinct jobs:

| Role | Server | Port | Principal | What it is for |
|---|---|---|---|---|
| Global user-MCP | `orch` | `:28173` | user `gogocat` (`Eli Rum`) | admin: fields, spaces, workspaces, runtime config |
| Task-MCP | `orchestra-beta-forge-agent` | `:28174` | agent `Forge Agent` | projects and tasks: create, move, checklists, close |

**Admin/config** goes through `orch` — the only global, user-identity MCP (it sees five spaces).
Writes through it land under the human (`Eli Rum`).

**Task work** goes through the workspace's agent runtime: `orchestra-beta-forge-agent`
(`:28174`, bot `Forge Agent`, allowed in `ForgePlan` only). If it stops reaching the board, it was
renamed or its allowlist shrank — re-verify, do not guess.

Always verify identity before the first write: `get_current_context` (who am I) +
`list_workspaces` (can this connection see the board). Never guess a server name — the
`orchestra-beta-*` runtimes change as the Orchestra app is reconfigured.

**Neither endpoint is part of the repository.** `.mcp.json` is gitignored, so the
server list is per-machine and a fresh clone has none of it — wire both endpoints
locally before expecting the board to answer:

```jsonc
// .mcp.json — gitignored
"orchestra-beta-forge-agent": {
  "type": "http",
  "url": "http://127.0.0.1:28174/mcp",
  "headers": { "Authorization": "Bearer ${ORCHESTRA_FORGE_AGENT_TOKEN}" }
}
```

The token goes in `.claude/settings.local.json` under `env` (also gitignored) and is
referenced as `${VAR}` — never written into a file that git can see. The server name
must also be listed in `enabledMcpjsonServers` or it stays dormant.

**A listening port is not a reachable board.** Measured 2026-09-17: ports `28173` and
`28174` both answered — the Orchestra app holds them — while neither MCP server was
present in the session. If `mcp__orch__*` / `mcp__orchestra-beta-forge-agent__*` are
absent, record the sync in `TODO.md` instead of guessing, and never hardcode a runtime
name to "restore" the connection.

## Field roles → this board's fields

The runbook speaks in roles; the actual field UIDs are workspace-specific. Resolve them at runtime
with `list_fields` — never hardcode a UID, because an option **name** sent where a UID is required
fails silently for custom fields.

| Role | Field here |
|---|---|
| status | `Status` — Backlog / To Do / Doing / Review / Done / Blocked |
| stage marker | `Phase` — Shape / Validate / Code / Evidence / Done |
| depth | `Depth` — Tactical / Standard / Deep / Critical |
| artifact | `Artifact` — the `[ARTIFACT-ID]` the task backs |
| type | `Type` — PRD / RFC / ADR / Spec / Epic / Problem / Evidence / bug / feature / docs |
| sprint | `Sprint` |
| branch | `Branch` |
| tags | `Tags` |

## Task naming

- With an artifact: `[ARTIFACT-ID] description` — e.g. `[PRD-019] MCP session state machine`.
- Without (bug / feature): plain description + `Tags`.

## Operations → calls

Bare tool names; the runtime prefix differs per runtime (`mcp__orch__` here). Match on the bare name.

| Operation | Call |
|---|---|
| list startable | `query_entities` — `Blocked by` empty, status ≠ Done |
| read one | `get_entity` + `read_messages` (paginated) + `get_checklists` |
| claim | `update_entity` — Status, Phase, Executor and Model in one call |
| tick | `manage_checklist_item` |
| report | `send_message` into the task's own chat |
| close | `update_entity` — Status and Phase together |
