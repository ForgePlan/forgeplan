---
title: forgeplan_blindspots
description: "Show blind spots — decisions (PRD/RFC/ADR/Epic) without linked evidence, and orphan artifacts with no connections."
---

Show blind spots in the workspace — active decisions (PRD, RFC, ADR, Epic) that have no linked evidence, plus orphan artifacts with zero incoming or outgoing links. Blind spots are the #1 technical debt signal in Forgeplan: an `active` PRD with no evidence is a false promise.

**Category**: Dashboards & Graph

## When an agent calls it

- **After session start** — if `forgeplan_health` reports `verdict: debt`, call this for the detailed list.
- **Before merging a PR** — make sure new work doesn't leave new blind spots.
- **During refactoring** — find orphan Notes / Problems that should be deprecated or re-linked.
- **Quality sweeps** — produce a fix list sortable by owner / kind.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

```json
{
  "blind_spots": [
    {
      "id": "PRD-042",
      "kind": "prd",
      "status": "active",
      "reason": "no linked evidence",
      "r_eff": 0.0
    },
    {
      "id": "RFC-006",
      "kind": "rfc",
      "status": "active",
      "reason": "no linked evidence"
    }
  ],
  "orphans": [
    {
      "id": "NOTE-017",
      "kind": "note",
      "reason": "no incoming or outgoing links"
    }
  ],
  "summary": {
    "blind_spots_count": 2,
    "orphans_count": 1
  }
}
```

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_blindspots` — list offenders.
2. For each blind spot: `forgeplan_new` evidence → `forgeplan_link` → `forgeplan_score`.
3. For each orphan: either `forgeplan_link` to a parent, or `forgeplan_deprecate`.
4. Re-run `forgeplan_health` → expect `verdict: healthy`.

## CLI equivalent

```bash
forgeplan blindspots
```

## See also

- [`forgeplan_health`](/docs/mcp/forgeplan_health/) — full health report.
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) — create an EvidencePack.
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) — attach evidence to a decision.
- [Methodology guide](/docs/methodology/overview/)
