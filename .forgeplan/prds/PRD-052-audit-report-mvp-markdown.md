---
depth: standard
id: PRD-052
kind: prd
links:
- target: EPIC-004
  relation: based_on
status: draft
title: Audit Report MVP Markdown
---

---
id: PRD-052
title: "Audit Report MVP Markdown"
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

# PRD-052: Audit Report MVP Markdown

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Executive Summary

### Vision

One-command technical documentation export: `forgeplan audit-report --standard eu-ai-act --format md`
генерирует markdown-отчёт со всеми per-artifact metadata (author, activation date,
evidence chain, R_eff history, supersession lineage), покрывающий **EU AI Act Article 11
(Technical Documentation)** и **Article 12 (Record-keeping)** — part of a full compliance
package, not the full compliance system.

### Scope clarification (post-audit 2026-04-17)

**This PRD addresses Art.11 (technical documentation) and Art.12 (record-keeping).**

**This PRD does NOT address Art.9 (Risk Management System)** — Art.9 requires hazard
identification, residual-risk analysis, testing under foreseeable misuse, post-market
monitoring feedback loop. That is a separate RMS process, not artifact-based export.
Future PRD may add Art.9 RMS support via Problem/Solution/Evidence pipeline, but not
this MVP.

### Problem

EU AI Act вступает в силу 2026-08-02. High-risk AI-системы (fintech, health, HR)
обязаны предоставлять **technical documentation** (Art.11) и вести **records** (Art.12)
каждого инженерного решения с evidence. У ForgePlan уже есть все необходимые данные:
PRD/ADR в git, EvidencePack с CL-level, R_eff decay, supersession chains. Чего нет —
единой команды, которая собирает это в compliance-отчёт соответствующий Art.11 §§ 1-8.

**Impact**: enterprise compliance-buyer не может показать Art.11 technical documentation
regulator без ручной сборки 50+ markdown-файлов; CTO fintech-компании тратит 10 часов
в год на аудит; ML-ops lead не может предоставить Art.12 records для ML-decisions.
Technical-documentation-кейс — один из двух enterprise-unblockers (вместе с LLM provider
swap).

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Compliance officer | Готовит Art.11 technical documentation для EU AI Act audit | Вручную собирает PRD + Evidence + ADR из 50+ markdown файлов |
| CTO fintech | Готовит ежегодный compliance report (Art.11 + Art.12) | 10 часов ручной сборки → ошибки и пропуски в audit trail |
| ML ops lead | Предоставляет Art.12 records для ML-решений | Нет structured export; evidence разбросан по ADR |

### Differentiators

- Только инструмент, у которого есть evidence decay + R_eff history — регулятор видит
  «свежесть» каждого доказательства, а не просто наличие
- Single command, markdown out-of-the-box (mdbook/pandoc-friendly для PDF pipeline)
- Filter surface: `--since`, `--until`, `--kind` — targeted audit windows

---

## Success Criteria

<!-- BMAD: Каждый критерий MUST быть SMART — Specific, Measurable, Achievable, Relevant, Time-bound. -->
<!-- Запрещены формулировки: "улучшить", "повысить", "ускорить" без конкретных чисел. -->

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Report non-empty на реальном workspace | Размер файла (200 artifacts) | 0 | > 10 KB | v0.20.0 | `wc -c report.md` |
| SC-2 | Report покрывает обязательные секции | Секций на каждый активированный artifact | 0 | ≥ 5 (metadata, evidence chain, R_eff history, decay timeline, supersession) | v0.20.0 | grep assertion в integration test |
| SC-3 | Date filters work corrrectly | Artifacts внутри окна | Все | Только попадающие в окно | v0.20.0 | Integration test с `--since` и `--until` |
| SC-4 | Runtime | Wall time на 200-artifact workspace | N/A | < 5 s | v0.20.0 | `time forgeplan audit-report` |

---

## Product Scope

### MVP (In-Scope)

- `forgeplan audit-report --standard eu-ai-act --format md|json`
- Фильтры: `--since YYYY-MM-DD`, `--until YYYY-MM-DD`, `--kind prd,rfc,adr`
- Per-artifact секции: title, author, created/activated/superseded dates, evidence chain
  (все EVID → artifact links), R_eff history, decay timeline, supersession lineage
- Deterministic output: тот же workspace snapshot → тот же report

### Out of Scope

- PDF-генерация (требует pandoc/mdbook) — NOTE follow-up
- SOC 2 / ISO 42001 / GDPR Art.22 стандарты — NOTE follow-ups после MVP
- Interactive report viewer
- Auto-upload к regulator endpoints

### Growth Vision

- PDF export через mdbook/pandoc pipeline
- Дополнительные стандарты (SOC 2, ISO 42001, GDPR Art.22)
- Web-viewer с фильтрами и drill-down в evidence chain

---

## User Journeys

<!-- BMAD: Для каждого типа пользователя (из Target Users) описать минимум один journey. -->
<!-- Каждый journey должен иметь хотя бы один FR в секции Functional Requirements. -->

### Journey 1: Compliance officer — annual audit package

