---
title: forgeplan dispatch
description: "Compute a parallel-safe work plan for N sub-agents - buckets, serial queue, reasoning. Read-only; the entry point for multi-agent orchestration."
---

`forgeplan dispatch` proposes buckets for the available agents and a serial queue. It is advisory, not a file lock or a guarantee that different agents' buckets are disjoint. In this source revision, overlap checks compare a candidate only with items already in the same bucket; overlapping artifacts can land in different buckets. Review cross-bucket file ownership before parallel execution.

How the planner avoids conflicts:

- **Active claims are skipped** - if another agent has already claimed an artifact (see [`forgeplan claim`](/docs/cli/claim/)), it is excluded from the plan.
- **File overlap is checked within each bucket** using Jaccard similarity, `|A ∩ B| / |A ∪ B|`, over declared path strings. A candidate moves to the serial queue only if no bucket fits; missing `affected_files` is serialized. Globs are not expanded into an exhaustive write set.
- **Structural dependencies are checked** - unresolved blockers are omitted from parallel buckets. This is a snapshot, not a lock against subsequent changes.
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

Returns up to three populated buckets. Review overlaps across buckets before assigning them. The orchestrator hands `buckets[0]` to agent 0, `buckets[1]` to agent 1, and so on; empty buckets are possible.

### Example 2: Replan an entire Epic, regardless of status

```bash
forgeplan dispatch --agents 4 --epic EPIC-005 --status any
```

Plans across all artifacts in `EPIC-005`, including active and superseded ones (not just drafts). Useful for retrospectives or when an Epic mixes draft and active work that all need attention.

### Example 3: Stricter conflict detection

```bash
forgeplan dispatch --agents 2 --overlap-threshold 0.15 --json
```

Lowers the Jaccard threshold to 0.15. This rejects more within-bucket placements but does not fix the cross-bucket limitation described above. Claims are coordination signals, not exclusive file access.

## How it fits the workflow

Multi-agent work is a four-step loop: `dispatch` → `claim` → work → `release` → `dispatch` again. The orchestrator owns `dispatch` (and `release --force` for cleaning up after crashed agents); each sub-agent owns `claim` and `release` for the artifact it is working on. Use [`forgeplan claims`](/docs/cli/claims/) between dispatches to see who is working on what.

## See also

- [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) - MCP equivalent
- [`forgeplan claim`](/docs/cli/claim/) - sub-agent picks a bucket item
- [`forgeplan release`](/docs/cli/release/) - return a slot to the pool
- [`forgeplan claims`](/docs/cli/claims/) - monitor in-flight work
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan dispatch --help`.

```text
Compute a parallel-safe work plan for N sub-agents (PRD-057 dispatcher)

Usage: forgeplan dispatch [OPTIONS] --agents <AGENTS>

Options:
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
<!-- forgeplan-runtime:end -->
