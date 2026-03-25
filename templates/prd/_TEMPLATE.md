---
id: PRD-{NNN}
title: "{title}"
status: Draft
author:
created: YYYY-MM-DD
updated: YYYY-MM-DD
epic: EPIC-{NNN}
priority: P0 / P1 / P2 / P3
depth: tactical / standard / deep / critical
domain: general / healthcare / fintech / govtech / edtech
projectType: web_app / api_backend / mobile_app / cli_tool / library
stepsCompleted: []
---

# PRD-{NNN}: {Product Area / Feature Name}

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Executive Summary

### Vision

Что мы строим и почему это важно. Одно предложение, описывающее конечное состояние.

### Problem

Какую проблему решаем. Для кого. Что происходит сейчас и почему это плохо.

**Impact**: Как проблема влияет на пользователей / бизнес (числа, метрики).

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Persona 1 | ... | ... |
| Persona 2 | ... | ... |

### Differentiators

- Чем наше решение отличается от существующих альтернатив
- Уникальное ценностное предложение

---

## Success Criteria

<!-- BMAD: Каждый критерий MUST быть SMART — Specific, Measurable, Achievable, Relevant, Time-bound. -->
<!-- Запрещены формулировки: "улучшить", "повысить", "ускорить" без конкретных чисел. -->

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | ... | ... | ... | ... | ... | ... |
| SC-2 | ... | ... | ... | ... | ... | ... |

---

## Product Scope

### MVP (In-Scope)

- Что входит в минимально жизнеспособный продукт
- Конкретные функции и возможности

### Out of Scope

- Что мы явно НЕ делаем в текущей итерации
- Что откладываем на будущее

### Growth Vision

- Направления развития после MVP
- Потенциальные расширения (без обязательств)

---

## User Journeys

<!-- BMAD: Для каждого типа пользователя (из Target Users) описать минимум один journey. -->
<!-- Каждый journey должен иметь хотя бы один FR в секции Functional Requirements. -->

### Journey 1: {Persona 1 — Scenario Name}

**Цель пользователя**: ...

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | ... | ... | ... |
| 2 | ... | ... | ... |
| 3 | ... | ... | ... |

**Результат**: Что пользователь получает в итоге.

### Journey 2: {Persona 2 — Scenario Name}

**Цель пользователя**: ...

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | ... | ... | ... |
| 2 | ... | ... | ... |

**Результат**: ...

---

## Functional Requirements

<!-- ============================================================ -->
<!-- BMAD QUALITY REMINDERS (НЕ УДАЛЯТЬ):                        -->
<!--                                                              -->
<!-- FORMAT: "[Actor] can [capability]"                            -->
<!--   OK:    "User can filter projects by status"                -->
<!--   BAD:   "Filter component renders project list"             -->
<!--                                                              -->
<!-- NO IMPLEMENTATION LEAKAGE:                                   -->
<!--   Запрещены названия технологий (React, Django, PostgreSQL,  -->
<!--   Redis, AWS, Docker, etc.) ЕСЛИ они не являются частью      -->
<!--   capability. PRD описывает ЧТО, не КАК.                    -->
<!--   OK:    "API consumer can retrieve data via REST endpoint"  -->
<!--   BAD:   "React component fetches data using Redux store"    -->
<!--                                                              -->
<!-- NO SUBJECTIVE ADJECTIVES:                                    -->
<!--   Запрещены: "быстро", "удобно", "интуитивно", "легко",     -->
<!--   "просто", "эффективно" — без конкретных метрик.            -->
<!--                                                              -->
<!-- TRACEABILITY:                                                -->
<!--   Каждый FR MUST traceably link to a User Journey.           -->
<!--   Orphan FR (без связи с journey) = validation failure.      -->
<!-- ============================================================ -->

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | [Actor] can [capability] | Journey 1 |
| FR-002 | Core | Must | [Actor] can [capability] | Journey 1 |
| FR-003 | UX | Should | [Actor] can [capability] | Journey 2 |
| FR-004 | Integration | Could | [Actor] can [capability] | Journey 2 |

---

## Non-Functional Requirements

