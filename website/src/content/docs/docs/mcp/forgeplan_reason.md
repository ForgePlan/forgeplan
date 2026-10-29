---
title: forgeplan_reason
description: "Analyze an artifact using FPF ADI reasoning cycle: Abduction (3+ hypotheses) → Deduction (evaluate each) → Induction (synthesize recommendation). Requires LLM provider."
---

Runs FPF ADI (Abduction → Deduction → Induction) against an artifact through a configured LLM provider. The prompt asks for at least three competing hypotheses, testable deductions, and an induction grounded in evidence; that request is not a guarantee of model compliance or measured confidence. ADI remains mandatory for Deep/Critical work under the methodology. This MCP call has no `fpf` input; CLI `--fpf` is a separate option.

**Category**: Reasoning & AI

## When an agent calls this

- After `forgeplan_validate` PASS but before coding a Deep/Critical artifact - the mandatory ADI gate.
- When the user is hesitating between two approaches - ADI produces comparable predictions.
- During adversarial review: re-reason with different constraints to stress-test the current decision.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to analyze with the ADI reasoning cycle. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ReasonParams`_

## Returns

Returns `artifact_id`, `artifact_title`, `analysis` (a string), `provider`, and `model`, plus workspace trace and a hint. There is no top-level MCP `hypotheses` or `induction` array/object contract.

Inspect the analysis for genuinely competing alternatives, predictions, evidence and uncertainty. Model-generated confidence is not an empirical reliability measurement or permission to activate.

Illustrative response excerpt:

```json
{
  "artifact_id": "PRD-042",
  "artifact_title": "Example design",
  "analysis": "Abduction: compare the alternatives. Deduction: state discriminating predictions. Induction: choose only as strongly as the evidence supports."
}
```

## Example invocation

```json
{ "id": "PRD-001" }
```

With typical agent context:

> PRD-042 is Deep depth. The agent runs ADI and reviews its actual analysis before coding.

```json
{ "id": "PRD-042" }
```

## Typical sequence

`forgeplan_validate` PASS → `forgeplan_reason` (mandatory for Deep/Critical) → review alternatives and evidence with the user → choose a direction → code. Report uncertainty instead of treating an unsupported confidence number as a gate.

## CLI equivalent

- [`forgeplan reason`](/docs/cli/reason/) - same pipeline, terminal output

## See also

- [MCP overview](/docs/mcp/)
- [FPF methodology](/docs/methodology/adi/)
- [`forgeplan_fpf_search`](/docs/mcp/forgeplan_fpf_search/) - direct KB lookup
- [`forgeplan_decompose`](/docs/mcp/forgeplan_decompose/) - reasoning applied to breakdown

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Analyze an artifact using FPF ADI reasoning cycle: Abduction (3+ hypotheses) → Deduction (evaluate each) → Induction (synthesize recommendation). Requires LLM provider.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | yes | string | Artifact ID to analyze with ADI reasoning cycle |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to analyze with ADI reasoning cycle",
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
  "title": "ReasonParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
