---
title: forgeplan_generate
description: "Generate a full artifact body from a natural-language description using the configured LLM provider."
---

Creates a new artifact **with a fully-authored body** from a natural-language description. Unlike `forgeplan_new` (which produces an empty template stub), `generate` uses the configured LLM (OpenAI / Claude / Gemini / Ollama / any OpenAI-compatible endpoint) to draft all required sections in one call. The agent still needs to run `forgeplan_validate` afterwards because the LLM may miss MUST rules.

**Category**: Creating Artifacts

## When an agent calls this

- User provides a rich description and wants a ready-to-review first draft, not an empty stub.
- Migrating a decision from chat history into a formal ADR — feed the chat summary as description.
- Bulk bootstrap: turning an informal roadmap into 5 draft PRDs in one session.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `kind` | `string` | yes | Artifact kind: `prd`, `epic`, `spec`, `rfc`, `adr`, `problem`, `solution`, `evidence`. |
| `description` | `string` | yes | Natural language description of what to generate. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::GenerateParams`_

## Returns

The new artifact ID plus the generated body. Unlike `forgeplan_new`, the agent can usually go straight to `forgeplan_validate` without an intermediate `forgeplan_update` — the body is already populated.

Example response shape:

```json
{
  "id": "PRD-043",
  "kind": "prd",
  "status": "draft",
  "path": ".forgeplan/prds/prd-043-oauth2-login.md",
  "body": "# PRD-043: OAuth2 login flow\n\n## Problem\n...",
  "llm": { "provider": "gemini", "model": "gemini-3-flash-preview", "tokens": 1847 }
}
```

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

`forgeplan_route` (confirm depth) → `forgeplan_search` (dup check) → `forgeplan_generate` → `forgeplan_validate` → (fix any gaps via `forgeplan_update`) → `forgeplan_reason` (for Standard+) → `forgeplan_activate`.

## CLI equivalent

- [`forgeplan generate`](/docs/cli/generate/) — same operation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) — empty-stub alternative
- [`forgeplan_capture`](/docs/mcp/forgeplan_capture/) — capture decisions from conversation
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) — always validate generated output
