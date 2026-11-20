---
title: "forgeplan constellation impact"
description: "Stage 7 — explainable impact: who depends on this artifact, by which relation path, and where the bounded walk stopped"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation impact --help`.

```text
Stage 7 — explainable impact: who depends on this artifact, by which relation path, and where the bounded walk stopped

Usage: forgeplan constellation impact [OPTIONS] <ID>

Arguments:
  <ID>  The changed artifact's canonical key or display id

Options:
      --depth <DEPTH>          Maximum structural hops (default 4) [default: 4]
      --max-nodes <MAX_NODES>  Maximum impacted nodes (default 256) [default: 256]
      --snapshot <SNAPSHOT>    Walk at a recorded snapshot instead of the live sources
      --allow-partial          Tolerate dead sources: even a required source's outage becomes a `failures` entry on a partial answer instead of an error
      --json                   Output as JSON
  -h, --help                   Print help
  -V, --version                Print version
```
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
