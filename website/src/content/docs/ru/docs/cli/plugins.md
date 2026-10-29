---
title: "forgeplan plugins"
description: "Plugin detection + recommendations (PRD-067) — list installed plugins, doctor health-check, info on specific plugin"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan plugins --help`.

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

- [`forgeplan plugins list`](/ru/docs/cli/plugins-list/)
- [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/)
- [`forgeplan plugins info`](/ru/docs/cli/plugins-info/)
<!-- forgeplan-runtime:end -->

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
