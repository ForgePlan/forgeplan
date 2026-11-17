---
title: forgeplan_generate
description: "Generate an artifact using AI from a natural language description. Requires an LLM provider in .forgeplan/config.yaml. Supports OpenAI-compatible HTTP providers, Claude, Gemini, Ollama, and explicit subscription-authenticated local CLI providers."
---

Creates a new artifact **with a fully-authored body** from a natural-language description. Unlike `forgeplan_new` (which produces an empty template stub), `generate` uses the configured LLM (OpenAI / Claude / Gemini / Ollama / any OpenAI-compatible endpoint) to draft all required sections in one call. The agent still needs to run `forgeplan_validate` afterwards because the LLM may miss MUST rules.

**Category**: Creating Artifacts

## When an agent calls this

- User provides a rich description and wants a ready-to-review first draft, not an empty stub.
- Migrating a decision from chat history into a formal ADR - feed the chat summary as description.
- Bulk bootstrap: turning an informal roadmap into 5 draft PRDs in one session.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `description` | `string` | yes | Natural language description of what to generate |
| `kind` | `string` | yes | Artifact kind to generate via LLM Enum: `"prd"`, `"epic"`, `"spec"`, `"rfc"`, `"adr"`, `"problem"`, `"solution"`, `"evidence"`, `"note"`, `"refresh"`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP text content with `id`, `kind`, `title`, `filepath`, `provider`, `model`, `resolved_workspace`, `resolved_via`, and `_next_action`. It does **not** include the generated `body`, `status`, `path`, token counts, or a nested `llm` object. Read the saved body with `forgeplan_get` before validation.

Illustrative payload excerpt (trace and guidance omitted):

```json
{
  "id": "PRD-043",
  "kind": "prd",
  "title": "OAuth2 login flow",
  "filepath": "/workspace/.forgeplan/prds/prd-043-oauth2-login-flow.md",
  "provider": "ollama",
  "model": "configured-model"
}
```

The artifact is persisted as `draft` at `standard` depth. Its title is the first description line, truncated to 80 characters; the handler writes the Markdown projection before syncing the store. LLM configuration uses environment overrides and may select an explicitly configured local CLI provider. Provider failures return an MCP `isError` response with text and configuration/login guidance; generation has no template-only success fallback.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "kind": "prd",
  "description": "OAuth2 login flow"
}
```

## Typical sequence

Suggested workflow: route → search for duplicates → generate → read with `forgeplan_get` → validate → fix gaps → reason if the team's process requires it → review and activate. Generation does not automatically perform those later steps.

## CLI equivalent

- [`forgeplan generate`](/docs/cli/generate/) - same operation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - empty-stub alternative
- [`forgeplan_capture`](/docs/mcp/forgeplan_capture/) - capture decisions from conversation
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - always validate generated output
