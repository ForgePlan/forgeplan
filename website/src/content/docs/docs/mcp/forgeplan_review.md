---
title: forgeplan_review
description: "Review an artifact - run validation and show lifecycle checklist. Shows MUST/SHOULD findings and whether artifact can be activated."
---

Reviews depth-aware validation findings and lifecycle prerequisites for an artifact, returning `can_activate` and warnings. This is a read-only preflight, not activation itself or a guarantee that a later write succeeds: the activation handler also evaluates its configured provenance gate and write-time conditions.

**Category**: Quality

## When an agent calls it

- **Before activation** - confirm all gates pass to avoid activation errors.
- **PR review** - run on each touched artifact to catch missing sections or MUST failures.
- **Author self-check** - faster than running `validate` + `score` + manual lifecycle inspection.
- **Automated quality hooks** - pre-commit / CI can call this and fail the build if any artifact regresses.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to review. |
| `workspace` | `string \| null` | no | Explicit absolute or tilde-expanded workspace path. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ReviewParams`_

## Returns

The JSON text payload contains `artifact_id`, boolean `can_activate`,
`must_findings`, `should_findings`, `warnings`, and `_next_action`.
There is no nested `validation`/`lifecycle` object, numeric `r_eff`,
or string `verdict` field in this response. A false `can_activate` is a
review result; a tool error is a separate failure to perform the review.

## Example invocation

```json
{ "id": "PRD-042" }
```

## Typical sequence

1. Inspect `can_activate`, all findings, and warnings from `forgeplan_review`.
2. Correct evidenced issues with `forgeplan_update`; do not invent changes to make the report clean.
3. Review again and retain any unmet implementation/evidence obligations.
4. Call `forgeplan_activate` only when the task's prerequisites are satisfied; inspect its separate result.

## CLI equivalent

```bash
forgeplan review PRD-042
```

## See also

- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - validation only, without lifecycle check.
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - R_eff recomputation only.
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - the action gated by this review.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Review an artifact — run validation and show lifecycle checklist. Shows MUST/SHOULD findings and whether artifact can be activated.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | yes | string | Artifact ID to review |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to review",
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
  "title": "ReviewParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
