---
title: forgeplan_blindspots
description: "Show blind spots - decisions (PRD/RFC/ADR/Epic) without linked evidence, and orphan artifacts with no connections."
---

Show decisions without linked evidence and artifacts without incoming or outgoing relations. Evidence-required kinds are PRD, RFC, ADR, Epic, Spec, Problem, and Solution; draft, deprecated, and superseded decisions are excluded from the blind-spot check. This is a heuristic worklist, not an assessment of whether a decision is true.

**Category**: Dashboards & Graph

## When an agent calls it

- **After session start** - if `forgeplan_health` reports missing evidence or orphans under `needs_attention` / `unhealthy`, inspect the detailed list.
- **Before merging a PR** - make sure new work doesn't leave new blind spots.
- **During refactoring** - find orphan Notes / Problems that should be deprecated or re-linked.
- **Quality sweeps** — fetch IDs for a remediation list; use `forgeplan_get` for owner/kind details absent from this report.

## Input parameters

_No declared input fields. Call this tool with an empty object `{}`._

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP `content[0].text` with `blind_spots`, `orphans`, `total_blind_spots`, `total_orphans`, and `_next_action`. Blind spots contain `id`, `title`, `issue`; orphans are ID strings, not objects. There is no nested `summary`, per-item `kind`, `status`, `reason`, or score.

Illustrative response:

```json
{
  "blind_spots": [
    {"id": "PRD-042", "title": "Auth system", "issue": "No linked evidence — decision without proof"}
  ],
  "orphans": ["PRD-042"],
  "total_blind_spots": 1,
  "total_orphans": 1,
  "_next_action": "1 blind spot(s). Score `PRD-042` first: `forgeplan_score PRD-042`."
}
```

The call does not create evidence, add links, deprecate artifacts, or repair the workspace. It reads the server's default workspace (no `workspace` input). Workspace errors return plain-text tool errors; report failures can be JSON-RPC errors. A clean blind-spot report does not imply every other health signal is clean.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_blindspots` - list offenders.
2. For each blind spot: gather real evidence → `forgeplan_new` evidence → `forgeplan_update` with `verdict`, `congruence_level` and `evidence_type` → `forgeplan_link` → `forgeplan_score`.
3. For each orphan: either `forgeplan_link` to a parent, or `forgeplan_deprecate`.
4. Re-run `forgeplan_health` and inspect all remaining findings; fixing these two categories alone does not guarantee `healthy`.

## CLI equivalent

```bash
forgeplan blindspots
```

## See also

- [`forgeplan_health`](/docs/mcp/forgeplan_health/) - full health report.
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - create an EvidencePack.
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - attach evidence to a decision.
- [Methodology guide](/docs/methodology/overview/)
