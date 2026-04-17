---
depth: standard
id: PRD-051
kind: prd
links:
- target: EPIC-004
  relation: based_on
status: draft
title: Discover Quickstart and FPF Explain
---

---
id: PRD-051
title: "Discover Quickstart and FPF Explain"
status: Draft
author: ForgePlan Team
created: 2026-04-17
updated: 2026-04-17
epic: EPIC-004
priority: P1
depth: standard
domain: general
projectType: cli_tool
stepsCompleted: []
---

# PRD-051: Discover Quickstart and FPF Explain

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Executive Summary

### Vision

Brownfield onboarding за одну команду (`forgeplan discover --quickstart`) плюс разговорный
доступ к FPF Knowledge Base через `forgeplan fpf explain <rule-id>` и
`forgeplan fpf examples <section>` — минимальный барьер входа для legacy-проектов и
для изучения reasoning-правил.

### Problem

`forgeplan discover` (EPIC-003 Sprint 13.3-13.4) реализован как сессионный state machine и
требует 5-7 шагов, чтобы получить первый seed PRD. Новый пользователь на brownfield-проекте
упирается в friction прежде, чем увидит первую ценность. Параллельно: 204 секции FPF KB
доступны через `forgeplan fpf search`, но поиск возвращает фрагменты — нет единственной
команды «объясни правило B.3 целиком с примером и связями». FPF как reasoning-pillar
фактически недоступен в разговорном формате.

**Impact**: enterprise brownfield adoption блокируется (все legacy-репозитории — brownfield);
FPF как reasoning-pillar не используется, потому что фрагментарный search даёт плохой
контекст и для человека, и для агента. Killer-фича остаётся невидимой.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Brownfield lead | Tech lead на legacy Rust-репозитории без PRD | 5-шаговый discover session слишком долог для первого контакта с инструментом |
| FPF-novice | Читает methodology, слышал про B.3 Trust Calculus | `fpf search "trust"` возвращает 20 фрагментов, нет coherent explain целого правила |
| Claude Code agent | Агент должен понять FPF-концепт, чтобы применить его в reasoning | Фрагментарный поиск даёт плохой контекст для ADI |

### Differentiators

- `discover --quickstart` выполняется за один shot вместо пошаговой сессии — специально для
  brownfield onboarding
- `fpf explain` = coherent reading experience, аналог `man <topic>`, но на уровне FPF-правила
  с определением, примером и related-ссылками
- Работает offline при закешированном BGE-M3 индексе; keyword fallback без semantic feature

---

## Success Criteria

<!-- BMAD: Каждый критерий MUST быть SMART — Specific, Measurable, Achievable, Relevant, Time-bound. -->
<!-- Запрещены формулировки: "улучшить", "повысить", "ускорить" без конкретных чисел. -->

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Quickstart seeds PRD from codebase | PRD с заполненными секциями (≥ 3 FR) | 0 | 1 PRD | v0.20.0 | Integration test assertion |
| SC-2 | Quickstart runtime | Wall time на 100-file repo | N/A | < 30 s | v0.20.0 | `time forgeplan discover --quickstart` |
| SC-3 | `fpf explain` returns coherent text | Word count для известного правила | 0 | 200..800 слов | v0.20.0 | `forgeplan fpf explain B.3 \| wc -w` |
| SC-4 | `fpf examples` returns applied patterns | Количество применённых примеров | 0 | ≥ 1 snippet | v0.20.0 | Integration test assertion |

---

## Product Scope

### MVP (In-Scope)

- `forgeplan discover --quickstart <path>` — scan + auto-tag + генерация одного seed PRD из
  top-tier (T1) файла
- Отчёт quickstart: количество tagged files, ID созданного PRD, перечень next-step команд
- `forgeplan fpf explain <rule-id>` — coherent объяснение (определение + пример + related)
- `forgeplan fpf examples <section>` — applied-пример из `.forgeplan/` workspace
- Idempotency: повторный запуск quickstart на том же path не дублирует артефакты

### Out of Scope

- Interactive LLM-dialog внутри discover (остаётся существующая session-форма)
- Multi-seed PRD — генерация только одного seed PRD из top-tier файла
- `fpf export` в PDF / HTML
- `fpf edit` CLI для редактирования KB из командной строки

### Growth Vision

- `discover --quickstart --with-reason` — ADI-цикл сразу после seed
- Multi-seed (top-N tier-1 files → N seed PRDs) в v0.21+
- `fpf explain --llm` — LLM-based summary вместо keyword extract

---

## User Journeys

<!-- BMAD: Для каждого типа пользователя (из Target Users) описать минимум один journey. -->
<!-- Каждый journey должен иметь хотя бы один FR в секции Functional Requirements. -->

### Journey 1: Brownfield lead — one-shot onboarding

