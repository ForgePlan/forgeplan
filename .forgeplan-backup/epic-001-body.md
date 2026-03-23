# EPIC-001: Forgeplan v1.0 — Real Methodology Engine

## Vision

Превратить Forgeplan из файлового менеджера артефактов в реальный methodology engine который находит пробелы, оценивает готовность, ведёт журнал решений, и даёт предсказуемый routing — без LLM для core логики.

## Outcomes

- Одна команда показывает здоровье проекта (health dashboard)
- Движок сам находит blind spots (артефакты без evidence, решения без ADR)
- Журнал решений с timeline и quality scores
- Depth-aware валидация (не schema check)
- Rule-based routing (детерминированный, без LLM)
- F-G-R scoring — вычисляемые метрики качества
- Lifecycle: Draft → Active → Superseded/Deprecated

## Children

| ID | Type | Title | Status | Progress |
|----|------|-------|--------|----------|
| PRD-002 | PRD | FPF Reasoning Engine | Done | 100% |
| PRD-003 | PRD | Health Dashboard + Blind Spots | Done | 100% |
| PRD-004 | PRD | Decision Journal | Done | 100% |
| PRD-005 | PRD | Validation v2 | Done | 100% |
| PRD-006 | PRD | Smart Routing v2 | Done | 100% |
| PRD-007 | PRD | Artifact Lifecycle | Done | 100% |

## Phases

- [x] Phase 1: Health Dashboard + Blind Spots (PRD-003)
- [x] Phase 2: Decision Journal (PRD-004)
- [x] Phase 3: Validation v2 (PRD-005)
- [x] Phase 4: Smart Routing v2 (PRD-006)
- [x] Phase 5: Lifecycle Commands (PRD-007)
- [x] Phase 6: FPF Engine — F-G-R, Bounded Contexts, Explore-Exploit (PRD-002)

## Progress

```
Phase 1  ████████████████████████  1/1  (100%)
Phase 2  ████████████████████████  1/1  (100%)
Phase 3  ████████████████████████  1/1  (100%)
Phase 4  ████████████████████████  1/1  (100%)
Phase 5  ████████████████████████  1/1  (100%)
Phase 6  ████████████████████████  1/1  (100%)
─────────────────────────────────────────────────
TOTAL                              6/6  (100%)
```

## Non-Goals

- Desktop UI (Phase 5, отдельный Epic)
- Ethics module (FPF Part D)
- Multi-user / team features
