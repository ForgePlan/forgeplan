[English](SPEC-SCHEMA.md) · [Русский](SPEC-SCHEMA.ru.md)

# SPEC Schema — Formal Specification

## Когда создавать Spec

- API design (REST, GraphQL, gRPC)
- Data model changes
- Protocol definition
- UI component specification
- Integration contract

**Правило**: Spec нужен когда есть **контракт** между двумя системами или командами.

## Обязательные секции

| # | Секция | Обязательно? | Валидация |
|---|--------|-------------|-----------|
| 1 | **Meta Header** | ✅ MUST | Status, Author, PRD link, Type |
| 2 | **Summary** | ✅ MUST | Что специфицируется |
| 3 | **API Contracts** или **Data Models** | ✅ MUST (≥1) | Endpoints/schemas с examples |
| 4 | **Validation Rules** | SHOULD | Field constraints |
| 5 | **Events / Side Effects** | SHOULD | What happens on create/update/delete |
| 6 | **Versioning** | SHOULD | Version history |
| 7 | **Related Artifacts** | ✅ MUST | PRD, RFC links |

## Spec Types

| Type | Содержит | Example |
|------|----------|---------|
| API | Endpoints, request/response, errors | REST API для user management |
| Data Model | Entities, relationships, constraints | Prisma schema changes |
| Protocol | Message format, sequence | WebSocket protocol |
| UI Spec | Components, states, interactions | Design system component |

## Numbering

| Format | Example |
|--------|---------|
| ID | `SPEC-NNN` |
| File | `SPEC-{NNN}-{kebab-case}.md` |
| Path | `docs/specs/SPEC-015-oauth2-api.md` |

## Связанные документы

- [PRD-SCHEMA.ru.md](../schemas/PRD-SCHEMA.ru.md) — Структура и правила валидации PRD
- [EPIC-SCHEMA.ru.md](../schemas/EPIC-SCHEMA.ru.md) — Структура и агрегация Epic
- [ARTIFACT-MODEL.ru.md](../methodology/ARTIFACT-MODEL.ru.md) — Полная иерархия артефактов
