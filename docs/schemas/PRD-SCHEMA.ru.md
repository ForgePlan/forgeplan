[English](PRD-SCHEMA.md) · [Русский](PRD-SCHEMA.ru.md)

# PRD Schema — Product Requirements Document

## Когда создавать PRD

| Ситуация | Нужен PRD? | Альтернатива |
|----------|-----------|--------------|
| Новая user-facing фича | ✅ Да | — |
| Значительное изменение продукта | ✅ Да | — |
| Мелкий баг-фикс | ❌ Нет | Сразу RFC или PR |
| Рефакторинг (без UI changes) | ❌ Нет | ADR → RFC |
| Инфраструктура | ❌ Нет | RFC |
| API для внешних клиентов | ✅ Да | + SPEC |
| Внутренний API | ❌ Нет | SPEC → RFC |

**Правило**: PRD нужен когда есть **пользователь** с **проблемой** и нужно определить **что** строить.

## Depth Calibration

| Complexity | Depth | Обязательные секции | Пример |
|-----------|-------|---------------------|--------|
| **Tactical** | 1-2 часа | Problem + Goals + Requirements (3-5) | Добавить фильтр в таблицу |
| **Standard** | 1-2 дня | Все секции | Новый модуль настроек |
| **Deep** | 3-5 дней | Все секции + User Research + Metrics Plan | Новая подсистема |
| **Critical** | 1-2 недели | Всё + Stakeholder Sign-offs + Risk Analysis | Платёжная система |

## Обязательные секции

### Для всех depth levels:

| # | Секция | Обязательно? | Валидация |
|---|--------|-------------|-----------|
| 1 | **Meta Header** | ✅ MUST | Status, Author, Created, Updated, Priority |
| 2 | **Problem Statement** | ✅ MUST | ≥ 2 предложения, содержит "потому что" / "impact" |
| 3 | **Goals** | ✅ MUST | ≥ 1 цель, каждая measurable |
| 4 | **Non-Goals** | ✅ MUST | ≥ 1 пункт (scope boundary) |
| 5 | **Functional Requirements** | ✅ MUST | ≥ 1 REQ с Priority (Must/Should/Could) |
| 6 | **Success Metrics** | ✅ MUST | ≥ 1 KPI с Current + Target |
| 7 | **Related Artifacts** | ✅ MUST | Links to SPEC/RFC/ADR if exist |

### Для Standard+:

| # | Секция | Обязательно? | Валидация |
|---|--------|-------------|-----------|
| 8 | **Target Audience** | ✅ MUST | ≥ 1 persona с описанием |
| 9 | **User Stories** | SHOULD | "As a [role], I want [X], so that [Y]" |
| 10 | **Non-Functional Requirements** | SHOULD | Performance, Security, Scalability |
| 11 | **Dependencies** | SHOULD | External/internal deps |
| 12 | **Risks** | SHOULD | ≥ 1 risk с mitigation |

### Для Deep/Critical:

| # | Секция | Обязательно? | Валидация |
|---|--------|-------------|-----------|
| 13 | **Timeline** | ✅ MUST | Milestones с датами |
| 14 | **Stakeholders** | ✅ MUST | Sign-off checkboxes |
| 15 | **Acceptance Criteria** | ✅ MUST | Given/When/Then format |
| 16 | **Competitive Analysis** | COULD | If applicable |

## Validation Rules (из BMAD)

### Quality Gates

1. **Completeness** — все MUST секции заполнены (не placeholder)
2. **Measurability** — каждый Goal имеет числовой target
3. **Traceability** — каждый REQ имеет уникальный ID (REQ-N)
4. **Density** — Problem Statement ≥ 50 слов
5. **Scope Clarity** — Non-Goals ≥ 1 пункт
6. **No Implementation Leakage** — PRD описывает ЧТО, не КАК
7. **Consistency** — Goals и Requirements не противоречат друг другу

### Adversarial Review (из BMAD)

При review PRD, reviewer **ОБЯЗАН** найти хотя бы 1 проблему:
- Неизмеримый Goal?
- Missing edge case?
- Unrealistic timeline?
- Забытый stakeholder?
- Security/privacy concern?

Если ревьюер не нашёл ни одной проблемы — **пересмотреть более внимательно**.

## Numbering

| Format | Example |
|--------|---------|
| ID | `PRD-NNN` (sequential per project) |
| File | `PRD-{NNN}-{kebab-case-title}.md` |
| Path | `docs/prds/PRD-042-social-login.md` |

## Status Lifecycle

```
Draft → Review → Approved → Implementing → Implemented → Closed
                    ↓
               Rejected (with reason)
```

## Progress Bars (same format as RFC)

```
Phase 0  ████████████████████████  8/8   (100%) DONE
Phase 1  ██████████████░░░░░░░░░░  7/12  ( 58%)
─────────────────────────────────────────────────
TOTAL                              15/20 (75.0%)
```

## Links to Other Artifacts

```
PRD-001 ──creates──→ SPEC-001 (contracts)
PRD-001 ──creates──→ RFC-042 (architecture)
PRD-001 ──creates──→ ADR-007 (decisions)
PRD-001 ──belongs──→ EPIC-003 (initiative)
```
