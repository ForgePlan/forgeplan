---
title: forgeplan_phase_advance
description: "Manually advance (or set) the advisory **artifact lifecycle phase** marker for an artifact (shape/validate/adi/code/test/audit/evidence/done). Appends a transition to the history. Forward and out-of-order jumps are allowed, including a direct `done` override, and so is moving BACKWARDS - this tool is the deliberate-correction path (PRD-086 FR-007). Automatic advancement triggered by other tools refuses to move a phase backwards. Full phase enforcement lands in a later PRD under EPIC-005. Use when auto-advancement missed a transition or when reclassifying workflow state. NOTE: this targets the artifact lifecycle phase machine, NOT the methodology session phase machine (idle/routing/shaping/coding/evidence/pr) - for that use `forgeplan_guard` (PROB-065)."
---

Sets the advisory artifact phase in `.forgeplan/state/<id>.yaml`. Manual forward jumps, direct `done`, and **backward** corrections are allowed; automatic advancement by other tools refuses regression. This is not the methodology session phase checked by `forgeplan_guard`, and setting a marker does not perform or validate the work.

**Category**: Lifecycle (advisory)

## When an agent calls it

- Auto-advancement missed: tool ran but phase tracking was off, now turning it on.
- Reclassification: artifact got promoted from `code` to `audit` after the PR review wave.
- Backfilling: legacy artifact predates PRD-056, agent walks it through to `done`.
- Recording a deliberate skip: jump directly to `done` for a trivial fix, with a `reason`.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to advance |
| `reason` | `string \| null` | no | Optional reason / justification (recorded in history) Schema default: `null`. |
| `to` | `string` | yes | Target phase to advance to Enum: `"shape"`, `"validate"`, `"adi"`, `"code"`, `"test"`, `"audit"`, `"evidence"`, `"done"`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The payload contains `artifact_id`, `current_phase`, `workflow_type`, `advanced_at`, `history_entries`, `reason` (the sanitized request value or null) and `_next_action`. It does not include workspace trace fields. On an unchanged phase, `advanced_at` and history remain unchanged even though `reason` can echo a new request value.

Failures use MCP error text beginning `Failed to advance phase: ...`, not a JSON `ok: false` object. An oversized reason produces a JSON-RPC invalid-parameters error.

## Guards and side effects

- A valid filesystem-safe artifact ID and enum `to` are required. `reason` is optional and capped at 4096 bytes at the MCP boundary.
- The handler does not load an artifact record or check its existence. Missing phase state is initialized at `shape` with workflow type `greenfield`, then moved to the requested phase.
- A same-phase call records no duplicate transition; missing state is still materialized. History is capped at 1024 entries, preserving the initial entry and recent transitions, so it is not an unbounded immutable audit log.
- **0.38.0 distinction:** manual advancement does not consult `phase.enabled`; that setting controls the automatic hooks. The generic failure hint mentioning it is not proof that a disabled config blocked this manual call.
- The tool only writes phase state. It does not activate the artifact, run tests, create evidence or move the methodology session. Inspect the current marker before deliberately moving it backwards; there is no extra confirmation flag.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "RFC-001",
  "to": "test",
  "reason": "Implementation ready for testing."
}
```

## Typical sequence

1. [`forgeplan_phase`](/docs/mcp/forgeplan_phase/) - read current state.
2. Do the work for the suggested-next phase.
3. `forgeplan_phase_advance` to record the transition.
4. Loop until `current_phase: "done"`.

## CLI equivalent

[`forgeplan phase-advance --to <phase> <id>`](/docs/cli/phase-advance/) - same write, same advisory
semantics.

## See also

- [`forgeplan_phase`](/docs/mcp/forgeplan_phase/) - read current state + history
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - the methodology activation gate
- [Methodology guide](/docs/methodology/overview/) - Shape → Validate → Code → Evidence → Activate
