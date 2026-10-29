---
title: forgeplan blocked
description: "Показывает заблокированные артефакты и то, что их блокирует"
---

Показывает невыполненные зависимости структурных типов `based_on`, `refines`, `supersedes`, `contradicts`. Цели в `active`, `deprecated`, `superseded` считаются разрешёнными; `draft`, `stale` и отсутствующие цели могут блокировать.

## Когда использовать

- Начало спринта - выявление застрявшей работы до её назначения
- Ежедневный стендап - "что ждёт меня?"
- Отчёт для CI — проверяйте JSON-счётчики явно; наличие блокировщиков само по себе не даёт ненулевого кода выхода
- Аудит зависимостей во время заморозки релиза

## Не использовать, когда

- Нужен обход зависимостей → [`forgeplan order`](/ru/docs/cli/order/)
- Нужны слепые пятна качества → [`forgeplan blindspots`](/ru/docs/cli/blindspots/)
- Нужен полный граф → [`forgeplan graph`](/ru/docs/cli/graph/)

## Использование

```text
forgeplan blocked [OPTIONS] [ID]
```

## Аргументы

```text
  [ID]  Конкретный ID артефакта для проверки (необязательно)
```

## Опции

```text
      --json     Вывод в формате JSON для машинной обработки
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

Показать всё, что заблокировано в рабочем пространстве:

```bash
forgeplan blocked
```

Проверить один конкретный артефакт:

```bash
forgeplan blocked PRD-046
```

Машиночитаемый формат для дашборда:

```bash
forgeplan blocked --json | jq '.blocked[] | .id'
```

## Интерпретация вывода

Иллюстрация условий зависимостей, не дословный вывод текущей CLI:

```text
PRD-046  Docs v0.18.0 catch-up         [draft]
  blocked by:
    RFC-005  Graph expansion           [draft]     ← ещё не active
    ADR-008  Cloudflare Pages choice   [draft]     ← ещё не active

RFC-007  Embed cache policy            [draft]
  blocked by:
    EVID-019 Benchmark results         [missing]   ← ссылка, но не существует
```

| Индикатор | Значение |
|---|---|
| `[draft]` | Блокировщик существует, но ещё не `active` |
| `[stale]` | Блокировщик превысил дату `valid_until` |
| `[deprecated]` | Разрешена для расчёта блокировки; актуальность проверяйте отдельно |
| `[missing]` | Указанный ID не существует - неработающая ссылка |

Успешный отчёт возвращает 0 и при наличии блокировщиков. JSON всего workspace содержит `blocked`, `blocked_count`, `ready`, `ready_count`, `cycles`; запрос с ID — `id`, `blocked`, `blocked_by`.

## Как это вписывается

`blocked` - это _runtime_ представление графа зависимостей:

```
order    → обход графа и классификация ready/blocked
blocked  →  current obstacles (учитывает статус)
graph    →  полная картина
```

Используйте [`order`](/ru/docs/cli/order/) для изучения зависимостей и
[`health`](/ru/docs/cli/health/) для отдельной диагностики состояния проекта.

## См. также

- [`forgeplan order`](/ru/docs/cli/order/) - обход зависимостей
- [`forgeplan graph`](/ru/docs/cli/graph/) - полный граф
- [`forgeplan blindspots`](/ru/docs/cli/blindspots/) - решения без доказательств
- [`forgeplan health`](/ru/docs/cli/health/) - диагностика проекта

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan blocked --help`.

```text
Show blocked artifacts and their dependencies

Usage: forgeplan blocked [OPTIONS] [ID]

Arguments:
  [ID]  Specific artifact ID to check (optional)

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
