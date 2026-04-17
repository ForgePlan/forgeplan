---
depth: standard
id: PRD-053
kind: prd
links:
- target: EPIC-004
  relation: based_on
status: draft
title: LLM Provider Trait
---

---
id: PRD-053
title: "LLM Provider Trait"
status: Draft
author: ForgePlan Team
created: 2026-04-17
updated: 2026-04-17
epic: EPIC-004
priority: P0
depth: deep
domain: general
projectType: cli_tool
stepsCompleted: []
---

# PRD-053: LLM Provider Trait

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Executive Summary

### Vision

Refactor concrete `LlmClient` в `trait LlmProvider` с четырьмя production-ready
реализациями (Anthropic, OpenAI-compatible, Ollama, Gemini) плюс CLI-команды
`forgeplan provider list/set/test` для runtime switching — enterprise adoption
unblocker. Backwards compat: legacy `provider: gemini` в config.yaml продолжает
работать без миграции.

### Problem

Текущий `LlmClient` (crates/forgeplan-core/src/llm/mod.rs) — concrete struct
с `generate()`, где выбор между Anthropic и OpenAI-compatible реализован через
branching (`if is_anthropic() else openai_compatible`). Поддержка provider'ов
("openai", "claude", "gemini", "ollama", "custom") сделана внутри одного struct.
Enterprise-клиенты (EU fintech / health / HR с ML) не могут использовать ForgePlan,
потому что у них уже есть Anthropic / OpenAI contracts, а Gemini хардкоджен как
default. Добавить provider = модифицировать core `generate()` — closed for extension.

**Impact**: один killer feature (reasoning) блокирует adoption во всём EU-enterprise.
Compliance-case (PRD-052) + provider swap (этот PRD) — два enterprise-unblockers.
Без trait-based extension невозможны community-providers (HuggingFace, custom).

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Enterprise CTO | Evaluating ForgePlan, уже имеет Anthropic/OpenAI contract | Gemini хардкоджен как default → блокер закупки |
| Privacy-conscious user | Хочет запускать Ollama локально без отправки данных | Config поддерживает Ollama, но `test` и `doctor` не знают как его пингануть |
| Plugin author | Хочет добавить HuggingFace / custom provider | Нет открытой extension point → требуется fork |

### Differentiators

- Четыре production-ready providers в одном v0.20.0 release через один trait
- Backwards compat гарантирована: legacy config.yaml работает без миграции
- `forgeplan doctor` предупреждает о legacy формате и предлагает `provider set` для
  миграции
- Extension point для community providers — impl trait и готово

---

## Success Criteria

<!-- BMAD: Каждый критерий MUST быть SMART — Specific, Measurable, Achievable, Relevant, Time-bound. -->
<!-- Запрещены формулировки: "улучшить", "повысить", "ускорить" без конкретных чисел. -->

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Четыре providers поддержаны | Количество working integration tests | 1 | 4 (Anthropic, OpenAI, Gemini, Ollama) | v0.20.0 | Integration test count |
| SC-2 | Backwards compat с legacy config | v0.19.0 config.yaml работает без миграции | N/A | Pass | v0.20.0 | Snapshot test на legacy config |
| SC-3 | Binary size delta | MB добавлено refactor'ом | 0 | ≤ 1 MB (baseline 43 MB) | v0.20.0 | `ls -lh target/release/forgeplan` до/после |
| SC-4 | Runtime switching работает | `provider set` меняет активный provider в следующей команде | N/A | Works | v0.20.0 | Integration test с mocked HTTP |
| SC-5 | Vtable overhead негligible | Overhead vs concrete dispatch | N/A | < 0.1% от LLM call latency | v0.20.0 | Criterion microbenchmark |

---

## Product Scope

### MVP (In-Scope)

- `trait LlmProvider { generate, validate_config, name }` с dyn-compatible signature
- Четыре реализации: `AnthropicProvider`, `OpenAiCompatibleProvider`, `OllamaProvider`,
  `GeminiProvider`
- `forgeplan provider list` — перечисляет available providers + active + config status
- `forgeplan provider set <name>` — переключает в `config.yaml`
- `forgeplan provider test [--mock]` — живой ping или mock для CI
- Backwards compat: legacy `provider: gemini` работает без миграции
- `forgeplan doctor` warning о legacy формате

