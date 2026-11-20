---
title: "forgeplan constellation add"
description: "Добавить источник в созвездие. Корень .forgeplan проверяется при регистрации и чтении текущих данных; символьные ссылки отклоняются."
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation add --help`.

```text
Add a `.forgeplan` directory as a source. Rejects symlinks and non-workspaces at registration. Live reads recheck the registered source boundary

Usage: forgeplan constellation add [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to the source's `.forgeplan` directory

Options:
      --to <TO>   Constellation to add to (default: the active one)
      --optional  Mark the source optional: a strict query tolerates its outage as a named `failures` entry instead of refusing to answer
  -h, --help      Print help
  -V, --version   Print version
```
<!-- forgeplan-runtime:end -->

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
