---
title: forgeplan_guard
description: "Check if a methodology phase transition is allowed. Use before performing actions to avoid blocked operations. Example: can I go from 'shaping' to 'coding'? Returns allowed=true/false with reason."
---

Checks a proposed **methodology session** transition against the saved session phase and route depth. It does not perform the transition, inspect artifact validation or R_eff, or advance the separate artifact lifecycle phase. A permitted transition is not proof that code, evidence, commits, or a PR are ready.

**Category**: Quality

## When an agent calls it

- **Before changing session phase** - ask whether the transition is allowed.
- **Recovery** - inspect a refused transition and the returned hints.

The selected implementation checks a transition table with explicit skips and a Tactical bypass. For example, `shaping` → `coding` is allowed by that table without checking for an active artifact or a validation result. Keep artifact validation and activation gates separate.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `target_session_phase` | `string` | yes | Target session phase: `idle`, `routing`, `shaping`, `coding`, `evidence`, or `pr`. `target_phase` is a legacy alias, not the canonical schema field. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::GuardParams`_

## Returns

Illustrative response excerpts. Hints are returned in `next_action` and `_next_action`, not a `fix` field.

```json
{
  "allowed": true,
  "current_phase": "shaping",
  "target_phase": "coding",
  "reason": "Transition allowed"
}
```

Blocked case:

```json
{
  "allowed": false,
  "current_phase": "routing",
  "target_phase": "pr",
  "reason": "Cannot go from 'routing' to 'pr'. Complete shaping → coding → evidence first"
}
```

## Example invocation

```json
{ "target_session_phase": "coding" }
```

## Typical sequence

1. `forgeplan_session` - know where you are.
2. `forgeplan_guard` with `target_session_phase` - check the next hop.
3. If `allowed: false`, read `reason`, `next_action`, and `_next_action`; resolve the session ordering issue without treating the hints as evidence that artifact gates passed.
4. Retry guard → proceed.

## CLI equivalent

The selected CLI has no `forgeplan guard` command. Use `forgeplan session` to inspect session state; the transition check described here is an MCP tool.

## See also

- [`forgeplan_session`](/docs/mcp/forgeplan_session/) - current phase.
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - separate artifact-content validation, not a check performed by this guard.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Check if a methodology **session phase** transition is allowed (idle → routing → shaping → coding → evidence → pr). Use before performing session-level actions to avoid blocked operations. Example: can I go from 'shaping' to 'coding'? Returns allowed=true/false with reason. NOTE: this guards the methodology session phase machine, NOT the artifact lifecycle phase machine (shape/validate/adi/code/test/audit/evidence/done) — for that use `forgeplan_phase_advance`. The `target_session_phase` argument is the canonical name; `target_phase` is accepted as a deprecated alias for backward compatibility (PROB-065).

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `target_session_phase` | yes | SessionPhaseKind | Target **methodology session** phase to check<br>(idle/routing/shaping/coding/evidence/pr).<br><br>Not to be confused with the **artifact lifecycle** phase enum<br>(shape/validate/adi/code/test/audit/evidence/done) exposed by<br>`forgeplan_phase_advance` (PROB-065). The legacy field name<br>`target_phase` remains accepted via serde alias so existing<br>callers do not break, but new callers should use<br>`target_session_phase`. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "SessionPhaseKind": {
      "description": "Methodology **session** phase enum (idle/routing/shaping/coding/evidence/pr).\n\nDistinct from artifact lifecycle `Phase` (shape/validate/adi/code/test/\naudit/evidence/done) exposed by `forgeplan_phase_advance`. Both enums\nshare an `evidence` variant lexically, but they target different state\nmachines (PROB-065). The MCP arg surface is named `target_session_phase`\n(with `target_phase` retained as a serde alias for backward compatibility)\nto make the bounded context explicit at the schema layer.",
      "enum": [
        "idle",
        "routing",
        "shaping",
        "coding",
        "evidence",
        "pr"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "target_session_phase": {
      "$ref": "#/$defs/SessionPhaseKind",
      "description": "Target **methodology session** phase to check\n(idle/routing/shaping/coding/evidence/pr).\n\nNot to be confused with the **artifact lifecycle** phase enum\n(shape/validate/adi/code/test/audit/evidence/done) exposed by\n`forgeplan_phase_advance` (PROB-065). The legacy field name\n`target_phase` remains accepted via serde alias so existing\ncallers do not break, but new callers should use\n`target_session_phase`."
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
    "target_session_phase"
  ],
  "title": "GuardParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
