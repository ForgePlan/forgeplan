---
title: "forgeplan playbook"
description: "Playbook runtime (PRD-065 / SPEC-003) — declarative YAML orchestration with delegations to external plugins"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan playbook --help`.

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

- [`forgeplan playbook list`](/ru/docs/cli/playbook-list/)
- [`forgeplan playbook show`](/ru/docs/cli/playbook-show/)
- [`forgeplan playbook validate`](/ru/docs/cli/playbook-validate/)
- [`forgeplan playbook run`](/ru/docs/cli/playbook-run/)
<!-- forgeplan-runtime:end -->

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
