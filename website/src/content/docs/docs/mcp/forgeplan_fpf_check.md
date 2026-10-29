---
title: forgeplan_fpf_check
description: "Evaluate the FPF rule set against a specific artifact. Returns every matched rule, the winning rule (first in priority order - same as runtime), rules that did not match, and the recommended action bucket. Use this to answer 'what should I do next with this artifact, and why?'"
---

Runs the active FPF rules against a single artifact and returns everything the engine saw: which rules matched, which didn't, which one won under the priority order, and the recommended action bucket (`EXPLORE` / `INVESTIGATE` / `EXPLOIT`). This is the agent's "what do I do next?" tool - it turns the abstract rule set from `forgeplan_fpf_rules` into a concrete recommendation tied to a real artifact.

**Category**: FPF Knowledge Base

## When an agent calls this

- After the user asks "what's my next action on PRD-042?" - inspect `winning.message` when `winning` is non-null.
- During review - inspect matched conditions and their source rules rather than treating an action label as a score threshold.
- To understand rule-based recommendations - `fpf_check` exposes the selected rule set; it is not a trace of every health or LLM decision.
- Before activation - review the recommendation separately from validation, evidence, and provenance gates. `EXPLOIT` is not an activation permit.

## How it works

1. Loads the effective rule set (config overrides > defaults), same path as `forgeplan_fpf_rules`.
2. Fetches the artifact by ID (frontmatter + R_eff + evidence links).
3. Evaluates every rule's condition tree against the artifact state.
4. Collects matches and sorts by `priority` - **the first match wins**, exactly as the runtime engine does.
5. Reports the winning rule's `action` as the recommended bucket along with its `message`.

This tool evaluates `fpf.rules` (or the built-in default rules), not a separate universal depth-adjusted threshold table. Default rules use literal conditions such as draft `r_eff < 0.01`, active/stale `r_eff < 0.5`, and `r_eff >= 0.7` plus `overall >= 0.6`. Custom rules may include depth conditions. There is no automatic `+0.10` Deep adjustment in this check.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID, e.g. `PRD-042`. The handler rejects IDs longer than 128 bytes; the schema does not expose that cap. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::FpfCheckParams`_

## Returns

Illustrative response excerpt for a draft that matches no default rule. `unmatched` contains rule-name strings; the winner field is `winning`, which can be `null`.

```json
{
  "artifact_id": "PRD-042",
  "kind": "prd",
  "status": "draft",
  "matched": [],
  "unmatched": ["blind-spot", "weak-evidence", "orphan-active", "medium-quality", "ready-to-build"],
  "winning": null
}
```

If the ID doesn't exist, the tool returns an error so the agent can fall back to `forgeplan_search`.

## Example invocation

```json
{ "id": "PRD-042" }
```

## Typical sequence

```
forgeplan_list (status=draft)       ← find candidates
forgeplan_fpf_check { id: "PRD-X" } ← what's the recommended bucket?
  → EXPLORE:     forgeplan_new evidence + forgeplan_link
  → INVESTIGATE: forgeplan_reason + add measurements
  → EXPLOIT:     forgeplan_review → forgeplan_activate
```

## CLI equivalent

- [`forgeplan fpf check <ID>`](/docs/cli/fpf-check/) - identical output rendered in the terminal.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_fpf_rules`](/docs/mcp/forgeplan_fpf_rules/) - inventory of rules evaluated here
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - compute the R_eff that rules match against
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) - ADI reasoning that complements rule-based checks
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - lifecycle transition with its own gates, independent of the FPF action label

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Check which FPF rules match a given artifact, showing all matched rules, the winning rule (first in priority order, same as runtime), and rules that did not match. Use this to understand FPF engine behavior for a specific artifact before acting on it.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | yes | string | Artifact ID to check (e.g. "PRD-041") |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to check (e.g. \"PRD-041\")",
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
  "title": "FpfCheckParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
