---
depth: tactical
id: NOTE-027
kind: note
links:
- target: EPIC-002
  relation: informs
status: draft
title: 'Idea: Ruflo/Gastown Integration — MCP server + custom agents + hooks'
---

## Ruflo/Gastown Integration

### Ruflo (4 способа)
1. **MCP-сервер** — нативная поддержка, forgeplan serve → .agents/config.toml
2. **Custom Agent YAML** — architecture-guardian с forgeplan tools
3. **27 hooks** — PreToolUse/PostToolUse для validate/drift
4. **Intelligence Loop** — RETRIEVE фаза читает forgeplan context

### Gastown (workaround)
- Directives (markdown-инъекция в контекст агента)
- Mail-система (инструкции агентам)
- Нет MCP, нет plugin API — loose coupling only

### Вердикт
Ruflo: 30 минут на интеграцию (MCP + hooks). Gastown: 2-3 часа, хрупко.

