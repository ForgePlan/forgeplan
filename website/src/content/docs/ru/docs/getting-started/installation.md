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

### Что даёт каждый плагин

| Плагин | Что ты сможешь сделать |
|---|---|
| `fpl-skills` | Набираешь `/smith` — он сам понимает какая у тебя задача и какая методология подходит. Драйвит каждодневные команды: `/forge-cycle` чтобы провести задачу до конца, `/audit` для multi-expert code review, `/sprint` для волновой реализации, `/forgeplan-cookbook` чтобы быстро найти нужный forgeplan-инструмент. Мозги системы. |
| `agents-pro` | Запускаешь именованных специалистов когда нужно: `brief-intake` превращает мутную идею в структурированный Brief, `adr-architect` пишет архитектурное решение с тремя обдуманными альтернативами, `research-analyst` собирает prior art до того как ты прыгнешь в реализацию, `guardian` делает последнюю проверку перед активацией артефакта. |
| `agents-sparc` | Используешь SPARC пятифазный поток для любой новой фичи: Specification → Pseudocode → Architecture → Refinement → Completion. Без него первый PRD на новом проекте ляжет без SPARC-структуры — придётся переделывать spec-фазу руками. |
| `agents-core` | Реально пишешь, ревьюишь и тестируешь код. Агент `coder` правит файлы в изолированном worktree, `code-reviewer` выдаёт структурированные findings против спеки, `tester` гоняет suite и считает coverage delta. |
| `forgeplan-workflow` | Запускаешь команду `/forge-cycle` — reactive-enforcer который проводит артефакт через validate → ADI → review → activate шаг за шагом. Плюс `/forge-audit` для multi-expert audit'а и guardian gate, который решает готов ли PRD к merge. |

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
