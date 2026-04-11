---
title: forgeplan_decompose
description: "Decompose a PRD into RFC tasks using AI. Analyzes functional requirements and suggests 3-7 RFCs with titles, descriptions, scope, and dependencies. Requires LLM provider."
---

Takes a validated PRD and produces a breakdown into 3–7 RFCs, each with title, scope, FR mapping, and dependencies. The agent calls this when a PRD is too big to implement in one sprint and needs to be split — decompose returns a draft RFC DAG the agent can then materialize via `forgeplan_new` + `forgeplan_update` + `forgeplan_link`.

**Category**: Reasoning & AI

## When an agent calls this

- After a Standard/Deep PRD passes validation and it's time to plan implementation.
- Sprint planning: user asks "how should we split PRD-042 across the next 3 sprints?".
- After `forgeplan_reason` settles on a direction — decompose translates it into shippable RFCs.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | PRD artifact ID to decompose into RFC tasks. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::DecomposeParams`_

## Returns

A list of proposed RFCs with titles, scope summaries, covered FR IDs, and a dependency DAG expressed as `depends_on` arrays. The response is a **draft** — nothing is persisted until the agent explicitly creates artifacts via `forgeplan_new`.

Example response shape:

```json
{
  "source": "PRD-042",
  "rfcs": [
    {
      "title": "Token issuer service",
      "scope": "Signs JWTs, rotates keys, exposes /token endpoint.",
      "covers": ["FR-001", "FR-002"],
      "depends_on": []
    },
    {
      "title": "Session blacklist store",
      "scope": "Redis-backed revocation list with TTL.",
      "covers": ["FR-003"],
      "depends_on": ["Token issuer service"]
    }
  ]
}
```

## Example invocation

```json
{ "id": "PRD-001" }
```

With typical agent context:

> PRD-042 is too big for one sprint. Agent asks decompose for a breakdown, then creates the resulting RFC stubs.

```json
{ "id": "PRD-042" }
```

## Typical sequence

`forgeplan_validate` PASS → `forgeplan_reason` → `forgeplan_decompose` → loop: `forgeplan_new rfc` + `forgeplan_update` + `forgeplan_link relation=based_on target=PRD-042` for each suggested RFC → sprint planning.

## CLI equivalent

- [`forgeplan decompose`](/docs/cli/decompose/) — same operation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) — upstream reasoning step
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) — materialize the decomposition
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) — connect children to parent PRD
