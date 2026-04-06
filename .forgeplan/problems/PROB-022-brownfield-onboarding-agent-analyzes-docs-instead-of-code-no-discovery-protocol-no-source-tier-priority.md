---
depth: tactical
id: PROB-022
kind: problem
links:
- target: EPIC-002
  relation: informs
- target: NOTE-041
  relation: refines
status: draft
title: Brownfield onboarding — agent analyzes docs instead of code, no discovery protocol, no source tier priority
---

## Problem Statement

При установке ForgePlan на legacy/brownfield проект, агент (Claude Code) идёт в docs/ и строит knowledge base из существующей документации. Документация может быть устаревшей, неполной, или описывать планы вместо текущего состояния. Агент зацикливается на первом найденном документе вместо анализа кодовой базы.

## Signal

Реальный случай: агент нашёл в docs/ документ про миграцию JS→TS и построил весь research вокруг него, проигнорировав код, git history, JSDoc.

## Root Cause

1. ForgePlan не имеет discovery protocol — нет инструкций "начни с кода"
2. Нет приоритетов источников (source tier) — docs и код равноценны
3. Нет MCP tools для structured discovery — агент делает free-form research
4. Нет маркировки legacy docs — docs/ выглядит как source of truth

## Proposed Solution: forgeplan discover

**Архитектура**: ForgePlan = оркестратор + хранилище. Агент (Claude Code) = парсер кода. ForgePlan не трогает код.

**MCP Tools**:
- `forgeplan_discover_start` — создаёт discovery session, возвращает structured protocol агенту
- `forgeplan_discover_finding` — агент сообщает находку (tier, kind, source, content)
- `forgeplan_discover_complete` — закрывает session, генерирует summary report

**Protocol (что агент получает)**:
```
Phase 1 DETECT: Read package.json/Cargo.toml → identify stack → call finding(tier:1, kind:note)
Phase 2 STRUCTURE: ls src/ (3 levels) → map modules → call finding(tier:1, kind:note)
Phase 3 CODE: Read entry points, types, exports → call finding(tier:1, kind:prd/rfc)
Phase 4 GIT: git log -100, git shortlog → hot files, patterns → call finding(tier:1, kind:problem)
Phase 5 TESTS: find test files, estimate coverage → call finding(tier:2, kind:evidence)
Phase 6 DOCS: scan docs/, README → tag legacy-doc → call finding(tier:3, kind:note, tag:legacy-doc)
Phase 7 SYNTHESIZE: review all findings → call complete()
```

**Source Tiers**:
- Tier 1 (Truth): код, git, package manifests
- Tier 2 (Extracted): JSDoc, tests, CI configs  
- Tier 3 (Supplementary): docs/, README — tagged legacy-doc, unverified

**Marketplace**: Agent config `.claude/agents/discover.md` + ForgePlan plugin с MCP tools

## Constraints
- ForgePlan НЕ парсит код — агент это делает
- Protocol должен работать с любым AI agent (Claude, GPT, Cursor)
- Большие проекты: sampling strategy (key files, not all files)
- Без LLM: phases 1,2,4,5 работают (file listing, git log = deterministic)

## Related
- NOTE-041 (original idea)
- NOTE-039 (DSL scripting — may apply to custom discovery rules)
- RFC-002 (Graph Intelligence — discover feeds the graph)
- ForgePlan marketplace (agent distribution)


