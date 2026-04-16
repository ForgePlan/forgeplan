---
title: forgeplan unlink
description: "Remove a relation between two artifacts"
---

Remove a typed relation between two artifacts. The inverse of
[`forgeplan link`](/docs/cli/link/). Use `unlink` to fix mistakes — wrong
direction, wrong relation type, stale edge left over from a superseded
decision — without rewriting markdown by hand.

## Usage

```text
forgeplan unlink [OPTIONS] <SOURCE> <TARGET>
```

## Arguments

```text
  <SOURCE>  Source artifact ID
  <TARGET>  Target artifact ID
```

## Options

```text
      --relation <RELATION>  Relationship type to remove [default: informs]
  -h, --help                 Print help
  -V, --version              Print version
```

## What it does

1. Looks up the relation in the LanceDB `links` table keyed on
   `(source, target, relation)`.
2. Deletes the row if found.
3. Refreshes any derived caches — `score`, `graph`, and `blocked` will reflect
   the change on next run.
4. Exits cleanly even if no matching relation exists, making the command
   idempotent and safe to script.

Unlinking does **not** delete either artifact. It only removes the edge.

## Examples

Fix a wrong-direction link:

```bash
forgeplan unlink PRD-001 EVID-001 --relation informs
forgeplan link EVID-001 PRD-001 --relation informs
```

Remove a stale `based_on` after superseding the parent:

```bash
forgeplan unlink RFC-006 PRD-025 --relation based_on
forgeplan link RFC-006 PRD-030 --relation based_on
```

Drop a `contradicts` edge that was recorded in error:

```bash
forgeplan unlink EVID-017 ADR-004 --relation contradicts
```

## Relation type must match

The `--relation` flag must match the type of the edge you want to remove. If
you linked `EVID-001 → PRD-001` with `--relation informs`, calling
`forgeplan unlink EVID-001 PRD-001 --relation based_on` will be a no-op because
no such edge exists.

When in doubt, inspect the graph first:

```bash
forgeplan graph PRD-001         # see all edges touching PRD-001
forgeplan show PRD-001 --links  # list links in tabular form
```

## Self-link guard (PROB-019)

Since self-links cannot be created (see [`forgeplan link`](/docs/cli/link/)),
`forgeplan unlink PRD-001 PRD-001` will never find a match and exits as a
no-op.

## Side effects

- [`forgeplan score`](/docs/cli/score/) recomputes R_eff for the target. If
  you remove the only supporting `informs` edge, the score drops to 0 and the
  artifact becomes a blind spot on [`forgeplan health`](/docs/cli/health/).
- [`forgeplan blocked`](/docs/cli/blocked/) and
  [`forgeplan order`](/docs/cli/order/) re-run topological sort.
- The Mermaid graph from [`forgeplan graph`](/docs/cli/graph/) loses the edge.

## Notes

- Unlink is idempotent — running it twice has the same effect as running it
  once. No error on a missing edge.
- To remove an artifact entirely (including all its links), use
  [`forgeplan delete`](/docs/cli/delete/) — it cascade-deletes relations in a
  single pass.
- Direct edits to `.forgeplan/<kind>s/<id>.md` never touch the `links` table.
  If you need to hand-edit relations, run
  [`forgeplan scan-import`](/docs/cli/scan-import/) afterward.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan link`](/docs/cli/link/) — create a relation
- [`forgeplan graph`](/docs/cli/graph/) — visualize current relations
- [`forgeplan score`](/docs/cli/score/) — recompute R_eff after unlinking
- [Methodology guide](/docs/methodology/overview/)
