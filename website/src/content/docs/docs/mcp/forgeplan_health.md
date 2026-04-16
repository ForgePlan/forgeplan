---
title: forgeplan_health
description: "Show project health dashboard — gaps, risks, blind spots, orphans, stale evidence, and recommended next actions. No LLM needed."
---

Show project health dashboard — gaps, risks, blind spots, orphans, stale evidence, and recommended next actions. This is the single most important MCP tool for an AI agent: call it at session start to understand project state before doing any work.

**Category**: Dashboards & Graph

## When an agent calls it

- **Session start** — first call after `memory_recall` to understand current project state.
- **Before starting new work** — check if there are debts (blind spots, orphans, stale artifacts) that should be fixed first.
- **After a sprint** — verify nothing regressed (new blind spots, orphaned evidence).
- **Before release** — confirm health is clean.

The tool is fast, deterministic, and requires no LLM. Always prefer it over `list` + manual inspection.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

A structured health report with:

- `total_artifacts` — count by kind (PRD/RFC/ADR/Epic/Spec/Evidence/...).
- `by_status` — draft / active / stale / superseded / deprecated breakdown.
- `blind_spots` — active decisions (PRD/RFC/ADR/Epic) with no linked evidence.
- `orphans` — artifacts with zero incoming + outgoing links.
- `stale` — artifacts with expired `valid_until` dates.
- `risks` — decisions with R_eff below a confidence threshold.
- `next_actions` — ordered list of suggested fixes (create evidence, link orphan, refresh stale).
- `verdict` — `"healthy"` or `"debt"` (presence of blind spots / orphans / stale flips to debt).

```json
{
  "verdict": "debt",
  "total_artifacts": 184,
  "blind_spots": [
    { "id": "PRD-042", "kind": "prd", "reason": "active, no evidence" }
  ],
  "orphans": [
    { "id": "NOTE-017", "kind": "note" }
  ],
  "stale": [
    { "id": "ADR-003", "days_expired": 12 }
  ],
  "next_actions": [
    "Create EvidencePack for PRD-042 and link",
    "Link NOTE-017 to parent decision or deprecate",
    "Refresh ADR-003 (expired 2026-03-30)"
  ]
}
```

## Example invocation

```json
{}
```

## Typical sequence

1. `memory_recall("Forgeplan")` — restore context from prior sessions.
2. `forgeplan_health` — get current state.
3. If `verdict == "debt"` → fix blind spots / orphans **first**, before new work.
4. `forgeplan_route` — route next task once health is clean.

## CLI equivalent

```bash
forgeplan health
```

## See also

- [`forgeplan_status`](/docs/mcp/forgeplan_status/) — simpler kind × status breakdown.
- [`forgeplan_blindspots`](/docs/mcp/forgeplan_blindspots/) — focused blind-spot list.
- [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) — expired `valid_until` list.
- [Methodology guide](/docs/methodology/overview/)
