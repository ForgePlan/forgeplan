---
title: Руководство по настройке Claude Code
description: Настройте Claude Code для максимальной продуктивности с Forgeplan
---

## Что такое CLAUDE.md?

`CLAUDE.md` - это память проекта Claude Code, файл в корневом каталоге вашего репозитория, который сообщает Claude о вашем проекте, соглашениях и рабочих процессах. Claude читает его при каждом запуске сессии.

## Рекомендуемая структура CLAUDE.md

На основе производственных конфигураций для множества проектов:

```markdown
# CLAUDE.md

## Быстрый старт
- Настройка среды разработки
- Ключевые команды
- Где что найти

## Методология (Forgeplan)
- Route → Shape → Validate → Code → Evidence → Activate
- Таблица калибровки глубины
- Поток создания артефактов

## Рабочий процесс Git
- Стратегия ветвления (main ← dev ← feat/*)
- Формат коммитов (conventional commits + Refs)
- Конвейер PR: Code → Audit → Fix → Test → PR

## Хуки принудительного исполнения
- forge-safety-hook.sh - блокирует опасные команды
- pre-commit-fmt.sh - проверка форматирования
- commit-test-check.sh - тесты для новых функций

## Память (Hindsight)
- Начало сессии: memory_recall("project")
- После решений: memory_retain("what we decided")
- Анализ: memory_reflect("what patterns")

## Жесткие требования
- Правила для конкретного языка
- Архитектурные ограничения
- Стандарты тестирования
```

## Раздел Forgeplan

Добавьте это в CLAUDE.md любого проекта для интеграции Forgeplan:

```markdown
## Forgeplan

### Начало сессии
forgeplan health   # слепые пятна, сироты (артефакт без связей) - исправить В ПЕРВУЮ ОЧЕРЕДЬ

### Перед любой задачей
forgeplan route "task description"   # определяет глубину

### Полный цикл (Standard+)
1. forgeplan new prd "Title"         # создать артефакт
2. Заполнить ОБЯЗАТЕЛЬНЫЕ разделы    # Problem, Goals, FR
3. forgeplan validate PRD-XXX        # гейты качества
4. forgeplan reason PRD-XXX          # ADI: 3+ гипотезы
5. Код + тест каждой публичной функции
6. forgeplan new evidence "..."      # создать доказательство
7. forgeplan link EVID-XXX PRD-XXX   # соединить
8. forgeplan score PRD-XXX           # R_eff > 0
9. forgeplan activate PRD-XXX        # черновик → активный

### Тактическая глубина
Просто код. Артефакты не требуются.
```

Пример описывает рабочий процесс, а не проверку каждого инструмента на всех
раскладках workspace. Проверенное покрытие совместимости и оставшиеся
ограничения описаны в [гайде Constellation](/ru/docs/guides/constellation/).

## Хуки принудительного исполнения

Проектные хуки могут автоматизировать проверки качества. Имена ниже —
примеры, не скрипты, устанавливаемые `forgeplan setup-skill`: подготовьте
и проверьте каждый исполняемый скрипт перед включением:

```bash
# .claude/hooks/forge-safety-hook.sh
# Блокирует: git push --force, rm -rf /, cargo publish, DROP TABLE

# .claude/hooks/pre-commit-fmt.sh  
# Блокирует коммит, если код не отформатирован

# .claude/hooks/commit-test-check.sh
# Предупреждает, если новая публичная функция не имеет теста
```

### Настройка хуков

Пример `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/forge-safety-hook.sh"
          }
        ]
      }
    ]
  }
}
```

Использована [схема command-hook Claude Code](https://code.claude.com/docs/en/hooks).
Скрипт должен разбирать вход инструмента и возвращать описанный блокирующий
результат: само имя файла не обеспечивает выполнение правила.

## Конфигурация MCP сервера

Добавьте Forgeplan в качестве MCP сервера для AI агентов:

```json
// .mcp.json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"]
    }
  }
}
```

Запускает full MCP профиль по умолчанию, включая запись. Имена и схемы
зависят от версии; см. [обзор MCP](/ru/docs/mcp/) и `tools/list` сервера.
Для нескольких workspace передавайте `serve --workspace /path/to/project`;
`workspace` запроса приоритетнее `FORGEPLAN_WORKSPACE`, default сервера и
поиска от CWD, но не меняет цель обслуживания.
[Настройка MCP](/ru/docs/guides/mcp-setup/) описывает отдельный
`constellation-read` профиль и условия auto-index.

## Интеграция памяти (Hindsight)

Сохраняйте знания между сессиями:

| Когда | Инструмент | Пример |
|------|------|---------|
| Начало сессии | `memory_recall` | "Что мы решили по поводу аутентификации?" |
| После решения | `memory_retain` | "Выбрали JWT вместо сессий, потому что..." |
| Анализ | `memory_reflect` | "Какие паттерны здесь работают лучше всего?" |

## Рекомендуемые разрешения

Пример `.claude/settings.json`:

```json
{
  "permissions": {
    "allow": [
      "Bash(cargo:*)",
      "Bash(forgeplan:*)",
      "Bash(git add:*)",
      "Bash(git commit:*)",
      "Bash(git status:*)",
      "Bash(git diff:*)",
      "Bash(git log:*)",
      "Bash(git branch:*)",
      "Bash(git checkout:*)",
      "Bash(npm:*)",
      "Read",
      "Glob",
      "Grep"
    ],
    "deny": [
      "Bash(git push --force*)",
      "Bash(rm -rf /*)",
      "Bash(cargo publish*)"
    ]
  }
}
```

Это примеры [permission rules](https://code.claude.com/docs/en/permissions),
не полноценная shell-песочница. Каждой подкоманде Git нужно отдельное правило:
список через запятую внутри `Bash(...)` не задаёт альтернативы. Проверьте
широкие разрешения под свой проект; три deny-шаблона не дают исчерпывающей
защиты от иначе записанных shell-команд.

## Настройка нескольких проектов

Для монорепозитория или настройки с несколькими проектами каждая поддиректория может иметь свой собственный CLAUDE.md:

```
project/
├── CLAUDE.md          ← корневая конфигурация (git, методология)
├── packages/
│   ├── core/
│   │   └── CLAUDE.md  ← правила для конкретного пакета
│   └── web/
│       └── CLAUDE.md  ← правила для фронтенда
└── .claude/
    ├── hooks/         ← общие хуки
    └── settings.json  ← разрешения
```

## Лучшие практики

1.  **Держите CLAUDE.md менее 500 строк** - Claude читает его каждую сессию. Слишком длинный = потерянный контекст.
2.  **Размещайте детали в docs/, а не в CLAUDE.md** - ссылайтесь на `docs/guides/X.md` для глубокого контента.
3.  **Обновляйте после решений** - новая конвенция? Немедленно добавьте ее в CLAUDE.md.
4.  **Хуки вместо инструкций** - "никогда не принудительно пушить" в CLAUDE.md - это предложение. Хук - это принуждение.
5.  **Здоровье Forgeplan в первую очередь** - всегда начинайте сессию с `forgeplan health`, чтобы выявить слепые пятна.