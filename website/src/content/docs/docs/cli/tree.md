---
title: forgeplan tree
description: "Show artifact hierarchy as ASCII tree - decomposition view"
---

Render the artifact hierarchy as an ASCII tree. Use this to understand how
work is decomposed: Epic → PRD → RFC → ADR, plus attached Specs, Evidence,
and Notes. The first eligible typed relation selects a parent (source is the child,
target is the parent); `parent_epic` is a fallback when no relation selected one.

## When to use

- Onboarding a new contributor - "show me the decomposition"
- Auditing an Epic before a sprint - see all children in one view
- Preparing a release summary - all PRDs grouped by Epic
- Spot-checking orphans - artifacts without a parent float at the top

## Not to use when

- You need cross-link edges (parent/child only is not enough) → use [`forgeplan graph`](/docs/cli/graph/)
- You need execution order → use [`forgeplan order`](/docs/cli/order/)
- You need a flat list → use [`forgeplan list`](/docs/cli/list/)

## Usage

```text
forgeplan tree [OPTIONS] [ID]
```

## Arguments

```text
  [ID]  Root artifact ID (shows all roots if omitted)
```

## Options

```text
      --depth <DEPTH>  Maximum depth (default: unlimited) [default: 99]
      --json           Output as JSON
  -h, --help           Print help
  -V, --version        Print version
```

## Examples

Full forest - every root artifact in the workspace:

```bash
forgeplan tree
```

Just one Epic and everything under it:

```bash
forgeplan tree EPIC-003
```

Shallow view - only direct children:

```bash
forgeplan tree EPIC-003 --depth 1
```

## Output interpretation

Text output places `PROGRESS`, `R_EFF`, `STATUS`, and `KIND` columns to the left of the tree, followed by `<ID> "<title>"`. The progress bar visualizes the stored R_eff; it is not task-checkbox completion. Evidence, Note, and Refresh rows show dots instead of a score bar.

Illustrative JSON for an explicitly selected root (invented values, not an executed run):

```json
{
  "id": "EPIC-003",
  "title": "Search, Discovery, Intelligence",
  "kind": "epic",
  "status": "active",
  "r_eff": 0.5,
  "children": [
    {
      "id": "PRD-039",
      "title": "BM25 production search",
      "kind": "prd",
      "status": "draft",
      "r_eff": 0.0,
      "children": []
    }
  ]
}
```

`--json` without an ID returns an array of root nodes; with an ID it returns one node. The hint goes to stderr. At the depth limit, `children` is empty. Default depth is 99, not literally unlimited. JSON `kind` is derived from the ID prefix, while text uses the stored kind.

Eligible relation types are `based_on`, `refines`, `informs`, `belongs_to`, and `child_of`. Each child is placed under only the first parent encountered, not under every linked parent; use `graph` to inspect all edges. Cycles can hide a component from the root forest, so `tree` is not a cycle validator. Text mode rejects an unknown explicit ID; JSON mode instead builds a placeholder node with empty title and `status: "unknown"`.

Orphans (artifacts without a parent) appear at the top level - if you see
a `PRD-007` floating next to `EPIC-003`, it is a candidate for either
parenting under an existing Epic or a new one.

## How it fits

`tree` answers "what is the shape of this project?" - the static structure.
Use it together with:

- `graph` for arbitrary typed edges (parent/child only is not enough)
- `order` when you need to know what to build first

## See also

- [`forgeplan graph`](/docs/cli/graph/) - full Mermaid diagram
- [`forgeplan order`](/docs/cli/order/) - topological execution order
- [`forgeplan blocked`](/docs/cli/blocked/) - what cannot start yet
- [Artifact model](/docs/methodology/overview/)