**Цель пользователя**: собрать single-file compliance deliverable за 2026 год.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan audit-report --standard eu-ai-act --format md --since 2026-01-01 > audit-2026.md` | Markdown со всеми активированными artifacts от 2026-01-01 | < 5 s |
| 2 | Reviews `audit-2026.md`, прикладывает к compliance package | Single-file deliverable готов | Deterministic — повторный запуск даёт тот же результат |

**Результат**: годовой audit package собирается одной командой вместо 10 часов ручной работы.

### Journey 2: CTO fintech — quick R_eff snapshot для ADR

**Цель пользователя**: получить machine-readable snapshot всех ADR с evidence graph.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan audit-report --standard eu-ai-act --format json --kind adr` | JSON со всеми ADR и их evidence graph | Для программного потребления |
| 2 | Пайпит в jq / дашборд | Structured data ready | Дальнейшая аналитика вне scope |

**Результат**: CTO получает structured input для внутреннего compliance-дашборда.

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
| FR-001 | Core | Must | Compliance officer can run `forgeplan audit-report --standard eu-ai-act --format md` to generate a markdown compliance report covering all activated artifacts | Journey 1 |
| FR-002 | Core | Must | Report consumer can read per-artifact sections containing title, author, activation date, evidence chain, R_eff history, decay timeline, and supersession lineage | Journey 1 |
| FR-003 | Core | Must | User can filter report by `--since YYYY-MM-DD` and `--until YYYY-MM-DD` to bound the audit window | Journey 1 |
| FR-004 | Core | Should | User can filter report by `--kind prd,rfc,adr` to scope artifact types | Journey 2 |
| FR-005 | Core | Must | User can pass `--format json` to receive machine-readable output for downstream tooling | Journey 2 |

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
| NFR-001 | Performance | Report shall generate | < 5 s wall clock | 200-artifact workspace, markdown format | Integration test timing via `time` |
| NFR-002 | Compliance | Report shall map to EU AI Act Art.11 (Technical Documentation) sections §§ 1-8 | Explicit § mapping table in `audit_report/standards.rs` + ≥ 5 sections per activated artifact | EU AI Act Article 11 clauses (Annex IV mapping) | Schema assertion + manual legal review |
| NFR-003 | Reliability | Report output shall be deterministic | Identical output | Identical workspace snapshot, same CLI flags | Snapshot test сравнением двух runs |

---

## Acceptance Criteria

<!-- Обязательно для depth: deep / critical. Опционально для standard. -->
<!-- Формат: Given / When / Then (Gherkin-style) -->

### AC-1: Report contains mandated sections

```gherkin
Given a workspace with 200 artifacts (from ForgePlan itself)
When user runs `forgeplan audit-report --standard eu-ai-act --format md > report.md`
Then report.md contains phrase "Evidence Chain" at least once per activated artifact
And report.md contains "R_eff History" at least once per activated artifact
And file size is greater than 10 KB
```

### AC-2: Date filter is correct

```gherkin
Given artifacts with activated_at dates spanning 2025-01-01 to 2026-04-17
When user runs `forgeplan audit-report --standard eu-ai-act --since 2026-01-01`
Then report excludes all artifacts activated before 2026-01-01
And report includes all artifacts activated on or after 2026-01-01
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| `projection::render_projection_record()` | Technical | Ready | forgeplan-core |
| `journal::build_journal()` | Technical | Ready | forgeplan-core |
| `export::export_all()` pattern | Technical | Ready | forgeplan-core |
| `ArtifactRecord` fields (author, created_at, activated_at, valid_until, links) | Technical | Ready | forgeplan-core/db/store.rs |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | EU AI Act Art.11 / Annex IV interpretation меняется до 2026-08-02 | Medium | High | Схема в `audit_report/standards.rs` versioned (v1 = Annex IV 2024-07 baseline); добавление новых sections через schema bump; output включает `standard_version` поле | ForgePlan Team |
| R-2 | Regulator rejects because we claimed Art.9 compliance but delivered Art.11 | — | — | **Resolved 2026-04-17**: scope clarified in Vision; README + --help output explicitly mentions Art.11 + Art.12, NOT Art.9 |
| R-2 | Performance degrades на very large workspace (>1000 artifacts) | Low | Medium | Streaming markdown output; NFR-001 измеряется только на 200 artifacts | ForgePlan Team |

---

## Timeline

<!-- Обязательно для depth: deep / critical. -->

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD Approved | 2026-04-22 | После merge Sprint 1 |
| MVP | 2026-04-25 | FR-001..005 shipped |
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

- crates/forgeplan-core/src/audit_report/mod.rs (NEW — report generator)
- crates/forgeplan-core/src/audit_report/standards.rs (NEW — schema per standard)
- crates/forgeplan-cli/src/commands/audit_report.rs (NEW — CLI wrapper)
- crates/forgeplan-cli/src/main.rs (EDIT — register command)
- crates/forgeplan-cli/src/commands/mod.rs (EDIT — export module)
- crates/forgeplan-cli/tests/audit_report_integration_test.rs (NEW)

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-004 | Parent epic | Draft |
| PRD-051 | Sibling Sprint 2 PRD | Draft |

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

