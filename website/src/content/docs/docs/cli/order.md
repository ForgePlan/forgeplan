---
title: forgeplan order
description: "Topological sort of artifacts — what to build first"
---

Sort all artifacts into topological execution order using dependency edges
(`blocks`, `depends_on`, parent/child). The output is a linearized walk
suitable for sprint planning: "do PRD-002 before PRD-007, because PRD-007
depends on it."

## When to use

- Sprint planning — sequence multiple PRDs correctly
- Dependency auditing — spot missing edges that should exist
- CI gate — fail the build on dependency cycles

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
forgeplan order --json | jq -r '.[] | "- [ ] \(.id) \(.title)"'
```

## Output interpretation

One artifact per line, earliest-first (breadth-first tiebreaker):

```
1.  ADR-003  Files as source of truth       [active]
2.  PRD-039  BM25 production search         [active]
3.  RFC-004  Layered search architecture    [active]
4.  EVID-018 Benchmark results              [active]
5.  PRD-040  Scoring intelligence           [draft]
```

| Column  | Meaning                                               |
|---------|-------------------------------------------------------|
| Index   | Position in the topological walk                      |
| ID      | Artifact ID                                           |
| Title   | Short title                                           |
| Status  | `[active]`, `[draft]`, etc. — lets you skip terminal  |

If a cycle is detected, the command exits with status 1 and prints the cycle
path — fix the offending `blocks` / `depends_on` edge and re-run.

## How it fits

`order` is the "execute in this sequence" view. Pair with `blocked`:

```
order    → ideal sequence (ignores status)
blocked  → what is stuck right now
```

A healthy workspace: everything in `order[0:k]` is `[active]` or `[done]`,
and `blocked` is empty.

## See also

- [`forgeplan blocked`](/docs/cli/blocked/) — unresolved blockers
- [`forgeplan tree`](/docs/cli/tree/) — hierarchy view
- [`forgeplan graph`](/docs/cli/graph/) — full edge graph
