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

Stale is derived freshness, not a stored lifecycle transition. This report does not change status, renew anything, or recompute the score cache. Expired evidence scores 0.1 in the evidence scorer; that is an expiry override, not a CL0 assignment or multiplicative weight.

## Input parameters

_No declared input fields. Call this tool with an empty object `{}`._

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP `content[0].text`: `stale`, `total`, and `_next_action`. Each item contains only `id`, `title`, `valid_until`, `days_expired`; no `kind` or `linked_to`.

The store includes dates strictly before today's UTC date, across all kinds/statuses. Missing or unparseable expiry dates are skipped. The store accepts date-only and RFC 3339 values, but this handler calculates `days_expired` from date-only values; an included RFC 3339 expiry can therefore report zero days. Records are sorted by the stored `valid_until` string. A missing workspace is a plain-text tool error; store failures can be JSON-RPC errors.

Illustrative response assuming UTC date 2026-04-11:

```json
{
  "stale": [
    {"id": "EVID-047", "title": "Auth tests", "valid_until": "2026-03-30", "days_expired": 12}
  ],
  "total": 1,
  "_next_action": "1 stale artifact(s). Inspect first: `forgeplan_get EVID-047`."
}
```

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_stale` - list expired artifacts.
2. For each: decide renew vs reopen vs deprecate.
3. `forgeplan_renew` to extend `valid_until` (if the decision is still sound).
4. Alternatively, `forgeplan_reopen` to create a lineage-linked draft from an eligible stale active lifecycle artifact.
5. `forgeplan_decay` - see R_eff impact from remaining stale evidence.

## CLI equivalent

```bash
forgeplan stale
```

## See also

- [`forgeplan_decay`](/docs/mcp/forgeplan_decay/) - quantify R_eff loss from aged evidence.
- [`forgeplan_renew`](/docs/cli/renew/) - extend `valid_until`.
- [`forgeplan_reopen`](/docs/cli/reopen/) - create a lineage-linked new draft.
- [Methodology guide](/docs/methodology/overview/)
