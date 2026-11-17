---
title: forgeplan_score
description: "Compute and cache recursive R_eff from eligible evidence and dependencies, including evidence-type and relation penalties. Displayed evidence scores use a flat helper and can differ from R_eff; terminal packs remain listed as excluded."
---

Computes recursive R_eff (effective reliability) across an artifact's own evidence and dependencies, never an average. Superseded/deprecated evidence stays in the breakdown with `excluded: true` but does not contribute to the minimum. Draft evidence remains eligible. Non-expired refuting evidence scores zero; expired evidence scores 0.1. A leaf EvidencePack can score on its own structured fields without inventing child evidence.

**Category**: Quality & Validation

## When an agent calls this

- After creating and linking an EvidencePack - "did R_eff actually move above zero?"
- As part of `forgeplan_health` deep-dive: which active artifacts still have R_eff = 0?
- Before proposing `forgeplan_activate` for a Deep/Critical artifact where evidence is mandatory.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to score |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP `content[0].text`. Fields are `id`, `title`, `r_eff`, `evidence`, `self_score`, `formality`, `granularity`, `reliability`, `overall_grade`, nullable `weakest_link`, `factors`, `decay_penalty`, and `_next_action`. Each `evidence` item has `id`, `verdict` (debug spelling such as `Supports`), `congruence_level`, `score`, `expired`, `status`, and `excluded`. There is no `breakdown`, `limiting_evidence`, `cl`, per-item evidence type, or decay factor.

The breakdown's `score` uses the flat helper (verdict minus CL penalty, or 0.1 when expired). The recursive aggregate also subtracts evidence-type penalties (test/measurement 0, benchmark 0.1, audit 0.2) and walks outgoing `informs`, `based_on`, `refines`, and `depends_on` dependencies. Relation penalties are 0.1 for `informs`/`based_on`, zero for `refines`/`depends_on`. Draft/terminal dependencies and note/memory dependencies are skipped; evidence-to-supported-artifact `informs`/`based_on` edges are not followed backwards. Thus the displayed evidence minimum need not equal `r_eff`; `weakest_link` identifies a limiting dependency, not necessarily an EvidencePack.

Despite the read-only annotation, this handler takes a workspace write lock and persists the recomputed score cache. If persistence fails it attempts a display-only recomputation. Workspace resolution can return JSON-RPC invalid params; missing IDs and lock failures return tool errors with plain text. It does not activate or renew artifacts.

Illustrative partial payload (additional fields listed above are omitted here):

```json
{
  "id": "PRD-042",
  "title": "Auth system",
  "r_eff": 0.8,
  "self_score": 0.8,
  "evidence": [
    {"id": "EVID-056", "verdict": "Supports", "congruence_level": 3, "score": 1.0, "expired": false, "status": "active", "excluded": false}
  ],
  "weakest_link": null,
  "factors": [],
  "decay_penalty": 0.0
}
```

This example represents audit-type evidence: flat score 1.0 versus recursive self-score 0.8.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "PRD-001"
}
```

## Typical sequence

Create evidence with `forgeplan_new` (`kind: "evidence"`), fill its structured fields through `forgeplan_update`, link it, then score the decision. Inspect `forgeplan_review` before activation; `r_eff > 0` alone does not pass every gate. `forgeplan_decay` is a separate read-only expiry comparison, not the recursive score cache update.

## CLI equivalent

- [`forgeplan score`](/docs/cli/score/) - same computation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - how evidence is attached
- [`forgeplan_decay`](/docs/mcp/forgeplan_decay/) - evidence TTL management
- [`forgeplan_health`](/docs/mcp/forgeplan_health/) - aggregated R_eff dashboard
