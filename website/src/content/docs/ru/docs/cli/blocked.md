---
title: forgeplan blocked
description: "Показывает неразрешённые структурные зависимости одного артефакта или всего графа. К структурным относятся based_on, refines, supersedes; informs не задаёт порядок работ. Зависимости со статусом active, deprecated, superseded считаются разрешёнными."
---

Показывает неразрешённые структурные зависимости одного артефакта или всего графа. К структурным относятся `based_on`, `refines`, `supersedes`; `informs` не задаёт порядок работ. Зависимости со статусом active, deprecated, superseded считаются разрешёнными.

## Использование

```text
forgeplan blocked [OPTIONS] [ID]
```

## Аргументы

```text
  [ID]  Specific artifact ID to check (optional)
```

## Опции

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

```bash
forgeplan blocked
forgeplan blocked PRD-046
forgeplan blocked --json | jq '.blocked[] | .id'
forgeplan blocked PRD-046 --json | jq '.blocked, .blocked_by'
```

## Вывод и автоматизация

JSON пространства содержит записи `blocked` с `id` и `blocked_by`, а также `ready`, `ready_count`, `blocked_count`, `cycles`, `_next_action`, `hints`. Для одного артефакта поля — `id`, `blocked`, `blocked_by`, `_next_action`, `hints`. Наличие блокировок не меняет код завершения на 1. Для проверки зависимостей явно разбирайте JSON. Для порогов состояния используйте текстовый `health --ci`: в 0.38.0 сочетание `health --json --ci` возвращает результат до применения порогов CI (#545).

## Планирование

Используйте `order` для последовательности и `graph` для всех типов связей. Если зависимость отсутствует или образует цикл, проверьте связь, а не активируйте блокирующий артефакт вслепую.

## См. также

- [`forgeplan order`](/ru/docs/cli/order/) - топологическая последовательность
- [`forgeplan graph`](/ru/docs/cli/graph/) - полный граф зависимостей
- [`forgeplan blindspots`](/ru/docs/cli/blindspots/) - решения без доказательств
- [`forgeplan health`](/ru/docs/cli/health/) - сводка по проекту