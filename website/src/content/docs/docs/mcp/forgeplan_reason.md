---
title: forgeplan_reason
description: "Analyze an artifact using FPF ADI reasoning cycle: Abduction (3+ hypotheses) → Deduction (evaluate each) → Induction (synthesize recommendation). Requires LLM provider."
---

Runs the FPF ADI (Abduction → Deduction → Induction) reasoning cycle against an artifact, optionally seeded with FPF Knowledge Base context. Abduction generates 3+ competing hypotheses, Deduction produces testable predictions for each, and Induction synthesizes a conclusion with justified confidence. For Deep and Critical depth artifacts, ADI is **mandatory** — the agent should never commit code until `forgeplan_reason` has been consulted.

**Category**: Reasoning & AI

## When an agent calls this

- After `forgeplan_validate` PASS but before coding a Deep/Critical artifact — the mandatory ADI gate.
- When the user is hesitating between two approaches — ADI produces comparable predictions.
- During adversarial review: re-reason with different constraints to stress-test the current decision.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to analyze with the ADI reasoning cycle. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ReasonParams`_

## Returns

A structured ADI report. The `hypotheses` array has 3+ entries, each with predictions. `induction` contains the synthesized recommendation and a confidence score the agent should surface verbatim.

Example response shape:

```json
{
  "id": "PRD-042",
  "hypotheses": [
    { "id": "H1", "claim": "Use JWT with 15m access / 7d refresh", "predictions": ["..."] },
    { "id": "H2", "claim": "Use server-side sessions with Redis", "predictions": ["..."] },
    { "id": "H3", "claim": "OAuth2 device flow for CLI clients", "predictions": ["..."] }
  ],
  "deduction": [
    { "hypothesis": "H1", "supported_by": ["..."], "risks": ["..."] }
  ],
  "induction": {
    "recommendation": "H1 with session blacklist fallback",
    "confidence": 0.78,
    "rationale": "Aligns with existing infrastructure; lowest rollback cost."
  }
}
```

## Example invocation

```json
{ "id": "PRD-001", "fpf": true }
```

With typical agent context:

> PRD-042 is Deep depth. Agent runs ADI with FPF KB context before letting code happen.

```json
{ "id": "PRD-042", "fpf": true }
```

## Typical sequence

`forgeplan_validate` PASS → `forgeplan_reason` (Deep/Critical must) → agent presents hypotheses to user → user picks direction (or concurs with induction) → code. If induction confidence is low (< 0.5), the agent should surface the uncertainty rather than forge ahead.

## CLI equivalent

- [`forgeplan reason`](/docs/cli/reason/) — same pipeline, terminal output

## See also

- [MCP overview](/docs/mcp/)
- [FPF methodology](/docs/methodology/adi/)
- [`forgeplan_fpf_search`](/docs/mcp/forgeplan_fpf_search/) — direct KB lookup
- [`forgeplan_decompose`](/docs/mcp/forgeplan_decompose/) — reasoning applied to breakdown
