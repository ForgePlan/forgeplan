---
title: forgeplan_phase
description: "Read advisory phase state for an artifact - current phase, history, workflow type."
---

Returns the advisory artifact lifecycle phase (`shape`, `validate`, `adi`,
`code`, `test`, `audit`, `evidence`, `done`) and transition history from
`.forgeplan/state/<id>.yaml`. This is separate from session-phase enforcement.
A missing file yields `current_phase: "unknown"` with empty history; malformed
or unreadable state can return a tool error. Disabling tracking does not erase
an existing state file.

**Category**: Lifecycle (advisory)

## When an agent calls it

- Session start on an in-flight artifact: "where did I leave off?".
- Before invoking a heavy tool: confirm we are past the right phase (e.g. don't run
  `forgeplan_score` while still in `shape`).
- Reviewing an old artifact: walk the history to understand how it got to its current state.
- Audit / debugging: every phase transition recorded with timestamp and optional reason.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID whose phase state to read. |
| `workspace` | `string \| null` | no | Explicit absolute or tilde-expanded workspace path. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::PhaseReadParams`_

## Returns

Existing-state responses serialize `artifact_id`, `workflow_type`,
`current_phase`, `advanced_at`, `history`, `schema_version`, plus
`_next_action`. Each history entry uses `from` (optional), `to`, `at`,
and optional `reason` — not `phase` and `ts`. Timestamps describe recorded
transitions, not independent proof that the named work was completed.

When no state file exists yet:

```json
{
  "artifact_id": "PRD-001",
  "current_phase": "unknown",
  "workflow_type": "greenfield",
  "history": [],
  "message": "No phase state file on disk - advisory only, never an error",
  "_next_action": "`PRD-001` has no phase state yet. ..."
}
```

## Example invocation

```json
{ "id": "PRD-057" }
```

## Typical sequence

1. `forgeplan_phase` - read current phase.
2. If `current_phase: "unknown"` and tracking is desired:
   [`forgeplan_phase_advance --to shape`](/docs/mcp/forgeplan_phase_advance/).
3. Otherwise follow the `_next_action` hint to the suggested next phase.

## CLI equivalent

[`forgeplan phase <id>`](/docs/cli/) - same data, same advisory semantics.

## See also

- [`forgeplan_phase_advance`](/docs/mcp/forgeplan_phase_advance/) - write the next transition
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - gate around the `validate` phase
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - the `done` terminal state of the methodology
- [Methodology guide](/docs/methodology/overview/) - Shape → Validate → Code → Evidence → Activate

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Read advisory phase state for an artifact. Returns current_phase, workflow_type, timestamps, and the full append-only transition history from `.forgeplan/state/<id>.yaml`. If no state file exists yet (pre-PRD-056 artifact or phase tracking was disabled), returns `current_phase: unknown` — never an error. Phase tracking is advisory and never blocks other tools.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | yes | string | Artifact ID whose phase state to read |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID whose phase state to read",
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
    "id"
  ],
  "title": "PhaseReadParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
