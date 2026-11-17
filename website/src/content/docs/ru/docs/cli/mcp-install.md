---
title: forgeplan mcp install
description: "Smart-merge Forgeplan into Claude Code, Cursor, or Windsurf MCP config - cross-platform, idempotent, brew-upgrade-safe."
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

- Для настройки отдельных инструментов — команда пишет запись запуска, не настройки пространства. Сервер ищет `.forgeplan/` от рабочего каталога запуска.
- Для HTTP / сетевого MCP - Forgeplan поставляется только со stdio.
- Чтобы удалить Forgeplan из клиента - отредактируйте файл конфигурации вручную;
  `install` только добавляет.

## Использование

```text
forgeplan mcp install [OPTIONS] --client <CLIENT>
```

## Опции

```text
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

`--binary-path` и `--use-name` взаимоисключающи. Автоопределение предпочитает путь из PATH, только если он разрешается в запущенный бинарник; иначе использует `current_exe`. Стабильный путь Homebrew вроде `/opt/homebrew/bin/forgeplan` предпочтителен, но не гарантирован. Изучите выбранный путь после установки.

## Примеры

### Пример 1: Claude Code, на пользователя

```bash
forgeplan mcp install --client claude
```

Записывает `~/.claude.json`. Scope по умолчанию - `user`, поэтому каждый проект,
который открывает Claude Code, видит инструменты Forgeplan.

### Пример 2: Cursor, только проект

```bash
forgeplan mcp install --client cursor --scope project
```

Записывает `./.cursor/mcp.json`. Forgeplan загружается только когда репозиторий
выбран активным рабочим пространством - полезно, когда лишь часть
проектов в монорепозитории использует Forgeplan.

### Пример 3: Превью перед записью

```bash
forgeplan mcp install --client windsurf --dry-run
```

Печатает подписанный построчный diff текущего и предлагаемого JSON, не записывая конфиг и не создавая его каталоги. Это не самостоятельный документ объединённого JSON. Для применения повторите те же client/scope/path без `--dry-run`: общая подсказка `forgeplan mcp install` не содержит обязательного флага клиента.

### Пример 4: Использовать короткое имя вместо абсолютного пути

```bash
forgeplan mcp install --client cursor --use-name forgeplan
```

Записывает `"command": "forgeplan"` - рассчитывает на `$PATH` в момент запуска MCP.
**Оговорка для GUI-клиентов macOS**: Claude Code Mac и Cursor app **не** наследуют
shell PATH, поэтому короткие имена ломаются, если вы не настроили
`launchctl setenv PATH ...`. Дефолт (абсолютный путь) - более безопасный выбор.

## Записываемые файлы конфигурации

| Клиент | User scope | Project scope |
|---|---|---|
| `claude` | `~/.claude.json` | `./.mcp.json` |
| `cursor` | `~/.cursor/mcp.json` | `./.cursor/mcp.json` |
| `windsurf` | `~/.codeium/windsurf/mcp_config.json` | _не поддерживается_ |

У Windsurf нет per-project конфигурации; передавайте `--scope user` (это значение
по умолчанию).

## Поведение умного слияния

- Заменяет `command` и `args` (`["serve"]`). Удаляет `transport` и `type`, только если значение равно `"stdio"`; явные HTTP/SSE и прочие неизвестные поля сохраняются.
- **Сохраняет** существующий блок `env` для записи (project-specific API-ключи и т. д.).
- Оставляет все остальные серверы в `mcpServers` нетронутыми.
- Идемпотентно - повторный запуск с теми же флагами ничего не делает.
- Отклоняет некорректный JSON, root или `mcpServers` не объектного типа и конфигурацию по симлинку, а не перезаписывает их молча. Явный `--binary-path` должен быть абсолютным путём к существующему исполняемому обычному файлу.

## Место в рабочем процессе

`mcp install` - мост между «бинарник на диске» и «агент может вызывать инструменты
Forgeplan». После успеха перезапустите клиента и проверьте, что он обнаружил инструменты Forgeplan в нужном проекте. Отдельный CLI-вызов `forgeplan health` проверяет пространство, а не успешность запуска сервера клиентом. Для самой установки записи LLM/модель не нужны; требования отдельных инструментов различаются.

## См. также

- [`forgeplan mcp`](/ru/docs/cli/mcp/) - родительская команда
- [`forgeplan mcp serve`](/ru/docs/cli/mcp-serve/) - запустить сервер (алиас)
- [`forgeplan serve`](/ru/docs/cli/serve/) - справочник по нижележащему серверу
- [Индекс MCP-инструментов](/ru/docs/mcp/) - что предоставляет сервер после установки
- [`forgeplan health`](/ru/docs/cli/health/) - проверка после рестарта клиента
