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

Таблица глубины и checklist ниже — **конвенции авторинга**, не полная исполняемая
спецификация валидатора. Длительности — примеры планирования, не автоматические
пороги глубины. Реализованные проверки описаны отдельно.

## Checklist авторинга

### Для всех depth levels:

| # | Секция | Ожидание к документу | Рекомендация по содержимому (не автоматическая проверка) |
|---|--------|-------------|-----------|
| 1 | **Meta Header** | Включить | `id` и `status`; Author, Created, Updated и Priority полезны для прослеживаемости |
| 2 | **Problem Statement** | Включить | Описать проблему и её влияние; число предложений и причинная формулировка — рекомендации ревьюеру |
| 3 | **Goals** | ✅ MUST | ≥ 1 цель, каждая measurable |
| 4 | **Non-Goals** | ✅ MUST | ≥ 1 пункт (scope boundary) |
| 5 | **Functional Requirements** | ✅ MUST | ≥ 1 REQ с Priority (Must/Should/Could) |
| 6 | **Success Metrics** | Рекомендация; validator MUST на Deep | KPI с Current + Target |
| 7 | **Related Artifacts** | ✅ MUST | Links to SPEC/RFC/ADR if exist |

### Для Standard+:

| # | Секция | Ожидание к документу | Рекомендация по содержимому (не автоматическая проверка) |
|---|--------|-------------|-----------|
| 8 | **Target Audience** | ✅ MUST | ≥ 1 persona с описанием |
| 9 | **User Stories** | SHOULD | "As a [role], I want [X], so that [Y]" |
| 10 | **Non-Functional Requirements** | SHOULD | Performance, Security, Scalability |
| 11 | **Dependencies** | SHOULD | External/internal deps |
| 12 | **Risks** | SHOULD | ≥ 1 risk с mitigation |

### Для Deep/Critical:

| # | Секция | Ожидание к документу | Рекомендация по содержимому (не автоматическая проверка) |
|---|--------|-------------|-----------|
| 13 | **Timeline** | ✅ MUST | Milestones с датами |
| 14 | **Stakeholders** | ✅ MUST | Sign-off checkboxes |
| 15 | **Acceptance Criteria** | ✅ MUST | Given/When/Then format |
| 16 | **Competitive Analysis** | COULD | If applicable |

## Реализованная валидация и рекомендации ревьюеру

`validate` проверяет наличие metadata (`id`, `status`), нужных секций,
placeholders и содержимого по глубине. Проверка наличия status сама по себе не
доказывает допустимость его значения. Проверяется наличие Problem; причинное
объяснение или два предложения этой проверкой не гарантируются. Success Metrics
имеет MUST в Deep rules. Плотность Problem в 50 слов — SHOULD на Standard и MUST
на Deep. Эти проверки не доказывают измеримость целей и отсутствие противоречий.

Активация — отдельная операция: она может отказать из-за длины, отсутствия
evidence или заглушек даже без MUST findings у validate. CLI `validate` работает
на legacy и migrated Constellation layout; full MCP `forgeplan_validate`
также успешно проверен. Границы проверки см. в [оговорках пилота](../operations/CONSTELLATION-PILOT.ru.md).

### Checklist ручного review

1. **Completeness** — заполнить нужное содержимое, не placeholders
2. **Measurability** — каждый Goal имеет числовой target
3. **Traceability** — каждый REQ имеет уникальный ID (REQ-N)
4. **Density** — содержательно объяснить проблему; severity порога 50 слов зависит от глубины, как указано выше
5. **Scope Clarity** — Non-Goals ≥ 1 пункт
6. **No Implementation Leakage** — PRD описывает ЧТО, не КАК
7. **Consistency** — Goals и Requirements не противоречат друг другу

### Adversarial Review (из BMAD)

Активно ищите дефекты и подтверждайте каждое замечание конкретным основанием:
- Неизмеримая цель?
- Пропущенный edge case?
- Нереалистичный срок?
- Забытый stakeholder?
- Риск безопасности или приватности?

Ноль подтверждённых дефектов допустим. Укажите область проверки и непроверенные
риски; не выдумывайте замечание ради количества. Опциональный CLI `--adversarial`
может выдать `adversarial-too-clean`, если замечаний нет; severity этого сообщения —
**COULD**, не блокирующий MUST.

## Numbering

| Format | Example |
|--------|---------|
| ID | `PRD-NNN` (display number выделяется по типу внутри проекта) |
| File | `PRD-{NNN}-{kebab-case-title}.md` |
| Path | `.forgeplan/prds/PRD-042-social-login.md` |

## Status Lifecycle

```text
draft --activate--> active
active --> superseded | deprecated | stale
stale --renew--> active
stale --reopen--> deprecated + new draft
stale --deprecate--> deprecated
```

`superseded` и `deprecated` — terminal. Review, approval, implementation и closure
могут описывать этапы процесса, но не значения сохраняемого `status`.
`review` отдельного lifecycle status не создаёт.

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
