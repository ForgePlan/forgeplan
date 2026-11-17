---
title: forgeplan_reason
description: "Analyze an artifact using FPF ADI reasoning cycle: Abduction (3+ hypotheses) → Deduction (evaluate each) → Induction (synthesize recommendation). Requires LLM provider."
---

Runs the FPF ADI (Abduction → Deduction → Induction) reasoning cycle against an artifact using the configured LLM provider. It asks for competing hypotheses, testable predictions and a synthesized conclusion. The MCP handler passes artifact context but no optional FPF knowledge-base seed; it has no `fpf` input flag. The CLI's `--fpf` option is a separate surface.

**Category**: Reasoning & AI

## When an agent calls this

- After validation and before implementation when the team's workflow calls for ADI. Deep/Critical ADI is a workflow policy, not a mandatory gate enforced by this handler.
- When the user is hesitating between two approaches - ADI produces comparable predictions.
- During adversarial review: re-reason with different constraints to stress-test the current decision.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to analyze with ADI reasoning cycle |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The payload contains `artifact_id`, `artifact_title`, `analysis`, `provider`, `model`, the workspace trace and `_next_action`. Read the generated `analysis` text: this endpoint does not expose separate `hypotheses`, `deduction` or `induction` JSON arrays.

LLM output is a proposal, not measured evidence. Review its reasoning and validate the artifact before relying on it.

The handler loads LLM configuration with environment overrides and calls the provider. It does not save the analysis into the artifact or create evidence. Missing IDs return an MCP error; provider failures return `isError` with text and a configuration/login hint, not a substitute analysis.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "PRD-001"
}
```

## Typical sequence

Suggested workflow: `forgeplan_validate` → `forgeplan_reason` → review the hypotheses and uncertainty with the user → update the decision or gather evidence → implementation. The tool does not enforce user selection, a confidence threshold, or an automatic transition to coding.

## CLI equivalent

- [`forgeplan reason`](/docs/cli/reason/) - same pipeline, terminal output

## See also

- [MCP overview](/docs/mcp/)
- [FPF methodology](/docs/methodology/adi/)
- [`forgeplan_fpf_search`](/docs/mcp/forgeplan_fpf_search/) - direct KB lookup
- [`forgeplan_decompose`](/docs/mcp/forgeplan_decompose/) - reasoning applied to breakdown
