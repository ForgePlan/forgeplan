---
title: forgeplan_score
description: "Compute R_eff quality score for an artifact based on linked evidence. R_eff uses the weakest-link principle: score = min(evidence_scores)."
---

Computes the R_eff (effective reliability) score for an artifact, derived from its linked **non-terminal** EvidencePacks — packs with a `superseded`/`deprecated` status stay in the response flagged `excluded: true` (with their raw own-merit score) but do not feed the min. R_eff follows the weakest-link principle - it's the minimum over evidence scores, never an average - so one CL0 EvidencePack can drag the whole score to 0.1. Agents use this to answer "is this decision trustworthy enough to ship on?" and to surface PRDs that need more evidence.

**Category**: Quality & Validation

## When an agent calls this

- After creating and linking an EvidencePack - "did R_eff actually move above zero?"
- As part of `forgeplan_health` deep-dive: which active artifacts still have R_eff = 0?
- Before proposing `forgeplan_activate` for a Deep/Critical artifact where evidence is mandatory.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to score. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ScoreParams`_

## Returns

The response includes `r_eff`, `self_score`, F-G-R fields, `weakest_link`, `factors`, `decay_penalty`, and an `evidence` array. Each evidence entry includes `verdict`, `congruence_level`, `score`, `expired`, `status`, and `excluded`. A decision without current supporting evidence scores zero; EvidencePack leaves can have their own score. The recursive computation can also be limited by a linked artifact, not only a directly attached EvidencePack.

The `evidence[].score` breakdown uses flat own-merit scoring; recursive `r_eff` additionally accounts for evidence type and dependency penalties, so those surfaces can differ. Output verdict strings here are `Supports`/`Weakens`/`Refutes`; authored evidence input keys use lowercase verdict values. Expired eligible evidence returns 0.1 before verdict arithmetic, including an expired refuting pack: the emitted catalog's unconditional refutes statement below does not describe that exception.

Illustrative response excerpt for two unexpired supporting measurement packs, not a captured score:

```json
{
  "id": "PRD-042",
  "r_eff": 0.9,
  "weakest_link": "EVID-056",
  "evidence": [
    { "id": "EVID-057", "verdict": "Supports", "congruence_level": 3, "score": 1.0, "expired": false, "status": "active", "excluded": false },
    { "id": "EVID-056", "verdict": "Supports", "congruence_level": 2, "score": 0.9, "expired": false, "status": "active", "excluded": false }
  ]
}
```

## Example invocation

```json
{ "id": "PRD-001" }
```

With typical agent context:

> Agent just linked EVID-057 to PRD-042 and wants to verify R_eff moved from 0.00 to a real number.

```json
{ "id": "PRD-042" }
```

## Typical sequence

`forgeplan_new evidence` → `forgeplan_update` (literal structured keys `verdict`, `congruence_level`, `evidence_type`) → `forgeplan_link` → `forgeplan_score` → review all activation gates before `forgeplan_activate`. A positive score alone is not activation acceptance. On legacy layouts this call persists the derived cached score; on migrated layouts it computes from the captured canonical view. Its registration is not proof that every layout or input was exercised.

## CLI equivalent

- [`forgeplan score`](/docs/cli/score/) - same computation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - how evidence is attached
- [`forgeplan_decay`](/docs/mcp/forgeplan_decay/) - evidence TTL management
- [`forgeplan_health`](/docs/mcp/forgeplan_health/) - aggregated R_eff dashboard

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Compute R_eff quality score for an artifact based on linked evidence. R_eff uses the weakest-link principle: score = min(evidence_scores) over the artifact's CURRENT evidence — packs with a terminal status (superseded/deprecated) are excluded from the min (ADR-020) but stay listed with `excluded: true`. An active refutes pack still zeroes the score.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | yes | string | Artifact ID to score |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to score",
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
  "title": "ScoreParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
