---
title: forgeplan_route
description: "Recommend task depth and artifact pipeline. MCP and CLI can use configured LLM routing with keyword fallback; CLI auto mode can escalate a Deep result to Level 2."
---

Recommends a depth and artifact pipeline before non-trivial work. Treat it as routing advice to review against the task's actual risk and scope, not proof that the chosen design or effort is correct. The MCP handler attempts Level 1 when workspace LLM configuration is available and falls back to Level 0 keywords. A configured local CLI provider need not use an API key.

**Category**: Reasoning & AI

## When an agent calls this

- User says "please add X to the project" - agent routes before deciding whether to create artifacts.
- Before `forgeplan_new`: the depth from the route determines which template sections are mandatory.
- When a refactor's depth is uncertain - use routing as input to the decision, not an infallible classifier.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `description` | `string` | yes | Task description in natural language. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::RouteParams`_

## Returns

Returns `depth`, `pipeline`, `triggers`, `confidence`, `level`, `explanation`, `display`, `_alternatives`, and `_next_action`. Each alternative carries `depth`, `pipeline`, and `reasoning`. Confidence is the classifier's estimate, not a measured success probability.

Illustrative response excerpt:

```json
{
  "depth": "Standard",
  "pipeline": ["prd", "rfc"],
  "level": 0,
  "explanation": "Example routing explanation",
  "_alternatives": []
}
```

## Example invocation

```json
{ "description": "add rate limiting to API" }
```

With typical agent context:

> User asks for a feature. Agent routes first so the rest of the flow aligns with the correct depth.

```json
{ "description": "rewrite the LanceDB storage layer to use SQLite" }
```

## Typical sequence

`forgeplan_route` → review depth → for Standard+, search for existing decisions and author the required artifacts → `forgeplan_validate` → ADI when required (Deep/Critical) → code. Tactical routing does not waive tests or applicable project policy.

## CLI equivalent

- [`forgeplan route`](/docs/cli/route/) - the selected CLI defaults to auto mode: it attempts configured Level 1 LLM routing with keyword fallback, and can escalate a successful Deep Level 1 result to Level 2 when `--level` is omitted. Captured help still says Level 0 is the default; the handler behaves differently. For keyword-only routing, pass explicit `--level 0` **without `--explain`**: adding `--explain` can invoke the configured provider even with Level 0.

## See also

- [MCP overview](/docs/mcp/)
- [Depth calibration guide](/docs/methodology/routing/)
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - next step after routing
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) - ADI, mandatory for Deep/Critical depths

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Suggest depth level (Tactical/Standard/Deep/Critical) and artifact pipeline for a task description. Uses LLM classification (Level 1) when API key is configured, falls back to rule-based keywords (Level 0).

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `description` | yes | string | Task description in natural language |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "description": {
      "description": "Task description in natural language",
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
    "description"
  ],
  "title": "RouteParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
