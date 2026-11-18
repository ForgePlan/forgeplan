---
title: forgeplan_contradictions
description: "forgeplan_contradictions — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

Implementation boundary: the response has a flat `contradictions` list and a `limitations` list, not separate empty arrays for the three deferred classes described below. Only title-similarity `hypothesis_duplicate` is computed; above 1000 hypotheses the pairwise pass is skipped and the limit is reported. No matches is not proof of semantic consistency.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Issue #287 Phase C — detect cross-artifact contradictions (v1: hypothesis_duplicate via Jaccard ≥ 0.6 on titles). Three other classes (invariant_conflict, glossary_divergence, scenario_vs_invariant) require LLM judgement and are deferred — buckets included in response with empty arrays + limitations list.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "ContradictionsParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->

## Examples

- [MCP setup and workspace scope](/docs/guides/mcp-setup/)
- [Constellation walkthrough and handle sequence](/docs/guides/constellation/)

## See also

- [MCP reference](/docs/mcp/)
- [Constellation reference](/docs/reference/constellation/)
