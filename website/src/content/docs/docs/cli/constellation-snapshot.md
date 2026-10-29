---
title: "forgeplan constellation snapshot"
description: "Pin every source of the active constellation at its current Git HEAD and write the lockfile to `~/.forgeplan/snapshots/`. Sources that are not git checkouts are reported, not pinned"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation snapshot --help`.

```text
Pin every source of the active constellation at its current Git HEAD and write the lockfile to `~/.forgeplan/snapshots/`. Sources that are not git checkouts are reported, not pinned

Usage: forgeplan constellation snapshot

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
