---
title: forgeplan serve
description: "Запускает сервер Model Context Protocol (MCP) Forgeplan через stdio. Клиент управляет дочерним процессом и обменивается JSON-RPC через stdin/stdout. Сервер 0.38.0 предоставляет 73 инструмента, а не копию всех команд CLI."
---

Запускает сервер Model Context Protocol (MCP) Forgeplan через stdio. Клиент управляет дочерним процессом и обменивается JSON-RPC через stdin/stdout. Сервер 0.38.0 предоставляет 73 инструмента, а не копию всех команд CLI.

## Использование

```text
forgeplan serve
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Доступные операции

Примеры: `forgeplan_new`, `forgeplan_get`, `forgeplan_validate`, `forgeplan_score`, `forgeplan_reason`, `forgeplan_claim`, `forgeplan_discover_finding`. Инструментов `forgeplan_context`, `forgeplan_tree`, `forgeplan_fgr`, `forgeplan_tag`, `forgeplan_remember`, `forgeplan_new_evidence` нет. Доказательство создаётся через `forgeplan_new` с нужным типом. Все 73 схемы перечислены в справочнике MCP.

## Настройка клиента

Используйте установщик, учитывающий клиент, а не переносите одну схему конфигурации между клиентами:

```bash
forgeplan mcp install --client claude --scope user --dry-run
forgeplan mcp install --client claude --scope user
```

Пользовательская конфигурация Claude — `~/.claude.json`, проектная — `.mcp.json`. У Cursor и Windsurf свои пути и соглашения. Запускайте сервер с рабочим каталогом проекта; не каждый клиент поддерживает поле `cwd`. После изменения настроек переподключите клиент.

## Ручной запуск

```bash
# Run from the intended project root:
forgeplan serve
```

Сервер ждёт сообщения протокола, не интерактивные команды. stdout нужен для протокола. Файлы пространства и состояние методологической сессии сохраняются между запросами; API не следует считать не хранящим состояние.

## Устранение проблем

Если пространство не найдено, проверьте каталог запуска. Если инструментов нет, переподключите клиент и изучите tools/list. Для LLM нужен настроенный провайдер. Отличайте включённую в сборку возможность семантического поиска CLI от обработчика MCP: в 0.38.0 запросы MCP smart/semantic идут по пути smart search без векторов, поскольку семантические оценки к нему не подключены. Загрузка весов BGE-M3 это ограничение MCP не исправляет. Поиск по ключевым словам доступен.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [Справочник по MCP-инструментам](/ru/docs/mcp/) - полный список предоставляемых инструментов
- [`forgeplan health`](/ru/docs/cli/health/) - проверка рабочего пространства перед запуском сервера
- [`forgeplan reindex`](/ru/docs/cli/reindex/) - перестроение индекса, если инструменты возвращают устаревшие данные
