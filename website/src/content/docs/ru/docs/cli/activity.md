---
title: forgeplan activity
description: "Просмотр записанных MCP-вызовов, статусов, аргументов и длительности в журнале."
---

`forgeplan activity` читает записанные вызовы MCP-инструментов из `.forgeplan/logs/tools-YYYY-MM-DD.jsonl`: имя инструмента, аргументы, статус (`ok`, `tool_err` или `rpc_err`) и длительность. MCP пишет журнал в режиме best-effort, поэтому это не полный аудит всех CLI-команд и изменений workspace.

Это CLI-вариант [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) (MCP-инструмента); оба читают одни и те же лог-файлы.

## Когда использовать

- Сессия упала или была прервана - посмотреть последние 10–20 действий агента.
- Нужно проверить деструктивные операции (delete / supersede / deprecate) за неделю.
- Workflow тормозит, и нужно увидеть какие именно вызовы инструментов происходили и в каком порядке.
- Сбор фактов для Note или Problem после починки хрупкого процесса - копируйте подходящие записи в тело артефакта.

## Когда НЕ использовать

- Нужны число вызовов, p50/p95 и суммарная длительность по инструментам - используйте [`forgeplan activity-stats`](/ru/docs/cli/activity-stats/), который агрегирует те же записи.
- Нужен живой поток событий - записи появляются на каждый вызов, но для real-time мониторинга используйте `tail -f .forgeplan/logs/tools-*.jsonl`.
- Воркспейс свежий, истории ещё нет - читать нечего.

## Использование

```text
forgeplan activity [OPTIONS]
```

## Опции

```text
      --since-hours <SINCE_HOURS>  Time window in hours back from now (1..=720, default 24) [default: 24]
      --tool <TOOL>                Filter by tool name. Comma-separated for multiple: "forgeplan_score,forgeplan_activate"
      --status <STATUS>            Filter by status: ok, tool_err, or rpc_err. Omit for all
      --limit <LIMIT>              Cap result set (most recent N). 1..=5000, default 500 [default: 500]
      --json                       Output as JSON for machine consumption
  -h, --help                       Print help
  -V, --version                    Print version
```

## Примеры

### Пример 1: Последний час работы

```bash
forgeplan activity --since-hours 1
```

Печатает подходящие записи за последние 60 минут с учётом стандартного лимита 500 записей. Полезно после прерывания сессии для восстановления контекста.

### Пример 2: Аудит деструктивных операций за неделю

```bash
forgeplan activity --since-hours 168 \
  --tool forgeplan_delete,forgeplan_supersede,forgeplan_deprecate
```

Показывает подходящие записи с учётом лимита, не исчерпывающий аудит семи дней. До [`undo-last`](/ru/docs/cli/undo-last/) или [`restore`](/ru/docs/cli/restore/) изучите операцию и квитанцию восстановления.

### Пример 3: Только ошибки, машинно-читаемый вывод

```bash
forgeplan activity --status tool_err --limit 50 --json | jq '.entries[]'
```

Передаёт JSON-вывод в `jq` для дополнительной фильтрации или передачи в скрипт. Флаг `--limit` ограничивает количество результатов - на долгоживущих воркспейсах легко случайно вытащить тысячи записей.

## Место в рабочем процессе

Используйте [`forgeplan activity-stats`](/ru/docs/cli/activity-stats/), чтобы найти медленные или падающие MCP-инструменты, затем `forgeplan activity --tool <имя>` для отдельных записей. При неожиданной деструктивной операции сначала изучите её и квитанцию восстановления, прежде чем вызывать [`forgeplan undo-last`](/ru/docs/cli/undo-last/).

## См. также

- [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) - MCP-эквивалент
- [`forgeplan activity-stats`](/ru/docs/cli/activity-stats/) - агрегаты по инструментам
- [`forgeplan undo-last`](/ru/docs/cli/undo-last/) - откатить последнюю деструктивную операцию
- [`forgeplan restore`](/ru/docs/cli/restore/) - восстановить конкретный soft-deleted артефакт
- [Обзор CLI](/ru/docs/cli/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan activity --help`.

```text
Query the activity log — append-only JSONL record of every MCP tool invocation at .forgeplan/logs/tools-YYYY-MM-DD.jsonl. Use this to reconstruct what the agent did over a time window, attribute LLM-token spend, or audit destructive operations

Usage: forgeplan activity [OPTIONS]

Options:
      --since-hours <SINCE_HOURS>  Time window in hours back from now (1..=720, default 24) [default: 24]
      --tool <TOOL>                Filter by tool name. Comma-separated for multiple: "forgeplan_score,forgeplan_activate"
      --status <STATUS>            Filter by status: ok, tool_err, or rpc_err. Omit for all
      --limit <LIMIT>              Cap result set (most recent N). 1..=5000, default 500 [default: 500]
      --json                       Output as JSON for machine consumption
  -h, --help                       Print help
  -V, --version                    Print version
```
<!-- forgeplan-runtime:end -->
