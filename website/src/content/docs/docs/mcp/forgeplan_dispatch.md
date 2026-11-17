---
title: forgeplan_dispatch
description: "Propose agent buckets and a serial queue without starting workers. Cross-agent file overlap must be checked independently; the 0.38 planner does not guarantee parallel safety."
---

Computes a proposed plan with one bucket per agent and a serial queue; it does not start agents, create claims or change artifacts. Live claimed IDs are skipped regardless of claimant. Missing `affected_files` goes to the serial queue; domain matching is case-insensitive when skills are supplied.

**Released safety limit:** file-overlap checks use `>=` the Jaccard threshold (default 0.3), but compare a candidate only with items already in the same bucket. They do not check conflicts across different agents' buckets. Inspect cross-bucket file overlap yourself before concurrent execution; the plan is not a guarantee of conflict-free parallel work. Dependency checks likewise use the released structural graph; inspect cycles separately.

**Category**: Multi-agent

## When an agent calls it

- Start of a multi-agent sprint: orchestrator wants 2-5 sub-agents on non-overlapping work.
- After a [`forgeplan_release`](/docs/mcp/forgeplan_release/) - re-plan because the candidate set changed.
- After [`forgeplan_new`](/docs/mcp/forgeplan_new/) creates fresh drafts - incorporate them.
- TTL expiry on a stuck claim - re-dispatch to backfill the freed slot.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `agent_skills` | `array<array<string>>` | no | Per-agent skill lists in index order. Empty entries default to "any skill"; omit to disable skill matching entirely. Schema default: `[]`. |
| `agents` | `integer` | yes | Number of sub-agents the orchestrator can hand work to. Required; clamped to `>= 1` downstream. `format`: `uint`. `minimum`: `0`. |
| `epic` | `string \| null` | no | Optional filter: only artifacts with this parent Epic ID. Matches the `parent_epic` frontmatter field. |
| `kind` | `string \| null` | no | Optional filter: only consider artifacts of this kind (`prd`/`rfc`/`spec`/etc.). When omitted, all kinds are considered. |
| `overlap_threshold` | `number \| null` | no | Jaccard threshold above which two artifacts are considered file-conflicting. Default 0.3. Clamp 0.0..=1.0. `format`: `double`. |
| `status` | `string \| null` | no | Optional filter: consider only artifacts in this status (default `draft`). Set to `"any"` to include every lifecycle state. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The handler accepts 1–64 agents, at most one skill-list entry per agent and 32 skills per entry. It rejects zero/out-of-range agent counts and thresholds outside `[0,1]` with `isError` text rather than silently clamping. Default status is `draft`; exact `"any"` removes that filter. These handler checks are stricter than some schema descriptions above. The handler uses the write-capable workspace resolver even though it only computes a plan.

Illustrative JSON payload in MCP text content:

```json
{
  "buckets": [
    ["PRD-057"],
    ["RFC-012"]
  ],
  "serial_queue": [{"id": "SPEC-018", "reason": "missing affected_files frontmatter"}],
  "reasoning": [
    "PRD-057: assigned to agent 0 (no file conflict)",
    "RFC-012: assigned to agent 1 (no file conflict, skill match)",
    "SPEC-018: serialized (no affected_files declared — treated as shared-ground, deferred for safety)"
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
not be read - check server logs.

`candidate_count` is after frontmatter/epic/dependency filtering but before claim exclusion. `claimed_count` is the workspace-wide live-claim count, not the number of selected candidates excluded. `blocked_count` counts skipped blocked candidates; those are also appended as `{id, reason}` serial entries. Read every serial reason: dependency-blocked work is not ready merely because it is in the serial queue. Claims are advisory coordination, not write locks. No workspace trace is attached by this handler in this release.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "agents": 2,
  "kind": "rfc",
  "agent_skills": [
    [
      "rust"
    ],
    [
      "docs"
    ]
  ]
}
```

## Typical sequence

1. `forgeplan_dispatch agents=N` - orchestrator gets the plan.
2. Each sub-agent `i` calls [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) on `buckets[i][0]`.
3. Sub-agents work; orchestrator polls [`forgeplan_claims`](/docs/mcp/forgeplan_claims/).
4. [`forgeplan_release`](/docs/mcp/forgeplan_release/) on completion → re-dispatch.

## CLI equivalent

[`forgeplan dispatch`](/docs/cli/) - same engine. Shell-driven orchestrators use the CLI;
LLM-driven orchestrators use the MCP tool.

## See also

- [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) - sub-agent picks a bucket item
- [`forgeplan_release`](/docs/mcp/forgeplan_release/) - return a slot to the pool
- [`forgeplan_claims`](/docs/mcp/forgeplan_claims/) - monitor in-flight work
