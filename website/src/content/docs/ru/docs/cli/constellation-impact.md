---
title: "forgeplan constellation impact"
description: "Stage 7 — explainable impact: who depends on this artifact, by which relation path, and where the bounded walk stopped"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation impact --help`.

```text
Stage 7 — explainable impact: who depends on this artifact, by which relation path, and where the bounded walk stopped

Usage: forgeplan constellation impact [OPTIONS] <ID>

Arguments:
  <ID>  The changed artifact's canonical key or display id

Options:
      --depth <DEPTH>          Maximum structural hops (default 4) [default: 4]
      --max-nodes <MAX_NODES>  Maximum impacted nodes (default 256) [default: 256]
      --snapshot <SNAPSHOT>    Walk at a recorded snapshot instead of the live sources
      --allow-partial          Tolerate dead sources: even a required source's outage becomes a `failures` entry on a partial answer instead of an error
      --json                   Output as JSON
  -h, --help                   Print help
  -V, --version                Print version
```
<!-- forgeplan-runtime:end -->

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
