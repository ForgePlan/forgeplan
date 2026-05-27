---
title: Установка
description: Установите Forgeplan - CLI, AI Skill или MCP Server
---

## AI Skill (рекомендуется для AI-агентов)

Установите навык `/forge` для Claude Code, Cursor, Codex, Gemini и более чем 40 AI-агентов:

```bash
forgeplan setup-skill   # пишет ~/.claude/skills/forge/SKILL.md, без сети
```

После установки используйте в чате:
```
/forge "Add OAuth2 authentication"
```

**Альтернатива**: если у вас уже установлен CLI, используйте вместо этого встроенную команду - она встраивает файл навыка напрямую, без необходимости подключения к сети:

```bash
forgeplan setup-skill
```

Подробности см. в [`forgeplan setup-skill`](/docs/cli/setup-skill/).

**Откройте для себя больше плагинов**: [Обзор Marketplace](/docs/marketplace/overview/).

## Полный harness-комплект (Claude Code)

Навыка `/forge` выше достаточно чтобы начать. Для **полной связки Forgeplan** — `/smith` master-оркестратор, `/forge-cycle` reactive enforcer, `/audit`, `/sprint`, `/methodology-check`, `/forgeplan-cookbook`, guardian + канонические агенты Profile A/B/C/D, FPF ADI reasoning, Hindsight cross-session memory — установите рекомендованный набор плагинов. Это то, что `/smith-bootstrap` Step 0a ожидает на новом проекте.

Запустите эти команды изнутри сессии Claude Code:

```bash
# Один раз — добавить marketplace
/plugin marketplace add ForgePlan/marketplace

# 5 MUST плагинов — полный pipeline зависит от всех пяти
/plugin install fpl-skills@ForgePlan-marketplace          # 34 skill'а: smith, forge-cycle, forgeplan-cookbook, audit, sprint, methodology-check, ...
/plugin install agents-pro@ForgePlan-marketplace          # 28 агентов: smith, guardian, brief-intake, adr-architect, research-analyst, ...
/plugin install agents-sparc@ForgePlan-marketplace        # 5 SPARC phase-агентов — первый PRD диспатчится в specification
/plugin install agents-core@ForgePlan-marketplace         # 11 базовых агентов: coder, code-reviewer, tester
/plugin install forgeplan-workflow@ForgePlan-marketplace  # /forge-cycle + /forge-audit + guardian gate enforcement

# 2 SHOULD плагина — настоятельно рекомендуются
/plugin install fpf@ForgePlan-marketplace                 # FPF ADI reasoning — обязателен для Standard+ артефактов
/plugin install fpl-hsmem@ForgePlan-marketplace           # Hindsight cross-session memory (per-project bank)

# Перезагрузить чтобы активировать
/reload-plugins
```

После reload `/smith-bootstrap` для нового репо или `/smith` для рекомендаций следующего действия готовы к работе.

### Зачем нужны все 5 MUST плагинов

| Плагин | Что даёт | Без него |
|---|---|---|
| `fpl-skills` | `/smith`, `/forge-cycle`, `/audit`, `/sprint`, `/forgeplan-cookbook`, всего 34 skill'а | Нет оркестратора, нет методологического routing'а |
| `agents-pro` | тело smith-агента, guardian, brief-intake, adr-architect, research-analyst (28 агентов) | Нет Profile A создателей, нет guardian gate |
| `agents-sparc` | specification, architecture, pseudocode, refinement, sparc-orchestrator | Первый PRD silently fallback'ит на generic Profile A, теряя SPARC контракт |
| `agents-core` | coder, code-reviewer, tester (11 агентов) | Нет Profile C-coder для реального кода, нет канонических ревьюеров |
| `forgeplan-workflow` | `/forge-cycle` (reactive enforcer), `/forge-audit`, guardian gate enforcement | Нет драйвера 4-слойного пайплайна, нет команды `/forge`, нет audit |

### Опциональные плагины

| Плагин | Когда добавлять |
|---|---|
| `laws-of-ux` | Frontend / UX code review по 30 Законам UX |
| `agents-domain` | Domain-специфичные агенты: TypeScript, Go, Python, Next.js, React, Rust и др. |
| `agents-github` | GitHub workflow агенты: PR, issues, releases, projects, workflows |
| `forgeplan-brownfield-pack` | Онбординг существующих кодовых баз через 7-фазный Discover-протокол |
| `forgeplan-orchestra` | Синхронизация с Orchestra task management |

См. [Обзор Marketplace](/docs/marketplace/overview/) для полного каталога плагинов.

## Бинарный файл CLI

### macOS (Homebrew)

```bash
brew install forgeplan/tap/forgeplan
```

### Из исходного кода (Rust)

```bash
cargo install forgeplan
```

### Релизы GitHub

Загрузите предварительно собранные бинарные файлы из [Релизов GitHub](https://github.com/ForgePlan/forgeplan/releases).

## MCP Server (для AI-агентов)

Добавьте в файл `.mcp.json` вашего проекта:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"],
      "env": {}
    }
  }
}
```

## Инициализация рабочего пространства

```bash
forgeplan init -y
```

Это создаст каталог `.forgeplan/` с конфигурацией и хранилищем LanceDB.

## Проверка установки

```bash
forgeplan --version
forgeplan health
```

:::note
AI-агенты всегда должны использовать `forgeplan init -y` (неинтерактивный режим).
:::
