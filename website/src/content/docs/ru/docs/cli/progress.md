---
title: forgeplan progress
description: "Трекер прогресса для артефактов на основе чекбоксов"
---

Считает отмеченные и неотмеченные задачи в телах артефактов. Показывает отметки автора, а не доказательство реализации или приёмки требований.

## Когда использовать

-   Ежедневные стендапы - быстрый процент выполнения для каждого PRD
-   Подготовка к релизу - какие PRD ещё не выполнены на 100%
-   Отчётность - передача JSON в дашборды

## Когда не использовать

- Для оценки объявленных доказательств есть [`forgeplan score`](/ru/docs/cli/score/).
- Для сводки задолженности есть [`forgeplan health`](/ru/docs/cli/health/).

## Использование

```text
forgeplan progress [OPTIONS] [ID]
```

## Аргументы

```text
  [ID]  ID артефакта (показывает все, если пропущено)
```

## Опции

```text
      --json     Вывод в формате JSON для машинной обработки
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

Все артефакты, таблица по умолчанию:

```bash
forgeplan progress
```

Только один PRD:

```bash
forgeplan progress PRD-001
```

Машиночитаемый формат для дашборда:

```bash
forgeplan progress --json | jq '.progress[] | {id, percent}'
```

## Интерпретация вывода

Иллюстрация таблицы, не записанный runtime-вывод:

```
ID        KIND  PERCENT  BAR                  DONE / TOTAL
PRD-001   prd     75%    [██████████████░░░░]   6 / 8
PRD-046   prd     40%    [████████░░░░░░░░░░]   4 / 10
RFC-004   rfc    100%    [██████████████████]   5 / 5
```

| Столбец    | Значение                                             |
|------------|------------------------------------------------------|
| `PERCENT`  | `done / total` в виде целого процента                |
| `BAR`      | 24-позиционный индикатор из заполненных/пустых блоков |
| `DONE`     | Количество чекбоксов `- [x]`                         |
| `TOTAL`    | Количество чекбоксов `- [x]` + `- [ ]`               |

Артефакты без чекбоксов исключаются и из таблицы, и из JSON-массива `progress`.

## Как это вписывается

`progress` считает отмеченные задачи; [`score`](/ru/docs/cli/score/) оценивает объявленные доказательства, [`health`](/ru/docs/cli/health/) показывает задолженность. Ни один из них не доказывает завершение реализации. 100% чекбоксов и положительный R_eff сами по себе не являются приёмкой.

## См. также

- [`forgeplan score`](/ru/docs/cli/score/) - R_eff
- [`forgeplan health`](/ru/docs/cli/health/) - задолженность
- [`forgeplan list`](/ru/docs/cli/list/) - поиск ID

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan progress --help`.

```text
Show checkbox progress for artifacts

Usage: forgeplan progress [OPTIONS] [ID]

Arguments:
  [ID]  Artifact ID (shows all if omitted)

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
