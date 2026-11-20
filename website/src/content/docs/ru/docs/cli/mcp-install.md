---
title: forgeplan mcp install
description: "Добавляет Forgeplan в конфигурацию MCP Claude Code, Cursor или Windsurf, сохраняя другие записи."
---

`forgeplan mcp install` записывает MCP-сервер Forgeplan в файл конфигурации клиента
(`.mcp.json`, `~/.claude.json`, `~/.cursor/mcp.json` или `~/.codeium/windsurf/mcp_config.json`),
не затирая ничего другого, что там уже есть. Команда определяет абсолютный путь к
работающему бинарнику, мерджит его в карту `mcpServers` и сохраняет существующий блок
`env` для записи Forgeplan - поэтому повторный запуск после обновления Homebrew просто
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

- Для выбора профиля или политики workspace: установка пишет конфигурацию клиента; фактический запуск сервера настраивается и проверяется отдельно.
- Для HTTP / сетевого MCP - Forgeplan поставляется только со stdio.
- Чтобы удалить Forgeplan из клиента - отредактируйте файл конфигурации вручную;
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

`--binary-path` и `--use-name` взаимоисключающие. Стабильная запись PATH выбирается лишь при совпадении с исполняемым файлом установщика; иначе используется `current_exe`. Стабильность после обновления не безусловна. Чтобы не смешать выбранную `0.39.0+constellation.s1.1` и установленную `0.38.0`, задайте абсолютный `--binary-path` выбранной сборки и изучите dry-run.

## Примеры

### Пример 1: Claude Code, на пользователя

```bash
forgeplan mcp install --client claude
```

Пишет user-scoped запись в `~/.claude.json`; доступность в проектах зависит от загрузки и инициализации клиентом.

### Пример 2: Cursor, только проект

```bash
forgeplan mcp install --client cursor --scope project
```

Пишет `./.cursor/mcp.json` для project scope. Загрузка определяется workspace и политикой клиента.

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

Записывает `"command": "forgeplan"` и полагается на PATH при старте MCP. GUI-приложения macOS могут не наследовать shell PATH; абсолютный путь убирает эту зависимость. Проверяйте фактическое окружение клиента, а не предполагаемое универсальное значение PATH.

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
- Идемпотентно - повторный запуск с теми же флагами ничего не делает.

## Место в рабочем процессе

`mcp install` записывает конфигурацию, но не доказывает успешный старт или вызов сервера клиентом. Создаёт `args: ["serve"]` и сохраняет существующий `env`. После рестарта проверьте инициализацию MCP и каталог инструментов нужного бинарника, профиля и workspace. Отдельный CLI-вызов `forgeplan health` не проверяет это MCP-соединение.

## См. также

- [`forgeplan mcp`](/ru/docs/cli/mcp/) - родительская команда
- [`forgeplan mcp serve`](/ru/docs/cli/mcp-serve/) - запустить сервер (алиас)
- [`forgeplan serve`](/ru/docs/cli/serve/) - справочник по нижележащему серверу
- [Индекс MCP-инструментов](/ru/docs/mcp/) - что предоставляет сервер после установки
- [`forgeplan health`](/ru/docs/cli/health/) - проверка после рестарта клиента

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan mcp install --help`.

```text
Install forgeplan MCP config into a client (Claude / Cursor / Windsurf).

Smart-merge: replaces command/args/transport, preserves existing `env`. Idempotent — safe to re-run.

Usage: forgeplan mcp install [OPTIONS] --client <CLIENT>

Options:
  -c, --client <CLIENT>
          Target client: claude, cursor, or windsurf

  -s, --scope <SCOPE>
          Config scope: user (global) or project (local). Default: user
          
          [default: user]

      --binary-path <BINARY_PATH>
          Override binary path (default: detected from current_exe)

      --use-name <USE_NAME>
          Use short name instead of absolute path: forgeplan or fpl. Requires PATH to include the binary's directory at MCP launch time. macOS GUI apps may not inherit shell PATH — use absolute path (the default) for maximum portability

      --dry-run
          Print proposed change without writing

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
<!-- forgeplan-runtime:end -->
