---
title: forgeplan_deprecate
description: "Deprecate an artifact (active/stale → deprecated) with a reason."
---

Moves an active or stale artifact to terminal `deprecated` without a replacement. Unlike `supersede`, this does not name a successor. The required reason is written to the body's `## Deprecation` section and returned by subsequent reads; inspect the mutation's reconciliation result and read the canonical file back.

**Category**: Lifecycle

## When an agent calls this

- Feature retirement: "we're removing rate limiting from v3, deprecate PRD-020".
- Abandoned direction: an exploratory PRD that turned out infeasible.
- Cleanup during `forgeplan_health` remediation: stale artifacts that nobody will renew.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to deprecate. |
| `reason` | `string` | yes | Reason for deprecation. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::DeprecateParams`_

## Returns

Returns `deprecated`, `reason`, `dependents_affected`, and a recovery `receipt_id`, plus hints and workspace/reconciliation metadata. The reason belongs in the Markdown body, not a new frontmatter field.

Illustrative response excerpt:

```json
{
  "deprecated": "PRD-020",
  "reason": "Feature cancelled; no replacement.",
  "dependents_affected": []
}
```

## Example invocation

```json
{ "id": "PRD-020", "reason": "Feature cancelled; see Q2 planning doc." }
```

With typical agent context:

> Stakeholders decided to drop a feature. Agent captures the reason and deprecates.

```json
{ "id": "PRD-020", "reason": "Feature cancelled in Q2 planning; no replacement." }
```

## Typical sequence

List active/stale artifacts with `forgeplan_list`, read the target and confirm retirement with the user, then call `forgeplan_deprecate`. Review affected dependents rather than assuming health counts shrink. For a stale decision that remains valid, the alternative is CLI `forgeplan renew ID --reason "..." --until YYYY-MM-DD`; there is no `forgeplan_renew` MCP tool in this catalog.

## CLI equivalent

- [`forgeplan deprecate`](/docs/cli/deprecate/) - same operation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) - when a replacement exists
- [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) - precedes deprecate for expired artifacts
- [`forgeplan_delete`](/docs/mcp/forgeplan_delete/) - removal with receipt-based recovery; review recovery limits before use

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Deprecate an artifact (active → deprecated) with a reason.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | yes | string | Artifact ID to deprecate |
| `reason` | yes | string | Reason for deprecation |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to deprecate",
      "type": "string"
    },
    "reason": {
      "description": "Reason for deprecation",
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
    "reason"
  ],
  "title": "DeprecateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
