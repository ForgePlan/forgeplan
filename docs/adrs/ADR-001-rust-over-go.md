---
id: ADR-001
title: "Rust вместо Go для CLI"
status: Accepted
depth: standard
valid_until: 2027-03-21
problem_ref: ""
created: 2026-03-21
updated: 2026-03-21
---

# ADR-001: Rust вместо Go для CLI

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Context

Нужно выбрать язык для CLI-приложения forgeplan. Требования: single binary, cross-platform, type safety для парсинга markdown/YAML frontmatter, embedded templates. Reference implementations: quint-code (Go), git-adr (Rust).

## Decision

Выбран **Rust** с clap (derive), serde, tera, pulldown-cmark.

**Selected**: Rust

**Why Selected**: Compile-time гарантии для парсеров, единый core для CLI и Tauri desktop app, мощная экосистема для markdown/YAML обработки через serde + pulldown-cmark.

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| Go | Rejected | Слабая type safety (interface{}), нет pattern matching для парсеров. Quint-code reference, быстрая компиляция (2s), cobra CLI |
| Rust | **Chosen** | Compile-time guarantees, serde, clap derive macros, shared core для Tauri |
| TypeScript | Rejected | Не single binary, нужен Node.js runtime. OpenSpec reference, remark/unified для markdown |

## Consequences

### Positive
- Compile-time guarantees для парсеров markdown и YAML frontmatter
- serde обеспечивает типобезопасную (де)сериализацию YAML/JSON
- clap derive macros минимизируют boilerplate для CLI
- Shared core (forgeplan-core) используется и в CLI, и в Tauri desktop app

### Negative (trade-offs)
- Медленная компиляция (30-60s для полной сборки)
- Более крутая кривая обучения по сравнению с Go
- Меньше Go-libraries для MCP (Model Context Protocol)

### Risks
- ONNX Runtime Rust bindings (crate `ort`) менее зрелые чем Go-аналоги

<!-- Depth: standard+ — обязательно для standard, deep, critical -->

## Invariants

- Весь core code в forgeplan-core (shared library)
- CLI и Desktop App -- thin wrappers над core
- Никакой бизнес-логики в forgeplan-cli или forgeplan-tauri

## Evidence Requirements

- Compile time < 60s для incremental build
- Binary size < 15MB without ONNX
- Все парсеры покрыты property-based тестами

## Valid Until

**Дата**: 2027-03-21

**Обоснование TTL**: 1 год -- достаточно чтобы пройти Phase 3 (Rust CLI) и Phase 4 (Tauri Desktop). К этому моменту будет достаточно evidence для подтверждения или пересмотра решения.

**Refresh Triggers** (когда пере-оценить досрочно):
- LanceDB или ONNX Runtime откажутся от поддержки Rust SDK
- Compile time превысит 120s для incremental build
- Появится Go/TS framework с аналогичным shared core для CLI + Desktop

## AI Guidance

> Правила для AI-агентов при работе с этим решением.

- Весь новый код пишется на Rust с английскими идентификаторами
- Не предлагать переписать на Go или TypeScript без нового RFC
- При генерации кода предполагать что forgeplan-core -- единственное место для бизнес-логики
- Если задача конфликтует с этим ADR, явно указать на конфликт

## Implementation Plan

### Phase 0: Foundation
- [ ] **0.1** Настроить Cargo workspace с forgeplan-core и forgeplan-cli
- [ ] **0.2** Добавить базовые зависимости: clap, serde, tera, pulldown-cmark

### Phase 1: Core
- [ ] **1.1** Реализовать artifact types в forgeplan-core
- [ ] **1.2** Реализовать YAML frontmatter parser

## Implementation Log

<!-- Add wave entries as sprints are completed:

### Wave 1 — YYYY-MM-DD
| Task | Teammate | Status | Files |
|------|----------|--------|-------|
| 0.1 | ... | Done | ... |
-->

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| EPIC-001 | Epic | based_on |
| PRD-001 | PRD | informs |
