---
title: forgeplan_decay
description: "Show evidence decay impact on R_eff scores. Lists artifacts where expired evidence has degraded quality scores, with current vs fresh R_eff comparison."
---

Show how aged / expired evidence has degraded R_eff scores across the workspace. For each affected decision, it reports the **current** R_eff (with decay penalties applied) vs a hypothetical **fresh** R_eff (if the expired evidence were renewed). This quantifies the "silent quality loss" caused by evidence aging.

**Category**: Quality

## When an agent calls it

- **Quality audit** — see which decisions are silently losing trust from old evidence.
- **Prioritising refresh work** — the biggest current-vs-fresh deltas are the highest-impact renewals.
- **Before a release** — make sure shipped decisions aren't riding on decayed evidence.
- **Explaining R_eff drops** to the user ("why did PRD-039 score go from 0.85 to 0.41?").

Decay follows the R_eff rule: expired evidence is not removed — it drops to CL0 with weight 0.1, so the weakest link in the chain collapses the overall score.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

```json
{
  "decayed": [
    {
      "id": "PRD-039",
      "kind": "prd",
      "r_eff_current": 0.41,
      "r_eff_fresh": 0.85,
      "delta": -0.44,
      "expired_evidence": [
        { "id": "EVID-047", "days_expired": 12 }
      ]
    }
  ],
  "total_impacted": 1
}
```

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_stale` — find expired artifacts.
2. `forgeplan_decay` — see which decisions they damaged.
3. Sort by `delta` descending — highest-impact first.
4. `forgeplan_renew` or create new `EvidencePack` → `forgeplan_link`.
5. `forgeplan_score` — confirm R_eff rebounded.

## CLI equivalent

```bash
forgeplan decay
```

## See also

- [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) — list of expired artifacts.
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) — recompute R_eff for a specific artifact.
- [`forgeplan_journal`](/docs/mcp/forgeplan_journal/) — chronological view with R_eff.
- [Methodology guide](/docs/methodology/overview/)
