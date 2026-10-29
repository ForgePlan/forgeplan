---
title: forgeplan tree
description: "Show artifact hierarchy as ASCII tree - decomposition view"
---

Renders a single-parent projection of typed relations, with `parent_epic` as fallback when no parent edge is selected. The handler chooses the first recognized parent; it is not the full multi-parent dependency graph.

## When to use

- Onboarding a new contributor - "show me the decomposition"
- Auditing an Epic before a sprint - see all children in one view
- Preparing a release summary - all PRDs grouped by Epic
- Inspect roots, which are not necessarily linkless health orphans.

## Not to use when

- You need cross-link edges (not just parent/child) → use [`forgeplan graph`](/docs/cli/graph/)
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
      --depth <DEPTH>  Maximum depth [default: 99; finite, not unlimited]
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

Illustrative decomposition, not exact runtime columns (which also show kind, R_eff and progress):

```
EPIC-003 [active]  Search, Discovery, Intelligence
├── PRD-039 [active]  BM25 production search
│   ├── RFC-004 [active]  Layered search architecture
│   │   └── ADR-007 [active]  Choose bm25 crate v2.3.2
│   └── EVID-018 [active]  Benchmark results
└── PRD-040 [active]  Scoring intelligence
    └── RFC-005 [draft]   Graph expansion
```

A child is displayed under its first recognized parent, not under every linked parent. For all typed edges use `forgeplan graph`. A root may have meaningful links without a recognized parent; do not equate top-level placement with the health orphan test. The captured help says “unlimited”, but the actual default is the finite depth 99.

## How it fits

`tree` answers "what is the shape of this project?" - the static structure.
Use it together with:

- `graph` - for arbitrary typed edges (not only parent/child)
- `order` - when you need to know what to build first

## See also

- [`forgeplan graph`](/docs/cli/graph/) - full Mermaid diagram
- [`forgeplan order`](/docs/cli/order/) - topological execution order
- [`forgeplan blocked`](/docs/cli/blocked/) - what cannot start yet
- [Artifact model](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan tree --help`.

```text
Show artifact hierarchy as ASCII tree

Usage: forgeplan tree [OPTIONS] [ID]

Arguments:
  [ID]  Root artifact ID (shows all roots if omitted)

Options:
      --depth <DEPTH>  Maximum depth (default: unlimited) [default: 99]
      --json           Output as JSON
  -h, --help           Print help
  -V, --version        Print version
```
<!-- forgeplan-runtime:end -->
