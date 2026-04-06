[English](ARTIFACT-MODEL.md) · [Русский](ARTIFACT-MODEL.ru.md)

# Artifact Model — PRD Process Engine

## Иерархия артефактов

```
                    ┌─────────────┐
                    │    EPIC     │  Стратегическая инициатива
                    │ (группирует)│  "User Auth System"
                    └──────┬──────┘
                           │ 1:N
                    ┌──────┴──────┐
                    │     PRD     │  Что и зачем
                    │(requirements)│  "Social Login Feature"
                    └──────┬──────┘
                           │ 1:N
              ┌────────────┼────────────┐
              │            │            │
       ┌──────┴──────┐ ┌──┴───┐ ┌──────┴──────┐
       │    SPEC     │ │ RFC  │ │    ADR      │
       │ (контракты) │ │(архит)│ │  (решения)  │
       │"OAuth2 API" │ │"Impl"│ │"Why Auth0?" │
       └──────┬──────┘ └──┬───┘ └─────────────┘
              │            │
              └──────┬─────┘
                     │
              ┌──────┴──────┐
              │   SPRINT    │  Реализация (waves)
              │  (execution)│
              └─────────────┘
```

## Типы артефактов

### 1. EPIC (Epic-NNN)
**Цель**: Группировка связанных PRD/RFC/ADR в одну стратегическую инициативу.

| Поле | Описание |
|------|----------|
| ID | Epic-NNN (sequential) |
| Name | Краткое название инициативы |
| Status | Draft → Active → Done → Archived |
| Outcomes | Измеримые результаты |
| Children | PRD[], RFC[], ADR[] |
| Progress | Агрегированный из children |
| Owner | Product/Engineering lead |
| Timeline | Start → Target completion |

**Когда создавать**: Инициатива > 2 RFC, кросс-командная работа, roadmap item.

### 2. PRD (PRD-NNN)
**Цель**: Описать ЧТО нужно пользователю и ЗАЧЕМ (product perspective).

| Поле | Описание |
|------|----------|
| ID | PRD-NNN |
| Epic | Parent Epic (optional) |
| Status | Draft → Review → Approved → Implemented → Closed |
| Problem | Какую проблему решаем |
| Goals | Что хотим достичь (measurable) |
| Non-Goals | Что НЕ делаем (scope) |
| Requirements | Functional (REQ-N) + Non-Functional (NFR-N) |
| Success Metrics | KPIs, OKRs |
| Acceptance Criteria | Definition of Done |
| Stakeholders | Sign-offs |

**Когда создавать**: Новая фича, значительное изменение продукта, user-facing change.

### 3. SPEC (SPEC-NNN)
**Цель**: Формальная спецификация КАК ИМЕННО работает (contracts, data models).

| Поле | Описание |
|------|----------|
| ID | SPEC-NNN |
| PRD | Parent PRD |
| Type | API | Data Model | Protocol | UI Spec |
| Status | Draft → Approved → Implemented |
| Contracts | API endpoints, schemas, interfaces |
| Data Models | Entity definitions, relationships |
| Constraints | Validation rules, limits |
| Examples | Request/response examples |

**Когда создавать**: API design, data model changes, protocol definition.

### 4. RFC (RFC-NNN)
**Цель**: Архитектурное предложение КАК СТРОИМ (design, implementation plan).

| Поле | Описание |
|------|----------|
| ID | RFC-NNN |
| PRD/SPEC | Parent (optional) |
| Status | Draft → Discussion → Accepted → Implemented → Superseded |
| Summary | Что предлагается |
| Motivation | Зачем нужно изменение |
| Design | Архитектура, компоненты |
| Alternatives | Рассмотренные варианты |
| Phases | Implementation phases с checkboxes |
| Progress Bars | ASCII progress visualization |

**Когда создавать**: Архитектурное решение, новый компонент, migration.

### 5. ADR (ADR-NNN)
**Цель**: Зафиксировать ПОЧЕМУ выбрали конкретное решение (audit trail).

| Поле | Описание |
|------|----------|
| ID | ADR-NNN |
| Context | Ситуация, в которой принято решение |
| Decision | Что решили |
| Rationale | Почему именно так |
| Alternatives | Что ещё рассматривали |
| Consequences | Плюсы и минусы решения |
| Status | Proposed → Accepted → Deprecated → Superseded |

**Когда создавать**: При выборе технологии, архитектурном trade-off, значимом решении.

## Связи между артефактами

```
Epic ──1:N──→ PRD      "Epic contains PRDs"
PRD  ──1:N──→ SPEC     "PRD specifies contracts"
PRD  ──1:N──→ RFC      "PRD drives architecture"
PRD  ──1:N──→ ADR      "PRD requires decisions"
RFC  ──1:N──→ ADR      "RFC documents decisions"
SPEC ──1:1──→ RFC      "Spec informs RFC"
RFC  ──1:N──→ Sprint   "RFC executed in sprints"
```

## Lifecycle Flow

```
IDEA
  ↓
  ├─ Small? ──→ RFC only (no PRD needed)
  ├─ Medium? ─→ PRD → RFC → Sprint
  └─ Large? ──→ Epic → PRD[] → SPEC[] → RFC[] → ADR[] → Sprint[]

Decision needed at any point?
  └─→ ADR (captures context + decision + rationale)
```

## Status Values (unified)

| Status | PRD | SPEC | RFC | ADR | Epic |
|--------|-----|------|-----|-----|------|
| Draft | Requirements gathering | Designing contracts | Writing proposal | Collecting context | Scoping |
| Review/Discussion | Stakeholder review | Technical review | Team discussion | — | — |
| Approved/Accepted | Ready to implement | Contract locked | Approved to build | Decision final | Active work |
| Implemented | Feature shipped | API deployed | Code complete | — | — |
| Done/Closed | Verified in prod | — | All phases done | — | All PRDs done |
| Superseded | Replaced by PRD-N | — | Replaced by RFC-N | Replaced by ADR-N | — |
| Deprecated | — | — | — | No longer valid | Cancelled |

## Numbering Convention

| Artifact | Format | Example |
|----------|--------|---------|
| Epic | `EPIC-NNN` | EPIC-001 |
| PRD | `PRD-NNN` | PRD-042 |
| Spec | `SPEC-NNN` | SPEC-015 |
| RFC | `RFC-NNN` | RFC-128 |
| ADR | `ADR-NNN` | ADR-007 |

Numbers are sequential per artifact type, project-scoped, never reused.

## Связанные документы

- [PRD-RFC-ADR-FLOW.ru.md](PRD-RFC-ADR-FLOW.ru.md) — Дерево решений: какой артефакт создавать
- [DEPTH-CALIBRATION.ru.md](DEPTH-CALIBRATION.ru.md) — Как depth определяет требования к артефактам
- [FORGEPLAN-GUIDE.ru.md](FORGEPLAN-GUIDE.ru.md) — Полный практический гайд
