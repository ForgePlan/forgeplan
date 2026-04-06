---
depth: standard
id: PRD-020
kind: prd
links:
- target: RFC-003
  relation: refines
status: active
title: LLM-first Smart Routing
---

# PRD-020: LLM-first Smart Routing

## Progress

```
FR  ░░░░░░░░░░░░░░░░░░░░░░░░  0/8  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/8  (  0%)
```

---

## Executive Summary

### Vision

Forgeplan routing определяет depth задачи семантически через LLM, а не по ключевым словам — работая на любом языке с точностью >85%.

### Problem

Текущий Smart Routing v2 использует 15 hardcoded keyword triggers + 6 эвристик для определения depth задачи. Это покрывает ~60% английских кейсов. Для русского языка была попытка добавить ключевые слова, но она провалилась из-за морфологической сложности: "новая команда" / "новой команды" / "новую команду" — три разные формы одного слова. Каждый новый язык требовал бы поддержки отдельных списков склонений и спряжений.

**Impact**: AI-агенты (основные потребители MCP) получают неверный routing в ~40% случаев на английском и ~100% на других языках. Неверный depth = либо избыточная бюрократия (Deep вместо Tactical), либо пропущенная документация (Tactical вместо Standard).

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| AI Agent | Claude Code, Cursor через MCP tools | Неверный depth для задач на русском/смешанном языке |
| CLI Developer | Разработчик использующий `forgeplan route` напрямую | Keyword mismatch для нестандартных формулировок |
| Team Lead | Управляет методологией команды | Не может доверять автоматическому routing без ручной проверки |

### Differentiators

- Единственный tool, сочетающий rule-based fallback с LLM-классификацией для methodology routing
- FPF Knowledge Base (204 секции) инжектируется в prompt для domain-aware классификации
- Graceful degradation: без API key всё работает как раньше (Level 0)

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Точность depth-классификации (Level 1) | accuracy % | ~60% (Level 0) | >85% | По готовности | Тест-сьют из 50 задач на 3 языках |
| SC-2 | Латентность Level 1 | секунды p95 | N/A | <3s | По готовности | Benchmark на 20 запросов |
| SC-3 | Обратная совместимость Level 0 | regression count | 0 | 0 | Всегда | Существующие unit tests |
| SC-4 | Offline-работоспособность | availability | 100% | 100% | Всегда | Tests без API key |

---

## Product Scope

### MVP (In-Scope)

- Level 0 (keywords): сохранить текущий rule-based engine как fallback
- Level 1 (LLM classify): LLM классифицирует depth + pipeline по описанию задачи
- Автоматический fallback Level 1 → Level 0 при отсутствии API key или ошибке LLM
- Индикация использованного уровня в output (level: 0|1)
- Инъекция FPF KB контекста в Level 1 prompt
- Кастомизация prompt через `.forgeplan/prompts/route.md`
- Флаг `--level 0|1` для принудительного выбора уровня

### Out of Scope

- Level 2 (FPF reasoning) — полный ADI cycle для сложных случаев (будущая итерация)
- Изменение структуры RoutingResult
- Continuous learning / feedback loops
- Multi-turn routing conversations
- Кэширование LLM-ответов routing

### Growth Vision

- Level 2: полный FPF ADI cycle для ambiguous/critical задач
- Обучение на истории routing решений пользователя
- Routing confidence threshold с авто-эскалацией Level 0 → 1 → 2

---

## User Journeys

### Journey 1: AI Agent — Routing задачи на русском языке

**Цель пользователя**: Получить корректный depth для задачи описанной на русском.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | Agent вызывает `route("добавить валидацию email в форму регистрации")` | Система определяет API key настроен → Level 1 | Auto-detect уровня |
| 2 | — | LLM классифицирует: depth=standard, pipeline=PRD→RFC | FPF KB в prompt |
| 3 | — | Ответ: `{depth: standard, pipeline: [prd, rfc], level: 1, confidence: 90%}` | level: 1 в output |

**Результат**: Agent получает корректный depth без зависимости от языка описания.

### Journey 2: Developer — Offline routing без API key

**Цель пользователя**: Использовать routing без настройки LLM provider.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | Developer запускает `forgeplan route "add email validation"` | Нет API key → Level 0 | Graceful fallback |
| 2 | — | Keyword match: "add" → Standard | Текущая логика |
| 3 | — | Ответ: `{depth: standard, pipeline: [prd, rfc], level: 0}` | level: 0 в output |

