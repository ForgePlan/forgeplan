---
title: forgeplan dispatch
description: "Build an advisory work plan for N sub-agents with buckets, a serial queue, and reasoning. Read-only; does not guarantee conflict-free work across agents."
---

`forgeplan dispatch` distributes candidate artifact IDs across one bucket per agent and a serial queue. It is a planning heuristic, not a file lock or a guarantee that simultaneous workers cannot collide.

How the planner avoids conflicts:

- **Active claims are skipped** - if another agent has already claimed an artifact (see [`forgeplan claim`](/docs/cli/claim/)), it is excluded from the plan.
- **Declared file overlap is checked within each bucket** using Jaccard similarity (`intersection / union`, conflict at or above the threshold). The released implementation can put overlapping artifacts in different buckets, so the orchestrator must review cross-bucket overlaps before starting workers. Paths are compared as strings, not expanded globs. Missing file declarations go to the serial queue.
- **Structural dependencies are considered** - candidates in the topological blocked set are deferred. Active, deprecated, and superseded parents count as resolved; this is not a check that implementation work is complete.
- **Optional filters** - `--epic` / `--kind` narrow the candidate set to one Epic or one artifact kind.

Read-only: this command never modifies the workspace. After dispatch, each agent must call [`forgeplan claim`](/docs/cli/claim/) on its bucket item before touching files. Mirrors [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) on the MCP side - LLM-driven orchestrators usually use the MCP tool, shell scripts use this CLI.

## When to use

- Start of a multi-agent sprint with 2–5 sub-agents that should work without stomping on each other.
- After a [`forgeplan release`](/docs/cli/release/) - slot opened up, replan to fill it.
- After [`forgeplan new`](/docs/cli/new/) created fresh draft artifacts - replan so the new ones get assigned.
- A stuck claim expired (TTL ran out) - replan to assign the artifact to a fresh agent.

## When NOT to use

- You have only one agent - there is no plan to compute, just pick the next artifact.
- You expect this to claim or modify state - it does not. After dispatch, each agent must run [`forgeplan claim`](/docs/cli/claim/) explicitly.
- Without first checking [`forgeplan claims`](/docs/cli/claims/) - dispatch already excludes claimed work, but knowing what is in flight helps you set the right `--agents` count.

## Usage

```text
forgeplan dispatch [OPTIONS] --agents <AGENTS>
```

## Options

```text
  -n, --agents <AGENTS>
          Number of sub-agents the orchestrator can hand work to (>=1, max 64)
      --epic <EPIC>
          Optional filter: only artifacts with this parent Epic ID
  -t, --kind <KIND>
          Optional filter: only consider artifacts of this kind (prd/rfc/spec/...)
  -s, --status <STATUS>
          Status filter (default `draft`; pass `any` for all states) [default: draft]
      --overlap-threshold <OVERLAP_THRESHOLD>
          Jaccard threshold for file-overlap conflict detection (default 0.3) [default: 0.3]
      --json
          Output as JSON for machine consumption
  -h, --help
          Print help
  -V, --version
          Print version
```

## Examples

### Example 1: Plan 3 agents on draft PRDs

```bash
forgeplan dispatch --agents 3 --kind prd
```

Returns three buckets (possibly empty). The orchestrator hands `buckets[0]` to agent 0 and so on, after checking conflicts between buckets. `affected_files` comes from projection frontmatter, falling back to the Affected Files body section; missing or unparseable projections are skipped.

### Example 2: Replan an entire Epic, regardless of status

```bash
forgeplan dispatch --agents 4 --epic EPIC-005 --status any
```

Plans across all artifacts in `EPIC-005`, including active and superseded ones. Useful for retrospectives or when an Epic mixes draft and active work that all need attention.

### Example 3: Stricter conflict detection

```bash
forgeplan dispatch --agents 2 --overlap-threshold 0.15 --json
```

Lowers the within-bucket conflict threshold to 0.15. A candidate is deferred only when no bucket accepts it; this does not enforce separation of overlapping paths across different agents.

The JSON object contains `buckets` (arrays of IDs), `serial_queue` (objects
with `id` and `reason`), `reasoning`, `generated_at`, `agent_count`,
`overlap_threshold`, `candidate_count`, `claimed_count`, `skipped_parse_errors`,
`blocked_count`, `_next_action`, and `hints`. `candidate_count` is measured
before claim exclusion; `claimed_count` counts all live workspace claims,
not just claims matching the filters. No agents are launched or claimed.
`--agents` must be 1–64 and the threshold must be in `[0, 1]`.

## How it fits the workflow

Multi-agent work is a four-step loop: `dispatch` → `claim` → work → `release` → `dispatch` again. The orchestrator owns `dispatch` (and `release --force` for cleaning up after crashed agents); each sub-agent owns `claim` and `release` for the artifact it is working on. Use [`forgeplan claims`](/docs/cli/claims/) between dispatches to see who is working on what.

## See also

- [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) - MCP equivalent
- [`forgeplan claim`](/docs/cli/claim/) - sub-agent picks a bucket item
- [`forgeplan release`](/docs/cli/release/) - return a slot to the pool
- [`forgeplan claims`](/docs/cli/claims/) - monitor in-flight work
- [CLI overview](/docs/cli/)
