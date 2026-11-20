---
title: "forgeplan constellation remove"
description: "Remove a source from a constellation"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation remove --help`.

```text
Remove a source from a constellation

Usage: forgeplan constellation remove [OPTIONS] <PATH>

Arguments:
  <PATH>  Path as it was added (canonicalized before matching)

Options:
      --from <FROM>  Constellation to remove from (default: the active one)
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
