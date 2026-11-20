---
title: Настройка MCP - установка одной командой
description: Подключите Forgeplan как MCP-сервер к Claude Code, Cursor или Windsurf за 30 секунд.
---

После `brew install forgeplan` подключение к AI-агенту - это **одна команда**.
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

Перезапустите клиент и проверьте список инструментов. Интерфейс зависит от
версии и профиля; см. [актуальный обзор MCP](/ru/docs/mcp/).

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
- Обновляет `command` / `args` при повторном запуске после обновления Forgeplan
- **Сохраняет ваш `env`** (API-ключи, `RUST_LOG`, кастомные пути)
- Не трогает другие MCP-серверы в файле
- Идемпотентна для генерируемых defaults, но собственные `args` всё равно заменяются. Перед переподключением настроенного Constellation-сервера проверьте `--profile`, `--workspace` и `--auto-index`.

Captured `mcp install --help` всё ещё говорит «command/args/transport»;
текущий writer не пишет `transport` и удаляет старые `transport: "stdio"`
и `type: "stdio"`. Прочие неизвестные поля сохраняются. Проверьте собственные
HTTP/SSE-настройки, не считайте их автоматически корректным stdio config.

### Профили, workspace и обслуживание

В локальной `0.39.0+constellation.s1.1` (не опубликованной 0.38.0) `serve`
по умолчанию использует `--profile full`, включая запись.
`--profile constellation-read` - отдельный registry/handle интерфейс чтения,
не полное пространство имён с отключённой записью. `forgeplan mcp serve`
и отдельный `forgeplan-mcp` принимают `--auto-index` и `--workspace`,
но не `--profile`.

Для явной full-profile цели используйте такой пример списка аргументов
в записи сервера клиента:

```json
["serve", "--profile", "full", "--workspace", "/path/to/project", "--auto-index"]
```

`--workspace` задаёт default сервера. На старте full-сервера цель обслуживания:
`FORGEPLAN_WORKSPACE` → default сервера → поиск от CWD. Задайте тот же проект
в окружении клиента либо явно `FORGEPLAN_WORKSPACE=""`: при `unset` CLI может
восстановить значение из родительского `.env`.
Явный `workspace` инструмента приоритетен для конкретного запроса; далее идут
окружение → default сервера → CWD. Другой запрос не перемещает обслуживание.
CWD агента не становится CWD сервера автоматически.

Одного `--auto-index` недостаточно для эмбеддингов. Auto-drain нужны явный
`embedding:` config, сборка с semantic-search, BGE-M3 и полный снапшот в кэше.
См. [настройку Constellation](/ru/docs/guides/constellation/) и
[контракты чтения](/ru/docs/reference/constellation/).

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
GUI-приложения macOS могут не наследовать shell `$PATH`: короткое имя
бинарника не найдётся, если его каталога там нет. Предпочитайте
сгенерированный абсолютный путь; перед использованием короткого имени
проверьте фактическое окружение клиента.
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

Успешный ответ `forgeplan_health` от нужного workspace показывает, что
этот вызов дошёл до сервера. Сообщение о проблемах health тоже может быть
корректным MCP-ответом; один вызов не проверяет все tools или semantic readiness.

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

Инициализируйте из корня нужного проекта и проверьте выведенный путь.
`init -y` может найти workspace у родителя и завершиться без изменений.
Для существующего workspace см. [конфигурацию](/ru/docs/getting-started/configuration/).
Передавайте явный `workspace` в инструментах, которые его принимают:
каталог агента не определяет автоматически каталог сервера.

### Перезапуск после `brew upgrade`

`forgeplan mcp install` идемпотентна - перезапустите её после любого
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

В этой stdio-записи не указывайте `transport`: генератор его не пишет.
