---
title: "forgeplan constellation add"
description: "Add a `.forgeplan` directory as a source. Validates the path at registration time — a symlink or a non-workspace fails here, not at every later read"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation add --help`.

```text
Add a `.forgeplan` directory as a source. Validates the path at registration time — a symlink or a non-workspace fails here, not at every later read

Usage: forgeplan constellation add [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to the source's `.forgeplan` directory

Options:
      --to <TO>   Constellation to add to (default: the active one)
      --optional  Mark the source optional: a strict query tolerates its outage as a named `failures` entry instead of refusing to answer
  -h, --help      Print help
  -V, --version   Print version
```
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
