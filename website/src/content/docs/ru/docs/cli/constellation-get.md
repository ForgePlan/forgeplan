---
title: "forgeplan constellation get"
description: "Fetch one artifact by id across the active constellation. When two sources claim the same id the answer is `ambiguous` and names them"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation get --help`.

```text
Fetch one artifact by id across the active constellation. When two sources claim the same id the answer is `ambiguous` and names them

Usage: forgeplan constellation get [OPTIONS] <ID>

Arguments:
  <ID>  

Options:
      --snapshot <SNAPSHOT>  Read at a recorded snapshot instead of the live sources
      --allow-partial        Tolerate dead sources: even a required source's outage becomes a `failures` entry on a partial answer instead of an error
      --json                 Output as JSON
  -h, --help                 Print help
  -V, --version              Print version
```
<!-- forgeplan-runtime:end -->

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
