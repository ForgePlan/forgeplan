---
title: forgeplan_new
description: "Create a new artifact from template. Generates a sequential ID (e.g., PRD-001), renders the template, stores in LanceDB, and writes a markdown projection."
---

Creates a new artifact stub from the built-in template for its kind. The agent calls this when it has decided (usually via `forgeplan_route`) what kind of artifact is needed next — typically the moment after the human request is classified as Standard or deeper. The returned ID is the handle the agent uses for every subsequent operation.

**Category**: Creating Artifacts

## When an agent calls this

- After `forgeplan_route` returns `Depth: Standard, Pipeline: PRD → RFC` and the agent needs the PRD stub.
- When the agent is told to "create an ADR for X decision we just made" and wants a skeleton to fill in.
- When decomposing work — after `forgeplan_decompose` suggests RFCs, the agent may call `new` once per suggested RFC.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `kind` | `string` | yes | Artifact kind: `prd`, `epic`, `spec`, `rfc`, `adr`, `problem`, `solution`, `evidence`, `note`, `refresh`. |
| `title` | `string` | yes | Artifact title. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::NewParams`_

## Returns

The generated ID plus the rendered body. The agent should immediately fill MUST sections via `forgeplan_update` rather than leave a stub — unpopulated PRDs count as blind spots in `forgeplan_health`.

Example response shape:

```json
{
  "id": "PRD-042",
  "kind": "prd",
  "status": "draft",
  "path": ".forgeplan/prds/prd-042-authentication-system.md",
  "body": "# PRD-042: Authentication system\n\n## Problem\n..."
}
```

## Example invocation

```json
{ "kind": "prd", "title": "Authentication system" }
```

With typical agent context:

> Router returned `Depth: Standard, Pipeline: PRD → RFC`. Agent creates the PRD stub before touching code.

```json
{ "kind": "prd", "title": "Rate limit auth endpoints" }
```

## Typical sequence

`forgeplan_route` → `forgeplan_new` → `forgeplan_update` (fill MUST sections) → `forgeplan_validate` (PASS) → `forgeplan_reason` (ADI for Standard+) → code → evidence → `forgeplan_activate`.

## CLI equivalent

- [`forgeplan new`](/docs/cli/new/) — same operation with interactive prompts

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_generate`](/docs/mcp/forgeplan_generate/) — LLM-authored body instead of a stub
- [`forgeplan_update`](/docs/mcp/forgeplan_update/) — fill the stub
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) — confirm completeness
