---
title: forgeplan_phase_advance
description: "Manually advance (or set) the advisory phase marker for an artifact and record a transition."
---

Writes an advisory artifact phase marker to `.forgeplan/state/<id>.yaml`. Explicit corrections can move forward or backward, including directly to `done`; they do not certify the work or bypass policy gates. A changed phase appends a timestamped reason to bounded history; repeating the same phase does not add a history entry.

This is distinct from the methodology **session** phase machine guarded by `forgeplan_guard`.

**Category**: Lifecycle (advisory)

## When an agent calls it

- Auto-advancement missed: tool ran but phase tracking was off, now turning it on.
- Reclassification: artifact got promoted from `code` to `audit` after the PR review wave.
- Backfilling: legacy artifact predates PRD-056, agent walks it through to `done`.
- Recording a deliberate skip: jump directly to `done` for a trivial fix, with a `reason`.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to advance. |
| `to` | `string` | yes | Target phase. One of `shape`, `validate`, `adi`, `code`, `test`, `audit`, `evidence`, `done`. |
| `reason` | `string` | no | Optional justification recorded in history. Hard cap 4096 bytes (rejected at the boundary to prevent DoS). |

_Schema source: `crates/forgeplan-mcp/src/server.rs::PhaseAdvanceParams`_

## Returns

Illustrative success response:

```json
{
  "artifact_id": "PRD-057",
  "current_phase": "test",
  "workflow_type": "greenfield",
  "advanced_at": "2026-04-26T11:00:00Z",
  "history_entries": 4,
  "reason": "FR tests green",
  "_next_action": "`PRD-057` advanced to `test`. Suggested next: `audit`."
}
```

On an invalid state file or unwritable filesystem, MCP returns an error result with text, not a JSON `ok`/`error` envelope. Illustrative text:

```text
Failed to advance phase: ...

_next_action: Check `.forgeplan/state/` is writable; verify phase tracking is enabled in config.yaml (`phase.enabled: true`).
```

The returned hint mentions config, but explicit advancement does not check `phase.enabled`.

## Example invocation

Standard transition:

```json
{ "id": "PRD-057", "to": "test", "reason": "FR tests green" }
```

Skip ahead (advisory, no validation gate):

```json
{ "id": "NOTE-019", "to": "done", "reason": "trivial typo fix" }
```

## Typical sequence

1. [`forgeplan_phase`](/docs/mcp/forgeplan_phase/) - read current state.
2. Do the work for the suggested-next phase.
3. `forgeplan_phase_advance` to record the transition.
4. Loop until `current_phase: "done"`.

## CLI equivalent

[`forgeplan phase-advance <id> --to <phase>`](/docs/cli/) - same explicit correction path.

## See also

- [`forgeplan_phase`](/docs/mcp/forgeplan_phase/) - read current state + history
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - the methodology activation gate
- [Methodology guide](/docs/methodology/overview/) - Shape → Validate → Code → Evidence → Activate

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Manually advance (or set) the advisory **artifact lifecycle phase** marker for an artifact (shape/validate/adi/code/test/audit/evidence/done). Appends a transition to the history. Forward and out-of-order jumps are allowed, including a direct `done` override, and so is moving BACKWARDS — this tool is the deliberate-correction path (PRD-086 FR-007). Automatic advancement triggered by other tools refuses to move a phase backwards. Full phase enforcement lands in a later PRD under EPIC-005. Use when auto-advancement missed a transition or when reclassifying workflow state. NOTE: this targets the artifact lifecycle phase machine, NOT the methodology session phase machine (idle/routing/shaping/coding/evidence/pr) — for that use `forgeplan_guard` (PROB-065).

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | yes | string | Artifact ID to advance |
| `reason` | no | string \| null | Optional reason / justification (recorded in history) |
| `to` | yes | PhaseArg | Target phase to advance to |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "PhaseArg": {
      "description": "JSON-Schema enum mirroring `forgeplan_core::phase::Phase`.\nLLM clients constrain-sample against this so `advance` tools\nget exact values (\"shape\", \"validate\", etc.) rather than paraphrases.",
      "enum": [
        "shape",
        "validate",
        "adi",
        "code",
        "test",
        "audit",
        "evidence",
        "done"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to advance",
      "type": "string"
    },
    "reason": {
      "default": null,
      "description": "Optional reason / justification (recorded in history)",
      "type": [
        "string",
        "null"
      ]
    },
    "to": {
      "$ref": "#/$defs/PhaseArg",
      "description": "Target phase to advance to"
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
    "id",
    "to"
  ],
  "title": "PhaseAdvanceParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
