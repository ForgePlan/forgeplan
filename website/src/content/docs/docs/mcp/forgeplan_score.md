---
title: forgeplan_score
description: "Compute R_eff quality score for an artifact based on linked evidence. R_eff uses the weakest-link principle: score = min(evidence_scores)."
---

Computes the R_eff (effective reliability) score for an artifact, derived from its linked EvidencePacks. R_eff follows the weakest-link principle — it's the minimum over evidence scores, never an average — so one CL0 EvidencePack can drag the whole score to 0.1. Agents use this to answer "is this decision trustworthy enough to ship on?" and to surface PRDs that need more evidence.

**Category**: Quality & Validation

## When an agent calls this

- After creating and linking an EvidencePack — "did R_eff actually move above zero?"
- As part of `forgeplan_health` deep-dive: which active artifacts still have R_eff = 0?
- Before proposing `forgeplan_activate` for a Deep/Critical artifact where evidence is mandatory.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to score. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ScoreParams`_

## Returns

The computed R_eff plus a breakdown per contributing EvidencePack, showing each evidence's verdict, congruence level, evidence type, and decay factor. If no evidence is linked, returns `r_eff: 0.0` with an explanation — this is the "blind spot" signal.

Example response shape:

```json
{
  "id": "PRD-042",
  "r_eff": 0.87,
  "limiting_evidence": "EVID-056",
  "breakdown": [
    { "id": "EVID-057", "verdict": "supports", "cl": 3, "score": 1.0 },
    { "id": "EVID-056", "verdict": "supports", "cl": 2, "score": 0.87 }
  ]
}
```

## Example invocation

```json
{ "id": "PRD-001" }
```

With typical agent context:

> Agent just linked EVID-057 to PRD-042 and wants to verify R_eff moved from 0.00 to a real number.

```json
{ "id": "PRD-042" }
```

## Typical sequence

`forgeplan_new evidence` → `forgeplan_update` (structured fields: verdict/CL/evidence_type) → `forgeplan_link` → `forgeplan_score` → if > 0 → `forgeplan_activate`. For stale detection, score is also called after `forgeplan_decay` to see which artifacts dropped below threshold.

## CLI equivalent

- [`forgeplan score`](/docs/cli/score/) — same computation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) — how evidence is attached
- [`forgeplan_decay`](/docs/mcp/forgeplan_decay/) — evidence TTL management
- [`forgeplan_health`](/docs/mcp/forgeplan_health/) — aggregated R_eff dashboard