**Результат**: Всё работает как раньше, без деградации.

### Journey 3: Developer — Принудительный выбор уровня

**Цель пользователя**: Проверить routing на конкретном уровне.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | Developer запускает `forgeplan route --level 0 "сложная задача"` | Принудительно Level 0 | Игнорирует API key |
| 2 | — | Keyword match (может быть неточным для русского) | Ожидаемо |

**Результат**: Пользователь контролирует уровень routing.

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | [Agent/User] can route task description to depth via Level 0 (keywords) instantly offline | Journey 2 |
| FR-002 | Core | Must | [Agent/User] can route task description via Level 1 (LLM) when API key configured | Journey 1 |
| FR-003 | Core | Must | [System] can automatically fall back from Level 1 to Level 0 when no API key or LLM error | Journey 2 |
| FR-004 | UX | Must | [System] can show which routing level was used in output (level: 0 or 1) | Journey 1, 2 |
| FR-005 | Integration | Must | [System] can inject FPF KB context into Level 1 prompt for domain-aware classification | Journey 1 |
| FR-006 | Config | Should | [User] can customize Level 1 prompt via `.forgeplan/prompts/route.md` | Journey 1 |
| FR-007 | UX | Should | [System] can show routing capabilities during `forgeplan init` (Level 0 always, Level 1 if API key) | Journey 2 |
| FR-008 | Core | Should | [Agent/User] can use `--level 0\|1` flag to force specific routing level | Journey 3 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | Level 0 shall respond | <10ms p95 | Любые условия | Benchmark test |
| NFR-002 | Performance | Level 1 shall respond | <3s p95 | При доступном LLM API | Benchmark test 20 запросов |
| NFR-003 | Accuracy | Level 1 shall classify depth correctly | >85% accuracy | Тест-сьют 50 задач 3 языка | Automated test suite |
| NFR-004 | Compatibility | Existing RoutingResult consumers shall not break | 0 breaking changes | Все existing callers | Compilation + existing tests |

---

## Acceptance Criteria

### AC-1: LLM routing с API key

```gherkin
Given API key настроен в config
When  пользователь вызывает `forgeplan route "добавить новую фичу"`
Then  система использует Level 1 (LLM) для классификации
And   результат содержит level: 1
And   depth корректно определён
```

### AC-2: Fallback без API key

```gherkin
Given API key НЕ настроен
When  пользователь вызывает `forgeplan route "add new feature"`
Then  система автоматически использует Level 0 (keywords)
And   результат содержит level: 0
And   поведение идентично текущему
```

### AC-3: Fallback при ошибке LLM

```gherkin
Given API key настроен, но LLM API возвращает ошибку
When  пользователь вызывает `forgeplan route "task description"`
Then  система fallback на Level 0
And   результат содержит level: 0
And   warning о fallback выводится в stderr
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| Multi-provider LLM client | Technical | Ready | forgeplan-core/llm |
| FPF Knowledge Base (204 sections) | Technical | Ready | forgeplan-core/fpf |
| Prompt customization system | Technical | Ready | forgeplan-core/llm |
| Smart Routing v2 (Level 0) | Technical | Ready | forgeplan-core/routing |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | LLM API латентность >3s для некоторых провайдеров | Medium | Medium | Timeout + fallback на Level 0 | Core team |
| R-2 | LLM неверно классифицирует depth | Medium | High | FPF KB context + тест-сьют для регрессии | Core team |
| R-3 | Расход токенов на routing (cost) | Low | Medium | Краткий prompt, только classification task | Core team |

---

## Affected Files

- `crates/forgeplan-core/src/routing/mod.rs` — добавление Level 1 dispatch
- `crates/forgeplan-core/src/routing/signals.rs` — Level 0 без изменений
- `crates/forgeplan-core/src/llm/route.rs` — LLM classify prompt + FPF KB injection
- `crates/forgeplan-core/src/fpf/knowledge.rs` — context extraction для routing
- `crates/forgeplan-cli/src/commands/route.rs` — `--level` flag, level display
- `crates/forgeplan-core/src/config/types.rs` — routing level config

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| PRD-006 | Предшественник (Smart Routing v2) | Active |
| PROB-006 | Мотивация (routing misses UX scope) | Active |
| RFC-001 | Родительская архитектура CLI | Active |


