---
title: forgeplan_decompose
description: "Decompose a PRD into RFC tasks using AI. Analyzes functional requirements and suggests 3-7 RFCs with titles, descriptions, scope, and dependencies. Requires LLM provider."
---

Asks the configured LLM to propose RFC tasks for a PRD. The prompt aims for 3–7 proposals with titles, descriptions, scope, dependencies, and estimated depth, but the response is free-form Markdown, not a validated RFC DAG. Validate the PRD before requesting decomposition and review the generated text before creating artifacts.

**Category**: Reasoning & AI

## When an agent calls this

- After a Standard/Deep PRD passes validation and it's time to plan implementation.
- Sprint planning: user asks "how should we split PRD-042 across the next 3 sprints?".
- After `forgeplan_reason` settles on a direction - decompose translates it into shippable RFCs.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | PRD artifact ID to decompose into RFC tasks. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::DecomposeParams`_

## Returns

The handler returns `prd_id`, `prd_title`, `tasks` (a Markdown **string**), `provider`, and `model`, with workspace trace and a next-action hint. It does not persist the proposed RFCs. The task count and dependency structure are not enforced by a response schema.

Illustrative response excerpt:

```json
{
  "prd_id": "PRD-042",
  "prd_title": "Authentication system",
  "tasks": "1. Token issuer service — cover FR-001 and FR-002.\n2. Session revocation — cover FR-003; depends on the issuer design."
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

- [`forgeplan decompose`](/docs/cli/decompose/) - same operation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) - upstream reasoning step
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - materialize the decomposition
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - connect children to parent PRD

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Decompose a PRD into RFC tasks using AI. Analyzes functional requirements and suggests 3-7 RFCs with titles, descriptions, scope, and dependencies. Requires LLM provider.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | yes | string | PRD artifact ID to decompose into RFC tasks |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "PRD artifact ID to decompose into RFC tasks",
      "type": "string"
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "DecomposeParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
