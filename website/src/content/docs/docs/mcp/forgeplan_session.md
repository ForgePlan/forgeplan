---
title: forgeplan_session
description: "Show current methodology session state — phase (idle/routing/shaping/coding/evidence/pr), active artifact, depth, enforcement status. Use this to know where in the workflow you are."
---

Show the current methodology session state — which phase you are in (`idle` / `routing` / `shaping` / `coding` / `evidence` / `pr`), the active artifact driving the session, the routed depth, and whether methodology enforcement is enabled. The session is Forgeplan's state machine that ties Route → Shape → Code → Evidence → Activate together (PRD-019).

**Category**: Quality

## When an agent calls it

- **Resume work** — "what was I doing before?" — the session remembers active artifact and phase.
- **Before a transition** — read state, then call `forgeplan_guard` to check if the next transition is allowed.
- **Consistency checks** — if you think you're "coding" but session says `shaping`, something is off.
- **Onboarding new agents** — they can discover the workflow entrypoint via this tool.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

```json
{
  "phase": "coding",
  "active_artifact": "PRD-042",
  "depth": "standard",
  "enforcement": "enabled",
  "started_at": "2026-04-11T09:17:00Z",
  "history": [
    { "phase": "routing", "at": "2026-04-11T09:05:00Z" },
    { "phase": "shaping", "at": "2026-04-11T09:10:00Z" },
    { "phase": "coding", "at": "2026-04-11T09:17:00Z" }
  ]
}
```

If nothing is active:

```json
{
  "phase": "idle",
  "active_artifact": null,
  "depth": null
}
```

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_session` — read state.
2. `forgeplan_guard` with target phase — check if transition is allowed.
3. If allowed, perform the phase work (code / evidence / PR).
4. Phase auto-advances as artifacts move through lifecycle.

## CLI equivalent

```bash
forgeplan session
```

## See also

- [`forgeplan_guard`](/docs/mcp/forgeplan_guard/) — gate for phase transitions.
- [`forgeplan_route`](/docs/mcp/forgeplan_route/) — entry point that opens a session.
- [Methodology guide](/docs/methodology/overview/)
