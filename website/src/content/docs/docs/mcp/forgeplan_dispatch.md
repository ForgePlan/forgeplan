---
title: forgeplan_dispatch
description: "Compute a parallel-safe work plan for N sub-agents — buckets, serial queue, reasoning."
---

The orchestrator entry point for multi-agent work. Returns one bucket per agent with
artifacts that can be worked in parallel without file conflicts, plus a serial queue for
leftover work. Skips artifacts already claimed (live claim by another agent), defers
artifacts whose `affected_files` Jaccard-overlap exceeds the threshold (default 0.3),
respects the structural dependency graph (blocked artifacts never enter a bucket), and
when `agent_skills` is provided routes by domain match. Read-only — does not mutate
workspace state.

**Category**: Multi-agent

## When an agent calls it

- Start of a multi-agent sprint: orchestrator wants 2–5 sub-agents on non-overlapping work.
- After a [`forgeplan_release`](/docs/mcp/forgeplan_release/) — re-plan because the candidate set changed.
- After [`forgeplan_new`](/docs/mcp/forgeplan_new/) creates fresh drafts — incorporate them.
- TTL expiry on a stuck claim — re-dispatch to backfill the freed slot.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `agents` | `number` | yes | Number of sub-agents (1..=`MAX_AGENTS`). PRD-057 targets 2–5. |
| `kind` | `string` | no | Filter to one artifact kind (`prd`, `rfc`, `spec`, etc.). Default: all kinds. |
| `epic` | `string` | no | Only candidates whose `parent_epic` frontmatter matches this Epic ID. |
| `status` | `string` | no (default `"draft"`) | Status filter. `"any"` to include all lifecycle states. |
| `agent_skills` | `string[][]` | no | Per-agent skill lists in index order (max `MAX_SKILLS_PER_AGENT` per agent). |
| `overlap_threshold` | `number` | no (default 0.3) | Jaccard threshold for file-conflict deferral. Range `[0.0, 1.0]`. |

_Schema source: `crates/forgeplan-mcp/src/types.rs::DispatchParams`_

## Returns

```json
{
  "buckets": [
    ["PRD-057"],
    ["RFC-012"]
  ],
  "serial_queue": ["SPEC-018"],
  "reasoning": [
    "PRD-057 → bucket 0 (no skill match, no claim, no overlap)",
    "RFC-012 → bucket 1 (skill match: 'rust')",
    "SPEC-018: deferred (file overlap with PRD-057 @ Jaccard 0.42)"
  ],
  "generated_at": "2026-04-26T10:00:00Z",
  "agent_count": 2,
  "overlap_threshold": 0.3,
  "candidate_count": 3,
  "claimed_count": 0,
  "skipped_parse_errors": 0,
  "blocked_count": 0,
  "_next_action": "Plan ready: 3 candidate(s), 2 parallel bucket(s), 1 serial ..."
}
```

Hand `buckets[i]` to sub-agent `i`. Re-dispatch when the claim set or candidate set
changes. `skipped_parse_errors > 0` means at least one candidate's frontmatter could
not be read — check server logs.

## Example invocation

Default: 3 agents, draft PRDs only:

```json
{ "agents": 3, "kind": "prd" }
```

Skill-aware dispatch:

```json
{
  "agents": 2,
  "agent_skills": [["rust", "mcp"], ["docs", "ru"]],
  "overlap_threshold": 0.25
}
```

Whole-Epic re-plan:

```json
{ "agents": 4, "epic": "EPIC-005", "status": "any" }
```

## Typical sequence

1. `forgeplan_dispatch agents=N` — orchestrator gets the plan.
2. Each sub-agent `i` calls [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) on `buckets[i][0]`.
3. Sub-agents work; orchestrator polls [`forgeplan_claims`](/docs/mcp/forgeplan_claims/).
4. [`forgeplan_release`](/docs/mcp/forgeplan_release/) on completion → re-dispatch.

## CLI equivalent

[`forgeplan dispatch`](/docs/cli/) — same engine. Shell-driven orchestrators use the CLI;
LLM-driven orchestrators use the MCP tool.

## See also

- [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) — sub-agent picks a bucket item
- [`forgeplan_release`](/docs/mcp/forgeplan_release/) — return a slot to the pool
- [`forgeplan_claims`](/docs/mcp/forgeplan_claims/) — monitor in-flight work
