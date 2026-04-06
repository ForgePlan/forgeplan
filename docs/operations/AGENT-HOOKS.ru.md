[English](AGENT-HOOKS.md) · [Русский](AGENT-HOOKS.ru.md)

# Хуки агентов — Автоматическая интеграция Forgeplan с AI-агентами

Forgeplan может автоматически предоставлять контекст проекта AI-агентам через хуки.
Это руководство описывает интеграцию с Claude Code, но те же принципы применимы к любому агентному фреймворку.

## Хук SessionStart (Claude Code)

Добавьте в `.claude/settings.json`:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "matcher": "",
        "command": "forgeplan health --compact --json 2>/dev/null || true"
      }
    ]
  }
}
```

Это запускает `forgeplan health` в начале каждого промпта, автоматически предоставляя агенту контекст проекта.

### Что видит агент

`forgeplan health --compact --json` возвращает компактный JSON-пакет (< 500 токенов):

```json
{
  "total": 27,
  "active": 11,
  "draft": 16,
  "blind_spots": 0,
  "at_risk": 2,
  "stale": 1,
  "next_action": "Review EVID-003 — evidence expires in 3 days"
}
```

Это сообщает агенту:
- Сколько артефактов существует и их распределение по статусам
- Есть ли слепые зоны (решения без доказательств)
- Какое следующее действие наиболее срочное

## Хук PostToolUse

Напоминает агенту фиксировать решения после значительных изменений файлов:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "command": "echo '[Forgeplan] Consider: does this change represent a decision worth capturing? Use forgeplan capture if so.'"
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
        "matcher": "implement|build|create|add feature",
        "command": "forgeplan route \"$PROMPT\" --json 2>/dev/null || true"
      }
    ]
  }
}
```

Агент получает рекомендации по маршрутизации:
```json
{
  "depth": "Standard",
  "pipeline": ["PRD", "RFC"],
  "confidence": 85
}
```

## MCP-сервер (рекомендуется)

Для более глубокой интеграции запустите Forgeplan как MCP-сервер:

```bash
forgeplan serve
```

Это предоставляет доступ к 26+ инструментам через MCP stdio transport, давая агенту полный CRUD-доступ к артефактам, валидации, оценке и поиску без необходимости CLI-хуков.

### Конфигурация MCP для Claude Code

Добавьте в `.claude/settings.json`:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"],
      "cwd": "/path/to/your/project"
    }
  }
}
```

## Хук безопасности (Forge Mode)

Блокирует опасные команды даже в yolo/acceptEdits режиме:

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

При обнаружении — exit 2, команда блокируется. Агент получает сообщение: "BLOCKED by forge-safety-hook".

### Три зоны доверия (FPF B.3)

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

1. **Начинайте с health** — хук SessionStart даёт агенту ситуационную осведомлённость
2. **MCP > хуки** — MCP предоставляет структурированный доступ к инструментам; хуки — только текст
3. **Хуки должны быть легковесными** — `2>/dev/null || true` предотвращает блокировку агента из-за сбоев хуков
4. **Не злоупотребляйте автоматизацией** — хук PostToolUse должен предлагать, а не принуждать к созданию артефактов
5. **Маршрутизация перед работой** — помогает агенту решить, нужно ли создавать артефакты или просто писать код
