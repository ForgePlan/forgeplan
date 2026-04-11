---
title: forgeplan_journal
description: "Show decision journal — chronological timeline of ADR, Note, Problem, Solution artifacts with R_eff scores and evidence status."
---

Show the decision journal — a chronological timeline of ADR, Note, Problem and Solution artifacts, each annotated with its current `R_eff` score and linked evidence. Use this to reconstruct **why** something was decided and **how strong** that decision currently is.

**Category**: Dashboards & Graph

## When an agent calls it

- **Session start after a long absence** — walk the last N decisions to rebuild context.
- **Before making a related decision** — check whether a prior Note / ADR / Problem already covers the topic.
- **At-risk audit** — call with `risk: true` to surface only decisions with weak or expired evidence.
- **Onboarding** — generate a narrative of the project's decision history.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `kind` | `string` | no | Filter by kind: `adr`, `note`, `problem`, or `solution`. Omit for all. |
| `risk` | `bool` | no | When `true`, show only at-risk decisions (weak R_eff or expired evidence). |

_Schema source: `crates/forgeplan-mcp/src/server.rs::JournalParams`_

## Returns

An ordered list of journal entries, newest first:

```json
{
  "entries": [
    {
      "id": "ADR-005",
      "kind": "adr",
      "title": "Lifecycle v2 — terminal deprecated/superseded",
      "created_at": "2026-04-03",
      "status": "active",
      "r_eff": 0.82,
      "evidence": ["EVID-047", "EVID-051"],
      "at_risk": false
    },
    {
      "id": "PROB-034",
      "kind": "problem",
      "title": "F1/F2 validator penalizes bmad density",
      "created_at": "2026-04-09",
      "r_eff": 0.0,
      "evidence": [],
      "at_risk": true
    }
  ]
}
```

## Example invocation

```json
{ "kind": "adr", "risk": true }
```

## Typical sequence

1. `forgeplan_journal` with `risk: true` — list at-risk decisions.
2. `forgeplan_get` for each entry — read full body.
3. `forgeplan_new` evidence → `forgeplan_link` → `forgeplan_score` — refresh R_eff.

## CLI equivalent

```bash
forgeplan journal --kind adr --risk
```

## See also

- [`forgeplan_blindspots`](/docs/mcp/forgeplan_blindspots/) — decisions with no evidence at all.
- [`forgeplan_decay`](/docs/mcp/forgeplan_decay/) — R_eff degradation from aging evidence.
- [Methodology guide](/docs/methodology/overview/)
