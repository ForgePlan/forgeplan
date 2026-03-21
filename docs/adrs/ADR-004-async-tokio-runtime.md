---
id: ADR-004
title: "Full async (tokio) вместо sync CLI"
status: Accepted
depth: standard
valid_until: 2027-03-22
problem_ref: ""
created: 2026-03-22
updated: 2026-03-22
---

# ADR-004: Full async (tokio) вместо sync CLI

## Context

Phase 3D: интеграция LanceDB как primary storage. LanceDB Rust SDK полностью async (tokio). Нужно решить: конвертировать весь CLI в async или использовать sync wrapper.

Текущий CLI полностью sync (97 тестов). LanceDB API: `db.connect().execute().await`, `table.query().execute().await`, `table.add(reader).execute().await` — всё async.

## Decision

**Full async migration**: весь core становится async, CLI использует `#[tokio::main]`.

**Selected**: tokio runtime + async fn в core + async tests

**Why Selected**: Чистая архитектура без sync/async boundary. Готовность к Phase 4 (Tauri 2.0 — тоже async). Нет риска nested runtime panic.

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| Full async (tokio) | **Chosen** | Чистая архитектура, Tauri-ready, нет nested runtime |
| Sync wrapper (block_on) | Rejected | Nested runtime panic при вызове из async контекста (Tauri). Скрытая сложность |
| Async only for DB layer | Rejected | Sync/async boundary в каждой команде. Сложный тестинг |

## Consequences

### Positive
- Единая async модель во всём проекте
- LanceDB API используется напрямую (нет обёрток)
- Готовность к Tauri 2.0 (async IPC commands)
- tokio::fs для параллельного I/O (list_artifacts на большом workspace)
- Тесты с `#[tokio::test]` — стандартный async testing

### Negative (trade-offs)
- Все 97 тестов нужно обновить (async fn + .await)
- tokio в зависимостях увеличивает compile time (~30s)
- Binary size +~500KB (tokio runtime)
- Простые функции (find_workspace, parse_frontmatter) остаются sync — нужна дисциплина

### Risks
- Compile time увеличится из-за tokio + arrow + lancedb (~2-3 min full build)
- Новые разработчики должны знать async Rust

## Invariants

- I/O-heavy функции (store CRUD, search, graph) — ДОЛЖНЫ быть async
- Pure computation (validation, scoring, frontmatter parsing) — ДОЛЖНЫ остаться sync
- CLI entry point — `#[tokio::main]` (multi-threaded runtime)
- Тесты — `#[tokio::test]` для async, `#[test]` для sync
- Integration tests (assert_cmd) — НЕ меняются (тестируют binary, не library)

## Evidence Requirements

- Все 97 тестов проходят после миграции
- Binary size < 15MB (NFR-002) после добавления tokio + lancedb + arrow
- Startup time < 100ms (NFR-001) — tokio runtime не должен замедлять

## Valid Until

**Дата**: 2027-03-22

**Refresh Triggers**:
- Появление sync-only embedded DB лучше LanceDB
- Tauri откажется от async в пользу sync
- tokio runtime станет несовместим с целевыми платформами

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| RFC-003 | RFC | based_on (LanceDB integration) |
| ADR-002 | ADR | informs (LanceDB выбран, требует async) |
| PRD-001 | PRD | informs |
| EPIC-001 | Epic | parent |
