---
title: forgeplan order
description: "Inspect the structural dependency graph, cycles and ready/blocked classification"
---

Inspect linked artifacts using structural relations `based_on`, `refines`,
`supersedes` and `contradicts`. Informational `informs` edges do not block.
Only nodes present in those structural edges enter the result; this is not an
inventory of every artifact.

## When to use

- Sprint planning — inspect dependency paths and readiness before choosing work
- Dependency auditing - spot missing edges that should exist
- CI reporting — inspect `cycles` explicitly; this command does not fail merely because a cycle exists

## Not to use when

- You want to see only _blocked_ items → use [`forgeplan blocked`](/docs/cli/blocked/)
- You want hierarchy → use [`forgeplan tree`](/docs/cli/tree/)
- You want all edges → use [`forgeplan graph`](/docs/cli/graph/)

## Usage

```text
forgeplan order [OPTIONS]
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

Print the full topological order:

```bash
forgeplan order
```

Pipe into a sprint doc template:

```bash
forgeplan order --json | jq -r '.order[] | "- [ ] \(.)"'
```

## Output interpretation

The text report marks active (`v`), blocked (`x`) and other ready (`o`) artifacts.
Rows contain IDs and status/readiness, not a title column. JSON contains `order`
(an array of IDs), `ready`, `blocked`, `cycles` and `_next_action`.

Do not treat the displayed walk as a verified dependency-first execution plan:
the current implementation traverses stored source→target adjacency, while a
source declares a dependency on its target. Check the dependency edges and
`ready`/`blocked` classification before scheduling work.

Cycles are printed as warnings and included in JSON; successful reporting still
returns 0. Correct the relevant structural relation, not an unsupported `blocks` edge.

## How it fits

`order` is a dependency-inspection view. Pair with `blocked`:

```
order    → graph walk, cycles and readiness
blocked  → what is stuck right now
```

`active`, `deprecated` and `superseded` dependencies count as resolved.
No blockers does not prove the workspace is otherwise healthy, and `done` is not a core persisted lifecycle status.

## See also

- [`forgeplan blocked`](/docs/cli/blocked/) - unresolved blockers
- [`forgeplan tree`](/docs/cli/tree/) - hierarchy view
- [`forgeplan graph`](/docs/cli/graph/) - full edge graph

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan order --help`.

```text
Show artifacts in topological order (dependency order)

Usage: forgeplan order [OPTIONS]

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
