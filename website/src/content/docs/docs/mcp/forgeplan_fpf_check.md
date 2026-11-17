---
title: forgeplan_fpf_check
description: "Check which FPF rules match a given artifact, showing all matched rules, the winning rule (first in priority order, same as runtime), and rules that did not match. Use this to understand FPF engine behavior for a specific artifact before acting on it."
---

Runs the active FPF rules against a single artifact and returns everything the engine saw: which rules matched, which didn't, which one won under the priority order, and the recommended action bucket (`EXPLORE` / `INVESTIGATE` / `EXPLOIT`). This is the agent's "what do I do next?" tool - it turns the abstract rule set from `forgeplan_fpf_rules` into a concrete recommendation tied to a real artifact.

**Category**: FPF Knowledge Base

## When an agent calls this

- After the user asks "what's my next action on PRD-042?" - the winning rule's `message` is the answer.
- During a review loop, inspect which rules match active artifacts and why.
- To inspect a rule-based recommendation independently of LLM reasoning.
- Before activation, inspect the recommendation; this read-only check does not grant activation or replace lifecycle validation.

## How it works

1. Loads the effective rule set (config overrides > defaults), same path as `forgeplan_fpf_rules`.
2. Fetches the artifact by ID (frontmatter + R_eff + evidence links).
3. Evaluates every rule's condition tree against the artifact state.
4. Collects matches and sorts by `priority` - **the first match wins**, exactly as the runtime engine does.
5. Reports the winning rule's `action` as the recommended bucket along with its `message`.

The built-in rules use status, evidence score, quality and link count, not universal `0.33/0.66` buckets or automatic depth uplifts. See [`forgeplan_fpf_rules`](/docs/mcp/forgeplan_fpf_rules/) for the five default conditions. Nonempty `fpf.rules` replaces the defaults.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to check (e.g. "PRD-041") |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Successful output is JSON in MCP text content. Illustrative values for an artifact matching only `blind-spot`:

```json
{
  "artifact_id": "PRD-042",
  "kind": "prd",
  "status": "draft",
  "matched": [
    {
      "name": "blind-spot",
      "priority": 1,
      "action": "EXPLORE",
      "message": "Draft with no evidence (R_eff < 0.01). Needs evidence to validate."
    }
  ],
  "unmatched": ["weak-evidence", "orphan-active", "medium-quality", "ready-to-build"],
  "winning": {
    "name": "blind-spot",
    "priority": 1,
    "action": "EXPLORE",
    "message": "Draft with no evidence (R_eff < 0.01). Needs evidence to validate."
  },
  "summary": "1 matched, 4 unmatched, winning: blind-spot",
  "_next_action": "Illustrative guidance; read winning.message."
}
```

`winning` is `null` if no rule matches. `unmatched` contains names, not failure explanations. There is no nested `artifact`, score, depth or thresholds object. `_next_action` is guidance, not an enforcement result.

If the ID doesn't exist, the tool returns an error so the agent can fall back to `forgeplan_search`.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "PRD-001"
}
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

- [`forgeplan fpf check <ID>`](/docs/cli/fpf-check/) - the same rule evaluation, rendered for the terminal.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_fpf_rules`](/docs/mcp/forgeplan_fpf_rules/) - inventory of rules evaluated here
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - compute the R_eff that rules match against
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) - ADI reasoning that complements rule-based checks
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - a separate lifecycle operation with its own checks
