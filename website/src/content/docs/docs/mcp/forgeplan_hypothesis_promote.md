---
title: forgeplan_hypothesis_promote
description: "forgeplan_hypothesis_promote — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Promote a hypothesis along the verification state machine (parked → inferred → strong-inferred → verified; or any → refuted; or any → parked). Mutates the artifact body via file-first projection: updates the `hypothesis_state:` frontmatter field and appends an audit entry to the `## Lifecycle State` section. Returns a list of artifacts that informs / based_on the hypothesis (cascade) — informational only, not auto-bumped. Rejects illegal transitions with a structured error enumerating the allowed next states.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `evidence_refs` | no | array | IDs of evidence artifacts (typically `EVID-NNN`) backing the<br>promotion. May be empty (e.g. parked-for-hold) but the rationale is<br>still mandatory. |
| `hypothesis_id` | yes | string | Hypothesis artifact id or slug (`HYP-001` / `hyp-some-claim`). Must<br>resolve to an artifact whose `kind == "hypothesis"`. |
| `new_state` | yes | string | Target lifecycle state in kebab-case: `parked`, `inferred`,<br>`strong-inferred`, `verified`, or `refuted`. |
| `rationale` | yes | string | Free-form one-line rationale for the audit trail in `## Lifecycle State`.<br>Stored verbatim in the body, sanitised for hint emission. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Params for `forgeplan_hypothesis_promote` — moves a HYP- artifact along\nthe verification state machine (parked → inferred → strong-inferred →\nverified | refuted). See `forgeplan_core::hypothesis` for the full\ntransition matrix and rejection rules.",
  "properties": {
    "evidence_refs": {
      "default": [],
      "description": "IDs of evidence artifacts (typically `EVID-NNN`) backing the\npromotion. May be empty (e.g. parked-for-hold) but the rationale is\nstill mandatory.",
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "hypothesis_id": {
      "description": "Hypothesis artifact id or slug (`HYP-001` / `hyp-some-claim`). Must\nresolve to an artifact whose `kind == \"hypothesis\"`.",
      "type": "string"
    },
    "new_state": {
      "description": "Target lifecycle state in kebab-case: `parked`, `inferred`,\n`strong-inferred`, `verified`, or `refuted`.",
      "type": "string"
    },
    "rationale": {
      "description": "Free-form one-line rationale for the audit trail in `## Lifecycle State`.\nStored verbatim in the body, sanitised for hint emission.",
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
    "hypothesis_id",
    "new_state",
    "rationale"
  ],
  "title": "HypothesisPromoteParams",
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
