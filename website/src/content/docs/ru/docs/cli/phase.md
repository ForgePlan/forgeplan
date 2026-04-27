---
title: forgeplan phase
description: "Read the advisory phase state for an artifact — current phase, workflow type, full transition history. Phase tracking is advisory and never blocks other tools."
---

`forgeplan phase` показывает, на какой стадии методологии находится артефакт — Shape, Validate, Adi, Code, Test, Audit, Evidence или Done — и печатает полную историю того, как он туда попал (timestamp-ы, причины). Данные хранятся в `.forgeplan/state/<id>.yaml` и только дописываются: каждый переход сохраняется навсегда.

Фаза **рекомендательная** — это подсказка для людей и агентов, а не блокировка. Никакая другая команда Forgeplan не откажет в работе из-за «неправильной» фазы. Если у артефакта ещё нет файла состояния (создан до появления отслеживания фаз или с `phase.enabled: false` в конфиге), команда напечатает `current_phase: unknown` с пустой историей — это норма, не ошибка.

Это CLI-вариант [`forgeplan_phase`](/ru/docs/mcp/forgeplan_phase/) на MCP-стороне.

## Когда использовать

- Возвращаетесь к артефакту, который уже в работе — «на чём я остановился?».
- Перед запуском дорогого инструмента — убедиться, что артефакт прошёл нужную фазу (например, не запускать `forgeplan score`, пока артефакт ещё в `shape`).
- Ревью старого артефакта — прочитать историю переходов и понять путь, который он прошёл.
- Аудит или отладка — у каждого перехода есть timestamp и опциональная причина, можно реконструировать решения.

## Когда НЕ использовать

- Как жёсткий гейт для блокировки работы — фаза рекомендательная. Для структурной блокировки используйте [`forgeplan validate`](/ru/docs/cli/validate/).
- Для lifecycle-переходов (`draft` → `active` → `superseded`) — это отдельная state-машина; см. [`forgeplan activate`](/ru/docs/cli/activate/), [`forgeplan supersede`](/ru/docs/cli/supersede/), [`forgeplan deprecate`](/ru/docs/cli/deprecate/).
- На Note или тривиальных тактических фиксах — отслеживание фаз не ожидается для одноразовой работы.

## Использование

```text
forgeplan phase [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  Artifact ID whose phase state to read
```

## Опции

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

### Пример 1: Инспекция in-flight PRD

```bash
forgeplan phase PRD-057
```

Печатает текущую фазу, тип workflow и последние три перехода в text-режиме (полная
история — в `--json`). Типичный вывод:

```text
PRD-057 — current_phase: code (greenfield)
  advanced_at: 2026-04-26T09:30:00Z
  history (last 3):
    shape    2026-04-25T14:00:00Z
    validate 2026-04-25T15:20:00Z
    code     2026-04-26T09:30:00Z  reason: FRs implemented
```

### Пример 2: Полная история как JSON

```bash
forgeplan phase PRD-057 --json | jq '.history'
```

Возвращает каждый переход, который артефакт когда-либо записал. Полезно для аудитов или для построения timeline-представления в смежном инструменте.

### Пример 3: Артефакт без файла состояния

```bash
forgeplan phase PRD-001
```

Если у артефакта нет файла состояния (создан до появления отслеживания фаз или с выключенным `phase.enabled`), вывод покажет `current_phase: unknown` и пустую историю. Это намеренно, не ошибка. Начать отслеживание можно через [`forgeplan phase-advance`](/ru/docs/cli/phase-advance/).

## Место в рабочем процессе

Отслеживание фаз — слой наблюдаемости над методологическим пайплайном (Shape → Validate → Code → Evidence → Activate). Чтение через `phase`, запись через [`forgeplan phase-advance`](/ru/docs/cli/phase-advance/). Сейчас разрешены переходы вне порядка (например, сразу в Done для опечатки); строгая проверка порядка планируется в более позднем PRD под EPIC-005.

## См. также

- [`forgeplan_phase`](/ru/docs/mcp/forgeplan_phase/) — MCP-эквивалент
- [`forgeplan phase-advance`](/ru/docs/cli/phase-advance/) — записать следующий переход
- [`forgeplan validate`](/ru/docs/cli/validate/) — гейт вокруг фазы `validate`
- [`forgeplan activate`](/ru/docs/cli/activate/) — терминальное `done` методологии
- [Руководство по методологии](/ru/docs/methodology/overview/) — Shape → Validate → Code → Evidence → Activate
