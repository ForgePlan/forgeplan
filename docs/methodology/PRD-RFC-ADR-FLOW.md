[English](PRD-RFC-ADR-FLOW.md) · [Русский](PRD-RFC-ADR-FLOW.ru.md)

# PRD → RFC → ADR Flow — Decision Tree

Runtime reference: **Forgeplan 0.38.0**. The tree below helps choose documents; arrows describe an authoring process, not automatic creation or lifecycle transitions. RFC proposes a technical approach; ADR records a consequential decision; use a Note when neither applies.

## Quick Decision: Which Document to Create?

```
You have a task. Start here:
                    │
        ┌───────────┴───────────┐
        │ Is there a USER       │
        │ with a PROBLEM?       │
        └───────┬───────┬───────┘
               YES     NO
                │       │
                ▼       ▼
            ┌──────┐ ┌──────────────────────┐
            │ PRD  │ │ Technical decision?   │
            └──┬───┘ └────┬────────┬────────┘
               │         YES      NO
               │          │        │
               │          ▼        ▼
               │      ┌──────┐  ┌──────┐
               │      │ RFC  │  │ NOTE │
               │      └──────┘  └──────┘
               │
        ┌──────┴──────┐
        │ Need API    │
        │ contracts?  │
        └───┬────┬────┘
           YES  NO
            │    │
            ▼    │
        ┌──────┐ │
        │ SPEC │ │
        └──┬───┘ │
           │     │
           ▼     ▼
        ┌──────────┐
        │   RFC    │
        │ (archit) │
        └────┬─────┘
             │
      ┌──────┴──────┐
      │ Making a    │
      │ decision?   │
      └───┬────┬────┘
         YES  NO
          │    │
          ▼    ▼
      ┌──────┐  Sprint
      │ ADR  │
      └──────┘
```

## Full Flow (Step by Step)

### Path 1: New Feature (Full Path)

1. Investigate the user problem; use `/research` when the public `fpl-skills` plugin is installed.
2. Run `forgeplan route "Describe the task"` and review the suggested depth and pipeline.
3. Create a PRD with `forgeplan new prd "Feature title"` and fill its requirements and outcomes. The CLI creates a template; it does not conduct the interview or implement the feature.
4. Create a SPEC when a contract is needed. Core accepts structural API/data contracts **or** behavioural requirements/scenarios. Public `/spec-author` produces and reviews the latter for scenario-driven implementation.
5. Create an RFC for design/implementation planning and an ADR for significant decisions, according to depth and task scope.
6. Validate each artifact with `forgeplan validate <ID> --ci`; resolve MUST findings and review SHOULD findings. `forgeplan reason <ID>` needs a configured LLM provider and generates ADI reasoning, not independent verification.
7. Obtain independent review before freezing the contract. Public `agents-tdd` `/tdd` requires active PRD + active scenario-bearing SPEC; its test validator certifies RED tests before GREEN. Do not rewrite scenarios or frozen tests to fit code.
8. Implement and verify using the project's tooling; `/sprint`, `/build` and `/audit` are marketplace orchestration skills, not core CLI commands.
9. Record the actual review/test results in EVID, link to the target, score and use the appropriate lifecycle command. A specification-review EVID can justify freezing a contract before code exists; do not label it implementation-test evidence.

### Path 2: Refactoring / Tech Debt (Quick Path)

Investigate the current behaviour and choose depth. Record a consequential choice in an ADR and a plan in an RFC when needed; do not invent a user-facing PRD for purely internal work. Preserve behaviour with explicit contracts and verification.

### Path 3: Bug / Incident

Investigate the root cause, record a Problem or Note when useful, fix and verify it. An ADR is appropriate only if the fix introduces a decision worth preserving; it is not compulsory for every bug.

### Path 4: Roadmap / Large Initiative

Create an Epic to group multiple deliverables; create the PRDs, SPECs, RFCs and ADRs each part actually needs. Set parent metadata / graph links explicitly and track implementation with checkboxes.

## When to Use What

| Need | Artifact / public entry point |
|------|-------------------------------|
| User problem, outcomes and product requirements | `forgeplan new prd "Title"` |
| Checkable API/data or behavioural contract | `forgeplan new spec "Title"`; `/spec-author` for scenario authoring |
| Proposed technical approach | `forgeplan new rfc "Title"` |
| Decision and consequences | `forgeplan new adr "Title"` |
| Multi-artifact initiative | `forgeplan new epic "Title"` |
| Research, implementation, review orchestration | Public marketplace `/research`, `/sprint`, `/build`, `/audit`, as installed |

Slash skills require the matching plugin/harness. Installing the CLI alone does not install those orchestrators.

## Artifact Lifecycle

```
draft ──activate──→ active ──supersede──→ superseded
                      ├────deprecate──→ deprecated
                      └────mark stale─→ stale ──renew──→ active
                                          └─deprecate→ deprecated
```

`superseded` and `deprecated` are terminal in the lifecycle state machine. `reopen` creates a new draft from active/stale and retires the old artifact; it does not revive a terminal one. The advisory `phase` marker (shape/validate/adi/code/test/audit/evidence/done) is separate and permits out-of-order manual changes. Neither an `active` artifact nor a `done` phase proves the implementation shipped.

Expiry detection (`forgeplan stale`) only reports overdue dates; it does not persist `status: stale`. For a persisted stale record, `renew` follows the stale→active transition; an expired active record is not automatically in that state.

Activation normally checks MUST validation and, for PRD/Epic/SPEC/RFC/ADR, body length, evidence presence and stubs. Use lifecycle verbs; `update --status active|superseded|deprecated` cannot bypass them. `--force` is an explicit methodology-gate override, not evidence of verification.

## Linking Rules

| Relationship | CLI example (existing IDs) |
|--------------|----------------------------|
| SPEC refines product needs | `forgeplan link SPEC-001 PRD-001 --relation refines` |
| RFC depends on product needs | `forgeplan link RFC-042 PRD-001 --relation based_on` |
| ADR follows design | `forgeplan link ADR-007 RFC-042 --relation based_on` |
| Evidence supports a target | `forgeplan link EVID-001 PRD-001 --relation informs` |
| Replacement retires an active decision | `forgeplan supersede ADR-007 --by ADR-012` |

The public typed relations are `informs`, `based_on`, `supersedes`, `contradicts`, `refines`. Epic membership is `parent_epic`, not a `belongs_to` link accepted by `forgeplan link`. A body table does not create an edge.

## Depth Calibration (from Quint-code)

| Depth | Core route pipeline |
|-------|---------------------|
| Note / Tactical | No required artifact pipeline |
| Standard | PRD → RFC |
| Deep | PRD → SPEC → RFC → ADR |
| Critical | Accepted input alias for Deep; no additional core Epic gate |

This is a routing recommendation, not automatic pipeline creation. An Epic / expanded hierarchy for strategic work is methodology guidance. See [DEPTH-CALIBRATION.md](DEPTH-CALIBRATION.md) for actual runtime signals and gates.

## Checklist Before Starting Implementation

- [ ] Problem Statement is clear?
- [ ] Goals are measurable?
- [ ] Non-Goals are defined (scope)?
- [ ] Architecture is described (RFC)?
- [ ] Key decisions are documented (ADR)?
- [ ] Acceptance Criteria exist?
- [ ] Risks are assessed?
