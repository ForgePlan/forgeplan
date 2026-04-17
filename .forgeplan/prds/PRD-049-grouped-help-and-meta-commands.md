---
depth: standard
id: PRD-049
kind: prd
links:
- target: EPIC-004
  relation: based_on
status: draft
title: Grouped Help and Meta Commands
---

---
id: PRD-049
title: "Grouped Help and Meta Commands"
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

# PRD-049: Grouped Help and Meta Commands

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Executive Summary

### Vision

Сделать 60 CLI-команд ForgePlan discoverable через grouped `--help` (8 категорий),
новую мета-команду `forgeplan commands` и self-running `forgeplan demo` на
изолированной `TempDir` песочнице, чтобы первый контакт с продуктом раскрывал
killer-фичи за минуты, а не за дни чтения исходников.

### Problem

60 рабочих CLI-команд (~10k LOC) существуют, покрыты 1194 тестами, но публичный
README упоминает ~15. grep на 12 killer-команд (estimate, calibrate, discover,
blindspots, decompose, journal, drift, gaps, coverage, tag, remember, recall) даёт
0 совпадений в README. Новый пользователь видит `forgeplan --help` как плоский
список из 60 строк — не знает, что важно, что pro, что deprecated. AI-агенту
(Claude Code, Cursor) ещё сложнее: без категорий он не может выбрать правильную
команду для задачи и переходит к брутфорсу через `list`.

**Impact**: 5★ на GitHub при 1194 тестах и зрелом backend — launch не произошёл.
Enterprise-evaluator не находит estimate/doctor/audit за 10 минут и уходит.
AI-агенты вынуждены держать в контексте всю документацию вместо грамотно
структурированного `--help`.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Новый Rust-dev | Ставит `brew install forgeplan`, запускает `--help` впервые | Плоский список 60 команд без категорий приводит к закрытию терминала за 30 секунд |
| AI-агент (Claude Code / Cursor) | Читает `--help` через MCP для ориентации в CLI | Нет группировки — не может выбрать правильную команду и делает брутфорс `list` |
| Enterprise evaluator | Оценивает tool за 10 минут до покупки | Не находит estimate/audit/doctor быстро, игнорирует весь продукт |

### Differentiators

- Rule-based grouped help без LLM — мгновенно, offline, без API-ключей
- Self-running `demo` в песочнице — первый AI-dev tool, который работает прямо
  из install без настройки workspace и конфигурации
- Hero-text в `--help` объявляет lineage продукта (Quint-code + BMAD + OpenSpec +
  FPF + git-adr + LanceDB) — маркетинг на уровне первой команды

---

## Success Criteria

<!-- BMAD: Каждый критерий MUST быть SMART — Specific, Measurable, Achievable, Relevant, Time-bound. -->
<!-- Запрещены формулировки: "улучшить", "повысить", "ускорить" без конкретных чисел. -->

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Grouped help visible | Количество категорий в выводе `forgeplan --help` | 0 | 8 | v0.20.0 | `forgeplan --help \| grep -c "^[A-Z]*:$"` |
| SC-2 | Meta command discoverable | Число команд, распечатанных `forgeplan commands` | 0 | ≥ 60 | v0.20.0 | Подсчёт строк CLI-вывода в integration-тесте |
| SC-3 | Demo runs on fresh install | Wall time `demo --skip-semantic` на чистом TempDir | N/A | < 30 s | v0.20.0 | `time forgeplan demo --skip-semantic` в CI |
| SC-4 | Hero text contains synthesis phrase | Substring match в выводе `--help` | 0 | 1 | v0.20.0 | `forgeplan --help \| grep -c "synthesis\|Quint-code"` |

---

## Product Scope

### MVP (In-Scope)

- `forgeplan commands` — печатает сгруппированное дерево команд по 8 категориям
  (CORE / QUALITY / REASONING / LIFECYCLE / DISCOVERY / ESTIMATE / AUDIT / PRO)
- `forgeplan commands --json` — structured JSON output для AI-агентов и скриптов
- `forgeplan commands --category <NAME>` — фильтр по одной категории
- `forgeplan demo` — self-running sandbox на изолированном TempDir:
  init → route → new prd → reason → evidence → activate
