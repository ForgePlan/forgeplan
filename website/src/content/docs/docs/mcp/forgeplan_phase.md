---
title: forgeplan_phase
description: "Read advisory phase state for an artifact — current phase, history, workflow type."
---

Returns the advisory methodology phase for an artifact (Shape, Validate, Adi, Code, Test,
Audit, Evidence, Done) plus the full append-only transition history from
`.forgeplan/state/<id>.yaml`. Phase tracking is **advisory** — no other tool blocks on it.
If the state file does not exist (pre-PRD-056 artifact, or `phase.enabled: false` in
config) the response is `current_phase: "unknown"` with an empty history; never an error.

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

_Schema source: `crates/forgeplan-mcp/src/server.rs::PhaseReadParams`_

## Returns

```json
{
  "artifact_id": "PRD-057",
  "current_phase": "code",
  "workflow_type": "greenfield",
  "advanced_at": "2026-04-26T09:30:00Z",
  "history": [
    { "phase": "shape", "ts": "2026-04-25T14:00:00Z", "reason": null },
    { "phase": "validate", "ts": "2026-04-25T15:20:00Z", "reason": null },
    { "phase": "code", "ts": "2026-04-26T09:30:00Z", "reason": "FRs implemented" }
  ],
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
  "message": "No phase state file on disk — advisory only, never an error",
  "_next_action": "`PRD-001` has no phase state yet. ..."
}
```

## Example invocation

```json
{ "id": "PRD-057" }
```

## Typical sequence

1. `forgeplan_phase` — read current phase.
2. If `current_phase: "unknown"` and tracking is desired:
   [`forgeplan_phase_advance --to shape`](/docs/mcp/forgeplan_phase_advance/).
3. Otherwise follow the `_next_action` hint to the suggested next phase.

## CLI equivalent

[`forgeplan phase <id>`](/docs/cli/) — same data, same advisory semantics.

## See also

- [`forgeplan_phase_advance`](/docs/mcp/forgeplan_phase_advance/) — write the next transition
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) — gate around the `validate` phase
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) — the `done` terminal state of the methodology
- [Methodology guide](/docs/methodology/overview/) — Shape → Validate → Code → Evidence → Activate
