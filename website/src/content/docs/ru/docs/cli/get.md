---
title: forgeplan get
description: "Читает артефакт по slug или отображаемому ID. Текстовый вывод показывает метаданные, содержимое и связи; это не побайтовая копия исходного markdown-файла. JSON предоставляет поля для скриптов."
---

Читает артефакт по slug или отображаемому ID. Текстовый вывод показывает метаданные, содержимое и связи; это не побайтовая копия исходного markdown-файла. JSON предоставляет поля для скриптов.

## Использование

```text
forgeplan get [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  Artifact ID
```

## Опции

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

```bash
forgeplan get PRD-001
forgeplan get RFC-002 | less
# Structured evidence fields are in the body, not a frontmatter object:
forgeplan get EVID-012 --json | jq -r '.body'
```

## Контракт JSON

Поля верхнего уровня: `id`, `slug`, `kind`, `status`, `title`, `depth`, `author`, `parent_epic`, `valid_until`, `r_eff`, `created_at`, `updated_at`, `body`, `links`, `_next_action`. Вложенного объекта `frontmatter` нет. Найдите ID через `list` или `search`, затем изучите через `get`. Соответствующий инструмент MCP — `forgeplan_get`.

`links.outbound` — массив объектов `{target, relation}`, `links.inbound` — `{source, relation}`. Оба массива присутствуют и при отсутствии связей. Необязательные метаданные остаются `null`; `slug` извлекается из frontmatter сохранённого тела. Нужны инициализированный воркспейс и корректная конфигурация. Отсутствующий ID даёт обычную ошибку CLI с `Fix: forgeplan list`; `--json` не превращает её в успешную схему артефакта.

## См. также

- [`forgeplan list`](/ru/docs/cli/list/) - обнаружение ID
- [`forgeplan search`](/ru/docs/cli/search/) - поиск по запросу
- [`forgeplan validate`](/ru/docs/cli/validate/) - проверка качества
- [`forgeplan score`](/ru/docs/cli/score/) - метрики R_eff + F-G-R
- [Руководство по методологии](/ru/docs/methodology/overview/)
