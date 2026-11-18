---
title: "forgeplan constellation export"
description: "Stage 9 — portable export: every document and edge of the active constellation (or a recorded snapshot) as JSONL or SCIP-shaped records, with provenance and no absolute local paths"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation export --help`.

```text
Stage 9 — portable export: every document and edge of the active constellation (or a recorded snapshot) as JSONL or SCIP-shaped records, with provenance and no absolute local paths

Usage: forgeplan constellation export [OPTIONS]

Options:
      --format <FORMAT>      Record format: `jsonl` (constellation-export-v1) or `scip` [default: jsonl]
      --snapshot <SNAPSHOT>  Export at a recorded snapshot instead of the live sources
      --allow-partial        Tolerate dead sources: even a required source's outage becomes a `failures` record in the stream instead of an error
      --out <OUT>            Write to this file instead of stdout
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
