---
title: forgeplan_blindspots
description: "Show blind spots - decisions (PRD/RFC/ADR/Epic) without linked evidence, and orphan artifacts with no connections."
---

Show blind spots in the workspace - active decisions (PRD, RFC, ADR, Epic) that have no linked evidence, plus orphan artifacts with zero incoming or outgoing links. Blind spots are the #1 technical debt signal in Forgeplan: an `active` PRD with no evidence is a false promise.

**Category**: Dashboards & Graph

## When an agent calls it

- **After session start** - if `forgeplan_health` reports blind spots or orphans, call this for the detailed list. Its health verdict is not `debt`.
- **Before merging a PR** - make sure new work doesn't leave new blind spots.
- **During refactoring** - find orphan Notes / Problems that should be deprecated or re-linked.
- **Quality sweeps** - produce a fix list sortable by owner / kind.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

Illustrative response excerpt, not a recorded workspace measurement. The handler returns blind-spot objects with `id`, `title`, and `issue`; `orphans` contains ID strings.

```json
{
  "blind_spots": [
    {
      "id": "PRD-042",
      "title": "Authentication system",
      "issue": "No linked evidence — decision without proof"
    }
  ],
  "orphans": ["NOTE-017"],
  "total_blind_spots": 1,
  "total_orphans": 1
}
```

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_blindspots` - list offenders.
2. For each blind spot: `forgeplan_new` evidence → `forgeplan_link` → `forgeplan_score`.
3. For each orphan: either `forgeplan_link` to a parent, or `forgeplan_deprecate`.
4. Re-run `forgeplan_health` and inspect all remaining warning classes. Resolving these two lists alone does not guarantee `verdict: healthy`.

## CLI equivalent

```bash
forgeplan blindspots
```

## See also

- [`forgeplan_health`](/docs/mcp/forgeplan_health/) - full health report.
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - create an EvidencePack.
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - attach evidence to a decision.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Show blind spots — decisions (PRD/RFC/ADR/Epic) without linked evidence, and orphan artifacts with no connections.

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
