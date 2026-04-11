---
title: forgeplan discover list
description: "Перечислить все сессии обнаружения существующих систем в рабочем пространстве"
---

`forgeplan discover list` выводит каждую сессию обнаружения, хранящуюся в рабочем пространстве — активные и завершённые — с ID, статусом, временем создания и кратким обзором покрытия. Это индексное представление всех сессий.

## Когда использовать

- **Чтобы возобновить прерванную сессию** — найдите ID для передачи в `discover show` или `discover complete`.
- **Чтобы провести аудит истории обнаружения** — посмотрите, сколько проходов адаптации / обновления было у проекта.
- **Перед началом новой сессии** — избегайте дублирования активной.
- **В CI или скриптах** — перечислите сессии для автоматизации.

## Когда НЕ использовать

- Чтобы просмотреть результаты одной сессии — используйте [`discover show`](/docs/cli/discover-show/).
- Чтобы начать новую сессию — используйте [`discover start`](/docs/cli/discover-start/).

## Использование

```text
forgeplan discover list [OPTIONS]
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

```bash
# Все сессии
forgeplan discover list

# Типичный сценарий восстановления "где я остановился"
forgeplan discover list
forgeplan discover show disc-002
forgeplan discover complete disc-002
```

## Что вы увидите

Таблица (или список) с одной строкой на сессию, обычно показывающая:

- **ID сессии** (`disc-NNN`)
- **Статус** — активная / завершённая
- **Создана** — временная метка ISO
- **Количество находок** — сколько вызовов `discover_finding` было сделано для этой сессии
- **Покрытие** — краткий обзор уровней (какие из code/git/tests/docs были затронуты)

## Как это работает

`discover list` является примитивом перечисления для подсистемы обнаружения. Всё остальное (`show`, `complete`) работает с конкретным ID сессии, который вы обычно выбираете из этого списка.

```
discover list → pick an ID → discover show / complete
```

## См. также

- [`forgeplan discover`](/docs/cli/discover/) — родительская команда
- [`forgeplan discover start`](/docs/cli/discover-start/) — создать новую сессию
- [`forgeplan discover show`](/docs/cli/discover-show/) — просмотреть конкретную сессию
- [`forgeplan discover complete`](/docs/cli/discover-complete/) — завершить сессию
