---
title: forgeplan mcp
description: "Parent command for MCP integration helpers - install Forgeplan into Claude Code, Cursor, or Windsurf, and start the MCP server."
---

`forgeplan mcp` группирует помощники, которые нужны клиенту AI-агента, чтобы общаться с Forgeplan
по протоколу Model Context Protocol. Это **не** инструмент, который вы вызываете из агентов -
его подкоманды выполняются на хост-машине, чтобы прописать бинарник в файлы конфигурации
клиента (`mcp install`) или вручную запустить stdio-сервер (`mcp serve`, алиас для
[`forgeplan serve`](/ru/docs/cli/serve/)).

Выбранная сборка `0.39.0+constellation.s1.1` предоставляет 76 инструментов в full-профиле и 9 в дополнительном read-only профиле. Это снимки интерфейса, не приёмка всех операций и не глобальный запрет записи. Установленный релиз `0.38.0` не изменён. `mcp install` настраивает клиента; инициализацию и вызовы нужно проверить в самом клиенте.

## Когда использовать

- Сразу после установки Forgeplan и перед первой сессией `claude` / `cursor`.
- Когда обновление Forgeplan через Homebrew делает абсолютный путь в `.mcp.json`
  устаревшим (`forgeplan mcp install` пере-определит его).
- При отладке интеграции: `forgeplan mcp serve` запускает тот же сервер, что и
  `forgeplan serve`, поэтому JSON-RPC-трафик можно изучать с помощью `mcp-inspector`.

## Когда НЕ использовать

- Для повседневной работы с артефактами - для этого есть MCP-инструменты на стороне
  агента (`forgeplan_*`).
- Для HTTP / сетевого доступа - `mcp` покрывает только stdio. Forgeplan - local-first.

## Использование

```text
forgeplan mcp <COMMAND>
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Подкоманды

| Команда | Назначение |
|---|---|
| [`install`](/ru/docs/cli/mcp-install/) | Умное слияние Forgeplan в конфигурацию клиента (Claude / Cursor / Windsurf) |
| [`serve`](/ru/docs/cli/mcp-serve/) | Алиас для [`forgeplan serve`](/ru/docs/cli/serve/) - запускает stdio MCP-сервер |
| `help` | Печатает help для `mcp` или его подкоманд |

## Примеры

### Онбординг свежей установки Claude Code

```bash
forgeplan mcp install --client claude --scope user
```

Пишет user-scoped запись в `~/.claude.json`. Инструменты доступны после загрузки и успешной инициализации клиентом; после обновлений проверяйте путь к бинарнику.

### Project-scoped конфигурация Cursor

```bash
forgeplan mcp install --client cursor --scope project
```

Пишет project-scoped запись в `./.cursor/mcp.json`. Загрузка и запуск зависят от настоящего workspace и политики клиента.

### Запустить сервер вручную (только для отладки)

```bash
cd /path/to/project
forgeplan mcp serve
```

Эффект тот же, что и у `forgeplan serve`. Полезно, когда вы вручную пробрасываете
JSON-RPC или подключаете `mcp-inspector`.

## Место в рабочем процессе

`mcp install` настраивает клиента, `mcp serve` - runtime, который обычно запускает клиент. До использования инструментов подтвердите бинарник, профиль, workspace и handshake. Регистрация сама не доказывает прохождение гейтов методологии.

## См. также

- [`forgeplan mcp install`](/ru/docs/cli/mcp-install/) - прописать Forgeplan в клиента
- [`forgeplan mcp serve`](/ru/docs/cli/mcp-serve/) - алиас для `forgeplan serve`
- [`forgeplan serve`](/ru/docs/cli/serve/) - основной справочник по MCP-серверу
- [Индекс MCP-инструментов](/ru/docs/mcp/) - что предоставляет сервер
- [`forgeplan health`](/ru/docs/cli/health/) - проверить рабочее пространство перед подключением

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan mcp --help`.

```text
MCP integration helpers (install config for AI agent clients)

Usage: forgeplan mcp <COMMAND>

Commands:
  serve    Start MCP server (alias for `forgeplan serve`)
  install  Install forgeplan MCP config into a client (Claude / Cursor / Windsurf)
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

- [`forgeplan mcp serve`](/ru/docs/cli/mcp-serve/)
- [`forgeplan mcp install`](/ru/docs/cli/mcp-install/)
<!-- forgeplan-runtime:end -->
