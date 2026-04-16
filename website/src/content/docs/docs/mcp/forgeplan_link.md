---
title: forgeplan_link
description: "Link two artifacts with a typed relationship. Valid types: informs, based_on, supersedes, contradicts, refines."
---

Creates a typed relationship between two artifacts. Links are how Forgeplan builds its dependency graph — they drive health reports, R_eff scoring, topological ordering, and visual graphs. The agent calls this every time it creates supporting evidence, a replacement RFC, or a child PRD that inherits from an Epic.

**Category**: Editing Artifacts

## When an agent calls this

- After creating an EvidencePack: link `EVID-XXX` → `PRD-YYY` with relation `informs` so R_eff can compute.
- When decomposing an Epic: link each new PRD back to the Epic with `based_on`.
- When replacing a design: link the new RFC to the old one with `supersedes` (complementary to `forgeplan_supersede`).

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `source` | `string` | yes | Source artifact ID. |
| `target` | `string` | yes | Target artifact ID. |
| `relation` | `string` | no (default: `"informs"`) | Relationship type: `informs`, `based_on`, `supersedes`, `contradicts`, `refines`. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::LinkParams`_

## Returns

A confirmation with the persisted edge. The graph is updated immediately and will show up in the next `forgeplan_graph` / `forgeplan_health` / `forgeplan_score` call.

Example response shape:

```json
{
  "ok": true,
  "source": "EVID-057",
  "target": "PRD-042",
  "relation": "informs"
}
```

## Example invocation

```json
{ "source": "EVID-001", "target": "PRD-001", "relation": "informs" }
```

With typical agent context:

> Agent finished implementation, created EVID-057 with benchmark results, and now links it to the PRD so the score can turn green.

```json
{ "source": "EVID-057", "target": "PRD-042", "relation": "informs" }
```

## Typical sequence

`forgeplan_new` (evidence) → `forgeplan_update` (structured fields) → `forgeplan_link` → `forgeplan_score` (now > 0) → `forgeplan_activate`. For Epic decomposition: `forgeplan_new` (epic) → `forgeplan_new` (prd) → `forgeplan_link relation=based_on` → repeat.

## CLI equivalent

- [`forgeplan link`](/docs/cli/link/) — same operation, positional args

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) — consumes link graph for R_eff
- [`forgeplan_graph`](/docs/mcp/forgeplan_graph/) — visualize the link graph
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) — stateful version of a supersedes link