### Out of Scope

- Streaming provider API (SSE / token streams)
- Function calling / tool use через provider trait
- Vision API / multimodal input
- Multi-provider load balancing / fallback chains
- BYOK custom HuggingFace-style providers за пределами OpenAI-compatible

### Growth Vision

- Streaming support для всех providers
- Function-calling trait extension
- Provider plugin system (dynamic loading)
- Fallback chains с retry budget

---

## User Journeys

<!-- BMAD: Для каждого типа пользователя (из Target Users) описать минимум один journey. -->
<!-- Каждый journey должен иметь хотя бы один FR в секции Functional Requirements. -->

### Journey 1: Enterprise CTO — swap to Anthropic

**Цель пользователя**: переключить ForgePlan с Gemini на Anthropic Claude без ручного
редактирования config.yaml.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan provider list` | Четыре providers, active = gemini | Показывает config status для каждого |
| 2 | `forgeplan provider set anthropic` | Config обновлён, печатает active = anthropic | Atomic write в config.yaml |
| 3 | `forgeplan provider test` | Live ping — «anthropic reachable, model claude-opus-4-7 responded» | Exit 0 |
| 4 | `forgeplan reason PRD-001` | Использует Anthropic API | Смена не требует restart |

**Результат**: enterprise CTO разблокирован без ручного редактирования YAML.

### Journey 2: Privacy-conscious user — Ollama local

**Цель пользователя**: использовать Ollama локально, без отправки данных наружу.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan provider set ollama` | Config обновлён с base_url http://localhost:11434 | Default base_url |
| 2 | `forgeplan provider test --mock` | Mock bypass для offline CI | Exit 0 без network calls |
| 3 | `forgeplan reason PRD-001` | Использует локальный Ollama | Приватность обеспечена |

**Результат**: локальный inference работает, CI-friendly благодаря `--mock`.

### Journey 3: Existing user — silent upgrade

**Цель пользователя**: обновиться до v0.20.0 без ломки существующего workflow.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `brew upgrade forgeplan` к v0.20.0 | Existing config.yaml с `provider: gemini` продолжает работать | Zero migration steps |
| 2 | `forgeplan reason PRD-001` | Успешно выполняется через Gemini | Backwards compat подтверждён |
| 3 | `forgeplan doctor` | Warns "Legacy provider format detected; run `forgeplan provider set gemini` to migrate" | Non-blocking warning |

**Результат**: существующие пользователи не видят регрессии; миграция опциональна.

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
| FR-001 | Core | Must | Developer can implement a new provider by implementing the `LlmProvider` trait with methods `generate`, `validate_config`, and `name` | Journey 1 |
| FR-002 | Core | Must | User can invoke `forgeplan provider list` to see all built-in providers and the currently active one with config status | Journey 1 |
| FR-003 | Core | Must | User can invoke `forgeplan provider set <name>` to switch the active provider in config.yaml | Journey 1 |
| FR-004 | Core | Must | User can invoke `forgeplan provider test` to live-ping the active provider, or `forgeplan provider test --mock` for CI usage | Journey 2 |
| FR-005 | Core | Must | Existing user can run v0.20.0 with a v0.19.0 config.yaml containing `provider: gemini` without any migration step | Journey 3 |
| FR-006 | UX | Should | User can run `forgeplan doctor` and receive a warning about legacy config format with the exact remediation command | Journey 3 |

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
| NFR-001 | Performance | Release binary size delta shall stay within budget | ≤ 1 MB delta (baseline 43 MB → max 44 MB) | Release build, default features | `ls -lh target/release/forgeplan` before/after refactor |
| NFR-002 | Performance | Vtable dispatch overhead shall remain negligible | < 0.1% of total LLM call latency | Criterion microbench, mocked HTTP | Benchmark suite in `crates/forgeplan-core/benches/` |
| NFR-003 | Reliability | All four providers shall pass integration tests | 100% passing | Mocked HTTP responses in CI | CI test count |
| NFR-004 | Compatibility | Legacy config.yaml shall remain compatible | 100% successful parse and dispatch | v0.19.0 config format with `provider: gemini` field | Snapshot test on frozen v0.19.0 config file |

