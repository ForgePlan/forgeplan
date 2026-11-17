---
title: forgeplan_discover_start
description: "Start a brownfield discovery session. Returns a structured protocol (7 phases: detect/structure/code/git/tests/docs/synthesize) that the AI agent follows to map an existing codebase. ForgePlan provides the protocol; the agent parses code and reports findings via forgeplan_discover_finding."
---

Starts a **brownfield discovery session** for an existing codebase. Forgeplan returns the seven-phase protocol (`detect`, `structure`, `code`, `git`, `tests`, `docs`, `synthesize`); the agent performs the investigation and reports findings. `forgeplan_discover_complete` later marks the session completed and returns counts. It does not synthesize proposals or run health on the agent's behalf in 0.38.0.

**Category**: Brownfield Discovery

## When an agent calls this

- First session on an unfamiliar repository - "what is this project and what state is it in?"
- Onboarding a legacy codebase into the Forgeplan methodology without fabricating artifacts from thin air.
- After a large merge or acquisition, when the agent needs to re-map the codebase before proposing new decisions.
- Before a migration PRD - discovery makes the "as-is" state explicit so the migration goals are grounded.

The discover protocol is intentionally **agent-driven**: ForgePlan never reads source files itself. It tells the agent *what* to look at and in *which tier order*, then accepts the findings.

## Source tier priority

The shipped discovery protocol uses **three**, not four, source tiers:

| Tier | Sources |
|---|---|
| **1** | Source code, git history and package manifests. |
| **2** | Tests, JSDoc comments and CI configuration. |
| **3** | `docs/`, README files and wikis. |

The protocol instructs the agent to inspect stronger sources first. It does not itself resolve contradictions. `forgeplan_discover_finding` rejects tier values outside 1-3, and tier-3 findings receive the `source=legacy-doc` tag.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `project_name` | `string` | yes | Project name for the discovery session |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The response contains `session_id` (for example `disc-20261023-120000`), `project_name`, `session_status`, `current_phase`, `protocol` and `_next_action`. `protocol` contains `version`, the ordered `phases` array (each with `phase`, `order`, `name`, `instructions`) and `source_tier_rules` (`t1`, `t2`, `t3`).

`status` is still emitted as a deprecated alias for the session status, with `_field_warnings`. It is not an artifact lifecycle status. Store the returned session handle rather than constructing one from the project name.

The initial `session_status` is `started` and `current_phase` is `detect`.
The session is saved as `.forgeplan/discovery/<session_id>.json`; no finding
artifact is created by this call. The protocol is instructions for the caller,
not a source-code scanner or an automatically enforced phase progression.
Success is JSON inside MCP text content; this handler does not attach workspace
trace fields. Workspace write-resolution can fail at JSON-RPC level; a session-save
failure is a tool error with plain text, not a protocol result.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "project_name": "billing-service"
}
```

## Typical sequence

```
forgeplan_discover_start
    ↓ returns session_id + 7-phase protocol
forgeplan_discover_finding (tier=1, phase=detect,    …)
forgeplan_discover_finding (tier=1, phase=structure, …)
forgeplan_discover_finding (tier=1, phase=code,      …)
forgeplan_discover_finding (tier=1, phase=git,       …)
forgeplan_discover_finding (tier=2, phase=tests,     …)
forgeplan_discover_finding (tier=3, phase=docs,      …)
forgeplan_discover_complete (session_id)
    ↓ returns counts and IDs of created draft artifacts; caller reviews them
```

The agent is expected to iterate per phase - it can emit many findings per phase before advancing.

## CLI equivalent

- [`forgeplan discover start`](/docs/cli/discover-start/) - same flow from the terminal, for a human operator.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_discover_finding`](/docs/mcp/forgeplan_discover_finding/) - append a single observation
- [`forgeplan_discover_complete`](/docs/mcp/forgeplan_discover_complete/) - finish the session and inspect its summary
- [`forgeplan_health`](/docs/mcp/forgeplan_health/) - post-discovery project state
