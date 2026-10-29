---
title: "forgeplan playbook run"
description: "Run a playbook — sequential step execution with delegations"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan playbook run --help`.

```text
Run a playbook — sequential step execution with delegations

Usage: forgeplan playbook run [OPTIONS] <TARGET>

Arguments:
  <TARGET>  Playbook name or path

Options:
      --yes          Confirm execution (required — prevents accidental runs)
      --allow-shell  Allow `Delegation::Command` (shell-exec) steps to run. PROB-053 / PRD-074 §FR-1: default-deny gate for the CWE-78 surface. Implies `--yes`. Alternative: set `[playbook] allow_shell = true` in workspace `config.yaml`
      --dry-run      Print steps without executing
      --step <STEP>  Start from specific step (1-indexed)
      --json         Output as JSON
  -h, --help         Print help
  -V, --version      Print version
```
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
