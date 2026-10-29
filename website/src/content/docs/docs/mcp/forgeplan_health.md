---
title: forgeplan_health
description: "Show project health dashboard - gaps, risks, blind spots, orphans, stale evidence, and recommended next actions. No LLM needed."
---

Show project health dashboard - gaps, risks, blind spots, orphans, stale evidence, and recommended next actions. This is the single most important MCP tool for an AI agent: call it at session start to understand project state before doing any work.

**Category**: Dashboards & Graph

## When an agent calls it

- **Session start** - first call after `memory_recall` to understand current project state.
- **Before starting new work** - check if there are debts (blind spots, orphans, stale artifacts) that should be fixed first.
- **After a sprint** - verify nothing regressed (new blind spots, orphaned evidence).
- **Before release** - confirm health is clean.

The tool is fast, deterministic, and requires no LLM. Always prefer it over `list` + manual inspection.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

The JSON text payload includes:

- `total`, `by_kind`, `by_status`, `by_derived_status`: inventory and breakdowns.
- `blind_spots`, `at_risk`, `orphans`, `stale_count`: trust/link/expiry findings; `orphans` is an array of IDs.
- `possible_duplicates`, `active_stubs`, `stale_drafts`, `gitignore_drift`: additional findings.
- `phase_mismatches`, `advisory_phase_mismatches`, `phase_read_errors`: advisory phase diagnostics.
- `active_claims`, `active_claim_count`, `skipped_claim_files`: MCP claim information.
- `next_actions`, `_next_action`, `verdict_summary`: suggested follow-up and explanation.
- `verdict`: exactly `"healthy"`, `"needs_attention"`, or `"unhealthy"`; **not** `"debt"`.

Do not branch on the old `total_artifacts`, `risks`, or `stale` example
fields. This dashboard aggregates implemented checks; it does not prove
that all project requirements, handlers, or documentation claims were reviewed.

## Example invocation

```json
{}
```

## Typical sequence

1. `memory_recall("Forgeplan")` - restore context from prior sessions.
2. `forgeplan_health` - get current state.
3. If `verdict != "healthy"`, inspect the actual findings and prioritise them with the task owner; the aggregate label is not permission for unrelated mutations.
4. `forgeplan_route` - route next task once health is clean.

## CLI equivalent

```bash
forgeplan health
```

## See also

- [`forgeplan_status`](/docs/mcp/forgeplan_status/) - simpler kind × status breakdown.
- [`forgeplan_blindspots`](/docs/mcp/forgeplan_blindspots/) - focused blind-spot list.
- [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) - expired `valid_until` list.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Show project health dashboard — gaps, risks, blind spots, orphans, stale evidence, and recommended next actions. Returns aggregate `verdict` field ("healthy" | "needs_attention" | "unhealthy") + `verdict_summary` for one-line human rendering. Branch on `verdict` for CI gates and agent decision logic. No LLM needed.

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
