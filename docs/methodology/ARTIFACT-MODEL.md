[English](ARTIFACT-MODEL.md) · [Русский](ARTIFACT-MODEL.ru.md)

# Artifact Model — PRD Process Engine

Runtime contract: **Forgeplan 0.38.0**. The hierarchy is a modelling convention, not an automatically generated tree. Sprint is a marketplace execution workflow, not an artifact kind. PRD captures needs; SPEC supplies a checkable structural or behavioural contract.

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

### 1. EPIC (EPIC-NNN)
**Purpose**: Group related PRDs/RFCs/ADRs into a single strategic initiative.

| Field | Description |
|-------|-------------|
| ID | EPIC-NNN (allocated per kind) |
| Name | Short initiative name |
| Status | Shared lifecycle below |
| Outcomes | Measurable results |
| Children | PRD[], RFC[], ADR[] |
| Progress | Template shows a manual child summary; `progress` counts the selected body's checkboxes |
| Owner | Product/Engineering lead |
| Timeline | Start → Target completion |

**When to create**: Initiative with > 2 RFCs, cross-team work, roadmap item.

### 2. PRD (PRD-NNN)
**Purpose**: Describe WHAT the user needs and WHY (product perspective).

| Field | Description |
|-------|-------------|
| ID | PRD-NNN |
| Epic | Parent Epic (optional) |
| Status | Shared lifecycle below |
| Problem | What problem we are solving |
| Goals | What we want to achieve (measurable) |
| Non-Goals | What we are NOT doing (scope) |
| Requirements | Functional (FR-NNN) + Non-Functional (NFR-NNN) |
| Success Metrics | KPIs, OKRs |
| Acceptance Criteria | Definition of Done |
| Stakeholders | Sign-offs |

**When to create**: New feature, significant product change, user-facing change.

### 3. SPEC (SPEC-NNN)
**Purpose**: Specify an implementation-checkable contract: API/data structure, observable behaviour, or both. In a scenario-driven workflow, reviewed `#### Scenario` blocks are the test oracle; they are not product discovery prose.

| Field | Description |
|-------|-------------|
| ID | SPEC-NNN |
| PRD | Parent PRD |
| Type | API, Data Model, Protocol, UI Spec or behavioural specification |
| Status | Shared lifecycle below |
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
| Status | Shared lifecycle below |
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
| Status | Shared lifecycle below |

**When to create**: Technology choice, architectural trade-off, significant decision.

## Relationships Between Artifacts

```
SPEC-001 ──refines──→ PRD-001
RFC-001  ──based_on─→ PRD-001
ADR-001  ──based_on─→ RFC-001
EVID-001 ──informs──→ PRD-001
PRD-001.parent_epic = EPIC-001
```

Create graph edges explicitly with `forgeplan link`. Public link types are `informs`, `based_on`, `supersedes`, `contradicts`, `refines`; `creates`, `contains`, `belongs_to` and `Sprint` are not extra public link/kind values. Parent membership is metadata (`parent_epic`); body tables alone do not mutate it or graph links. Cardinality is not restricted to one SPEC per RFC.

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

## Status Values (unified)

All five structured kinds use the same stored statuses:

| Status / transition | Meaning |
|---------------------|---------|
| `draft → active` | `activate`, normally gated by MUST validation, minimum body length, evidence presence and no detected stub |
| `active → superseded` | `supersede --by <new>`; successor linked, old record retained |
| `active → deprecated` | `deprecate --reason "..."` |
| `active → stale` | Allowed stored-state transition; expiry reporting alone does not persist it |
| `stale → active` | `renew --reason "..." --until YYYY-MM-DD` |
| `stale → deprecated` | Retire, or `reopen` into a new draft |

`superseded` and `deprecated` are terminal in the state machine. `reopen` accepts active/stale, creates a new draft and retires the original; it does not resurrect terminal artifacts. `Review`, `Approved`, `Implemented`, `Done`, `Archived` and `Closed` are not runtime lifecycle values. Track delivery with checkboxes and advisory phases (`shape` through `done`) separately from validity status.

`update --status active|superseded|deprecated` is rejected; use lifecycle commands. Activation's evidence-presence gate is not proof that tests passed or a universal R_eff threshold. The CLI does not implement or independently verify the feature for you.

Release 0.38 does not apply the full transition guard to raw metadata updates; resetting a terminal record is possible through that gap ([#550](https://github.com/ForgePlan/forgeplan/issues/550)). The terminal-state table describes the intended lifecycle, not a guarantee that every write path enforces it. Use the dedicated commands.

Keep three state views separate:
- Stored lifecycle: `draft`, `active`, `stale`, `superseded`, `deprecated`. `forgeplan stale` reports expired dates without changing status. Renewal is intended for reviewed stale records; 0.38 also accepts draft without activation gates ([#547](https://github.com/ForgePlan/forgeplan/issues/547)), which must not be used as an activation shortcut.
- Computed display state: `STUB → SHAPED → VALIDATED → EVIDENCED → ACTIVATED`. This is derived, not stored. Active/superseded/deprecated records display ACTIVATED; EVIDENCED requires linked evidence and positive R_eff. The SHAPED heading heuristic is not the full validator.
- Advisory artifact phase: `shape`, `validate`, `adi`, `code`, `test`, `audit`, `evidence`, `done`. Manual `phase-advance` permits out-of-order changes and does not update lifecycle status.

## Numbering Convention

| Artifact | Format | Example |
|----------|--------|---------|
| Epic | `EPIC-NNN` | EPIC-001 |
| PRD | `PRD-NNN` | PRD-042 |
| Spec | `SPEC-NNN` | SPEC-015 |
| RFC | `RFC-NNN` | RFC-128 |
| ADR | `ADR-NNN` | ADR-007 |

New IDs are allocated per kind with a shared Git-common-directory lock across worktrees of the same repository and collision checks; separate clones do not share that lock. Slug identity and numeric display identity coexist. In the shipped default creation path `assigned_number` is set immediately; the frontmatter helper preserves an explicit null for deferred assignment. Do not assume every new artifact waits for a merge bot or displays `?`.

Legacy artifacts without slug remain addressable by ID and do not require a cosmetic migration. See [ID-ASSIGNMENT.ru.md](ID-ASSIGNMENT.ru.md) for the current boundary and preserved rollout history.

## Related Documents

- [PRD-RFC-ADR-FLOW.md](PRD-RFC-ADR-FLOW.md) — Decision tree: which artifact to create
- [DEPTH-CALIBRATION.md](DEPTH-CALIBRATION.md) — How depth determines artifact requirements
- [FORGEPLAN-GUIDE.md](FORGEPLAN-GUIDE.md) — Complete practical guide
