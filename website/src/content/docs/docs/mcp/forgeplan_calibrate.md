---
title: forgeplan_calibrate
description: "Recommend Tactical, Standard or Deep from artifact content without changing its stored depth."
---

Suggests an artifact depth using deterministic heuristics, without modifying the artifact. This released implementation suggests `Tactical`, `Standard` or `Deep`; it does not produce a distinct `Critical` level. Depth-sensitive validation and a team's ADI/review policy are separate from this recommendation.

**Category**: Quality

## When an agent calls it

- **After creating a stub** - let calibration suggest the right depth before filling in sections.
- **During review** - sanity-check that the author chose a reasonable depth.
- **Audit sweep** - find artifacts marked `Tactical` that actually warrant `Deep` based on body content.
- **Onboarding agents** - teach them what factors push depth up (security, breaking changes, cross-team scope).

Signals include checkbox count, English security/breaking-change markers, outgoing relation count, a parent Epic, H2 section count and body line count. Checkbox thresholds are >3 for Standard and >10 for Deep; links >2/>5, H2 sections >4/>8 and lines >50/>200. Security or breaking-change markers imply Deep; a parent Epic implies Standard. The highest signal wins, or Tactical if none match. This is heuristic text matching, not semantic or language-independent analysis.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string \| null` | no | Artifact ID (checks all if omitted) Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Illustrative JSON payload in MCP text content:

```json
{
  "results": [
    {
      "artifact_id": "PRD-042",
      "artifact_title": "Login flow",
      "current_depth": "tactical",
      "suggested_depth": "Deep",
      "escalation_needed": true,
      "signals": [
        {"name": "security_section", "value": "present", "minimum_depth": "Deep"}
      ]
    }
  ],
  "total_escalations": 1,
  "_next_action": "Illustrative guidance to review an under-depth artifact."
}
```

`results` includes every checked record, not just mismatches. `total_escalations` counts only recommendations above current depth. Suggested and signal depths use title case; `current_depth` preserves the stored value. With `id`, matching is case-insensitive and an unknown ID returns an MCP error. Relation-read failures are logged and counted as zero links. No LLM or automatic depth write is involved.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_calibrate` - scan the workspace.
2. Review each escalation and explicitly update the artifact's frontmatter depth if warranted; re-routing alone does not change it.
3. Validate under the new depth.
4. Run ADI/review as required by team policy; calibration does not enforce those steps.

## CLI equivalent

```bash
forgeplan calibrate
forgeplan calibrate PRD-042
```

## See also

- [`forgeplan_route`](/docs/mcp/forgeplan_route/) - depth selection at task inception.
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - depth-aware validation.
- [Methodology guide](/docs/methodology/overview/)
