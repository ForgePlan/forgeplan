---
title: forgeplan phase-advance
description: "Manually advance (or set) the advisory phase marker for an artifact and record an immutable transition entry. Out-of-order jumps allowed."
---

`forgeplan phase-advance` переводит артефакт на следующую методологическую фазу (Shape, Validate, Adi, Code, Test, Audit, Evidence, Done) и записывает переход в `.forgeplan/state/<id>.yaml` с timestamp и опциональной причиной. История только дописывается — однажды записанное нельзя отредактировать или удалить, можно только добавить новую запись.

Это **рекомендательный** слой: команда не проверяет, имеет ли смысл прыжок (можно перескочить с Shape сразу в Done) — переходы вне порядка разрешены by design, это удобно для тривиальных фиксов или для исторического заполнения старых артефактов. Строгая проверка порядка планируется в более позднем PRD под EPIC-005.

Аналог [`forgeplan_phase_advance`](/ru/docs/mcp/forgeplan_phase_advance/) на MCP-стороне.

## Когда использовать

- Инструмент сработал, но отслеживание фаз было выключено — теперь хотите, чтобы артефакт отражал реальное положение дел, переведите вручную.
- Артефакт перешёл из `code` в `audit`, потому что только что закончилась волна PR-ревью — зафиксируйте это.
- Старый артефакт (созданный до отслеживания фаз) надо провести по фазам, чтобы он корректно отображался в текущих отчётах.
- Тривиальный фикс позволяет сразу прыгнуть в `done` — передайте `--reason`, чтобы прыжок был задокументирован.

## Когда НЕ использовать

- Как структурный гейт (что-то, что блокирует другие команды) — `phase-advance` только пишет маркер. Для реальной блокировки используйте [`forgeplan validate`](/ru/docs/cli/validate/).
- Чтобы переименовать фазу или переписать историю — записи неизменяемые. Добавьте новую запись с корректирующим `--reason`.
- Без `--reason`, когда прыжок неочевиден — через полгода во время аудита вы не вспомните, почему так сделали.

## Использование

```text
forgeplan phase-advance [OPTIONS] --to <TO> <ID>
```

## Аргументы

```text
  <ID>  Artifact ID to advance
```

## Опции

```text
      --to <TO>          Target phase: shape, validate, adi, code, test, audit, evidence, done [possible values: shape, validate, adi, code, test, audit, evidence, done]
      --reason <REASON>  Optional reason / justification (recorded in history)
      --json             Output as JSON for machine consumption
  -h, --help             Print help
  -V, --version          Print version
```

## Примеры

### Пример 1: Переход после прохождения тестов

```bash
forgeplan phase-advance PRD-057 --to test --reason "FR tests green"
```

Записывает переход с короткой причиной. Причина сохраняется навсегда — будущие аудиты смогут точно понять, почему артефакт сдвинулся.

### Пример 2: Прыжок вперёд для тривиального фикса

```bash
forgeplan phase-advance NOTE-019 --to done --reason "trivial typo fix"
```

Пропуск промежуточных фаз разрешён (рекомендательный слой). Всегда сопровождайте прыжок понятной `--reason`, чтобы аудитор, читающий историю позже, понял ваше решение.

### Пример 3: Историческое заполнение старого артефакта

```bash
forgeplan phase-advance PRD-001 --to shape
forgeplan phase-advance PRD-001 --to validate
forgeplan phase-advance PRD-001 --to code --reason "backfilled from git history"
```

Проводит артефакт, созданный до появления отслеживания фаз, через фазы — чтобы он корректно отображался в текущих отчётах. Причина в финальном переходе объясняет, откуда взялись данные.

## Место в рабочем процессе

Отслеживание фаз идёт рядом с методологическим пайплайном (Shape → Validate → Code → Evidence → Activate). Текущее состояние читается через [`forgeplan phase`](/ru/docs/cli/phase/), следующий переход записывается через `phase-advance`. Поле `--reason` — это аудит-след того, как артефакт двигался по пайплайну; относитесь к нему как к commit-сообщению.

## См. также

- [`forgeplan_phase_advance`](/ru/docs/mcp/forgeplan_phase_advance/) — MCP-эквивалент
- [`forgeplan phase`](/ru/docs/cli/phase/) — чтение текущего состояния и истории
- [`forgeplan activate`](/ru/docs/cli/activate/) — гейт активации методологии
- [Руководство по методологии](/ru/docs/methodology/overview/) — Shape → Validate → Code → Evidence → Activate
