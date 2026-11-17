---
title: forgeplan list
description: "Выводит список артефактов с фильтрами - основная команда инвентаризации"
---

Выводит список артефактов в текущем рабочем пространстве, опционально отфильтрованных по типу, статусу,
или тегу. Это основная команда инвентаризации: она отвечает на вопрос «что есть в проекте?» перед планированием, роутингом или поиском. Для фильтра по дате `search --since` в 0.38.0 нужен семантический режим: CLI-поиск по словам пока игнорирует `--since`, `--status` и `--limit` (#544).

## Когда использовать

- В начале сессии, после `forgeplan health`, чтобы просканировать существующие артефакты
- Перед `forgeplan new` - чтобы избежать создания дубликатов
- При составлении объёма спринта ("все активные PRD с тегом `auth`")
- В скриптах CI для перечисления артефактов по статусу для отчётности

## Когда не использовать

- Вам нужно полное содержимое → используйте [`forgeplan get`](/ru/docs/cli/get/)
- Вам нужно нечёткое/семантическое сопоставление → используйте [`forgeplan search`](/ru/docs/cli/search/)
- Вам нужна иерархия (Epic → PRD → RFC) → используйте [`forgeplan tree`](/ru/docs/cli/tree/)
- Вам нужен порядок зависимостей → используйте [`forgeplan order`](/ru/docs/cli/order/)

## Использование

```text
forgeplan list [OPTIONS]
```

## Опции

```text
  -t, --type <TYPE>      Filter by kind (prd, epic, spec, rfc, adr, etc.)
  -s, --status <STATUS>  Filter by status (draft, active, etc.)
      --tag <TAG>        Filter by tag. Supports "key=value" or bare "key" (matches any value). Examples: --tag source=code, --tag legacy
      --json             Output as JSON for machine consumption
  -h, --help             Print help
  -V, --version          Print version
```

## Примеры

Вывести всё (вид по умолчанию в новом рабочем пространстве):

```bash
forgeplan list
```

Только активные PRD - текущий активный объём продукта:

```bash
forgeplan list --type prd --status active
```

Всё с тегом `security`, в формате JSON для скриптов:

```bash
forgeplan list --tag security --json | jq '.[] | .id'
```

## Интерпретация вывода

Вывод по умолчанию представляет собой таблицу из четырёх столбцов:

| Столбец  | Значение                                                  |
|----------|-----------------------------------------------------------|
| `ID`     | Стабильный ID артефакта (например, `PRD-001`, `RFC-002`, `EVID-012`) |
| `KIND`   | Тип артефакта - prd, rfc, adr, epic, note, problem, evidence |
| `STATUS` | Жизненный цикл: draft / active / superseded / deprecated / stale |
| `TITLE`  | Человекочитаемое название из frontmatter                     |

JSON — массив записей с полями `id`, `kind`, `status`, `title`. Для содержимого и метаданных используйте `get --json`.

## Как это работает

`list` открывает цикл исследования только для чтения:

```
list → get → tree → graph   (understand what exists and how it links)
```

Для планирования используйте в связке с `order` и `blocked`, чтобы увидеть состояние зависимостей. Для
сортировки качества используйте в паре с [`blindspots`](/ru/docs/cli/blindspots/), чтобы найти
артефакты без доказательств.

## Смотрите также

- [`forgeplan get`](/ru/docs/cli/get/) - прочитать полный markdown одного артефакта
- [`forgeplan tree`](/ru/docs/cli/tree/) - иерархический вид
- [`forgeplan search`](/ru/docs/cli/search/) - умный поиск по ключевым словам + семантический поиск
- [`forgeplan health`](/ru/docs/cli/health/) - дашборд на уровне проекта
- [Руководство по методологии](/ru/docs/methodology/overview/)