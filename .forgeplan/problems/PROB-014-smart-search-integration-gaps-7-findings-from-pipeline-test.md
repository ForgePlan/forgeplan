---
depth: standard
id: PROB-014
kind: problem
links:
- target: PRD-020
  relation: informs
- target: EPIC-001
  relation: informs
status: deprecated
title: Smart Search integration gaps — 7 findings from pipeline test
---

## Signal

Full pipeline integration test (vector + graph + scoring) на 82 реальных артефактах выявил 7 gaps.

## Context

Тест запущен 2026-03-29 на реальном .forgeplan/ workspace. Все компоненты работают по отдельности, но при комбинации видны пробелы.

## Findings

### P0: Must fix

**F1. Embed only title, not body** — Vector search ищет по title (20 слов), игнорирует body (200+ слов). 90% смысла в body. Запрос 'routing' не находит PRD-020 первым потому что EVID-026 title ближе.
- Fix: embed title + first 200-300 chars of body

**F2. Graph не показывает тип связи** — 'PROB-012 links: PRD-018' — но КАК связаны? informs/contradicts/supersedes? contradicts = красный флаг.
- Fix: включить relation type в graph walk output

### P1: Should fix

**F3. Нет persistent embeddings** — каждый запуск = re-embed 82 артефактов (~2 sec). На 500+ будет 10+ sec.
- Fix: forgeplan embed сохраняет в LanceDB vector column, search читает готовые

**F4. Нет combined score** — vector similarity, R_eff, graph connectivity существуют отдельно. Нет единого ранга.
- Fix: combined = vector_sim * 0.5 + r_eff * 0.3 + graph_centrality * 0.2

**F5. Нет gap detection** — PRD-020 (Deep) не имеет ADR. Система не предупреждает.
- Fix: forgeplan gaps — проверяет pipeline compliance по depth

### P2: Could fix

**F6. Evidence blind spots** — EVID-015, EVID-025, EVID-026, EVID-027 active но R_eff=0.00. Evidence без structured fields.
- Fix: manual data quality — добавить verdict/CL/type

**F7. Нет anti-results** — search показывает что найдено, но не что ПРОПУЩЕНО (missing ADR для Deep).
- Fix: merge с F5 (gap detection)

## Impact

Без этих фиксов smart search работает на 60% потенциала. Vector находит, но не по body. Graph показывает связи, но без типов. Scoring есть, но не combined.

## Anti-Goodhart

- Не оптимизировать combined score на тестовых запросах (overfit)
- Не добавлять фичи ради фич — каждый fix должен улучшать реальный UX


