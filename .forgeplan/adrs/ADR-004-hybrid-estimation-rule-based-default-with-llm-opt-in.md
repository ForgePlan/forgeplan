---
depth: standard
id: ADR-004
kind: adr
links:
- target: RFC-005
  relation: based_on
- target: PRD-022
  relation: refines
status: active
title: Hybrid Estimation — Rule-based Default with LLM Opt-in
---

---
id: ADR-004
title: "Hybrid Estimation — Rule-based Default with LLM Opt-in"
status: Draft
author: User + AI
created: 2026-03-31
updated: 2026-03-31
rfc: RFC-005
depth: standard
---

# ADR-004: Hybrid Estimation — Rule-based Default with LLM Opt-in

## Context

PRD-022 требует estimate engine с Fibonacci complexity scoring. Ключевой вопрос: кто определяет complexity — правила, LLM, или пользователь вручную?

Три конкурирующих требования:
1. **Offline-first**: Forgeplan должен работать без API key
2. **Accuracy**: LLM даёт более точные оценки (~80% vs ~60% для правил)
3. **Determinism**: Одинаковый input должен давать одинаковый output

## Decision

**Hybrid подход: Rule-based L0 (default) + LLM L1 (opt-in) + Manual override (highest priority).**

Приоритет: Manual > LLM > Rules.

```
estimate PRD-022                    → Rule-based (L0), offline, <1s
estimate PRD-022 --ai-score         → LLM scoring (L1), requires API key, <30s
update PRD-022 --complexity FR-001=5 → Manual override, highest trust
```

Этот паттерн повторяет Smart Routing из PRD-020 (L0 rules / L1 LLM) — consistency across features.

## Alternatives Considered

| # | Alternative | Why Rejected |
|---|-------------|-------------|
| A | Rules only | Недостаточная accuracy для сложных FR |
| B | LLM only | Не работает offline, стоит деньги, не детерминированно |

## Consequences

**Positive:**
- Работает offline (rule-based L0 всегда доступен)
- LLM улучшает когда доступен — graceful enhancement
- Manual override даёт полный контроль пользователю
- Consistent с паттерном Smart Routing

**Negative:**
- Два scoring path = больше тестов
- Rule-based scoring может быть неточным для нестандартных FR
- Кеширование LLM результатов добавляет сложность

**Neutral:**
- Grade multipliers (Jun×2.0, Mid×1.5, Sen×1.0, PS×0.7, AI×0.4) — настраиваемые в config.yaml с hardcoded defaults

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| RFC-005 | RFC | decided_by |
| PRD-022 | PRD | implements |
| PRD-020 | PRD | pattern_source (L0/L1 hybrid) |
| ADR-003 | ADR | consistent_with (files-first approach) |

