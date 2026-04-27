---
title: forgeplan activity
description: "Query the activity log — append-only JSONL record of every MCP tool invocation. Use to reconstruct what the agent did, attribute spend, or audit destructive operations."
---

`forgeplan activity` показывает, что на самом деле делал агент (или вы). Каждая команда Forgeplan пишет одну строку в дневной лог-файл `.forgeplan/logs/tools-YYYY-MM-DD.jsonl` — имя инструмента, аргументы, статус (ok / ошибка), длительность. Эта команда читает лог и печатает записи, подходящие под ваши фильтры, чтобы можно было ответить «что происходило за последний час?» не полагаясь на память.

Это CLI-вариант [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) (MCP-инструмента); оба читают одни и те же лог-файлы.

## Когда использовать

- Сессия упала или была прервана — посмотреть последние 10–20 действий агента.
- Нужно проверить деструктивные операции (delete / supersede / deprecate) за неделю.
- Workflow тормозит, и нужно увидеть какие именно вызовы инструментов происходили и в каком порядке.
- Сбор фактов для Note или Problem после починки хрупкого процесса — копируйте подходящие записи в тело артефакта.

## Когда НЕ использовать

- Нужны итоги (количество вызовов, среднее время) по каждому инструменту — используйте [`forgeplan activity-stats`](/ru/docs/cli/activity-stats/), он агрегирует те же данные.
- Нужен живой поток событий — записи появляются на каждый вызов, но для real-time мониторинга используйте `tail -f .forgeplan/logs/tools-*.jsonl`.
- Воркспейс свежий, истории ещё нет — читать нечего.

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

Печатает все вызовы инструментов за последние 60 минут, начиная с самых свежих.
Полезно сразу после прерывания сессии для восстановления контекста.

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

Передаёт JSON-вывод в `jq` для дополнительной фильтрации или передачи в скрипт. Флаг `--limit` ограничивает количество результатов — на долгоживущих воркспейсах легко случайно вытащить тысячи записей.

## Место в рабочем процессе

Лог активности — это аудит-trail под всеми остальными командами Forgeplan. Обычная связка: сначала запустите [`forgeplan activity-stats`](/ru/docs/cli/activity-stats/), чтобы по агрегатам найти медленный или падающий инструмент, затем `forgeplan activity --tool <имя>` здесь, чтобы увидеть конкретные вызовы. Если в логе всплыла неожиданная деструктивная операция — откатите её через [`forgeplan undo-last`](/ru/docs/cli/undo-last/).

## См. также

- [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) — MCP-эквивалент
- [`forgeplan activity-stats`](/ru/docs/cli/activity-stats/) — агрегаты по инструментам
- [`forgeplan undo-last`](/ru/docs/cli/undo-last/) — откатить последнюю деструктивную операцию
- [`forgeplan restore`](/ru/docs/cli/restore/) — восстановить конкретный soft-deleted артефакт
- [Обзор CLI](/ru/docs/cli/)
