---
title: forgeplan mcp
description: "Parent command for MCP integration helpers — install Forgeplan into Claude Code, Cursor, or Windsurf, and start the MCP server."
---

`forgeplan mcp` группирует помощники, которые нужны клиенту AI-агента, чтобы общаться с Forgeplan
по протоколу Model Context Protocol. Это **не** инструмент, который вы вызываете из агентов —
его подкоманды выполняются на хост-машине, чтобы прописать бинарник в файлы конфигурации
клиента (`mcp install`) или вручную запустить stdio-сервер (`mcp serve`, алиас для
[`forgeplan serve`](/ru/docs/cli/serve/)).

Forgeplan — MCP-first: большая часть повседневной поверхности (72 инструмента) доступна
через сервер. Эта родительская команда существует для того, чтобы один-единственный
`forgeplan mcp install --client claude` довёл вас от свежего `brew install forgeplan` до
работающего агента без ручного редактирования JSON.

## Когда использовать

- Сразу после установки Forgeplan и перед первой сессией `claude` / `cursor`.
- Когда обновление Forgeplan через Homebrew делает абсолютный путь в `.mcp.json`
  устаревшим (`forgeplan mcp install` пере-определит его).
- При отладке интеграции: `forgeplan mcp serve` запускает тот же сервер, что и
  `forgeplan serve`, поэтому JSON-RPC-трафик можно изучать с помощью `mcp-inspector`.

## Когда НЕ использовать

- Для повседневной работы с артефактами — для этого есть MCP-инструменты на стороне
  агента (`forgeplan_*`).
- Для HTTP / сетевого доступа — `mcp` покрывает только stdio. Forgeplan — local-first.

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
| [`serve`](/ru/docs/cli/mcp-serve/) | Алиас для [`forgeplan serve`](/ru/docs/cli/serve/) — запускает stdio MCP-сервер |
| `help` | Печатает help для `mcp` или его подкоманд |

## Примеры

### Онбординг свежей установки Claude Code

```bash
forgeplan mcp install --client claude --scope user
```

Записывает Forgeplan в `~/.claude.json`, чтобы каждая сессия Claude Code видела
инструменты `mcp__forgeplan__*`. Идемпотентно — безопасно перезапускать после обновлений.

### Project-scoped конфигурация Cursor

```bash
forgeplan mcp install --client cursor --scope project
```

Создаёт `./.cursor/mcp.json`, чтобы сервер Forgeplan запускался только когда открыт
этот репозиторий. Идеально для монорепозиториев, где Forgeplan используют не все проекты.

### Запустить сервер вручную (только для отладки)

```bash
cd /path/to/project
forgeplan mcp serve
```

Эффект тот же, что и у `forgeplan serve`. Полезно, когда вы вручную пробрасываете
JSON-RPC или подключаете `mcp-inspector`.

## Место в рабочем процессе

`mcp install` — это однократный шаг настройки между «бинарник на диске» и «агент может
вызывать инструменты». После того как он отработает, остальная методология
(Shape → Validate → Code → Evidence → Activate) выполняется через MCP-инструменты,
которые предоставляет сервер. `mcp serve` — это runtime; вы почти никогда не
вызываете его вручную, потому что клиент запускает его за вас.

## См. также

- [`forgeplan mcp install`](/ru/docs/cli/mcp-install/) — прописать Forgeplan в клиента
- [`forgeplan mcp serve`](/ru/docs/cli/mcp-serve/) — алиас для `forgeplan serve`
- [`forgeplan serve`](/ru/docs/cli/serve/) — основной справочник по MCP-серверу
- [Индекс MCP-инструментов](/ru/docs/mcp/) — что предоставляет сервер
- [`forgeplan health`](/ru/docs/cli/health/) — проверить рабочее пространство перед подключением
