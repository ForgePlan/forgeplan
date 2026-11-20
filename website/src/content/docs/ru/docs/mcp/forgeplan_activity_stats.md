---
title: forgeplan_activity_stats
description: "Aggregate the activity log by tool name - counts, error rates, p50/p95 duration, total time."
---

Читает тот же журнал активности, что и [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/),
но возвращает по одной строке на каждое уникальное имя инструмента с роллапами:
число вызовов, ошибок, p50/p95 длительности и сумму записанных длительностей.
При параллельных вызовах сумма может превышать время сессии.
Эти поля не измеряют токены или денежные затраты.

**Категория**: Observability & Audit

## Когда агент вызывает

- Старт debug-сессии: «какой инструмент горячий за последние 24 часа?».
- Триаж стоимости / латентности, когда пользователь сообщает о медленном workflow.
- Pre-release sanity check: какие-то инструменты показывают повышенный `err_count`
  с прошлого билда?
- После марафонской сессии - убедиться, что число деструктивных вызовов совпадает
  с ментальной моделью агента (никаких сюрпризов в виде `forgeplan_delete`).

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `since_hours` | `integer` | нет (по умолчанию 24) | Окно в часах; обработчик ограничивает его диапазоном `1..=720`. `720` = 30 дней. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::ActivityStatsParams`_

## Возвращает
Иллюстративный ответ, не измерение реальной сессии:


```json
{
  "stats": [
    {
      "tool": "forgeplan_score",
      "count": 42,
      "err_count": 0,
      "p50_ms": 110,
      "p95_ms": 240,
      "total_ms": 5180
    },
    {
      "tool": "forgeplan_search",
      "count": 18,
      "err_count": 1,
      "p50_ms": 95,
      "p95_ms": 410,
      "total_ms": 2200
    }
  ],
  "total_calls": 60,
  "total_errors": 1,
  "total_ms": 7380,
  "since_hours": 24,
  "_next_action": "60 call(s), 1 error(s), 7380 ms total. Drill into busiest: `forgeplan_activity tool=\"forgeplan_score\"`."
}
```

Строки отсортированы по `total_ms` по убыванию (сначала самые дорогие), что совпадает
с тем, как подсказка `_next_action` выводит топовый инструмент.

## Пример вызова

Дефолтное окно (последние 24 ч):

```json
{}
```

Месячный роллап:

```json
{ "since_hours": 720 }
```

## Типичная последовательность

1. `forgeplan_activity_stats` - найти горячий или сбоящий инструмент.
2. `forgeplan_activity tool=<hot>` - углубиться в отдельные вызовы.
3. Если выделяются ошибки - `forgeplan_activity tool=<hot> status=tool_err`.

## CLI эквивалент

`forgeplan activity-stats --since-hours 720 --json` читает журнал через CLI.
Для собственного анализа используйте `.forgeplan/logs/tools-*.jsonl`;
ни один из этих интерфейсов не выдаёт число токенов или итоговый счёт.

## См. также

- [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) - entry-level drill-down
- [`forgeplan_undo_last`](/ru/docs/mcp/forgeplan_undo_last/) - в паре со статистикой откатывает осечки
- [Обзор MCP](/ru/docs/mcp/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Aggregate statistics from the activity log grouped by tool name: count, error count, p50/p95 duration, total time. Use to attribute LLM-token spend and identify slow tools. Params: since_hours (default 24, max 720).

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `since_hours` | нет | integer \| null | Time window in hours. Default 24. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "since_hours": {
      "default": null,
      "description": "Time window in hours. Default 24.",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "ActivityStatsParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
