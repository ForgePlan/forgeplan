---
title: "forgeplan playbook run"
description: "Run a playbook — sequential step execution with delegations"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan playbook run --help`.

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

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
