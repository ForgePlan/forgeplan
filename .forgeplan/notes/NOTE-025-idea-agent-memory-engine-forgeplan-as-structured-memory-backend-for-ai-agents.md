---
depth: tactical
id: NOTE-025
kind: note
links:
- target: EPIC-002
  relation: informs
status: draft
title: 'Idea: Agent Memory Engine — forgeplan as structured memory backend for AI agents'
---

## Agent Memory Engine

**R_eff: HIGH** — самое перспективное направление.

### Тезис
Forgeplan становится structured memory layer для Claude Code, Ruflo, Gastown и других AI-агентов.

### Evidence (supports)
- forgeplan serve уже MCP-сервер (тест 10.9 PASS)
- --json на всех командах — агент может парсить
- context --json даёт полный reasoning context одним вызовом
- remember/recall — уже есть memory primitives
- score + validate — агент может оценивать качество своих решений

### Evidence (weakens)
- capture требует LLM — circular dependency
- Нет SDK/библиотеки — только CLI (overhead на subprocess calls)
- Нет event stream — агент не может подписаться на изменения

### Concrete Steps
1. Протестировать forgeplan serve как MCP-сервер в Claude Code
2. Создать Claude Code plugin: /fp-validate, /fp-context, /fp-score
3. Capture offline mode (без LLM — просто создать Note/ADR)
4. forgeplan watch --emit-events — JSON-stream для агентов

### Tier: 2 (средние усилия, трансформационный эффект)


