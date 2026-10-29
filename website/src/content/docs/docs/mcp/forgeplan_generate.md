---
title: forgeplan_generate
description: "Generate a full artifact body from a natural-language description using the configured LLM provider."
---

Creates a new draft with an LLM-generated body from a natural-language description, using the configured HTTP or supported local CLI provider. This writes canonical Markdown; generation is not evidence that required sections are complete or correct. Read the result with `forgeplan_get`, review and edit it, then validate.

**Category**: Creating Artifacts

## When an agent calls this

- User provides a rich description and wants a ready-to-review first draft, not an empty stub.
- Migrating a decision from chat history into a formal ADR - feed the chat summary as description.
- Bulk bootstrap: turning an informal roadmap into 5 draft PRDs in one session.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `kind` | `string` | yes | Artifact kind: `prd`, `epic`, `spec`, `rfc`, `adr`, `problem`, `solution`, `evidence`, `note`, `refresh`. |
| `description` | `string` | yes | Natural language description of what to generate. |
| `workspace` | `string \| null` | no | Explicit absolute or tilde-expanded workspace path. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::GenerateParams`_

## Returns

The parsed MCP payload contains `id`, `kind`, `title`, `filepath`,
`provider`, `model`, and `_next_action`, plus workspace-resolution and
index-publication fields. It does not return `body`, `status`, a nested
`llm` object, or token-usage measurements. Fetch the created body through
`forgeplan_get`. For a migrated workspace, inspect `index_pending` rather
than assuming the canonical write and derived publication completed together.

## Example invocation

```json
{ "kind": "prd", "description": "OAuth2 login flow" }
```

With typical agent context:

> User pastes a one-paragraph feature description. Agent generates a full PRD draft instead of a blank stub.

```json
{ "kind": "prd", "description": "Add OAuth2 login with Google and GitHub, support PKCE, 15m token TTL." }
```

## Typical sequence

`forgeplan_route` → `forgeplan_search` (duplicate check) → `forgeplan_generate` → `forgeplan_get` → review/edit → `forgeplan_validate` → reasoning as required by the routed depth → implementation and evidence → review → `forgeplan_activate`. Generation alone is not an activation prerequisite substitute.

## CLI equivalent

- [`forgeplan generate`](/docs/cli/generate/) - same operation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - empty-stub alternative
- [`forgeplan_capture`](/docs/mcp/forgeplan_capture/) - capture decisions from conversation
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - always validate generated output

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Generate an artifact using AI from a natural language description. Requires an LLM provider in .forgeplan/config.yaml. Supports OpenAI-compatible HTTP providers, Claude, Gemini, Ollama, and explicit subscription-authenticated local CLI providers.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `description` | yes | string | Natural language description of what to generate |
| `kind` | yes | ArtifactKindArg | Artifact kind to generate via LLM |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "ArtifactKindArg": {
      "enum": [
        "prd",
        "epic",
        "spec",
        "rfc",
        "adr",
        "problem",
        "solution",
        "evidence",
        "note",
        "refresh"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "description": {
      "description": "Natural language description of what to generate",
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/ArtifactKindArg",
      "description": "Artifact kind to generate via LLM"
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
    "kind",
    "description"
  ],
  "title": "GenerateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
