---
title: "forgeplan playbook"
description: "Playbook runtime (PRD-065 / SPEC-003) — declarative YAML orchestration with delegations to external plugins"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan playbook --help`.

```text
Playbook runtime (PRD-065 / SPEC-003) — declarative YAML orchestration with delegations to external plugins

Usage: forgeplan playbook <COMMAND>

Commands:
  list      List available playbooks (built-in + installed packs) with applicable recommendations based on project signals
  show      Show playbook structure (steps, delegations, requires)
  validate  Validate playbook YAML against SPEC-003 schema
  run       Run a playbook — sequential step execution with delegations
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

- [`forgeplan playbook list`](/docs/cli/playbook-list/)
- [`forgeplan playbook show`](/docs/cli/playbook-show/)
- [`forgeplan playbook validate`](/docs/cli/playbook-validate/)
- [`forgeplan playbook run`](/docs/cli/playbook-run/)
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
