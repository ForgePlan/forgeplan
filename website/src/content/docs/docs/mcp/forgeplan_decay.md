---
title: forgeplan_decay
description: "Show evidence decay impact on R_eff scores. Lists artifacts where expired evidence has degraded quality scores, with current vs fresh R_eff comparison."
---

Show how aged / expired evidence has degraded R_eff scores across the workspace. For each affected decision, it reports the **current** R_eff (with decay penalties applied) vs a hypothetical **fresh** R_eff (if the expired evidence were renewed). This quantifies the "silent quality loss" caused by evidence aging.

**Category**: Quality

## When an agent calls it

- **Quality audit** - see which decisions are silently losing trust from old evidence.
- **Prioritising refresh work** - the biggest current-vs-fresh deltas are the highest-impact renewals.
- **Before a release** - make sure shipped decisions aren't riding on decayed evidence.
- **Explaining R_eff drops** to the user ("why did PRD-039 score go from 0.85 to 0.41?").

Expiry does not rewrite the stored congruence level. The scoring implementation assigns expired eligible evidence a score of 0.1; terminal (`superseded`/`deprecated`) packs are excluded from the current-evidence minimum.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

Illustrative response excerpt, not a measured score change. `r_eff_drop` is `fresh_r_eff - current_r_eff`.

```json
{
  "entries": [
    {
      "artifact_id": "PRD-039",
      "artifact_title": "Authentication system",
      "current_r_eff": 0.1,
      "fresh_r_eff": 1.0,
      "r_eff_drop": 0.9,
      "expired_evidence": [
        { "id": "EVID-047", "valid_until": "2026-03-30", "days_expired": 12, "score": 0.1 }
      ]
    }
  ],
  "total_affected": 1
}
```

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_stale` - find expired artifacts.
2. `forgeplan_decay` - see which decisions they damaged.
3. Sort by `r_eff_drop` descending - highest-impact first.
4. Re-check the evidence before extending validity with CLI `forgeplan renew <ID> --reason <REASON> --until <YYYY-MM-DD>`, or create and link a new EvidencePack. There is no registered `forgeplan_renew` MCP tool.
5. `forgeplan_score` - inspect the new score; renewal does not guarantee an increase if another weak link remains.

## CLI equivalent

```bash
forgeplan decay
```

## See also

- [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) - list of expired artifacts.
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - recompute R_eff for a specific artifact.
- [`forgeplan_journal`](/docs/mcp/forgeplan_journal/) - chronological view with R_eff.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Show evidence decay impact on R_eff scores. Lists artifacts where expired evidence has degraded quality scores, with current vs fresh R_eff comparison.

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
