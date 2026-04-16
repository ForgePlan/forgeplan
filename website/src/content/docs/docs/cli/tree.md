---
title: forgeplan tree
description: "Show artifact hierarchy as ASCII tree — decomposition view"
---

Render the artifact hierarchy as an ASCII tree. Use this to understand how
work is decomposed: Epic → PRD → RFC → ADR, plus attached Specs, Evidence,
and Notes. Parent/child relationships come from the `parent` frontmatter
field and typed [`forgeplan link`](/docs/cli/link/) edges.

## When to use

- Onboarding a new contributor — "show me the decomposition"
- Auditing an Epic before a sprint — see all children in one view
- Preparing a release summary — all PRDs grouped by Epic
- Spot-checking orphans — artifacts without a parent float at the top

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
      --depth <DEPTH>  Maximum depth (default: unlimited) [default: 99]
      --json           Output as JSON
  -h, --help           Print help
  -V, --version        Print version
```

## Examples

Full forest — every root artifact in the workspace:

```bash
forgeplan tree
```

Just one Epic and everything under it:

```bash
forgeplan tree EPIC-003
```

Shallow view — only direct children:

```bash
forgeplan tree EPIC-003 --depth 1
```

## Output interpretation

The tree uses standard box-drawing characters. A typical fragment:

```
EPIC-003 [active]  Search, Discovery, Intelligence
├── PRD-039 [active]  BM25 production search
│   ├── RFC-004 [active]  Layered search architecture
│   │   └── ADR-007 [active]  Choose bm25 crate v2.3.2
│   └── EVID-018 [active]  Benchmark results
└── PRD-040 [active]  Scoring intelligence
    └── RFC-005 [draft]   Graph expansion
```

Each node shows: `<ID> [<status>]  <title>`. Children are indented under
their parent. A child linked to multiple parents appears under each — that
is expected and reveals shared dependencies.

Orphans (artifacts without a parent) appear at the top level — if you see
a `PRD-007` floating next to `EPIC-003`, it is a candidate for either
parenting under an existing Epic or a new one.

## How it fits

`tree` answers "what is the shape of this project?" — the static structure.
Use it together with:

- `graph` — for arbitrary typed edges (not only parent/child)
- `order` — when you need to know what to build first

## See also

- [`forgeplan graph`](/docs/cli/graph/) — full Mermaid diagram
- [`forgeplan order`](/docs/cli/order/) — topological execution order
- [`forgeplan blocked`](/docs/cli/blocked/) — what cannot start yet
- [Artifact model](/docs/methodology/overview/)
