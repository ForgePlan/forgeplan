---
title: forgeplan_decompose
description: "Decompose a PRD into RFC tasks using AI. Analyzes functional requirements and suggests 3-7 RFCs with titles, descriptions, scope, and dependencies. Requires LLM provider."
---

Asks the configured LLM to split a PRD into RFC proposals with titles, descriptions, FR scope, dependencies and estimated depth. The prompt aims for 3–7 proposals; the handler does not validate the PRD's kind, validation status, proposal count or dependency graph. Review the text before materializing it via `forgeplan_new`, `forgeplan_update` and `forgeplan_link`.

**Category**: Reasoning & AI

## When an agent calls this

- After a Standard/Deep PRD passes validation and it's time to plan implementation.
- Sprint planning: user asks "how should we split PRD-042 across the next 3 sprints?".
- After `forgeplan_reason` settles on a direction - decompose translates it into shippable RFCs.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string` | yes | PRD artifact ID to decompose into RFC tasks |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP text content with `prd_id`, `prd_title`, `tasks`, `provider`, `model`, `resolved_workspace`, `resolved_via`, and `_next_action`. **`tasks` is a Markdown string**, not an RFC array or a parsed DAG. The provider is asked for a numbered list; its output is not validated against a task schema.

Illustrative payload excerpt (trace and guidance omitted):

```json
{
  "prd_id": "PRD-042",
  "prd_title": "Login flow",
  "tasks": "1. **Token issuer service**\n   Description: Issue tokens.\n   Scope: FR-001.\n   Dependencies: None.\n   Estimated depth: Standard.",
  "provider": "ollama",
  "model": "configured-model"
}
```

The tool does not create RFCs or links or save the proposal text into the PRD. It resolves the workspace through the write-capable path and calls the LLM; provider errors return `isError` with text and configuration/login guidance. A missing ID also returns an MCP error.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "PRD-001"
}
```

## Typical sequence

Suggested workflow: validate PRD → reason if useful → decompose → review proposals → create each RFC → populate its body → link the new RFC to the PRD with `refines` (using the actual `forgeplan_link` `from`/`to` arguments) → plan implementation. These are separate agent actions, not automatic effects of decomposition.

## CLI equivalent

- [`forgeplan decompose`](/docs/cli/decompose/) - same operation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) - upstream reasoning step
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - materialize the decomposition
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - connect children to parent PRD
