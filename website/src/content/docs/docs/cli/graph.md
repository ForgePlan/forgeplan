---
title: forgeplan graph
description: "Generate a Mermaid dependency graph of linked artifacts"
---

Emit a Mermaid-format dependency graph of all linked artifacts. Unlike
[`tree`](/docs/cli/tree/), which only shows parent/child hierarchy, `graph`
renders every typed link (`informs`, `supersedes`, `blocks`, `relates_to`,
etc.) as an edge — the full decision DAG.

## When to use

- Paste into a GitHub issue/PR to visualize a proposal's impact
- Embed in an RFC or Epic to show scope
- Feed into `mmdc` (mermaid-cli) to render PNG/SVG
- Detect unexpected cycles or orphan clusters

## Not to use when

- You just need parent/child decomposition → use [`forgeplan tree`](/docs/cli/tree/)
- You need execution order → use [`forgeplan order`](/docs/cli/order/)
- You need a flat list → use [`forgeplan list`](/docs/cli/list/)

## Usage

```text
forgeplan graph [OPTIONS]
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

Emit the Mermaid source:

```bash
forgeplan graph
```

Redirect to a file and paste into GitHub:

```bash
forgeplan graph > deps.mmd
```

Render to PNG with mermaid-cli:

```bash
forgeplan graph > deps.mmd && mmdc -i deps.mmd -o deps.png
```

JSON for a custom UI:

```bash
forgeplan graph --json | jq '.edges | length'
```

## Output interpretation

Standard output is Mermaid `graph TD` syntax:

```
graph TD
  EPIC_003[EPIC-003<br/>Search, Discovery]
  PRD_039[PRD-039<br/>BM25 production]
  RFC_004[RFC-004<br/>Layered search]
  EVID_018[EVID-018<br/>Benchmark]
  EPIC_003 --> PRD_039
  PRD_039 --> RFC_004
  EVID_018 -.informs.-> PRD_039
```

Conventions:

| Syntax          | Meaning                                            |
|-----------------|----------------------------------------------------|
| `A --> B`       | Parent/child (structural)                          |
| `A -.informs.-> B` | Typed edge from link frontmatter                |
| `A -.supersedes.-> B` | Lifecycle edge, newer supersedes older       |
| Node label      | `<ID><br/><title>`                                 |

Active artifacts get a solid outline; superseded/deprecated ones get a dashed
outline (if your renderer supports the class definitions emitted at the top
of the graph).

With `--json`, the envelope is `{ nodes: [...], edges: [...] }` — ready for
Cytoscape, D3, or any graph library.

## How it fits

`graph` is the complete relational view. For different questions:

```
graph   →  all edges              (relational picture)
tree    →  parent/child only       (hierarchy)
order   →  topological walk        (execution)
blocked →  unresolved dependencies (planning)
```

## See also

- [`forgeplan tree`](/docs/cli/tree/) — hierarchy view
- [`forgeplan order`](/docs/cli/order/) — topological order
- [`forgeplan blocked`](/docs/cli/blocked/) — blocking dependencies
- [`forgeplan link`](/docs/cli/link/) — create typed edges
