---
depth: standard
id: PRD-025
kind: prd
links:
- target: EPIC-001
  relation: refines
status: draft
title: Nx Monorepo Migration — Polyglot Workspace
---

---
id: PRD-025
title: "Nx Monorepo Migration — Polyglot Workspace"
status: Draft
author: gogocat
created: 2026-04-05
updated: 2026-04-05
epic: EPIC-001
priority: P2
depth: deep
domain: general
projectType: cli_tool
stepsCompleted: []
---

# PRD-025: Nx Monorepo Migration — Polyglot Workspace

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Executive Summary

### Vision

Единый polyglot монорепо с Nx — Rust crates, JS apps и shared packages управляются через общий dependency graph с affected builds, кэшированием и единой CI pipeline.

### Problem

Forgeplan состоит из нескольких технологических доменов (Rust CLI/MCP, Astro website, будущий Tauri desktop, VS Code extension), которые живут в одном git-репо но не имеют общего инструмента оркестрации. Cargo workspace управляет только Rust crates, npm — только website. Результат: нет единого build all, нет affected builds в CI (PR пересобирает всё), нет shared design tokens между website и будущим desktop, нет dependency graph между Rust и JS. С добавлением каждого нового app (Tauri, VS Code) проблема будет расти нелинейно — каждый новый домен требует ручной CI интеграции, дупликации конфигов и ручного отслеживания зависимостей.

**Impact**: CI время растёт линейно с каждым PR (rebuild всего). Shared tokens дублируются. Новый app = неделя CI/config boilerplate.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Developer (self) | Разработчик Forgeplan | Ручной build/test каждого домена отдельно |
| CI/CD | GitHub Actions | Пересобирает всё на каждый PR, нет cache |
| Contributor | Внешний контрибьютор | Непонятно как собрать проект целиком |

### Differentiators

- **Polyglot**: Nx + @monodon/rust — единый граф для Rust И JS (Turbo не умеет Rust)
- **Affected builds**: `nx affected --target=build` — только то что затронуто изменением
- **Shared packages**: `packages/tokens` — один источник для CSS vars + JS constants
- **Ready for Tauri**: desktop app сразу получит shared core + tokens + UI

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

- Nx workspace config (nx.json, project.json для каждого package/crate)
- Migrate website/ → apps/website/
- Extract tokens.ts → packages/tokens/
- project.json для Rust crates (via @monodon/rust)
- `nx affected` working for both Rust and JS
- CI: `nx affected --target=build,test` instead of rebuild-all
- Justfile as human-friendly wrapper over nx commands

### Out of Scope

- Tauri desktop app setup (Phase 5)
- VS Code extension
- packages/ui (shared React components)
- Remote cache (Nx Cloud) — local cache only for now
- Micro-frontends or module federation

### Growth Vision

- `apps/desktop/` — Tauri 2.0 consuming forgeplan-core + packages/tokens + packages/ui
- `apps/vscode/` — VS Code extension consuming forgeplan-mcp
- `packages/ui/` — shared React components (forge design system)
- Nx Cloud remote cache for CI
- `nx release` for versioning and changelog automation

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
| FR-001 | Core | Must | [ ] Developer can run `nx build website` to build the landing page | J1 |
| FR-002 | Core | Must | [ ] Developer can run `nx test forgeplan-core` to test Rust crate | J1 |
| FR-003 | Core | Must | [ ] Developer can run `nx affected --target=build` to build only changed packages | J2 |
| FR-004 | Core | Must | [ ] Developer can run `nx graph` to see dependency graph (Rust + JS) | J1 |
| FR-005 | Core | Must | [ ] Shared tokens package exports COLORS and geometry utils for website and future apps | J1 |
| FR-006 | Infra | Must | [ ] CI runs `nx affected --target=build,test` instead of full rebuild | J2 |
| FR-007 | Infra | Should | [ ] Local build cache speeds up repeated builds | J1 |
| FR-008 | DX | Should | [ ] Justfile provides human-friendly aliases for nx commands | J1 |
| FR-009 | Core | Must | [ ] Existing `cargo test` and `npm run dev` still work independently | J3 |

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
| PRD Approved | 2026-04-05 | Requirements locked |
| Spec Complete | 2026-04-05 | API contracts defined |
| RFC Approved | 2026-04-05 | Architecture decided |
| MVP | 2026-04-05 | Core features shipped |
| GA | 2026-04-05 | Full release |

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

- crates/forgeplan-core/src/**
- crates/forgeplan-cli/src/**

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-025 | Parent epic | ... |
| SPEC-025 | API contracts | ... |
| RFC-025 | Architecture proposal | ... |
| ADR-025 | Decision record | ... |

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