<!-- ============================================================ -->
<!-- BMAD QUALITY REMINDERS (НЕ УДАЛЯТЬ):                        -->
<!--                                                              -->
<!-- FORMAT: "System shall [metric] [condition] [measurement]"    -->
<!--   OK:    "System shall respond within 200ms at p95 under     -->
<!--           1000 concurrent users, measured by APM"            -->
<!--   BAD:   "System should be fast and responsive"              -->
<!--                                                              -->
<!-- MEASURABILITY:                                               -->
<!--   Каждый NFR MUST содержать конкретное число и метод         -->
<!--   измерения. Запрещены: "быстрый", "отзывчивый",            -->
<!--   "масштабируемый", "надёжный" без цифр.                     -->
<!--                                                              -->
<!-- TEMPLATE: criterion + metric + condition + measurement       -->
<!-- ============================================================ -->

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | System shall respond | < 200ms p95 | Under 1000 concurrent users | APM monitoring |
| NFR-002 | Availability | System shall maintain uptime | 99.9% | Monthly | Uptime monitoring |
| NFR-003 | Security | System shall authenticate | OAuth2/OIDC | All API endpoints | Security audit |
| NFR-004 | Scalability | System shall handle | N concurrent users | Peak load | Load testing |

---

## Acceptance Criteria

<!-- Обязательно для depth: deep / critical. Опционально для standard. -->
<!-- Формат: Given / When / Then (Gherkin-style) -->

### AC-1: {Scenario Name}

```gherkin
Given [предусловие / начальное состояние]
When  [действие пользователя]
Then  [ожидаемый результат]
And   [дополнительный результат, если есть]
```

### AC-2: {Scenario Name}

```gherkin
Given [предусловие]
When  [действие]
Then  [результат]
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| Service X | Technical | Ready | Team A |
| API Y | External | In Progress | Partner |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | ... | Medium | High | ... | ... |
| R-2 | ... | Low | Critical | ... | ... |

---

## Timeline

<!-- Обязательно для depth: deep / critical. -->

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD Approved | YYYY-MM-DD | Requirements locked |
| Spec Complete | YYYY-MM-DD | API contracts defined |
| RFC Approved | YYYY-MM-DD | Architecture decided |
| MVP | YYYY-MM-DD | Core features shipped |
| GA | YYYY-MM-DD | Full release |

---

## Stakeholders

<!-- Обязательно для depth: deep / critical. -->

| Role | Name | Sign-off |
|------|------|----------|
| Product Owner | | [ ] |
| Engineering Lead | | [ ] |
| Design | | [ ] |
| QA | | [ ] |

---

## Affected Files

- crates/forgeplan-core/src/...
- crates/forgeplan-cli/src/...

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-{NNN} | Parent epic | ... |
| SPEC-{NNN} | API contracts | ... |
| RFC-{NNN} | Architecture proposal | ... |
| ADR-{NNN} | Decision record | ... |

---

<!-- ============================================================ -->
<!-- BMAD VALIDATION CHECKLIST (для автора и ревьюера):           -->
<!--                                                              -->
<!-- [ ] Executive Summary содержит vision + problem + users      -->
<!-- [ ] Success Criteria — все SMART с числами                   -->
<!-- [ ] Product Scope — MVP чётко отделён от out-of-scope        -->
<!-- [ ] User Journeys — минимум 1 на каждую персону              -->
<!-- [ ] FR — формат "[Actor] can [capability]", нет impl leakage -->
<!-- [ ] NFR — конкретные метрики, метод измерения                -->
<!-- [ ] Traceability — каждый FR ссылается на journey            -->
<!-- [ ] Acceptance Criteria — Given/When/Then (deep/critical)    -->
<!-- [ ] Risks — минимум 1 риск с mitigation                      -->
<!-- [ ] Related Artifacts — ссылки на SPEC/RFC/ADR если есть     -->
<!--                                                              -->
<!-- ADVERSARIAL REVIEW (BMAD):                                   -->
<!-- Ревьюер ОБЯЗАН найти минимум 1 проблему.                     -->
<!-- 0 найденных проблем = недостаточно тщательный review.        -->
<!-- ============================================================ -->

> **Next step**: После approve → создать SPEC (контракты) и/или RFC (архитектура).
