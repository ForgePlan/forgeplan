---
title: "forgeplan ingest"
description: "Ingest engine (PRD-066 / SPEC-004) — apply mapping YAML to plugin outputs, generate forge artifacts with file:line source refs"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan ingest --help`.

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

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
