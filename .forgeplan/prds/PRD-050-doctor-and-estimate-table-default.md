---
depth: standard
id: PRD-050
kind: prd
links:
- target: EPIC-004
  relation: based_on
status: draft
title: Doctor and Estimate Table Default
---

---
id: PRD-050
title: "Doctor and Estimate Table Default"
status: Draft
author: ForgePlan Team
created: 2026-04-17
updated: 2026-04-17
epic: EPIC-004
priority: P0
depth: standard
domain: general
projectType: cli_tool
stepsCompleted: []
---

# PRD-050: Doctor and Estimate Table Default

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Executive Summary

### Vision

Добавить `forgeplan doctor` как one-command диагностику (config, lance, LLM, embed
cache, structure, orphans, stale) и переключить `forgeplan estimate` default output
на compact multi-grade таблицу, чтобы пользователь видел состояние workspace и
killer-артефакт оценки за один вызов, без разбирательств с документацией.

### Problem

Когда workspace ломается (stale embed cache, некорректный `config.yaml`, отстающий
LanceDB-индекс), пользователь получает cryptic ошибки в отдельных командах и не
понимает, где именно сбой. Нет единой команды "что не так". Вторая проблема:
`forgeplan estimate` по умолчанию печатает verbose breakdown, но самый ценный
artefact — multi-grade таблица (Jun/Mid/Sen/PS/AI hours) — скрыт за флагом,
о котором никто не знает.

**Impact**: support-запросы "workspace broken, не знаю куда смотреть" + unique
killer-фича (multi-grade estimate) не видна новым пользователям. Returning user
после месяца паузы тратит 30+ минут на разбирательство вместо одной команды.
Enterprise-evaluator не видит estimate-таблицу и не понимает, зачем нужен
Forgeplan поверх обычного estimation-тикета.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Returning user | Открывает workspace после месяца паузы | Embed cache устарел, ничего не работает, неясно, где корневая причина |
| First-time estimator | Впервые запускает `forgeplan estimate PRD-001` | Получает текстовый breakdown на ~40 строк, не видит compact multi-grade таблицы |
| CI pipeline | Запускает `forgeplan validate --ci` и ищет структурный health workspace | Нет единого entry point `doctor --ci` с machine-readable exit code |

### Differentiators

- `forgeplan doctor` = композиция существующих `health::health_report()` + LLM
  ping + config validation; safe-by-default (не трогает storage)
- `estimate` default раскрывает multi-grade killer-фичу без `--help` exploration
- `doctor --fix` — non-destructive auto-repair (rebuild cache, reindex), без
  drop table и без удаления markdown

---

## Success Criteria

<!-- BMAD: Каждый критерий MUST быть SMART — Specific, Measurable, Achievable, Relevant, Time-bound. -->
<!-- Запрещены формулировки: "улучшить", "повысить", "ускорить" без конкретных чисел. -->

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Doctor covers critical checks | Количество верифицированных проверок | 0 | ≥ 7 | v0.20.0 | Integration-тест перечисляет 7 assertions |
| SC-2 | Doctor runs quickly | Wall time на mature workspace (200 артефактов) | N/A | < 3 s | v0.20.0 | `time forgeplan doctor --no-llm` в CI |
| SC-3 | Estimate table compact | Количество строк default output | ~40 | ≤ 15 | v0.20.0 | `forgeplan estimate PRD-001 \| wc -l` |
| SC-4 | Verbose flag restores old output | Количество строк с `--verbose` | 0 | ≥ 30 | v0.20.0 | `forgeplan estimate PRD-001 --verbose \| wc -l` |
| SC-5 | Doctor exit code on fail | Exit code при отсутствующем config | 0 | ≠ 0 | v0.20.0 | Assertion в CI integration-тесте |

---

## Product Scope

### MVP (In-Scope)

- `forgeplan doctor` с 7 проверками: config valid, lance/ readable, LLM provider
  reachable (skip с `--no-llm`), embed cache integrity, `.forgeplan/` structure,
  orphans count, stale artifacts count
- Флаги `--ci` (structured JSON output + non-zero exit на critical fail), `--fix`
  (auto-rebuild embed cache + reindex), `--no-llm` (skip live ping)
- `forgeplan estimate <id>` default → compact multi-grade таблица
  (ID, Description, Complexity, Jun, Mid, Sen, PS, AI hours)
