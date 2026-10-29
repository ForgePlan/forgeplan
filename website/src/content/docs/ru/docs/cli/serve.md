---
title: forgeplan serve
description: "Запуск сервера Forgeplan MCP (транспорт stdio) для интеграции с ИИ-агентами"
---

Запускает сервер Forgeplan MCP (Model Context Protocol) через stdio. Это основная точка интеграции для ИИ-агентов - Claude Code, Cursor, Windsurf и другие MCP-клиенты запускают `forgeplan serve` как дочерний процесс и общаются с ним по JSON-RPC через stdin/stdout.

## Использование

```text
forgeplan serve [OPTIONS]
```

## Опции

```text
      --auto-index        Фоновое обслуживание индекса Constellation
      --profile <PROFILE> full (по умолчанию) или constellation-read
      --workspace <PATH>  Default сервера (окружение на старте приоритетнее)
  -h, --help              Вывести справку
  -V, --version           Вывести версию
```

## Что он предоставляет

Профиль `full` по умолчанию содержит инструменты чтения **и записи**.
Актуальный для версии интерфейс смотрите в [обзоре MCP](/ru/docs/mcp/) и
`tools/list` работающего сервера. Регистрация инструмента не доказывает,
что он поддерживает мигрированное пространство.

В локальной сборке `0.39.0+constellation.s1.1` (не опубликованной 0.38.0)
`--profile constellation-read` выбирает отдельный интерфейс чтения с registry
и handles. Это не read-only переключатель полного пространства имён.
См. [настройку Constellation](/ru/docs/guides/constellation/) и
[контракты чтения](/ru/docs/reference/constellation/).

`--auto-index` включает обслуживание индекса, но само по себе не создаёт
эмбеддинги. Обработка очереди векторов также требует сборки с
`semantic-search`, явной секции `embedding:` с `model: bge-m3`, полного
снапшота модели в кэше и работающего maintenance host. Для уже мигрированного
пространства обслуживание включается автоматически.

## Транспорт

- **Только stdio** - без HTTP, без сокетов, без сетевого доступа. MCP-клиент владеет процессом и общается через каналы.
- **Выбор workspace** - `--workspace` задаёт default сервера. На старте
  full-сервера цель обслуживания: `FORGEPLAN_WORKSPACE` → default сервера →
  поиск от текущего каталога. Задайте нужный проект или явно пустую строку:
  при `unset` CLI может восстановить значение из `.env`. Захваченная справка
  ниже не оговаривает этот приоритет.
  Для инструментов с `workspace`: параметр запроса → окружение → default →
  текущий каталог. Допустимы абсолютные пути и `~/`. Поздний запрос к другому
  workspace не перемещает обслуживание.
- **Состояние профиля** - scope/snapshot handles профиля чтения живут в
  процессе сервера; после перезапуска они недействительны.

## Примеры

### Пример 1: Конфигурация клиента Claude Code
См. раздел "Типичное использование" ниже.

### Пример 2: Ручное дымовое тестирование
См. раздел "Ручное использование (отладка)" ниже.

## Типичное использование (автоматическое)

Обычно сервер запускает MCP-клиент. Для Claude Code используйте
`forgeplan mcp install` или запись в проектном `.mcp.json`:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve", "--profile", "full", "--workspace", "/path/to/your/project"],
      "env": { "FORGEPLAN_WORKSPACE": "/path/to/your/project" }
    }
  }
}
```

Форматы конфигурации клиентов различаются. После правки перезапустите клиент
и проверьте выбранный профиль через `tools/list`.

## Ручное использование (отладка)

Запускайте `serve` напрямую только в следующих случаях:

- **Отладка нового MCP-инструмента** - подключите его вручную, передайте JSON-RPC запросы через пайп, проверьте ответ.
- **Валидация соответствия протоколу** - запустите с `mcp-inspector` или аналогичным инструментом.
- **Дымовое тестирование после релиза** - убедитесь, что бинарный файл запускается и выводит схему своих инструментов.

```bash
# Ручное дымовое тестирование
cd /path/to/project
forgeplan serve
# (сервер ожидает JSON-RPC сообщений на stdin)
# Ctrl-C для выхода
```

Для интерактивного исследования инструмент `mcp-inspector` гораздо продуктивнее, чем ручное написание JSON-RPC.

## Устранение неполадок

- **"Рабочее пространство не найдено"** - укажите корень нужного проекта через
  `--workspace`; рабочий каталог агента и каталог сервера могут различаться.
- **Инструменты отсутствуют в клиенте** - перезапустите MCP-клиент после редактирования его конфигурации; большинство клиентов считывают `mcp.json` только при запуске.
- **Функции LLM не работают** — `reason` и LLM-классификация маршрута требуют провайдера в `.forgeplan/config.yaml`; ключевая маршрутизация сама по себе LLM не требует.
- **Семантический поиск недоступен** - различайте build feature
  `semantic-search`, файлы модели в кэше и векторы в индексе. См.
  [`forgeplan embed`](/ru/docs/cli/embed/). Принудительный semantic-поиск на
  мигрированном workspace не обещает keyword fallback.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [Справочник MCP](/ru/docs/mcp/) - интерфейс выбранного профиля
- [`forgeplan health`](/ru/docs/cli/health/) - диагностика workspace
- [`forgeplan index`](/ru/docs/cli/index-command/) - обслуживание Constellation; legacy `reindex` — только для подходящей раскладки

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan serve --help`.

```text
Start MCP server (stdio transport) for AI agent integration

Usage: forgeplan serve [OPTIONS]

Options:
      --auto-index
          Run a Constellation maintenance pass + embedding-queue drain in the background while serving (explicit opt-in; the index itself is never required for reads)

      --profile <PROFILE>
          Server profile: `full` (default — all tools) or `constellation-read` (stage-5 read profile — only scope/snapshot handle reads; no write tools, no raw paths accepted)
          
          [default: full]

      --workspace <PATH>
          Workspace this process maintains, when it is not the one the server was started in.
          
          Absent, the maintenance host follows the process's working directory. Present, it follows **this** path — so `serve --auto-index --workspace Y` in a checkout of X maintains Y and never X. Without the flag, `forgeplan serve` started in X had no way to keep a second workspace current at all, and any attempt to infer one from the server's own default resolution would silently pick X again.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
<!-- forgeplan-runtime:end -->
