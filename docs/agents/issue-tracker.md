# Issue tracker — ForgePlan

Read by the `task-cycle` skill and the `task-runner` agent before Stage 0, so neither has to probe
for the tracker or guess which board this repository writes to.

This is **this repository's tracker configuration contract**, not a default for Forgeplan users.
The workspace/project identifiers below are retained; endpoint names, ports, visibility and membership
are local deployment facts that must be checked live before use. This documentation refresh does not
re-verify the board or authorize tracker writes.

Forgeplan's graph stores decisions, evidence and artifact relations. The tracker stores execution
status, task dependencies, checklists and responsibility. `task-runner` maintains the latter; it
does not edit source files, choose methodology, or create/link/activate Forgeplan artifacts.
Those operations belong to the delegated specialist with the relevant permissions.

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

**Admin/config** is configured here through `orch`, under the human identity. This is a local role
mapping, not a universal statement that every server with this name has global access.

**Task work** is configured through `orchestra-beta-forge-agent` (`:28174`, bot `Forge Agent`).
An unreachable or unexpectedly empty board can reflect startup, authorization, membership, runtime
registration or configuration changes. Verify the cause; do not rename the server or retarget a UID
based on an empty result.

**Task membership (verified 2026-09-18).** Project membership does NOT cascade to tasks: each
task is its own chat with its own members, and the Task-MCP sees only tasks it is a member of.
`Forge Agent` (`7eea6f7ad3fc4b3196385e9d3e3cb856`) must be added to every task it should work on —
at creation time, and to any pre-existing tasks (project member with zero visible tasks until
then). Admin adds are done through `orch` (`:28173`) or in the Orchestra UI.

Before the first write, resolve the role and verify identity and workspace. The public
`orchestra-mcp` driver uses `docs/agents/orchestra.json` (schema v2) to pin expected endpoint,
workspace and user, then runs `skills/orchestra-mcp/scripts/orch-verify.sh <role> --json`
from the installed `forgeplan-orchestra` plugin. If no pin exists, do not invent one from a port:
use the recorded contract and live `get_current_context` / `list_workspaces`, or obtain the missing
deployment configuration. Stop on mismatches, NOT_READY, missing credentials or ambiguous servers.
Never infer the current principal from the registered runtime name alone.

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

Keep the bearer token in an untracked local environment/secret facility; the example references
`${ORCHESTRA_FORGE_AGENT_TOKEN}`, never a literal token. Claude Code can use its untracked local
settings, but other runtimes have their own configuration. Enable/approve the server in the actual
host; `enabledMcpjsonServers` is a Claude Code setting, not a universal MCP requirement.

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

The public task-cycle also uses `Executor`, `Model` and `Blocked by` roles; some boards additionally
use `Area` and `Role`. Their existence, option values and UIDs are **not** asserted by this file.
Resolve them with `list_fields` for this board. If a required field is missing, report the exact
missing configuration rather than sending a guessed UID or silently claiming the pickup succeeded.
Do not change this board's naming convention merely to match a generic plugin example.

## Task naming

- With an artifact: `[ARTIFACT-ID] description` — e.g. `[PRD-019] MCP session state machine`.
- Without (bug / feature): plain description + `Tags`.

## Operations → calls

Bare tool names below omit the host's runtime prefix. For task work resolve the configured **task**
endpoint, not `orch` just because that prefix happens to be present in a session.

| Operation | Call |
|---|---|
| list startable | `query_entities` — `Blocked by` empty, status ≠ Done |
| read one | `get_entity` + `read_messages` (paginated) + `get_checklists` |
| claim | `update_entity` — Status, Phase, Executor and Model in one call, after resolving actual field/option UIDs |
| tick | `manage_checklist_item` — only when observable proof exists; preserve others' completed items |
| report | Return in session by default; `send_message` into the task's own chat only when explicitly authorized |
| close | `update_entity` — completed Status and final Phase together, only after acceptance evidence exists |

The tracker operation called **claim** above is not `forgeplan_claim`. An artifact claim is an
advisory local signal with a TTL, not task assignment or a source-file write lock. `task-runner`
can read `forgeplan_claims`, but its public role prohibits taking/releasing artifact claims or
mutating Forgeplan artifacts.

Read the full task description, paginated discussion and checklist before acting. Inspect every
write response for `failed`, `failedFields` and warnings; read back important changes.
Closing a task does not activate an artifact, and activation alone does not prove every task
acceptance criterion. For blocked work, set blocked status and **leave Phase at the stage reached**;
record the blocker and explicit unblock trigger.

Posting comments, changing assignees/owners/due dates or membership, and deleting entities require
the user's explicit authorization under the connected server's rules. A plugin workflow does not
grant it. Do not add the bot to tasks simply to repair visibility without that authorization.

## Public role and driver sources

- [task-runner responsibilities and prohibitions](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-orchestra/agents/task-runner.md#L184-L221).
- [task-cycle tracker resolution and field roles](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-orchestra/skills/task-cycle/SKILL.md#L29-L90).
- [orchestra-mcp pin, endpoint and identity checks](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-orchestra/skills/orchestra-mcp/SKILL.md#L23-L113).
- [Core 0.38.0 advisory claims](https://github.com/ForgePlan/Forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/claim/mod.rs#L1-L10).
