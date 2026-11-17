---
title: forgeplan activity
description: "Query the activity log - append-only JSONL record of every MCP tool invocation. Use to reconstruct what the agent did, attribute spend, or audit destructive operations."
---

`forgeplan activity` читает записанные MCP-вызовы из дневных файлов `.forgeplan/logs/tools-YYYY-MM-DD.jsonl`: имя инструмента, аргументы, статус (`ok`, `tool_err` или `rpc_err`) и длительность. Это не журнал всех CLI-команд или всех действий агента. Он помогает узнать, какие записанные вызовы происходили за последний час.

Это CLI-вариант [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) (MCP-инструмента); оба читают одни и те же лог-файлы.

## Когда использовать

- Сессия упала или была прервана - посмотреть последние 10–20 действий агента.
- Нужно проверить деструктивные операции (delete / supersede / deprecate) за неделю.
- Workflow тормозит, и нужно увидеть какие именно вызовы инструментов происходили и в каком порядке.
- Сбор фактов для Note или Problem после починки хрупкого процесса - копируйте подходящие записи в тело артефакта.

## Когда НЕ использовать

- Нужны итоги (количество вызовов, среднее время) по каждому инструменту - используйте [`forgeplan activity-stats`](/ru/docs/cli/activity-stats/), он агрегирует те же данные.
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

По умолчанию сохраняет последние 500 подходящих вызовов и печатает их в хронологическом порядке журнала, а не от новых к старым. При таком окне записи относятся к последним 60 минутам. Полезно после прерывания сессии для восстановления контекста.

### Пример 2: Аудит деструктивных операций за неделю

```bash
forgeplan activity --since-hours 168 \
  --tool forgeplan_delete,forgeplan_supersede,forgeplan_deprecate
```

Поднимает каждое soft-delete / supersede / deprecate за 7 дней. Совмещайте с
[`forgeplan undo-last`](/ru/docs/cli/undo-last/) или [`forgeplan restore`](/ru/docs/cli/restore/),
чтобы откатить неожиданные операции.

### Пример 3: Только ошибки, машинно-читаемый вывод

```bash
forgeplan activity --status tool_err --limit 50 --json | jq '.entries[]'
```

Передаёт JSON-вывод в `jq` для дополнительной фильтрации или передачи в скрипт. Флаг `--limit` ограничивает количество результатов - на долгоживущих воркспейсах легко случайно вытащить тысячи записей.

## JSON и ограничения

JSON содержит `entries`, `total_scanned`, `returned`, `warnings`, `since_hours`, `_next_action`, `hints`. Поля записи: `ts`, `tool`, `args_hash`, `duration_ms`, `status`, `workspace`, необязательные `client_info` и `args`. Исходные аргументы отсутствуют без `activity.log_args: true`. Обработчик ограничивает окно диапазоном 1–720 часов, число результатов — 1–5000; неизвестная строка статуса просто не находит записей. Пустой результат — успешный отчёт, не ошибка команды. Требуется инициализированный воркспейс.

## Место в рабочем процессе

Лог активности — история записанных MCP-вызовов, а не всех изменений воркспейса. Используйте [`forgeplan activity-stats`](/ru/docs/cli/activity-stats/), чтобы найти медленные или падающие инструменты, затем `forgeplan activity --tool <имя>` для отдельных вызовов. Изучите неожиданную деструктивную операцию, прежде чем выбирать [`forgeplan undo-last`](/ru/docs/cli/undo-last/) или адресное восстановление.

## См. также

- [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) - MCP-эквивалент
- [`forgeplan activity-stats`](/ru/docs/cli/activity-stats/) - агрегаты по инструментам
- [`forgeplan undo-last`](/ru/docs/cli/undo-last/) - откатить последнюю деструктивную операцию
- [`forgeplan restore`](/ru/docs/cli/restore/) - восстановить конкретный soft-deleted артефакт
- [Обзор CLI](/ru/docs/cli/)
