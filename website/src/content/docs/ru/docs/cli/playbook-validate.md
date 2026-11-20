---
title: "forgeplan playbook validate"
description: "Validate playbook YAML against SPEC-003 schema"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan playbook validate --help`.

```text
Validate playbook YAML against SPEC-003 schema

Usage: forgeplan playbook validate [OPTIONS] <TARGET>

Arguments:
  <TARGET>  Playbook name (e.g. brownfield-code) or path to .yaml file

Options:
      --json     Output as JSON
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
