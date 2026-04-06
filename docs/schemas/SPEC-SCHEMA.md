[English](SPEC-SCHEMA.md) · [Русский](SPEC-SCHEMA.ru.md)

# SPEC Schema — Formal Specification

## When to Create a Spec

- API design (REST, GraphQL, gRPC)
- Data model changes
- Protocol definition
- UI component specification
- Integration contract

**Rule**: A Spec is needed when there is a **contract** between two systems or teams.

## Required Sections

| # | Section | Required? | Validation |
|---|---------|-----------|------------|
| 1 | **Meta Header** | ✅ MUST | Status, Author, PRD link, Type |
| 2 | **Summary** | ✅ MUST | What is being specified |
| 3 | **API Contracts** or **Data Models** | ✅ MUST (>= 1) | Endpoints/schemas with examples |
| 4 | **Validation Rules** | SHOULD | Field constraints |
| 5 | **Events / Side Effects** | SHOULD | What happens on create/update/delete |
| 6 | **Versioning** | SHOULD | Version history |
| 7 | **Related Artifacts** | ✅ MUST | PRD, RFC links |

## Spec Types

| Type | Contains | Example |
|------|----------|---------|
| API | Endpoints, request/response, errors | REST API for user management |
| Data Model | Entities, relationships, constraints | Prisma schema changes |
| Protocol | Message format, sequence | WebSocket protocol |
| UI Spec | Components, states, interactions | Design system component |

## Numbering

| Format | Example |
|--------|---------|
| ID | `SPEC-NNN` |
| File | `SPEC-{NNN}-{kebab-case}.md` |
| Path | `docs/specs/SPEC-015-oauth2-api.md` |

## Related Documents

- [PRD-SCHEMA.md](../schemas/PRD-SCHEMA.md) — PRD structure and validation rules
- [EPIC-SCHEMA.md](../schemas/EPIC-SCHEMA.md) — Epic structure and aggregation
- [ARTIFACT-MODEL.md](../methodology/ARTIFACT-MODEL.md) — Full artifact hierarchy
