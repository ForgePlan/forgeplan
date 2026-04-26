---
title: forgeplan_phase_advance
description: "Manually advance (or set) the advisory phase marker for an artifact and record a transition."
---

Writes the next phase to `.forgeplan/state/<id>.yaml`, appending an immutable history
entry with timestamp and optional reason. Advisory layer — does **not** validate phase
ordering, so out-of-order jumps (e.g. straight to `done` for a one-line fix) are allowed
by design. Full phase enforcement lands in a later PRD under EPIC-005. Use when
auto-advancement missed a transition or when reclassifying workflow state.

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

Failure (config disabled, filesystem unwritable):

```json
{
  "ok": false,
  "error": "Failed to advance phase: ...",
  "_next_action": "Check `.forgeplan/state/` is writable; verify phase tracking is enabled in config.yaml (`phase.enabled: true`)."
}
```

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

1. [`forgeplan_phase`](/docs/mcp/forgeplan_phase/) — read current state.
2. Do the work for the suggested-next phase.
3. `forgeplan_phase_advance` to record the transition.
4. Loop until `current_phase: "done"`.

## CLI equivalent

[`forgeplan phase advance <id> --to <phase>`](/docs/cli/) — same write, same advisory
semantics.

## See also

- [`forgeplan_phase`](/docs/mcp/forgeplan_phase/) — read current state + history
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) — the methodology activation gate
- [Methodology guide](/docs/methodology/overview/) — Shape → Validate → Code → Evidence → Activate