**Цель пользователя**: получить первый seed PRD на legacy-репозитории без ручного заполнения.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan init -y && forgeplan discover --quickstart .` | Scan 100+ файлов, auto-tag 50, создан PRD-001 из top-tier файла | < 30 s |
| 2 | `cat .forgeplan/prds/PRD-001*.md` | Seed PRD с реальными FR, извлечёнными из исходных комментариев и README | Готов к ручной доработке |
| 3 | `forgeplan validate PRD-001` | SHOULD warnings, 0 MUST errors | Seed помечен как "review required" |
| 4 | `forgeplan discover --quickstart .` (re-run after edits) | Zero new artifacts; report "idempotent — PRD-001 already exists for path signature `<sha>`" | FR-005 идемпотентность |

**Результат**: пользователь видит первый artifact за одну команду и понимает value proposition. Повторный запуск не плодит дубли — safe для brownfield итеративного onboarding.

### Journey 2: FPF-novice — reads rule

**Цель пользователя**: получить полное объяснение правила B.3 Trust Calculus.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan fpf explain B.3` | Параграф + определение + пример + related rules (C.2, B.5) | 200..800 слов |
| 2 | `forgeplan fpf examples trust-calculus` | Applied-pattern sample из `.forgeplan/` workspace | ≥ 1 snippet |

**Результат**: пользователь получает coherent reading experience по одному rule-id без
необходимости вручную собирать фрагменты из search.

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
| FR-001 | Core | Must | User can run `forgeplan discover --quickstart <path>` to scan a codebase, auto-tag files by source tier, and generate one seed PRD from the top-tier (T1) file | Journey 1 |
| FR-002 | Core | Must | User can see a quickstart report listing count of tagged files, created PRD ID, and suggested next-step commands | Journey 1 |
| FR-003 | Core | Must | User can run `forgeplan fpf explain <rule-id>` to retrieve a coherent explanation containing definition, example, and related rules | Journey 2 |
| FR-004 | Core | Should | User can run `forgeplan fpf examples <section>` to see applied patterns for an FPF concept | Journey 2 |
| FR-005 | UX | Should | User can re-run `discover --quickstart` on the same path without creating duplicate artifacts | Journey 1 |

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
| NFR-001 | Performance | Quickstart shall complete full scan + seed | < 30 s | 100-file Rust repo, no LLM calls | Integration test wall-clock timing |
| NFR-002 | Reliability | `fpf explain` shall return coherent text for known rule or clear error for unknown rule | Known rule: 200..800 words; unknown: exit 1 with suggestion | Offline with BGE-M3 cache populated | Integration test assertion |

---

## Acceptance Criteria

<!-- Обязательно для depth: deep / critical. Опционально для standard. -->
<!-- Формат: Given / When / Then (Gherkin-style) -->

### AC-1: Quickstart seeds from brownfield repo

```gherkin
Given a Rust repository with at least 50 source files
When user runs `forgeplan discover --quickstart .`
Then at least 20 files are tagged with source tier
And exactly one PRD is created with filled Problem and at least 3 FR rows
And the command exits with code 0 within 30 seconds
```

### AC-2: fpf explain known rule

```gherkin
Given FPF Knowledge Base has been ingested via `forgeplan fpf ingest`
When user runs `forgeplan fpf explain B.3`
Then output contains keyword "Trust Calculus"
And output lists at least one related rule
And word count is between 200 and 800
```

### AC-3: Quickstart is idempotent on re-run (covers FR-005)

```gherkin
Given user has run `forgeplan discover --quickstart .` once
  And it created PRD-001
When user runs `forgeplan discover --quickstart .` again on the same path
Then no new PRD is created
  And the command exits with code 0
  And output contains "idempotent" or "already exists"
  And PRD-001 content is unchanged (same sha256)
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| `discover` session state machine (EPIC-003) | Technical | Ready | forgeplan-cli |
| FPF KB — 204 секции ingested | Technical | Ready | forgeplan-core |
| BGE-M3 semantic index | Technical | Optional (keyword fallback) | feature-gated |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | Quickstart seed PRD имеет низкое качество | Medium | Medium | `--quickstart` помечает PRD как "seed — review required"; validate не блокирует, но выдаёт SHOULD warnings | ForgePlan Team |
| R-2 | FPF rule ID не существует в KB | Low | Low | Error с подсказкой: `Did you mean B.3? Run 'forgeplan fpf search <term>'` | ForgePlan Team |

---

## Timeline

<!-- Обязательно для depth: deep / critical. -->

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD Approved | 2026-04-22 | После merge Sprint 1 (PRD-049 + PRD-050) |
| MVP | 2026-04-24 | FR-001..005 shipped |
| GA | 2026-05-02 | v0.20.0 Epic release |

---

## Stakeholders

<!-- Обязательно для depth: deep / critical. -->

| Role | Name | Sign-off |
|------|------|----------|
| Product Owner | ForgePlan Team | [ ] |
| Engineering Lead | ForgePlan Team | [ ] |
| Design | ForgePlan Team | [ ] |
| QA | ForgePlan Team | [ ] |

---

## Affected Files

- crates/forgeplan-cli/src/commands/discover.rs (EDIT — add `--quickstart`)
- crates/forgeplan-cli/src/commands/fpf.rs (EDIT — add `explain` + `examples` subcommands)
- crates/forgeplan-core/src/fpf/mod.rs (EDIT — public API для explain)
- crates/forgeplan-cli/tests/discover_quickstart_test.rs (NEW)
- crates/forgeplan-cli/tests/fpf_explain_test.rs (NEW)

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-004 | Parent epic | Draft |
| EPIC-003 | Prerequisite foundation | Active |
| PRD-035 | Tags + Discover p1 (foundation) | Active |
| PRD-042 | FPF KB Vector Search | Active |

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

