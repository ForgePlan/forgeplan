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

This report uses the **flat** evidence minimum, not the recursive engine used by `forgeplan_score`: no dependency cascade or evidence-type penalty is included. Expiry overrides an evidence score to 0.1 without changing CL or deleting the pack. Terminal evidence is excluded; draft evidence counts. The report lists non-evidence artifacts with linked expired eligible evidence, even when the hypothetical fresh score is equal or lower.

## Input parameters

_No declared input fields. Call this tool with an empty object `{}`._

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP `content[0].text` with `entries`, `total_affected`, and `_next_action`. Each entry has `artifact_id`, `artifact_title`, `current_r_eff`, `fresh_r_eff`, `r_eff_drop` (`fresh - current`), and `expired_evidence`. Each expired item has `id`, `valid_until`, `days_expired`, `score` (normally 0.1). There is no `decayed`, `delta`, `kind`, or `total_impacted`.

Illustrative partial payload (dynamic `_next_action` omitted):

```json
{
  "entries": [
    {
      "artifact_id": "PRD-039",
      "artifact_title": "Auth system",
      "current_r_eff": 0.1,
      "fresh_r_eff": 0.9,
      "r_eff_drop": 0.8,
      "expired_evidence": [
        {"id": "EVID-047", "valid_until": "2026-03-30", "days_expired": 12, "score": 0.1}
      ]
    }
  ],
  "total_affected": 1
}
```

Values assume one supporting CL2 pack and UTC date 2026-04-11. This is a report only: it does not renew evidence, update scores, or mutate status. Workspace errors are plain-text tool errors; report failures can be JSON-RPC errors. Failed relation lookups degrade to empty relations and can omit affected entries.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_stale` - find expired artifacts.
2. `forgeplan_decay` - see which decisions they damaged.
3. Read `r_eff_drop` (already sorted descending) — largest positive drops are highest-impact refresh candidates.
4. `forgeplan_renew` or create new `EvidencePack` → `forgeplan_link`.
5. `forgeplan_score` - confirm R_eff rebounded.

## CLI equivalent

```bash
forgeplan decay
```

## See also

- [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) - list of expired artifacts.
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - recompute R_eff for a specific artifact.
- [`forgeplan_journal`](/docs/mcp/forgeplan_journal/) - chronological view with R_eff.
- [Methodology guide](/docs/methodology/overview/)
