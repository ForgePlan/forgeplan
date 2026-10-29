---
title: "forgeplan constellation search"
description: "Search across the active constellation's sources"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation search --help`.

```text
Search across the active constellation's sources

Usage: forgeplan constellation search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  

Options:
  -n, --limit <LIMIT>        Maximum hits (default: 10) [default: 10]
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
