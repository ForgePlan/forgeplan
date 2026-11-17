---
title: forgeplan_fpf_section
description: "Get full content of a specific FPF section by ID (e.g. 'B.3', 'C.2.2', 'A.1')."
---

Fetches the full Markdown content of a loaded FPF section by its section ID, such as `B.3`, `C.2.2` or `A.1`. Discover the IDs available in the workspace with `forgeplan_fpf_list` or `forgeplan_fpf_search`.

**Category**: FPF Knowledge Base

## When an agent calls it

- **After `forgeplan_fpf_search`** - read the full text of the top hit.
- **Direct lookup** - when you already know the section ID from prior context or a citation.
- **Navigation** - search or list related section IDs; this endpoint does not return a parent/child tree.
- **Context grounding** - pull a specific principle into an ADI reasoning prompt.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string` | yes | FPF section ID (e.g. "B.3", "C.2.2") |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP text content with `section_id`, `title`, `body` and `line_count`, plus `_next_action`. There are no `parent`, `children`, `siblings`, `path` or `word_count` fields. The ID is matched exactly against a loaded section; this does not recursively concatenate child sections.

An unknown section returns `isError` with plain-text content and a hint to list available sections; the hint is text, not a JSON `_next_action` property. Store failures also return errors. If the knowledge base is empty, ingest it through the CLI before retrying. Reading a section has no artifact mutation or automatic ADI-seeding effect.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "B.3"
}
```

## Typical sequence

1. `forgeplan_fpf_search` - find relevant loaded sections.
2. `forgeplan_fpf_section` with a returned section ID as `id` - read the full body.
3. Follow actual references in that body or search related concepts.
4. Use the text in the agent's reasoning; the MCP `forgeplan_reason` call has no FPF-seeding argument.

## CLI equivalent

```bash
forgeplan fpf section B.3
```

## See also

- [`forgeplan_fpf_search`](/docs/mcp/forgeplan_fpf_search/) - discover sections by query.
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) - separate ADI analysis; MCP has no FPF-seeding input.
- [Methodology guide](/docs/methodology/overview/)
