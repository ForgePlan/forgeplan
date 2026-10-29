---
title: forgeplan unlink
description: "Remove a relation between two artifacts"
---

Remove a typed relation between two artifacts. The inverse of
[`forgeplan link`](/docs/cli/link/). Use `unlink` to fix mistakes - wrong
direction, wrong relation type, stale edge left over from a superseded
decision - without rewriting markdown by hand.

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

The command removes the matching `(source, target, relation)` edge and
updates the canonical projection. On a Constellation-owned workspace in
local `0.39.0+constellation.s1.1`, it edits the source file's `links:`
frontmatter and requests reconcile without reopening legacy storage.

A missing edge is an error (`Relation '…' from … to … not found`), not a
successful no-op. A second identical invocation therefore need not succeed.

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

`--relation` must match the recorded edge. If the link uses `informs`,
requesting removal of `based_on` reports that the relation was not found.

When in doubt, inspect the graph first:

```bash
forgeplan graph                 # workspace graph; no positional artifact ID
forgeplan get PRD-001 --json     # inspect links in the artifact response
```

## Self-link guard (PROB-019)

Self-links are rejected at creation. Attempting to unlink a nonexistent
self-link therefore reports a missing relation, not a successful no-op.

## Side effects

- [`forgeplan score`](/docs/cli/score/) recomputes R_eff. Removing the only
  supporting `informs` edge can remove the decision's evidence support.
- [`forgeplan blocked`](/docs/cli/blocked/) and
  [`forgeplan order`](/docs/cli/order/) reread dependency structure.
- [`forgeplan graph`](/docs/cli/graph/) no longer shows the removed edge.

## Notes

- Removing a missing edge reports an error; scripts must handle that outcome explicitly.
- Unlink removes an edge, not an artifact. Do not delete the artifact to correct a relation.
- Prefer `link`/`unlink` to direct frontmatter edits. For derived-state maintenance,
  use `forgeplan index` on Constellation or legacy `reindex` as appropriate, not `scan-import`.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan link`](/docs/cli/link/) - create a relation
- [`forgeplan graph`](/docs/cli/graph/) - visualize current relations
- [`forgeplan score`](/docs/cli/score/) - recompute R_eff after unlinking
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan unlink --help`.

```text
Remove a relation between two artifacts

Usage: forgeplan unlink [OPTIONS] <SOURCE> <TARGET>

Arguments:
  <SOURCE>  Source artifact ID
  <TARGET>  Target artifact ID

Options:
      --relation <RELATION>  Relationship type to remove [default: informs]
  -h, --help                 Print help
  -V, --version              Print version
```
<!-- forgeplan-runtime:end -->
