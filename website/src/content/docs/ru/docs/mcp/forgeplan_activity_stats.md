---
title: forgeplan_activity_stats
description: "Aggregate the activity log by tool name — counts, error rates, p50/p95 duration, total time."
---

Читает тот же журнал активности, что и [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/),
но возвращает по одной строке на каждое уникальное имя инструмента с роллапами:
число вызовов, число ошибок, p50 / p95 длительность, суммарное wall time.
Используйте как отправную точку при расследовании того, куда сессия потратила время,
вместо постраничного просмотра отдельных записей.

**Категория**: Observability & Audit

## Когда агент вызывает

- Старт debug-сессии: «какой инструмент горячий за последние 24 часа?».
- Триаж стоимости / латентности, когда пользователь сообщает о медленном workflow.
- Pre-release sanity check: какие-то инструменты показывают повышенный `err_count`
  с прошлого билда?
- После марафонской сессии — убедиться, что число деструктивных вызовов совпадает
  с ментальной моделью агента (никаких сюрпризов в виде `forgeplan_delete`).

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `since_hours` | `number` | no (default 24, max 720) | Временное окно в часах. `720` = 30 дней. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::ActivityStatsParams`_

## Возвращает

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
  "_next_action": "60 total call(s), 1 error(s), 7380 ms total. Top by time: `forgeplan_score` ..."
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

1. `forgeplan_activity_stats` — найти горячий или сбоящий инструмент.
2. `forgeplan_activity tool=<hot>` — углубиться в отдельные вызовы.
3. Если выделяются ошибки — `forgeplan_activity tool=<hot> status=tool_err`.

## CLI эквивалент

Прямой CLI-команды нет — инструментирование активности живёт в MCP-слое (PRD-055).
Для ad-hoc анализа `jq` поверх `.forgeplan/logs/tools-*.jsonl` даёт ту же форму
с большей гибкостью.

## См. также

- [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) — entry-level drill-down
- [`forgeplan_undo_last`](/ru/docs/mcp/forgeplan_undo_last/) — в паре со статистикой откатывает осечки
- [Обзор MCP](/ru/docs/mcp/)
