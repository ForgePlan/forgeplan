---
title: forgeplan_guard
description: "Check if a methodology **session phase** transition is allowed (idle → routing → shaping → coding → evidence → pr). Use before performing session-level actions to avoid blocked operations. Example: can I go from 'shaping' to 'coding'? Returns allowed=true/false with reason. NOTE: this guards the methodology session phase machine, NOT the artifact lifecycle phase machine (shape/validate/adi/code/test/audit/evidence/done) - for that use `forgeplan_phase_advance`. The `target_session_phase` argument is the canonical name; `target_phase` is accepted as a deprecated alias for backward compatibility (PROB-065)."
---

Checks whether the **methodology session** may transition to `target_session_phase` (`idle`, `routing`, `shaping`, `coding`, `evidence`, `pr`). This is a read-only preflight against the saved session state, not an operation that moves the session, blocks arbitrary file edits, or records an artifact phase. For the separate artifact phase marker (`shape` through `done`), use [`forgeplan_phase_advance`](/docs/mcp/forgeplan_phase_advance/).

**Category**: Quality

## When an agent calls it

- **Before code or PR work** - ask whether the saved session allows that transition.
- **Before a guarded operation** - a client or installed hook can use the result to decide whether to proceed; calling this tool alone does not install enforcement.
- **Recovery** - after an error, guard tells you which prerequisite is missing and how to fix it.

Use the canonical input key `target_session_phase`. The old `target_phase` spelling remains a compatibility alias, but is not the schema field new clients should send.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `target_session_phase` | `string` | yes | Target **methodology session** phase to check (idle/routing/shaping/coding/evidence/pr).  Not to be confused with the **artifact lifecycle** phase enum (shape/validate/adi/code/test/audit/evidence/done) exposed by `forgeplan_phase_advance` (PROB-065). The legacy field name `target_phase` remains accepted via serde alias so existing callers do not break, but new callers should use `target_session_phase`. Enum: `"idle"`, `"routing"`, `"shaping"`, `"coding"`, `"evidence"`, `"pr"`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The response contains `current_phase`, `target_phase`, `allowed`, `reason`, `next_action` and `_next_action`. Note that the response key is still `target_phase`, even though the input key is `target_session_phase`.

When `allowed` is false, read `reason` and the returned hint, satisfy the missing prerequisite, and check again. The result does not move either state machine.

Success, including `allowed: false`, is JSON in MCP text content, not an MCP error. This check uses only saved phase and `route_depth`: it does not load artifact validation, evidence or ADI results. Tactical depth permits every transition; otherwise same-phase/reset-to-idle, the normal sequence, and explicit skips (`idle→shaping`, `idle→coding`, `shaping→evidence`, `coding→pr`) are allowed. Even `enforced: false` from an unset route depth does not mean all transitions are allowed.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "target_session_phase": "coding"
}
```

## Typical sequence

1. `forgeplan_session` - know where you are.
2. `forgeplan_guard` with `target_session_phase` - check the next hop.
3. If `allowed: false`, read `reason` and `_next_action` before proceeding.
4. Retry guard → proceed.

## CLI equivalent

There is no CLI command for the guard - it is an advisory check on the MCP
surface only. These related CLI operations are available:

```bash
forgeplan session           # the phase you are in
forgeplan validate PRD-001  # separate artifact validation, not a check performed by guard
```

## See also

- [`forgeplan_session`](/docs/mcp/forgeplan_session/) - current phase.
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - separate artifact-content validation.
- [Methodology guide](/docs/methodology/overview/)