- `forgeplan demo --skip-semantic` — bypass BGE-M3 инициализации (CI, first-run)
- `help_heading` attribute на каждый из 60 subcommand в `main.rs`
- Hero-text в main `forgeplan --help`: synthesis phrase + 5 строк lineage

### Out of Scope

- Interactive TUI для команд (cursive/ratatui) — отдельный Epic
- Search внутри `commands` (уже покрыт `forgeplan search`)
- `demo` с живыми LLM-вызовами (требует API-ключа, не подходит для first-run)
- Переписанный README — отдельный sprint в Distribution Epic
- Локализация вывода `--help` — задача i18n Epic

### Growth Vision

- После v0.20.0 — `demo --keep` с сохранением workspace для повторного осмотра
- Group-specific examples per category (`commands --category ESTIMATE --examples`)
- `commands --deprecated` — отдельный список legacy-команд для миграций

---

## User Journeys

<!-- BMAD: Для каждого типа пользователя (из Target Users) описать минимум один journey. -->
<!-- Каждый journey должен иметь хотя бы один FR в секции Functional Requirements. -->

### Journey 1: Новый Rust-dev — discover via help

**Цель пользователя**: За 2 минуты после install понять, какие killer-команды
предлагает ForgePlan и чем он отличается от `git`, `cargo`, `gh`.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `brew install forgeplan && forgeplan --help` | Hero-text с synthesis phrase + grouped categories CORE/QUALITY/REASONING/... | Первый контакт |
| 2 | `forgeplan commands` | Дерево по 8 категориям с one-liner на каждую команду | Visibility solved |
| 3 | `forgeplan demo --skip-semantic` | Sandbox walkthrough за < 30 s, печать каждого шага + итоги | Immediate value |

**Результат**: Пользователь увидел lineage, нашёл estimate/doctor/discover в
категориях ESTIMATE/QUALITY/DISCOVERY и прогнал полный pipeline за < 3 минут.

### Journey 2: AI-агент — machine-readable help

