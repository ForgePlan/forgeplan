[English](AGENT-HOOKS.md) · [Русский](AGENT-HOOKS.ru.md)

# Хуки агентов — Автоматическая интеграция Forgeplan с AI-агентами

Forgeplan может автоматически предоставлять контекст проекта AI-агентам через хуки.
Это руководство описывает интеграцию с Claude Code, но те же принципы применимы к любому агентному фреймворку.

## Хук UserPromptSubmit (Claude Code)

Добавьте в `.claude/settings.json`:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {"type": "command", "command": "forgeplan health --compact"}
        ]
      }
    ]
  }
}
```

Это запускает `forgeplan health` в начале каждого промпта, автоматически предоставляя агенту контекст проекта.

### Что видит агент

`forgeplan health --compact` выдаёт компактный текст. Для структурированных
данных используйте `forgeplan health --json` или MCP `forgeplan_health`.

`--json` приоритетнее `--compact`: вместе они возвращают полный JSON dashboard,
а не пакет с лимитом 500 токенов. Среди полей — `total`, `by_status`,
массивы `blind_spots`/`at_risk`, `stale_count` и CLI hint `_next_action`.
Размер зависит от данных workspace; смотрите реальный ответ. Хук даёт
справочный контекст, но не доказывает прохождение validation или release gates.

## Хук PostToolUse

Напоминает агенту фиксировать решения после значительных изменений файлов:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {"type": "command", "command": "echo '[Forgeplan] Consider: does this change represent a decision worth capturing? Use forgeplan capture if so.'"}
        ]
      }
    ]
  }
}
```

Это намеренно легковесное решение — текстовое напоминание, а не автоматическое действие.

## Хук Route-Before-Work

Автоматическое определение глубины перед началом работы над задачей:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {"type": "command", "command": "forgeplan route \"$(jq -r '.prompt // empty')\" --level 0"}
        ]
      }
    ]
  }
}
```

Этот пример запускается на каждом промпте. У `UserPromptSubmit` нет фильтрации
слов промпта через `matcher`; текст приходит в `.prompt` JSON stdin, а не в
переменной `$PROMPT`. Нужен `jq`. Это примеры добровольной настройки, не установка
и не изменение native settings этого репозитория.

Агент получает текстовые CLI-рекомендации, не JSON. У `route` нет `--json`;
`--level 0` явно исключает LLM routing. Не скрывайте stderr и exit status,
чтобы отказ хука не выглядел успешной рекомендацией. Для структурированного
ответа используйте контракт MCP `forgeplan_route`.

## MCP-сервер (рекомендуется)

Для более глубокой интеграции запустите Forgeplan как MCP-сервер:

```bash
forgeplan serve
```

В scoped-сборке `0.39.0+constellation.s1.1` зарегистрированы 76 full tools
и отдельные 9 tools профиля `constellation-read`. Профиль `full` с записью остаётся
профилем по умолчанию; регистрация сама по себе не доказывает проверку всех tools.
CLI `validate` работает на legacy и migrated Constellation layout; full MCP
`forgeplan_validate` также успешно проверен на migrated артефактах.
Границы проверки см. в [оговорках пилота](CONSTELLATION-PILOT.ru.md).

### Конфигурация MCP для Claude Code

Добавьте в `.mcp.json` проекта (подставьте пути executable и workspace):

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve", "--workspace", "/path/to/your/project"]
    }
  }
}
```

Для пилота Constellation задайте явный версионированный executable и
`serve --profile full --auto-index --workspace <project-root>`. Startup
maintenance scope предпочитает `FORGEPLAN_WORKSPACE` флагу; request selection
предпочитает явный workspace запроса server defaults. Для pin проекта проверьте
или снимите env override. Для automatic embedding нужны также активный `config.embedding`,
полный snapshot BGE-M3 в кэше и сборка с semantic-search. См.
[переносимый runbook](CONSTELLATION-PILOT.ru.md); read-only server не является writer.

## Хук безопасности (Forge Mode)

Если хук установлен и включён в Claude Code, он может отклонить совпавшие
буквальные строки Bash-команд до исполнения:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/forge-safety-hook.sh",
            "timeout": 3
          }
        ]
      }
    ]
  }
}
```

`forge-safety-hook.sh` проверяет чёрный список:
- `git push --force` / `git push -f`
- `git reset --hard`
- `rm -rf /` / `rm -rf ~`
- `cargo publish`
- `DROP TABLE`

При буквальном совпадении чёрного списка в вызове `Bash` скрипт возвращает 2
и `BLOCKED by forge-safety-hook`. Это не shell parser и не sandbox: другие
tools и эквивалентные написания команд не покрыты гарантией.
Запреты репозитория действуют и тогда, когда хук не распознал команду.

### Три зоны доверия (локальная учебная аналогия, не алгоритм FPF B.3)

| Зона | Управляется | Механизм |
|------|-------------|----------|
| **Green** (безопасно) | `settings.local.json` allow | Wildcard whitelist: `Bash(cargo:*)` |
| **Yellow** (обратимо) | Claude Code acceptEdits | Файловые операции авто-разрешены |
| **Red** (необратимо) | `forge-safety-hook.sh` | PreToolUse blacklist блокирует |

## Хук методологии (активация навыков)

Напоминает о доступных методологических командах:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/skill-activation-hook.sh"
          }
        ]
      }
    ]
  }
}
```

Выводит: доступные навыки (/forge, /fpf-simple), правила методологии (Shape → Validate → Code → Evidence → Activate), напоминание о Rust-навыках.

## Лучшие практики

1. **Начинайте с health** — `UserPromptSubmit` вызывается на каждом промпте; `SessionStart` нужен именно для начала сессии
2. **У MCP и хуков разные задачи** — MCP предоставляет tools, эти command hooks печатают контекст и напоминания
3. **Не скрывайте отказы** — `2>/dev/null || true` подавляет диагностику и статус ошибки; при осознанном использовании для необязательного контекста это не PASS гейта
4. **Не злоупотребляйте автоматизацией** — хук PostToolUse должен предлагать, а не принуждать к созданию артефактов
5. **Маршрутизация перед работой** — помогает агенту решить, нужно ли создавать артефакты или просто писать код
