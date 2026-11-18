---
title: forgeplan_calibrate
description: "Suggest Tactical, Standard, or Deep depth from artifact content, links, and structure."
---

Suggest an appropriate **depth** level for one or all artifacts using heuristic content analysis. The selected implementation returns `Tactical`, `Standard`, or `Deep`; the emitted catalog description below also names `Critical`, but the calibration algorithm does not produce it. A suggestion does not change artifact metadata or prove implementation readiness.

**Category**: Quality

## When an agent calls it

- **After creating a stub** - let calibration suggest the right depth before filling in sections.
- **During review** - sanity-check that the author chose a reasonable depth.
- **Audit sweep** - find artifacts marked `Tactical` that actually warrant `Deep` based on body content.
- **Onboarding agents** - teach them what factors push depth up (security, breaking changes, cross-team scope).

Heuristics considered: security sections present, breaking-change markers, outgoing link count, body length and section complexity.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | no | Artifact ID to calibrate. Omit to calibrate all artifacts in the workspace. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::CalibrateParams`_

## Returns

Illustrative response excerpt. The handler returns `results`, `signals`, and `total_escalations`, not a `suggestions` array.

```json
{
  "results": [
    {
      "artifact_id": "PRD-042",
      "artifact_title": "Authentication system",
      "current_depth": "tactical",
      "suggested_depth": "Deep",
      "escalation_needed": true,
      "signals": [
        { "name": "security_section", "value": "present", "minimum_depth": "Deep" }
      ]
    }
  ],
  "total_escalations": 1
}
```

## Example invocation

```json
{ "id": "PRD-042" }
```

## Typical sequence

1. `forgeplan_calibrate` - scan the workspace.
2. For a justified mismatch, use the CLI metadata operation, for example `forgeplan update PRD-042 --depth deep`. The MCP `forgeplan_update` schema has no `depth` parameter.
3. `forgeplan_validate` - rerun validation under the new depth (stricter MUST rules).
4. Follow the applicable ADI requirement for the chosen depth; calibration itself does not perform reasoning.

## CLI equivalent

```bash
forgeplan calibrate
forgeplan calibrate PRD-042
```

## See also

- [`forgeplan_route`](/docs/mcp/forgeplan_route/) - depth selection at task inception.
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - depth-aware validation.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Suggest depth level (Tactical/Standard/Deep/Critical) for artifacts based on content analysis. Detects security sections, breaking changes, link count, body complexity.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | no | string \| null | Artifact ID (checks all if omitted) |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "default": null,
      "description": "Artifact ID (checks all if omitted)",
      "type": [
        "string",
        "null"
      ]
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
  "title": "CalibrateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
