---
title: forgeplan_guard
description: "Check if a methodology phase transition is allowed. Use before performing actions to avoid blocked operations. Example: can I go from 'shaping' to 'coding'? Returns allowed=true/false with reason."
---

Check whether a methodology phase transition is allowed by the session state machine (PRD-019). The guard enforces the core rule: you cannot jump phases without completing prerequisites. Example: you cannot transition from `shaping` to `coding` unless an active artifact exists and validation has passed. Call this before any phase-changing action (`forgeplan_activate`, commits, PR creation).

**Category**: Quality

## When an agent calls it

- **Before code** — "can I go from shaping to coding?" — confirms a validated artifact exists.
- **Before commit** — agents and hooks call guard to block commits without an artifact at Standard+ depth.
- **Before PR** — ensures evidence and R_eff gates are met.
- **Recovery** — after an error, guard tells you which prerequisite is missing and how to fix it.

This is the mechanism that prevents "code first, document later" drift. When enforcement is enabled, violations block the operation with a clear reason.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `target_phase` | `string` | yes | Target phase to check: `idle`, `routing`, `shaping`, `coding`, `evidence`, or `pr`. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::GuardParams`_

## Returns

```json
{
  "allowed": true,
  "from": "shaping",
  "to": "coding",
  "reason": "PRD-042 validated (PASS), ADI reasoning recorded"
}
```

Blocked case:

```json
{
  "allowed": false,
  "from": "shaping",
  "to": "coding",
  "reason": "No validated artifact — run forgeplan_validate first",
  "fix": "forgeplan validate PRD-042"
}
```

## Example invocation

```json
{ "target_phase": "coding" }
```

## Typical sequence

1. `forgeplan_session` — know where you are.
2. `forgeplan_guard` with `target_phase` — check the next hop.
3. If `allowed: false`, follow `fix` instruction (validate, link evidence, etc.).
4. Retry guard → proceed.

## CLI equivalent

```bash
forgeplan guard --target coding
```

## See also

- [`forgeplan_session`](/docs/mcp/forgeplan_session/) — current phase.
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) — unblock the shaping → coding transition.
- [Methodology guide](/docs/methodology/overview/)
