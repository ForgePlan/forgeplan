---
title: forgeplan get
description: "Чтение полного содержимого артефакта по ID - получение markdown, удобное для ИИ"
---

Читает полный артефакт по ID или разрешимому slug. На мигрированном workspace выбранная сборка использует снимок канонического Markdown и сообщает, опубликован ли он. На legacy-workspace читается производный индекс. Канонический Markdown, индекс и снапшоты федерации - разные источники.

## Когда использовать

- ИИ-агенту нужен полный контекст PRD / RFC / ADR перед написанием кода
- Вы хотите просмотреть конкретное решение в терминале, не открывая файл
- Вам нужен вывод в формате JSON для передачи в другие инструменты Forgeplan

## Когда не использовать

- Нужен список → [`forgeplan list`](/ru/docs/cli/list/).
- Не знаете ID → [`forgeplan search`](/ru/docs/cli/search/).
- Нужен граф связей → [`forgeplan graph`](/ru/docs/cli/graph/) или [`forgeplan tree`](/ru/docs/cli/tree/).

## Использование

```text
forgeplan get [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  ID артефакта
```

## Опции

```text
      --json     Вывод в формате JSON для машинной обработки
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

Прочитать PRD полностью - стандартное получение для ИИ-агента:

```bash
forgeplan get PRD-001
```

Прочитать RFC и передать через пейджер:

```bash
forgeplan get RFC-002 | less
```

Получить полное поле `body`, не предполагая JSON-объект `frontmatter`:

```bash
forgeplan get EVID-012 --json | jq -r '.body'
```

## Интерпретация вывода

Текстовый вывод содержит метаданные, связи, тело и подсказки: это не поток только исходного файла. В мигрированном CLI-пути JSON `body` содержит весь захваченный Markdown, включая frontmatter. У full-profile `forgeplan_constellation_get` другой контракт: разобранное тело без frontmatter.

В `--json` поля `title`, `depth`, `author`, `tags`, временные метки и `r_eff` находятся на верхнем уровне вместе с `body`, `links.outbound`, `links.inbound` и `_next_action`. Вложенного `frontmatter` нет. Мигрированный путь добавляет `index`, `generation_id`, `snapshot_id`, `source_set_digest` и `persisted`. Значение `persisted: false` означает, что свежий снимок ещё не является опубликованным поколением; чтение само не публикует поколение.

Неразрешимый ID вызывает ошибку `Artifact '<ID>' not found` с подсказкой `forgeplan list`.

## Как это вписывается

`get` - это "детальный просмотр" по отношению к "индексному просмотру" `list`:

```
list (найти) → get (прочитать) → validate / reason / link / score (действовать)
```

MCP-инструмент чтения артефакта называется [`forgeplan_get`](/ru/docs/mcp/forgeplan_get/), а не `read_artifact`. Закреплённое чтение зависит от профиля и имеет отдельный интерфейс.

## См. также

- [`forgeplan list`](/ru/docs/cli/list/) - список ID
- [`forgeplan search`](/ru/docs/cli/search/) - поиск
- [`forgeplan validate`](/ru/docs/cli/validate/) - проверка качества
- [`forgeplan score`](/ru/docs/cli/score/) - R_eff и F-G-R
- [Методология](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan get --help`.

```text
Read a full artifact by ID

Usage: forgeplan get [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
