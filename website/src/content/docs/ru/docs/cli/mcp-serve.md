---
title: forgeplan mcp serve
description: "Alias for `forgeplan serve` - starts the Forgeplan MCP server on stdio."
---

`forgeplan mcp serve` - это тонкий алиас для [`forgeplan serve`](/ru/docs/cli/serve/).
Обе команды запускают один stdio MCP-сервер (73 инструмента в снимке релиза
0.38.0). Он работает до завершения соединения или прерывания процесса.
Алиас сохраняет согласованность пространства команд `mcp`: после
`forgeplan mcp install` его удобно использовать для отладки.

Полный справочник (доступные инструменты, детали транспорта, troubleshooting,
примеры конфигурации клиентов) - см. [`forgeplan serve`](/ru/docs/cli/serve/).

## Когда использовать

- Ручная отладка: подавать JSON-RPC запросы, изучать ответы.
- Валидация соответствия MCP-протоколу через `mcp-inspector`.
- Smoke-тестирование после релиза, что бинарник стартует и отдаёт схему инструментов.

## Когда НЕ использовать

- Для повседневной работы с агентом - MCP-клиент (Claude Code, Cursor, Windsurf)
  запускает сервер за вас. Ручной вызов - редкость.
- Для настройки инструментов runtime-флагами — их нет. При запуске пространство
  ищется от текущего каталога; поддерживающие это инструменты могут выбрать
  другое через вход `workspace` или `FORGEPLAN_WORKSPACE`.

## Использование

```text
forgeplan mcp serve
```

Эквивалентно:

```text
forgeplan serve
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

Runtime-опций нет. Каталог запуска задаёт пространство по умолчанию, но не
границу изоляции одного пространства: поддерживающие маршрутизацию инструменты
могут направлять отдельные вызовы в другие пространства. Провайдер читается
из конфигурации при LLM-операции; запуск сервера не вызывает LLM.

## Примеры

### Smoke-тест после установки

```bash
cd /path/to/project
forgeplan mcp serve
# Server waits on stdin for JSON-RPC. Ctrl-C to exit.
```

Если процесс не падает сразу же - бинарник стартует. Используйте `mcp-inspector`
для интерактивного дампа списка инструментов.

### Отладка кастомного MCP-инструмента

```bash
RUST_LOG=debug forgeplan mcp serve
```

CLI-точка входа не устанавливает tracing subscriber, поэтому один
`RUST_LOG=debug` в 0.38.0 не гарантирует rmcp-журнал диспетчеризации.
Для изучения initialize и `tools/list` используйте protocol inspector.

## Место в рабочем процессе

`mcp serve` - это runtime entry point: AI-агенты (Claude Code, Cursor, Windsurf)
запускают его как подпроцесс через файл конфигурации, который записал
`forgeplan mcp install`. Вы почти никогда не вызываете его напрямую при обычной
работе с артефактами - цикл методологии (Shape → Validate → Code → Evidence → Activate)
выполняется через инструменты, которые предоставляет сервер, а не через эту команду.

## См. также

- [`forgeplan serve`](/ru/docs/cli/serve/) - основной справочник (инструменты, транспорт, troubleshooting)
- [`forgeplan mcp`](/ru/docs/cli/mcp/) - родительская команда
- [`forgeplan mcp install`](/ru/docs/cli/mcp-install/) - прописать это в клиента
- [Индекс MCP-инструментов](/ru/docs/mcp/) - что предоставляет сервер
- [`forgeplan health`](/ru/docs/cli/health/) - проверить рабочее пространство перед запуском
