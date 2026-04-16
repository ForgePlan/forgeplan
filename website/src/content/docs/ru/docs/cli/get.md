---
title: forgeplan get
description: "Чтение полного содержимого артефакта по ID — получение markdown, удобное для ИИ"
---

Прочитать полное содержимое артефакта в формате markdown по его ID. Так ИИ-агенты и люди извлекают отдельный документ-решение для контекста. Команда считывает данные из представления, проецируемого LanceDB, которое синхронизировано с исходным markdown-файлом в `.forgeplan/`.

## Когда использовать

- ИИ-агенту нужен полный контекст PRD / RFC / ADR перед написанием кода
- Вы хотите просмотреть конкретное решение в терминале, не открывая файл
- Вам нужен вывод в формате JSON для передачи в другие инструменты Forgeplan

## Когда не использовать

- Вам нужен список кандидатов → используйте [`forgeplan list`](/docs/cli/list/)
- Вы не знаете точный ID → используйте [`forgeplan search`](/docs/cli/search/)
- Вам нужны также связанные артефакты → используйте [`forgeplan graph`](/docs/cli/graph/) или [`forgeplan tree`](/docs/cli/tree/)

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

Прочитать PRD полностью — стандартное получение для ИИ-агента:

```bash
forgeplan get PRD-001
```

Прочитать RFC и передать через пейджер:

```bash
forgeplan get RFC-002 | less
```

Вывод JSON — тело плюс весь frontmatter, готовый для `jq`:

```bash
forgeplan get EVID-012 --json | jq '.frontmatter.verdict, .frontmatter.congruence_level'
```

## Интерпретация вывода

Вывод по умолчанию — это необработанный markdown-файл: YAML frontmatter, ограниченный `---`, за которым следуют разделы тела (Problem, Goals, FR и т.д.). Это тот же текст, который человек увидит, открыв файл в редакторе.

С опцией `--json` структура вывода следующая:

```json
{
  "id": "PRD-001",
  "kind": "prd",
  "status": "active",
  "frontmatter": { "title": "...", "tags": [...], "created": "...", ... },
  "body": "## Problem\n..."
}
```

Если ID не существует, команда завершает работу со статусом 1 и выводит `Error: artifact not found: <ID>`.

## Как это вписывается

`get` — это "детальный просмотр" по отношению к "индексному просмотру" `list`:

```
list (найти) → get (прочитать) → validate / reason / link / score (действовать)
```

Для ИИ-агентов, использующих MCP, `get` соответствует инструменту `read_artifact` в соотношении 1:1.

## См. также

- [`forgeplan list`](/docs/cli/list/) — обнаружение ID
- [`forgeplan search`](/docs/cli/search/) — поиск по запросу
- [`forgeplan validate`](/docs/cli/validate/) — проверка качества
- [`forgeplan score`](/docs/cli/score/) — метрики R_eff + F-G-R
- [Руководство по методологии](/docs/methodology/overview/)
