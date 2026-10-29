---
title: forgeplan discover show
description: "Показать статус и накопленные результаты сессии обнаружения"
---

`forgeplan discover show <SESSION_ID>` выводит текущее состояние сессии обнаружения brownfield - стадию протокола, на которой она находится, сколько результатов представил агент, и категоризированное резюме того, что было зафиксировано на данный момент.

Используйте её во время выполнения для отслеживания прогресса и перед `discover complete` для проверки полноты охвата.

## Когда использовать

- **Во время выполнения** - агент добавлял результаты, и вы хотите увидеть текущую картину.
- **Перед `discover complete`** - убедитесь, что сессия действительно охватила интересующие вас области.
- **Во время отладки** - убедитесь, что результаты агента попадают в нужную сессию (полезно, когда несколько сессий выполняются параллельно).
- **При возобновлении** приостановленной сессии после перерыва.

## Когда НЕ использовать

- Для списка всех сессий - используйте [`discover list`](/docs/cli/discover-list/).
- Чтобы начать новую - используйте [`discover start`](/docs/cli/discover-start/).

## Использование

```text
forgeplan discover show <SESSION_ID> [OPTIONS]
```

## Аргументы

```text
  <SESSION_ID>   Идентификатор сессии обнаружения
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

```bash
forgeplan discover show disc-001
forgeplan discover show disc-001 | less

# Типичная проверка во время выполнения
forgeplan discover start "Project discovery"
# ...агент работает некоторое время...
forgeplan discover show disc-001
```

## Что вы увидите

Отчёт показывает:

- **Сессию** - ID, проект, статус, фазу, время начала/завершения.
- **Находки по фазам и уровням источников** - число записанных находок.
- **Последние 10 находок** - фаза, уровень, ссылка на артефакт и заголовок.
- **Всего находок** и подсказку. Подсказка не доказывает полноту обследования.

## Как это работает

`discover show` читает `.forgeplan/discover/<SESSION_ID>.json`, не LanceDB. Он показывает записанное состояние, а не независимо проверяет охват источников.

```
discover start → [агент добавляет через MCP] → discover show → discover complete
```

## См. также

- [`forgeplan discover`](/docs/cli/discover/) - родительская команда
- [`forgeplan discover start`](/docs/cli/discover-start/) - начать сессию
- [`forgeplan discover list`](/docs/cli/discover-list/) - все сессии
- [`forgeplan discover complete`](/docs/cli/discover-complete/) - завершение + рекомендации

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan discover show --help`.

```text
Show status of a discovery session

Usage: forgeplan discover show <SESSION_ID>

Arguments:
  <SESSION_ID>  Session ID (e.g. disc-20260407-abc)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
