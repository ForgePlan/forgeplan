---
title: forgeplan order
description: "Report the structural relation graph, readiness, blockers, and cycles. Includes based_on, refines, supersedes, and contradicts; the 0.38.0 traversal is not a prerequisite-first execution queue."
---

Report the relation graph using structural relations (`based_on`, `refines`, `supersedes`, `contradicts`). `informs` does not schedule work. Only nodes in structural edges appear; unlinked artifacts are absent.

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

```bash
forgeplan order
forgeplan order --json | jq -r '.order[] | "- [ ] \(.)"'
forgeplan order --json | jq '.cycles, .blocked, .ready'
```

## Output and exit behavior

JSON has `order` (an array of IDs), `cycles`, `blocked` (id and blocked_by), `ready`, and `_next_action`. Text marks active, ready, or blocked entries with their real lifecycle status. The command reports cycles without returning a failing exit status; inspect `.cycles` explicitly for a CI policy. Active, deprecated, and superseded dependencies count as resolved.

In 0.38.0 the Kahn traversal appends an edge's source before its target, while readiness treats the source as depending on that target. Consequently, do not execute `.order` as a prerequisite-first work queue. Use `blocked_by` and `ready` to inspect unresolved dependencies. Nodes omitted by cyclic traversal may not appear in those classifications; inspect `cycles` as well.

## Planning

Use `blocked` to focus on current obstacles, `tree` for hierarchy, and `graph` for the complete relation graph. Dependency readiness does not mean validation or evidence gates have passed.

## See also

- [`forgeplan blocked`](/docs/cli/blocked/) - unresolved blockers
- [`forgeplan tree`](/docs/cli/tree/) - hierarchy view
- [`forgeplan graph`](/docs/cli/graph/) - full edge graph
