---
title: "forgeplan constellation snapshot"
description: "Pin every source of the active constellation at its current Git HEAD and write the lockfile to `~/.forgeplan/snapshots/`. Sources that are not git checkouts are reported, not pinned"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation snapshot --help`.

```text
Pin every source of the active constellation at its current Git HEAD and write the lockfile to `~/.forgeplan/snapshots/`. Sources that are not git checkouts are reported, not pinned

Usage: forgeplan constellation snapshot

Options:
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