**Цель пользователя**: AI-агент получает structured список команд, чтобы
выбрать правильный инструмент под задачу пользователя без парсинга человеческого
help-текста.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan commands --json` | Structured JSON `{category: [{name, about}]}` | Parseable для LLM |
| 2 | `forgeplan commands --category REASONING` | Фильтр на одну категорию | Focused context |
| 3 | `forgeplan reason --help` | Category heading "REASONING:" в `--help` команды | Контекст сохранён |

**Результат**: AI-агент строит корректный plan вызовов CLI без угадывания
команд и без лишних обращений к README.

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
| FR-001 | Core | Must | Developer can invoke `forgeplan commands` to list all 60 CLI commands grouped by 8 categories (CORE, QUALITY, REASONING, LIFECYCLE, DISCOVERY, ESTIMATE, AUDIT, PRO) | Journey 1 |
| FR-002 | Core | Must | AI agent can invoke `forgeplan commands --json` to receive a structured commands list as JSON with `{category, name, about}` per entry | Journey 2 |
| FR-003 | Core | Must | User can pass `forgeplan commands --category REASONING` to filter output to a single category | Journey 2 |
| FR-004 | Core | Must | User can run `forgeplan demo` to execute an end-to-end workflow (init → route → new → reason → evidence → activate) inside an isolated sandbox | Journey 1 |
| FR-005 | Core | Must | User can pass `--skip-semantic` flag to `forgeplan demo` to bypass semantic-search initialization for CI and first-run scenarios | Journey 1 |
| FR-006 | UX | Must | User can see hero-text in `forgeplan --help` that identifies the synthesis of Quint-code, BMAD, OpenSpec, FPF and git-adr | Journey 1 |
| FR-007 | UX | Must | User can run `forgeplan <any-subcommand> --help` and see the command's category heading in the help output | Journey 2 |

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
| NFR-001 | Performance | Demo walkthrough completes end-to-end | < 30 s wall clock | `--skip-semantic` flag set, fresh TempDir, release build | `time forgeplan demo --skip-semantic` в CI |
| NFR-002 | Reliability | Demo leaves no artifacts outside TempDir | 0 files written outside sandbox | Default run without `--keep` | Filesystem diff assertion в integration-тесте |
| NFR-003 | Compatibility | Clap `help_heading` attribute supported on pinned dependency | Clap v4.x | Verified via `cargo tree \| grep clap` | CI `cargo check` + integration test grep |
| NFR-004 | Usability | `forgeplan commands` output remains readable | ≤ 120 символов в строке | TTY width ≥ 80 | Визуальная проверка + test на длину строк |

---

## Acceptance Criteria

<!-- Обязательно для depth: deep / critical. Опционально для standard. -->
<!-- Формат: Given / When / Then (Gherkin-style) -->

### AC-1: Grouped help visible

```gherkin
Given a fresh forgeplan install on a clean machine
When a user runs `forgeplan --help`
Then the output contains 8 category headings matching ["CORE", "QUALITY", "REASONING", "LIFECYCLE", "DISCOVERY", "ESTIMATE", "AUDIT", "PRO"]
And commands are listed under their respective category headings
And a hero-text containing "synthesis" or "Quint-code" is printed above the categories
```

### AC-2: Demo is self-contained

```gherkin
Given an isolated TempDir with no existing workspace
When a user runs `forgeplan demo --skip-semantic`
Then a workspace is initialized inside the TempDir
And exactly one PRD is created, scored and activated with evidence
And no files are written outside the TempDir
And the process exits with status 0 within 30 seconds
```

### AC-3: Machine-readable commands

```gherkin
Given forgeplan is installed
When an AI agent runs `forgeplan commands --json`
Then stdout contains a JSON document parseable by `serde_json`
And the document lists at least 60 commands across 8 categories
And each command entry includes `name`, `about` and `category` fields
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| Clap v4.x with `help_heading` attribute | Technical | Ready | crates.io (pinned in Cargo.toml) |
| `tempfile` crate | Technical | Ready | Already a dev-dependency |
| `workspace::init_workspace()` reuse | Technical | Ready | forgeplan-core |
| `ui::styled_*` helpers | Technical | Ready | forgeplan-cli/src/ui.rs |
| EPIC-004 shape merged | Sequential | In progress | ForgePlan Team |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | Clap `help_heading` ведёт себя иначе на pinned 4.x версии | Low | Medium | Verify в первом коммите через `cargo doc`; fallback = manual `override_help` template | ForgePlan Team |
| R-2 | Demo медленно инициализирует workspace на первом запуске | Medium | Low | TempDir + `--skip-semantic` по умолчанию; integration-тест на NFR-001 | ForgePlan Team |
| R-3 | 60 `help_heading` атрибутов засоряют `main.rs` | Medium | Low | Выделить таблицу категорий в отдельный модуль `commands/categories.rs`, генерировать аттрибуты из одного источника | ForgePlan Team |
| R-4 | JSON-формат `commands --json` станет breaking API | Low | Medium | Закрепить схему в integration-тесте; semver bump при любом изменении полей | ForgePlan Team |

---

## Timeline

<!-- Обязательно для depth: deep / critical. -->

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD Approved | 2026-04-18 | MUST секции заполнены, validate PASS |
| RFC Approved | 2026-04-18 | Архитектура help_heading + demo утверждена |
| MVP | 2026-04-19 | FR-001..007 shipped, integration-тесты зелёные |
| GA | 2026-05-02 | Релиз v0.20.0 вместе с остальными Epic-004 PRD |

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

- crates/forgeplan-cli/src/main.rs
- crates/forgeplan-cli/src/commands/commands.rs (NEW)
- crates/forgeplan-cli/src/commands/demo.rs (NEW)
- crates/forgeplan-cli/src/commands/mod.rs
- crates/forgeplan-cli/Cargo.toml
- crates/forgeplan-cli/tests/commands_integration_test.rs (NEW)
- crates/forgeplan-cli/tests/demo_integration_test.rs (NEW)

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-004 | Parent epic | Draft |
| PRD-050 | Sibling Sprint 1 (Doctor + estimate table default) | Draft |
| NOTE-029 | Feature request source (CLI discoverability) | Active (backlog) |

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


