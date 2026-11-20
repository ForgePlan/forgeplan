---
title: forgeplan_dispatch
description: "Compute advisory agent buckets and a serial queue; orchestrators must check cross-bucket file conflicts."
---

Computes one work bucket per agent and a serial queue using declared
`affected_files`, active claims, structural dependencies, and optional domain
skills. The current algorithm checks overlap only against artifacts already in
the bucket being considered, not against other agents' buckets. Even identical
declared file sets can therefore reach different concurrent agents. Within a
candidate bucket, Jaccard overlap **at or above** the threshold (default 0.3)
rejects that placement; it does not guarantee cross-bucket safety.

This read-only plan is advisory. Before any handoff, the orchestrator must compare
**all concurrently assigned file sets** and serialize or reassign overlapping
work, including declared overlaps. Artifact claims are advisory markers, not file
locks; acquiring different claims does not make file sets disjoint. The captured
runtime description below calls the plan “parallel-safe”, but the current
implementation does not establish that guarantee.

**Category**: Multi-agent

## When an agent calls it

- Start of a multi-agent sprint: orchestrator wants 2–5 sub-agents on non-overlapping work.
- After a [`forgeplan_release`](/docs/mcp/forgeplan_release/) - re-plan because the candidate set changed.
- After [`forgeplan_new`](/docs/mcp/forgeplan_new/) creates fresh drafts - incorporate them.
- TTL expiry on a stuck claim - re-dispatch to backfill the freed slot.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `agents` | `integer` | yes | Number of sub-agents, 1..=64. The MCP handler rejects values outside this range. |
| `kind` | `string` | no | Filter to one artifact kind (`prd`, `rfc`, `spec`, etc.). Default: all kinds. |
| `epic` | `string` | no | Only candidates whose `parent_epic` frontmatter matches this Epic ID. |
| `status` | `string` | no (default `"draft"`) | Status filter. `"any"` to include all lifecycle states. |
| `agent_skills` | `string[][]` | no | Per-agent skill lists; at most one list per requested agent and 32 skills per list. Omission defaults to `[]`, not null. |
| `overlap_threshold` | `number \| null` | no (default 0.3) | Inclusive Jaccard conflict threshold. The MCP handler rejects values outside `[0.0, 1.0]`, despite the captured schema's clamp wording. |
| `workspace` | `string \| null` | no | Explicit absolute or tilde-expanded workspace path. |

_Schema source: `crates/forgeplan-mcp/src/types.rs::DispatchParams`_

## Returns
The following is an illustrative payload, not a recorded dispatch result.


```json
{
  "buckets": [
    ["PRD-057"],
    ["RFC-012"]
  ],
  "serial_queue": [{"id": "SPEC-018", "reason": "file overlap >=0.30 with bucket 0"}],
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

Only hand `buckets[i]` to sub-agent `i` after comparing all concurrent file sets
across buckets and serializing or reassigning overlaps. Re-dispatch when the claim
set or candidate set changes, then repeat that comparison. `skipped_parse_errors > 0`
means at least one candidate's frontmatter could not be read - check server logs.

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

1. `forgeplan_dispatch agents=N` - orchestrator gets the advisory plan.
2. Orchestrator compares all concurrently assigned file sets across buckets, including declared overlaps, and serializes or reassigns conflicting work before handoff.
3. For each resulting non-empty assignment, sub-agent `i` calls [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) on its next artifact; skip empty buckets. These artifact markers do not lock files.
4. Sub-agents work only within the reviewed assignments; orchestrator monitors [`forgeplan_claims`](/docs/mcp/forgeplan_claims/).
5. [`forgeplan_release`](/docs/mcp/forgeplan_release/) on completion → re-dispatch and repeat the cross-bucket comparison.

## CLI equivalent

[`forgeplan dispatch`](/docs/cli/) - same engine. Shell-driven orchestrators use the CLI;
LLM-driven orchestrators use the MCP tool.

## See also

- [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) - sub-agent picks a bucket item
- [`forgeplan_release`](/docs/mcp/forgeplan_release/) - return a slot to the pool
- [`forgeplan_claims`](/docs/mcp/forgeplan_claims/) - monitor in-flight work

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Compute a parallel-safe work plan for N sub-agents. Returns buckets (one per agent), a serial queue for leftover work, and human-readable reasoning for every placement decision. Skips artifacts already claimed, defers artifacts with file-overlap >= threshold (default Jaccard 0.3), and when agent_skills is provided routes by domain match. Read-only — does not mutate workspace state.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `agent_skills` | no | array | Per-agent skill lists in index order. Empty entries default to<br>"any skill"; omit to disable skill matching entirely. |
| `agents` | yes | integer | Number of sub-agents the orchestrator can hand work to. Required;<br>clamped to `&gt;= 1` downstream. |
| `epic` | no | string \| null | Optional filter: only artifacts with this parent Epic ID. Matches<br>the `parent_epic` frontmatter field. |
| `kind` | no | string \| null | Optional filter: only consider artifacts of this kind<br>(`prd`/`rfc`/`spec`/etc.). When omitted, all kinds are considered. |
| `overlap_threshold` | no | number \| null | Jaccard threshold above which two artifacts are considered<br>file-conflicting. Default 0.3. Clamp 0.0..=1.0. |
| `status` | no | string \| null | Optional filter: consider only artifacts in this status (default<br>`draft`). Set to `"any"` to include every lifecycle state. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "agent_skills": {
      "default": [],
      "description": "Per-agent skill lists in index order. Empty entries default to\n\"any skill\"; omit to disable skill matching entirely.",
      "items": {
        "items": {
          "type": "string"
        },
        "type": "array"
      },
      "type": "array"
    },
    "agents": {
      "description": "Number of sub-agents the orchestrator can hand work to. Required;\nclamped to `>= 1` downstream.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "epic": {
      "description": "Optional filter: only artifacts with this parent Epic ID. Matches\nthe `parent_epic` frontmatter field.",
      "type": [
        "string",
        "null"
      ]
    },
    "kind": {
      "description": "Optional filter: only consider artifacts of this kind\n(`prd`/`rfc`/`spec`/etc.). When omitted, all kinds are considered.",
      "type": [
        "string",
        "null"
      ]
    },
    "overlap_threshold": {
      "description": "Jaccard threshold above which two artifacts are considered\nfile-conflicting. Default 0.3. Clamp 0.0..=1.0.",
      "format": "double",
      "type": [
        "number",
        "null"
      ]
    },
    "status": {
      "description": "Optional filter: consider only artifacts in this status (default\n`draft`). Set to `\"any\"` to include every lifecycle state.",
      "type": [
        "string",
        "null"
      ]
    },
    "workspace": {
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "agents"
  ],
  "title": "DispatchParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
