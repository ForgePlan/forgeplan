---
title: forgeplan mcp serve
description: "Запуск полного MCP-сервера по stdio."
---

`forgeplan mcp serve` запускает полный MCP-сервер по stdio. Поведение
совпадает с full-профилем [`forgeplan serve`](/ru/docs/cli/serve/), включая
запись, но параметра `--profile` здесь **нет**. Отдельный бинарь
`forgeplan-mcp` также принимает `--auto-index` и `--workspace`, но не
`--profile`. Для отдельного профиля чтения используйте
`forgeplan serve --profile constellation-read`.

Состав инструментов и схемы зависят от версии. См. [обзор MCP](/ru/docs/mcp/)
и `tools/list` работающего сервера.

## Когда использовать

- Ручная отладка: подавать JSON-RPC запросы, изучать ответы.
- Валидация соответствия MCP-протоколу через `mcp-inspector`.
- Smoke-тестирование после релиза, что бинарник стартует и отдаёт схему инструментов.

## Когда НЕ использовать

- Для повседневной работы с агентом - MCP-клиент (Claude Code, Cursor, Windsurf)
  запускает сервер за вас. Ручной вызов - редкость.
- Для выбора профиля чтения - используйте `forgeplan serve --profile constellation-read`.

## Использование

```text
forgeplan mcp serve [OPTIONS]
```

Эквивалентно:

```text
forgeplan serve --profile full
```

## Опции

```text
      --auto-index        Фоновое обслуживание индекса Constellation
      --workspace <PATH>  Default сервера (окружение на старте приоритетнее)
  -h, --help              Вывести справку
  -V, --version           Вывести версию
```

На старте full-сервера цель обслуживания: `FORGEPLAN_WORKSPACE` → default
сервера из `--workspace` → поиск от текущего каталога. Задайте нужный проект
или явно пустую строку: при `unset` CLI может восстановить значение из `.env`.
Захваченная справка ниже не оговаривает этот приоритет.
Для запроса: явный `workspace` → окружение → default сервера → текущий каталог.
Поздние запросы не перемещают работающий хост.
На уже мигрированном workspace обслуживание подразумевается. `--auto-index` не настраивает
автоматические эмбеддинги. Условия описаны в [руководстве Constellation](/ru/docs/guides/constellation/).

## Примеры

### Smoke-тест после установки

```bash
cd /path/to/project
forgeplan mcp serve
# Server waits on stdin for JSON-RPC. Ctrl-C to exit.
```

Живой процесс показывает лишь, что он не завершился. Для изучения интерфейса выполните
полную инициализацию MCP и `tools/list` (например, через `mcp-inspector`); ни то, ни другое не доказывает работоспособность всех инструментов.

### Отладка кастомного MCP-инструмента

```bash
RUST_LOG=debug forgeplan mcp serve
```

`RUST_LOG=debug` поднимает rmcp-trace диспатча - полезно, когда новый инструмент
зарегистрирован, но клиент утверждает, что его не существует.

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

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan mcp serve --help`.

```text
Start MCP server (alias for `forgeplan serve`)

Usage: forgeplan mcp serve [OPTIONS]

Options:
      --auto-index        Run a Constellation maintenance pass + embedding-queue drain in the background while serving. Same meaning as `forgeplan serve --auto-index`, and on an already-migrated workspace it is implied
      --workspace <PATH>  Workspace this process maintains, when it is not the one the server was started in. Same meaning as `forgeplan serve --workspace`
  -h, --help              Print help
  -V, --version           Print version
```
<!-- forgeplan-runtime:end -->