---

## Acceptance Criteria

<!-- Обязательно для depth: deep / critical. Опционально для standard. -->
<!-- Формат: Given / When / Then (Gherkin-style) -->

### AC-1: Runtime provider switching

```gherkin
Given config.yaml has `provider: gemini`
When user runs `forgeplan provider set anthropic`
Then config.yaml is updated to `provider: anthropic`
And subsequent `forgeplan reason PRD-001` uses the Anthropic provider verified by mock HTTP
```

### AC-2: Backwards compatibility

```gherkin
Given a workspace created with v0.19.0 whose config.yaml contains `provider: gemini`
When the binary is upgraded to v0.20.0
And user runs `forgeplan reason PRD-001`
Then the command succeeds and dispatches to the Gemini provider
And no migration step is required from the user
```

### AC-3: Binary size budget

```gherkin
Given the release build for v0.19.0 measures 43 MB
When the PRD-053 refactor is merged and a release build is produced
Then the resulting binary size is ≤ 44 MB
And the 1 MB headroom is documented in EVID pack
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| ADR-007 approved (trait vs enum decision) | Methodology | Draft (this Epic) | ForgePlan Team |
| `Config::load()` | Technical | Ready | forgeplan-core |
| Existing `LlmClient::generate()` impl (as reference for refactor) | Technical | Ready | forgeplan-core |
| PRD-050 doctor command | Technical | Draft (Sprint 1) | ForgePlan Team |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | Breaking change ломает существующие workspaces | Medium | Critical | `#[serde(default)]` на provider field + migration warning в doctor + explicit legacy config integration test (NFR-004) | ForgePlan Team |
| R-2 | Binary size blow-up > 1 MB от monomorphization | Low | Medium | Замер до merge; при превышении — fallback на enum dispatch (ADR-007 H2) | ForgePlan Team |
| R-3 | `provider test` flaky в CI (live API) | Medium | Low | `--mock` mode с stubbed HTTP responses; live test только локально | ForgePlan Team |
| R-4 | Trait object safety violation | Low | High | CI check `cargo check --all-features` с `dyn LlmProvider`; trait MUST быть dyn-compatible | ForgePlan Team |

---

## Timeline

<!-- Обязательно для depth: deep / critical. -->

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD Approved | 2026-04-26 | После ADR-007 approved |
| ADI reasoning complete | 2026-04-26 | `forgeplan reason PRD-053 --fpf` — 3+ hypotheses |
| Spec Complete | 2026-04-28 | Trait signature + provider contracts финализированы |
| RFC Approved | 2026-04-28 | Архитектурное предложение approved |
| MVP | 2026-04-30 | FR-001..006 shipped |
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

- crates/forgeplan-core/src/llm/mod.rs (REFACTOR — разбить на trait + providers/)
- crates/forgeplan-core/src/llm/provider.rs (NEW — trait definition)
- crates/forgeplan-core/src/llm/providers/mod.rs (NEW — module root)
- crates/forgeplan-core/src/llm/providers/anthropic.rs (NEW)
- crates/forgeplan-core/src/llm/providers/openai.rs (NEW)
- crates/forgeplan-core/src/llm/providers/ollama.rs (NEW)
- crates/forgeplan-core/src/llm/providers/gemini.rs (NEW)
- crates/forgeplan-core/src/config/mod.rs (EDIT — provider enum + backwards compat serde)
- crates/forgeplan-cli/src/commands/provider.rs (NEW — CLI wrapper)
- crates/forgeplan-cli/src/commands/doctor.rs (EDIT — legacy provider format warning)
- crates/forgeplan-cli/src/main.rs (EDIT — register command)
- crates/forgeplan-cli/src/commands/mod.rs (EDIT — export module)
- crates/forgeplan-cli/tests/provider_integration_test.rs (NEW)

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-004 | Parent epic | Draft |
| ADR-007 | Decision record (trait vs enum) | Draft |
| PRD-050 | Doctor edits в scope (legacy warning) | Draft |

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

