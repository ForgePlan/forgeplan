---
title: forgeplan_stale
description: "Detect stale artifacts with expired valid_until dates. Returns the list of expired artifacts with days since expiry."
---

Detect artifacts whose `valid_until` date has passed. These are "stale" - the decision or evidence they carry may no longer reflect reality and should be refreshed, renewed, or deprecated. This is the first step in the evidence-decay quality loop.

**Category**: Quality

## When an agent calls it

- **Periodic sweeps** - run weekly to catch expired decisions before they silently degrade R_eff.
- **Before release** - confirm no active artifact ships with expired evidence.
- **Session start** - complements `forgeplan_health` by showing per-item expiry.
- **After a long absence** - many artifacts may have drifted past `valid_until` while you were away.

Stale does not mean invalid. Expired eligible evidence contributes 0.1 to scoring without rewriting its stored congruence level; terminal packs remain excluded. A decision's own expiry is not the same as evidence decay.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

Illustrative response excerpt for 2026-04-11, not a live expiry report.

```json
{
  "stale": [
    {
      "id": "EVID-047",
      "title": "Authentication measurement",
      "valid_until": "2026-03-30",
      "days_expired": 12
    },
    {
      "id": "ADR-003",
      "title": "Architecture decision",
      "valid_until": "2026-04-01",
      "days_expired": 10
    }
  ],
  "total": 2
}
```

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_stale` - list expired artifacts.
2. For each: decide renew vs reopen vs deprecate.
3. If still supported, use CLI `forgeplan renew <ID> --reason <REASON> --until <YYYY-MM-DD>` to extend validity.
4. For a new draft with retirement of the old artifact, use CLI `forgeplan reopen <ID> --reason <REASON>`. Neither `forgeplan_renew` nor `forgeplan_reopen` is registered as an MCP tool.
5. `forgeplan_decay` - see R_eff impact from remaining stale evidence.

Expiry detection does not rewrite persisted lifecycle status. `renew` checks that stored status and its allowed transition; an expired record still stored as `active` is not automatically eligible for an `active` → `active` renewal.

## CLI equivalent

```bash
forgeplan stale
```

## See also

- [`forgeplan_decay`](/docs/mcp/forgeplan_decay/) - quantify R_eff loss from aged evidence.
- [`forgeplan renew`](/docs/cli/renew/) - extend `valid_until`.
- [`forgeplan reopen`](/docs/cli/reopen/) - create a lineage-linked new draft.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Detect stale artifacts with expired valid_until dates. Returns the list of expired artifacts with days since expiry.

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
