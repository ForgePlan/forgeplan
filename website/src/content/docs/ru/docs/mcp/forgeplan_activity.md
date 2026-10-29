---
title: forgeplan_activity
description: "Запрос записанной MCP-активности workspace по времени, инструменту и исходу."
---

Возвращает подходящие записи из `.forgeplan/logs/tools-YYYY-MM-DD.jsonl`.
В них есть имя инструмента, дайджест аргументов, исход и длительность,
не расход токенов или отдельное поле класса ошибки. Запись best-effort
и без fsync: отсутствие строки не доказывает, что операции не было.
Исследуйте расходы по длительностям и числу вызовов вместе с учётом
провайдера, не выдавая их за прямые измерения токенов или стоимости.

**Категория**: Observability & Audit

## Когда агент вызывает

- После прерывания сессии - «какие инструменты я запускал в последний час?».
- Перед вызовом деструктивной операции - убедиться, что предыдущая завершилась и не
  повторилась по ошибке.
- Когда пользователь спрашивает «куда ушли токены?» - углубиться в самые медленные / частые
  инструменты.
- Чтобы построить судебный след для `Note` после починки хрупкого workflow.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `since_hours` | `integer \| null` | no (по умолчанию 24) | Скользящее окно часов назад, ограниченное 1..=720. Несмотря на “today-only” в тексте захваченной схемы, пропуск означает 24 часа в handler. |
| `tool` | `string` | no | Имена инструментов через запятую для фильтра, например `"forgeplan_score,forgeplan_activate"`. |
| `status` | `string` | no | Фильтр по статусу - одно из `ok`, `tool_err`, `rpc_err`. |
| `limit` | `integer \| null` | no (по умолчанию 500) | Оставляет N самых свежих записей; ограничивается 1..=5000. |
| `workspace` | `string \| null` | no | Явный абсолютный путь workspace или путь с `~/`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::ActivityQueryParams`_

## Возвращает

Ниже иллюстративный частичный payload, не зафиксированный аудит.
Реальные записи также содержат `args_hash`, `workspace`, необязательное
`client_info` и opt-in `args`. Проверяйте `warnings` о пропущенных повреждённых данных.

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

1. `forgeplan_activity_stats` - быстрый агрегат, чтобы найти загруженные / медленные инструменты.
2. `forgeplan_activity tool=<top>` - углубиться в записи конкретного инструмента.
3. Если деструктивная операция всплыла неожиданно: [`forgeplan_undo_last`](/ru/docs/mcp/forgeplan_undo_last/).

## CLI эквивалент

Прямого CLI-аналога пока нет - журнал активности намеренно MCP-first
(введён через PRD-055). Сырые JSONL-файлы в `.forgeplan/logs/tools-*.jsonl`
человекочитаемы и работают как fallback.

## См. также

- [`forgeplan_activity_stats`](/ru/docs/mcp/forgeplan_activity_stats/) - агрегаты по инструментам
- [`forgeplan_undo_last`](/ru/docs/mcp/forgeplan_undo_last/) - отменить последнюю деструктивную операцию
- [`forgeplan_restore`](/ru/docs/mcp/forgeplan_restore/) - восстановить конкретный мягко удалённый артефакт
- [Обзор MCP](/ru/docs/mcp/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Query the activity log — append-only JSONL record of every MCP tool invocation at .forgeplan/logs/tools-YYYY-MM-DD.jsonl. Use this to reconstruct what the agent did over a time window, attribute LLM-token spend, or audit destructive operations. Params: since_hours (default 24, max 720), tool (comma-separated names to filter), status (ok/tool_err/rpc_err), limit (default 500, max 5000 — keeps most recent).

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `limit` | нет | integer \| null | Cap result set (most recent N). Default 500, max 5000. |
| `since_hours` | нет | integer \| null | Time window in hours back from now (e.g. 1 = last hour, 24 = last day).<br>Omit for today-only scope. Values 1..=720 (30 days). |
| `status` | нет | string \| null | Filter by status: `ok`, `tool_err`, or `rpc_err`. Omit for all. |
| `tool` | нет | string \| null | Filter by tool name. Repeat with comma to match multiple:<br>`"forgeplan_score,forgeplan_activate"`. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "limit": {
      "default": null,
      "description": "Cap result set (most recent N). Default 500, max 5000.",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "since_hours": {
      "default": null,
      "description": "Time window in hours back from now (e.g. 1 = last hour, 24 = last day).\nOmit for today-only scope. Values 1..=720 (30 days).",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "status": {
      "default": null,
      "description": "Filter by status: `ok`, `tool_err`, or `rpc_err`. Omit for all.",
      "type": [
        "string",
        "null"
      ]
    },
    "tool": {
      "default": null,
      "description": "Filter by tool name. Repeat with comma to match multiple:\n`\"forgeplan_score,forgeplan_activate\"`.",
      "type": [
        "string",
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
  "title": "ActivityQueryParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
