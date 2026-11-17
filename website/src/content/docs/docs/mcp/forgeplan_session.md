---
title: forgeplan_session
description: "Show current methodology session state - phase (idle/routing/shaping/coding/evidence/pr), active artifact, depth, enforcement status. Use this to know where in the workflow you are."
---

Show the current methodology session state - which phase you are in (`idle` / `routing` / `shaping` / `coding` / `evidence` / `pr`), the active artifact driving the session, the routed depth, and whether methodology enforcement is enabled. The session is Forgeplan's state machine that ties Route → Shape → Code → Evidence → Activate together (PRD-019).

**Category**: Quality

## When an agent calls it

- **Resume work** - "what was I doing before?" - the session remembers active artifact and phase.
- **Before a transition** - read state, then call `forgeplan_guard` to check if the next transition is allowed.
- **Consistency checks** - if you think you're "coding" but session says `shaping`, something is off.
- **Onboarding new agents** - they can discover the workflow entrypoint via this tool.

## Input parameters

_No declared input fields. Call this tool with an empty object `{}`._

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

| Field | Meaning |
|---|---|
| `phase` | Saved session phase: `idle`, `routing`, `shaping`, `coding`, `evidence` or `pr`. |
| `active_artifact` | Artifact driving the session, when set. |
| `route_depth` | Saved depth string or `null`; the MCP route handler does not save it. |
| `enforced` | Derived from saved depth: false for `null` or exact `"tactical"`, true otherwise. Not proof that hooks or validation gates ran. |
| `phase_started_at` | Timestamp for the current phase. |
| `history_count` | Number of saved history entries, not the full history array. |
| `next_action`, `_next_action` | Guidance for the next step. |

The session is distinct from the per-artifact phase files read by [`forgeplan_phase`](/docs/mcp/forgeplan_phase/). The response does not have `depth`, `enforcement`, `started_at` or a full `history` array.

Success is JSON in MCP text content. The tool reads `.forgeplan/session.yaml` without changing it; a missing, unreadable or malformed session file yields the default idle state with null optional fields and empty history. History is retained up to 20 transitions. This endpoint has no per-call workspace argument. Session state is separate from actual code progress; a stale session does not prove work was not done.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_session` - read state.
2. `forgeplan_guard` with target phase - check if transition is allowed.
3. If allowed, perform the phase work (code / evidence / PR).
4. Re-read the session after the operation; do not infer its state from the separate artifact phase marker.

## CLI equivalent

```bash
forgeplan session
```

## See also

- [`forgeplan_guard`](/docs/mcp/forgeplan_guard/) - gate for phase transitions.
- [`forgeplan_route`](/docs/mcp/forgeplan_route/) - recommends depth without opening or updating a session on the MCP surface.
- [Methodology guide](/docs/methodology/overview/)
