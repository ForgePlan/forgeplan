---
title: forgeplan_new
description: "Create canonical artifact Markdown from a template and report identity and index publication state."
---

Creates canonical artifact Markdown from the built-in template for its kind.
The returned identity is a reference for subsequent operations, not a
`constellation-read` scope or snapshot handle.

**Category**: Creating Artifacts

## When an agent calls this

- After `forgeplan_route` returns `Depth: Standard, Pipeline: PRD → RFC` and the agent needs the PRD stub.
- When the agent is told to "create an ADR for X decision we just made" and wants a skeleton to fill in.
- When decomposing work - after `forgeplan_decompose` suggests RFCs, the agent may call `new` once per suggested RFC.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `kind` | `string` | yes | Artifact kind: `prd`, `epic`, `spec`, `rfc`, `adr`, `problem`, `solution`, `evidence`, `note`, `refresh`. |
| `title` | `string` | yes | Artifact title. |
| `parent_id` | `string?` | no | Evidence only: display ID or slug of the parent to auto-link with `informs`. Other kinds reject it. |
| `workspace` | `string?` | no | Absolute or `~/`-expanded workspace path; overrides env/server default/CWD discovery. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::NewParams`_

## Returns

The MCP result contains JSON text in `content[]` (`type: "text"`).
Parse that JSON payload; it is not a top-level JSON-RPC artifact object.
The source-defined success fields are:

| Fields | Meaning |
|---|---|
| `id`, `kind`, `title`, `filepath` | Created identity and canonical Markdown location |
| `slug`, `predicted_number`, `assigned_number` | Optional identity fields; absent values can be omitted |
| `id_canonical`, `id_display`, `hint` | Canonical/display references and usage guidance |
| `_next_action`, `warnings` | Workflow hint and duplicate warnings |
| `auto_linked`, `auto_link_warnings` | Evidence parent-link outcome when applicable |
| `resolved_workspace`, `resolved_via` | Workspace actually used and resolution source |
| `index_pending` | Whether derived publication is still unacknowledged |
| `index_pending_reason`, `index_generation` | Conditional publication reason/generation |

It does **not** return `body`, `status`, or `path` as the old example did.
Read with `forgeplan_get`, then fill required sections using `forgeplan_update`.
`index_pending: true` does not mean the canonical write was rolled back.

This is the full-profile authoring tool; it is not part of the separate
`constellation-read` namespace. See [Constellation](/docs/guides/constellation/).

## Example invocation

```json
{ "kind": "prd", "title": "Authentication system", "workspace": "/path/to/project" }
```

With typical agent context:

> Router returned `Depth: Standard, Pipeline: PRD → RFC`. Agent creates the PRD stub before touching code.

```json
{ "kind": "prd", "title": "Rate limit auth endpoints" }
```

## Typical sequence

`forgeplan_route` → `forgeplan_new` → `forgeplan_get` → `forgeplan_update`
→ `forgeplan_validate` → reasoning / implementation / evidence as appropriate.
This describes workflow intent, not proof that all lifecycle tools support
a migrated workspace. Full MCP validation has been exercised in the local
build; do not infer other tools' migration support from registration alone.

## CLI equivalent

- [`forgeplan new`](/docs/cli/new/) - same operation with interactive prompts

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_generate`](/docs/mcp/forgeplan_generate/) - LLM-authored body instead of a stub
- [`forgeplan_update`](/docs/mcp/forgeplan_update/) - fill the stub
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - confirm completeness

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Create a new artifact from template. Generates a sequential ID (e.g., PRD-001), renders the template, and writes the canonical markdown projection. On a workspace whose index is owned by the Constellation protocol the derived row is published by the reconcile pass reported as `index_pending` in the response; elsewhere it also takes the legacy projection.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `kind` | yes | ArtifactKindArg | Artifact kind to create |
| `parent_id` | no | string \| null | Issue #295: optional parent artifact ID for auto-link.<br>When supplied AND kind=evidence, an `informs` link is created<br>from the new EVID to the named parent in the same call.<br>Reduces the 3-step EVID flow (new → update → link) to 2 steps<br>(new+autolink → update).<br>Accepts display id (`PRD-074`) or slug (`prd-auth-system`).<br>For kinds other than `evidence` the parameter is rejected with<br>a structured error (other auto-link patterns are intentionally<br>out of scope for v1 — see issue #295). |
| `title` | yes | string | Artifact title |
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
    "kind": {
      "$ref": "#/$defs/ArtifactKindArg",
      "description": "Artifact kind to create"
    },
    "parent_id": {
      "default": null,
      "description": "Issue #295: optional parent artifact ID for auto-link.\nWhen supplied AND kind=evidence, an `informs` link is created\nfrom the new EVID to the named parent in the same call.\nReduces the 3-step EVID flow (new → update → link) to 2 steps\n(new+autolink → update).\nAccepts display id (`PRD-074`) or slug (`prd-auth-system`).\nFor kinds other than `evidence` the parameter is rejected with\na structured error (other auto-link patterns are intentionally\nout of scope for v1 — see issue #295).",
      "type": [
        "string",
        "null"
      ]
    },
    "title": {
      "description": "Artifact title",
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
    "kind",
    "title"
  ],
  "title": "NewParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
