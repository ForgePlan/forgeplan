---
depth: standard
id: ADR-001
kind: adr
links:
- target: EPIC-002
  relation: refines
- target: NOTE-009
  relation: supersedes
status: active
title: No adapter traits — AI agent is the orchestrator, not Forgeplan
---

# ADR-004: No Adapter Traits — AI Agent is the Orchestrator

## Status
Accepted

## Context
При проектировании v2.0 (EPIC-002) рассматривалось 5 подходов к интеграции с внешними системами (PROB-009). Первоначально предложена Adapter Layer с Rust traits:
- MemoryProvider (Hindsight, custom)
- CodeIndex (tree-sitter, grep)
- TaskTracker (Orchestra, Linear)
- LLMProvider (Gemini, Claude API)
- NotificationSink

Это зафиксировано в NOTE-009 "Architecture: Adapter Layer".

## Decision
**Отвергаем adapter traits.** Forgeplan НЕ интегрируется напрямую с внешними системами (кроме LLM).

Интеграция — ответственность AI agent (Claude Code), который оркестрирует вызовы к разным MCP servers:
- Agent вызывает Forgeplan MCP для артефактов
- Agent вызывает Hindsight MCP для памяти
- Agent вызывает Orchestra MCP для задач
- Agent сам собирает context и принимает решения

## Rationale
1. **FPF A.7 Strict Distinction**: Forgeplan = knowledge (method), Claude Code = orchestration (work). Не смешивать.
2. **Loose coupling**: Forgeplan не знает о существовании Hindsight или Orchestra. Это позволяет заменять любой tool без изменения Forgeplan.
3. **Zero-config**: Forgeplan работает полностью автономно без внешних зависимостей.
4. **Simplicity**: Нет abstract traits = нет mock'ов, нет DI complexity, нет leaky abstractions.

## Alternatives Considered
1. **Adapter traits** (NOTE-009) — отвергнуто: over-engineering, tight coupling
2. **MCP-to-MCP calls** — отвергнуто: Forgeplan MCP вызывает Hindsight MCP = circular dependency risk
3. **Plugin system** — отвергнуто: complexity для minimal gain

## Consequences
- **Positive**: Forgeplan core остаётся простым (~20K LOC). Единственная external dep = LLM provider.
- **Negative**: AI agent несёт больше ответственности за оркестрацию. Нужны хорошие skills/CLAUDE.md.
- **Neutral**: NOTE-009 (Adapter Layer) устарела и помечена как superseded.

## Supersedes
- NOTE-009 "Architecture: Adapter Layer — traits for Memory, CodeIndex, TaskTracker, LLM"

## Related
- EPIC-002 (Forgeplan v2.0 Vision)
- PROB-009 (Multi-Agent Architecture)
- NOTE-010 (v0.11 crate decisions)

## Affected Files
- crates/forgeplan-core/src/db/**
- crates/forgeplan-core/src/artifact/**
