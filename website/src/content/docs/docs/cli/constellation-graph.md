---
title: "forgeplan constellation graph"
description: "The relation graph of the active constellation (or a recorded snapshot): every artifact as a source-qualified node, every declared link as a typed edge with its provenance"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation graph --help`.

```text
The relation graph of the active constellation (or a recorded snapshot): every artifact as a source-qualified node, every declared link as a typed edge with its provenance

Usage: forgeplan constellation graph [OPTIONS]

Options:
      --limit <LIMIT>        Maximum nodes and edges each (default 500) [default: 500]
      --snapshot <SNAPSHOT>  Read at a recorded snapshot instead of the live sources
      --allow-partial        Tolerate dead sources: even a required source's outage becomes a `failures` entry on a partial answer instead of an error
      --json                 Output as JSON
  -h, --help                 Print help
  -V, --version              Print version
```
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
