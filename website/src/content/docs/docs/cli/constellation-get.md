---
title: "forgeplan constellation get"
description: "Fetch one artifact by id across the active constellation. When two sources claim the same id the answer is `ambiguous` and names them"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation get --help`.

```text
Fetch one artifact by id across the active constellation. When two sources claim the same id the answer is `ambiguous` and names them

Usage: forgeplan constellation get [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
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
