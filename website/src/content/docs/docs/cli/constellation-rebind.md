---
title: "forgeplan constellation rebind"
description: "Rebind a source after its checkout moved. Source identity is path-derived, so a moved checkout is a new source: this rewrites the registry path and, by default, the index's path-bound marker so `forgeplan index` answers at the new location. References qualified by the old `src:` id stop resolving — that is the contract"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation rebind --help`.

```text
Rebind a source after its checkout moved. Source identity is path-derived, so a moved checkout is a new source: this rewrites the registry path and, by default, the index's path-bound marker so `forgeplan index` answers at the new location. References qualified by the old `src:` id stop resolving — that is the contract

Usage: forgeplan constellation rebind [OPTIONS] <SOURCE> <NEW_PATH>

Arguments:
  <SOURCE>    Stored path, path fragment, or `src:` id (or fragment) of the source
  <NEW_PATH>  The `.forgeplan` directory's new location

Options:
      --of <OF>            Constellation to rebind in (default: the active one)
      --skip-index-marker  Do not rewrite the index's PROTOCOL marker at the new path
      --force              Rebind even when the marker at the new path names a checkout other than the moved one
  -h, --help               Print help
  -V, --version            Print version
```
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
