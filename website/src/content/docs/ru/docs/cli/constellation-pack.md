---
title: "forgeplan constellation pack"
description: "Stage 6 — build a budgeted ContextPack for a task: lexical ranking, evidence units with roles, omissions and a spend receipt"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation pack --help`.

```text
Stage 6 — build a budgeted ContextPack for a task: lexical ranking, evidence units with roles, omissions and a spend receipt

Usage: forgeplan constellation pack [OPTIONS] <TASK>

Arguments:
  <TASK>  What the agent must do — the ranking terms come from this text

Options:
      --budget <BUDGET>      Token budget for the whole pack (estimated tokens, chars/4) [default: 8000]
      --snapshot <SNAPSHOT>  Pack at a recorded snapshot instead of the live sources
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
