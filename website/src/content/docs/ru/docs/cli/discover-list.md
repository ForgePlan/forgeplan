---
title: forgeplan discover list
description: "Перечислить все сессии обнаружения существующих систем в рабочем пространстве"
---

`forgeplan discover list` выводит сохранённые сессии с колонками Session ID, Project, Status, Phase и Findings. Команда читает файлы сессий, не открывая индекс артефактов.

## Когда использовать

- **Чтобы возобновить прерванную сессию** - найдите ID для передачи в `discover show` или `discover complete`.
- **Чтобы провести аудит истории обнаружения** - посмотрите, сколько проходов адаптации / обновления было у проекта.
- **Перед началом новой сессии** - избегайте дублирования активной.
- **В CI или скриптах** - перечислите сессии для автоматизации.

## Когда НЕ использовать

- Для результатов одной сессии используйте [`discover show`](/ru/docs/cli/discover-show/).
- Для новой сессии используйте [`discover start`](/ru/docs/cli/discover-start/).

## Использование

```text
forgeplan discover list [OPTIONS]
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
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

Колонки таблицы:

- **Session ID** - копируйте фактический ID; `disc-002` выше приведён для примера.
- **Project** - имя проекта, возможно сокращённое.
- **Status** - сохранённый статус сессии.
- **Phase** - текущая фаза исследования, не сводка покрытия.
- **Findings** - число сохранённых находок.

## Как это работает

`discover list` является примитивом перечисления для подсистемы обнаружения. Всё остальное (`show`, `complete`) работает с конкретным ID сессии, который вы обычно выбираете из этого списка.

```
discover list → pick an ID → discover show / complete
```

## См. также

- [`forgeplan discover`](/ru/docs/cli/discover/) - родительская команда
- [`forgeplan discover start`](/ru/docs/cli/discover-start/) - создать сессию
- [`forgeplan discover show`](/ru/docs/cli/discover-show/) - просмотреть сессию
- [`forgeplan discover complete`](/ru/docs/cli/discover-complete/) - завершить сессию

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan discover list --help`.

```text
List all discovery sessions in the workspace

Usage: forgeplan discover list

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
