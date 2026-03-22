---
id: ADR-006
title: "LanceDB as sole source of truth — no file-based fallback"
status: Accepted
depth: standard
valid_until: 2027-03-22
problem_ref: ""
created: 2026-03-22
updated: 2026-03-22
---

# ADR-006: LanceDB as Sole Source of Truth — No File-Based Fallback

## Context

Phase 3A-3C использовали dual-store: markdown файлы = source of truth + LanceDB для queries. Phase 3D перевёл LanceDB в primary store с markdown как projection (write-only, git-tracked).

Вопрос: нужен ли file-based fallback для случаев когда LanceDB недоступен?

## Decision

**Selected**: LanceDB = единственный source of truth. Файлы = projections. Нет fallback на файлы.

**Why Selected**:
- LanceDB embedded (не сервер) — "недоступен" = corrupted data, что файлы тоже не исправят
- Dual-read создаёт sync bugs (файл отредактирован вручную → LanceDB не знает)
- MCP server (Phase 4.1) работает только через LanceStore — fallback не имеет смысла
- Один path = меньше кода, меньше багов: 8 store methods вместо 16

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| LanceDB primary, file fallback | Rejected | Complexity: нужно detect corruption + merge state. File edits bypass LanceDB → inconsistency |
| Files primary, LanceDB cache | Rejected | Регресс к Phase 3A. Нет structured queries, нет atomic ops |
| **LanceDB only + markdown projection** | **Selected** | Clean architecture. Single source of truth. Projections = derived views |

## Consequences

- `forgeplan init` создаёт LanceDB tables — обязательно перед любой операцией
- Markdown файлы в `.forgeplan/` — read-only для людей, write-only для Forgeplan
- Ручные правки markdown НЕ подхватываются (нужен future `forgeplan import`)
- Git tracks markdown projections — diff/blame работает на projections
- Backup = backup `.forgeplan/lance/` directory

## References

- ADR-002: LanceDB over SQLite (storage choice)
- RFC-003: LanceDB integration (schema, migration)
- Phase 3D: Implementation (PR #5, #6)
