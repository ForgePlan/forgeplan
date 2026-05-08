---
author: scan-import
depth: standard
id: PRD-001
kind: prd
links:
- target: ADR-003
  relation: informs
status: draft
title: RFC ADR FLOW
---

[English](PRD-RFC-ADR-FLOW.md) · [Русский](PRD-RFC-ADR-FLOW.ru.md)

# PRD → RFC → ADR Flow — Decision Tree

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
               │      │ RFC  │  │ ADR  │
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

```
1. /research [topic]          ← study the problem
2. /write-doc prd [topic]     ← describe WHAT and WHY
3. Review PRD                 ← adversarial review
4. /write-doc spec [topic]    ← describe API/data model (if needed)
5. /write-doc rfc [topic]     ← describe HOW to build
6. /write-doc adr [decision]  ← document WHY this way
7. /sprint RFC-NNN Phase X    ← implement
8. /audit                     ← verify
9. memory_retain()            ← save to memory
```

### Path 2: Refactoring / Tech Debt (Quick Path)

```
1. /research [topic]          ← study current state
2. /write-doc adr [decision]  ← document decision + plan
3. /sprint ADR-NNN Phase X    ← implement
```

### Path 3: Bug / Incident

```
1. Investigate               ← find root cause
2. /write-doc adr [fix]      ← document the decision
3. Fix + PR                  ← implement
```

### Path 4: Roadmap / Large Initiative

```
1. /deep-research [topic]     ← deep research
2. Create Epic                ← strategic initiative
3. N × /write-doc prd         ← PRD for each part
4. N × /write-doc rfc         ← RFC for each PRD
5. N × /sprint                ← implementation in phases
```

## When to Use What

| I want to... | Create | Command |
|--------------|--------|---------|
| Describe a new feature for users | PRD | `/write-doc prd` |
| Describe API contracts | SPEC | `/write-doc spec` |
| Propose an architectural solution | RFC | `/write-doc rfc` |
| Document an accepted decision | ADR | `/write-doc adr` |
| Combine multiple PRDs/RFCs into an initiative | Epic | `/write-doc epic` |
| Quickly study a topic | — | `/research` |
| Deep study before major work | — | `/deep-research` |
| Implement in phases | — | `/sprint` |
| Verify quality | — | `/audit` |

## Artifact Lifecycle

```
             Draft
               │
         ┌─────┴─────┐
         ▼            ▼
      Review       (skip for
         │          tactical)
         ▼
     Approved ──────────────────→ Rejected
         │                         (with reason)
         ▼
   Implementing
         │
         ▼
   Implemented ─→ verify ─→ Closed
```

## Linking Rules

| Link | Description | Example |
|------|-------------|---------|
| PRD → SPEC | PRD produces a specification | PRD-001 → SPEC-001 |
| PRD → RFC | PRD produces architecture | PRD-001 → RFC-042 |
| RFC → ADR | RFC produces decisions | RFC-042 → ADR-007 |
| PRD → Epic | PRD belongs to an epic | PRD-001 → EPIC-003 |
| ADR supersedes ADR | Decision replaces another | ADR-012 supersedes ADR-007 |

## Depth Calibration (from Quint-code)

| Signal | Depth | What to Create |
|--------|-------|----------------|
| Quick fix, 1 file | **Tactical** | Nothing or Note |
| Feature for 1-3 days | **Standard** | PRD (tactical) → RFC |
| New module, 1-2 weeks | **Deep** | PRD → SPEC → RFC → ADR |
| New subsystem, cross-team | **Critical** | Epic → PRD[] → SPEC[] → RFC[] → ADR[] |

**Rule**: when in doubt, choose one level higher. An extra PRD is better than having to redo things later.

## Checklist Before Starting Implementation

- [ ] Problem Statement is clear?
- [ ] Goals are measurable?
- [ ] Non-Goals are defined (scope)?
- [ ] Architecture is described (RFC)?
- [ ] Key decisions are documented (ADR)?
- [ ] Acceptance Criteria exist?
- [ ] Risks are assessed?


