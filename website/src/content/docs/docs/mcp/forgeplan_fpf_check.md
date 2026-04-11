---
title: forgeplan_fpf_check
description: "Evaluate the FPF rule set against a specific artifact. Returns every matched rule, the winning rule (first in priority order — same as runtime), rules that did not match, and the recommended action bucket. Use this to answer 'what should I do next with this artifact, and why?'"
---

Runs the active FPF rules against a single artifact and returns everything the engine saw: which rules matched, which didn't, which one won under the priority order, and the recommended action bucket (`EXPLORE` / `INVESTIGATE` / `EXPLOIT`). This is the agent's "what do I do next?" tool — it turns the abstract rule set from `forgeplan_fpf_rules` into a concrete recommendation tied to a real artifact.

**Category**: FPF Knowledge Base

## When an agent calls this

- After the user asks "what's my next action on PRD-042?" — the winning rule's `message` is the answer.
- During a review loop — check every active artifact to spot ones stuck below the EXPLORE threshold.
- To debug a surprising `forgeplan_reason` or `forgeplan_health` output — `fpf_check` shows the exact rule causing the recommendation.
- Before `forgeplan_activate` — confirm the artifact is in the EXPLOIT bucket, not EXPLORE.

## How it works

1. Loads the effective rule set (config overrides > defaults), same path as `forgeplan_fpf_rules`.
2. Fetches the artifact by ID (frontmatter + R_eff + evidence links).
3. Evaluates every rule's condition tree against the artifact state.
4. Collects matches and sorts by `priority` — **the first match wins**, exactly as the runtime engine does.
5. Reports the winning rule's `action` as the recommended bucket along with its `message`.

The thresholds that define the buckets come from `fpf.thresholds` in config (`explore_reff`, `investigate_reff`, `exploit_reff`). Depth affects the thresholds: a Critical artifact needs stronger evidence to reach EXPLOIT than a Tactical one, so the same R_eff can land in different buckets depending on depth.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID (case-insensitive), e.g. `PRD-042`, `RFC-007`, `ADR-003`. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::FpfCheckParams`_

## Returns

```json
{
  "artifact": {
    "id": "PRD-042",
    "kind": "prd",
    "status": "draft",
    "depth": "deep",
    "r_eff": 0.28
  },
  "winning_rule": {
    "name": "low_trust_explore",
    "priority": 10,
    "action": "EXPLORE",
    "message": "R_eff 0.28 < 0.33 explore threshold — add evidence or narrow scope before activation."
  },
  "matched": [
    { "name": "low_trust_explore", "priority": 10, "action": "EXPLORE" },
    { "name": "draft_needs_adi",   "priority": 50, "action": "INVESTIGATE" }
  ],
  "unmatched": [
    { "name": "high_trust_exploit", "priority": 30, "action": "EXPLOIT",
      "reason": "r_eff (0.28) < exploit_reff (0.66)" }
  ],
  "thresholds": {
    "explore_reff":     0.33,
    "investigate_reff": 0.66,
    "exploit_reff":     0.66,
    "depth_adjustment": "+0.10 for deep"
  }
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

- [`forgeplan fpf check <ID>`](/docs/cli/fpf-check/) — identical output rendered in the terminal.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_fpf_rules`](/docs/mcp/forgeplan_fpf_rules/) — inventory of rules evaluated here
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) — compute the R_eff that rules match against
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) — ADI reasoning that complements rule-based checks
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) — terminal action once the artifact earns EXPLOIT
