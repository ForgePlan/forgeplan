---
title: forgeplan_route
description: "Suggest depth level (Tactical/Standard/Deep/Critical) and artifact pipeline for a task description. Uses LLM classification (Level 1) when API key is configured, falls back to rule-based keywords (Level 0)."
---

Classifies an incoming task description into a depth level and recommends an artifact pipeline. This is the **first** MCP call in every non-trivial workflow — it tells the agent whether to just code (Tactical), create a PRD → RFC (Standard), the full PRD → Spec → RFC → ADR chain (Deep), or escalate to an Epic with cross-team review (Critical). Getting routing right saves hours of overthinking OR underthinking.

**Category**: Reasoning & AI

## When an agent calls this

- User says "please add X to the project" — agent routes before deciding whether to create artifacts.
- Before `forgeplan_new`: the depth from the route determines which template sections are mandatory.
- When the agent is uncertain whether a refactor is Tactical or Standard — route settles it.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `description` | `string` | yes | Task description in natural language. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::RouteParams`_

## Returns

A recommendation with depth, pipeline (list of artifact kinds in order), confidence (0-1), rationale, and alternatives the agent can offer if the user disagrees. Level 1 (LLM) returns richer rationale; Level 0 (keyword fallback) returns a terser shape but is still actionable.

Example response shape:

```json
{
  "depth": "Standard",
  "pipeline": ["prd", "rfc"],
  "confidence": 0.85,
  "rationale": "Multi-file feature touching auth flow; reversible within a sprint.",
  "alternatives": [
    { "depth": "Deep", "pipeline": ["prd", "spec", "rfc", "adr"], "when": "if crypto primitives change" }
  ]
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

`forgeplan_route` → (if Tactical) jump straight to code → (if Standard+) `forgeplan_search` (dup check) → `forgeplan_new` → `forgeplan_update` → `forgeplan_validate` → `forgeplan_reason`. Routing is cheap; call it whenever the task feels ambiguous.

## CLI equivalent

- [`forgeplan route`](/docs/cli/route/) — same classifier, human output

## See also

- [MCP overview](/docs/mcp/)
- [Depth calibration guide](/docs/methodology/routing/)
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) — next step after routing
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) — ADI for Standard+ depths
