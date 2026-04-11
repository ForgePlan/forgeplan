---
title: forgeplan_stale
description: "Detect stale artifacts with expired valid_until dates. Returns the list of expired artifacts with days since expiry."
---

Detect artifacts whose `valid_until` date has passed. These are "stale" — the decision or evidence they carry may no longer reflect reality and should be refreshed, renewed, or deprecated. This is the first step in the evidence-decay quality loop.

**Category**: Quality

## When an agent calls it

- **Periodic sweeps** — run weekly to catch expired decisions before they silently degrade R_eff.
- **Before release** — confirm no active artifact ships with expired evidence.
- **Session start** — complements `forgeplan_health` by showing per-item expiry.
- **After a long absence** — many artifacts may have drifted past `valid_until` while you were away.

Stale ≠ invalid — the artifact still exists, but its R_eff contribution is penalised (CL0, weight 0.1 in the scoring formula) until it's renewed.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

```json
{
  "stale": [
    {
      "id": "EVID-047",
      "kind": "evidence",
      "valid_until": "2026-03-30",
      "days_expired": 12,
      "linked_to": ["PRD-039", "RFC-006"]
    },
    {
      "id": "ADR-003",
      "kind": "adr",
      "valid_until": "2026-04-01",
      "days_expired": 10
    }
  ],
  "total_stale": 2
}
```

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_stale` — list expired artifacts.
2. For each: decide renew vs reopen vs deprecate.
3. `forgeplan_renew` to extend `valid_until` (if the decision is still sound).
4. `forgeplan_reopen` to create a new draft while deprecating the stale one.
5. `forgeplan_decay` — see R_eff impact from remaining stale evidence.

## CLI equivalent

```bash
forgeplan stale
```

## See also

- [`forgeplan_decay`](/docs/mcp/forgeplan_decay/) — quantify R_eff loss from aged evidence.
- [`forgeplan_renew`](/docs/cli/renew/) — extend `valid_until`.
- [`forgeplan_reopen`](/docs/cli/reopen/) — create a lineage-linked new draft.
- [Methodology guide](/docs/methodology/overview/)
