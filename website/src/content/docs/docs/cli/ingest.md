---
title: "forgeplan ingest"
description: "Ingest engine (PRD-066 / SPEC-004) — apply mapping YAML to plugin outputs, generate forge artifacts with file:line source refs"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan ingest --help`.

```text
Ingest engine (PRD-066 / SPEC-004) — apply mapping YAML to plugin outputs, generate forge artifacts with file:line source refs

Usage: forgeplan ingest [OPTIONS] --mapping <MAPPING> --source <SOURCE>

Options:
      --mapping <MAPPING>  Path to mapping YAML file
      --source <SOURCE>    Source path (file or dir to ingest)
      --dry-run            Print drafts without writing
      --update             Allow update of existing artifacts (vs skip on hash match)
      --json               Output as JSON
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
