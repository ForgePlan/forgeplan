---
title: Установка
description: Установите Forgeplan — CLI, AI Skill или MCP Server
---

## AI Skill (рекомендуется для AI-агентов)

Установите навык `/forge` для Claude Code, Cursor, Codex, Gemini и более чем 40 AI-агентов:

```bash
npx skills add ForgePlan/marketplace --skill forge
```

После установки используйте в чате:
```
/forge "Add OAuth2 authentication"
```

**Альтернатива**: если у вас уже установлен CLI, используйте вместо этого встроенную команду — она встраивает файл навыка напрямую, без необходимости подключения к сети:

```bash
forgeplan setup-skill
```

Подробности см. в [`forgeplan setup-skill`](/docs/cli/setup-skill/).

**Откройте для себя больше плагинов**: [Обзор Marketplace](/docs/marketplace/overview/).

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
