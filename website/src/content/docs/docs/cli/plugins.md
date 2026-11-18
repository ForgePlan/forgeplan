---
title: "forgeplan plugins"
description: "Plugin detection + recommendations (PRD-067) — list installed plugins, doctor health-check, info on specific plugin"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan plugins --help`.

```text
Plugin detection + recommendations (PRD-067) — list installed plugins, doctor health-check, info on specific plugin

Usage: forgeplan plugins <COMMAND>

Commands:
  list    List installed plugins (Claude / agent-skills / Cursor / Forgeplan)
  doctor  Health check across known plugins — reports missing, outdated, OK
  info    Show details for a specific plugin (path, version, description)
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

- [`forgeplan plugins list`](/docs/cli/plugins-list/)
- [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/)
- [`forgeplan plugins info`](/docs/cli/plugins-info/)
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
