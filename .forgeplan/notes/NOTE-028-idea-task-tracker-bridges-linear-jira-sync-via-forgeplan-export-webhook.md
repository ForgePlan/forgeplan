---
depth: tactical
id: NOTE-028
kind: note
links:
- target: EPIC-002
  relation: informs
status: draft
title: 'Idea: Task Tracker Bridges — Linear/Jira sync via forgeplan export + webhook'
---

## Task Tracker Bridges

### Сценарии интеграции (из E2E обсуждения)

**Сценарий 1: Jira + Confluence + Forgeplan**
- Confluence → документы (вики, RFC-текст)
- Jira → задачи (спринты, эпики, баги)  
- Forgeplan → решения (граф, scoring, validation, evidence)
- Хороший сетап: Jira для daily work, Forgeplan для архитектурных решений
- Плохой: синкать каждый Jira ticket в forgeplan (разные уровни абстракции)

**Сценарий 2: Notion + Telegram + Forgeplan**
- Проблема: решения в Telegram теряются, Notion — плоские заметки
- Forgeplan: ADR + Evidence + Score + связь с PRD
- Хороший: Telegram для обсуждений, Forgeplan для решений, Notion для wiki

**Сценарий 3: С нуля (Linear + Forgeplan + Ruflo)**
- Linear → задачи, спринты (бесплатный tier)
- Forgeplan → решения, architecture
- Claude Code + Ruflo → AI-исполнение
- Идеальный AI-first стек

### Конкретные мосты

**Мост 1: Forgeplan → Task Tracker**
- forgeplan sync linear --project PET --map epic=prd,task=rfc
- Или webhook: forgeplan watch --on-activate 'curl linear-api/create-epic'
- MVP: shell-скрипт (forgeplan list --json | jq → linear-cli)

**Мост 2: Task Tracker → Forgeplan**
- При создании Epic в Linear → forgeplan new prd 'PROJ-123: Title'
- При закрытии спринта → forgeplan new evidence 'Sprint 14 results'

### Effort
- MVP sync скрипт: 1-2 дня
- Webhook integration: 1 неделя
- Bidirectional real-time: 2-3 недели
