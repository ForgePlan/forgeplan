---
title: forgeplan mcp install
description: "Smart-merge Forgeplan into Claude Code, Cursor, or Windsurf MCP config — cross-platform, idempotent, brew-upgrade-safe."
---

`forgeplan mcp install` записывает MCP-сервер Forgeplan в файл конфигурации клиента
(`.mcp.json`, `~/.claude.json`, `~/.cursor/mcp.json` или `~/.codeium/windsurf/mcp_config.json`),
не затирая ничего другого, что там уже есть. Команда определяет абсолютный путь к
работающему бинарнику, мерджит его в карту `mcpServers` и сохраняет существующий блок
`env` для записи Forgeplan — поэтому повторный запуск после обновления Homebrew просто
обновляет путь.

Кроссплатформенно: macOS, Linux, Windows (использует `dirs::home_dir()` и `PATHEXT` для
разрешения путей).

## Когда использовать

- Первая настройка после `brew install forgeplan` (или эквивалента).
- После того как `brew upgrade forgeplan` инвалидирует абсолютный путь Cellar,
  зашитый в `.mcp.json`.
- При онбординге нового клиента (например, вы использовали Claude Code, теперь
  добавляете Cursor).
- В CI для бутстрапа изолированного окружения агента с доступным Forgeplan.

## Когда НЕ использовать

- Для пер-tool конфигурации — настраивать нечего, сервер читает `./.forgeplan/`.
- Для HTTP / сетевого MCP — Forgeplan поставляется только со stdio.
- Чтобы удалить Forgeplan из клиента — отредактируйте файл конфигурации вручную;
  `install` только добавляет.

## Использование

```text
forgeplan mcp install [OPTIONS] --client <CLIENT>
```

## Опции

```text
  -c, --client <CLIENT>          Target client: claude, cursor, or windsurf
  -s, --scope <SCOPE>            Config scope: user (global) or project (local) [default: user]
      --binary-path <PATH>       Override binary path (default: detected from current_exe)
      --use-name <NAME>          Use short name instead of absolute path: forgeplan or fpl
      --dry-run                  Print proposed change without writing
  -h, --help                     Print help
  -V, --version                  Print version
```

`--binary-path` и `--use-name` взаимоисключающи. По умолчанию команда разрешает
работающий бинарник в стабильный, не-версионированный путь (например,
`/opt/homebrew/bin/forgeplan`, а не Cellar-локацию), чтобы `brew upgrade` не ломал запись.

## Примеры

### Пример 1: Claude Code, на пользователя

```bash
forgeplan mcp install --client claude
```

Записывает `~/.claude.json`. Scope по умолчанию — `user`, поэтому каждый проект,
который открывает Claude Code, видит инструменты Forgeplan.

### Пример 2: Cursor, только проект

```bash
forgeplan mcp install --client cursor --scope project
```

Записывает `./.cursor/mcp.json`. Forgeplan загружается только когда активным
рабочим пространством является этот репозиторий — полезно, когда лишь часть
проектов в монорепозитории использует Forgeplan.

### Пример 3: Превью перед записью

```bash
forgeplan mcp install --client windsurf --dry-run
```

Печатает смерженный JSON без изменений на файловой системе. Просмотрите diff,
затем перезапустите без `--dry-run`, когда всё устроит.

### Пример 4: Использовать короткое имя вместо абсолютного пути

```bash
forgeplan mcp install --client cursor --use-name forgeplan
```

Записывает `"command": "forgeplan"` — рассчитывает на `$PATH` в момент запуска MCP.
**Оговорка для GUI-клиентов macOS**: Claude Code Mac и Cursor app **не** наследуют
shell PATH, поэтому короткие имена ломаются, если вы не настроили
`launchctl setenv PATH ...`. Дефолт (абсолютный путь) — более безопасный выбор.

## Записываемые файлы конфигурации

| Клиент | User scope | Project scope |
|---|---|---|
| `claude` | `~/.claude.json` | `./.mcp.json` |
| `cursor` | `~/.cursor/mcp.json` | `./.cursor/mcp.json` |
| `windsurf` | `~/.codeium/windsurf/mcp_config.json` | _не поддерживается_ |

У Windsurf нет per-project конфигурации; передавайте `--scope user` (это значение
по умолчанию).

## Поведение умного слияния

- Заменяет `command`, `args` и transport для записи `forgeplan`.
- **Сохраняет** существующий блок `env` для записи (project-specific API-ключи и т. д.).
- Оставляет все остальные серверы в `mcpServers` нетронутыми.
- Идемпотентно — повторный запуск с теми же флагами ничего не делает.

## Место в рабочем процессе

`mcp install` — мост между «бинарник на диске» и «агент может вызывать инструменты
Forgeplan». После успеха перезапустите клиента, и поверхность методологии
(Shape → Validate → Code → Evidence → Activate) станет доступна через
инструменты `mcp__forgeplan__*`. После рестарта вызовите `forgeplan health`, чтобы
убедиться, что сервер стартует без ошибок.

## См. также

- [`forgeplan mcp`](/ru/docs/cli/mcp/) — родительская команда
- [`forgeplan mcp serve`](/ru/docs/cli/mcp-serve/) — запустить сервер (алиас)
- [`forgeplan serve`](/ru/docs/cli/serve/) — справочник по нижележащему серверу
- [Индекс MCP-инструментов](/ru/docs/mcp/) — что предоставляет сервер после установки
- [`forgeplan health`](/ru/docs/cli/health/) — проверка после рестарта клиента
