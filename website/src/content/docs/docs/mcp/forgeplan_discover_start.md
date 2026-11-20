---
title: forgeplan_discover_start
description: "Start a brownfield discovery session. Returns a structured protocol (7 phases: detect/structure/code/git/tests/docs/synthesize) that the AI agent follows to map an existing codebase."
---

Starts a **brownfield discovery session** and returns a seven-phase protocol (detect / structure / code / git / tests / docs / synthesize). The agent reads sources and records observations with `forgeplan_discover_finding`. `forgeplan_discover_complete` marks the session completed and returns counts; health checks and synthesis of follow-up proposals remain separate agent work.

**Category**: Brownfield Discovery

## When an agent calls this

- First session on an unfamiliar repository - "what is this project and what state is it in?"
- Onboarding a legacy codebase into the Forgeplan methodology without fabricating artifacts from thin air.
- After a large merge or acquisition, when the agent needs to re-map the codebase before proposing new decisions.
- Before a migration PRD - discovery makes the "as-is" state explicit so the migration goals are grounded.

The discover protocol is intentionally **agent-driven**: ForgePlan never reads source files itself. It tells the agent *what* to look at and in *which tier order*, then accepts the findings.

## Source tier priority

The returned protocol instructs the agent to prioritize sources; it does not inspect the source content or enforce its truth:

| Tier | Sources | Role |
|------|---------|------|
| **1** | Code, Git history, package manifests | Implementation and recorded intent |
| **2** | Tests, JSDoc comments, CI configuration | Behavioral expectations and supporting context |
| **3** | Docs, README, wiki | Claims to reconcile against stronger evidence |

Resolve conflicts with concrete evidence. A tier number is not proof that a statement is correct.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `project_name` | `string` | yes | Human-readable name for the discovery session (e.g. `"legacy-billing-service"`). Used as the session identifier prefix and in the final summary. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::DiscoverStartParams`_

## Returns

Returns `session_id`, `project_name`, `session_status`, `current_phase`, and a nested `protocol` containing `version`, `phases`, and `source_tier_rules`. Each phase has `phase`, `order`, `name`, and `instructions`. The legacy `status` field also refers to the session, not any artifact.

For example, the returned protocol's source-tier rules are:

```json
{
  "t1": ["code", "git", "package manifests"],
  "t2": ["tests", "JSDoc comments", "CI configs"],
  "t3": ["docs/ directory", "README files", "wiki"]
}
```

## Example invocation

```json
{ "project_name": "legacy-billing-service" }
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
    ↓ agent reviews findings and prepares follow-up proposals separately
```

The agent is expected to iterate per phase - it can emit many findings per phase before advancing.

## CLI equivalent

- [`forgeplan discover start`](/docs/cli/discover-start/) - same flow from the terminal, for a human operator.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_discover_finding`](/docs/mcp/forgeplan_discover_finding/) - append a single observation
- [`forgeplan_discover_complete`](/docs/mcp/forgeplan_discover_complete/) - complete the session and summarize stored findings
- [`forgeplan_health`](/docs/mcp/forgeplan_health/) - post-discovery project state

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Start a brownfield discovery session. Returns a structured protocol (7 phases: detect/structure/code/git/tests/docs/synthesize) that the AI agent follows to map an existing codebase. ForgePlan provides the protocol; the agent parses code and reports findings via forgeplan_discover_finding.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `project_name` | yes | string | Project name for the discovery session |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "project_name": {
      "description": "Project name for the discovery session",
      "type": "string"
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "project_name"
  ],
  "title": "DiscoverStartParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
