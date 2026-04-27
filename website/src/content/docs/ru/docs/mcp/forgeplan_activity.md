---
title: forgeplan_activity
description: "Query the activity log — append-only JSONL record of every MCP tool invocation."
---

Возвращает записи журнала активности рабочего пространства
(`.forgeplan/logs/tools-YYYY-MM-DD.jsonl`), которые подходят под заданные фильтры.
Forgeplan логирует каждый вызов MCP-инструмента — имя инструмента, дайджест аргументов,
статус, длительность, класс ошибки — чтобы агент мог восстановить, что произошло, не
полагаясь на ненадёжную память. Используйте, чтобы атрибутировать расход LLM-токенов,
аудитить деструктивные операции или восстановить таймлайн сессии после прерывания.

**Категория**: Observability & Audit

## Когда агент вызывает

- После прерывания сессии — «какие инструменты я запускал в последний час?».
- Перед вызовом деструктивной операции — убедиться, что предыдущая завершилась и не
  повторилась по ошибке.
- Когда пользователь спрашивает «куда ушли токены?» — углубиться в самые медленные / частые
  инструменты.
- Чтобы построить судебный след для `Note` после починки хрупкого workflow.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `since_hours` | `number` | no (default 24, max 720) | Временное окно в часах назад от текущего момента. `1` = последний час, `720` = последние 30 дней. |
| `tool` | `string` | no | Имена инструментов через запятую для фильтра, например `"forgeplan_score,forgeplan_activate"`. |
| `status` | `string` | no | Фильтр по статусу — одно из `ok`, `tool_err`, `rpc_err`. |
| `limit` | `number` | no (default 500, max 5000) | Ограничение размера выборки; оставляет N самых свежих записей. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::ActivityQueryParams`_

## Возвращает

```json
{
  "entries": [
    {
      "ts": "2026-04-26T10:14:22Z",
      "tool": "forgeplan_score",
      "status": "ok",
      "duration_ms": 142
    }
  ],
  "total_scanned": 312,
  "returned": 1,
  "warnings": [],
  "since_hours": 24,
  "_next_action": "1 entries in window. Busiest tool: `forgeplan_score`. ..."
}
```

Подсказка `_next_action` направляет агента к правильному следующему шагу
(`forgeplan_activity_stats` для агрегатов или более узкий фильтр `tool=`).

## Пример вызова

Последний час работы:

```json
{ "since_hours": 1 }
```

Все деструктивные операции за последнюю неделю:

```json
{ "since_hours": 168, "tool": "forgeplan_delete,forgeplan_supersede,forgeplan_deprecate" }
```

Только ошибки:

```json
{ "status": "tool_err", "limit": 50 }
```

## Типичная последовательность

1. `forgeplan_activity_stats` — быстрый агрегат, чтобы найти загруженные / медленные инструменты.
2. `forgeplan_activity tool=<top>` — углубиться в записи конкретного инструмента.
3. Если деструктивная операция всплыла неожиданно: [`forgeplan_undo_last`](/ru/docs/mcp/forgeplan_undo_last/).

## CLI эквивалент

Прямого CLI-аналога пока нет — журнал активности намеренно MCP-first
(введён через PRD-055). Сырые JSONL-файлы в `.forgeplan/logs/tools-*.jsonl`
человекочитаемы и работают как fallback.

## См. также

- [`forgeplan_activity_stats`](/ru/docs/mcp/forgeplan_activity_stats/) — агрегаты по инструментам
- [`forgeplan_undo_last`](/ru/docs/mcp/forgeplan_undo_last/) — отменить последнюю деструктивную операцию
- [`forgeplan_restore`](/ru/docs/mcp/forgeplan_restore/) — восстановить конкретный мягко удалённый артефакт
- [Обзор MCP](/ru/docs/mcp/)
