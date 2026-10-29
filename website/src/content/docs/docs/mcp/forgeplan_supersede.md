---
title: forgeplan_supersede
description: "Supersede an artifact (active → superseded). Creates a supersedes link to the replacement and marks the original as terminal."
---

Marks an active artifact as superseded by a replacement and records a `supersedes` link. This is terminal in ordinary lifecycle transitions, not irreversible: `forgeplan_restore` can reverse it while an eligible receipt remains. The operation captures a recovery receipt before mutation.

**Category**: Lifecycle

## When an agent calls this

- To displace an outdated EvidencePack after re-verification: terminal packs leave the eligible R_eff population. Legacy dependent-score refresh is best-effort; migrated workspaces recompute during reconciliation. This is not a guarantee of universal ancestor-cache reconciliation.

- Redesign: new RFC-019 replaces RFC-018 - agent calls `supersede RFC-018 --by RFC-019`.
- Second iteration of a decision after new evidence contradicts the old one.
- Renaming/restructuring: create new artifact, then supersede the old so history stays linked.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to supersede. |
| `by` | `string` | yes | Replacement artifact ID. |
| `workspace` | `string \| null` | no | Explicit absolute or tilde-expanded workspace path. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::SupersedeParams`_

## Returns

The JSON text payload contains `superseded`, `replacement`,
`dependents_affected`, `warnings`, `receipt_id`, and `_next_action`, plus
workspace-resolution and index-publication fields. It has no `from`, `to`,
or `link` object. Read the affected artifacts to inspect their status and
links, and inspect `index_pending` for publication acknowledgement.

## Example invocation

```json
{ "id": "RFC-018", "by": "RFC-019" }
```

With typical agent context:

> Team adopted a new retry strategy documented in RFC-019. Agent supersedes RFC-018.

```json
{ "id": "RFC-018", "by": "RFC-019" }
```

## Typical sequence

Create, fill, validate, implement and evidence the replacement as required, then activate it before superseding the old decision. This is workflow guidance, not a claim that the handler requires the replacement to be active. Review `dependents_affected`; supersession does not automatically implement their required changes.

## CLI equivalent

- [`forgeplan supersede`](/docs/cli/supersede/) - same operation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - preceding step
- [`forgeplan_deprecate`](/docs/mcp/forgeplan_deprecate/) - retirement without replacement
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - manual link creation

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Supersede an artifact (active → superseded). Creates link to replacement and notifies dependents.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `by` | yes | string | Replacement artifact ID |
| `id` | yes | string | Artifact ID to supersede |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "by": {
      "description": "Replacement artifact ID",
      "type": "string"
    },
    "id": {
      "description": "Artifact ID to supersede",
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
    "id",
    "by"
  ],
  "title": "SupersedeParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
