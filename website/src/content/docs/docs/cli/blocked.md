---
title: forgeplan blocked
description: "Show blocked artifacts and what is blocking them"
---

List unmet dependencies on structural relations: `based_on`, `refines`, `supersedes` and `contradicts`. Targets in `active`, `deprecated` or `superseded` count as resolved; draft, stale and missing targets can block.

## When to use

- Sprint start - identify work that is stuck before assigning
- Daily standup - "what is waiting on me?"
- CI reporting — inspect the JSON counts explicitly; findings alone do not set a failing exit code
- Dependency audit during a release freeze

## Not to use when

- You want full execution order → use [`forgeplan order`](/docs/cli/order/)
- You want quality blind spots → use [`forgeplan blindspots`](/docs/cli/blindspots/)
- You want raw dependency graph → use [`forgeplan graph`](/docs/cli/graph/)

## Usage

```text
forgeplan blocked [OPTIONS] [ID]
```

## Arguments

```text
  [ID]  Specific artifact ID to check (optional)
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

Show everything that is blocked in the workspace:

```bash
forgeplan blocked
```

Check one specific artifact:

```bash
forgeplan blocked PRD-046
```

Machine-readable for a dashboard:

```bash
forgeplan blocked --json | jq '.blocked[] | .id'
```

## Output interpretation

Illustrative dependency conditions, not a literal current CLI transcript:

```text
PRD-046  Docs v0.18.0 catch-up         [draft]
  blocked by:
    RFC-005  Graph expansion           [draft]     ← not yet active
    ADR-008  Cloudflare Pages choice   [draft]     ← not yet active

RFC-007  Embed cache policy            [draft]
  blocked by:
    EVID-019 Benchmark results         [missing]   ← referenced but does not exist
```

| Indicator  | Meaning                                              |
|------------|------------------------------------------------------|
| `[draft]`  | Blocker exists but is not yet active                 |
| `[stale]`  | Blocker exceeded its `valid_until` date              |
| `[deprecated]` | Resolved for blocking purposes; review relevance separately |
| `[missing]` | Referenced ID does not exist - broken link          |

Successful reporting returns exit code 0 even when blockers exist. Workspace-wide JSON has `blocked`, `blocked_count`, `ready`, `ready_count` and `cycles`; with an ID it has `id`, `blocked` and `blocked_by`.

## How it fits

`blocked` is the _runtime_ view of the dependency graph:

```
order    →  graph walk and ready/blocked classification
blocked  →  current obstacles (considers status)
graph    →  complete picture
```

Combine with [`order`](/docs/cli/order/) to re-plan around blockers, and with
[`health`](/docs/cli/health/) to see how much of the project is stuck.

## See also

- [`forgeplan order`](/docs/cli/order/) - topological sequence
- [`forgeplan graph`](/docs/cli/graph/) - full dependency graph
- [`forgeplan blindspots`](/docs/cli/blindspots/) - decisions without evidence
- [`forgeplan health`](/docs/cli/health/) - project rollup

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan blocked --help`.

```text
Show blocked artifacts and their dependencies

Usage: forgeplan blocked [OPTIONS] [ID]

Arguments:
  [ID]  Specific artifact ID to check (optional)

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
