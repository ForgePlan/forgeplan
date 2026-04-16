---
title: Настройка MCP — установка одной командой
description: Подключите Forgeplan как MCP-сервер к Claude Code, Cursor или Windsurf за 30 секунд.
---

После `brew install forgeplan` подключение к AI-агенту — это **одна команда**.
Никакого редактирования JSON, никакого копирования. Smart-merge сохраняет
ваш существующий конфиг.

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

Перезапустите клиента. Готово — все 47 `forgeplan_*` MCP-инструментов
доступны агенту.

## Что делает команда

Команда записывает запись `forgeplan` в MCP-конфиг вашего клиента:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "/opt/homebrew/bin/forgeplan",
      "args": ["serve"],
      "transport": "stdio"
    }
  }
}
```

Это **smart-merge**:
- Заменяет `command` / `args` / `transport` (чтобы `forgeplan upgrade` работал чисто)
- **Сохраняет ваш `env`** (API-ключи, `RUST_LOG`, кастомные пути)
- Не трогает другие MCP-серверы в файле
- Идемпотентна — безопасно перезапускать

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
надёжный вариант — работает в любом клиенте, включая GUI-приложения macOS,
которые не наследуют ваш shell `$PATH`.

Если хотите использовать короткое имя (и уверены что клиент запускается
с настроенным `$PATH`):

```bash
forgeplan mcp install --client claude --use-name fpl       # запишет "fpl"
forgeplan mcp install --client claude --use-name forgeplan # запишет "forgeplan"
```

:::caution
**GUI-приложения macOS** (Claude Code Mac app, Cursor app) получают только
системный `$PATH` по умолчанию — `/opt/homebrew/bin` в нём **нет**. Короткие
имена молча не сработают в этих клиентах. Используйте абсолютный путь
(default), если не настроили `launchctl setenv PATH ...` системно.
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
#    (Claude Code, Cursor, Windsurf — полностью закрыть и открыть)

# 2. В вашей директории проекта инициализируйте workspace
forgeplan init -y

# 3. Проверьте что MCP подключен
#    Спросите агента: "use forgeplan_health to check the project"
```

Если агент вернёт "healthy project status" — MCP работает.

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
Error: refusing to write to symlink: ~/.claude.json — remove the symlink and re-run install
```

Целевой файл — symlink. Мы отказываемся следовать (security: предотвращает
атаки когда злоумышленник заранее создаёт symlink на чувствительный файл,
куда мы бы записали наш конфиг). Замените symlink обычным файлом или
удалите.

### Already up to date

```
✓ Claude Code MCP config already up to date: ~/.claude.json
```

Конфиг совпадает с тем что мы бы записали — менять нечего. Идемпотентность
работает как задумано.

### Workspace not initialized

После установки агент вызывает `forgeplan_*` tool и получает:

```
Workspace not initialized. Call forgeplan_init first.
```

Запустите `forgeplan init -y` в директории проекта, или попросите агента
вызвать `forgeplan_init` через MCP — он использует свою текущую директорию.

### Перезапуск после `brew upgrade`

`forgeplan mcp install` идемпотентна — перезапустите её после любого
обновления версии чтобы обновить конфиг. Детектированный путь к binary
автоматически подхватит новую версию.

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

Поле `transport: "stdio"` опционально (большинство клиентов используют
stdio по умолчанию).
