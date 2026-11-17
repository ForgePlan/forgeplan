---
title: forgeplan fpf
description: "База знаний First Principles Framework - приём, поиск и применение правил FPF к артефактам"
---

`forgeplan fpf` объединяет поиск по базе знаний **First Principles Framework (FPF)** и проверку правил для артефактов. Корпус загружается из отдельно установленных файлов разделов; его размер зависит от источника, а не задан в бинарнике.

База знаний добавляет необязательный контекст в `forgeplan reason --fpf`. Механизм правил и расчёт R_eff работают с данными артефактов и конфигурацией: загруженный корпус и LLM им не нужны.

## Когда использовать

- **Один раз для каждого рабочего пространства** - запустите `fpf ingest` после `forgeplan init`, чтобы база знаний была доступна локально.
- **Во время рассуждений** - `fpf search "trust calculus"`, чтобы получить контекст из первых принципов при формировании PRD или ADR.
- **Во время планирования спринта** - `fpf dashboard`, чтобы увидеть ограниченные контексты, оценки качества и рекомендации по explore-vs-exploit.
- **Во время валидации** - `fpf check PRD-XXX`, чтобы увидеть, какие правила FPF срабатывают для артефакта и какое действие они предлагают.
- **Для онбординга** - `fpf list` + `fpf section B.3`, чтобы прочитать спецификацию непосредственно из CLI.

## Когда НЕ использовать

- Для общих операций CRUD с артефактами - используйте `forgeplan new`, `validate`, `review`, `activate` вместо этого.
- Для общего состояния проекта - используйте `forgeplan health`, а не `fpf dashboard` (эти две команды дополняют друг друга, а не взаимозаменяемы).
- Для рассуждений ADI используйте `forgeplan reason <ID> --fpf`. Ему нужен настроенный LLM; самому `fpf search` он не требуется.

## Использование

```text
forgeplan fpf <COMMAND>
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Подкоманды

```text
  dashboard  Show FPF dashboard — bounded contexts, quality scores, explore-exploit actions
  ingest     Ingest FPF spec into knowledge base
  search     Search FPF knowledge base
  section    Show a specific FPF section
  list       List all FPF sections
  status     Show FPF knowledge base status — source, ingested count, staleness
  rules      List active FPF rules grouped by action (EXPLORE/INVESTIGATE/EXPLOIT)
  check      Check which FPF rules match a given artifact
  help       Print this message or the help of the given subcommand(s)
```

## Примеры

```bash
# One-time setup after forgeplan init
forgeplan fpf ingest

# Explore the KB
forgeplan fpf status
forgeplan fpf list
forgeplan fpf section B.3

# Pull first-principles context into your reasoning
forgeplan fpf search "trust calculus"
forgeplan fpf search "bounded context"

# Apply FPF rules to the project and specific artifacts
forgeplan fpf dashboard
forgeplan fpf rules
forgeplan fpf check PRD-019
```

## Как это вписывается

FPF находится на **уровне рассуждений** конвейера Forgeplan:

```
Shape → Validate → ADI (FPF KB) → Code → Evidence → Activate
```

- **Поиск знаний:** обычный `fpf search` использует взвешенное сопоставление подстрок в заголовках и телах разделов. `--semantic` выбирает поиск по косинусной близости векторов BGE-M3, а не слияние BM25 с векторами.
- **Правила артефактов:** `fpf rules` показывает правила конфигурации или стандартные правила. `fpf check` выводит совпадения и первое из них по возрастанию priority. Предложенное действие — рекомендация, а не доказательство готовности к активации.
- **Методология:** схема выше — рекомендуемый процесс, а не последовательность, которую автоматически выполняет `fpf`. Инструкции агенту и методологическая политика не равны реализованным ограничениям CLI.

База знаний — таблица `fpf_spec` в `.forgeplan/lance/` (производная, игнорируется Git). `fpf ingest` загружает внешние файлы разделов и заменяет содержимое таблицы, а не импортирует артефакты проекта. `rules`, `check` и `dashboard` работают без загрузки корпуса.

## См. также

- [`forgeplan fpf dashboard`](/ru/docs/cli/fpf-dashboard/) - ограниченные контексты + обзор explore/exploit
- [`forgeplan fpf ingest`](/ru/docs/cli/fpf-ingest/) - однократная загрузка базы знаний
- [`forgeplan fpf search`](/ru/docs/cli/fpf-search/) - семантический поиск по разделам FPF
- [`forgeplan fpf section`](/ru/docs/cli/fpf-section/) - чтение конкретного раздела
- [`forgeplan fpf list`](/ru/docs/cli/fpf-list/) - все разделы
- [`forgeplan fpf status`](/ru/docs/cli/fpf-status/) - состояние базы знаний и просроченность
- [`forgeplan fpf rules`](/ru/docs/cli/fpf-rules/) - активные правила по действиям
- [`forgeplan fpf check`](/ru/docs/cli/fpf-check/) - правила, соответствующие артефакту
- [`forgeplan reason`](/ru/docs/cli/reason/) - цикл ADI, управляемый FPF
- [Руководство по методологии](/ru/docs/methodology/overview/) - где FPF вписывается в полный рабочий процесс
