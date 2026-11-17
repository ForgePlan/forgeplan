---
title: Настройка MCP - установка одной командой
description: Зарегистрируйте Forgeplan как MCP-сервер для Claude Code, Cursor или Windsurf.
---

После `brew install forgeplan/tap/forgeplan` зарегистрируйте бинарник в
AI-клиенте одной командой. Редактировать JSON не нужно; smart-merge
сохраняет существующий конфиг.

## Быстрая установка

Выберите клиента:

```bash
# Claude Code (по умолчанию scope: глобальный ~/.claude.json)
forgeplan mcp install --client claude

# Cursor
forgeplan mcp install --client cursor

# Windsurf
forgeplan mcp install --client windsurf
```

Перезапустите клиента и разрешите сервер, если появится запрос. Опубликованный
stdio-сервер Forgeplan 0.38.0 предоставляет **73 инструмента**. Регистрация
не устанавливает навыки и агентов маркетплейса, не инициализирует workspace
и не настраивает провайдера LLM.

## Что делает команда

Команда записывает запись `forgeplan` в MCP-конфиг вашего клиента:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "/opt/homebrew/bin/forgeplan",
      "args": ["serve"]
    }
  }
}
```

Это **smart-merge**:
- Заменяет `command` / `args` и удаляет устаревшие поля `transport: "stdio"` и `type: "stdio"`
- **Сохраняет ваш `env`** (API-ключи, `RUST_LOG`, кастомные пути)
- Не трогает другие MCP-серверы в файле
- Идемпотентна - безопасно перезапускать

## Опции

### Scope: user vs project

```bash
forgeplan mcp install --client claude --scope user      # ~/.claude.json (default)
forgeplan mcp install --client claude --scope project   # ./.mcp.json (per-repo)
```

Project-scope установка позволяет каждому репозиторию использовать свой
forgeplan binary или env.

### Короткое имя (`forgeplan` или `fpl`)

По умолчанию команда записывает **абсолютный путь** к binary. Это самый
надёжный вариант - работает в любом клиенте, включая GUI-приложения macOS,
которые не наследуют ваш shell `$PATH`.

Если хотите использовать короткое имя (и уверены что клиент запускается
с настроенным `$PATH`):

```bash
forgeplan mcp install --client claude --use-name fpl       # запишет "fpl"
forgeplan mcp install --client claude --use-name forgeplan # запишет "forgeplan"
```

:::caution
**GUI-приложения macOS** могут не наследовать `$PATH` вашей оболочки. Из-за
этого короткое имя может не запустить сервер. Оставьте абсолютный путь по
умолчанию, если не проверили окружение запуска клиента.
:::

### Кастомный binary

```bash
forgeplan mcp install --client claude --binary-path /custom/path/forgeplan
```

Путь валидируется: должен быть абсолютным, существовать, быть обычным
файлом и исполняемым. Пустые строки, относительные пути, управляющие
символы и bidi-override кодпоинты отклоняются.

### Dry-run

Посмотреть что изменится без записи:

```bash
forgeplan mcp install --client claude --dry-run
```

Вывод показывает построчный diff предлагаемых изменений.

## После установки

```bash
# 1. Перезапустите AI-клиента чтобы загрузил новый конфиг
#    (Claude Code, Cursor, Windsurf - полностью закрыть и открыть)

# 2. В вашей директории проекта инициализируйте workspace
forgeplan init -y

# 3. Проверьте что MCP подключен
#    Спросите агента: "use forgeplan_health to check the project"
```

Успешный ответ `forgeplan_health` подтверждает, что вызов дошёл до сервера;
при этом вердикт может сообщать о проблемах проекта. Проверьте, что ответ
описывает нужный проект, а не только содержит `healthy`.

### Выбор рабочего пространства

`forgeplan serve` стартует в каталоге запуска, заданном клиентом. Регистрация
MCP с project scope выбирает файл конфигурации, а не привязывает workspace.
Запускайте клиента из нужного проекта и инициализируйте это пространство.
Некоторые инструменты 0.38.0 принимают явный аргумент `workspace` для
worktree, но `forgeplan_health` и `forgeplan_init` его не принимают.
Смена каталога в оболочке агента не меняет каталог уже работающего MCP-сервера.

Модель и LLM настраиваются отдельно. Чтение артефактов, валидация и health
не требуют LLM, а `forgeplan_reason` требует. Семантическому поиску нужны
модель и векторы артефактов. См. [Установка](/ru/docs/getting-started/installation/)
и [Конфигурация](/ru/docs/getting-started/configuration/).

## Пути конфигов по клиентам

| Клиент | User scope | Project scope |
|--------|------------|---------------|
| Claude Code | `~/.claude.json` | `./.mcp.json` |
| Cursor | `~/.cursor/mcp.json` | `./.cursor/mcp.json` |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` | не поддерживается |

Windows использует `%USERPROFILE%` вместо `~`.

## Troubleshooting

### Symlink отклонён

```
Error: refusing to write to symlink: ~/.claude.json - remove the symlink and re-run install
```

Целевой файл - symlink. Мы отказываемся следовать (security: предотвращает
атаки когда злоумышленник заранее создаёт symlink на чувствительный файл,
куда мы бы записали наш конфиг). Замените symlink обычным файлом или
удалите.

### Already up to date

```
✓ Claude Code MCP config already up to date: ~/.claude.json
```

Конфиг совпадает с тем что мы бы записали - менять нечего. Идемпотентность
работает как задумано.

### Workspace not initialized

После установки агент вызывает `forgeplan_*` tool и получает:

```
Workspace not initialized. Call forgeplan_init first.
```

Выполните `forgeplan init -y` в нужном проекте и убедитесь, что MCP-сервер
запущен для него. `forgeplan_init` инициализирует пространство относительно
каталога запуска сервера, а не произвольного каталога в оболочке агента.

### Перезапуск после `brew upgrade`

Повторите `forgeplan mcp install`, если расположение бинарника изменилось или
конфиг ещё ссылается на версионный каталог Cellar. Определение пути предпочитает
стабильную запись в PATH, если она ведёт к запущенному бинарнику.
После обновления перезапустите клиента, чтобы заменить уже работающий сервер.

## Ручная настройка (если хотите вручную)

Если предпочитаете редактировать JSON сами, вот минимальная запись:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "/opt/homebrew/bin/forgeplan",
      "args": ["serve"]
    }
  }
}
```

Не добавляйте `transport: "stdio"`. Установщик 0.38.0 не записывает это поле
и удаляет такое устаревшее значение из существующих записей; `serve` по
умолчанию использует stdio.
