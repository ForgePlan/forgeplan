[English](SPEC-SCHEMA.md) · [Русский](SPEC-SCHEMA.ru.md)

# SPEC Schema — Formal Specification

## When to Create a Spec

- API design (REST, GraphQL, gRPC)
- Data model changes
- Protocol definition
- UI component specification
- Integration contract

**Rule**: Use a SPEC when an implementation needs a checkable contract: data/API structure, observable behaviour, or both. A PRD explains what users need and why; a SPEC fixes what an implementation must satisfy.

## Required Sections

Forgeplan **0.38.0** accepts both structural and behavioural SPECs at every depth. The runtime MUST checks are:

| Check | Accepted headings / fields |
|-------|----------------------------|
| Metadata | Non-empty frontmatter `id` and `status` |
| Summary | `Summary`, `Executive Summary`, `Overview` |
| Checkable contract (at least one) | Structural: `API…`, `Data Model…`, `Contract…`; behavioural: `Requirements…`, `Behavioral Contract…`, `Behavioural Contract…` |
| Related artifacts | `Related…` or `Dependencies…` |

Heading matching is case-insensitive by prefix at any Markdown heading level. Author, parent PRD, type, scope, validation rules, events and versioning are template guidance, not extra runtime MUST checks. Use English heading names even in Russian prose.

### Structural contract

Specify request/response schemas, errors, constraints and examples under `## API Contracts`, `## Data Models` or `## Contract`. This form does **not** need artificial `Requirement`/`Scenario` blocks to pass validation.

### Behavioural contract and test oracle

The public `/spec-author` workflow uses observable scenarios as the primary oracle: the expected behaviour against which tests and code are checked.

```markdown
## Summary
Specify how the runner handles a failed step.

## Requirements
### Requirement: Stop after failure
The runner MUST stop when a step exits with a non-zero status.
#### Scenario: Second step fails
- GIVEN a three-step run
- WHEN step two exits with status 1
- THEN step three is not executed
- AND the report identifies step two as failed

## Related
No parent artifact for this standalone example.
```

`spec-requirement-has-scenario` is a **SHOULD** rule. It applies only when the body opens a `### Requirement…`; each such block needs a `#### Scenario…` before the next heading at level 3 or shallower. Fenced examples and HTML comments are ignored by this rule. It checks heading structure, not GIVEN/WHEN/THEN semantics or test coverage. A `Requirements` heading alone can satisfy the contract MUST without proving a usable oracle.

The marketplace workflow is stricter than core validation. `/spec-author` reviews and freezes scenarios before implementation; the coder must not rewrite them to pass tests. The separate `agents-tdd` `/tdd` cycle requires **active PRD + active SPEC with scenarios**, plans tests, obtains assertion-level RED, and has an independent test validator certify them before GREEN. That workflow's hooks freeze tests and pin the SPEC hash; `forgeplan activate` alone does not install or provide that enforcement.

Conformance vectors are optional for pure algorithmic cores or cross-language equivalence, not a blanket SPEC requirement. Keep pseudocode, invariants and API details where useful without replacing observable scenarios in the scenario-driven workflow.

### Validation is not implementation verification

The shared placeholder/stub checks apply to SPECs too. An untouched structural template is not a finished contract: many `{placeholder}` fields trigger `no-stub-content` (SHOULD in validation, blocking in activation). Review content and evidence separately; a schema PASS does not certify behaviour.

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
| Path | `.forgeplan/specs/SPEC-015-oauth2-api.md` |

SPEC uses the shared `draft`, `active`, `stale`, `superseded`, `deprecated` lifecycle, not `Approved` / `Implemented`. Change a frozen scenario through an explicit SPEC revision/delta and supersession, then restart the relevant implementation cycle.

## Related Documents

- [PRD-SCHEMA.md](../schemas/PRD-SCHEMA.md) — PRD structure and validation rules
- [EPIC-SCHEMA.md](../schemas/EPIC-SCHEMA.md) — Epic structure and aggregation
- [ARTIFACT-MODEL.md](../methodology/ARTIFACT-MODEL.md) — Full artifact hierarchy
