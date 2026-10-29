---
title: forgeplan_coverage_business
description: "forgeplan_coverage_business — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

`extract_score` is a structural coverage heuristic: expected IDs come from this Domain Model's Composition section, while actual counts span the workspace. The `canonical` DDL/SDL flags and coherence/reproducibility values are currently unconditional `false`/`0` placeholders, not executed checks or proof of a working plugin integration.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Issue #287 Phase C — compute extract_score (0.0-1.0) for a Domain Model. Reads the DM's ## Composition section to derive expected counts of glossary / use_cases / invariants / scenarios / hypotheses; counts actuals across the workspace; returns weighted extract_score. Canonical fields (DDL/SDL/pseudo-code) default to 0/false until marketplace#79 Phase E lands.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `domain_model_id` | yes | string | Required DM- artifact id. The Composition section of that<br>Domain Model artifact provides the `expected` counts; this<br>tool counts what's actually present in the workspace. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "domain_model_id": {
      "description": "Required DM- artifact id. The Composition section of that\nDomain Model artifact provides the `expected` counts; this\ntool counts what's actually present in the workspace.",
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
    "domain_model_id"
  ],
  "title": "CoverageBusinessParams",
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
