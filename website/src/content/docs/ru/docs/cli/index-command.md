---
title: "forgeplan index"
description: "Run one Constellation index maintenance pass (capture → reconcile → publish, verified), optionally draining the embedding queue"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan index --help`.

```text
Run one Constellation index maintenance pass (capture → reconcile → publish, verified), optionally draining the embedding queue

Usage: forgeplan index [OPTIONS]

Options:
      --embed-profile <EMBED_PROFILE>  Also drain the embedding queue for this profile (requires the semantic-search build feature)
      --deadline-secs <DEADLINE_SECS>  Deadline for the whole pass, in seconds [default: 300]
      --list                           List a read-only page of documents in the published generation
      --offset <OFFSET>                First document to show with --list [default: 0]
      --generations                    List all generations under lance/ with size and protection state (current / leased / keepable) — the generation index
      --gc <KEEP>                      Garbage-collect generations: keep the newest N deletable ones plus CURRENT and anything a live reader holds pinned
  -h, --help                           Print help
  -V, --version                        Print version
```
<!-- forgeplan-runtime:end -->

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