- `forgeplan estimate <id> --verbose` восстанавливает поведение v0.19.0
  (полный breakdown с confidence reasons и hints)
- Doctor safe-by-default: без `--fix` не пишет в storage

### Out of Scope

- `doctor --fix` с destructive-операциями (drop table, wipe markdown)
- Auto-repair для broken LanceDB schema (требует migration, отдельный ADR)
- `estimate --pdf` export — отложено в Distribution Epic
- Telemetry upload результатов doctor в внешний сервис
- `doctor --watch` long-running для CI — отложено в Growth Vision

### Growth Vision

- `doctor --watch` для CI long-running health check
- `estimate --compare <id1> <id2>` для A/B grade comparison
- Plugin-система для custom doctor-проверок (user-defined health rules)
- Rich-формат `doctor --ci` с JUnit XML для интеграции с CI-dashboards

---

## User Journeys

<!-- BMAD: Для каждого типа пользователя (из Target Users) описать минимум один journey. -->
<!-- Каждый journey должен иметь хотя бы один FR в секции Functional Requirements. -->

### Journey 1: Returning user — unblock workspace

**Цель пользователя**: Понять за одну команду, почему workspace не отвечает
после месяца паузы, и починить проблему без `rm -rf .forgeplan`.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan doctor` | 7 проверок напечатаны, 1 FAILED (stale embed cache) с подсказкой | Source identified |
| 2 | `forgeplan doctor --fix` | Auto-rebuild cache, re-run checks, итог: all green | Unblocked без ручного wipe |
| 3 | `forgeplan list` | Артефакты снова находятся, работа продолжается | Продукт работает |

**Результат**: Пользователь восстановил рабочий workspace за две команды
вместо 30-минутной сессии чтения issue-трекера.

### Journey 2: First-time estimator — видит value

**Цель пользователя**: Получить за первый запуск `estimate` понятную таблицу
оценок по грейдам, чтобы передать её в планирование sprint'а.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan estimate PRD-049` | Compact table: ID / Description / Complexity / Jun 16h / Mid 12h / Sen 8h / PS 5.6h / AI 1.0h | Immediate value |
| 2 | `forgeplan estimate PRD-049 --verbose` | Полный breakdown + confidence reasons + hints | Deep dive on demand |

**Результат**: Пользователь видит multi-grade таблицу с первого запуска,
понимает уникальную ценность Forgeplan относительно Jira estimation.

### Journey 3: CI pipeline — gate on health

