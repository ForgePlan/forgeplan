---
title: forgeplan blocked
description: "Show blocked artifacts and what is blocking them"
---

List artifacts that cannot make progress because one or more of their
dependencies is not yet `active` (or worse — `deprecated`/`stale`). For each
blocked artifact, the blocker chain is printed so you can see exactly what
to unblock.

## When to use

- Sprint start — identify work that is stuck before assigning
- Daily standup — "what is waiting on me?"
- CI gate — fail if anything critical-depth is blocked
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
forgeplan blocked --json | jq '.[] | .id'
```

## Output interpretation

```
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
| `[deprecated]` | Blocker was deprecated — update the edge         |
| `[missing]` | Referenced ID does not exist — broken link          |

Exit code is 0 if nothing is blocked, 1 otherwise — handy for CI gates.

## How it fits

`blocked` is the _runtime_ view of the dependency graph:

```
order    →  planned sequence (ignores status)
blocked  →  current obstacles (considers status)
graph    →  complete picture
```

Combine with [`order`](/docs/cli/order/) to re-plan around blockers, and with
[`health`](/docs/cli/health/) to see how much of the project is stuck.

## See also

- [`forgeplan order`](/docs/cli/order/) — topological sequence
- [`forgeplan graph`](/docs/cli/graph/) — full dependency graph
- [`forgeplan blindspots`](/docs/cli/blindspots/) — decisions without evidence
- [`forgeplan health`](/docs/cli/health/) — project rollup
