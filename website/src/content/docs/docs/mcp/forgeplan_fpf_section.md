---
title: forgeplan_fpf_section
description: "Get full content of a specific FPF section by ID (e.g. 'B.3', 'C.2.2', 'A.1')."
---

Fetch the ingested Markdown body of an FPF section by its section ID, for example `B.3` (Trust Calculus) or `B.5` (ADI cycle). The workspace must already contain the auxiliary FPF knowledge base. This returns the imported edition's content, not a guarantee that it matches current upstream FPF.

**Category**: FPF Knowledge Base

## When an agent calls it

- **After `forgeplan_fpf_search`** - read the full text of the top hit.
- **Direct lookup** - when you already know the section ID from prior context or a citation.
- **Navigation** - discover other IDs with `forgeplan_fpf_list` or `forgeplan_fpf_search`; this response has no parent/child links.
- **Context grounding** - pull a specific principle into an ADI reasoning prompt.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | FPF section ID, e.g. `"B.3"`, `"C.2.2"`, `"A.1"`. |
| `workspace` | `string \| null` | no | Explicit absolute or tilde-expanded workspace path. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::FpfSectionParams`_

## Returns

The JSON text payload contains `section_id`, `title`, `body`, `line_count`,
and `_next_action`. It has no `path`, `parent`, `children`, `siblings`,
or `word_count` fields.

An unknown ID returns an MCP tool error with text and a hint to list
available sections, not a JSON `suggestions` array.

## Example invocation

```json
{ "id": "B.3" }
```

## Typical sequence

1. Search with `forgeplan_fpf_search {"query":"trust"}` and inspect actual hits.
2. Pass the returned `section_id` to `forgeplan_fpf_section`.
3. Discover additional IDs through search/list rather than assuming a child exists.
4. Use the retrieved content as grounding, keeping its edition distinct from current upstream claims.

## CLI equivalent

```bash
forgeplan fpf section B.3
```

## See also

- [`forgeplan_fpf_search`](/docs/mcp/forgeplan_fpf_search/) - discover sections by query.
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) - ADI reasoning with FPF context.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Get full content of a specific FPF section by ID (e.g. 'B.3', 'C.2.2', 'A.1').

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | yes | string | FPF section ID (e.g. "B.3", "C.2.2") |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "FPF section ID (e.g. \"B.3\", \"C.2.2\")",
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
  "title": "FpfSectionParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
