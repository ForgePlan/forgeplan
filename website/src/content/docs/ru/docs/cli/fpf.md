---
title: forgeplan fpf
description: "База знаний First Principles Framework — приём, поиск и применение правил FPF к артефактам"
---

`forgeplan fpf` является родительской командой для **Базы знаний First Principles Framework (FPF)** — корпуса рассуждений из 204 разделов, который обеспечивает цикл ADI Forgeplan, Trust Calculus и механизм правил explore/investigate/exploit.

FPF является теоретической основой для `forgeplan reason`, оценки R_eff и проверок целостности методологии. Подкоманды `fpf` позволяют загружать спецификацию, выполнять семантический поиск по ней, проверять активные правила и проверять, как эти правила применяются к конкретным артефактам.

## Когда использовать

- **Один раз для каждого рабочего пространства** — запустите `fpf ingest` после `forgeplan init`, чтобы база знаний была доступна локально.
- **Во время рассуждений** — `fpf search "trust calculus"`, чтобы получить контекст из первых принципов при формировании PRD или ADR.
- **Во время планирования спринта** — `fpf dashboard`, чтобы увидеть ограниченные контексты, оценки качества и рекомендации по explore-vs-exploit.
- **Во время валидации** — `fpf check PRD-XXX`, чтобы увидеть, какие правила FPF срабатывают для артефакта и какое действие они предлагают.
- **Для онбординга** — `fpf list` + `fpf section B.3`, чтобы прочитать спецификацию непосредственно из CLI.

## Когда НЕ использовать

- Для общих операций CRUD с артефактами — используйте `forgeplan new`, `validate`, `review`, `activate` вместо этого.
- Для общего состояния проекта — используйте `forgeplan health`, а не `fpf dashboard` (эти две команды дополняют друг друга, а не взаимозаменяемы).
- Для запусков рассуждений ADI — используйте `forgeplan reason --fpf`, который внутренне обращается к базе знаний; прямой `fpf search` предназначен для людей, просматривающих необработанный контент.

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
  dashboard  Показать панель FPF — ограниченные контексты, оценки качества, действия explore-exploit
  ingest     Принять спецификацию FPF в базу знаний
  search     Поиск по базе знаний FPF
  section    Показать конкретный раздел FPF
  list       Перечислить все разделы FPF
  status     Показать статус базы знаний FPF — источник, количество принятых, просроченность
  rules      Перечислить активные правила FPF, сгруппированные по действиям (EXPLORE/INVESTIGATE/EXPLOIT)
  check      Проверить, какие правила FPF соответствуют данному артефакту
  help       Вывести это сообщение или справку по указанным подкомандам
```

## Примеры

```bash
# Однократная настройка после forgeplan init
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

- **PRD-041** встраивает правила FPF в этапы роутинга/валидации.
- **PRD-042** добавляет векторный поиск BGE-M3 по 204 разделам (тот же конвейер, что и поиск артефактов — BM25 + семантическая фузия).
- **PRD-043** обеспечивает целостность методологии: артефакт, нарушающий правила ограниченного контекста или Trust Calculus, помечается перед активацией.

База знаний хранится в LanceDB по пути `.forgeplan/lance/` (производная, игнорируется Git). Исходные разделы находятся в репозитории Forgeplan и встраиваются при приёме.

## См. также

- [`forgeplan fpf dashboard`](/docs/cli/fpf-dashboard/) — ограниченные контексты + обзор explore/exploit
- [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/) — однократная загрузка базы знаний
- [`forgeplan fpf search`](/docs/cli/fpf-search/) — семантический поиск по разделам FPF
- [`forgeplan fpf section`](/docs/cli/fpf-section/) — чтение конкретного раздела
- [`forgeplan fpf list`](/docs/cli/fpf-list/) — все разделы
- [`forgeplan fpf status`](/docs/cli/fpf-status/) — состояние базы знаний и просроченность
- [`forgeplan fpf rules`](/docs/cli/fpf-rules/) — активные правила по действиям
- [`forgeplan fpf check`](/docs/cli/fpf-check/) — правила, соответствующие артефакту
- [`forgeplan reason`](/docs/cli/reason/) — цикл ADI, управляемый FPF
- [Methodology guide](/docs/methodology/overview/) — где FPF вписывается в полный рабочий процесс
