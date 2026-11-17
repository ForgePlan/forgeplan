---
title: forgeplan_phase
description: "Read advisory phase state for an artifact. Returns current_phase, workflow_type, timestamps, and the full append-only transition history from `.forgeplan/state/<id>.yaml`. If no state file exists yet (pre-PRD-056 artifact or phase tracking was disabled), returns `current_phase: unknown` - never an error. Phase tracking is advisory and never blocks other tools."
---

Returns the advisory methodology phase for an artifact (Shape, Validate, Adi, Code, Test,
Audit, Evidence, Done) plus the full append-only transition history from
`.forgeplan/state/<id>.yaml`. Phase tracking is **advisory** - no other tool blocks on it.
If the state file does not exist (pre-PRD-056 artifact, or `phase.enabled: false` in
config) the response is `current_phase: "unknown"` with an empty history. This fallback is not proof the artifact exists. Invalid IDs and some I/O errors can still return errors.

**Category**: Lifecycle (advisory)

## When an agent calls it

- Session start on an in-flight artifact: "where did I leave off?".
- Before invoking a heavy tool: confirm we are past the right phase (e.g. don't run
  `forgeplan_score` while still in `shape`).
- Reviewing an old artifact: walk the history to understand how it got to its current state.
- Audit / debugging: every phase transition recorded with timestamp and optional reason.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID whose phase state to read |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP text content. Illustrative state (history entries use `from`, `to`, `at`; absent optional `from`/`reason` fields are omitted):

```json
{
  "artifact_id": "PRD-057",
  "current_phase": "code",
  "workflow_type": "greenfield",
  "advanced_at": "2026-04-26T09:30:00Z",
  "history": [
    { "to": "shape", "at": "2026-04-25T14:00:00Z" },
    { "from": "shape", "to": "validate", "at": "2026-04-25T15:20:00Z" },
    { "from": "validate", "to": "code", "at": "2026-04-26T09:30:00Z", "reason": "FRs implemented" }
  ],
  "schema_version": 1,
  "_next_action": "`PRD-057` is on phase `code`. Suggested next: `test`. Manual override: `forgeplan_phase_advance PRD-057 --to <phase>`."
}
```

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

The unknown response has no `advanced_at` or `schema_version`. Corrupt, oversized, symlinked or future-schema state files can also be treated as unknown. Existing state remains readable when automatic tracking is disabled. This read does not check lifecycle status or enforce the suggested next step.

A YAML deserialization failure also causes `read_phase` to attempt renaming the state file to `<id>.yaml.corrupt.<timestamp>`. It returns unknown whether quarantine succeeds or fails. Missing, oversized, symlinked and future-schema files follow separate checks; do not assume every unknown response performed a rename.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "PRD-001"
}
```

## Typical sequence

1. `forgeplan_phase` - read current phase.
2. If `current_phase: "unknown"` and tracking is desired:
   [`forgeplan_phase_advance` with `to: "shape"`](/docs/mcp/forgeplan_phase_advance/).
3. Otherwise follow the `_next_action` hint to the suggested next phase.

## CLI equivalent

[`forgeplan phase <id>`](/docs/cli/) - same data, same advisory semantics.

## See also

- [`forgeplan_phase_advance`](/docs/mcp/forgeplan_phase_advance/) - write the next transition
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - gate around the `validate` phase
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - separate lifecycle activation; a `done` marker is not proof of activation
- [Methodology guide](/docs/methodology/overview/) - Shape → Validate → Code → Evidence → Activate
