---
title: forgeplan mcp serve
description: "Alias for `forgeplan serve` — starts the Forgeplan MCP server on stdio."
---

`forgeplan mcp serve` — это тонкий алиас для [`forgeplan serve`](/ru/docs/cli/serve/).
Обе команды запускают один и тот же MCP-сервер (stdio transport, 71 инструмента,
одно рабочее пространство) и завершаются по Ctrl-C. Алиас существует, чтобы
namespace `mcp` был внутренне консистентным — после `forgeplan mcp install`
очевидное продолжение для отладки — это `forgeplan mcp serve`.

Полный справочник (доступные инструменты, детали транспорта, troubleshooting,
примеры конфигурации клиентов) — см. [`forgeplan serve`](/ru/docs/cli/serve/).

## Когда использовать

- Ручная отладка: подавать JSON-RPC запросы, изучать ответы.
- Валидация соответствия MCP-протоколу через `mcp-inspector`.
- Smoke-тестирование после релиза, что бинарник стартует и отдаёт схему инструментов.

## Когда НЕ использовать

- Для повседневной работы с агентом — MCP-клиент (Claude Code, Cursor, Windsurf)
  запускает сервер за вас. Ручной вызов — редкость.
- Для конфигурации сервера — на командной строке настраивать нечего. Сервер
  читает `./.forgeplan/`.

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

Никаких runtime-опций — сервер берёт рабочее пространство из `./.forgeplan/`, а
LLM-провайдера — из `.forgeplan/config.yaml`.

## Примеры

### Smoke-тест после установки

```bash
cd /path/to/project
forgeplan mcp serve
# Server waits on stdin for JSON-RPC. Ctrl-C to exit.
```

Если процесс не падает сразу же — бинарник стартует. Используйте `mcp-inspector`
для интерактивного дампа списка инструментов.

### Отладка кастомного MCP-инструмента

```bash
RUST_LOG=debug forgeplan mcp serve
```

`RUST_LOG=debug` поднимает rmcp-trace диспатча — полезно, когда новый инструмент
зарегистрирован, но клиент утверждает, что его не существует.

## Место в рабочем процессе

`mcp serve` — это runtime entry point: AI-агенты (Claude Code, Cursor, Windsurf)
запускают его как подпроцесс через файл конфигурации, который записал
`forgeplan mcp install`. Вы почти никогда не вызываете его напрямую при обычной
работе с артефактами — цикл методологии (Shape → Validate → Code → Evidence → Activate)
выполняется через инструменты, которые предоставляет сервер, а не через эту команду.

## См. также

- [`forgeplan serve`](/ru/docs/cli/serve/) — основной справочник (инструменты, транспорт, troubleshooting)
- [`forgeplan mcp`](/ru/docs/cli/mcp/) — родительская команда
- [`forgeplan mcp install`](/ru/docs/cli/mcp-install/) — прописать это в клиента
- [Индекс MCP-инструментов](/ru/docs/mcp/) — что предоставляет сервер
- [`forgeplan health`](/ru/docs/cli/health/) — проверить рабочее пространство перед запуском
