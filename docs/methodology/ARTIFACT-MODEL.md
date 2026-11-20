[English](ARTIFACT-MODEL.md) · [Русский](ARTIFACT-MODEL.ru.md)

# Artifact Model — PRD Process Engine

The hierarchy, cardinalities and content fields below are authoring conventions,
not database constraints. Sprint is an execution concept, not an artifact kind.
Typed links are `informs`, `based_on`, `supersedes`, `contradicts`, `refines`;
Epic membership also has `parent_epic`. A prose arrow does not create a link.

## Artifact Hierarchy

```
                    ┌─────────────┐
                    │    EPIC     │  Strategic initiative
                    │ (groups)    │  "User Auth System"
                    └──────┬──────┘
                           │ 1:N
                    ┌──────┴──────┐
                    │     PRD     │  What and why
                    │(requirements)│  "Social Login Feature"
                    └──────┬──────┘
                           │ 1:N
              ┌────────────┼────────────┐
              │            │            │
       ┌──────┴──────┐ ┌──┴───┐ ┌──────┴──────┐
       │    SPEC     │ │ RFC  │ │    ADR      │
       │ (contracts) │ │(arch)│ │ (decisions) │
       │"OAuth2 API" │ │"Impl"│ │"Why Auth0?" │
       └──────┬──────┘ └──┬───┘ └─────────────┘
              │            │
              └──────┬─────┘
                     │
              ┌──────┴──────┐
              │   SPRINT    │  Implementation (waves)
              │  (execution)│
              └─────────────┘
```

## Artifact Types

### 1. EPIC (Epic-NNN)
**Purpose**: Group related PRDs/RFCs/ADRs into a single strategic initiative.

| Field | Description |
|-------|-------------|
| ID | Epic-NNN (sequential) |
| Name | Short initiative name |
| Status | `draft`, `active`, `superseded`, `deprecated`, `stale` — see lifecycle below |
| Outcomes | Measurable results |
| Children | PRD[], RFC[], ADR[] |
| Progress | Aggregated from children |
| Owner | Product/Engineering lead |
| Timeline | Start → Target completion |

**When to create**: Initiative with > 2 RFCs, cross-team work, roadmap item.

### 2. PRD (PRD-NNN)
**Purpose**: Describe WHAT the user needs and WHY (product perspective).

| Field | Description |
|-------|-------------|
| ID | PRD-NNN |
| Epic | Parent Epic (optional) |
| Status | Same persisted lifecycle as all artifact kinds; see below |
| Problem | What problem we are solving |
| Goals | What we want to achieve (measurable) |
| Non-Goals | What we are NOT doing (scope) |
| Requirements | Functional (REQ-N) + Non-Functional (NFR-N) |
| Success Metrics | KPIs, OKRs |
| Acceptance Criteria | Definition of Done |
| Stakeholders | Sign-offs |

**When to create**: New feature, significant product change, user-facing change.

### 3. SPEC (SPEC-NNN)
**Purpose**: Formal specification of HOW EXACTLY it works (contracts, data models).

| Field | Description |
|-------|-------------|
| ID | SPEC-NNN |
| PRD | Parent PRD |
| Type | API | Data Model | Protocol | UI Spec |
| Status | Same persisted lifecycle as all artifact kinds; see below |
| Contracts | API endpoints, schemas, interfaces |
| Data Models | Entity definitions, relationships |
| Constraints | Validation rules, limits |
| Examples | Request/response examples |

**When to create**: API design, data model changes, protocol definition.

### 4. RFC (RFC-NNN)
**Purpose**: Architectural proposal for HOW WE BUILD (design, implementation plan).

| Field | Description |
|-------|-------------|
| ID | RFC-NNN |
| PRD/SPEC | Parent (optional) |
| Status | Same persisted lifecycle as all artifact kinds; implementation phases are separate |
| Summary | What is proposed |
| Motivation | Why the change is needed |
| Design | Architecture, components |
| Alternatives | Considered alternatives |
| Phases | Implementation phases with checkboxes |
| Progress Bars | ASCII progress visualization |

**When to create**: Architectural decision, new component, migration.

### 5. ADR (ADR-NNN)
**Purpose**: Record WHY a specific decision was made (audit trail).

| Field | Description |
|-------|-------------|
| ID | ADR-NNN |
| Context | Situation in which the decision was made |
| Decision | What was decided |
| Rationale | Why this particular choice |
| Alternatives | What else was considered |
| Consequences | Pros and cons of the decision |
| Status | Same persisted lifecycle as all artifact kinds; acceptance is a process decision |

**When to create**: Technology choice, architectural trade-off, significant decision.

## Relationships Between Artifacts

```
Epic ──1:N──→ PRD      "Epic contains PRDs"
PRD  ──1:N──→ SPEC     "PRD specifies contracts"
PRD  ──1:N──→ RFC      "PRD drives architecture"
PRD  ──1:N──→ ADR      "PRD requires decisions"
RFC  ──1:N──→ ADR      "RFC documents decisions"
SPEC ──1:1──→ RFC      "Spec informs RFC"
RFC  ──1:N──→ Sprint   "RFC executed in sprints"
```

## Lifecycle Flow

```
IDEA
  ↓
  ├─ Small? ──→ RFC only (no PRD needed)
  ├─ Medium? ─→ PRD → RFC → Sprint
  └─ Large? ──→ Epic → PRD[] → SPEC[] → RFC[] → ADR[] → Sprint[]

Decision needed at any point?
  └─→ ADR (captures context + decision + rationale)
```

## Persisted Status and Process Phases

Core lifecycle kinds use `draft`, `active`, `superseded`, `deprecated`, `stale`;
other kinds may have different or no stored lifecycle.

```text
draft --activate--> active
active --> superseded | deprecated | stale
stale --renew--> active
stale --reopen--> deprecated + new draft
stale --deprecate--> deprecated
```

`superseded` and `deprecated` are terminal. `review` reports checks and does not
create a status. Approved, Accepted, Implementing, Implemented, Done, Closed,
Archived and Cancelled may describe a team's process in prose, not persisted
`status` values. Session phases and artifact lifecycle phase markers are also
separate from status; completing code or tests does not automatically activate
an artifact. Validation checks and activation gates are distinct.

## Numbering Convention

| Artifact | Format | Example |
|----------|--------|---------|
| Epic | `EPIC-NNN` | EPIC-001 |
| PRD | `PRD-NNN` | PRD-042 |
| Spec | `SPEC-NNN` | SPEC-015 |
| RFC | `RFC-NNN` | RFC-128 |
| ADR | `ADR-NNN` | ADR-007 |

Display numbers are allocated per artifact kind within a project. Preserve
assigned numbers as a convention; local allocation is not a cross-branch
uniqueness guarantee. Canonical slug identity and the dated CI-assignment
proposal are distinguished in [ID-ASSIGNMENT.ru.md](ID-ASSIGNMENT.ru.md).

## Related Documents

- [PRD-RFC-ADR-FLOW.md](PRD-RFC-ADR-FLOW.md) — Decision tree: which artifact to create
- [DEPTH-CALIBRATION.md](DEPTH-CALIBRATION.md) — How depth determines artifact requirements
- [FORGEPLAN-GUIDE.md](FORGEPLAN-GUIDE.md) — Complete practical guide
