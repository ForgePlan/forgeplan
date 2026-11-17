---
title: forgeplan_route
description: "Recommend a task depth and artifact pipeline using a configured LLM provider or keyword fallback. Keyless providers are supported; Critical is not a separate core pipeline."
---

Classifies a task description and recommends a depth and artifact pipeline. Call it before creating artifacts when the appropriate workflow is unclear. This is advice: the handler does not create artifacts, change session phase or apply its suggested depth to later calls.

**Category**: Reasoning & AI

## When an agent calls this

- User says "please add X to the project" - agent routes before deciding whether to create artifacts.
- Before `forgeplan_new`: the depth from the route determines which template sections are mandatory.
- When the agent is uncertain whether a refactor is Tactical or Standard - route settles it.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `description` | `string` | yes | Task description in natural language |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Both classification levels use the same JSON shape in MCP text content: `depth`, `pipeline`, `triggers` (trigger IDs), `confidence`, `level`, `explanation`, `display`, `_alternatives`, and `_next_action`. Each alternative has `depth`, `pipeline`, and `reasoning`; there are no `rationale`, `alternatives`, or `when` fields.

The handler attempts Level 1 when the resolved workspace has LLM configuration (with environment overrides), optionally supplying FPF context. Missing workspace/configuration or an unavailable/unparseable LLM result falls back to Level 0 keyword rules. Explicit local CLI providers do not require an HTTP API key. Very short input also uses Level 0. The returned `level` identifies the path; routing does not run full ADI reasoning automatically.

The released core maps `Tactical` to no artifacts, `Standard` to PRD → RFC and `Deep` to PRD → Spec → RFC → ADR. It has no separate Critical pipeline despite the tool description's four-tier wording.

Illustrative response excerpt for a parseable Level 1 Standard classification (alternatives, display and guidance omitted):

```json
{
  "depth": "Standard",
  "pipeline": ["prd", "rfc"],
  "triggers": [],
  "confidence": 0.9,
  "level": 1,
  "explanation": "A feature with several implementation choices."
}
```

Level 1 confidence is fixed at `0.9` by the routing implementation, not a measured probability that the LLM recommendation is correct.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "description": "Add rate limiting to the API"
}
```

## Typical sequence

Suggested workflow: route → review the recommendation → search for duplicates → create artifacts as needed → populate and validate them → reason when required by team policy. Review the actual `pipeline`; routing does not execute it. Level 1 may call an external provider and is not necessarily cheap.

## CLI equivalent

- [`forgeplan route`](/docs/cli/route/) - same classifier, human output

## See also

- [MCP overview](/docs/mcp/)
- [Depth calibration guide](/docs/methodology/routing/)
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - next step after routing
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) - ADI for Standard+ depths
