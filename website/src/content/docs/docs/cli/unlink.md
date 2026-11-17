---
title: forgeplan unlink
description: "Remove one directed, typed relation without deleting either artifact. Source, target, and relation must identify an existing edge. Unlike an idempotent “remove if present” operation, CLI 0.38.0 refuses a missing edge."
---

Remove one directed, typed relation without deleting either artifact. Source, target, and relation must identify an existing edge. Unlike an idempotent “remove if present” operation, CLI 0.38.0 refuses a missing edge.

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

## Matching and identifiers

`--relation` defaults to `informs`. A different type or reversed direction does not match: the command returns a relation-not-found error rather than succeeding as a no-op. This path compares stored IDs case-insensitively but does not resolve slugs like `link` does; copy the stored IDs from `get` or `graph --json`. Repeating a successful unlink fails because the edge is now absent.

## Examples

```bash
# Inspect real endpoints and relation types first:
forgeplan get PRD-001
forgeplan graph --json
# Only if this wrong-direction edge actually exists:
forgeplan unlink PRD-001 EVID-001 --relation informs
forgeplan link EVID-001 PRD-001 --relation informs
forgeplan score PRD-001
```

## Side effects and recovery

The command deletes the relation and refreshes both participants’ Markdown projections on a best-effort basis. It requests a cached-score refresh for the source; explicitly score the affected decision after changing evidence links. Other graph reports reflect the new edge set when run again. It logs the unlink; it does not delete artifacts or undo a lifecycle transition. Removing a self-edge that was never created also fails as missing.

## Prefer relation commands to manual edits

Use `link --replace` to change a relation type on an existing source/target pair, or `unlink` followed by `link` to change direction. Reindex adds missing frontmatter links, but removing a link from Markdown alone does not necessarily delete the stored edge. Do not rely on reindex as a replacement for unlink.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan link`](/docs/cli/link/) - create a relation
- [`forgeplan graph`](/docs/cli/graph/) - visualize current relations
- [`forgeplan score`](/docs/cli/score/) - recompute R_eff after unlinking
- [Methodology guide](/docs/methodology/overview/)
