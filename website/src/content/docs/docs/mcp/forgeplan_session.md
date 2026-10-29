---
title: forgeplan_session
description: "Show current methodology session state - phase (idle/routing/shaping/coding/evidence/pr), active artifact, depth, enforcement status. Use this to know where in the workflow you are."
---

Show the current methodology session state - which phase you are in (`idle` / `routing` / `shaping` / `coding` / `evidence` / `pr`), the active artifact driving the session, the routed depth, and whether methodology enforcement is enabled. The session is Forgeplan's state machine that ties Route → Shape → Code → Evidence → Activate together (PRD-019).

Session state is not the per-artifact advisory phase marker (`forgeplan_phase`). Reading or routing a task does not itself prove work completion or authorize activation.

**Category**: Quality

## When an agent calls it

- **Resume work** - "what was I doing before?" - the session remembers active artifact and phase.
- **Before a transition** - read state, then call `forgeplan_guard` to check if the next transition is allowed.
- **Consistency checks** - if you think you're "coding" but session says `shaping`, something is off.
- **Onboarding new agents** - they can discover the workflow entrypoint via this tool.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

Returns `phase`, `active_artifact`, `route_depth`, `enforced`, `phase_started_at`, `history_count`, `next_action`, and `_next_action`.

Illustrative response excerpt:

```json
{
  "phase": "coding",
  "active_artifact": "PRD-042",
  "route_depth": "standard",
  "enforced": true,
  "history_count": 3
}
```

With no active artifact, `active_artifact` and `route_depth` can be null. The response gives a history count, not a full transition history.

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_session` - read state.
2. `forgeplan_guard` with target phase - check if transition is allowed.
3. If allowed, perform the phase work (code / evidence / PR).
4. Check session state explicitly; do not confuse automatic artifact phase markers with session transitions.

## CLI equivalent

```bash
forgeplan session
```

## See also

- [`forgeplan_guard`](/docs/mcp/forgeplan_guard/) - gate for phase transitions.
- [`forgeplan_route`](/docs/mcp/forgeplan_route/) - routing advice; the selected MCP route handler does not save session state
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Show current methodology session state — phase (idle/routing/shaping/coding/evidence/pr), active artifact, depth, enforcement status. Use this to know where in the workflow you are.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| — | — | — | No parameters. |

<details>
<summary>JSON Schema</summary>

```json
{
  "properties": {},
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
