---
title: forgeplan order
description: "Структурный граф зависимостей, циклы и классификация ready/blocked"
---

Изучает связанные артефакты по структурным отношениям `based_on`, `refines`, `supersedes`, `contradicts`. Информационные `informs` не блокируют. В результат входят лишь узлы этих структурных рёбер, не все артефакты.

## Когда использовать

- Планирование спринта — изучение зависимостей и готовности перед выбором работы
-   Аудит зависимостей - выявление отсутствующих границ, которые должны существовать
- Отчёт для CI — проверяйте `cycles` явно; сам цикл не даёт ненулевого кода выхода

## Когда не использовать

- Только блокировки → [`forgeplan blocked`](/ru/docs/cli/blocked/)
- Иерархия → [`forgeplan tree`](/ru/docs/cli/tree/)
- Все связи → [`forgeplan graph`](/ru/docs/cli/graph/)

## Использование

```text
forgeplan order [OPTIONS]
```

## Опции

```text
      --json     Вывод в формате JSON для машинной обработки
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

Вывести полный топологический порядок:

```bash
forgeplan order
```

Передать в шаблон документа спринта:

```bash
forgeplan order --json | jq -r '.order[] | "- [ ] \(.)"'
```

## Интерпретация вывода

В текстовом отчёте `v` обозначает активный, `x` — заблокированный, `o` — другой
готовый артефакт. Строки содержат ID и состояние/готовность, не столбец заголовка.
JSON содержит `order` (массив ID), `ready`, `blocked`, `cycles`, `_next_action`.

Не считайте порядок доказанным планом «сначала зависимости»: текущая реализация
обходит сохранённую смежность source→target, хотя source объявляет зависимость
от target. Перед назначением работ изучите сами рёбра и `ready`/`blocked`.

Циклы печатаются как предупреждения и входят в JSON; успешный отчёт всё равно возвращает 0. Исправляйте структурную связь, а не неподдержанный тип `blocks`.

## Как это вписывается

`order` — представление для изучения зависимостей. Используйте вместе с `blocked`:

```
order    → обход графа, циклы и готовность
blocked  → что застряло прямо сейчас
```

Зависимости в `active`, `deprecated`, `superseded` считаются разрешёнными. Отсутствие блокировок не доказывает здоровье всего workspace; `done` не является основным сохранённым статусом жизненного цикла.

## См. также

- [`forgeplan blocked`](/ru/docs/cli/blocked/) - невыполненные зависимости
- [`forgeplan tree`](/ru/docs/cli/tree/) - иерархия
- [`forgeplan graph`](/ru/docs/cli/graph/) - полный граф связей

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan order --help`.

```text
Show artifacts in topological order (dependency order)

Usage: forgeplan order [OPTIONS]

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