**Цель пользователя**: Автоматически блокировать merge, если workspace в
нездоровом состоянии, без живых LLM-вызовов.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan doctor --ci --no-llm` в pipeline | JSON output, exit 0 при healthy, non-zero при critical | Pipeline gate работает |
| 2 | При fail — CI publish JSON в artefacts | Разработчик читает структурный отчёт | Actionable feedback |

**Результат**: Merge-gate на workspace health работает без секретов и
внешних зависимостей.

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
| FR-001 | Core | Must | User can invoke `forgeplan doctor` to run at least 7 diagnostic checks (config, index, LLM provider, embed cache, directory structure, orphans, stale artifacts) | Journey 1 |
| FR-002 | Core | Must | User can pass `--ci` flag to `forgeplan doctor` for structured JSON output and a non-zero exit code on critical failures | Journey 3 |
| FR-003 | Core | Should | User can pass `--fix` flag to `forgeplan doctor` to auto-repair recoverable issues. Auto-repair operations are **explicitly enumerated** and touch only derived state: (a) rebuild `.fastembed_cache/`, (b) `scan-import` missing markdown rows into LanceDB, (c) clear stale lock files. Operations that could lose data (drop LanceDB tables, wipe markdown, modify `config.yaml`, re-run migrations) are NEVER performed by `--fix`. Non-interactive invocation requires `--fix --yes` (ditto for CI). | Journey 1 |
| FR-004 | Core | Must | User can pass `--no-llm` flag to `forgeplan doctor` to skip the LLM provider reachability probe for offline or CI scenarios | Journey 3 |
| FR-005 | Core | Must | Doctor must not mutate storage by default (read-only behavior when `--fix` is absent) | Journey 3 |
| FR-006 | Core | Must | User can invoke `forgeplan estimate <id>` and receive a compact multi-grade table (Jun, Mid, Sen, PS, AI hours) as the default output | Journey 2 |
| FR-007 | Core | Must | User can pass `--verbose` to `forgeplan estimate` to see the previous detailed breakdown with confidence reasons and hints | Journey 2 |

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
| NFR-001 | Performance | Doctor runs end-to-end | < 3 s wall clock | Mature workspace (200 артефактов), `--no-llm` | Integration-тест с таймером |
| NFR-002 | Reliability | Doctor is non-destructive by default | 0 storage mutations | Default run (без `--fix`) | Filesystem diff assertion |
| NFR-003 | Compatibility | Estimate backwards-compat через `--verbose` | Вывод совпадает со snapshot v0.19.0 | Verified против v0.19.0 snapshot | Snapshot-тест на текстовый diff |
| NFR-004 | Observability | Doctor `--ci` выход соответствует schema | JSON валиден против закреплённой schema | Любой запуск с `--ci` | Integration-тест с `serde_json::from_str` + schema check |

---

## Acceptance Criteria

<!-- Обязательно для depth: deep / critical. Опционально для standard. -->
<!-- Формат: Given / When / Then (Gherkin-style) -->

### AC-1: Doctor diagnoses missing config

```gherkin
Given a workspace with a corrupted .forgeplan/config.yaml
When a user runs `forgeplan doctor`
Then at least one check reports FAILED with a descriptive message pointing at config.yaml
And the process exits with a non-zero status code
And no files are modified during the run
```

### AC-2: Estimate default is compact

```gherkin
Given an activated PRD with filled Functional Requirements
When a user runs `forgeplan estimate PRD-049`
Then stdout contains a table with columns ID, Description, Complexity, Jun, Mid, Sen, PS, AI
And total line count of the output is at most 15 lines
And passing `--verbose` reproduces the v0.19.0 breakdown format
```

### AC-3: Doctor auto-fix

```gherkin
Given a workspace with a stale embed cache
When a user runs `forgeplan doctor --fix`
Then the embed cache is rebuilt
And a second run of `forgeplan doctor` reports all checks green
And no markdown files under .forgeplan/ are deleted or modified
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| `health::health_report()` reuse | Technical | Ready | forgeplan-core |
| `estimate::display::format_table()` reuse | Technical | Ready | forgeplan-core |
| `LlmClient::generate()` reuse | Technical | Ready | forgeplan-core |
| `Config::load()` reuse | Technical | Ready | forgeplan-core |
| PRD-049 merged to dev | Sequential | Blocked by PRD-049 | EPIC-004 |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | `doctor --fix` повреждает LanceDB при rebuild | Low | High | Dry-run mode на первом этапе; explicit confirmation; backup-флаг перед rebuild | ForgePlan Team |
| R-2 | Estimate compact table теряет критичный контекст | Medium | Medium | `--verbose` сохраняет все v0.19.0 данные; snapshot-тест закрепляет формат | ForgePlan Team |
| R-3 | Live LLM ping в `doctor` делает команду flaky в CI | High | Medium | `--no-llm` flag + автоматический skip если переменная окружения `CI=true` | ForgePlan Team |
| R-4 | 7 проверок разрастаются до 15 при росте фич | Medium | Low | Регистр проверок через trait `DoctorCheck`, добавление через конфиг | ForgePlan Team |

---

## Timeline

<!-- Обязательно для depth: deep / critical. -->

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD Approved | 2026-04-19 | Заблокировано до merge PRD-049 |
| RFC Approved | 2026-04-20 | Архитектура doctor + estimate flip утверждена |
| MVP | 2026-04-21 | FR-001..007 shipped, integration-тесты зелёные |
| GA | 2026-05-02 | Релиз v0.20.0 в составе Epic-004 |

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

- crates/forgeplan-cli/src/commands/doctor.rs (NEW)
- crates/forgeplan-cli/src/commands/estimate.rs
- crates/forgeplan-cli/src/main.rs
- crates/forgeplan-cli/src/commands/mod.rs
- crates/forgeplan-cli/tests/doctor_integration_test.rs (NEW)

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-004 | Parent epic | Draft |
| PRD-049 | Prerequisite (Sprint 1, shipped first) | Draft |
| NOTE-029 | Feature request source (CLI discoverability + doctor ask) | Active (backlog) |

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


