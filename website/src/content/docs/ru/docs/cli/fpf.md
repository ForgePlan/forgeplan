---
title: forgeplan fpf
description: "База знаний First Principles Framework - приём, поиск и применение правил FPF к артефактам"
---

`forgeplan fpf` объединяет загрузку **базы знаний First Principles Framework (FPF)**, поиск, чтение разделов, дашборд и просмотр исполняемых правил. Число разделов определяется загруженным источником, а не гарантированным корпусом из 204 разделов.

FPF лежит в основе подхода к рассуждениям, но численные оценки и правила ForgePlan - локальная адаптация, не буквальная реализация всего upstream FPF. База знаний и исполняемые правила - разные интерфейсы.

## Когда использовать

- **После инициализации workspace** - установите источник разделов FPF или укажите каталог через `fpf ingest --path`.
- **Во время рассуждений** - `fpf search "trust calculus"`, чтобы получить контекст из первых принципов при формировании PRD или ADR.
- **Во время планирования спринта** - `fpf dashboard`, чтобы увидеть ограниченные контексты, оценки качества и рекомендации по explore-vs-exploit.
- **Во время валидации** - `fpf check PRD-XXX`, чтобы увидеть, какие правила FPF срабатывают для артефакта и какое действие они предлагают.
- **Для онбординга** - `fpf list` + `fpf section B.3`, чтобы прочитать спецификацию непосредственно из CLI.

## Когда НЕ использовать

- Для общих операций CRUD с артефактами - используйте `forgeplan new`, `validate`, `review`, `activate` вместо этого.
- Для общего состояния проекта - используйте `forgeplan health`, а не `fpf dashboard` (эти две команды дополняют друг друга, а не взаимозаменяемы).
- Для ADI используйте `forgeplan reason --fpf`; обогащение условно. В выбранном пути для мигрированного workspace legacy-контекст KB пропускается.

## Использование

```text
forgeplan fpf <COMMAND>
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Подкоманды

```text
  dashboard  Показать панель FPF - ограниченные контексты, оценки качества, действия explore-exploit
  ingest     Принять спецификацию FPF в базу знаний
  search     Поиск по базе знаний FPF
  section    Показать конкретный раздел FPF
  list       Перечислить все разделы FPF
  status     Показать статус базы знаний FPF - источник, количество принятых, просроченность
  rules      Перечислить активные правила FPF, сгруппированные по действиям (EXPLORE/INVESTIGATE/EXPLOIT)
  check      Проверить, какие правила FPF соответствуют данному артефакту
  help       Вывести это сообщение или справку по указанным подкомандам
```

## Примеры

```bash
# Загрузить установленные разделы FPF; иначе передайте их каталог через --path
forgeplan fpf ingest

# Исследование базы знаний
forgeplan fpf status
forgeplan fpf list
forgeplan fpf section B.3

# Получение контекста из первых принципов для ваших рассуждений
forgeplan fpf search "trust calculus"
forgeplan fpf search "bounded context"

# Применение правил FPF к проекту и конкретным артефактам
forgeplan fpf dashboard
forgeplan fpf rules
forgeplan fpf check PRD-019
```

## Как это вписывается

FPF находится на **уровне рассуждений** конвейера Forgeplan:

```
Shape → Validate → ADI (FPF KB) → Code → Evidence → Activate
```

- **PRD-041** ввёл интерфейс просмотра правил.
- **PRD-042** добавил необязательный векторный поиск BGE-M3 по загруженным разделам; это не BM25-поиск артефактов.
- **PRD-043** относится к целостности методологии; список разделов не доказывает прохождение гейтов.

Ingest читает установленные разделы FPF или явный `--path` и сохраняет производную таблицу `fpf_spec` в `.forgeplan/lance/`. Сборка semantic-search пытается создать эмбеддинги при загрузке; keyword-сборка сохраняет текст. `fpf search`/`section`/`list` читают вспомогательную таблицу без воссоздания таблиц артефактов.

## См. также

- [`forgeplan fpf dashboard`](/ru/docs/cli/fpf-dashboard/) - обзор проекта
- [`forgeplan fpf ingest`](/ru/docs/cli/fpf-ingest/) - загрузка базы знаний
- [`forgeplan fpf search`](/ru/docs/cli/fpf-search/) - поиск по разделам
- [`forgeplan fpf section`](/ru/docs/cli/fpf-section/) - чтение раздела
- [`forgeplan fpf list`](/ru/docs/cli/fpf-list/) - список разделов
- [`forgeplan fpf status`](/ru/docs/cli/fpf-status/) - состояние KB
- [`forgeplan fpf rules`](/ru/docs/cli/fpf-rules/) - активные правила
- [`forgeplan fpf check`](/ru/docs/cli/fpf-check/) - проверка артефакта
- [`forgeplan reason`](/ru/docs/cli/reason/) - ADI
- [Методология](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf --help`.

```text
FPF Knowledge Base — dashboard, ingest, search, sections

Usage: forgeplan fpf <COMMAND>

Commands:
  dashboard  Show FPF dashboard — bounded contexts, quality scores, explore-exploit actions
  ingest     Ingest FPF spec into knowledge base
  search     Search FPF knowledge base
  section    Show a specific FPF section
  list       List all FPF sections
  status     Show FPF knowledge base status — source, ingested count, staleness
  rules      List active FPF rules grouped by action (EXPLORE/INVESTIGATE/EXPLOIT)
  check      Check which FPF rules match a given artifact
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

- [`forgeplan fpf dashboard`](/ru/docs/cli/fpf-dashboard/)
- [`forgeplan fpf ingest`](/ru/docs/cli/fpf-ingest/)
- [`forgeplan fpf search`](/ru/docs/cli/fpf-search/)
- [`forgeplan fpf section`](/ru/docs/cli/fpf-section/)
- [`forgeplan fpf list`](/ru/docs/cli/fpf-list/)
- [`forgeplan fpf status`](/ru/docs/cli/fpf-status/)
- [`forgeplan fpf rules`](/ru/docs/cli/fpf-rules/)
- [`forgeplan fpf check`](/ru/docs/cli/fpf-check/)
<!-- forgeplan-runtime:end -->
